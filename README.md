# TapTapDeck

A context-aware control deck for touchscreen monitors. Dynamically changes its UI depending on which applications are running.

## Features

- **Context-aware switching** — Automatically detects running applications and switches to the appropriate control layout
- **Plugin system** — Extensible via JSON-based plugins for custom apps and workflows
- **Touchscreen-optimized UI** — Large, touch-friendly buttons and layouts designed for monitors
- **Cross-platform** — Runs on Windows, macOS, and Linux

## Tech Stack

- **Frontend:** Svelte 5, TypeScript, Vite
- **Backend:** Tauri 2, Rust
- **System info:** sysinfo

## Quick Start

### Prerequisites

- Node.js 18+
- Rust toolchain
- Tauri CLI: `npm install -g @tauri-apps/cli`

### Setup

```bash
git clone <repository-url>
cd Tap-Tap-Deck
npm install
cp .env.example .env
npx tauri dev
```

Or using Taskfile (after copying `.env.example` to `.env`):

```bash
task install && task dev
```

## Project Structure

```
Tap-Tap-Deck/
├── src/              # Svelte frontend (modes, layouts, components)
├── src-tauri/        # Rust backend (detectors, actions, layout engine)
├── plugins/          # JSON plugin definitions
├── docs/             # Documentation
└── ...
```

## Adding Plugins

Drop a JSON file in the `plugins/` directory. Each plugin must follow the schema:

- `id` — Unique plugin identifier
- `name` — Display name
- `priority` — Detection priority (higher = preferred when multiple match)
- `triggers` — Process/window matching rules
- `layout` — Button grid and actions

See existing plugins (`default.json`, `vscode.json`, `spotify.json`, `teams.json`) for examples.

## Documentation

- [Architecture](docs/architecture.md)
- [Detectors](docs/detectors.md)
- [Layouts](docs/layouts.md)
- [Plugins](docs/plugins.md)
- [Actions](docs/actions.md)
- [Roadmap](docs/roadmap.md)

## License

MIT
