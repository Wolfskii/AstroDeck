# Actions

Buttons can execute either legacy string actions or declarative `actionSpec` actions.

## Legacy String Actions

Legacy actions still use the existing format:

```text
namespace.command
```

Examples:

- `spotify.togglePlay`
- `spotify.setVolume`
- `teams.toggleMute`
- `core.settings`

These are dispatched in `src-tauri/src/actions/mod.rs`.

## Value-Based Actions

Some actions also accept a numeric payload through `execute_action_value`.

Current built-in value action:

| Action | Description |
|--------|-------------|
| `spotify.setVolume` | Sets Spotify device volume to `0-100` |

This is used by the reusable media-player scene slider.

## Declarative Plugin Actions

Plugins can define `actionSpec` instead of an explicit string `action`. The loader generates a synthetic runtime action ID and registers it automatically.

Supported declarative kinds:

| Kind | Fields | Behavior |
|------|--------|----------|
| `openUrl` | `url` | Opens a URL using the OS |
| `openPath` | `path` | Opens a local path using the OS |
| `launch` | `program`, `args?` | Starts an external application or executable |

Examples:

```json
{ "label": "Docs", "actionSpec": { "kind": "openUrl", "url": "https://example.com" } }
```

```json
{ "label": "Open Folder", "actionSpec": { "kind": "openPath", "path": "." } }
```

```json
{ "label": "Launch App", "actionSpec": { "kind": "launch", "program": "MyApp.exe", "args": ["--quick"] } }
```

## Built-In Namespaces

| Namespace | Description |
|-----------|-------------|
| `teams` | Teams meeting controls |
| `spotify` | Spotify controls and Spotify Web API integration |
| `core` | App-level actions such as settings and refresh |

## Spotify Actions

| Action | Description |
|--------|-------------|
| `spotify.togglePlay` | Toggle play/pause |
| `spotify.nextTrack` | Next track |
| `spotify.prevTrack` | Previous track |
| `spotify.like` | Toggle saved-track state in the user library |
| `spotify.setVolume` | Set Spotify device volume using a slider/value payload |

## Notes

- String actions remain the compatibility path for existing built-in plugins.
- Declarative actions are the preferred path for user-authored plugins that should execute real OS behavior without needing new Rust code.
