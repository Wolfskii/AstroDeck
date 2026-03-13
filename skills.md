# TapTapDeck - AI Skills Reference

## Extending Subsystems

### Detectors

1. Create `src-tauri/src/detectors/<name>_detector.rs`
2. Implement `Detector` trait:
   - `fn id(&self) -> &str` - must match a plugin id
   - `fn detect(&self, system: &System) -> bool` - use `system.processes()` from sysinfo
3. Add to `all_detectors()` in `detectors/mod.rs`
4. Detectors run in background loop; match triggers plugin activation

### Plugins

1. Create `plugins/<id>.json`
2. Schema: `id`, `name`, `priority` (higher = preferred), `triggers`, `layout`
3. Triggers: `process` (substring match), `windowTitleContains` (stubbed)
4. Layout: `grid: [rows, cols]`, `buttons: [{ label, emoji?, image?, action }]`
5. Loaded at startup from `plugins/` dir; sorted by priority descending

### Actions

1. Create `src-tauri/src/actions/<namespace>_actions.rs`
2. Implement `pub fn handle(command: &str) -> Result<(), String>`
3. Add `mod <namespace>_actions;` and match arm in `actions/mod.rs`
4. Action format: `namespace.command` (e.g. `spotify.togglePlay`, `teams.reaction.like`)
5. Dispatch splits on first dot: `namespace` = parts[0], `command` = parts[1]

### Layouts

- Layouts come from plugin JSON; no separate layout extension point
- `layout_engine::get_layout(scene_id, plugins)` finds plugin by id, returns its layout
- Frontend `layouts.ts` has optional builtin fallbacks

## Common Patterns

- **Scene resolution**: Detectors + triggers produce matched plugin ids; mode_engine picks highest priority
- **Event flow**: Backend emits `scene-changed`; frontend listens and updates layout
- **Tauri commands**: `#[tauri::command]` in lib.rs, `invoke("command_name", { args })` from frontend
- **Sub-commands**: Actions can use `command.starts_with("reaction.")` for namespaced commands

## JSON Plugin Schema Reference

```json
{
  "id": "unique-plugin-id",
  "name": "Display Name",
  "priority": 0,
  "triggers": {
    "process": "ProcessName",
    "windowTitleContains": "Window text"
  },
  "layout": {
    "grid": [2, 3],
    "buttons": [
      {
        "label": "Button Label",
        "emoji": "🔘",
        "image": null,
        "action": "namespace.command"
      }
    ]
  }
}
```

## Tauri Command Patterns

**Defining commands** (in `src-tauri/src/lib.rs`):

```rust
#[tauri::command]
fn get_active_scene(state: tauri::State<AppState>) -> Result<SceneResponse, String> { ... }

#[tauri::command]
fn execute_action(action: String) -> Result<(), String> { ... }
```

**Registering**: Add to `tauri::generate_handler![get_active_scene, execute_action, ...]`

**Invoking from frontend**:

```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke<SceneState>("get_active_scene");
await invoke("execute_action", { action: "spotify.togglePlay" });
```

**Events**: Backend `app_handle.emit("scene-changed", payload)`; frontend `listen("scene-changed", handler)`.
