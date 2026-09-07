use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::Message;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::spotify::{
    get_access_token, SpotifyPlaylist, SpotifyPlaylistPage, SpotifyState, OFFICIAL_CLIENT_ID,
};

const ROOTLIST_LIMIT: usize = 500;
const IMAGE_CDN: &str = "https://i.scdn.co/image/";

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("spotify-desktop")
            .worker_threads(2)
            .build()
            .expect("Spotify desktop runtime")
    })
}

pub fn drop_session(spotify: &SpotifyState) {
    if let Ok(mut guard) = spotify.desktop_session.lock() {
        *guard = None;
    }
}

pub fn ensure_session(spotify: &SpotifyState) -> Result<Session, String> {
    if let Ok(guard) = spotify.desktop_session.lock() {
        if let Some(session) = guard.as_ref() {
            return Ok(session.clone());
        }
    }

    let access_token = get_access_token(spotify)?;
    let session = connect_session(access_token)?;
    if let Ok(mut guard) = spotify.desktop_session.lock() {
        *guard = Some(session.clone());
    }
    Ok(session)
}

fn connect_session(access_token: String) -> Result<Session, String> {
    runtime().block_on(async {
        let mut config = SessionConfig::default();
        config.client_id = OFFICIAL_CLIENT_ID.to_string();
        let session = Session::new(config, None);
        session
            .connect(Credentials::with_access_token(access_token), false)
            .await
            .map_err(|e| format!("Spotify desktop session failed: {e}"))?;
        Ok(session)
    })
}

pub fn list_playlists(
    spotify: &SpotifyState,
    offset: u32,
    limit: u32,
) -> Result<SpotifyPlaylistPage, String> {
    let session = match ensure_session(spotify) {
        Ok(session) => session,
        Err(err) => {
            drop_session(spotify);
            return Err(err);
        }
    };

    match fetch_rootlist_page(&session, offset, limit) {
        Ok(page) => Ok(page),
        Err(err) => {
            drop_session(spotify);
            let session = ensure_session(spotify)?;
            fetch_rootlist_page(&session, offset, limit).map_err(|retry| {
                format!("{retry} (after reconnect; first error: {err})")
            })
        }
    }
}

fn fetch_rootlist_page(
    session: &Session,
    offset: u32,
    limit: u32,
) -> Result<SpotifyPlaylistPage, String> {
    runtime().block_on(async {
        let body = tokio::time::timeout(
            Duration::from_secs(20),
            session.spclient().get_rootlist(0, Some(ROOTLIST_LIMIT)),
        )
        .await
        .map_err(|_| "Spotify desktop playlist list timed out".to_string())?
        .map_err(|e| format!("Spotify desktop playlist list failed: {e}"))?;

        let root = SelectedListContent::parse_from_bytes(&body)
            .map_err(|e| format!("Spotify desktop playlist list could not be decoded: {e}"))?;
        Ok(page_from_rootlist(&root, offset, limit))
    })
}

fn page_from_rootlist(
    root: &SelectedListContent,
    offset: u32,
    limit: u32,
) -> SpotifyPlaylistPage {
    let Some(contents) = root.contents.as_ref() else {
        return SpotifyPlaylistPage {
            items: Vec::new(),
            offset,
            limit,
            total: 0,
            next_offset: None,
        };
    };
    let meta = &contents.meta_items;
    let mut items = Vec::new();
    for (index, item) in contents.items.iter().enumerate() {
        let Some(id) = item.uri().strip_prefix("spotify:playlist:") else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let meta = meta.get(index);
        let name = meta
            .map(|entry| entry.attributes.name())
            .filter(|name| !name.is_empty())
            .unwrap_or("Playlist")
            .to_string();
        let owner_name = meta
            .map(|entry| entry.owner_username())
            .filter(|owner| !owner.is_empty())
            .map(str::to_string);
        items.push(SpotifyPlaylist {
            id: id.to_string(),
            name,
            uri: format!("spotify:playlist:{id}"),
            image_url: meta.and_then(|entry| playlist_cover(&entry.attributes)),
            track_count: meta
                .map(|entry| entry.length().max(0) as u32)
                .unwrap_or(0),
            owner_name,
        });
    }

    let total = items.len() as u32;
    let start = (offset as usize).min(items.len());
    let end = (start + limit as usize).min(items.len());
    let sliced = items[start..end].to_vec();
    let next_offset = (end as u32 != total).then_some(end as u32);

    SpotifyPlaylistPage {
        items: sliced,
        offset,
        limit,
        total,
        next_offset,
    }
}

fn playlist_cover(
    attributes: &librespot_protocol::playlist4_external::ListAttributes,
) -> Option<String> {
    for target in ["xlarge", "large", "default", "small"] {
        if let Some(url) = attributes
            .picture_size
            .iter()
            .find(|size| size.target_name() == target)
            .map(|size| size.url())
            .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        {
            return Some(url.to_string());
        }
    }
    attributes
        .picture_size
        .first()
        .map(|size| size.url())
        .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        .map(str::to_string)
        .or_else(|| image_url(attributes.picture()))
}

fn image_url(file_id: &[u8]) -> Option<String> {
    if file_id.is_empty() {
        return None;
    }
    let hex = file_id.iter().fold(String::new(), |mut hex, byte| {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    Some(format!("{IMAGE_CDN}{hex}"))
}

pub fn open_spotify_uri(uri: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", uri])
            .status()
            .map_err(|e| format!("Failed to open {uri} in Spotify: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(uri)
            .status()
            .map_err(|e| format!("Failed to open {uri} in Spotify: {e}"))?;
        return Ok(());
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(uri)
            .status()
            .map_err(|e| format!("Failed to open {uri} in Spotify: {e}"))?;
        Ok(())
    }
}
