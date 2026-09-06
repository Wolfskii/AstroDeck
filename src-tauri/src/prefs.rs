use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppPreferences {
    #[serde(default = "default_start_minimized")]
    pub start_minimized: bool,
    #[serde(default)]
    pub start_fullscreen: bool,
    #[serde(default = "default_show_update_popups")]
    pub show_update_popups: bool,
    #[serde(default = "default_scene_background")]
    pub scene_background: String,
    #[serde(default)]
    pub show_settings_terminal: bool,
    #[serde(default = "default_controls_backdrop")]
    pub controls_backdrop: bool,
    #[serde(default = "default_controls_transparency")]
    pub controls_transparency: u8,
    #[serde(default = "default_controls_overlay_color")]
    pub controls_overlay_color: String,
    #[serde(default)]
    pub controls_overlay_custom: Option<bool>,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            start_minimized: default_start_minimized(),
            start_fullscreen: false,
            show_update_popups: default_show_update_popups(),
            scene_background: default_scene_background(),
            show_settings_terminal: false,
            controls_backdrop: default_controls_backdrop(),
            controls_transparency: default_controls_transparency(),
            controls_overlay_color: default_controls_overlay_color(),
            controls_overlay_custom: None,
        }
    }
}

fn default_start_minimized() -> bool {
    true
}

fn default_show_update_popups() -> bool {
    true
}

fn default_scene_background() -> String {
    "mesh-gradient".to_string()
}

fn default_controls_backdrop() -> bool {
    true
}

fn default_controls_transparency() -> u8 {
    35
}

fn default_controls_overlay_color() -> String {
    "#000000".to_string()
}

fn parse_transparency(value: u8) -> u8 {
    value.min(100)
}

fn parse_overlay_color(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        trimmed.to_ascii_lowercase()
    } else {
        default_controls_overlay_color()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlsBackdropState {
    enabled: bool,
    transparency: u8,
    color: String,
    custom: bool,
}

const SCENE_BACKGROUNDS: &[&str] = &[
    "off",
    "mesh-gradient",
    "dithering",
    "neuro-noise",
    "grain-gradient",
    "metaballs",
    "pulsing-border",
    "fluted-glass",
    "water",
    "liquid-gradient",
    "aurora",
    "cosmos",
    "warp",
    "prism",
    "horizon",
    "spectrum",
    "bokeh",
    "ripple",
    "helix",
    "plasma",
    "kaleido",
];

fn parse_scene_background(value: &str) -> String {
    if SCENE_BACKGROUNDS.contains(&value) {
        value.to_string()
    } else {
        default_scene_background()
    }
}

fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|directory| directory.join("preferences.json"))
        .map_err(|error| error.to_string())
}

fn load_preferences(path: &std::path::Path) -> Result<AppPreferences, String> {
    match fs::read(path) {
        Ok(contents) => serde_json::from_slice(&contents).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppPreferences::default()),
        Err(error) => Err(error.to_string()),
    }
}

fn save_preferences(path: &std::path::Path, preferences: &AppPreferences) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_string_pretty(preferences).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_update_popups_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(load_preferences(&preferences_path(&app)?)?.show_update_popups)
}

#[tauri::command]
pub fn set_update_popups_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.show_update_popups = enabled;
    save_preferences(&path, &preferences)
}

pub fn start_minimized(app: &tauri::AppHandle) -> bool {
    preferences_path(app)
        .and_then(|path| load_preferences(&path))
        .map(|preferences| preferences.start_minimized)
        .unwrap_or_else(|_| default_start_minimized())
}

pub fn start_fullscreen(app: &tauri::AppHandle) -> bool {
    preferences_path(app)
        .and_then(|path| load_preferences(&path))
        .map(|preferences| preferences.start_fullscreen)
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_start_minimized(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(start_minimized(&app))
}

#[tauri::command]
pub fn set_start_minimized(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.start_minimized = enabled;
    save_preferences(&path, &preferences)
}

#[tauri::command]
pub fn get_start_fullscreen(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(start_fullscreen(&app))
}

#[tauri::command]
pub fn set_start_fullscreen(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.start_fullscreen = enabled;
    save_preferences(&path, &preferences)
}

#[tauri::command]
pub fn get_scene_background(app: tauri::AppHandle) -> Result<String, String> {
    let value = load_preferences(&preferences_path(&app)?)?.scene_background;
    Ok(parse_scene_background(&value))
}

#[tauri::command]
pub fn set_scene_background(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.scene_background = parse_scene_background(&id);
    save_preferences(&path, &preferences)?;
    let _ = app.emit("scene-background-changed", &preferences.scene_background);
    Ok(())
}

#[tauri::command]
pub fn get_show_settings_terminal(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(load_preferences(&preferences_path(&app)?)?.show_settings_terminal)
}

#[tauri::command]
pub fn set_show_settings_terminal(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.show_settings_terminal = enabled;
    save_preferences(&path, &preferences)?;
    let _ = app.emit("settings-terminal-changed", enabled);
    Ok(())
}

fn overlay_custom(preferences: &AppPreferences) -> bool {
    match preferences.controls_overlay_custom {
        Some(value) => value,
        None => {
            parse_overlay_color(&preferences.controls_overlay_color)
                != default_controls_overlay_color()
        }
    }
}

fn emit_controls_backdrop(app: &tauri::AppHandle, preferences: &AppPreferences) {
    let _ = app.emit(
        "controls-backdrop-changed",
        ControlsBackdropState {
            enabled: preferences.controls_backdrop,
            transparency: parse_transparency(preferences.controls_transparency),
            color: parse_overlay_color(&preferences.controls_overlay_color),
            custom: overlay_custom(preferences),
        },
    );
}

#[tauri::command]
pub fn get_controls_backdrop_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(load_preferences(&preferences_path(&app)?)?.controls_backdrop)
}

#[tauri::command]
pub fn set_controls_backdrop_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.controls_backdrop = enabled;
    save_preferences(&path, &preferences)?;
    emit_controls_backdrop(&app, &preferences);
    Ok(())
}

#[tauri::command]
pub fn get_controls_transparency(app: tauri::AppHandle) -> Result<u8, String> {
    Ok(parse_transparency(
        load_preferences(&preferences_path(&app)?)?.controls_transparency,
    ))
}

#[tauri::command]
pub fn set_controls_transparency(app: tauri::AppHandle, value: u8) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.controls_transparency = parse_transparency(value);
    save_preferences(&path, &preferences)?;
    emit_controls_backdrop(&app, &preferences);
    Ok(())
}

#[tauri::command]
pub fn get_controls_overlay_color(app: tauri::AppHandle) -> Result<String, String> {
    Ok(parse_overlay_color(
        &load_preferences(&preferences_path(&app)?)?.controls_overlay_color,
    ))
}

#[tauri::command]
pub fn set_controls_overlay_color(app: tauri::AppHandle, color: String) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.controls_overlay_color = parse_overlay_color(&color);
    save_preferences(&path, &preferences)?;
    emit_controls_backdrop(&app, &preferences);
    Ok(())
}

#[tauri::command]
pub fn get_controls_overlay_custom(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(overlay_custom(
        &load_preferences(&preferences_path(&app)?)?,
    ))
}

#[tauri::command]
pub fn set_controls_overlay_custom(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.controls_overlay_custom = Some(enabled);
    save_preferences(&path, &preferences)?;
    emit_controls_backdrop(&app, &preferences);
    Ok(())
}
