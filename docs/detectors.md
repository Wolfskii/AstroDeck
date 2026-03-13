# Detectors

Detectors determine which plugins/scenes are active based on running processes and window titles.

## Detection Loop

The detection loop runs in a background thread every `DETECTOR_INTERVAL` milliseconds, defaulting to `2000`.

Each cycle:

1. Refresh running processes using `sysinfo`.
2. Collect process names and executable filenames.
3. Collect visible window titles on Windows.
4. Evaluate each plugin against either:
   - a built-in detector for that plugin `id`, or
   - JSON trigger rules
5. Resolve the highest-priority matched scene.

## Built-In Detectors

Built-in detectors still exist for:

- `default`
- `teams`
- `spotify`
- `vscode`

If a plugin defines explicit trigger rules, those trigger rules are used as the authoritative match instead of letting the hardcoded detector bypass stricter filters.

## Trigger Fields

Supported plugin trigger fields:

| Field | Description |
|-------|-------------|
| `process` | Case-insensitive substring match |
| `processGlob` | Wildcard process match using `*` and `?` |
| `processesAny` | Match if any listed process pattern matches |
| `processesAll` | Match only if all listed process patterns match |
| `excludeProcesses` | Reject the plugin if any listed process pattern matches |
| `windowTitleContains` | Case-insensitive substring match across visible window titles |
| `windowTitleGlob` | Wildcard match against visible window titles |
| `windowTitlesAny` | Match if any listed window-title pattern matches |

## Wildcard Matching

Wildcard trigger fields support:

- `*` for any sequence of characters
- `?` for any single character

Examples:

- `*spotify*`
- `spotify.exe`
- `*meeting*`
- `Code?`

## Example

```json
{
  "triggers": {
    "processesAny": ["*spotify*", "*spotify.exe"],
    "excludeProcesses": ["*helper*"],
    "windowTitleGlob": "*Now Playing*"
  }
}
```

## Window Detection Notes

- Window-title matching currently collects visible top-level window titles on Windows.
- Non-Windows platforms fall back to process-only matching for now.

## Adding A New Detector

Hardcoded detectors are still useful for platform-specific heuristics, but most user-authored plugins should rely on JSON triggers first.
