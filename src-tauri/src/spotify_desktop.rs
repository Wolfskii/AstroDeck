use librespot_core::authentication::Credentials;
use librespot_core::cache::Cache;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::Message;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
#[cfg(windows)]
use std::time::Instant;
use tokio::runtime::Runtime;

use crate::spotify::{
    get_access_token, token_has_streaming_scope, SpotifyPlaylist, SpotifyPlaylistPage,
    SpotifyState, OFFICIAL_CLIENT_ID,
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
    Cache::new(Some(path), None::<PathBuf>, None::<PathBuf>, None).ok()
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
        return open_spotify_uri_windows(uri);
    }

    #[cfg(target_os = "macos")]
    {
        let play_uri = playlist_launch_uri(uri);
        let script = format!("tell application \"Spotify\" to play track \"{play_uri}\"");
        if Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        Command::new("open")
            .args(["-g", uri])
            .status()
            .map_err(|e| format!("Failed to play {uri} in Spotify: {e}"))?;
        return Ok(());
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        if play_uri_via_mpris(&playlist_launch_uri(uri)).is_ok() {
            return Ok(());
        }
        Command::new("xdg-open")
            .arg(playlist_launch_uri(uri))
            .status()
            .map_err(|e| format!("Failed to play {uri} in Spotify: {e}"))?;
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn play_uri_via_mpris(uri: &str) -> Result<(), String> {
    let status = Command::new("dbus-send")
        .args([
            "--session",
            "--type=method_call",
            "--dest=org.mpris.MediaPlayer2.spotify",
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player.OpenUri",
            &format!("string:{uri}"),
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("MPRIS OpenUri failed".to_string())
    }
}

#[cfg(windows)]
fn open_spotify_uri_windows(uri: &str) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let previous = unsafe { GetForegroundWindow() };
    let launch_uri = playlist_launch_uri(uri);
    let args = windows_start_args(&launch_uri);
    let status = Command::new("cmd")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to play {uri} in Spotify: {e}"))?;
    if !status.success() {
        return Err(format!("Failed to play {uri} in Spotify"));
    }

    // Spotify only starts the playlist if it can handle the URI in a normal
    // (not minimized) window. Give it time to do that, then put AstroDeck back
    // in front without hiding Spotify — minimizing too early cancelled playback.
    let previous_raw = previous.0 as isize;
    let _ = std::thread::Builder::new()
        .name("spotify-refocus".into())
        .spawn(move || {
            wait_for_spotify_window();
            std::thread::sleep(Duration::from_millis(800));
            if let Some(hwnd) = find_spotify_main_window() {
                focus_spotify_window(hwnd);
                std::thread::sleep(Duration::from_millis(120));
                send_spotify_play(hwnd);
            }
            std::thread::sleep(Duration::from_millis(250));
            restore_foreground_raw(previous_raw);
        });
    Ok(())
}

#[cfg(any(windows, test))]
fn windows_start_args(uri: &str) -> Vec<String> {
    vec![
        "/C".to_string(),
        "start".to_string(),
        String::new(),
        uri.to_string(),
    ]
}

fn playlist_launch_uri(context_uri: &str) -> String {
    let trimmed = context_uri.trim().trim_end_matches(':');
    if trimmed.ends_with(":play") {
        trimmed.to_string()
    } else {
        format!("{trimmed}:play")
    }
}

#[cfg(windows)]
fn wait_for_spotify_window() {
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if find_spotify_main_window().is_some_and(window_is_showing) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(windows)]
fn focus_spotify_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, SetForegroundWindow,
    };

    unsafe {
        let _ = AllowSetForegroundWindow(u32::MAX);
        let _ = SetForegroundWindow(hwnd);
    }
}

#[cfg(windows)]
fn send_spotify_play(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VK_SPACE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APPCOMMAND};

    // APPCOMMAND_MEDIA_PLAY = 46. Opening the playlist URI only shows it;
    // this asks Spotify to actually start playback.
    const APPCOMMAND_MEDIA_PLAY: isize = 46;
    let _ = unsafe {
        PostMessageW(
            hwnd,
            WM_APPCOMMAND,
            WPARAM(0),
            LPARAM(APPCOMMAND_MEDIA_PLAY << 16),
        )
    };

    let space_down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_SPACE,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let space_up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_SPACE,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [space_down, space_up];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
fn restore_foreground_raw(previous_raw: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, SetForegroundWindow,
    };

    if previous_raw == 0 {
        return;
    }
    let previous = HWND(previous_raw as *mut core::ffi::c_void);
    unsafe {
        let _ = AllowSetForegroundWindow(u32::MAX);
        let _ = SetForegroundWindow(previous);
    }
}

#[cfg(windows)]
fn window_is_showing(hwnd: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};

    if hwnd.0.is_null() {
        return false;
    }
    unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() && !window_is_cloaked(hwnd) }
}

#[cfg(windows)]
fn window_is_cloaked(hwnd: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};

    let mut cloaked: u32 = 0;
    let ok = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
    };
    ok.is_ok() && cloaked != 0
}

#[cfg(windows)]
fn spotify_pids() -> Vec<u32> {
    use sysinfo::{ProcessesToUpdate, System};

    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .processes()
        .iter()
        .filter_map(|(_, process)| {
            let name = process.name().to_string_lossy().to_ascii_lowercase();
            if name == "spotify.exe" || name == "spotify" {
                Some(process.pid().as_u32())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(windows)]
fn find_spotify_main_window() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetParent, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId,
        GWL_EXSTYLE,
    };

    const WS_EX_TOOLWINDOW: i32 = 0x80;
    let pids = spotify_pids();
    if pids.is_empty() {
        return None;
    }

    struct Search {
        pids: Vec<u32>,
        windows: Vec<HWND>,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if hwnd.0.is_null() {
            return BOOL(1);
        }
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        if unsafe { GetParent(hwnd) }.is_ok() {
            return BOOL(1);
        }
        let ex = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        if ex & WS_EX_TOOLWINDOW != 0 {
            return BOOL(1);
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32)) };
        if !search.pids.contains(&pid) {
            return BOOL(1);
        }
        search.windows.push(hwnd);
        BOOL(1)
    }

    let mut search = Search {
        pids,
        windows: Vec::new(),
    };
    unsafe {
        let ptr = &mut search as *mut Search;
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(ptr as isize));
    }

    search
        .windows
        .iter()
        .copied()
        .find(|hwnd| window_is_showing(*hwnd))
        .or_else(|| {
            search.windows.into_iter().max_by_key(|hwnd| {
                let mut rect = RECT::default();
                let ok = unsafe { GetWindowRect(*hwnd, &mut rect) };
                if ok.is_ok() {
                    (rect.right - rect.left).max(0) * (rect.bottom - rect.top).max(0)
                } else {
                    0
                }
            })
        })
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
    fn playlist_play_uses_normal_start_not_minimized() {
        let args = windows_start_args("spotify:playlist:abc:play");
        assert_eq!(
            args.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["/C", "start", "", "spotify:playlist:abc:play"]
        );
        assert!(!args.iter().any(|arg| arg == "/MIN"));
    }

    #[test]
    fn playlist_launch_uri_appends_play() {
        assert_eq!(
            playlist_launch_uri("spotify:playlist:abc"),
            "spotify:playlist:abc:play"
        );
        assert_eq!(
            playlist_launch_uri("spotify:playlist:abc:play"),
            "spotify:playlist:abc:play"
        );
    }
}
