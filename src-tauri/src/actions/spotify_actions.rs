#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SendMessageW, WM_APPCOMMAND,
};
#[cfg(windows)]
use windows::Win32::Foundation::{LPARAM, WPARAM};

#[cfg(target_os = "macos")]
use media_remote::prelude::*;

#[cfg(windows)]
const APPCOMMAND_MEDIA_NEXTTRACK: u16 = 11;
#[cfg(windows)]
const APPCOMMAND_MEDIA_PREVIOUSTRACK: u16 = 12;
#[cfg(windows)]
const APPCOMMAND_MEDIA_PLAY_PAUSE: u16 = 14;
#[cfg(windows)]
const APPCOMMAND_VOLUME_DOWN: u16 = 9;
#[cfg(windows)]
const APPCOMMAND_VOLUME_UP: u16 = 10;

#[cfg(windows)]
fn send_app_command(command: u16) -> Result<(), String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err("No foreground window is available for media command dispatch".to_string());
    }

    let lparam = LPARAM((command as isize) << 16);
    unsafe {
        SendMessageW(hwnd, WM_APPCOMMAND, WPARAM(hwnd.0 as usize), lparam);
    }
    Ok(())
}

#[cfg(windows)]
fn handle_windows(command: &str) -> Result<(), String> {
    match command {
        "togglePlay" => {
            log::info!("Spotify: sending OS media play/pause");
            send_app_command(APPCOMMAND_MEDIA_PLAY_PAUSE)
        }
        "nextTrack" => {
            log::info!("Spotify: sending OS media next track");
            send_app_command(APPCOMMAND_MEDIA_NEXTTRACK)
        }
        "prevTrack" => {
            log::info!("Spotify: sending OS media previous track");
            send_app_command(APPCOMMAND_MEDIA_PREVIOUSTRACK)
        }
        "volumeUp" => {
            log::info!("Spotify: sending OS volume up");
            send_app_command(APPCOMMAND_VOLUME_UP)
        }
        "volumeDown" => {
            log::info!("Spotify: sending OS volume down");
            send_app_command(APPCOMMAND_VOLUME_DOWN)
        }
        "like" => {
            log::warn!("Spotify: like current track is not available via generic OS media controls");
            Err("spotify.like requires Spotify-specific integration and is not available via generic OS media controls".to_string())
        }
        _ => {
            log::warn!("Unknown Spotify command: {}", command);
            Err(format!("Unknown Spotify command: {}", command))
        }
    }
}

#[cfg(target_os = "macos")]
fn adjust_macos_volume(delta: i32) -> Result<(), String> {
    let script = if delta >= 0 {
        format!(
            "set volume output volume ((output volume of (get volume settings)) + {})",
            delta
        )
    } else {
        format!(
            "set volume output volume ((output volume of (get volume settings)) - {})",
            -delta
        )
    };

    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("osascript exited with status {}", status))
    }
}

