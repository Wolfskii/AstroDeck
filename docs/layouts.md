# Layouts

The layout system defines how buttons are arranged on the deck. Layouts are grid-based and come from plugin JSON files (or built-in fallbacks).

## Grid System

- **Format**: `[rows, cols]` tuple, e.g. `[3, 3]` = 3 rows × 3 columns.
- Defined in `LayoutConfig.grid` in plugin JSON.
- Frontend `DeckGrid` renders a CSS Grid: `grid-template-columns: repeat(cols, 1fr)` and `grid-template-rows: repeat(rows, 1fr)`.

## Button Config

Each button has:

| Field | Type | Description |
|-------|------|-------------|
| `label` | string | Button text |
| `emoji` | string? | Emoji icon (shown if no `image`) |
| `image` | string? | Image URL/path for icon |
| `action` | string | Action string, e.g. `"spotify.togglePlay"` |

## Layout Engine

- **Backend** (`layout_engine.rs`): `get_layout(scene_id, plugins)` looks up the plugin by `id` and returns its `layout`.
- **Frontend**: `DeckGrid` receives `grid` and `buttons`; falls back to `getBuiltinLayout()` in `src/layouts/layouts.ts` when backend returns no layout.

## Built-in Fallbacks

`src/layouts/layouts.ts` defines fallback layouts for `default` and `teams` when plugin layout is missing. Used to ensure the UI always has a valid config.

## Styling

- **Minimum size**: Buttons are at least `120px` (`--button-min-size` in `app.css`).
- **Theme**: Dark (`--bg-primary`, `--bg-button`, etc.).
- **Interactions**: Glow on hover; scale + glow on press (`transform: scale(0.93)`, `box-shadow`).
- **Animations**: `glow-pulse`, `press-scale` keyframes defined in `app.css`.

## Example

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
