# Layouts

TapTapDeck now supports both generic grid layouts and template-driven scene layouts.

## Grid Layout

Grid layouts remain the default rendering path.

```json
{
  "layout": {
    "grid": [2, 3],
    "buttons": [
      { "label": "Play", "emoji": "⏯️", "action": "spotify.togglePlay" },
      { "label": "Next", "emoji": "⏭️", "action": "spotify.nextTrack" }
    ]
  }
}
```

`layout.grid` is still a `[rows, cols]` tuple and `layout.buttons` still defines the generic button grid.

## Media Player Template

Plugins can opt into a dedicated media-player scene:

```json
{
  "view": {
    "type": "mediaPlayer",
    "mediaPlayer": {
      "previous": { "label": "Previous", "emoji": "⏮️", "action": "spotify.prevTrack" },
      "playPause": { "label": "Play/Pause", "emoji": "⏯️", "action": "spotify.togglePlay" },
      "next": { "label": "Next", "emoji": "⏭️", "action": "spotify.nextTrack" },
      "like": { "label": "Like", "emoji": "❤️", "action": "spotify.like" },
      "volumeAction": "spotify.setVolume"
    }
  }
}
```

The template renders:

- Previous on the left
- Play/Pause centered and larger
- Next on the right
- Optional Like/Favorite below
- Optional touch-friendly volume slider when `volumeAction` is provided

## Button Definitions

Template buttons use the same button schema as grid buttons:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string? | Optional stable button ID |
| `label` | string | Text label |
| `emoji` | string? | Emoji fallback icon |
| `image` | string? | Local asset path, remote URL, or data URL |
| `action` | string | String action |
| `actionSpec` | object? | Declarative executable action |

## Fallback Behavior

- `layout` is still required even for template scenes.
- Template scenes use `view` for rendering, while `layout` remains the compatibility fallback and metadata source.
- Older plugins with no `view` continue to render as grids.
