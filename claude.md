# TapTapDeck - Claude Instructions

## Project Overview

TapTapDeck is a context-aware control deck: a desktop app that shows different button layouts based on detected applications. Built with Tauri 2, Svelte 5, TypeScript, and Vite.

## Architecture

1. **Detectors** - Trait-based process detection using sysinfo, runs in background loop
2. **Mode engine** - Resolves which plugin/scene is active (highest priority match)
3. **Layout engine** - Looks up layout for active scene from plugins
4. **Plugin engine** - Loads JSON plugins from `plugins/`, sorts by priority
5. **UI** - Renders grid of buttons, invokes actions via Tauri commands

## Key Directories

- `src/` - Svelte 5 frontend
- `src-tauri/` - Rust backend
- `plugins/` - JSON plugin definitions

## Coding Conventions

- Rust: snake_case, prefer functional patterns (iterators, closures, Option/Result)
- Svelte 5: Use `$state` and `$derived` runes; PascalCase components
- TypeScript: Strict mode

## Critical Invariants

Preserve: plugin JSON schema, Detector trait interface, action dispatch pattern (`namespace.command`).

## Task-Specific Guidance

### Adding a new detector

1. Create `src-tauri/src/detectors/<name>_detector.rs`
2. Implement the `Detector` trait: `id()` returns plugin id string, `detect(system)` returns bool using sysinfo
3. Add `mod <name>_detector;` in `detectors/mod.rs`
4. Register in `all_detectors()` vec

### Adding a new plugin

1. Create `plugins/<id>.json`
2. Follow schema: `id`, `name`, `priority`, `triggers` (optional process/windowTitleContains), `layout` with `grid` and `buttons`
3. Each button: `label`, `emoji` or `image`, `action` (e.g. `namespace.command`)

### Adding new actions

1. Create `src-tauri/src/actions/<namespace>_actions.rs` with `pub fn handle(command: &str) -> Result<(), String>`
2. Add `mod <namespace>_actions;` in `actions/mod.rs`
3. Add match arm in `actions::dispatch`: `"<namespace>" => <namespace>_actions::handle(command)`

### Svelte 5 patterns

- Use `$props()` for component props: `let { prop1, prop2 }: Props = $props();`
- Use `$state()` for reactive state, `$derived()` for computed values
- Import from `@tauri-apps/api/core` for `invoke`

## Testing

`task test` runs svelte-check and cargo test.
