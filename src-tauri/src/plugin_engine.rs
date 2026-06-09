use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "schemaVersion", default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub triggers: PluginTriggers,
    #[serde(default)]
    pub view: PluginViewConfig,
    pub layout: LayoutConfig,
    #[serde(default, skip_deserializing)]
    pub source: PluginSourceMeta,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginTriggers {
    pub process: Option<String>,
    #[serde(rename = "processGlob")]
    pub process_glob: Option<String>,
    #[serde(rename = "processesAny", default)]
    pub processes_any: Vec<String>,
    #[serde(rename = "processesAll", default)]
    pub processes_all: Vec<String>,
    #[serde(rename = "excludeProcesses", default)]
    pub exclude_processes: Vec<String>,
    #[serde(rename = "windowTitleContains")]
    pub window_title_contains: Option<String>,
    #[serde(rename = "windowTitleGlob")]
    pub window_title_glob: Option<String>,
    #[serde(rename = "windowTitlesAny", default)]
    pub window_titles_any: Vec<String>,
}

impl PluginTriggers {
    pub fn has_rules(&self) -> bool {
        self.process.is_some()
            || self.process_glob.is_some()
            || !self.processes_any.is_empty()
            || !self.processes_all.is_empty()
            || !self.exclude_processes.is_empty()
            || self.window_title_contains.is_some()
            || self.window_title_glob.is_some()
            || !self.window_titles_any.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub grid: (u32, u32),
    #[serde(default)]
    pub buttons: Vec<ButtonConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfig {
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub action: String,
    #[serde(rename = "actionSpec", default)]
    pub action_spec: Option<ActionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActionSpec {
    OpenUrl { url: String },
    OpenPath { path: String },
    Launch {
        program: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginViewConfig {
    #[serde(rename = "type", default = "default_view_type")]
    pub kind: String,
    #[serde(rename = "mediaPlayer", default)]
    pub media_player: Option<MediaPlayerViewConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaPlayerViewConfig {
    #[serde(default)]
    pub previous: Option<ButtonConfig>,
    #[serde(rename = "playPause", default)]
    pub play_pause: Option<ButtonConfig>,
    #[serde(default)]
    pub next: Option<ButtonConfig>,
    #[serde(default)]
    pub like: Option<ButtonConfig>,
    #[serde(default)]
    pub shuffle: Option<ButtonConfig>,
    #[serde(rename = "volumeAction", default)]
    pub volume_action: Option<String>,
    #[serde(rename = "seekAction", default)]
    pub seek_action: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSourceMeta {
    pub kind: String,
    #[serde(rename = "manifestPath")]
    pub manifest_path: Option<String>,
    #[serde(rename = "baseDir")]
    pub base_dir: Option<String>,
}

fn default_schema_version() -> u32 {
    1
}

fn default_view_type() -> String {
    "grid".to_string()
}

fn builtin_plugins_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("plugins");
    if from_manifest.exists() {
        return from_manifest;
    }

    // Packaged builds bundle `../plugins` as `$RESOURCE/_up_/plugins` (see tauri.conf.json).
    if let Ok(resolved) = app_handle.path().resolve(
        "../plugins",
        tauri::path::BaseDirectory::Resource,
    ) {
        if resolved.exists() {
            return resolved;
        }
        log::warn!("Resolved plugins path missing: {:?}", resolved);
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        for candidate in [
            resource_dir.join("_up_").join("plugins"),
            resource_dir.join("plugins"),
        ] {
            if candidate.exists() {
                return candidate;
            }
        }
    }

    log::error!("Built-in plugins directory not found; scene layouts will be unavailable");
    PathBuf::from("plugins")
}

fn external_plugins_dir(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    let home_dir = app_handle.path().home_dir().ok()?;
    let dir = home_dir.join("AstroDeck").join("plugins");
    if let Err(err) = fs::create_dir_all(&dir) {
        log::warn!("Could not create external plugins directory {:?}: {}", dir, err);
        return None;
    }
    Some(dir)
}

pub fn load_plugins(app_handle: &tauri::AppHandle) {
    let builtin_dir = builtin_plugins_dir(app_handle);
    let external_dir = external_plugins_dir(app_handle);

    log::info!("Loading built-in plugins from: {:?}", builtin_dir);
    if let Some(dir) = &external_dir {
        log::info!("Loading external plugins from: {:?}", dir);
    }

    let mut merged: HashMap<String, PluginConfig> = HashMap::new();

    for plugin in collect_plugins_from_dir(&builtin_dir, "builtin") {
        merged.insert(plugin.id.clone(), plugin);
    }

    if let Some(dir) = &external_dir {
        for plugin in collect_plugins_from_dir(dir, "external") {
            if let Some(previous) = merged.get(&plugin.id) {
                log::info!(
                    "External plugin '{}' overrides {} plugin from {:?}",
                    plugin.id,
                    previous.source.kind,
                    previous.source.manifest_path
                );
            }
            merged.insert(plugin.id.clone(), plugin);
        }
    }

    let mut plugins: Vec<PluginConfig> = merged.into_values().collect();
    plugins.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.id.cmp(&b.id))
    });

    let action_registry = build_action_registry(&plugins);

    let state = app_handle.state::<crate::AppState>();
    *state.plugins.lock().expect("failed to lock plugins") = plugins;
    *state
        .plugin_actions
        .lock()
        .expect("failed to lock plugin actions") = action_registry;

    log::info!(
        "Loaded {} plugins total ({} declarative actions)",
        state.plugins.lock().expect("failed to lock plugins").len(),
        state
            .plugin_actions
            .lock()
            .expect("failed to lock plugin actions")
            .len()
    );
}

pub fn build_action_registry(plugins: &[PluginConfig]) -> HashMap<String, ActionSpec> {
    let mut registry = HashMap::new();

    for plugin in plugins {
        for button in &plugin.layout.buttons {
            collect_button_action(&mut registry, button);
        }

        if let Some(media) = plugin.view.media_player.as_ref() {
            for button in [
                media.previous.as_ref(),
                media.play_pause.as_ref(),
                media.next.as_ref(),
                media.like.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                collect_button_action(&mut registry, button);
            }
        }
    }

    registry
}

fn collect_button_action(registry: &mut HashMap<String, ActionSpec>, button: &ButtonConfig) {
    if let Some(action_spec) = button.action_spec.clone() {
        registry.insert(button.action.clone(), action_spec);
    }
}

fn collect_plugins_from_dir(dir: &Path, source_kind: &str) -> Vec<PluginConfig> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!("Could not read plugins directory {:?}: {}", dir, err);
            return Vec::new();
        }
    };

