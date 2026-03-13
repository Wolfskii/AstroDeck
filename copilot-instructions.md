# TapTapDeck - GitHub Copilot Instructions

## File Patterns

| Pattern | Contents |
|---------|----------|
| `src/**/*.svelte` | Svelte 5 components; use `$props()`, `$state()`, `$derived` |
| `src/services/*.ts` | Tauri API wrappers; use `invoke()` from `@tauri-apps/api/core` |
| `src/types/index.ts` | Shared TypeScript interfaces (DeckButtonConfig, LayoutConfig, etc.) |
| `src-tauri/src/detectors/*.rs` | Detector implementations; implement `Detector` trait |
| `src-tauri/src/actions/*.rs` | Action handlers; `handle(command: &str) -> Result<(), String>` |
| `src-tauri/src/*_engine.rs` | Core logic (mode_engine, layout_engine, plugin_engine) |
| `plugins/*.json` | Plugin definitions; must match PluginConfig schema |

## Import Conventions

- Frontend: `import { invoke } from "@tauri-apps/api/core"` for Tauri
- Components: `import Component from "./Component.svelte"`
- Types: `import type { X } from "../types"`
- Detectors: `use super::Detector; use sysinfo::System;`
- Actions: `mod` in `actions/mod.rs`, match arm in `dispatch()`

## Svelte 5 Component Props

```typescript
interface Props {
  grid: [number, number];
  buttons: DeckButtonConfig[];
}
let { grid, buttons }: Props = $props();
```

## Rust Module Structure

- Detectors: `impl Detector for X { fn id() -> &str; fn detect(&self, system: &System) -> bool }`
- Actions: `pub fn handle(command: &str) -> Result<(), String>` with `match command { ... }`
- Actions dispatched by `namespace.command`; first dot splits namespace from command

## Plugin JSON Schema

```json
{
  "id": "string",
  "name": "string",
  "priority": 0,
  "triggers": { "process": "?", "windowTitleContains": "?" },
  "layout": {
    "grid": [2, 3],
    "buttons": [{ "label": "", "emoji": "?", "image": "?", "action": "ns.command" }]
  }
}
```
