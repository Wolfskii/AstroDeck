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

Some actions also accept a payload through `execute_action_value`.

| Action | Payload |
|--------|---------|
| `spotify.setVolume`, `youtubeMusic.setVolume`, `media.setVolume` | Volume `0-100` |
| `spotify.seek`, `youtubeMusic.seek`, `media.seek` | Position in milliseconds |
| `youtubeMusic.like` | Boolean saved state |
| `spotify.playPlaylist` | Playlist id or URI |

The media player uses these for the volume slider and the progress bar.

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
| `spotify` | Spotify playback, library, and playlists |
| `youtubeMusic` | Guest YouTube Music playback and saved tracks |
| `media` | System now-playing transport, seek, and output volume |
| `teams` | Teams meeting controls |
| `core` | App-level actions such as settings and refresh |

## Spotify Actions

| Action | Description |
|--------|-------------|
| `spotify.togglePlay` | Toggle play/pause |
| `spotify.nextTrack` | Next track |
| `spotify.prevTrack` | Previous track |
| `spotify.like` | Toggle saved-track state in the user library |
| `spotify.toggleShuffle` | Toggle shuffle |
| `spotify.setVolume` | Set volume from `0-100` |
| `spotify.seek` | Seek to a position in milliseconds |
| `spotify.playPlaylist` | Play a playlist id or `spotify:playlist:` URI |
| `spotify.volumeUp` / `spotify.volumeDown` | Nudge device volume |

Official Spotify sign-in plays inside AstroDeck. Transport for that mode does not require the Spotify desktop app. A custom Client ID uses the Web API.

## YouTube Music Actions

| Action | Description |
|--------|-------------|
| `youtubeMusic.togglePlay` | Toggle play/pause |
| `youtubeMusic.nextTrack` | Next track |
| `youtubeMusic.prevTrack` | Previous track |
| `youtubeMusic.like` | Save or unsave the current track in the local guest library |
| `youtubeMusic.toggleShuffle` | Toggle shuffle |
| `youtubeMusic.setVolume` | Set volume from `0-100` |
| `youtubeMusic.seek` | Seek to a position in milliseconds |

## System Media Actions

| Action | Description |
|--------|-------------|
| `media.togglePlay` | Toggle play/pause on the system session |
| `media.nextTrack` | Next track |
| `media.prevTrack` | Previous track |
| `media.setVolume` | Set the computer's output volume from `0-100` |
| `media.seek` | Seek to a position in milliseconds |

## Teams Actions

| Action | Description |
|--------|-------------|
| `teams.reaction.like` / `.heart` / `.clap` / `.laugh` / `.wow` | Send a meeting reaction through the Teams device API |
| `teams.toggleMute` | Mute or unmute (`Ctrl+Shift+M`) |
| `teams.toggleCamera` | Camera (`Ctrl+Shift+O`) |
| `teams.shareScreen` | Share (`Ctrl+Shift+E`) |
| `teams.raiseHand` | Raise hand (`Ctrl+Shift+K`) |
| `teams.chat` | Open chat through the Teams API |
| `teams.leaveMeeting` | Leave (`Ctrl+Shift+H`) |

## Notes

- String actions remain the compatibility path for existing built-in plugins.
- Declarative actions are the preferred path for user-authored plugins that should execute real OS behavior without needing new Rust code.
