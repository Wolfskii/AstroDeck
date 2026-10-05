use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppPreferences {
    #[serde(default = "default_start_minimized")]
    pub start_minimized: bool,
    /// Keep the open window off the OS taskbar. The tray icon still opens, hides, and quits.
    #[serde(default)]
    pub hide_taskbar_icon: bool,
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
    #[serde(default)]
    pub audio_visualizer: bool,
    #[serde(default = "default_settings_theme")]
    pub settings_theme: String,
    #[serde(default)]
    pub auto_switch_scenes: HashMap<String, bool>,
    #[serde(default = "default_temperature_unit")]
    pub temperature_unit: String,
    #[serde(default)]
    pub clock_location: String,
    #[serde(default = "default_clock_digit_color")]
    pub clock_digit_color: String,
    #[serde(default)]
    pub clock_digit_border: bool,
    #[serde(default = "default_clock_digit_border_color")]
    pub clock_digit_border_color: String,
    #[serde(default = "default_clock_digit_border_width")]
    pub clock_digit_border_width: u8,
    #[serde(default)]
    pub clock_bg_color_custom: bool,
    #[serde(default = "default_clock_bg_color")]
    pub clock_bg_color: String,
    #[serde(default)]
    pub clock_weather_card: bool,
    #[serde(default = "default_clock_weather_card_color")]
    pub clock_weather_card_color: String,
    #[serde(default)]
    pub custom_views: Vec<CustomViewPref>,
    #[serde(default)]
    pub disabled_apps: Vec<String>,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            start_minimized: default_start_minimized(),
            hide_taskbar_icon: false,
            start_fullscreen: false,
            show_update_popups: default_show_update_popups(),
            scene_background: default_scene_background(),
            show_settings_terminal: false,
            controls_backdrop: default_controls_backdrop(),
            controls_transparency: default_controls_transparency(),
            controls_overlay_color: default_controls_overlay_color(),
            controls_overlay_custom: None,
            audio_visualizer: false,
            settings_theme: default_settings_theme(),
            auto_switch_scenes: HashMap::new(),
            temperature_unit: default_temperature_unit(),
            clock_location: String::new(),
            clock_digit_color: default_clock_digit_color(),
            clock_digit_border: false,
            clock_digit_border_color: default_clock_digit_border_color(),
            clock_digit_border_width: default_clock_digit_border_width(),
            clock_bg_color_custom: false,
            clock_bg_color: default_clock_bg_color(),
            clock_weather_card: false,
            clock_weather_card_color: default_clock_weather_card_color(),
            custom_views: Vec::new(),
            disabled_apps: Vec::new(),
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

fn default_settings_theme() -> String {
    "system".to_string()
}

fn default_temperature_unit() -> String {
    "celsius".to_string()
}

fn parse_temperature_unit(value: &str) -> String {
    match value {
        "fahrenheit" => "fahrenheit".to_string(),
        _ => default_temperature_unit(),
    }
}

fn default_clock_digit_color() -> String {
    "#f3d37a".to_string()
}

fn default_clock_digit_border_color() -> String {
    "#111111".to_string()
}

fn default_clock_digit_border_width() -> u8 {
    4
}

fn default_clock_bg_color() -> String {
    "#3b82f6".to_string()
}

fn default_clock_weather_card_color() -> String {
    "#c5dbe8".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomViewPref {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
}

fn sanitize_custom_view(view: CustomViewPref) -> Result<CustomViewPref, String> {
    let id = view.id.trim().to_string();
    if id.len() < 12
        || id.len() > 64
        || !id.starts_with("view-")
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("Custom view id is invalid".to_string());
    }
    let name = view.name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err("Give the app a name of 40 characters or fewer".to_string());
    }
    let url = sanitize_embed_url(&view.url)?;
    let icon = view.icon.trim().to_ascii_lowercase();
    if icon.is_empty()
        || icon.len() > 48
        || !icon
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("Choose an icon for the app".to_string());
    }
    Ok(CustomViewPref {
        id,
        name: name.to_string(),
        url,
        icon,
        color: parse_hex_color(&view.color, "#38bdf8"),
    })
}

fn sanitize_embed_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 2000 {
        return Err("Enter an http or https website address".to_string());
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let scheme = with_scheme
        .split_once("://")
        .map(|(scheme, _)| scheme.to_ascii_lowercase());
    if scheme.as_deref() != Some("http") && scheme.as_deref() != Some("https") {
        return Err("Only http and https websites can be embedded".to_string());
    }
    if with_scheme.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err("That website address has invalid characters".to_string());
    }
    if let Some(rest) = with_scheme.split_once("://").map(|(_, rest)| rest) {
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        if host.is_empty() || host.contains('@') {
            return Err("Enter a website address without a username or password".to_string());
        }
    }
    Ok(with_scheme)
}

