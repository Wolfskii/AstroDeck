# Plugins

Plugins are JSON files that define scenes: when they activate (triggers) and what layout they show. They are loaded from the `plugins/` directory on startup.

## Plugin Schema

```json
{
  "id": "string",
  "name": "string",
  "priority": 0,
  "triggers": {
    "process": "optional substring to match",
    "windowTitleContains": "optional window title substring"
  },
  "layout": {
    "grid": [rows, cols],
    "buttons": [
      {
        "label": "string",
        "emoji": "optional string",
        "image": "optional string",
        "action": "namespace.command"
      }
    ]
  }
}
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique plugin ID; used when a detector matches and for layout lookup |
| `name` | string | Human-readable name |
| `priority` | number | Higher wins when multiple plugins match; default 0 |
| `triggers` | object | Optional: `process` (process name substring), `windowTitleContains` |
| `layout` | object | Grid dimensions and button array |

## Full Example

```json
{
  "id": "spotify",
  "name": "Spotify Control",
  "priority": 60,
  "triggers": {
    "process": "Spotify"
  },
  "layout": {
    "grid": [2, 3],
    "buttons": [
      { "label": "Play/Pause", "emoji": "⏯️", "action": "spotify.togglePlay" },
      { "label": "Next Track", "emoji": "⏭️", "action": "spotify.nextTrack" },
      { "label": "Prev Track", "emoji": "⏮️", "action": "spotify.prevTrack" },
      { "label": "Volume Up", "emoji": "🔊", "action": "spotify.volumeUp" },
      { "label": "Volume Down", "emoji": "🔉", "action": "spotify.volumeDown" },
      { "label": "Like", "emoji": "❤️", "action": "spotify.like" }
    ]
  }
}
```

## Plugin Loading

- **Directory**: `plugins/` (relative to resource dir, or `./plugins`, `../plugins`).
- **Scan**: On startup, all `.json` files in the directory are loaded.
- **Order**: Plugins are sorted by `priority` (descending) after load.
- **Errors**: Invalid JSON or schema is logged; plugin is skipped.

## Creating a New Plugin

1. Create `plugins/my-plugin.json` with valid schema.
2. Add a detector (optional) or rely on triggers.
3. Restart the app (or use hot-reload when implemented).
4. Ensure the plugin's `action` strings use existing namespaces (e.g. `core.*`) or add handlers in `actions/`.

## Action String Format

Actions use `"namespace.command"` (e.g. `"spotify.togglePlay"`). The dispatcher in `actions/mod.rs` routes by namespace. See [actions.md](actions.md).

## Hot-Reload (Planned)

- File watcher via `notify` crate.
- Reload plugins when JSON files change.
