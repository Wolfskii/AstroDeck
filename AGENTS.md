## Learned User Preferences
- Prefers TapTapDeck to run as a tray-first desktop app on Windows, starting hidden with a system tray icon and keeping the main window out of the browser.
- Wants the TapTapDeck control window to use normal OS window chrome (_, maximize, X) with closing the window hiding it to the tray instead of exiting the app.
- Expects tray interactions to be intuitive: left-click should show the main window if hidden, and right-click should expose only minimal app controls.
- Prefers `http://localhost:1420/` to remain available as the dedicated browser-based settings and debug panel with live logs, instead of using a separate Tauri settings window.
- Uses `task dev` and Taskfile-based commands to run and manage the project locally.

## Learned Workspace Facts
- TapTapDeck is implemented as a Tauri 2 + Rust backend with a Svelte 5 + TypeScript + Vite frontend in this repository.
- The project uses a JSON-based plugin system in the `plugins/` directory (e.g., Teams, Spotify, VS Code, default) to define scenes, triggers, layouts, and actions.
- The app uses a detection pipeline (sysinfo-based detectors → mode/scene engine → layout engine → plugin configs → Svelte UI) to pick the active scene at runtime.
- The browser at `http://localhost:1420/` functions as the project's settings and debug panel and receives a shared live log stream from the running desktop app.
- Spotify controls are split by capability: play/pause and track skip use OS-level media control, while like-song and Spotify-specific volume use Spotify Web API auth and stored tokens.
- The development workflow relies on `Taskfile.yml` tasks such as `task dev`, `task build`, and `task test` to coordinate frontend and Tauri commands.
