use crate::plugin_engine::PluginConfig;
use crate::{AppState, SceneResponse};
use tauri::{Emitter, Manager};

fn is_presence_scene(id: &str) -> bool {
    matches!(id, "teams" | "spotify" | "vscode" | "media")
}

fn emit_scene(app_handle: &tauri::AppHandle, scene_id: String, plugins: &[PluginConfig]) {
    let layout = crate::layout_engine::get_layout(&scene_id, plugins);
    let available_scenes: Vec<String> = plugins.iter().map(|p| p.id.clone()).collect();
    let response = SceneResponse {
        active_scene_id: scene_id,
        layout,
        available_scenes,
    };
    if let Err(e) = app_handle.emit("scene-changed", &response) {
        log::error!("Failed to emit scene-changed event: {}", e);
    }
}

/// Given a list of matched plugin IDs from detectors, resolve the highest-priority
/// active scene. Emits a `scene-changed` event if the scene actually changed.
pub fn resolve(app_handle: &tauri::AppHandle, matched_ids: &[String]) {
    let state = app_handle.state::<AppState>();
    let plugins = state.plugins.lock().expect("failed to lock plugins");
    let has_presence = matched_ids.iter().any(|id| is_presence_scene(id));

    if !has_presence {
        if let Ok(mut override_state) = state.manual_scene_override.lock() {
            *override_state = None;
        }
    } else if let Some(override_scene) = state
        .manual_scene_override
        .lock()
        .expect("failed to lock manual override")
        .clone()
    {
        let mut current = state.active_scene_id.lock().expect("failed to lock scene id");
        if *current != override_scene {
            *current = override_scene.clone();
            emit_scene(app_handle, override_scene, &plugins);
        }
        return;
    }

    let best = plugins
        .iter()
        .filter(|p| matched_ids.contains(&p.id))
        .max_by_key(|p| p.priority);

    let new_scene_id = best.map(|p| p.id.clone()).unwrap_or_else(|| "idle".to_string());

    let mut current = state.active_scene_id.lock().expect("failed to lock scene id");
    if *current != new_scene_id {
        log::info!("Scene changed: {} -> {}", *current, new_scene_id);
        *current = new_scene_id.clone();
        emit_scene(app_handle, new_scene_id, &plugins);
    }
}

/// Check if a given plugin's triggers match against the current system state.
pub fn matches_triggers(
    plugin: &PluginConfig,
    context: &crate::detectors::DetectionContext,
) -> bool {
    if let Some(ref proc) = plugin.triggers.process {
        if !context
            .processes
            .iter()
            .any(|p| p.to_lowercase().contains(&proc.to_lowercase()))
        {
            return false;
        }
    }

    if let Some(ref pattern) = plugin.triggers.process_glob {
        if !context
            .processes
            .iter()
            .any(|value| matches_glob(pattern, value))
        {
            return false;
        }
    }

    if !plugin.triggers.processes_any.is_empty()
        && !plugin
            .triggers
            .processes_any
            .iter()
            .any(|pattern| context.processes.iter().any(|value| matches_glob(pattern, value)))
    {
        return false;
    }

    if !plugin.triggers.processes_all.is_empty()
        && !plugin
            .triggers
            .processes_all
            .iter()
            .all(|pattern| context.processes.iter().any(|value| matches_glob(pattern, value)))
    {
        return false;
    }

    if plugin
        .triggers
        .exclude_processes
        .iter()
        .any(|pattern| context.processes.iter().any(|value| matches_glob(pattern, value)))
    {
        return false;
    }

    if let Some(ref title_match) = plugin.triggers.window_title_contains {
        let wanted = title_match.to_lowercase();
        if !context
            .window_titles
            .iter()
            .any(|title| title.to_lowercase().contains(&wanted))
        {
            return false;
        }
    }

    if let Some(ref pattern) = plugin.triggers.window_title_glob {
        if !context
            .window_titles
            .iter()
            .any(|title| matches_glob(pattern, title))
        {
            return false;
        }
    }

    if !plugin.triggers.window_titles_any.is_empty()
        && !plugin
            .triggers
            .window_titles_any
            .iter()
            .any(|pattern| context.window_titles.iter().any(|title| matches_glob(pattern, title)))
    {
        return false;
    }

    true
}

pub fn should_match_plugin(
    plugin: &PluginConfig,
    detector_match: bool,
    context: &crate::detectors::DetectionContext,
) -> bool {
    if plugin.triggers.has_rules() {
        return matches_triggers(plugin, context);
    }

    detector_match
}

fn matches_glob(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let value = value.to_lowercase();
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();

    let mut dp = vec![vec![false; value.len() + 1]; pattern.len() + 1];
    dp[0][0] = true;

    for i in 1..=pattern.len() {
        if pattern[i - 1] == b'*' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=pattern.len() {
        for j in 1..=value.len() {
            dp[i][j] = match pattern[i - 1] {
                b'*' => dp[i - 1][j] || dp[i][j - 1],
                b'?' => dp[i - 1][j - 1],
                other => dp[i - 1][j - 1] && other == value[j - 1],
            };
        }
    }

    dp[pattern.len()][value.len()]
}
