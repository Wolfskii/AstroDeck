use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppPreferences {
    #[serde(default = "default_start_minimized")]
    pub start_minimized: bool,
    #[serde(default)]
    pub start_fullscreen: bool,
    #[serde(default = "default_show_update_popups")]
    pub show_update_popups: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            start_minimized: default_start_minimized(),
            start_fullscreen: false,
            show_update_popups: default_show_update_popups(),
        }
    }
}

fn default_start_minimized() -> bool {
    true
}

fn default_show_update_popups() -> bool {
    true
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
