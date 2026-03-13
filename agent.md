# TapTapDeck - AI Agent Instructions

## Project Overview

TapTapDeck is a context-aware control deck: a desktop app that shows different button layouts based on detected applications (e.g. Spotify, Teams, VS Code). Built with Tauri 2, Svelte 5, TypeScript, and Vite.

## Architecture

Data flows through five stages:

1. **Detectors** - Trait-based process detection using sysinfo, runs in background loop
2. **Mode engine** - Resolves which plugin/scene is active (highest priority match)
3. **Layout engine** - Looks up layout for active scene from plugins
4. **Plugin engine** - Loads JSON plugins from `plugins/`, sorts by priority
5. **UI** - Renders grid of buttons, invokes actions via Tauri commands

## Key Directories

| Directory | Purpose |
|-----------|---------|
| `src/` | Svelte 5 frontend: components, services, types |
| `src-tauri/` | Rust backend: detectors, actions, engines |
| `plugins/` | JSON plugin definitions (id, name, priority, triggers, layout) |

## Coding Conventions

- **Rust**: snake_case for functions/variables, PascalCase for types
- **Svelte**: PascalCase for component files (e.g. `DeckButton.svelte`)
- **TypeScript**: Strict mode, explicit types for API boundaries

## Critical Invariants (Do Not Break)

When modifying the codebase, always preserve:

1. **Plugin JSON schema** - `id`, `name`, `priority`, `triggers`, `layout` with `grid` and `buttons`
2. **Detector trait interface** - `fn id(&self) -> &str` and `fn detect(&self, system: &System) -> bool`
3. **Action dispatch pattern** - Actions use `namespace.command` format, dispatched by prefix in `actions::dispatch`

## Testing

Run `task test` to execute svelte-check and cargo test.
