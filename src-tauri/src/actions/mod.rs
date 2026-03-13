mod spotify_actions;
mod teams_actions;

/// Dispatch an action string to the appropriate handler.
/// Action format: "namespace.command" (e.g. "spotify.togglePlay", "teams.toggleMute").
pub fn dispatch(action: &str, _app: &tauri::AppHandle, state: &crate::AppState) -> Result<(), String> {
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
            if let Ok(status) = crate::spotify::get_status(&state.spotify) {
                let payload = serde_json::json!({
                    "type": "spotifyStatus",
                    "payload": status,
                });
                let _ = state.log_bus.send(payload.to_string());
            }
            Ok(())
        }
        "core" => handle_core(command),
        _ => {
            log::warn!("Unknown action namespace: {}", namespace);
            Err(format!("Unknown action namespace: {}", namespace))
        }
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
