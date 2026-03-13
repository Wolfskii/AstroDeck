mod default_detector;
mod spotify_detector;
mod teams_detector;
mod vscode_detector;

use sysinfo::System;
use std::time::Duration;
use tauri::Manager;

pub trait Detector: Send + Sync {
    fn id(&self) -> &str;
    fn detect(&self, system: &System) -> bool;
}

fn all_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(teams_detector::TeamsDetector),
        Box::new(spotify_detector::SpotifyDetector),
        Box::new(vscode_detector::VscodeDetector),
        Box::new(default_detector::DefaultDetector),
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

            let process_names: Vec<String> = system
                .processes()
                .values()
                .map(|p| p.name().to_string_lossy().to_string())
                .collect();

            let state = app_handle.state::<crate::AppState>();
            let plugins = state.plugins.lock().expect("failed to lock plugins");

            let mut matched_ids: Vec<String> = Vec::new();

            for plugin in plugins.iter() {
                let detector_match = detectors.iter().any(|d| {
                    d.id() == plugin.id && d.detect(&system)
                });

                let trigger_match = crate::mode_engine::matches_triggers(
                    plugin,
                    &process_names,
                    "", // window title detection is platform-specific; stubbed for now
                );

                if detector_match || trigger_match {
                    matched_ids.push(plugin.id.clone());
                }
            }

            drop(plugins);
            crate::mode_engine::resolve(&app_handle, &matched_ids);

            std::thread::sleep(Duration::from_millis(interval_ms));
        }
    });
}
