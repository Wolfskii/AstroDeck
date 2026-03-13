use crate::plugin_engine::PluginConfig;
use crate::{AppState, SceneResponse};
use tauri::{Emitter, Manager};

/// Given a list of matched plugin IDs from detectors, resolve the highest-priority
/// active scene. Emits a `scene-changed` event if the scene actually changed.
pub fn resolve(app_handle: &tauri::AppHandle, matched_ids: &[String]) {
    let state = app_handle.state::<AppState>();
    let plugins = state.plugins.lock().expect("failed to lock plugins");

    if let Some(override_scene) = state
        .manual_scene_override
        .lock()
        .expect("failed to lock manual override")
        .clone()
    {
        let mut current = state.active_scene_id.lock().expect("failed to lock scene id");
        if *current != override_scene {
            *current = override_scene.clone();
            let layout = crate::layout_engine::get_layout(&override_scene, &plugins);
            let available_scenes: Vec<String> = plugins.iter().map(|p| p.id.clone()).collect();
            let response = SceneResponse {
                active_scene_id: override_scene,
                layout,
                available_scenes,
            };
            if let Err(e) = app_handle.emit("scene-changed", &response) {
                log::error!("Failed to emit manual override scene-changed event: {}", e);
            }
        }
        return;
    }

    let best = plugins
        .iter()
        .filter(|p| matched_ids.contains(&p.id))
        .max_by_key(|p| p.priority);

    let new_scene_id = best.map(|p| p.id.clone()).unwrap_or_else(|| "default".to_string());

    let mut current = state.active_scene_id.lock().expect("failed to lock scene id");
    if *current != new_scene_id {
        log::info!("Scene changed: {} -> {}", *current, new_scene_id);
        *current = new_scene_id.clone();

        let layout = crate::layout_engine::get_layout(&new_scene_id, &plugins);
        let available_scenes: Vec<String> = plugins.iter().map(|p| p.id.clone()).collect();

        let response = SceneResponse {
            active_scene_id: new_scene_id,
            layout,
            available_scenes,
        };

        if let Err(e) = app_handle.emit("scene-changed", &response) {
            log::error!("Failed to emit scene-changed event: {}", e);
        }
    }
}

/// Check if a given plugin's triggers match against the current system state.
pub fn matches_triggers(plugin: &PluginConfig, processes: &[String], _window_title: &str) -> bool {
    if let Some(ref proc) = plugin.triggers.process {
        let proc_lower = proc.to_lowercase();
        let found = processes.iter().any(|p| p.to_lowercase().contains(&proc_lower));
        if !found {
            return false;
        }
    }

    if let Some(ref title_match) = plugin.triggers.window_title_contains {
        if !title_match.is_empty() && !_window_title.to_lowercase().contains(&title_match.to_lowercase()) {
            return false;
        }
    }

    true
}
