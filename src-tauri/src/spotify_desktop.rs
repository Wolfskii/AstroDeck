use librespot_connect::{ConnectConfig, LoadRequest, LoadRequestOptions, Spirc};
use librespot_core::authentication::Credentials;
use librespot_core::cache::Cache;
use librespot_core::config::{DeviceType, SessionConfig};
use librespot_core::session::Session;
use librespot_metadata::audio::UniqueFields;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig, VolumeCtrl};
use librespot_playback::mixer::{self, MixerConfig};
use librespot_playback::player::{Player, PlayerEvent};
use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::Message;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

use crate::spotify::{
    get_access_token, token_has_streaming_scope, SpotifyPlaylist, SpotifyPlaylistPage,
    SpotifyState, OFFICIAL_CLIENT_ID,
};

const ROOTLIST_LIMIT: usize = 500;
const IMAGE_CDN: &str = "https://i.scdn.co/image/";
pub const OFFICIAL_DEVICE_NAME: &str = "AstroDeck";

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("spotify-desktop")
            .worker_threads(4)
            .build()
            .expect("Spotify desktop runtime")
    })
}

#[derive(Clone, Debug)]
pub struct OfficialPlayback {
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub cover_url: Option<String>,
    pub item_id: Option<String>,
    pub item_type: Option<String>,
    pub duration_ms: Option<u64>,
    pub progress_ms: Option<u64>,
    pub progress_at: Option<Instant>,
    pub is_playing: bool,
    pub volume_percent: u8,
    pub shuffle: bool,
    pub player_ready: bool,
}

impl Default for OfficialPlayback {
    fn default() -> Self {
        Self {
            track_name: None,
            artist_name: None,
            album_name: None,
            cover_url: None,
            item_id: None,
            item_type: None,
            duration_ms: None,
            progress_ms: None,
            progress_at: None,
            is_playing: false,
            volume_percent: 50,
            shuffle: false,
            player_ready: false,
        }
    }
}

pub fn drop_session(spotify: &SpotifyState) {
    shutdown_player(spotify);
    if let Ok(mut guard) = spotify.desktop_session.lock() {
        *guard = None;
    }
}

pub fn clear_credentials(spotify: &SpotifyState) {
    drop_session(spotify);
    if let Some(path) = cache_dir(spotify) {
        let credentials = path.join("credentials.json");
        if credentials.exists() {
            let _ = fs::remove_file(credentials);
        }
    }
}

pub fn ensure_session(spotify: &SpotifyState) -> Result<Session, String> {
    {
        let mut guard = spotify.desktop_session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = guard.as_ref() {
            if !session.is_invalid() {
                return Ok(session.clone());
            }
        }
        *guard = None;
    }
    shutdown_player(spotify);

    let cache = open_cache(spotify);
    if let Some(stored) = cache.as_ref().and_then(|cache| cache.credentials()) {
        match connect_session(stored, cache.clone()) {
            Ok(session) => {
                store_session(spotify, session.clone());
                return Ok(session);
            }
            Err(err) => {
                log::info!(
                    "Spotify stored desktop credentials failed, trying access token: {err}"
                );
            }
        }
    }

    let session = connect_with_access_token(spotify, cache)?;
    store_session(spotify, session.clone());
    Ok(session)
}

pub fn ensure_player(spotify: &SpotifyState) -> Result<(), String> {
    let session = ensure_session(spotify)?;
    {
        let guard = spotify.desktop_spirc.lock().map_err(|e| e.to_string())?;
        if guard.is_some() && !session.is_invalid() {
            return Ok(());
        }
    }

    shutdown_player(spotify);
    let cache = open_cache(spotify);
    let credentials = player_credentials(spotify, cache.as_ref())?;
    let playback = spotify.desktop_playback.clone();
    let spirc = start_spirc(session, credentials, playback)?;
    *spotify.desktop_spirc.lock().map_err(|e| e.to_string())? = Some(spirc);
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.player_ready = true;
    }
    Ok(())
}

pub fn playback_snapshot(spotify: &SpotifyState) -> OfficialPlayback {
    let Ok(guard) = spotify.desktop_playback.lock() else {
        return OfficialPlayback::default();
    };
    let mut snapshot = guard.clone();
    snapshot.progress_ms = interpolated_progress(&snapshot);
    snapshot
}

