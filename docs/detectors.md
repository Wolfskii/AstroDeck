# Detectors

Detectors determine which plugins/scenes are "active" based on running processes. They run in a background loop and feed the mode engine with matched plugin IDs.

## Detector Trait

All detectors implement the `Detector` trait in `src-tauri/src/detectors/mod.rs`:

```rust
pub trait Detector: Send + Sync {
    fn id(&self) -> &str;           // Plugin ID this detector matches (e.g. "teams")
    fn detect(&self, system: &System) -> bool;  // Returns true if app is running
}
```

- `id()` returns the plugin ID that this detector activates when it matches.
- `detect()` receives a `sysinfo::System` and returns whether the target app is running.

## Built-in Detectors

| Detector | ID | Detects |
|----------|-----|---------|
| `TeamsDetector` | `teams` | Processes containing "teams" or "ms-teams" |
| `SpotifyDetector` | `spotify` | Spotify process |
| `VscodeDetector` | `vscode` | VS Code / Code process |
| `DefaultDetector` | `default` | Always matches (fallback) |

## Detection Loop

- Runs in a **background thread** spawned at app startup.
- Interval: `DETECTOR_INTERVAL` env var (default **2000** ms).
- Uses `sysinfo` crate for cross-platform process enumeration.
- Each tick:
  1. Refreshes process list.
  2. For each plugin, checks: (a) detector match by `id`, or (b) trigger match (process name, window title).
  3. Calls `mode_engine::resolve()` with matched plugin IDs.

## Trigger Matching

Plugins can also match via **triggers** in their JSON (no detector needed):

- `process`: substring match against process names (case-insensitive).
- `windowTitleContains`: substring match against focused window title (platform-specific; currently stubbed).

Trigger matching is done in `mode_engine::matches_triggers()` and combined with detector matches via OR.

## Adding a New Detector

1. Create `src-tauri/src/detectors/my_detector.rs`:

```rust
use super::Detector;
use sysinfo::System;

pub struct MyDetector;

impl Detector for MyDetector {
    fn id(&self) -> &str {
        "my-plugin-id"
    }

    fn detect(&self, system: &System) -> bool {
        system.processes().values().any(|p| {
            p.name().to_string_lossy().to_lowercase().contains("myapp")
        })
    }
}
```

2. Add the module and register in `detectors/mod.rs`:

```rust
mod my_detector;
// ...

fn all_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        // ... existing detectors ...
        Box::new(my_detector::MyDetector),
    ]
}
```

3. Create a plugin JSON in `plugins/` with matching `id` and triggers (optional).
