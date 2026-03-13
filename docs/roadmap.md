# TapTapDeck Roadmap

## Phase 1: Local Deck Foundation (Current)

- **Detectors**: Process-based detection via `sysinfo`; trait-based detector system
- **Plugins**: JSON plugin loading from `plugins/` directory
- **Layout engine**: Grid-based layouts; lookup by scene ID
- **Mode engine**: Priority-based scene resolution; `scene-changed` events
- **Tauri desktop app**: Svelte 5 + Vite frontend; Rust backend
- **Actions**: Dispatcher with teams, spotify, core namespaces (stubs)

## Phase 2: Real Integrations

- **Teams**: Keyboard shortcut injection (Ctrl+Shift+M, etc.) or UI automation
- **Spotify**: Spotify Web API or D-Bus/media controls for play/pause, volume, like
- **VS Code**: Command API or extension for terminal, run, debug, git, search
- **Window title detection**: Platform-specific APIs for focused window title (triggers)

## Phase 3: Mobile Companion

- **WebSocket server**: Listen on configurable port (e.g. `WEBSOCKET_PORT`, default 3211)
- **Protocol**: JSON messages; server pushes scene state; client sends action commands
- **Mobile app**: React Native or Flutter; connect to desktop WebSocket
- **Synced state**: Scene and layout updates across desktop and mobile

## Phase 4: Plugin Marketplace

- **Community plugins**: Share plugin JSON files
- **Versioning**: Plugin version field; compatibility checks
- **Discovery**: Browse, install, update plugins from a central registry

## Phase 5: Advanced Features

- **OBS integration**: Scene switching, source visibility, stream control
- **Smart home**: Home Assistant, Philips Hue, etc.
- **Custom scripting**: Lua or scripting engine for plugin logic
- **Custom detectors**: User-defined process/window rules