pub fn should_show_spotify_scene(spotify: &SpotifyState) -> bool {
    let Ok(playback) = spotify.desktop_playback.lock() else {
        return false;
    };
    playback.is_playing || playback.track_name.is_some()
}

pub fn play_context_uri(spotify: &SpotifyState, context_uri: &str) -> Result<(), String> {
    ensure_player(spotify)?;
    with_spirc(spotify, |spirc| {
        spirc.activate().map_err(player_command_error)?;
        spirc
            .load(LoadRequest::from_context_uri(
                context_uri.to_string(),
                LoadRequestOptions {
                    start_playing: true,
                    ..LoadRequestOptions::default()
                },
            ))
            .map_err(player_command_error)?;
        spirc.play().map_err(player_command_error)
    })?;
    log::info!("Spotify desktop: streaming {context_uri} in AstroDeck");
    Ok(())
}

pub fn handle_transport(spotify: &SpotifyState, command: &str) -> Result<(), String> {
    ensure_player(spotify)?;
    with_spirc(spotify, |spirc| match command {
        "togglePlay" => spirc.play_pause().map_err(player_command_error),
        "nextTrack" => spirc.next().map_err(player_command_error),
        "prevTrack" => spirc.prev().map_err(player_command_error),
        other => Err(format!("Unknown Spotify player command: {other}")),
    })
}

pub fn toggle_shuffle(spotify: &SpotifyState) -> Result<bool, String> {
    ensure_player(spotify)?;
    let next = {
        let playback = spotify.desktop_playback.lock().map_err(|e| e.to_string())?;
        !playback.shuffle
    };
    with_spirc(spotify, |spirc| {
        spirc.shuffle(next).map_err(player_command_error)
    })?;
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.shuffle = next;
    }
    Ok(next)
}

pub fn set_volume(spotify: &SpotifyState, volume_percent: u8) -> Result<u8, String> {
    ensure_player(spotify)?;
    let volume_percent = volume_percent.min(100);
    with_spirc(spotify, |spirc| {
        spirc
            .set_volume(percent_to_volume(volume_percent))
            .map_err(player_command_error)
    })?;
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.volume_percent = volume_percent;
    }
    Ok(volume_percent)
}

pub fn adjust_volume(spotify: &SpotifyState, delta: i32) -> Result<u8, String> {
    let current = spotify
        .desktop_playback
        .lock()
        .map(|playback| playback.volume_percent)
        .unwrap_or(50) as i32;
    set_volume(spotify, (current + delta).clamp(0, 100) as u8)
}

pub fn seek(spotify: &SpotifyState, position_ms: u64) -> Result<(), String> {
    ensure_player(spotify)?;
    let position = u32::try_from(position_ms).unwrap_or(u32::MAX);
    with_spirc(spotify, |spirc| {
        spirc.set_position_ms(position).map_err(player_command_error)
    })?;
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.progress_ms = Some(position_ms);
        playback.progress_at = Some(Instant::now());
    }
    Ok(())
}

fn shutdown_player(spotify: &SpotifyState) {
    if let Ok(mut guard) = spotify.desktop_spirc.lock() {
        if let Some(spirc) = guard.take() {
            let _ = spirc.shutdown();
        }
    }
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        *playback = OfficialPlayback::default();
    }
}

fn store_session(spotify: &SpotifyState, session: Session) {
    if let Ok(mut guard) = spotify.desktop_session.lock() {
        *guard = Some(session);
    }
}

fn cache_dir(spotify: &SpotifyState) -> Option<PathBuf> {
    spotify
        .config
        .lock()
        .ok()?
        .desktop_cache_path
        .clone()
}

fn open_cache(spotify: &SpotifyState) -> Option<Cache> {
    let path = cache_dir(spotify)?;
    let files = path.join("files");
    Cache::new(Some(path), None::<PathBuf>, Some(files), None).ok()
}

fn connect_with_access_token(
    spotify: &SpotifyState,
    cache: Option<Cache>,
) -> Result<Session, String> {
    if token_has_streaming_scope(spotify)? == Some(false) {
        return Err(reconnect_for_desktop_session_error());
    }

    let access_token = get_access_token(spotify)?;
    connect_session(Credentials::with_access_token(access_token), cache)
        .map_err(desktop_session_login_error)
}

