## Learned User Preferences
- Prefers AstroDeck to run as a tray-first desktop app on Windows, starting hidden with a system tray icon and keeping the main window out of the browser.
- Wants the AstroDeck control window to use native OS window chrome for normal use (not a custom draggable frame); closing hides to the tray instead of exiting. Optional in-app fullscreen hides the native titlebar on the active display; Esc exits fullscreen.
- Expects tray interactions to be intuitive: left-click should show the main window if hidden, and right-click should expose only minimal app controls.
- Prefers `http://localhost:1420/` for browser-side debugging with live logs from the running app, with primary settings in the main window (deck/settings toggle and tray) rather than a second Tauri settings webview (that path was unstable: blank window / hung app).
- Uses `task dev` and Taskfile-based commands to run and manage the project locally.

## Learned Workspace Facts
- AstroDeck is implemented as a Tauri 2 + Rust backend with a Svelte 5 + TypeScript + Vite frontend in this repository.
- The project uses a JSON-based plugin system in the `plugins/` directory (e.g., Teams, Spotify, VS Code, default) to define scenes, triggers, layouts, and actions.
- The app uses a detection pipeline (sysinfo-based detectors → mode/scene engine → layout engine → plugin configs → Svelte UI) to pick the active scene at runtime.
- The browser at `http://localhost:1420/` attaches to the shared live log bus and carries browser-side debug controls; the main desktop window hosts the deck UI and in-app settings.
- Spotify controls are split by capability: play/pause and track skip use OS-level media control, while like-song and Spotify-specific volume use Spotify Web API auth and stored tokens.
- Packaged/desktop Spotify connect and disconnect use Tauri `invoke` commands (not only the browser WebSocket log bus). Spotify Web API features need a Client ID: `SPOTIFY_CLIENT_ID` env when set, otherwise a user-entered ID saved in app local data via settings.
- Spotify scene can show an expanded now-playing view (`MediaPlayerView`) with album art, transport controls, and Spotify-style scrub/volume UI.
- Docker-based Linux packaging from Windows runs `npm ci` in the container, expects `@tauri-apps/cli` in devDependencies for headless `npx tauri build`, and installs `xdg-utils` in the image so AppImage bundling can use `xdg-open`.
- The development workflow relies on `Taskfile.yml` tasks such as `task dev`, `task build`, and `task test` to coordinate frontend and Tauri commands.
