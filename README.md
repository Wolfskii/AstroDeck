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

## Packaging

TapTapDeck can now build native production packages through the Taskfile or npm scripts.

### Default Host Build

Build for the current OS:

```bash
task deploy
```

This runs the packaging script in `scripts/package-app.mjs`, builds the frontend, runs `tauri build`, and copies the resulting native bundle files into:

```text
.artifacts/<platform>/
```

The original native bundle output also remains in:

```text
src-tauri/target/release/bundle/
```

### Explicit Platform Tasks

```bash
task deploy:windows
task deploy:macos
task deploy:linux
```

Equivalent npm commands:

```bash
npm run package:app
npm run package:windows
npm run package:macos
npm run package:linux
```

### Platform Notes

- `task deploy` is the recommended default because it targets the current host OS automatically.
- Windows packaging works on Windows and produces native installers such as `.msi` and `.exe`.
- When the host OS is not Linux and you request a Linux build, the packaging script now tries to build inside Docker automatically. Docker must be installed and running.
- macOS packaging is not supported from Windows or Linux in this script. Docker is not a practical or supported way to produce real Tauri macOS `.app` / `.dmg` bundles because Apple tooling and signing are macOS-specific.

### Cross-Building From Windows

- `task deploy:linux` on Windows: supported through Docker, if Docker Desktop is installed and started.
- `task deploy:macos` on Windows: fails fast with a clear message telling you to use a real macOS machine or macOS CI runner.

## Project Structure

```
Tap-Tap-Deck/
├── src/              # Svelte frontend (modes, layouts, components)
├── src-tauri/        # Rust backend (detectors, actions, layout engine)
├── plugins/          # Built-in plugin folders and manifests
├── scripts/          # Packaging and helper scripts
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