fn connect_session(credentials: Credentials, cache: Option<Cache>) -> Result<Session, String> {
    runtime().block_on(async {
        let mut config = SessionConfig::default();
        config.client_id = OFFICIAL_CLIENT_ID.to_string();
        let session = Session::new(config, cache);
        session
            .connect(credentials, true)
            .await
            .map_err(|e| format!("{e}"))?;
        Ok(session)
    })
}

fn player_credentials(
    spotify: &SpotifyState,
    cache: Option<&Cache>,
) -> Result<Credentials, String> {
    if let Some(stored) = cache.and_then(|cache| cache.credentials()) {
        return Ok(stored);
    }
    if token_has_streaming_scope(spotify)? == Some(false) {
        return Err(reconnect_for_desktop_session_error());
    }
    Ok(Credentials::with_access_token(get_access_token(spotify)?))
}

fn start_spirc(
    session: Session,
    credentials: Credentials,
    playback: Arc<Mutex<OfficialPlayback>>,
) -> Result<Spirc, String> {
    runtime().block_on(async {
        let mixer_builder = mixer::find(None)
            .ok_or_else(|| "Spotify audio mixer is unavailable".to_string())?;
        let mixer = mixer_builder(MixerConfig::default())
            .map_err(|e| format!("Spotify mixer failed: {e}"))?;

        let sink_builder = audio_backend::find(None)
            .ok_or_else(|| "Spotify audio output is unavailable".to_string())?;
        let audio_format = AudioFormat::default();
        let mut player_config = PlayerConfig::default();
        player_config.position_update_interval = Some(Duration::from_secs(1));

        let player = Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || sink_builder(None, audio_format),
        );
        let mut events = player.get_player_event_channel();

        let connect_config = ConnectConfig {
            name: OFFICIAL_DEVICE_NAME.to_string(),
            device_type: DeviceType::Computer,
            ..ConnectConfig::default()
        };

        let (spirc, spirc_task) =
            Spirc::new(connect_config, session, credentials, player, mixer)
                .await
                .map_err(player_start_error)?;

        tokio::spawn(spirc_task);
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                apply_player_event(&playback, event);
            }
        });

        spirc.activate().map_err(player_command_error)?;
        Ok(spirc)
    })
}

fn with_spirc<R>(
    spotify: &SpotifyState,
    f: impl FnOnce(&Spirc) -> Result<R, String>,
) -> Result<R, String> {
    let guard = spotify.desktop_spirc.lock().map_err(|e| e.to_string())?;
    let spirc = guard
        .as_ref()
        .ok_or_else(|| "Spotify player is not running in AstroDeck".to_string())?;
    f(spirc)
}

