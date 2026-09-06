mod spotify_detector;
mod teams_detector;
mod vscode_detector;

use std::time::Duration;
use sysinfo::System;
use tauri::Manager;

#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
};

pub trait Detector: Send + Sync {
    fn id(&self) -> &str;
    fn detect(&self, system: &System) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct DetectionContext {
    pub processes: Vec<String>,
    pub window_titles: Vec<String>,
}

fn all_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(teams_detector::TeamsDetector),
        Box::new(spotify_detector::SpotifyDetector),
        Box::new(vscode_detector::VscodeDetector),
    ]
}

pub fn start_detection_loop(app_handle: tauri::AppHandle) {
    let interval_ms: u64 = std::env::var("DETECTOR_INTERVAL")
        .unwrap_or_else(|_| "2000".to_string())
        .parse()
        .unwrap_or(2000);

    std::thread::spawn(move || {
        let mut system = System::new_all();
        let detectors = all_detectors();

        loop {
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

            let context = collect_detection_context(&system);

            let state = app_handle.state::<crate::AppState>();
            let plugins = state.plugins.lock().expect("failed to lock plugins");

            let mut matched_ids: Vec<String> = Vec::new();

            for plugin in plugins.iter() {
                let detector_match = detectors.iter().any(|d| {
                    d.id() == plugin.id && d.detect(&system)
                });

                if crate::mode_engine::should_match_plugin(
                    plugin,
                    detector_match,
                    &context,
                ) {
                    matched_ids.push(plugin.id.clone());
                }
            }

            let has_media_plugin = plugins.iter().any(|plugin| plugin.id == "media");
            drop(plugins);

            if has_media_plugin && crate::os_media::has_local_session(&app_handle) {
                let local_playing = crate::os_media::current_local(&state)
                    .is_some_and(|payload| payload.is_playing);
                let spotify_present = matched_ids.iter().any(|id| id == "spotify");
                if (local_playing || !spotify_present)
                    && !matched_ids.iter().any(|id| id == "media")
                {
                    matched_ids.push("media".to_string());
                }
            }

            if let Ok(mut last) = state.last_matched_ids.lock() {
                *last = matched_ids.clone();
            }

            crate::mode_engine::resolve(&app_handle, &matched_ids);

            std::thread::sleep(Duration::from_millis(interval_ms));
        }
    });
}

fn collect_detection_context(system: &System) -> DetectionContext {
    let mut processes: Vec<String> = Vec::new();

    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_string();
        processes.push(name);

        if let Some(exe_name) = process
            .exe()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
        {
            processes.push(exe_name.to_string());
        }
    }

    let (window_titles, _focused_window_title) = collect_window_titles();

    DetectionContext {
        processes,
        window_titles,
    }
}

#[cfg(windows)]
fn collect_window_titles() -> (Vec<String>, Option<String>) {
    let mut titles = Vec::<String>::new();

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }

        let title = read_window_title(hwnd);
        if let Some(title) = title {
            let titles = unsafe { &mut *(lparam.0 as *mut Vec<String>) };
            titles.push(title);
        }

        BOOL(1)
    }

    unsafe {
        let titles_ptr = &mut titles as *mut Vec<String>;
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(titles_ptr as isize));
    }

    let focused = read_window_title(unsafe { GetForegroundWindow() });
    (titles, focused)
}

#[cfg(windows)]
fn read_window_title(hwnd: HWND) -> Option<String> {
    if hwnd.0.is_null() {
        return None;
    }

    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return None;
    }

    let mut buffer = vec![0u16; length as usize + 1];
    let written = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if written <= 0 {
        return None;
    }

    let title = String::from_utf16_lossy(&buffer[..written as usize]);
    let trimmed = title.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(not(windows))]
fn collect_window_titles() -> (Vec<String>, Option<String>) {
    (Vec::new(), None)
}
