mod spotify_actions;
mod teams_actions;

use serde_json::Value;
use std::process::Command;

/// Dispatch an action string to the appropriate handler.
/// Action format: "namespace.command" (e.g. "spotify.togglePlay", "teams.toggleMute").
pub fn dispatch(action: &str, _app: &tauri::AppHandle, state: &crate::AppState) -> Result<(), String> {
    if let Some(action_spec) = state
        .plugin_actions
        .lock()
        .map_err(|e| e.to_string())?
        .get(action)
        .cloned()
    {
        return execute_plugin_action(action, &action_spec);
    }

    let parts: Vec<&str> = action.splitn(2, '.').collect();
    if parts.len() < 2 {
        return Err(format!("Invalid action format: {}", action));
    }

    let namespace = parts[0];
    let command = parts[1];

    match namespace {
        "teams" => teams_actions::handle(command),
        "spotify" => {
            spotify_actions::handle(command, &state.spotify)?;
            Ok(())
        }
        "core" => handle_core(command),
        _ => {
            log::warn!("Unknown action namespace: {}", namespace);
            Err(format!("Unknown action namespace: {}", namespace))
        }
    }
}

pub fn dispatch_value(
    action: &str,
    value: Value,
    _app: &tauri::AppHandle,
    state: &crate::AppState,
) -> Result<(), String> {
    let parts: Vec<&str> = action.splitn(2, '.').collect();
    if parts.len() < 2 {
        return Err(format!("Invalid action format: {}", action));
    }

    let namespace = parts[0];
    let command = parts[1];

    match namespace {
        "spotify" => {
            spotify_actions::handle_value(command, value, &state.spotify)?;
            Ok(())
        }
        _ => Err(format!(
            "Action '{}' does not support value payload execution",
            action
        )),
    }
}

fn handle_core(command: &str) -> Result<(), String> {
    match command {
        "settings" => {
            log::info!("Core: opening settings");
            Ok(())
        }
        "plugins" => {
            log::info!("Core: listing plugins");
            Ok(())
        }
        "refresh" => {
            log::info!("Core: refreshing scene");
            Ok(())
        }
        "info" => {
            log::info!("Core: showing info");
            Ok(())
        }
        _ => {
            log::warn!("Unknown core command: {}", command);
            Err(format!("Unknown core command: {}", command))
        }
    }
}

fn execute_plugin_action(
    action_id: &str,
    action_spec: &crate::plugin_engine::ActionSpec,
) -> Result<(), String> {
    match action_spec {
        crate::plugin_engine::ActionSpec::OpenUrl { url } => {
            log::info!("Plugin action {}: opening URL {}", action_id, url);
            open_with_os(url)
        }
        crate::plugin_engine::ActionSpec::OpenPath { path } => {
            log::info!("Plugin action {}: opening path {}", action_id, path);
            open_path_with_os(path)
        }
        crate::plugin_engine::ActionSpec::Launch { program, args } => {
            log::info!(
                "Plugin action {}: launching '{}' with {} args",
                action_id,
                program,
                args.len()
            );
            Command::new(program)
                .args(args)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }
}

#[cfg(windows)]
fn open_with_os(target: &str) -> Result<(), String> {
    Command::new("cmd")
        .args(["/C", "start", "", target])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn open_with_os(target: &str) -> Result<(), String> {
    Command::new("open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_with_os(target: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn open_path_with_os(target: &str) -> Result<(), String> {
    Command::new("explorer")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn open_path_with_os(target: &str) -> Result<(), String> {
    open_with_os(target)
}
