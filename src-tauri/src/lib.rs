mod actions;
mod detectors;
mod layout_engine;
mod mode_engine;
mod plugin_engine;
mod prefs;
mod spotify;
mod updater;
mod websocket;
mod window_prefs;

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::broadcast;

pub use plugin_engine::PluginConfig;

pub struct AppState {
    pub plugins: Mutex<Vec<PluginConfig>>,
    pub plugin_actions: Mutex<HashMap<String, plugin_engine::ActionSpec>>,
    pub active_scene_id: Mutex<String>,
    pub manual_scene_override: Mutex<Option<String>>,
    pub spotify: spotify::SpotifyState,
    pub log_bus: broadcast::Sender<String>,
}

impl AppState {
    pub fn new() -> Self {
        let (log_tx, _) = broadcast::channel(512);
        Self {
            plugins: Mutex::new(Vec::new()),
            plugin_actions: Mutex::new(HashMap::new()),
            active_scene_id: Mutex::new("default".to_string()),
            manual_scene_override: Mutex::new(None),
            spotify: spotify::SpotifyState::default(),
            log_bus: log_tx,
        }
    }
}

pub fn apply_scene_change(
    app: &tauri::AppHandle,
    state: &AppState,
    requested_scene_id: String,
    manual_override: bool,
) -> Result<(), String> {
    let plugins = state.plugins.lock().map_err(|e| e.to_string())?;

    let target_id = if plugins.iter().any(|p| p.id == requested_scene_id) {
        requested_scene_id
    } else {
        "default".to_string()
    };

    {
        let mut override_state = state
            .manual_scene_override
            .lock()
            .map_err(|e| e.to_string())?;
        if manual_override {
            *override_state = Some(target_id.clone());
        } else {
            *override_state = None;
        }
    }

    {
        let mut current = state.active_scene_id.lock().map_err(|e| e.to_string())?;
        *current = target_id.clone();
    }

    let layout = layout_engine::get_layout(&target_id, &plugins);
    let available_scenes = plugins.iter().map(|p| p.id.clone()).collect();

    let response = SceneResponse {
        active_scene_id: target_id.clone(),
        layout,
        available_scenes,
    };

    if let Err(e) = app.emit("scene-changed", &response) {
        log::error!("Failed to emit scene-changed event: {}", e);
    }

    Ok(())
}

pub fn launch_spotify_auth(app: &tauri::AppHandle, state: &AppState) -> Result<String, String> {
    let auth_url = spotify::start_auth_flow(&state.spotify)?;
    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| format!("Failed to open browser for Spotify sign-in: {e}"))?;
    let log_bus = state.log_bus.clone();
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let app_state = app_handle.state::<AppState>();
        let spotify_state = &app_state.spotify;
        match spotify::complete_auth_via_callback(spotify_state) {
            Ok(()) => {
                let payload = serde_json::json!({
                    "type": "spotifyStatus",
                    "payload": spotify::get_status(spotify_state).ok(),
                });
                let _ = log_bus.send(payload.to_string());
                log::info!("Spotify authorization completed");
            }
            Err(e) => {
                log::error!("Spotify authorization failed: {}", e);
                let payload = serde_json::json!({
                    "type": "spotifyAuthError",
                    "payload": { "message": e },
                });
                let _ = log_bus.send(payload.to_string());
            }
        }
    });
    Ok(auth_url)
}

#[derive(serde::Serialize, Clone)]
pub struct SceneResponse {
    #[serde(rename = "activeSceneId")]
    pub active_scene_id: String,
    pub layout: Option<plugin_engine::LayoutConfig>,
    #[serde(rename = "availableScenes")]
    pub available_scenes: Vec<String>,
}

#[tauri::command]
fn get_active_scene(state: tauri::State<AppState>) -> Result<SceneResponse, String> {
    let scene_id = state
        .active_scene_id
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let plugins = state.plugins.lock().map_err(|e| e.to_string())?;

    let layout = layout_engine::get_layout(&scene_id, &plugins);
    let available_scenes = plugins.iter().map(|p| p.id.clone()).collect();

    Ok(SceneResponse {
        active_scene_id: scene_id,
        layout,
        available_scenes,
    })
}

#[tauri::command]
fn execute_action(action: String, app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    actions::dispatch(&action, &app, &state)
}

#[tauri::command]
fn execute_action_value(
    action: String,
    value: serde_json::Value,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    actions::dispatch_value(&action, value, &app, &state)
}

#[tauri::command]
fn get_plugins(state: tauri::State<AppState>) -> Result<Vec<PluginConfig>, String> {
    let plugins = state.plugins.lock().map_err(|e| e.to_string())?;
    Ok(plugins.clone())
}

/// Manually override the active scene from the settings/debug window.
#[tauri::command]
fn set_active_scene(scene_id: String, state: tauri::State<AppState>, app: tauri::AppHandle) -> Result<(), String> {
    apply_scene_change(&app, &state, scene_id, true)
}

