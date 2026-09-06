use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

/// Geometry and chrome we persist. Visibility is excluded so the tray-first
/// window stays hidden on launch (including OS login autostart).
pub fn tracked_flags() -> StateFlags {
    StateFlags::SIZE
        | StateFlags::POSITION
        | StateFlags::MAXIMIZED
        | StateFlags::FULLSCREEN
        | StateFlags::DECORATIONS
}

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_window_state::Builder::new()
        .with_state_flags(tracked_flags())
        .skip_initial_state("main")
        .with_denylist(&["settings"])
        .build()
}

/// Restore monitor, position, and size while the window stays hidden.
/// Maximized/fullscreen are applied later when the user shows the window,
/// so restore cannot steal focus or flash a window on login.
pub fn restore_hidden_geometry(window: &tauri::WebviewWindow) {
    let flags = StateFlags::SIZE | StateFlags::POSITION | StateFlags::DECORATIONS;
    if let Err(e) = window.restore_state(flags) {
        log::warn!("Failed to restore window geometry: {e}");
    }
    let _ = window.hide();
}

pub fn restore_show_state(window: &tauri::WebviewWindow) {
    let flags = StateFlags::SIZE
        | StateFlags::POSITION
        | StateFlags::MAXIMIZED
        | StateFlags::FULLSCREEN
        | StateFlags::DECORATIONS;
    if let Err(e) = window.restore_state(flags) {
        log::warn!("Failed to restore window show state: {e}");
    }
}

pub fn persist(app: &tauri::AppHandle) {
    if let Err(e) = app.save_window_state(tracked_flags()) {
        log::warn!("Failed to save window state: {e}");
    }
}
