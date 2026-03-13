/// Handle Teams-related actions.
/// Real integration with Teams APIs or keyboard shortcuts is future work.
pub fn handle(command: &str) -> Result<(), String> {
    match command {
        "toggleMute" => {
            log::info!("Teams: toggle mute");
            // Future: send Ctrl+Shift+M to Teams window
            Ok(())
        }
        "toggleCamera" => {
            log::info!("Teams: toggle camera");
            // Future: send Ctrl+Shift+O to Teams window
            Ok(())
        }
        "shareScreen" => {
            log::info!("Teams: share screen");
            // Future: send Ctrl+Shift+E to Teams window
            Ok(())
        }
        "raiseHand" => {
            log::info!("Teams: raise hand");
            // Future: send Ctrl+Shift+K to Teams window
            Ok(())
        }
        cmd if cmd.starts_with("reaction.") => {
            let reaction = &cmd["reaction.".len()..];
            log::info!("Teams: sending reaction '{}'", reaction);
            // Future: use Teams API or UI automation to send reaction
            Ok(())
        }
        _ => {
            log::warn!("Unknown Teams command: {}", command);
            Err(format!("Unknown Teams command: {}", command))
        }
    }
}
