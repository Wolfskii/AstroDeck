use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

/// Geometry and chrome we persist. Visibility is not saved; launch show/hide
/// is controlled by the start-minimized preference.
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

/// Restore last monitor, position, size, and chrome.
/// If `start_minimized` is set, keep the window hidden in the tray.
/// Otherwise show it with the last maximized/fullscreen state.
pub fn apply_launch_state(window: &tauri::WebviewWindow, start_minimized: bool) {
    let flags = if start_minimized {
        StateFlags::SIZE | StateFlags::POSITION | StateFlags::DECORATIONS
    } else {
        tracked_flags()
    };
    if let Err(e) = window.restore_state(flags) {
        log::warn!("Failed to restore window state: {e}");
    }
    if start_minimized {
        let _ = window.hide();
        return;
    }
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
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