    let mut plugins = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(manifest_path) = plugin_manifest_in_dir(&path) {
                if let Some(plugin) = load_plugin_manifest(&manifest_path, source_kind) {
                    plugins.push(plugin);
                }
            }
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            if let Some(plugin) = load_plugin_manifest(&path, source_kind) {
                plugins.push(plugin);
            }
        }
    }

    plugins
}

fn plugin_manifest_in_dir(dir: &Path) -> Option<PathBuf> {
    for candidate in ["plugin.json", "manifest.json"] {
        let path = dir.join(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn load_plugin_manifest(manifest_path: &Path, source_kind: &str) -> Option<PluginConfig> {
    let contents = match fs::read_to_string(manifest_path) {
        Ok(contents) => contents,
        Err(err) => {
            log::error!("Failed to read plugin manifest {:?}: {}", manifest_path, err);
            return None;
        }
    };

    let mut plugin = match serde_json::from_str::<PluginConfig>(&contents) {
        Ok(plugin) => plugin,
        Err(err) => {
            log::error!("Failed to parse plugin manifest {:?}: {}", manifest_path, err);
            return None;
        }
    };

    if let Err(err) = finalize_plugin(&mut plugin, manifest_path, source_kind) {
        log::error!("Failed to normalize plugin {:?}: {}", manifest_path, err);
        return None;
    }

    log::info!(
        "Loaded {} plugin: {} (priority {})",
        source_kind,
        plugin.name,
        plugin.priority
    );
    Some(plugin)
}

fn finalize_plugin(
    plugin: &mut PluginConfig,
    manifest_path: &Path,
    source_kind: &str,
) -> Result<(), String> {
    let base_dir = manifest_path
        .parent()
        .ok_or_else(|| format!("Plugin manifest {:?} has no parent directory", manifest_path))?;

    plugin.source = PluginSourceMeta {
        kind: source_kind.to_string(),
        manifest_path: Some(manifest_path.to_string_lossy().to_string()),
        base_dir: Some(base_dir.to_string_lossy().to_string()),
    };

    for button in &mut plugin.layout.buttons {
        finalize_button(button, base_dir, &plugin.id, None)?;
    }

    if let Some(media) = plugin.view.media_player.as_mut() {
        if let Some(button) = media.previous.as_mut() {
            finalize_button(button, base_dir, &plugin.id, Some("previous"))?;
        }
        if let Some(button) = media.play_pause.as_mut() {
            finalize_button(button, base_dir, &plugin.id, Some("playPause"))?;
        }
        if let Some(button) = media.next.as_mut() {
            finalize_button(button, base_dir, &plugin.id, Some("next"))?;
        }
        if let Some(button) = media.like.as_mut() {
            finalize_button(button, base_dir, &plugin.id, Some("like"))?;
        }
    }

    if plugin.view.kind.is_empty() {
        plugin.view.kind = default_view_type();
    }

    Ok(())
}

fn finalize_button(
    button: &mut ButtonConfig,
    base_dir: &Path,
    plugin_id: &str,
    fallback_id: Option<&str>,
) -> Result<(), String> {
    if let Some(action_spec) = button.action_spec.as_mut() {
        normalize_action_spec_paths(action_spec, base_dir);
    }

    if let Some(image) = button.image.clone() {
        if is_local_asset_path(&image) {
            let asset_path = base_dir.join(&image);
            match encode_local_asset(&asset_path) {
                Some(data_url) => button.image = Some(data_url),
                None => {
                    log::warn!(
                        "Could not resolve plugin image {:?} for button '{}'",
                        asset_path,
                        button.label
                    );
                }
            }
        }
    }

    if button.action.trim().is_empty() && button.action_spec.is_some() {
        let generated_id = button
            .id
            .clone()
            .or_else(|| fallback_id.map(ToString::to_string))
            .unwrap_or_else(|| slugify(&button.label));
        button.id = Some(generated_id.clone());
        button.action = format!("plugin.{}.{}", plugin_id, generated_id);
    }

    if button.action.trim().is_empty() {
        return Err(format!(
            "Button '{}' is missing both 'action' and 'actionSpec'",
            button.label
        ));
    }

    Ok(())
}

fn normalize_action_spec_paths(action_spec: &mut ActionSpec, base_dir: &Path) {
    match action_spec {
        ActionSpec::OpenPath { path } => {
            if is_local_asset_path(path) {
                let relative = path.clone();
                *path = base_dir.join(relative).to_string_lossy().to_string();
            }
        }
        ActionSpec::Launch { program, .. } => {
            if is_local_asset_path(program) {
                let relative = program.clone();
                *program = base_dir.join(relative).to_string_lossy().to_string();
            }
        }
        ActionSpec::OpenUrl { .. } => {}
    }
}

fn is_local_asset_path(value: &str) -> bool {
    !value.starts_with("http://")
        && !value.starts_with("https://")
        && !value.starts_with("data:")
        && !Path::new(value).is_absolute()
}

fn encode_local_asset(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mime = mime_for_path(path);
    Some(format!("data:{};base64,{}", mime, STANDARD.encode(bytes)))
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}
