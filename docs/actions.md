# Actions

Actions are the commands invoked when a deck button is pressed. They use a `"namespace.command"` format and are dispatched by the Rust backend.

## Action Format

```
namespace.command
```

Examples: `spotify.togglePlay`, `teams.toggleMute`, `core.settings`

## Dispatcher

- **Location**: `src-tauri/src/actions/mod.rs`
- **Function**: `dispatch(action: &str) -> Result<(), String>`
- Splits on first `.`; routes by `namespace` prefix.
- Unknown namespaces return `Err`.

## Built-in Namespaces

| Namespace | Module | Description |
|-----------|--------|-------------|
| `teams` | `teams_actions.rs` | Teams meeting controls |
| `spotify` | `spotify_actions.rs` | Spotify control |
| `core` | inline in `mod.rs` | App settings, plugins, refresh, info |

*Note: `vscode` namespace is planned (Phase 2); plugin exists but actions are not yet wired.*

## Teams Actions

| Command | Description |
|---------|-------------|
| `toggleMute` | Ctrl+Shift+M (stub) |
| `toggleCamera` | Ctrl+Shift+O (stub) |
| `shareScreen` | Ctrl+Shift+E (stub) |
| `raiseHand` | Ctrl+Shift+K (stub) |
| `reaction.like` | Send 👍 reaction |
| `reaction.heart` | Send ❤️ reaction |
| `reaction.clap` | Send 👏 reaction |
| `reaction.laugh` | Send 😂 reaction |
| `reaction.wow` | Send 😮 reaction |

*Real integration: keyboard shortcut injection or Teams API — future work.*

## Spotify Actions

| Command | Description |
|---------|-------------|
| `togglePlay` | Play/pause |
| `nextTrack` | Next track |
| `prevTrack` | Previous track |
| `volumeUp` | Volume up |
| `volumeDown` | Volume down |
| `like` | Like current track |

*Real integration: Spotify Web API or D-Bus/media controls — future work.*

## Core Actions

| Command | Description |
|---------|-------------|
| `settings` | Open settings |
| `plugins` | List plugins |
| `refresh` | Refresh scene |
| `info` | Show app info |

## Adding New Action Handlers

1. Create `src-tauri/src/actions/my_namespace_actions.rs`:

```rust
pub fn handle(command: &str) -> Result<(), String> {
    match command {
        "doSomething" => {
            log::info!("MyNamespace: do something");
            Ok(())
        }
        _ => Err(format!("Unknown command: {}", command)),
    }
}
```

2. Add module and match arm in `actions/mod.rs`:

```rust
mod my_namespace_actions;

// In dispatch():
match namespace {
    "teams" => teams_actions::handle(command),
    "spotify" => spotify_actions::handle(command),
    "my_namespace" => my_namespace_actions::handle(command),
    "core" => handle_core(command),
    _ => { ... }
}
```

3. Use in plugin JSON: `"action": "my_namespace.doSomething"`
