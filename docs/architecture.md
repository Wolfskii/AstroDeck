# TapTapDeck Architecture

## Overview

TapTapDeck is a context-aware control deck: a cross-platform desktop app that dynamically changes its UI based on which applications are running. Built with **Tauri 2** (Rust backend) and **Svelte 5** (TypeScript frontend).

## High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              TapTapDeck                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌─────────────────────┐    ┌──────────────────────┐   │
│  │  Detectors   │───▶│ Mode/Scene Engine   │───▶│   Layout Engine       │   │
│  │  (sysinfo)   │    │ (priority resolve)  │    │ (lookup by scene ID)  │   │
│  └──────────────┘    └─────────────────────┘    └───────────┬──────────┘   │
│         │                          │                        │              │
│         │                          │                        │              │
│         ▼                          ▼                        ▼              │
│  ┌──────────────┐           ┌──────────────┐         ┌──────────────┐      │
│  │   Plugins    │           │  AppState    │         │ Plugin Engine│      │
│  │  (JSON)      │           │  (Mutex)     │         │ (load/scan)  │      │
│  └──────────────┘           └──────┬───────┘         └──────────────┘      │
│                                   │                                         │
│                                   │ scene-changed event                     │
│                                   ▼                                         │
│                            ┌──────────────┐                                 │
│                            │     UI       │                                 │
│                            │ (Svelte 5)   │                                 │
│                            └──────────────┘                                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Data Flow

1. **Detectors** check running processes (via `sysinfo` crate) every `DETECTOR_INTERVAL` ms (default 2000).
2. **Mode engine** resolves which scene is active: matched plugin IDs are filtered by priority; highest wins.
3. **Layout engine** looks up layout config by scene ID from loaded plugins.
4. **Plugin engine** provides plugin definitions (loaded from `plugins/` JSON files).
5. **UI** renders the grid and buttons based on the layout; listens for `scene-changed` Tauri events.

## Directory Structure

| Path | Description |
|------|-------------|
| `src/` | Frontend: Svelte 5, Vite, TypeScript |
| `src-tauri/` | Backend: Rust, Tauri 2 |
| `plugins/` | JSON plugin definitions (auto-loaded on startup) |

## Frontend Stack

- **Svelte 5** with runes (`$props`, `$state`, `$derived`)
- **Vite** for build and dev server
- **TypeScript** for type safety
- Components: `DeckGrid`, `DeckButton`, mode-specific views (`TeamsMeetingMode`, `DefaultMode`)

## Backend Stack

- **Rust** with Tauri 2
- **sysinfo** for cross-platform process detection
- **serde** for JSON (de)serialization
- Modules: `detectors`, `mode_engine`, `layout_engine`, `plugin_engine`, `actions`

## State Management

- **AppState** (Rust): `Mutex`-wrapped `plugins` and `active_scene_id`
- **Frontend sync**: Tauri `scene-changed` event carries `SceneResponse` (active scene, layout, available scenes)
- Frontend subscribes via `app.listen()` and updates local state

## Plugins

- JSON files in `plugins/` directory
- Scanned on startup; no restart needed to add new files (hot-reload planned via `notify` crate)
- Each plugin defines: id, name, priority, triggers, layout (grid + buttons)

## Future: WebSocket Server

- Planned for **mobile companion** support
- Stub in `src-tauri/src/websocket.rs`
- Will listen on configurable port (e.g. `WEBSOCKET_PORT`, default 3211)
- Broadcast `scene-changed` to connected clients; accept action commands from remote clients
