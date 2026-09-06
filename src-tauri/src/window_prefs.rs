use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

/// Geometry and chrome we persist. Visibility is not saved; launch show/hide
/// is controlled by the start-minimized preference.
pub fn tracked_flags() -> StateFlags {
    StateFlags::SIZE
        | StateFlags::POSITION
        | StateFlags::MAXIMIZED
        | StateFlags::DECORATIONS
}

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_window_state::Builder::new()
        .with_state_flags(tracked_flags())
        .skip_initial_state("main")
        .with_denylist(&["settings"])
        .build()
}

/// Restore last monitor, position, size, and chrome, then stay in the tray.
/// The deck window is shown later when VS Code, Cursor, Teams, Spotify, or
/// local OS media is detected (unless start-minimized is enabled).
pub fn apply_launch_state(window: &tauri::WebviewWindow, _start_minimized: bool, start_fullscreen: bool) {
    let flags = StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::DECORATIONS;
    if let Err(e) = window.restore_state(flags) {
        log::warn!("Failed to restore window state: {e}");
    }
    apply_fullscreen(window, start_fullscreen);
    let _ = window.hide();
}

pub fn restore_show_state(window: &tauri::WebviewWindow, start_fullscreen: bool) {
    let flags = StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::DECORATIONS;
    if let Err(e) = window.restore_state(flags) {
        log::warn!("Failed to restore window show state: {e}");
    }
    apply_fullscreen(window, start_fullscreen);
}

fn apply_fullscreen(window: &tauri::WebviewWindow, start_fullscreen: bool) {
    if start_fullscreen {
        let _ = window.set_decorations(false);
        let _ = window.set_fullscreen(true);
    } else {
        let _ = window.set_fullscreen(false);
        let _ = window.set_decorations(true);
    }
}

pub fn persist(app: &tauri::AppHandle) {
    if let Err(e) = app.save_window_state(tracked_flags()) {
        log::warn!("Failed to save window state: {e}");
    }
}
