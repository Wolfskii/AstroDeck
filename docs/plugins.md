# Plugins

Plugins define scenes, triggers, buttons, optional scene templates, and executable actions.

## Plugin Locations

TapTapDeck loads plugins from two places at startup:

- Built-in plugins from the repo `plugins/` folder in dev and the bundled `plugins/` resources in packaged builds.
- User plugins from `~/TapTapDeck/plugins/`.

If a user plugin has the same `id` as a built-in plugin, the user plugin overrides the built-in one.

## Directory-Based Plugins

Plugins can now live in their own folder so they can carry icons or other assets beside the manifest:

```text
plugins/
  spotify/
    plugin.json
    icon.png
```

Legacy flat manifests like `plugins/my-plugin.json` are still supported during migration.

## Manifest Schema

```json
{
  "id": "my-media-plugin",
  "name": "My Media Plugin",
  "description": "Example plugin with local assets and declarative actions.",
  "schemaVersion": 1,
  "priority": 70,
  "triggers": {
    "processGlob": "*spotify*",
    "windowTitleContains": "Now Playing"
  },
  "view": {
    "type": "mediaPlayer",
    "mediaPlayer": {
      "previous": { "label": "Previous", "image": "prev.png", "action": "spotify.prevTrack" },
      "playPause": { "label": "Play/Pause", "image": "play.png", "action": "spotify.togglePlay" },
      "next": { "label": "Next", "image": "next.png", "action": "spotify.nextTrack" },
      "like": { "label": "Favorite", "emoji": "❤️", "action": "spotify.like" },
      "volumeAction": "spotify.setVolume"
    }
  },
  "layout": {
    "grid": [2, 2],
    "buttons": [
      { "label": "Previous", "emoji": "⏮️", "action": "spotify.prevTrack" },
      { "label": "Play/Pause", "emoji": "⏯️", "action": "spotify.togglePlay" },
      { "label": "Next", "emoji": "⏭️", "action": "spotify.nextTrack" },
      { "label": "Favorite", "emoji": "❤️", "action": "spotify.like" }
    ]
  }
}
```

## Core Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique scene/plugin ID |
| `name` | string | Display name |
| `description` | string? | Optional description used in settings/debug UI |
| `schemaVersion` | number | Schema version, currently `1` |
| `priority` | number | Higher priority wins when multiple plugins match |
| `triggers` | object | Process and window matching rules |
| `view` | object? | Optional template-driven scene renderer |
| `layout` | object | Generic grid fallback and metadata source |

## Button Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string? | Optional stable ID for declarative actions |
| `label` | string | Button text |
| `emoji` | string? | Emoji icon if no image is used |
| `image` | string? | Plugin-relative asset path, remote URL, or data URL |
| `action` | string | Legacy string action such as `spotify.nextTrack` |
| `actionSpec` | object? | Declarative executable action, used when no explicit `action` is provided |

## Declarative Actions

Declarative actions are resolved by the backend into a synthetic runtime action ID and executed without custom Rust code in the plugin itself.

Supported kinds:

```json
{ "kind": "openUrl", "url": "https://example.com" }
{ "kind": "openPath", "path": "docs/readme.md" }
{ "kind": "launch", "program": "C:/Program Files/App/app.exe", "args": ["--flag"] }
```

Relative `openPath` and `launch.program` values are resolved relative to the plugin folder.

## Images And Assets

- Relative `image` paths are resolved relative to the plugin folder.
- Local images are embedded as data URLs when the plugin is loaded, so they work in both the desktop window and the browser debug/settings panel.
- This lets plugin authors ship icons without needing hardcoded absolute paths.

## Scene Templates

`view.type` controls how a scene is rendered.

Current template types:

- `grid`: default grid renderer using `layout.grid` and `layout.buttons`
- `mediaPlayer`: a dedicated media layout with `previous`, `playPause`, `next`, optional `like`, and optional `volumeAction`

See [layouts.md](layouts.md) for details.

## Creating A Plugin

1. Create a folder under either built-in `plugins/` or external `~/TapTapDeck/plugins/`.
2. Add `plugin.json`.
3. Add optional local images in the same folder.
4. Define triggers and either legacy `action` strings or declarative `actionSpec`.
5. Restart TapTapDeck.

## Notes

- Built-in scenes and old flat manifests still work.
- Hot reload is still planned, not implemented.