fn apply_player_event(playback: &Mutex<OfficialPlayback>, event: PlayerEvent) {
    let Ok(mut playback) = playback.lock() else {
        return;
    };
    match event {
        PlayerEvent::TrackChanged { audio_item } => {
            playback.track_name = Some(audio_item.name);
            playback.cover_url = audio_item.covers.first().map(|cover| cover.url.clone());
            playback.duration_ms = Some(u64::from(audio_item.duration_ms));
            playback.item_id = audio_item.track_id.to_id().ok();
            let (artist, album, item_type) = metadata_from_unique_fields(audio_item.unique_fields);
            playback.artist_name = artist;
            playback.album_name = album;
            playback.item_type = Some(item_type.to_string());
        }
        PlayerEvent::Playing { position_ms, .. }
        | PlayerEvent::PositionCorrection { position_ms, .. } => {
            playback.is_playing = true;
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::Paused { position_ms, .. } => {
            playback.is_playing = false;
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::Seeked { position_ms, .. } => {
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::PositionChanged { position_ms, .. } => {
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::Stopped { .. } => {
            playback.is_playing = false;
        }
        PlayerEvent::VolumeChanged { volume } => {
            playback.volume_percent = volume_to_percent(volume);
        }
        PlayerEvent::ShuffleChanged { shuffle } => {
            playback.shuffle = shuffle;
        }
        PlayerEvent::Unavailable { track_id, denied, .. } => {
            log::warn!("Spotify track unavailable ({track_id:?}, denied={denied})");
        }
        _ => {}
    }
}

fn metadata_from_unique_fields(
    fields: UniqueFields,
) -> (Option<String>, Option<String>, &'static str) {
    match fields {
        UniqueFields::Track { artists, album, .. } => {
            let names: Vec<String> = artists.0.into_iter().map(|artist| artist.name).collect();
            let artist = if names.is_empty() {
                None
            } else {
                Some(names.join(", "))
            };
            (artist, Some(album), "track")
        }
        UniqueFields::Local {
            artists, album, ..
        } => (artists, album, "track"),
        UniqueFields::Episode { show_name, .. } => {
            (Some(show_name.clone()), Some(show_name), "episode")
        }
    }
}

fn set_progress(playback: &mut OfficialPlayback, position_ms: u32) {
    playback.progress_ms = Some(u64::from(position_ms));
    playback.progress_at = Some(Instant::now());
}

fn interpolated_progress(playback: &OfficialPlayback) -> Option<u64> {
    let progress = playback.progress_ms?;
    if !playback.is_playing {
        return Some(progress);
    }
    let elapsed = playback
        .progress_at
        .map(|at| at.elapsed().as_millis() as u64)
        .unwrap_or(0);
    Some(match playback.duration_ms {
        Some(duration) => (progress + elapsed).min(duration),
        None => progress + elapsed,
    })
}

fn percent_to_volume(percent: u8) -> u16 {
    let max = u32::from(VolumeCtrl::MAX_VOLUME);
    let pct = u32::from(percent.min(100));
    ((pct * max + 50) / 100) as u16
}

fn volume_to_percent(volume: u16) -> u8 {
    let max = u32::from(VolumeCtrl::MAX_VOLUME);
    ((u32::from(volume) * 100 + max / 2) / max) as u8
}

fn reconnect_for_desktop_session_error() -> String {
    "Spotify desktop playlists need a fresh login. Disconnect and connect again so the browser prompt can grant desktop access.".to_string()
}

fn desktop_session_login_error(err: String) -> String {
    let lower = err.to_ascii_lowercase();
    if lower.contains("bad credentials") || lower.contains("login failed") {
        reconnect_for_desktop_session_error()
    } else {
        format!("Spotify desktop session failed: {err}")
    }
}

fn player_start_error(err: impl std::fmt::Display) -> String {
    let message = err.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("invalid_credentials")
        || lower.contains("bad credentials")
        || lower.contains("premium")
    {
        "Spotify Premium is required to play in AstroDeck. Reconnect a Premium account, or switch to a developer app if you want the Spotify desktop app to handle playback.".to_string()
    } else {
        format!("Spotify player failed to start: {message}")
    }
}

fn player_command_error(err: impl std::fmt::Display) -> String {
    format!("Spotify player command failed: {err}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_credentials_asks_to_reconnect() {
        let message = desktop_session_login_error(
            "Permission denied { Login failed with reason: Bad credentials }".to_string(),
        );
        assert!(message.contains("Disconnect and connect again"));
        assert!(!message.contains("Bad credentials"));
    }

    #[test]
    fn volume_percent_roundtrip_stays_near_original() {
        for pct in [0_u8, 1, 25, 50, 75, 99, 100] {
            let back = volume_to_percent(percent_to_volume(pct));
            assert!(
                (i16::from(back) - i16::from(pct)).abs() <= 1,
                "{pct} mapped to {back}"
            );
        }
    }

    #[test]
    fn interpolated_progress_advances_while_playing() {
        let playback = OfficialPlayback {
            progress_ms: Some(1_000),
            progress_at: Some(Instant::now() - Duration::from_millis(500)),
            is_playing: true,
            duration_ms: Some(10_000),
            ..OfficialPlayback::default()
        };
        let next = interpolated_progress(&playback).expect("progress");
        assert!(next >= 1_400, "got {next}");
        assert!(next <= 1_700, "got {next}");
    }

    #[test]
    fn player_start_maps_keymaster_failure() {
        let message = player_start_error("Audio key: INVALID_CREDENTIALS");
        assert!(message.contains("Premium"));
        assert!(!message.contains("INVALID_CREDENTIALS"));
    }
}