fn sanitize_custom_views(views: Vec<CustomViewPref>) -> Result<Vec<CustomViewPref>, String> {
    if views.len() > 30 {
        return Err("You can add up to 30 apps".to_string());
    }
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::with_capacity(views.len());
    for view in views {
        let next = sanitize_custom_view(view)?;
        if !seen.insert(next.id.clone()) {
            return Err("Two apps have the same id".to_string());
        }
        clean.push(next);
    }
    Ok(clean)
}

fn parse_clock_digit_border_width(value: u8) -> u8 {
    value.clamp(1, 16)
}

fn parse_hex_color(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        trimmed.to_ascii_lowercase()
    } else {
        fallback.to_string()
    }
}

fn parse_clock_digit_color(value: &str) -> String {
    parse_hex_color(value, &default_clock_digit_color())
}

fn parse_settings_theme(value: &str) -> String {
    match value {
        "light" | "dark" | "system" => value.to_string(),
        _ => default_settings_theme(),
    }
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
    "silk",
    "vortex",
    "mosaic",
    "rain",
    "embers",
    "lattice",
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

pub fn hide_taskbar_icon(app: &tauri::AppHandle) -> bool {
    preferences_path(app)
        .and_then(|path| load_preferences(&path))
        .map(|preferences| preferences.hide_taskbar_icon)
        .unwrap_or(false)
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
pub fn get_hide_taskbar_icon(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(hide_taskbar_icon(&app))
}

#[tauri::command]
pub fn set_hide_taskbar_icon(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.hide_taskbar_icon = enabled;
    save_preferences(&path, &preferences)?;
    crate::window_prefs::refresh_taskbar_button(&app);
    Ok(())
}

#[tauri::command]
pub fn get_custom_views(app: tauri::AppHandle) -> Result<Vec<CustomViewPref>, String> {
    let views = load_preferences(&preferences_path(&app)?)?.custom_views;
    Ok(views
        .into_iter()
        .filter_map(|view| sanitize_custom_view(view).ok())
        .take(30)
        .collect())
}

#[tauri::command]
pub fn set_custom_views(
    app: tauri::AppHandle,
    views: Vec<CustomViewPref>,
) -> Result<Vec<CustomViewPref>, String> {
    let views = sanitize_custom_views(views)?;
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.custom_views = views.clone();
    save_preferences(&path, &preferences)?;
    Ok(views)
}

fn known_app_id(id: &str) -> bool {
    matches!(
        id,
        "clock" | "spotify" | "youtubeMusic" | "media" | "teams" | "vscode" | "weather"
    ) || (id.starts_with("view-")
        && (12..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
}

#[tauri::command]
pub fn get_disabled_apps(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let apps = load_preferences(&preferences_path(&app)?)?.disabled_apps;
    Ok(apps.into_iter().filter(|id| known_app_id(id)).collect())
}

#[tauri::command]
pub fn set_app_enabled(
    app: tauri::AppHandle,
    app_id: String,
    enabled: bool,
) -> Result<Vec<String>, String> {
    if !known_app_id(&app_id) {
        return Err("That app is not available".to_string());
    }
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.disabled_apps.retain(|id| id != &app_id && known_app_id(id));
    if !enabled {
        preferences.disabled_apps.push(app_id);
    }
    save_preferences(&path, &preferences)?;
    Ok(preferences.disabled_apps)
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

pub fn audio_visualizer_enabled(app: &tauri::AppHandle) -> bool {
    preferences_path(app)
        .and_then(|path| load_preferences(&path))
        .map(|preferences| preferences.audio_visualizer)
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_audio_visualizer_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(audio_visualizer_enabled(&app))
}

#[tauri::command]
pub fn set_audio_visualizer_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.audio_visualizer = enabled;
    save_preferences(&path, &preferences)?;
    crate::audio_viz::sync(&app, enabled);
    let _ = app.emit("audio-visualizer-changed", enabled);
    Ok(())
}

#[tauri::command]
pub fn get_audio_visualizer_status(
    app: tauri::AppHandle,
) -> Result<crate::audio_viz::AudioVisualizerStatus, String> {
    Ok(crate::audio_viz::status(audio_visualizer_enabled(&app)))
}

#[tauri::command]
pub fn get_settings_theme(app: tauri::AppHandle) -> Result<String, String> {
    Ok(parse_settings_theme(
        &load_preferences(&preferences_path(&app)?)?.settings_theme,
    ))
}

#[tauri::command]
pub fn set_settings_theme(app: tauri::AppHandle, theme: String) -> Result<String, String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.settings_theme = parse_settings_theme(&theme);
    save_preferences(&path, &preferences)?;
    let _ = app.emit("settings-theme-changed", &preferences.settings_theme);
    Ok(preferences.settings_theme)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockSettings {
    pub temperature_unit: String,
    pub location: String,
    pub digit_color: String,
    pub digit_border: bool,
    pub digit_border_color: String,
    pub digit_border_width: u8,
    pub bg_color_custom: bool,
    pub bg_color: String,
    pub weather_card: bool,
    pub weather_card_color: String,
}

fn clock_settings_from(preferences: &AppPreferences) -> ClockSettings {
    ClockSettings {
        temperature_unit: parse_temperature_unit(&preferences.temperature_unit),
        location: preferences.clock_location.trim().to_string(),
        digit_color: parse_clock_digit_color(&preferences.clock_digit_color),
        digit_border: preferences.clock_digit_border,
        digit_border_color: parse_clock_digit_color(&preferences.clock_digit_border_color),
        digit_border_width: parse_clock_digit_border_width(preferences.clock_digit_border_width),
        bg_color_custom: preferences.clock_bg_color_custom,
        bg_color: parse_hex_color(&preferences.clock_bg_color, &default_clock_bg_color()),
        weather_card: preferences.clock_weather_card,
        weather_card_color: parse_hex_color(
            &preferences.clock_weather_card_color,
            &default_clock_weather_card_color(),
        ),
    }
}

#[tauri::command]
pub fn get_clock_settings(app: tauri::AppHandle) -> Result<ClockSettings, String> {
    Ok(clock_settings_from(&load_preferences(
        &preferences_path(&app)?,
    )?))
}

#[tauri::command]
pub fn set_clock_settings(
    app: tauri::AppHandle,
    temperature_unit: String,
    location: String,
    digit_color: String,
    digit_border: bool,
    digit_border_color: String,
    digit_border_width: u8,
    bg_color_custom: bool,
    bg_color: String,
    weather_card: bool,
    weather_card_color: String,
) -> Result<ClockSettings, String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    preferences.temperature_unit = parse_temperature_unit(&temperature_unit);
    preferences.clock_location = location.trim().to_string();
    preferences.clock_digit_color = parse_clock_digit_color(&digit_color);
    preferences.clock_digit_border = digit_border;
    preferences.clock_digit_border_color = parse_clock_digit_color(&digit_border_color);
    preferences.clock_digit_border_width = parse_clock_digit_border_width(digit_border_width);
    preferences.clock_bg_color_custom = bg_color_custom;
    preferences.clock_bg_color = parse_hex_color(&bg_color, &default_clock_bg_color());
    preferences.clock_weather_card = weather_card;
    preferences.clock_weather_card_color =
        parse_hex_color(&weather_card_color, &default_clock_weather_card_color());
    save_preferences(&path, &preferences)?;
    Ok(clock_settings_from(&preferences))
}

#[tauri::command]
pub fn set_audio_visualizer_emit(app: tauri::AppHandle, emit: bool) -> Result<(), String> {
    crate::audio_viz::set_emit_frames(emit);
    crate::audio_viz::sync(&app, audio_visualizer_enabled(&app));
    Ok(())
}

pub fn auto_switch_enabled(app: &tauri::AppHandle, scene_id: &str) -> bool {
    preferences_path(app)
        .and_then(|path| load_preferences(&path))
        .map(|preferences| {
            if preferences.disabled_apps.iter().any(|id| id == scene_id) {
                return false;
            }
            auto_switch_from_map(&preferences.auto_switch_scenes, scene_id)
        })
        .unwrap_or(true)
}

fn auto_switch_from_map(map: &HashMap<String, bool>, scene_id: &str) -> bool {
    map.get(scene_id).copied().unwrap_or(true)
}

#[tauri::command]
pub fn get_auto_switch_scenes(app: tauri::AppHandle) -> Result<HashMap<String, bool>, String> {
    Ok(load_preferences(&preferences_path(&app)?)?.auto_switch_scenes)
}

#[tauri::command]
pub fn set_auto_switch_scene(
    app: tauri::AppHandle,
    scene_id: String,
    enabled: bool,
) -> Result<HashMap<String, bool>, String> {
    let path = preferences_path(&app)?;
    let mut preferences = load_preferences(&path)?;
    let id = scene_id.trim();
    if id.is_empty() {
        return Err("Scene id is required".to_string());
    }
    preferences
        .auto_switch_scenes
        .insert(id.to_string(), enabled);
    save_preferences(&path, &preferences)?;
    let _ = app.emit("auto-switch-scenes-changed", &preferences.auto_switch_scenes);
    Ok(preferences.auto_switch_scenes)
}