#[cfg(target_os = "macos")]
fn handle_macos(command: &str) -> Result<(), String> {
    let now_playing = NowPlaying::new();
    match command {
        "togglePlay" => {
            log::info!("Spotify: sending macOS play/pause");
            if now_playing.toggle() {
                Ok(())
            } else {
                Err("Failed to send macOS play/pause command".to_string())
            }
        }
        "nextTrack" => {
            log::info!("Spotify: sending macOS next track");
            if now_playing.next() {
                Ok(())
            } else {
                Err("Failed to send macOS next track command".to_string())
            }
        }
        "prevTrack" => {
            log::info!("Spotify: sending macOS previous track");
            if now_playing.previous() {
                Ok(())
            } else {
                Err("Failed to send macOS previous track command".to_string())
            }
        }
        "volumeUp" => {
            log::info!("Spotify: sending macOS volume up");
            adjust_macos_volume(6)
        }
        "volumeDown" => {
            log::info!("Spotify: sending macOS volume down");
            adjust_macos_volume(-6)
        }
        "like" => Err(
            "spotify.like requires Spotify-specific integration and is not available via generic OS media controls"
                .to_string(),
        ),
        _ => Err(format!("Unknown Spotify command: {}", command)),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn handle_unsupported(command: &str) -> Result<(), String> {
    Err(format!(
        "Spotify action '{}' is not implemented for this operating system",
        command
    ))
}

/// Handle Spotify-related actions.
pub fn handle(command: &str, spotify: &crate::spotify::SpotifyState) -> Result<(), String> {
    #[cfg(windows)]
    {
        match command {
            "volumeUp" => {
                let next = crate::spotify::adjust_volume(spotify, 6)?;
                log::info!("Spotify: set device volume to {}%", next);
                return Ok(());
            }
            "volumeDown" => {
                let next = crate::spotify::adjust_volume(spotify, -6)?;
                log::info!("Spotify: set device volume to {}%", next);
                return Ok(());
            }
            "like" => {
                log::info!("Spotify: invoking track library toggle");
                let now_saved = crate::spotify::toggle_current_track_saved(spotify)?;
                log::info!(
                    "Spotify: current track is now {} the library",
                    if now_saved { "saved to" } else { "removed from" }
                );
                return Ok(());
            }
            "toggleShuffle" => {
                let next = crate::spotify::toggle_shuffle(spotify)?;
                log::info!("Spotify: shuffle {}", if next { "on" } else { "off" });
                return Ok(());
            }
            "togglePlay" | "nextTrack" | "prevTrack" => {
                let result = handle_windows(command);
                if result.is_ok() {
                    crate::spotify::invalidate_playback_cache(spotify);
                }
                return result;
            }
            _ => return handle_windows(command),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match command {
            "volumeUp" => {
                let next = crate::spotify::adjust_volume(spotify, 6)?;
                log::info!("Spotify: set device volume to {}%", next);
                return Ok(());
            }
            "volumeDown" => {
                let next = crate::spotify::adjust_volume(spotify, -6)?;
                log::info!("Spotify: set device volume to {}%", next);
                return Ok(());
            }
            "like" => {
                log::info!("Spotify: invoking track library toggle");
                let now_saved = crate::spotify::toggle_current_track_saved(spotify)?;
                log::info!(
                    "Spotify: current track is now {} the library",
                    if now_saved { "saved to" } else { "removed from" }
                );
                return Ok(());
            }
            "toggleShuffle" => {
                let next = crate::spotify::toggle_shuffle(spotify)?;
                log::info!("Spotify: shuffle {}", if next { "on" } else { "off" });
                return Ok(());
            }
            "togglePlay" | "nextTrack" | "prevTrack" => {
                let result = handle_macos(command);
                if result.is_ok() {
                    crate::spotify::invalidate_playback_cache(spotify);
                }
                return result;
            }
            _ => return handle_macos(command),
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        return handle_unsupported(command);
    }
}

pub fn handle_value(
    command: &str,
    value: serde_json::Value,
    spotify: &crate::spotify::SpotifyState,
) -> Result<(), String> {
    match command {
        "setVolume" => {
            let volume = value
                .as_u64()
                .ok_or_else(|| "spotify.setVolume expects a numeric value".to_string())?
                .clamp(0, 100) as u8;
            let next = crate::spotify::set_volume(spotify, volume, None)?;
            log::info!("Spotify: set device volume to {}%", next);
            Ok(())
        }
        "seek" => {
            let position_ms = value
                .as_u64()
                .ok_or_else(|| "spotify.seek expects a numeric position in milliseconds".to_string())?;
            crate::spotify::seek(spotify, position_ms)?;
            log::info!("Spotify: seek to {} ms", position_ms);
            Ok(())
        }
        "like" => {
            let should_save = value.as_bool().ok_or_else(|| {
                "spotify.like expects a boolean saved state (true = save, false = remove)".to_string()
            })?;
            log::info!("Spotify: invoking track library update (should_save={})", should_save);
            let now_saved = crate::spotify::set_current_track_saved(spotify, should_save)?;
            log::info!(
                "Spotify: current track is now {} the library",
                if now_saved { "saved to" } else { "removed from" }
            );
            Ok(())
        }
        "playPlaylist" => {
            let playlist = value
                .as_str()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    "spotify.playPlaylist expects a playlist id or spotify:playlist URI".to_string()
                })?;
            crate::spotify::play_playlist(spotify, playlist)?;
            log::info!("Spotify: started playlist {playlist}");
            Ok(())
        }
        _ => Err(format!(
            "Spotify action '{}' does not support a value payload",
            command
        )),
    }
}
