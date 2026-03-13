use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub triggers: PluginTriggers,
    pub layout: LayoutConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginTriggers {
    pub process: Option<String>,
    #[serde(rename = "windowTitleContains")]
    pub window_title_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub grid: (u32, u32),
    pub buttons: Vec<ButtonConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfig {
    pub label: String,
    pub emoji: Option<String>,
    pub image: Option<String>,
    pub action: String,
}

fn plugins_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    // Dev: CARGO_MANIFEST_DIR is src-tauri/, so ../plugins is the project-root plugins dir
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("plugins");
    if from_manifest.exists() {
        return from_manifest;
    }

    // Production: bundled alongside the binary in the resource directory
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let from_resource = resource_dir.join("plugins");
        if from_resource.exists() {
            return from_resource;
        }
    }

    PathBuf::from("plugins")
}

pub fn load_plugins(app_handle: &tauri::AppHandle) {
    let dir = plugins_dir(app_handle);
    log::info!("Loading plugins from: {:?}", dir);

    let state = app_handle.state::<crate::AppState>();
    let mut plugins = state.plugins.lock().expect("failed to lock plugins");
    plugins.clear();

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Could not read plugins directory {:?}: {}", dir, e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<PluginConfig>(&contents) {
                Ok(plugin) => {
                    log::info!("Loaded plugin: {} (priority {})", plugin.name, plugin.priority);
                    plugins.push(plugin);
                }
                Err(e) => log::error!("Failed to parse plugin {:?}: {}", path, e),
            },
            Err(e) => log::error!("Failed to read plugin {:?}: {}", path, e),
        }
    }

    plugins.sort_by(|a, b| b.priority.cmp(&a.priority));
    log::info!("Loaded {} plugins total", plugins.len());
}