/// Forward a JSON-encoded LogEntry from the Tauri webview to the WS log bus
/// so that browser clients at http://localhost:1420 can receive live logs.
#[tauri::command]
fn log_to_bus(entry: String, state: tauri::State<AppState>) -> Result<(), String> {
    let _ = state.log_bus.send(entry);
    Ok(())
}

#[tauri::command]
fn get_spotify_status(
    fresh: Option<bool>,
    state: tauri::State<AppState>,
) -> Result<spotify::SpotifyStatus, String> {
    if fresh.unwrap_or(false) {
        spotify::get_status_fresh(&state.spotify)
    } else {
        spotify::get_status(&state.spotify)
    }
}

#[tauri::command]
fn peek_spotify_skip_track(
    direction: String,
    state: tauri::State<AppState>,
) -> Option<spotify::TrackPreview> {
    spotify::apply_optimistic_skip(&state.spotify, &direction)
}

#[tauri::command]
fn set_spotify_volume(
    volume_percent: u8,
    state: tauri::State<AppState>,
) -> Result<u8, String> {
    let next = spotify::set_volume(&state.spotify, volume_percent.clamp(0, 100), None)?;
    if let Ok(status) = spotify::get_status(&state.spotify) {
        let payload = serde_json::json!({
            "type": "spotifyStatus",
            "payload": status,
        });
        let _ = state.log_bus.send(payload.to_string());
    }
    Ok(next)
}

#[tauri::command]
fn start_spotify_auth(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    launch_spotify_auth(&app, &state)
}

#[tauri::command]
fn disconnect_spotify(state: tauri::State<AppState>) -> Result<(), String> {
    spotify::disconnect(&state.spotify)
}

#[tauri::command]
fn get_spotify_client_config(
    state: tauri::State<AppState>,
) -> Result<spotify::SpotifyClientConfigResponse, String> {
    Ok(spotify::get_client_config_for_ui(&state.spotify))
}

#[tauri::command]
fn set_spotify_client_id(client_id: String, state: tauri::State<AppState>) -> Result<(), String> {
    spotify::set_client_id_from_settings(&state.spotify, &client_id)?;
    if let Ok(status) = spotify::get_status(&state.spotify) {
        let payload = serde_json::json!({
            "type": "spotifyStatus",
            "payload": status,
        });
        let _ = state.log_bus.send(payload.to_string());
    }
    Ok(())
}

/// Create or focus the dedicated settings window from the tray or buttons.
#[tauri::command]
fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    let label = "settings";

    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }

    // Use the same app URL as the main window; in dev this is mapped to the Vite dev server
    // via `build.devUrl` in `tauri.conf.json`.
    tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::App("index.html".into()))
    .title("AstroDeck Settings")
    .inner_size(900.0, 700.0)
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn persist_window_state(app: tauri::AppHandle) -> Result<(), String> {
    window_prefs::persist(&app);
    Ok(())
}

#[tauri::command]
fn restore_window_show_state(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window_prefs::restore_show_state(&window, prefs::start_fullscreen(&app));
    }
    Ok(())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    window_prefs::persist(&app);
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenv::dotenv();
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(window_prefs::plugin())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_active_scene,
            execute_action,
            execute_action_value,
            get_plugins,
            set_active_scene,
            log_to_bus,
            get_spotify_status,
            peek_spotify_skip_track,
            set_spotify_volume,
            start_spotify_auth,
            disconnect_spotify,
            get_spotify_client_config,
            set_spotify_client_id,
            open_settings_window,
            persist_window_state,
            restore_window_show_state,
            quit_app,
            updater::get_app_version,
            updater::check_for_app_update,
            updater::download_and_install_update,
            prefs::get_update_popups_enabled,
            prefs::set_update_popups_enabled,
            prefs::get_start_minimized,
            prefs::set_start_minimized,
            prefs::get_start_fullscreen,
            prefs::set_start_fullscreen,
            prefs::get_scene_background,
            prefs::set_scene_background,
            prefs::get_show_settings_terminal,
            prefs::set_show_settings_terminal,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                window_prefs::persist(&window.app_handle());
            }
        })
        .setup(|app| {
            let app_handle = app.handle().clone();

            if let Some(window) = app.get_webview_window("main") {
                window_prefs::apply_launch_state(
                    &window,
                    prefs::start_minimized(&app_handle),
                    prefs::start_fullscreen(&app_handle),
                );
            }

            plugin_engine::load_plugins(&app_handle);
            detectors::start_detection_loop(app_handle.clone());
            spotify::init(&app_handle, &app.state::<AppState>().spotify)?;

            // Start WebSocket log bus so browser clients see live logs
            let log_tx = app.state::<AppState>().log_bus.clone();
            let port = std::env::var("WEBSOCKET_PORT")
                .unwrap_or_else(|_| "3211".to_string())
                .parse::<u16>()
                .unwrap_or(3211);
            let ws_app_handle = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                websocket::start_log_bus(port, ws_app_handle, log_tx).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running AstroDeck");
}
