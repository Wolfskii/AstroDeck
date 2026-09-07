use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::{distributions::Alphanumeric, Rng};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Manager;
use url::Url;


const SPOTIFY_AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const SPOTIFY_CURRENT_PLAYBACK_URL: &str = "https://api.spotify.com/v1/me/player";
const SPOTIFY_SET_VOLUME_URL: &str = "https://api.spotify.com/v1/me/player/volume";
const SPOTIFY_SEEK_URL: &str = "https://api.spotify.com/v1/me/player/seek";
const SPOTIFY_SHUFFLE_URL: &str = "https://api.spotify.com/v1/me/player/shuffle";
const SPOTIFY_LIBRARY_URL: &str = "https://api.spotify.com/v1/me/library";
const SPOTIFY_LIBRARY_CONTAINS_URL: &str = "https://api.spotify.com/v1/me/library/contains";
const SPOTIFY_QUEUE_URL: &str = "https://api.spotify.com/v1/me/player/queue";
const SPOTIFY_PLAYLISTS_URL: &str = "https://api.spotify.com/v1/me/playlists";
const SPOTIFY_PLAY_URL: &str = "https://api.spotify.com/v1/me/player/play";
#[derive(Serialize)]
struct LibraryUrisBody {
    uris: Vec<String>,
}
const CUSTOM_SPOTIFY_SCOPES: &str =
    "user-library-modify user-library-read user-read-playback-state user-modify-playback-state playlist-read-private playlist-read-collaborative";
/// Librespot's access-point login only accepts tokens that include `streaming`.
/// Web API playlist/library scopes alone are rejected as "Bad credentials".
const OFFICIAL_SPOTIFY_SCOPES: &str = "streaming";

/// Spotify's official desktop / librespot "keymaster" client id. Already approved,
/// with localhost `/login` redirects registered, so users do not create a developer app.
pub(crate) const OFFICIAL_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const OFFICIAL_REDIRECT_URI: &str = "http://127.0.0.1:8989/login";
const CUSTOM_REDIRECT_URI: &str = "http://127.0.0.1:43821/callback";

#[derive(Default)]
struct SavedTrackCache {
    track_id: Option<String>,
    saved: Option<bool>,
    checked_at: Option<SystemTime>,
    /// Track id we already attempted a library-contains lookup for (avoids 429 spam).
    saved_lookup_track_id: Option<String>,
}

#[derive(Debug, Copy, Clone)]
enum RateLimitScope {
    PlaybackRead,
    Library,
    Playlists,
}

#[derive(Default)]
struct ApiRateLimitState {
    playback_read_until: Option<SystemTime>,
    library_until: Option<SystemTime>,
    playlists_until: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackPreview {
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(rename = "itemType")]
    pub item_type: String,
    #[serde(rename = "trackName")]
    pub track_name: String,
    #[serde(rename = "artistName")]
    pub artist_name: Option<String>,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    #[serde(rename = "coverArtUrl")]
    pub cover_art_url: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
}

#[derive(Default)]
struct PlaybackCache {
    summary: Option<PlaybackSummary>,
    fetched_at: Option<SystemTime>,
    optimistic_item_id: Option<String>,
    optimistic_until: Option<SystemTime>,
}

#[derive(Default)]
struct QueueCache {
    up_next: Vec<TrackPreview>,
    fetched_at: Option<SystemTime>,
}

#[derive(Default)]
struct PlaylistCache {
    offset: u32,
    limit: u32,
    page: Option<SpotifyPlaylistPage>,
    fetched_at: Option<SystemTime>,
}

#[derive(Default)]
struct PlaybackHistory {
    recent: Vec<TrackPreview>,
}

const SPOTIFY_HTTP_TIMEOUT: Duration = Duration::from_secs(12);
const SAVED_TRACK_CACHE_TTL: Duration = Duration::from_secs(120);
/// Reuse /me/player responses for routine status polls (UI extrapolates progress locally).
const PLAYBACK_CACHE_TTL: Duration = Duration::from_secs(30);
const QUEUE_CACHE_TTL: Duration = Duration::from_secs(45);
const PLAYLIST_CACHE_TTL: Duration = Duration::from_secs(300);
const PLAYBACK_HISTORY_MAX: usize = 12;
/// Default backoff when Spotify omits Retry-After on a 429.
const SPOTIFY_DEFAULT_RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(60);
/// Max wait before a single automatic retry (user-initiated actions).
const SPOTIFY_MAX_RETRY_WAIT: Duration = Duration::from_secs(30);
/// Playlist browse should not freeze the overlay for a long Retry-After.
const SPOTIFY_PLAYLIST_RETRY_WAIT: Duration = Duration::from_secs(5);
const SPOTIFY_PLAYLIST_FIELDS: &str =
    "items(id,name,uri,images(url),owner(display_name),tracks(total)),total,limit,offset";

#[derive(Default)]
pub struct SpotifyState {
    pub config: Mutex<SpotifyConfig>,
    pub tokens: Mutex<Option<SpotifyTokens>>,
    pub auth_session: Mutex<Option<PendingAuth>>,
    saved_track_cache: Mutex<SavedTrackCache>,
    api_rate_limit: Mutex<ApiRateLimitState>,
    playback_cache: Mutex<PlaybackCache>,
    queue_cache: Mutex<QueueCache>,
    playlist_cache: Mutex<PlaylistCache>,
    playlist_fetch: Mutex<()>,
    playback_history: Mutex<PlaybackHistory>,
    pub(crate) desktop_session: Mutex<Option<librespot_core::session::Session>>,
    pub(crate) desktop_spirc: Mutex<Option<librespot_connect::Spirc>>,
    pub(crate) desktop_playback: Arc<Mutex<crate::spotify_desktop::OfficialPlayback>>,
}

pub fn invalidate_playback_cache(spotify: &SpotifyState) {
    let mut cache = spotify.playback_cache.lock().unwrap();
    cache.summary = None;
    cache.fetched_at = None;
}

pub fn invalidate_queue_cache(spotify: &SpotifyState) {
    let mut cache = spotify.queue_cache.lock().unwrap();
    cache.up_next.clear();
    cache.fetched_at = None;
}

fn interpolate_playback_progress(summary: PlaybackSummary, fetched_at: SystemTime) -> PlaybackSummary {
    if !summary.is_playing {
        return summary;
    }
    let Some(progress_ms) = summary.progress_ms else {
        return summary;
    };
    let elapsed_ms = SystemTime::now()
        .duration_since(fetched_at)
        .unwrap_or_default()
        .as_millis() as u64;
    let mut summary = summary;
    if let Some(duration_ms) = summary.duration_ms {
        summary.progress_ms = Some((progress_ms + elapsed_ms).min(duration_ms));
    } else {
        summary.progress_ms = Some(progress_ms + elapsed_ms);
    }
    summary
}

fn read_playback_cache(spotify: &SpotifyState, max_age: Duration) -> Option<PlaybackSummary> {
    let cache = spotify.playback_cache.lock().unwrap();
    let summary = cache.summary.as_ref()?;
    let fetched_at = cache.fetched_at?;
    let age = SystemTime::now()
        .duration_since(fetched_at)
        .unwrap_or(Duration::MAX);
    if age > max_age {
        return None;
    }
    Some(interpolate_playback_progress(summary.clone(), fetched_at))
}

fn read_playback_cache_stale(spotify: &SpotifyState) -> Option<PlaybackSummary> {
    let cache = spotify.playback_cache.lock().unwrap();
    let summary = cache.summary.as_ref()?.clone();
    let fetched_at = cache.fetched_at?;
    Some(interpolate_playback_progress(summary, fetched_at))
}

fn get_playback_for_mutation(spotify: &SpotifyState) -> Result<PlaybackSummary, String> {
    if let Some(cached) = read_playback_cache_stale(spotify) {
        return Ok(cached);
    }
    get_current_playback(spotify)
}

fn store_playback_cache(spotify: &SpotifyState, summary: &PlaybackSummary) {
    let previous = spotify
        .playback_cache
        .lock()
        .unwrap()
        .summary
        .clone();
    if let Some(prev) = previous.as_ref() {
        record_track_change(spotify, prev, summary);
    }
    let mut cache = spotify.playback_cache.lock().unwrap();
    cache.summary = Some(summary.clone());
    cache.fetched_at = Some(SystemTime::now());
}

fn update_playback_cache_fields<F>(spotify: &SpotifyState, update: F)
where
    F: FnOnce(&mut PlaybackSummary),
{
    let mut cache = spotify.playback_cache.lock().unwrap();
    if let Some(summary) = cache.summary.as_mut() {
        update(summary);
        cache.fetched_at = Some(SystemTime::now());
    }
}

fn cached_playback_item_id(spotify: &SpotifyState) -> Option<String> {
    spotify
        .playback_cache
        .lock()
        .unwrap()
        .summary
        .as_ref()
        .and_then(|summary| summary.item_id.clone())
}

fn spotify_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(SPOTIFY_HTTP_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

fn retry_after_from_response(response: &reqwest::blocking::Response) -> Duration {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(SPOTIFY_DEFAULT_RATE_LIMIT_BACKOFF)
}

fn scope_rate_limit_until(state: &mut ApiRateLimitState, scope: RateLimitScope) -> &mut Option<SystemTime> {
    match scope {
        RateLimitScope::PlaybackRead => &mut state.playback_read_until,
        RateLimitScope::Library => &mut state.library_until,
        RateLimitScope::Playlists => &mut state.playlists_until,
    }
}

fn set_scope_rate_limit(spotify: &SpotifyState, scope: RateLimitScope, backoff: Duration) {
    let until = SystemTime::now() + backoff;
    let mut state = spotify.api_rate_limit.lock().unwrap();
    let slot = scope_rate_limit_until(&mut state, scope);
    *slot = Some(match *slot {
        Some(existing) if existing > until => existing,
        _ => until,
    });
}

fn clear_scope_rate_limit(spotify: &SpotifyState, scope: RateLimitScope) {
    let mut state = spotify.api_rate_limit.lock().unwrap();
    *scope_rate_limit_until(&mut state, scope) = None;
}

fn scope_rate_limit_wait_remaining(
    spotify: &SpotifyState,
    scope: RateLimitScope,
) -> Option<Duration> {
    let state = spotify.api_rate_limit.lock().unwrap();
    let until = match scope {
        RateLimitScope::PlaybackRead => state.playback_read_until?,
        RateLimitScope::Library => state.library_until?,
        RateLimitScope::Playlists => state.playlists_until?,
    };
    until
        .duration_since(SystemTime::now())
        .ok()
        .filter(|duration| !duration.is_zero())
}

fn is_scope_rate_limited(spotify: &SpotifyState, scope: RateLimitScope) -> bool {
    scope_rate_limit_wait_remaining(spotify, scope).is_some()
}

fn is_playback_read_rate_limited(spotify: &SpotifyState) -> bool {
    is_scope_rate_limited(spotify, RateLimitScope::PlaybackRead)
}

fn is_library_rate_limited(spotify: &SpotifyState) -> bool {
    is_scope_rate_limited(spotify, RateLimitScope::Library)
}

fn scope_rate_limited_error(scope: RateLimitScope) -> String {
    match scope {
        RateLimitScope::PlaybackRead => {
            "Spotify playback read API is rate-limited. Wait for Retry-After, then try again."
                .to_string()
        }
        RateLimitScope::Library => {
            "Spotify library API is rate-limited. Wait for Retry-After, then try again."
                .to_string()
        }
        RateLimitScope::Playlists => {
            "Spotify playlist API is rate-limited. Wait for Retry-After, then try again."
                .to_string()
        }
    }
}

fn apply_scope_rate_limit_from_response(
    spotify: &SpotifyState,
    scope: RateLimitScope,
    response: &reqwest::blocking::Response,
) {
    if response.status().as_u16() != 429 {
        return;
    }
    let backoff = retry_after_from_response(response);
    set_scope_rate_limit(spotify, scope, backoff);
    log::warn!(
        "Spotify {:?} API returned 429; backing off for {} seconds (Retry-After)",
        scope,
        backoff.as_secs()
    );
}

/// Wait until Retry-After expires for a scope. Caps sleep for user-initiated retries.
fn wait_for_scope_rate_limit(
    spotify: &SpotifyState,
    scope: RateLimitScope,
    cap: Option<Duration>,
) {
    let Some(mut remaining) = scope_rate_limit_wait_remaining(spotify, scope) else {
        return;
    };
    if let Some(max) = cap {
        if remaining > max {
            log::info!(
                "Spotify {:?} Retry-After is {}s; waiting {}s before retry",
                scope,
                remaining.as_secs(),
                max.as_secs()
            );
            remaining = max;
        }
    } else {
        log::info!(
            "Spotify {:?} API rate-limited; waiting {}s per Retry-After",
            scope,
            remaining.as_secs()
        );
    }
    std::thread::sleep(remaining);
}

fn ensure_playback_read_available(spotify: &SpotifyState) -> Result<(), String> {
    if is_playback_read_rate_limited(spotify) {
        let seconds = scope_rate_limit_wait_remaining(spotify, RateLimitScope::PlaybackRead)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        return Err(format!(
            "{} Retry again in ~{} seconds.",
            scope_rate_limited_error(RateLimitScope::PlaybackRead),
            seconds
        ));
    }
    Ok(())
}

fn ensure_library_read_available(spotify: &SpotifyState) -> Result<(), String> {
    if is_library_rate_limited(spotify) {
        let seconds = scope_rate_limit_wait_remaining(spotify, RateLimitScope::Library)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        return Err(format!(
            "{} Retry again in ~{} seconds.",
            scope_rate_limited_error(RateLimitScope::Library),
            seconds
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpotifyAuthMode {
    /// OAuth against Spotify's official desktop client id. Default for new installs.
    #[default]
    Official,
    /// User-supplied Web API developer-app client id.
    Custom,
}

impl SpotifyAuthMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Custom => "custom",
        }
    }
}

pub fn uses_web_api(spotify: &SpotifyState) -> bool {
    spotify
        .config
        .lock()
        .map(|config| config.auth_mode == SpotifyAuthMode::Custom)
        .unwrap_or(false)
}

#[derive(Default, Clone)]
pub struct SpotifyConfig {
    pub auth_mode: SpotifyAuthMode,
    /// Effective client id used for OAuth and token refresh.
    pub client_id: String,
    /// User-entered developer-app id, kept when switching back from official login.
    pub custom_client_id: String,
    pub redirect_uri: String,
    pub token_path: Option<PathBuf>,
    /// `app_local_data_dir/spotify_client.json` — used when `SPOTIFY_CLIENT_ID` is unset.
    pub client_store_path: Option<PathBuf>,
    /// Librespot reusable credentials (`credentials.json`) for official desktop login.
    pub desktop_cache_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpotifyClientFile {
    #[serde(default)]
    auth_mode: Option<SpotifyAuthMode>,
    #[serde(default)]
    client_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyClientConfigResponse {
    pub auth_mode: SpotifyAuthMode,
    pub client_id: String,
    pub locked_by_env: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub image_url: Option<String>,
    pub track_count: u32,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyPlaylistPage {
    pub items: Vec<SpotifyPlaylist>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
    pub next_offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PlaylistsResponse {
    items: Vec<PlaylistItem>,
    #[serde(default)]
    offset: u32,
    #[serde(default)]
    limit: u32,
    #[serde(default)]
    total: u32,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    id: Option<String>,
    name: Option<String>,
    uri: Option<String>,
    #[serde(default)]
    images: Vec<PlaybackImage>,
    tracks: Option<PlaylistTracks>,
    owner: Option<PlaylistOwner>,
}

#[derive(Debug, Deserialize)]
struct PlaylistTracks {
    #[serde(default)]
    total: u32,
}

#[derive(Debug, Deserialize)]
struct PlaylistOwner {
    display_name: Option<String>,
}

#[derive(Serialize)]
struct PlayContextBody {
    context_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotifyStatus {
    #[serde(rename = "isConfigured")]
    pub is_configured: bool,
    #[serde(rename = "isAuthenticated")]
    pub is_authenticated: bool,
    #[serde(rename = "hasActiveDevice")]
    pub has_active_device: bool,
    #[serde(rename = "activeDeviceName")]
    pub active_device_name: Option<String>,
    #[serde(rename = "currentTrackName")]
    pub current_track_name: Option<String>,
    #[serde(rename = "currentArtistName")]
    pub current_artist_name: Option<String>,
    #[serde(rename = "currentCoverArtUrl")]
    pub current_cover_art_url: Option<String>,
    #[serde(rename = "currentAlbumName")]
    pub current_album_name: Option<String>,
    #[serde(rename = "progressMs")]
    pub progress_ms: Option<u64>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
    #[serde(rename = "playbackState")]
    pub playback_state: String,
    #[serde(rename = "isPlaying")]
    pub is_playing: bool,
    #[serde(rename = "currentVolumePercent")]
    pub current_volume_percent: Option<u8>,
    #[serde(rename = "currentItemType")]
    pub current_item_type: Option<String>,
    #[serde(rename = "currentItemId")]
    pub current_item_id: Option<String>,
    #[serde(rename = "isCurrentTrackSaved")]
    pub is_current_track_saved: Option<bool>,
    #[serde(rename = "isShuffle")]
    pub is_shuffle: bool,
    #[serde(rename = "grantedScopes")]
    pub granted_scopes: Vec<String>,
    #[serde(rename = "usesWebApi")]
    pub uses_web_api: bool,
    #[serde(rename = "nextTrackPreview", skip_serializing_if = "Option::is_none")]
    pub next_track_preview: Option<TrackPreview>,
    #[serde(rename = "prevTrackPreview", skip_serializing_if = "Option::is_none")]
    pub prev_track_preview: Option<TrackPreview>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct PlaybackSummary {
    pub device_id: String,
    pub device_name: String,
    pub is_playing: bool,
    pub volume_percent: u8,
    pub item_id: Option<String>,
    pub item_type: Option<String>,
    pub item_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub cover_art_url: Option<String>,
    pub progress_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub shuffle_state: bool,
}

#[derive(Debug, Clone)]
pub struct PendingAuth {
    pub state: String,
    pub code_verifier: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    #[serde(default)]
    scope: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueueResponse {
    #[serde(default)]
    queue: Vec<PlaybackItem>,
}

#[derive(Debug, Deserialize)]
struct PlaybackResponse {
    device: PlaybackDevice,
    is_playing: bool,
    progress_ms: Option<u64>,
    item: Option<PlaybackItem>,
    #[serde(default)]
    shuffle_state: bool,
}

#[derive(Debug, Deserialize)]
struct PlaybackDevice {
    id: Option<String>,
    name: String,
    volume_percent: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct PlaybackItem {
    id: Option<String>,
    name: String,
    #[serde(rename = "type")]
    item_type: String,
    duration_ms: Option<u64>,
    #[serde(default)]
    artists: Vec<PlaybackArtist>,
    album: Option<PlaybackAlbum>,
    #[serde(default)]
    images: Vec<PlaybackImage>,
}

#[derive(Debug, Deserialize)]
struct PlaybackAlbum {
    name: Option<String>,
    #[serde(default)]
    images: Vec<PlaybackImage>,
}

#[derive(Debug, Deserialize, Clone)]
struct PlaybackImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct PlaybackArtist {
    name: String,
}

fn read_stored_client_file(path: &PathBuf) -> Option<SpotifyClientFile> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn env_client_id() -> String {
    std::env::var("SPOTIFY_CLIENT_ID")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn env_redirect_uri() -> Option<String> {
    let value = std::env::var("SPOTIFY_REDIRECT_URI")
        .unwrap_or_default()
        .trim()
        .to_string();
    (!value.is_empty()).then_some(value)
}

fn resolve_auth_mode(stored: Option<&SpotifyClientFile>, env_id: &str) -> SpotifyAuthMode {
    if !env_id.is_empty() {
        return SpotifyAuthMode::Custom;
    }
    match stored {
        Some(file) => match file.auth_mode {
            Some(mode) => mode,
            None if !file.client_id.trim().is_empty() => SpotifyAuthMode::Custom,
            None => SpotifyAuthMode::Official,
        },
        None => SpotifyAuthMode::Official,
    }
}

fn stored_custom_client_id(stored: Option<&SpotifyClientFile>, env_id: &str) -> String {
    if !env_id.is_empty() {
        return env_id.to_string();
    }
    stored
        .map(|file| file.client_id.trim().to_string())
        .unwrap_or_default()
}

fn effective_client_id(mode: SpotifyAuthMode, custom_client_id: &str, env_id: &str) -> String {
    if !env_id.is_empty() {
        return env_id.to_string();
    }
    match mode {
        SpotifyAuthMode::Official => OFFICIAL_CLIENT_ID.to_string(),
        SpotifyAuthMode::Custom => custom_client_id.trim().to_string(),
    }
}

fn redirect_uri_for_mode(mode: SpotifyAuthMode) -> String {
    redirect_uri_for_mode_with(mode, env_redirect_uri().as_deref())
}

fn redirect_uri_for_mode_with(mode: SpotifyAuthMode, env_redirect: Option<&str>) -> String {
    match mode {
        SpotifyAuthMode::Official => OFFICIAL_REDIRECT_URI.to_string(),
        SpotifyAuthMode::Custom => env_redirect
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| CUSTOM_REDIRECT_URI.to_string()),
    }
}

fn apply_mode_to_config(config: &mut SpotifyConfig, mode: SpotifyAuthMode, env_id: &str) {
    config.auth_mode = mode;
    config.client_id = effective_client_id(mode, &config.custom_client_id, env_id);
    config.redirect_uri = redirect_uri_for_mode(mode);
}

fn oauth_scopes(mode: SpotifyAuthMode) -> &'static str {
    match mode {
        SpotifyAuthMode::Official => OFFICIAL_SPOTIFY_SCOPES,
        SpotifyAuthMode::Custom => CUSTOM_SPOTIFY_SCOPES,
    }
}

fn persist_client_file(spotify: &SpotifyState) -> Result<(), String> {
    let (store_path, file) = {
        let guard = spotify.config.lock().map_err(|e| e.to_string())?;
        let path = guard
            .client_store_path
            .clone()
            .ok_or_else(|| "Spotify paths not initialized.".to_string())?;
        (
            path,
            SpotifyClientFile {
                auth_mode: Some(guard.auth_mode),
                client_id: guard.custom_client_id.clone(),
            },
        )
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    fs::write(store_path, json).map_err(|e| e.to_string())
}

fn migrate_legacy_spotify_files(app: &tauri::AppHandle, new_base: &PathBuf) {
    let Ok(home) = app.path().home_dir() else {
        return;
    };

    let old_base = if cfg!(windows) {
        home.join("AppData").join("Roaming").join("com.taptapdeck.app")
    } else if cfg!(target_os = "macos") {
        home.join("Library")
            .join("Application Support")
            .join("com.taptapdeck.app")
    } else {
        home.join(".local").join("share").join("com.taptapdeck.app")
    };

    if old_base == *new_base {
        return;
    }

    for file in ["spotify_tokens.json", "spotify_client.json"] {
        let old_file = old_base.join(file);
        let new_file = new_base.join(file);
        if !old_file.exists() || new_file.exists() {
            continue;
        }
        match fs::copy(&old_file, &new_file) {
            Ok(_) => log::info!("Migrated {} from legacy TapTapDeck app data", file),
            Err(err) => log::warn!("Failed to migrate {}: {}", file, err),
        }
    }
}

pub fn init(app: &tauri::AppHandle, spotify: &SpotifyState) -> Result<(), String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    migrate_legacy_spotify_files(app, &base);

    let client_store_path = base.join("spotify_client.json");
    let token_path = base.join("spotify_tokens.json");
    let desktop_cache_path = base.join("spotify_desktop");

    let from_env = env_client_id();
    let stored = read_stored_client_file(&client_store_path);
    let auth_mode = resolve_auth_mode(stored.as_ref(), &from_env);
    let custom_client_id = stored_custom_client_id(stored.as_ref(), &from_env);
    let client_id = effective_client_id(auth_mode, &custom_client_id, &from_env);
    let redirect_uri = redirect_uri_for_mode(auth_mode);

    {
        let mut config = spotify.config.lock().map_err(|e| e.to_string())?;
        config.auth_mode = auth_mode;
        config.client_id = client_id;
        config.custom_client_id = custom_client_id;
        config.redirect_uri = redirect_uri;
        config.token_path = Some(token_path.clone());
        config.client_store_path = Some(client_store_path);
        config.desktop_cache_path = Some(desktop_cache_path);
    }

    if token_path.exists() {
        match fs::read_to_string(&token_path) {
            Ok(contents) => match serde_json::from_str::<SpotifyTokens>(&contents) {
                Ok(tokens) => {
                    let mut stored = spotify.tokens.lock().map_err(|e| e.to_string())?;
                    *stored = Some(tokens);
                }
                Err(e) => {
                    log::warn!("Failed to parse stored Spotify tokens: {}", e);
                }
            },
            Err(e) => log::warn!("Failed to read stored Spotify tokens: {}", e),
        }
    }

    Ok(())
}

pub fn get_client_config_for_ui(spotify: &SpotifyState) -> SpotifyClientConfigResponse {
    let locked_by_env = !env_client_id().is_empty();
    let (auth_mode, client_id) = spotify
        .config
        .lock()
        .map(|c| (c.auth_mode, c.custom_client_id.clone()))
        .unwrap_or_default();
    SpotifyClientConfigResponse {
        auth_mode,
        client_id,
        locked_by_env,
    }
}

pub fn set_auth_mode_from_settings(
    spotify: &SpotifyState,
    mode: SpotifyAuthMode,
) -> Result<(), String> {
    let env_id = env_client_id();
    if !env_id.is_empty() {
        return Err(
            "SPOTIFY_CLIENT_ID is set in the environment; unset it to change the Spotify login method."
                .to_string(),
        );
    }

    let previous_client_id = {
        let mut cfg = spotify.config.lock().map_err(|e| e.to_string())?;
        let previous = cfg.client_id.clone();
        apply_mode_to_config(&mut cfg, mode, "");
        previous
    };
    persist_client_file(spotify)?;
    log::info!("Spotify auth mode set to {}", mode.as_str());

    let next_client_id = spotify
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .client_id
        .clone();
    if previous_client_id != next_client_id && has_saved_tokens(spotify) {
        disconnect(spotify)?;
    }
    Ok(())
}

pub fn set_client_id_from_settings(
    spotify: &SpotifyState,
    client_id: &str,
) -> Result<(), String> {
    let env_id = env_client_id();
    if !env_id.is_empty() {
        return Err(
            "SPOTIFY_CLIENT_ID is set in the environment; unset it to save a Client ID from Settings."
                .to_string(),
        );
    }

    let trimmed = client_id.trim().to_string();
    let previous_client_id = {
        let mut cfg = spotify.config.lock().map_err(|e| e.to_string())?;
        let previous = cfg.client_id.clone();
        cfg.custom_client_id = trimmed.clone();
        apply_mode_to_config(&mut cfg, SpotifyAuthMode::Custom, "");
        previous
    };
    persist_client_file(spotify)?;

    let next_client_id = spotify
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .client_id
        .clone();
    if previous_client_id != next_client_id && has_saved_tokens(spotify) {
        disconnect(spotify)?;
    }
    Ok(())
}

fn preview_image_url(item: &PlaybackItem) -> Option<String> {
    item.album
        .as_ref()
        .and_then(|album| album.images.first())
        .map(|image| image.url.clone())
        .or_else(|| item.images.first().map(|image| image.url.clone()))
}

fn playback_item_to_preview(item: &PlaybackItem) -> Option<TrackPreview> {
    let item_id = item.id.clone()?;
    Some(TrackPreview {
        item_id,
        item_type: item.item_type.clone(),
        track_name: item.name.clone(),
        artist_name: item
            .artists
            .first()
            .map(|artist| artist.name.clone()),
        album_name: item.album.as_ref().and_then(|album| album.name.clone()),
        cover_art_url: preview_image_url(item),
        duration_ms: item.duration_ms,
    })
}

fn playback_summary_to_preview(summary: &PlaybackSummary) -> Option<TrackPreview> {
    let item_id = summary.item_id.clone()?;
    Some(TrackPreview {
        item_id,
        item_type: summary.item_type.clone().unwrap_or_else(|| "track".to_string()),
        track_name: summary.item_name.clone().unwrap_or_default(),
        artist_name: summary.artist_name.clone(),
        album_name: summary.album_name.clone(),
        cover_art_url: summary.cover_art_url.clone(),
        duration_ms: summary.duration_ms,
    })
}

fn preview_to_playback_summary(preview: &TrackPreview, base: &PlaybackSummary) -> PlaybackSummary {
    PlaybackSummary {
        device_id: base.device_id.clone(),
        device_name: base.device_name.clone(),
        is_playing: base.is_playing,
        volume_percent: base.volume_percent,
        item_id: Some(preview.item_id.clone()),
        item_type: Some(preview.item_type.clone()),
        item_name: Some(preview.track_name.clone()),
        artist_name: preview.artist_name.clone(),
        album_name: preview.album_name.clone(),
        cover_art_url: preview.cover_art_url.clone(),
        progress_ms: Some(0),
        duration_ms: preview.duration_ms,
        shuffle_state: base.shuffle_state,
    }
}

fn dedupe_queue_previews(items: Vec<TrackPreview>) -> Vec<TrackPreview> {
    let mut deduped: Vec<TrackPreview> = Vec::new();
    for item in items {
        if deduped
            .last()
            .is_some_and(|last| last.item_id == item.item_id)
        {
            continue;
        }
        deduped.push(item);
    }
    deduped
}

fn push_playback_history(spotify: &SpotifyState, preview: TrackPreview) {
    let mut history = spotify.playback_history.lock().unwrap();
    if history.recent.first().map(|t| t.item_id.as_str()) == Some(preview.item_id.as_str()) {
        return;
    }
    history.recent.retain(|track| track.item_id != preview.item_id);
    history.recent.insert(0, preview);
    history.recent.truncate(PLAYBACK_HISTORY_MAX);
}

fn record_track_change(
    spotify: &SpotifyState,
    previous: &PlaybackSummary,
    next: &PlaybackSummary,
) {
    if previous.item_id == next.item_id {
        return;
    }
    if let Some(preview) = playback_summary_to_preview(previous) {
        push_playback_history(spotify, preview);
    }
}

fn next_preview_from_cache(spotify: &SpotifyState, current_id: Option<&str>) -> Option<TrackPreview> {
    let cache = spotify.queue_cache.lock().unwrap();
    cache
        .up_next
        .iter()
        .find(|track| Some(track.item_id.as_str()) != current_id)
        .cloned()
}

fn prev_preview_from_history(spotify: &SpotifyState, current_id: Option<&str>) -> Option<TrackPreview> {
    let history = spotify.playback_history.lock().unwrap();
    history
        .recent
        .iter()
        .find(|track| Some(track.item_id.as_str()) != current_id)
        .cloned()
}

fn consume_queue_preview(spotify: &SpotifyState, item_id: &str) {
    let mut cache = spotify.queue_cache.lock().unwrap();
    if let Some(index) = cache.up_next.iter().position(|track| track.item_id == item_id) {
        cache.up_next.remove(index);
    }
}

fn fetch_playback_queue(spotify: &SpotifyState) -> Result<Vec<TrackPreview>, String> {
    ensure_playback_read_available(spotify)?;
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;
    let response = client
        .get(SPOTIFY_QUEUE_URL)
        .bearer_auth(&access_token)
        .send()
        .map_err(|e| format!("Spotify queue request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        apply_scope_rate_limit_from_response(spotify, RateLimitScope::PlaybackRead, &response);
        let body = response.text().unwrap_or_default();
        return Err(format!("Spotify queue query failed: {} {}", status, body));
    }

    let queue: QueueResponse = response.json().map_err(|e| e.to_string())?;
    Ok(queue
        .queue
        .iter()
        .filter_map(playback_item_to_preview)
        .collect())
}

fn maybe_refresh_queue_cache(spotify: &SpotifyState, force: bool) {
    if !force {
        let cache = spotify.queue_cache.lock().unwrap();
        if let Some(fetched_at) = cache.fetched_at {
            if SystemTime::now()
                .duration_since(fetched_at)
                .unwrap_or(Duration::MAX)
                < QUEUE_CACHE_TTL
            {
                return;
            }
        }
    }

    if is_playback_read_rate_limited(spotify) {
        return;
    }

    match fetch_playback_queue(spotify) {
        Ok(items) => {
            let mut cache = spotify.queue_cache.lock().unwrap();
            cache.up_next = dedupe_queue_previews(items);
            cache.fetched_at = Some(SystemTime::now());
        }
        Err(err) => {
            log::debug!("Spotify queue prefetch skipped: {}", err);
        }
    }
}

const PREV_RESTART_THRESHOLD_MS: u64 = 3000;
const OPTIMISTIC_SKIP_HOLD: Duration = Duration::from_secs(5);

fn pin_optimistic_item(spotify: &SpotifyState, item_id: &str) {
    let mut cache = spotify.playback_cache.lock().unwrap();
    cache.optimistic_item_id = Some(item_id.to_string());
    cache.optimistic_until = Some(SystemTime::now() + OPTIMISTIC_SKIP_HOLD);
}

fn store_playback_from_api(spotify: &SpotifyState, summary: &PlaybackSummary) -> PlaybackSummary {
    {
        let mut cache = spotify.playback_cache.lock().unwrap();
        let pin_expired = cache
            .optimistic_until
            .map(|until| SystemTime::now() > until)
            .unwrap_or(true);
        if pin_expired {
            cache.optimistic_item_id = None;
            cache.optimistic_until = None;
        } else if let Some(pinned) = cache.optimistic_item_id.clone() {
            if summary.item_id.as_deref() == Some(pinned.as_str()) {
                cache.optimistic_item_id = None;
                cache.optimistic_until = None;
            } else if cache.summary.is_some() {
                if let Some(current) = cache.summary.as_mut() {
                    current.is_playing = summary.is_playing;
                    current.volume_percent = summary.volume_percent;
                    current.shuffle_state = summary.shuffle_state;
                    current.device_id = summary.device_id.clone();
                    current.device_name = summary.device_name.clone();
                }
                cache.fetched_at = Some(SystemTime::now());
                return cache.summary.clone().unwrap();
            }
        }
    }
    store_playback_cache(spotify, summary);
    summary.clone()
}

pub fn apply_optimistic_skip(spotify: &SpotifyState, direction: &str) -> Option<TrackPreview> {
    maybe_refresh_queue_cache(spotify, false);
    let current = read_playback_cache_stale(spotify)?;
    let preview = match direction {
        "next" => next_preview_from_cache(spotify, current.item_id.as_deref())?,
        "prev" => {
            let progress_ms = current.progress_ms.unwrap_or(0);
            if progress_ms > PREV_RESTART_THRESHOLD_MS {
                let mut optimistic = current.clone();
                optimistic.progress_ms = Some(0);
                store_playback_cache(spotify, &optimistic);
                return None;
            }
            prev_preview_from_history(spotify, current.item_id.as_deref())?
        }
        _ => return None,
    };

    if direction == "next" {
        consume_queue_preview(spotify, &preview.item_id);
    }

    let optimistic = preview_to_playback_summary(&preview, &current);
    store_playback_cache(spotify, &optimistic);
    pin_optimistic_item(spotify, &preview.item_id);
    Some(preview)
}

pub fn get_status(spotify: &SpotifyState) -> Result<SpotifyStatus, String> {
    if !uses_web_api(spotify) {
        return official_desktop_status(spotify);
    }
    build_status(spotify, false)
}

pub fn get_status_fresh(spotify: &SpotifyState) -> Result<SpotifyStatus, String> {
    if !uses_web_api(spotify) {
        return official_desktop_status(spotify);
    }
    build_status(spotify, true)
}

fn official_desktop_status(spotify: &SpotifyState) -> Result<SpotifyStatus, String> {
    let stored_tokens = spotify.tokens.lock().map_err(|e| e.to_string())?.clone();
    let granted_scopes = stored_tokens
        .as_ref()
        .map(|tokens| parse_scopes(&tokens.scope))
        .unwrap_or_default();
    let authenticated = stored_tokens.is_some();
    let playback = crate::spotify_desktop::playback_snapshot(spotify);
    let needs_desktop_reconnect = authenticated
        && !granted_scopes.is_empty()
        && !granted_scopes.iter().any(|scope| scope == "streaming");

    if !authenticated {
        return Ok(SpotifyStatus {
            is_configured: true,
            is_authenticated: false,
            has_active_device: false,
            active_device_name: None,
            current_track_name: None,
            current_artist_name: None,
            current_cover_art_url: None,
            current_album_name: None,
            progress_ms: None,
            duration_ms: None,
            playback_state: "stopped".to_string(),
            is_playing: false,
            current_volume_percent: Some(playback.volume_percent),
            current_item_type: None,
            current_item_id: None,
            is_current_track_saved: None,
            is_shuffle: false,
            granted_scopes,
            uses_web_api: false,
            next_track_preview: None,
            prev_track_preview: None,
            message: "Sign in with Spotify desktop login. AstroDeck streams playback itself — no developer app, no Web API, and the Spotify desktop app is not required. Premium is required.".to_string(),
        });
    }

    let playback_state = if playback.is_playing {
        "playing"
    } else if playback.track_name.is_some() {
        "paused"
    } else {
        "stopped"
    };

    Ok(SpotifyStatus {
        is_configured: true,
        is_authenticated: true,
        has_active_device: playback.player_ready,
        active_device_name: playback
            .player_ready
            .then(|| crate::spotify_desktop::OFFICIAL_DEVICE_NAME.to_string()),
        current_track_name: playback.track_name,
        current_artist_name: playback.artist_name,
        current_cover_art_url: playback.cover_url,
        current_album_name: playback.album_name,
        progress_ms: playback.progress_ms,
        duration_ms: playback.duration_ms,
        playback_state: playback_state.to_string(),
        is_playing: playback.is_playing,
        current_volume_percent: Some(playback.volume_percent),
        current_item_type: playback.item_type,
        current_item_id: playback.item_id,
        is_current_track_saved: None,
        is_shuffle: playback.shuffle,
        granted_scopes,
        uses_web_api: false,
        next_track_preview: None,
        prev_track_preview: None,
        message: if needs_desktop_reconnect {
            "Spotify desktop login needs a fresh connect for playlists. Disconnect and connect again so the browser prompt can grant desktop access.".to_string()
        } else if playback.player_ready {
            "Spotify desktop login is connected. AstroDeck is the player — playlists, volume, seek, and transport stream here. The Spotify app is not required.".to_string()
        } else {
            "Spotify desktop login is connected. Start a playlist to stream in AstroDeck — the Spotify app is not required.".to_string()
        },
    })
}

fn build_status(spotify: &SpotifyState, fresh_playback: bool) -> Result<SpotifyStatus, String> {
    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    let configured = !config.client_id.is_empty();
    let stored_tokens = spotify.tokens.lock().map_err(|e| e.to_string())?.clone();
    let authenticated = stored_tokens.is_some();
    let granted_scopes = stored_tokens
        .as_ref()
        .map(|tokens| parse_scopes(&tokens.scope))
        .unwrap_or_default();

    if !configured {
        return Ok(SpotifyStatus {
            is_configured: false,
            is_authenticated: false,
            has_active_device: false,
            active_device_name: None,
            current_track_name: None,
            current_artist_name: None,
            current_cover_art_url: None,
            current_album_name: None,
            progress_ms: None,
            duration_ms: None,
            playback_state: "stopped".to_string(),
            is_playing: false,
            current_volume_percent: None,
            current_item_type: None,
            current_item_id: None,
            is_current_track_saved: None,
            is_shuffle: false,
            granted_scopes,
            uses_web_api: true,
            next_track_preview: None,
            prev_track_preview: None,
            message: "Spotify developer-app login needs a Client ID. Save one in Settings, or switch to Spotify desktop login."
                .to_string(),
        });
    }

    if !authenticated {
        return Ok(SpotifyStatus {
            is_configured: true,
            is_authenticated: false,
            has_active_device: false,
            active_device_name: None,
            current_track_name: None,
            current_artist_name: None,
            current_cover_art_url: None,
            current_album_name: None,
            progress_ms: None,
            duration_ms: None,
            playback_state: "stopped".to_string(),
            is_playing: false,
            current_volume_percent: None,
            current_item_type: None,
            current_item_id: None,
            is_current_track_saved: None,
            is_shuffle: false,
            granted_scopes,
            uses_web_api: true,
            next_track_preview: None,
            prev_track_preview: None,
            message: "Spotify is not connected yet.".to_string(),
        });
    }

    maybe_refresh_queue_cache(spotify, fresh_playback);

    let playback_result = if fresh_playback {
        refresh_current_playback(spotify)
    } else {
        get_current_playback(spotify)
    };

    match playback_result {
        Ok(playback) => build_spotify_status_from_playback(
            spotify,
            &playback,
            granted_scopes,
            "Spotify is connected.".to_string(),
            fresh_playback,
        ),
        Err(err) => {
            if let Some(playback) = read_playback_cache_stale(spotify) {
                let message = format!("{err} Showing last known playback.");
                build_spotify_status_from_playback(
                    spotify,
                    &playback,
                    granted_scopes,
                    message,
                    false,
                )
            } else {
                Ok(SpotifyStatus {
                    is_configured: true,
                    is_authenticated: true,
                    has_active_device: false,
                    active_device_name: None,
                    current_track_name: None,
                    current_artist_name: None,
                    current_cover_art_url: None,
                    current_album_name: None,
                    progress_ms: None,
                    duration_ms: None,
                    playback_state: "stopped".to_string(),
                    is_playing: false,
                    current_volume_percent: None,
                    current_item_type: None,
                    current_item_id: None,
                    is_current_track_saved: None,
                    is_shuffle: false,
                    granted_scopes,
                    uses_web_api: true,
                    next_track_preview: None,
                    prev_track_preview: None,
                    message: err,
                })
            }
        }
    }
}

fn build_spotify_status_from_playback(
    spotify: &SpotifyState,
    playback: &PlaybackSummary,
    granted_scopes: Vec<String>,
    mut message: String,
    fresh_playback: bool,
) -> Result<SpotifyStatus, String> {
    if is_playback_read_rate_limited(spotify) {
        message.push_str(" Playback reads are temporarily rate-limited.");
    }

    let is_current_track_saved = if playback.item_type.as_deref() == Some("track") {
        if has_scope(spotify, "user-library-read")? {
            playback
                .item_id
                .as_deref()
                .map(|track_id| resolve_display_saved_state(spotify, track_id, fresh_playback))
                .unwrap_or(None)
        } else {
            message =
                "Spotify is connected. Reconnect Spotify to enable saved-track status.".to_string();
            None
        }
    } else {
        None
    };

    Ok(SpotifyStatus {
        is_configured: true,
        is_authenticated: true,
        has_active_device: true,
        active_device_name: Some(playback.device_name.clone()),
        current_track_name: playback.item_name.clone(),
        current_artist_name: playback.artist_name.clone(),
        current_cover_art_url: playback.cover_art_url.clone(),
        current_album_name: playback.album_name.clone(),
        progress_ms: playback.progress_ms,
        duration_ms: playback.duration_ms,
        playback_state: if playback.is_playing {
            "playing".to_string()
        } else {
            "paused".to_string()
        },
        is_playing: playback.is_playing,
        current_volume_percent: Some(playback.volume_percent),
        current_item_type: playback.item_type.clone(),
        current_item_id: playback.item_id.clone(),
        is_current_track_saved,
        is_shuffle: playback.shuffle_state,
        granted_scopes,
        uses_web_api: true,
        next_track_preview: next_preview_from_cache(spotify, playback.item_id.as_deref()),
        prev_track_preview: prev_preview_from_history(spotify, playback.item_id.as_deref()),
        message,
    })
}

pub fn has_saved_tokens(spotify: &SpotifyState) -> bool {
    spotify
        .tokens
        .lock()
        .ok()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

pub fn disconnect(spotify: &SpotifyState) -> Result<(), String> {
    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(path) = config.token_path {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }

    {
        let mut tokens = spotify.tokens.lock().map_err(|e| e.to_string())?;
        *tokens = None;
    }
    {
        let mut pending = spotify.auth_session.lock().map_err(|e| e.to_string())?;
        *pending = None;
    }
    {
        let mut cache = spotify.playlist_cache.lock().map_err(|e| e.to_string())?;
        *cache = PlaylistCache::default();
    }
    crate::spotify_desktop::clear_credentials(spotify);

    Ok(())
}

pub fn start_auth_flow(spotify: &SpotifyState) -> Result<String, String> {
    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    if config.client_id.is_empty() {
        return Err(match config.auth_mode {
            SpotifyAuthMode::Official => {
                "Official Spotify desktop login is not configured.".to_string()
            }
            SpotifyAuthMode::Custom => {
                "Save your Spotify Client ID in Settings, or switch to Spotify desktop login."
                    .to_string()
            }
        });
    }

    let state = random_string(24);
    let code_verifier = random_string(96);
    let code_challenge = build_code_challenge(&code_verifier);

    {
        let mut pending = spotify.auth_session.lock().map_err(|e| e.to_string())?;
        *pending = Some(PendingAuth {
            state: state.clone(),
            code_verifier,
        });
    }

    let mut url = Url::parse(SPOTIFY_AUTHORIZE_URL).map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("scope", oauth_scopes(config.auth_mode))
        .append_pair("state", &state)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &code_challenge)
        .append_pair("show_dialog", "true");

    Ok(url.to_string())
}

pub fn complete_auth_via_callback(spotify: &SpotifyState) -> Result<(), String> {
    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    let redirect_uri = Url::parse(&config.redirect_uri).map_err(|e| e.to_string())?;
    let host = redirect_uri
        .host_str()
        .ok_or_else(|| "SPOTIFY_REDIRECT_URI must include a host".to_string())?;
    let port = redirect_uri
        .port_or_known_default()
        .ok_or_else(|| "SPOTIFY_REDIRECT_URI must include a port".to_string())?;
    let expected_path = redirect_uri.path().to_string();

    let listener = TcpListener::bind((host, port)).map_err(|e| e.to_string())?;
    listener
        .set_nonblocking(false)
        .map_err(|e| e.to_string())?;
    listener
        .set_ttl(64)
        .map_err(|e| e.to_string())?;

    let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;

    let mut request = [0_u8; 4096];
    let bytes_read = stream.read(&mut request).map_err(|e| e.to_string())?;
    let request_text = String::from_utf8_lossy(&request[..bytes_read]).to_string();
    let request_line = request_text
        .lines()
        .next()
        .ok_or_else(|| "Missing HTTP request line from Spotify callback".to_string())?;
    let path_with_query = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "Missing callback path from Spotify redirect".to_string())?;

    let callback_url =
        Url::parse(&format!("http://localhost{}", path_with_query)).map_err(|e| e.to_string())?;
    if callback_url.path() != expected_path {
        write_html_response(&mut stream, false, "Unexpected callback path")?;
        return Err("Spotify callback path did not match redirect URI".to_string());
    }

    let code = callback_url
        .query_pairs()
        .find_map(|(k, v)| (k == "code").then(|| v.to_string()));
    let returned_state = callback_url
        .query_pairs()
        .find_map(|(k, v)| (k == "state").then(|| v.to_string()));
    let error = callback_url
        .query_pairs()
        .find_map(|(k, v)| (k == "error").then(|| v.to_string()));

    if let Some(error) = error {
        write_html_response(&mut stream, false, "Spotify authorization failed")?;
        return Err(format!("Spotify authorization error: {}", error));
    }

    let code = code.ok_or_else(|| "Spotify callback did not include an authorization code".to_string())?;
    let returned_state =
        returned_state.ok_or_else(|| "Spotify callback did not include a state value".to_string())?;

    let pending = spotify
        .auth_session
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Spotify authorization session was not initialized".to_string())?;

    if pending.state != returned_state {
        write_html_response(&mut stream, false, "Spotify authorization state mismatch")?;
        return Err("Spotify callback state did not match the pending auth session".to_string());
    }

    let client = Client::new();
    let response = client
        .post(SPOTIFY_TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("code_verifier", pending.code_verifier.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        write_html_response(&mut stream, false, "Spotify token exchange failed")?;
        return Err(format!("Spotify token exchange failed: {} {}", status, body));
    }

    let token_response: TokenResponse = response.json().map_err(|e| e.to_string())?;
    let tokens = tokens_from_response(token_response, None)?;
    persist_tokens(spotify, tokens)?;
    clear_pending_auth(spotify)?;
    write_html_response(&mut stream, true, "Spotify connected. You can close this tab now.")?;
    if uses_web_api(spotify) {
        if let Err(err) = list_playlists(spotify, 0, 50) {
            log::info!("Spotify playlist cache warm after login skipped: {err}");
        }
    } else {
        crate::spotify_desktop::clear_credentials(spotify);
        if let Err(err) = crate::spotify_desktop::ensure_player(spotify) {
            log::info!("Spotify player after login skipped: {err}");
            if let Err(err) = crate::spotify_desktop::ensure_session(spotify) {
                log::info!("Spotify desktop session after login skipped: {err}");
            }
        }
        if let Err(err) = crate::spotify_desktop::list_playlists(spotify, 0, 50) {
            log::info!("Spotify desktop playlist cache warm after login skipped: {err}");
        }
    }
    Ok(())
}

fn web_api_required(action: &str) -> String {
    format!(
        "{action} uses Spotify's Web API. Switch Login method to Your Spotify developer app."
    )
}

pub fn toggle_current_track_saved(spotify: &SpotifyState) -> Result<bool, String> {
    if !uses_web_api(spotify) {
        return Err(web_api_required("Like"));
    }
    ensure_scope(spotify, "user-library-modify")?;
    let playback = get_playback_for_mutation(spotify)?;
    let item_id = playback
        .item_id
        .ok_or_else(|| "Spotify has no current item to save".to_string())?;
    let item_type = playback
        .item_type
        .clone()
        .unwrap_or_else(|| "track".to_string());
    if item_type != "track" {
        return Err(format!(
            "Spotify like/dislike currently supports tracks only, but the active item type is '{}'",
            item_type
        ));
    }

    let already_saved = resolve_saved_track_state(spotify, &item_id);
    apply_track_saved_state(spotify, &item_id, !already_saved)
}

pub fn set_current_track_saved(
    spotify: &SpotifyState,
    should_save: bool,
) -> Result<bool, String> {
    if !uses_web_api(spotify) {
        return Err(web_api_required("Like"));
    }
    ensure_scope(spotify, "user-library-modify")?;
    let (item_id, item_type) = match cached_playback_item_id(spotify) {
        Some(item_id) => (item_id, "track".to_string()),
        None => {
            let playback = get_playback_for_mutation(spotify)?;
            let item_id = playback
                .item_id
                .ok_or_else(|| "Spotify has no current item to save".to_string())?;
            let item_type = playback
                .item_type
                .clone()
                .unwrap_or_else(|| "track".to_string());
            (item_id, item_type)
        }
    };
    if item_type != "track" {
        return Err(format!(
            "Spotify like/dislike currently supports tracks only, but the active item type is '{}'",
            item_type
        ));
    }

    apply_track_saved_state(spotify, &item_id, should_save)
}

fn apply_track_saved_state(
    spotify: &SpotifyState,
    item_id: &str,
    should_save: bool,
) -> Result<bool, String> {
    log::info!(
        "Spotify: attempting track library update for item_id='{}' should_save='{}'",
        item_id,
        should_save
    );

    let item_uri = library_track_uri(item_id);
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;

    match update_track_saved_state(spotify, &client, &access_token, item_id, &item_uri, should_save) {
        Ok(()) => {
            set_saved_track_cache(spotify, item_id, should_save);
            clear_scope_rate_limit(spotify, RateLimitScope::Library);
            log::info!(
                "Spotify: current track is now {} the library",
                if should_save {
                    "saved to"
                } else {
                    "removed from"
                }
            );
            Ok(should_save)
        }
        Err(err) => {
            log::warn!(
                "Spotify track library update failed for item_id='{}' should_save='{}': {}",
                item_id,
                should_save,
                err
            );
            Err(err)
        }
    }
}

fn library_update_error(
    track_id: &str,
    track_uri: &str,
    should_save: bool,
    status_code: u16,
    body: &str,
) -> String {
    format!(
        "Spotify track library update failed for track_id='{}' track_uri='{}' should_save='{}': {} {}",
        track_id, track_uri, should_save, status_code, body
    )
}

fn send_library_update_once(
    client: &Client,
    access_token: &str,
    track_uri: &str,
    should_save: bool,
) -> Result<reqwest::blocking::Response, String> {
    let response = send_library_track_update(client, access_token, track_uri, should_save)?;
    if response.status().is_success() {
        return Ok(response);
    }
    if response.status().as_u16() == 400 || response.status().as_u16() == 404 {
        return send_library_track_update_json(client, access_token, track_uri, should_save);
    }
    Ok(response)
}

fn update_track_saved_state(
    spotify: &SpotifyState,
    client: &Client,
    access_token: &str,
    track_id: &str,
    track_uri: &str,
    should_save: bool,
) -> Result<(), String> {
    wait_for_scope_rate_limit(spotify, RateLimitScope::Library, Some(SPOTIFY_MAX_RETRY_WAIT));

    let response = send_library_update_once(client, access_token, track_uri, should_save)?;
    if response.status().is_success() {
        return Ok(());
    }

    let status_code = response.status().as_u16();
    if status_code == 429 {
        apply_scope_rate_limit_from_response(spotify, RateLimitScope::Library, &response);
        let _body = response.text().unwrap_or_default();
        log::info!(
            "Spotify library update rate-limited; retrying once after Retry-After for track_id='{}'",
            track_id
        );
        wait_for_scope_rate_limit(spotify, RateLimitScope::Library, Some(SPOTIFY_MAX_RETRY_WAIT));

        let retry = send_library_update_once(client, access_token, track_uri, should_save)?;
        if retry.status().is_success() {
            return Ok(());
        }
        let retry_status = retry.status().as_u16();
        if retry_status == 429 {
            apply_scope_rate_limit_from_response(spotify, RateLimitScope::Library, &retry);
        }
        let retry_body = retry.text().unwrap_or_default();
        return Err(library_update_error(
            track_id,
            track_uri,
            should_save,
            retry_status,
            &retry_body,
        ));
    }

    let body = response.text().unwrap_or_default();
    Err(library_update_error(
        track_id,
        track_uri,
        should_save,
        status_code,
        &body,
    ))
}

fn send_library_track_update(
    client: &Client,
    access_token: &str,
    track_uri: &str,
    should_save: bool,
) -> Result<reqwest::blocking::Response, String> {
    let request = if should_save {
        client
            .put(SPOTIFY_LIBRARY_URL)
            .bearer_auth(access_token)
            .query(&[("uris", track_uri)])
            .body("")
    } else {
        client
            .delete(SPOTIFY_LIBRARY_URL)
            .bearer_auth(access_token)
            .query(&[("uris", track_uri)])
            .body("")
    };
    request
        .send()
        .map_err(|e| format!("Spotify /me/library request failed: {}", e))
}

fn send_library_track_update_json(
    client: &Client,
    access_token: &str,
    track_uri: &str,
    should_save: bool,
) -> Result<reqwest::blocking::Response, String> {
    let body = serde_json::to_string(&LibraryUrisBody {
        uris: vec![track_uri.to_string()],
    })
    .map_err(|e| e.to_string())?;
    let request = if should_save {
        client
            .put(SPOTIFY_LIBRARY_URL)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .body(body)
    } else {
        client
            .delete(SPOTIFY_LIBRARY_URL)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .body(body)
    };
    request
        .send()
        .map_err(|e| format!("Spotify /me/library JSON request failed: {}", e))
}

fn resolve_saved_track_state(spotify: &SpotifyState, track_id: &str) -> bool {
    match get_track_saved_state(spotify, Some(track_id), false) {
        Ok(Some(saved)) => saved,
        _ => stale_cached_saved_track_state(spotify, track_id).unwrap_or(false),
    }
}

fn stale_cached_saved_track_state(spotify: &SpotifyState, track_id: &str) -> Option<bool> {
    let cache = spotify.saved_track_cache.lock().unwrap();
    if cache.track_id.as_deref() == Some(track_id) {
        cache.saved
    } else {
        None
    }
}

fn mark_saved_lookup_attempted(spotify: &SpotifyState, track_id: &str) {
    let mut cache = spotify.saved_track_cache.lock().unwrap();
    cache.saved_lookup_track_id = Some(track_id.to_string());
}

fn resolve_display_saved_state(
    spotify: &SpotifyState,
    track_id: &str,
    allow_network_fetch: bool,
) -> Option<bool> {
    if let Some(saved) = stale_cached_saved_track_state(spotify, track_id) {
        return Some(saved);
    }

    {
        let cache = spotify.saved_track_cache.lock().unwrap();
        if cache.saved_lookup_track_id.as_deref() == Some(track_id) {
            return cache.saved;
        }
    }

    if !allow_network_fetch || is_library_rate_limited(spotify) {
        return stale_cached_saved_track_state(spotify, track_id);
    }

    match fetch_track_saved_state(spotify, track_id) {
        Ok(saved) => {
            mark_saved_lookup_attempted(spotify, track_id);
            if let Some(value) = saved {
                set_saved_track_cache(spotify, track_id, value);
            }
            saved
        }
        Err(err) => {
            mark_saved_lookup_attempted(spotify, track_id);
            log::warn!("Spotify saved-state check unavailable: {}", err);
            stale_cached_saved_track_state(spotify, track_id)
        }
    }
}

fn cached_saved_track_state(
    spotify: &SpotifyState,
    cache: &SavedTrackCache,
    track_id: &str,
) -> Option<bool> {
    if cache.track_id.as_deref() != Some(track_id) {
        return None;
    }
    let saved = cache.saved?;
    let fresh = cache
        .checked_at
        .map(|checked_at| {
            SystemTime::now()
                .duration_since(checked_at)
                .unwrap_or_default()
                < SAVED_TRACK_CACHE_TTL
        })
        .unwrap_or(false);
    if fresh || is_library_rate_limited(spotify) {
        Some(saved)
    } else {
        None
    }
}

fn set_saved_track_cache(spotify: &SpotifyState, track_id: &str, saved: bool) {
    let mut cache = spotify.saved_track_cache.lock().unwrap();
    cache.track_id = Some(track_id.to_string());
    cache.saved = Some(saved);
    cache.checked_at = Some(SystemTime::now());
    cache.saved_lookup_track_id = Some(track_id.to_string());
}

pub fn get_track_saved_state(
    spotify: &SpotifyState,
    track_id: Option<&str>,
    force_refresh: bool,
) -> Result<Option<bool>, String> {
    let Some(track_id) = track_id else {
        return Ok(None);
    };

    if !force_refresh {
        let cache = spotify.saved_track_cache.lock().unwrap();
        if let Some(saved) = cached_saved_track_state(spotify, &cache, track_id) {
            return Ok(Some(saved));
        }
        if is_library_rate_limited(spotify) {
            return Ok(cache
                .track_id
                .as_deref()
                .filter(|id| *id == track_id)
                .and_then(|_| cache.saved));
        }
    } else if is_library_rate_limited(spotify) {
        return Ok(stale_cached_saved_track_state(spotify, track_id));
    }

    match fetch_track_saved_state(spotify, track_id) {
        Ok(saved) => {
            if let Some(value) = saved {
                set_saved_track_cache(spotify, track_id, value);
            }
            Ok(saved)
        }
        Err(err) => {
            if let Some(saved) = stale_cached_saved_track_state(spotify, track_id) {
                return Ok(Some(saved));
            }
            Err(err)
        }
    }
}

fn fetch_track_saved_state(spotify: &SpotifyState, track_id: &str) -> Result<Option<bool>, String> {
    ensure_library_read_available(spotify)?;

    let access_token = get_access_token(spotify)?;
    let track_uri = library_track_uri(track_id);
    let client = spotify_http_client()?;
    let response = client
        .get(SPOTIFY_LIBRARY_CONTAINS_URL)
        .bearer_auth(&access_token)
        .query(&[("uris", track_uri.as_str())])
        .send()
        .map_err(|e| format!("Spotify saved-track check request failed: {}", e))?;

    if response.status().is_success() {
        let values: Vec<bool> = response.json().map_err(|e| e.to_string())?;
        return Ok(values.into_iter().next());
    }

    let status = response.status();
    apply_scope_rate_limit_from_response(spotify, RateLimitScope::Library, &response);
    let body = response.text().unwrap_or_default();
    Err(format!(
        "Spotify saved-track check failed for track_id='{}' track_uri='{}': {} {}",
        track_id, track_uri, status, body
    ))
}

pub fn adjust_volume(spotify: &SpotifyState, delta: i32) -> Result<u8, String> {
    if !uses_web_api(spotify) {
        return crate::spotify_desktop::adjust_volume(spotify, delta);
    }
    let playback = get_playback_for_mutation(spotify)?;
    let device_id = playback.device_id;
    let current = playback.volume_percent as i32;
    let next_volume = (current + delta).clamp(0, 100) as u8;
    set_volume(spotify, next_volume, Some(device_id))
}

pub fn set_volume(spotify: &SpotifyState, volume_percent: u8, device_id: Option<String>) -> Result<u8, String> {
    if !uses_web_api(spotify) {
        return crate::spotify_desktop::set_volume(spotify, volume_percent);
    }
    let device_id = match device_id {
        Some(id) => id,
        None => get_playback_for_mutation(spotify)?.device_id,
    };
    let access_token = get_access_token(spotify)?;

    let client = spotify_http_client()?;
    let response = client
        .put(SPOTIFY_SET_VOLUME_URL)
        .bearer_auth(access_token)
        .query(&[
            ("volume_percent", volume_percent.to_string()),
            ("device_id", device_id),
        ])
        .body("")
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        update_playback_cache_fields(spotify, |summary| {
            summary.volume_percent = volume_percent;
        });
        Ok(volume_percent)
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Spotify volume update failed: {} {}", status, body))
    }
}

pub fn seek(spotify: &SpotifyState, position_ms: u64) -> Result<(), String> {
    if !uses_web_api(spotify) {
        return crate::spotify_desktop::seek(spotify, position_ms);
    }
    let playback = get_playback_for_mutation(spotify)?;
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;
    let response = client
        .put(SPOTIFY_SEEK_URL)
        .bearer_auth(access_token)
        .query(&[
            ("position_ms", position_ms.to_string()),
            ("device_id", playback.device_id),
        ])
        .body("")
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        update_playback_cache_fields(spotify, |summary| {
            summary.progress_ms = Some(position_ms);
        });
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Spotify seek failed: {} {}", status, body))
    }
}

pub fn get_current_playback(spotify: &SpotifyState) -> Result<PlaybackSummary, String> {
    get_current_playback_with_refresh(spotify, false)
}

pub fn refresh_current_playback(spotify: &SpotifyState) -> Result<PlaybackSummary, String> {
    get_current_playback_with_refresh(spotify, true)
}

fn get_current_playback_with_refresh(
    spotify: &SpotifyState,
    force_refresh: bool,
) -> Result<PlaybackSummary, String> {
    if !force_refresh {
        if let Some(cached) = read_playback_cache(spotify, PLAYBACK_CACHE_TTL) {
            return Ok(cached);
        }
    }

    if !is_playback_read_rate_limited(spotify) {
        match fetch_current_playback(spotify) {
            Ok(summary) => return Ok(summary),
            Err(err) => {
                if let Some(cached) = read_playback_cache_stale(spotify) {
                    return Ok(cached);
                }
                return Err(err);
            }
        }
    }

    read_playback_cache_stale(spotify).ok_or_else(|| {
        let seconds = scope_rate_limit_wait_remaining(spotify, RateLimitScope::PlaybackRead)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        format!(
            "{} Retry again in ~{} seconds.",
            scope_rate_limited_error(RateLimitScope::PlaybackRead),
            seconds
        )
    })
}

fn fetch_current_playback(spotify: &SpotifyState) -> Result<PlaybackSummary, String> {
    ensure_playback_read_available(spotify)?;
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;
    let response = client
        .get(SPOTIFY_CURRENT_PLAYBACK_URL)
        .bearer_auth(&access_token)
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().as_u16() == 204 {
        return Err("Spotify has no active playback device right now.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        apply_scope_rate_limit_from_response(spotify, RateLimitScope::PlaybackRead, &response);
        let body = response.text().unwrap_or_default();
        return Err(format!("Spotify playback query failed: {} {}", status, body));
    }

    clear_scope_rate_limit(spotify, RateLimitScope::PlaybackRead);

    let playback: PlaybackResponse = response.json().map_err(|e| e.to_string())?;
    let device_id = playback
        .device
        .id
        .ok_or_else(|| "Spotify playback did not expose an active device id".to_string())?;

    let summary = PlaybackSummary {
        device_id,
        device_name: playback.device.name,
        is_playing: playback.is_playing,
        volume_percent: playback.device.volume_percent.unwrap_or(0),
        item_id: playback.item.as_ref().and_then(|item| item.id.clone()),
        item_type: playback.item.as_ref().map(|item| item.item_type.clone()),
        item_name: playback.item.as_ref().map(|item| item.name.clone()),
        artist_name: playback
            .item
            .as_ref()
            .and_then(|item| item.artists.first().map(|artist| artist.name.clone())),
        album_name: playback
            .item
            .as_ref()
            .and_then(|item| item.album.as_ref().and_then(|album| album.name.clone())),
        cover_art_url: playback.item.as_ref().and_then(playback_item_image_url),
        progress_ms: playback.progress_ms,
        duration_ms: playback
            .item
            .as_ref()
            .and_then(|item| item.duration_ms),
        shuffle_state: playback.shuffle_state,
    };
    Ok(store_playback_from_api(spotify, &summary))
}

pub fn toggle_shuffle(spotify: &SpotifyState) -> Result<bool, String> {
    if !uses_web_api(spotify) {
        return crate::spotify_desktop::toggle_shuffle(spotify);
    }
    let playback = get_playback_for_mutation(spotify)?;
    let next_state = !playback.shuffle_state;
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;
    let response = client
        .put(SPOTIFY_SHUFFLE_URL)
        .bearer_auth(access_token)
        .query(&[
            ("state", next_state.to_string()),
            ("device_id", playback.device_id),
        ])
        .body("")
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        update_playback_cache_fields(spotify, |summary| {
            summary.shuffle_state = next_state;
        });
        invalidate_queue_cache(spotify);
        maybe_refresh_queue_cache(spotify, true);
        Ok(next_state)
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Spotify shuffle update failed: {} {}", status, body))
    }
}

fn playlist_context_uri(id_or_uri: &str) -> Result<String, String> {
    let trimmed = id_or_uri.trim();
    if trimmed.is_empty() {
        return Err("Playlist id is required".to_string());
    }
    if trimmed.starts_with("spotify:playlist:") {
        return Ok(trimmed.to_string());
    }
    if trimmed.starts_with("spotify:") {
        return Err(format!("Unsupported Spotify URI for playlist play: {trimmed}"));
    }
    Ok(format!("spotify:playlist:{trimmed}"))
}

fn playlist_wait_error(seconds: u64, cached: bool) -> String {
    if cached {
        format!(
            "Spotify is rate-limiting playlist reads. Showing saved playlists. Try refresh in about {seconds} seconds."
        )
    } else {
        format!(
            "Spotify is rate-limiting playlist reads. Try again in about {seconds} seconds."
        )
    }
}

fn read_playlist_cache(
    spotify: &SpotifyState,
    offset: u32,
    limit: u32,
    max_age: Duration,
) -> Option<SpotifyPlaylistPage> {
    let cache = spotify.playlist_cache.lock().unwrap();
    let page = cache.page.as_ref()?;
    if cache.offset != offset || cache.limit != limit {
        return None;
    }
    let fetched_at = cache.fetched_at?;
    let age = SystemTime::now()
        .duration_since(fetched_at)
        .unwrap_or(Duration::MAX);
    (age <= max_age).then(|| page.clone())
}

fn read_playlist_cache_stale(
    spotify: &SpotifyState,
    offset: u32,
    limit: u32,
) -> Option<SpotifyPlaylistPage> {
    let cache = spotify.playlist_cache.lock().unwrap();
    if cache.offset != offset || cache.limit != limit {
        return None;
    }
    cache.page.clone()
}

fn store_playlist_cache(spotify: &SpotifyState, offset: u32, limit: u32, page: &SpotifyPlaylistPage) {
    let mut cache = spotify.playlist_cache.lock().unwrap();
    cache.offset = offset;
    cache.limit = limit;
    cache.page = Some(page.clone());
    cache.fetched_at = Some(SystemTime::now());
}

fn playlists_rate_limited_error(spotify: &SpotifyState) -> String {
    let seconds = scope_rate_limit_wait_remaining(spotify, RateLimitScope::Playlists)
        .map(|duration| duration.as_secs().max(1))
        .unwrap_or(1);
    playlist_wait_error(seconds, false)
}

fn fetch_playlists_page(
    spotify: &SpotifyState,
    offset: u32,
    limit: u32,
) -> Result<SpotifyPlaylistPage, String> {
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;
    let response = client
        .get(SPOTIFY_PLAYLISTS_URL)
        .bearer_auth(&access_token)
        .query(&[
            ("limit", limit.to_string()),
            ("offset", offset.to_string()),
            ("fields", SPOTIFY_PLAYLIST_FIELDS.to_string()),
        ])
        .send()
        .map_err(|e| format!("Spotify playlists request failed: {e}"))?;

    if response.status().as_u16() == 429 {
        apply_scope_rate_limit_from_response(spotify, RateLimitScope::Playlists, &response);
        return Err(playlists_rate_limited_error(spotify));
    }

    if !response.status().is_success() {
        let status = response.status();
        let backoff = retry_after_from_response(&response);
        let body = response.text().unwrap_or_default();
        if body.contains("API rate limit exceeded") {
            set_scope_rate_limit(spotify, RateLimitScope::Playlists, backoff);
            return Err(playlists_rate_limited_error(spotify));
        }
        return Err(format!("Spotify playlists query failed: {status} {body}"));
    }

    clear_scope_rate_limit(spotify, RateLimitScope::Playlists);
    let page: PlaylistsResponse = response.json().map_err(|e| e.to_string())?;
    let items = page
        .items
        .into_iter()
        .filter_map(|item| {
            let id = item.id?;
            let name = item.name.filter(|value| !value.is_empty())?;
            Some(SpotifyPlaylist {
                uri: item
                    .uri
                    .unwrap_or_else(|| format!("spotify:playlist:{id}")),
                image_url: item.images.first().map(|image| image.url.clone()),
                track_count: item.tracks.map(|tracks| tracks.total).unwrap_or(0),
                owner_name: item.owner.and_then(|owner| owner.display_name),
                id,
                name,
            })
        })
        .collect::<Vec<_>>();
    let fetched = items.len() as u32;
    let next_offset = (offset + fetched < page.total).then_some(offset + fetched);

    Ok(SpotifyPlaylistPage {
        items,
        offset: page.offset,
        limit: page.limit.max(limit),
        total: page.total,
        next_offset,
    })
}

pub fn list_playlists(
    spotify: &SpotifyState,
    offset: u32,
    limit: u32,
) -> Result<SpotifyPlaylistPage, String> {
    if !uses_web_api(spotify) {
        let limit = limit.clamp(1, 50);
        let _fetch = spotify
            .playlist_fetch
            .lock()
            .map_err(|e| e.to_string())?;
        if let Some(cached) = read_playlist_cache(spotify, offset, limit, PLAYLIST_CACHE_TTL) {
            return Ok(cached);
        }
        let page = crate::spotify_desktop::list_playlists(spotify, offset, limit)?;
        store_playlist_cache(spotify, offset, limit, &page);
        return Ok(page);
    }

    if !has_scope(spotify, "playlist-read-private")?
        && !has_scope(spotify, "playlist-read-collaborative")?
    {
        return Err(
            "Spotify token is missing playlist access. Disconnect and reconnect Spotify."
                .to_string(),
        );
    }

    let limit = limit.clamp(1, 50);
    let _fetch = spotify
        .playlist_fetch
        .lock()
        .map_err(|e| e.to_string())?;

    if let Some(cached) = read_playlist_cache(spotify, offset, limit, PLAYLIST_CACHE_TTL) {
        return Ok(cached);
    }

    let blocking_scope = [
        RateLimitScope::Playlists,
        RateLimitScope::PlaybackRead,
        RateLimitScope::Library,
    ]
    .into_iter()
    .find(|scope| is_scope_rate_limited(spotify, *scope));
    if let Some(scope) = blocking_scope {
        if let Some(cached) = read_playlist_cache_stale(spotify, offset, limit) {
            log::info!("Spotify {:?} API is rate-limited; returning cached playlists", scope);
            return Ok(cached);
        }
        let remaining = scope_rate_limit_wait_remaining(spotify, scope)
            .unwrap_or(Duration::from_secs(1));
        if remaining > SPOTIFY_PLAYLIST_RETRY_WAIT {
            return Err(playlist_wait_error(remaining.as_secs().max(1), false));
        }
        log::info!(
            "Spotify {:?} API is rate-limited; waiting {}s before playlist fetch",
            scope,
            remaining.as_secs()
        );
        std::thread::sleep(remaining);
    }

    match fetch_playlists_page(spotify, offset, limit) {
        Ok(page) => {
            store_playlist_cache(spotify, offset, limit, &page);
            Ok(page)
        }
        Err(err) => {
            if is_scope_rate_limited(spotify, RateLimitScope::Playlists) {
                let remaining = scope_rate_limit_wait_remaining(spotify, RateLimitScope::Playlists)
                    .unwrap_or(Duration::from_secs(2));
                if remaining <= SPOTIFY_PLAYLIST_RETRY_WAIT {
                    log::info!(
                        "Spotify playlists 429; retrying once after {}s",
                        remaining.as_secs()
                    );
                    std::thread::sleep(remaining);
                    match fetch_playlists_page(spotify, offset, limit) {
                        Ok(page) => {
                            store_playlist_cache(spotify, offset, limit, &page);
                            return Ok(page);
                        }
                        Err(retry_err) => {
                            if let Some(cached) = read_playlist_cache_stale(spotify, offset, limit) {
                                log::warn!(
                                    "Spotify playlists retry failed ({retry_err}); returning cache"
                                );
                                return Ok(cached);
                            }
                            return Err(retry_err);
                        }
                    }
                }
                if let Some(cached) = read_playlist_cache_stale(spotify, offset, limit) {
                    log::info!("Spotify playlists still rate-limited; returning cached page");
                    return Ok(cached);
                }
                return Err(playlist_wait_error(remaining.as_secs().max(1), false));
            }
            Err(err)
        }
    }
}

pub fn play_playlist(spotify: &SpotifyState, playlist: &str) -> Result<(), String> {
    if !uses_web_api(spotify) {
        let context_uri = playlist_context_uri(playlist)?;
        crate::spotify_desktop::play_context_uri(spotify, &context_uri)?;
        return Ok(());
    }

    let context_uri = playlist_context_uri(playlist)?;
    let device_id = get_playback_for_mutation(spotify).ok().map(|p| p.device_id);
    let access_token = get_access_token(spotify)?;
    let client = spotify_http_client()?;
    let body = serde_json::to_string(&PlayContextBody {
        context_uri: context_uri.clone(),
    })
    .map_err(|e| e.to_string())?;

    for attempt in 0..2 {
        let mut request = client
            .put(SPOTIFY_PLAY_URL)
            .bearer_auth(&access_token)
            .header("Content-Type", "application/json")
            .body(body.clone());
        if let Some(device_id) = device_id.as_ref() {
            request = request.query(&[("device_id", device_id)]);
        }

        let response = request
            .send()
            .map_err(|e| format!("Spotify play playlist request failed: {e}"))?;

        if response.status().as_u16() == 429 {
            apply_scope_rate_limit_from_response(spotify, RateLimitScope::Playlists, &response);
            if attempt == 0 {
                let remaining = scope_rate_limit_wait_remaining(spotify, RateLimitScope::Playlists)
                    .unwrap_or(Duration::from_secs(2));
                if remaining <= SPOTIFY_PLAYLIST_RETRY_WAIT {
                    std::thread::sleep(remaining);
                    continue;
                }
            }
            return Err(playlists_rate_limited_error(spotify));
        }

        if response.status().is_success() || response.status().as_u16() == 204 {
            invalidate_playback_cache(spotify);
            invalidate_queue_cache(spotify);
            return Ok(());
        }

        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Spotify play playlist failed for {context_uri}: {status} {body}"
        ));
    }

    Err(playlists_rate_limited_error(spotify))
}

fn playback_item_image_url(item: &PlaybackItem) -> Option<String> {
    item.album
        .as_ref()
        .and_then(|album| album.images.first())
        .map(|image| image.url.clone())
        .or_else(|| item.images.first().map(|image| image.url.clone()))
}

pub(crate) fn get_access_token(spotify: &SpotifyState) -> Result<String, String> {
    maybe_refresh_token(spotify)?;
    spotify
        .tokens
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|tokens| tokens.access_token.clone())
        .ok_or_else(|| "Spotify is not authenticated yet.".to_string())
}

fn ensure_scope(spotify: &SpotifyState, required_scope: &str) -> Result<(), String> {
    if has_scope(spotify, required_scope)? {
        Ok(())
    } else {
        Err(format!(
            "Spotify token is missing required scope '{}'. Disconnect and reconnect Spotify.",
            required_scope
        ))
    }
}

fn has_scope(spotify: &SpotifyState, required_scope: &str) -> Result<bool, String> {
    let scopes = current_scopes(spotify)?;
    Ok(scopes.iter().any(|scope| scope == required_scope))
}

fn library_track_uri(track_id: &str) -> String {
    format!("spotify:track:{}", track_id)
}

fn maybe_refresh_token(spotify: &SpotifyState) -> Result<(), String> {
    let needs_refresh = {
        let guard = spotify.tokens.lock().map_err(|e| e.to_string())?;
        match guard.as_ref() {
            Some(tokens) => tokens.expires_at <= now_unix_seconds() + 60,
            None => false,
        }
    };

    if !needs_refresh {
        return Ok(());
    }

    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    let current_tokens = spotify
        .tokens
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Spotify is not authenticated yet.".to_string())?;

    let client = Client::new();
    let response = client
        .post(SPOTIFY_TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", current_tokens.refresh_token.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Spotify token refresh failed: {} {}", status, body));
    }

    let refreshed: TokenResponse = response.json().map_err(|e| e.to_string())?;
    let tokens = tokens_from_response(refreshed, Some(&current_tokens))?;
    persist_tokens(spotify, tokens)
}

fn persist_tokens(spotify: &SpotifyState, tokens: SpotifyTokens) -> Result<(), String> {
    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(path) = config.token_path {
        let serialized = serde_json::to_string_pretty(&tokens).map_err(|e| e.to_string())?;
        fs::write(path, serialized).map_err(|e| e.to_string())?;
    }

    let mut stored = spotify.tokens.lock().map_err(|e| e.to_string())?;
    *stored = Some(tokens);
    Ok(())
}

fn clear_pending_auth(spotify: &SpotifyState) -> Result<(), String> {
    let mut pending = spotify.auth_session.lock().map_err(|e| e.to_string())?;
    *pending = None;
    Ok(())
}

fn tokens_from_response(
    response: TokenResponse,
    existing: Option<&SpotifyTokens>,
) -> Result<SpotifyTokens, String> {
    if response.token_type.to_lowercase() != "bearer" {
        return Err(format!("Unexpected Spotify token type: {}", response.token_type));
    }

    let scope = if response.scope.trim().is_empty() {
        existing.map(|tokens| tokens.scope.clone()).unwrap_or_default()
    } else {
        response.scope
    };

    Ok(SpotifyTokens {
        access_token: response.access_token,
        refresh_token: response
            .refresh_token
            .or_else(|| existing.map(|tokens| tokens.refresh_token.clone()))
            .ok_or_else(|| "Spotify did not return a refresh token".to_string())?,
        expires_at: now_unix_seconds() + response.expires_in,
        scope,
    })
}

fn build_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn write_html_response(stream: &mut std::net::TcpStream, success: bool, message: &str) -> Result<(), String> {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>AstroDeck Spotify</title></head><body style=\"font-family: sans-serif; background:#0b1020; color:#f8fafc; padding:32px;\"><h1>{}</h1><p>{}</p></body></html>",
        if success { "Spotify Connected" } else { "Spotify Authorization Error" },
        message
    );

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| e.to_string())
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_scopes(scope_string: &str) -> Vec<String> {
    scope_string
        .split_whitespace()
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect()
}

/// `None` if there is no token or Spotify omitted scopes; otherwise whether `streaming` was granted.
pub(crate) fn token_has_streaming_scope(spotify: &SpotifyState) -> Result<Option<bool>, String> {
    let guard = spotify.tokens.lock().map_err(|e| e.to_string())?;
    let Some(tokens) = guard.as_ref() else {
        return Ok(None);
    };
    if tokens.scope.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(
        parse_scopes(&tokens.scope)
            .iter()
            .any(|scope| scope == "streaming"),
    ))
}

fn current_scopes(spotify: &SpotifyState) -> Result<Vec<String>, String> {
    let tokens = spotify
        .tokens
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Spotify is not authenticated yet.".to_string())?;
    Ok(parse_scopes(&tokens.scope))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_status_describes_in_app_playback() {
        let spotify = SpotifyState::default();
        let status = official_desktop_status(&spotify).unwrap();
        assert!(!status.uses_web_api);
        assert!(status.message.contains("AstroDeck streams playback"));
        assert!(!status.message.contains("OS media"));
    }

    #[test]
    fn new_install_defaults_to_official_desktop_login() {
        assert_eq!(resolve_auth_mode(None, ""), SpotifyAuthMode::Official);
        assert_eq!(
            effective_client_id(SpotifyAuthMode::Official, "", ""),
            OFFICIAL_CLIENT_ID
        );
        assert_eq!(
            redirect_uri_for_mode(SpotifyAuthMode::Official),
            OFFICIAL_REDIRECT_URI
        );
    }

    #[test]
    fn legacy_saved_client_id_stays_on_custom_api() {
        let stored = SpotifyClientFile {
            auth_mode: None,
            client_id: "user-app-id".to_string(),
        };
        assert_eq!(
            resolve_auth_mode(Some(&stored), ""),
            SpotifyAuthMode::Custom
        );
        assert_eq!(
            effective_client_id(SpotifyAuthMode::Custom, "user-app-id", ""),
            "user-app-id"
        );
        assert_eq!(
            redirect_uri_for_mode_with(SpotifyAuthMode::Custom, None),
            CUSTOM_REDIRECT_URI
        );
        assert_eq!(
            redirect_uri_for_mode_with(SpotifyAuthMode::Custom, Some("http://127.0.0.1:9/cb")),
            "http://127.0.0.1:9/cb"
        );
        assert_eq!(
            redirect_uri_for_mode_with(SpotifyAuthMode::Official, Some("http://127.0.0.1:9/cb")),
            OFFICIAL_REDIRECT_URI
        );
    }

    #[test]
    fn env_client_id_forces_custom_api() {
        let stored = SpotifyClientFile {
            auth_mode: Some(SpotifyAuthMode::Official),
            client_id: String::new(),
        };
        assert_eq!(
            resolve_auth_mode(Some(&stored), "env-app-id"),
            SpotifyAuthMode::Custom
        );
        assert_eq!(
            effective_client_id(SpotifyAuthMode::Official, "", "env-app-id"),
            "env-app-id"
        );
    }

    #[test]
    fn stored_official_mode_keeps_custom_id_for_later() {
        let stored = SpotifyClientFile {
            auth_mode: Some(SpotifyAuthMode::Official),
            client_id: "user-app-id".to_string(),
        };
        assert_eq!(
            resolve_auth_mode(Some(&stored), ""),
            SpotifyAuthMode::Official
        );
        assert_eq!(stored_custom_client_id(Some(&stored), ""), "user-app-id");
        assert_eq!(
            effective_client_id(SpotifyAuthMode::Official, "user-app-id", ""),
            OFFICIAL_CLIENT_ID
        );
    }

    #[test]
    fn official_mode_does_not_use_web_api() {
        let spotify = SpotifyState::default();
        assert!(!uses_web_api(&spotify));
        let mut config = spotify.config.lock().unwrap();
        config.auth_mode = SpotifyAuthMode::Custom;
        drop(config);
        assert!(uses_web_api(&spotify));
    }

    #[test]
    fn client_file_round_trips_auth_mode() {
        let file = SpotifyClientFile {
            auth_mode: Some(SpotifyAuthMode::Official),
            client_id: "abc".to_string(),
        };
        let json = serde_json::to_string(&file).unwrap();
        let parsed: SpotifyClientFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.auth_mode, Some(SpotifyAuthMode::Official));
        assert_eq!(parsed.client_id, "abc");

        let legacy: SpotifyClientFile =
            serde_json::from_str(r#"{"client_id":"legacy-id"}"#).unwrap();
        assert_eq!(legacy.auth_mode, None);
        assert_eq!(legacy.client_id, "legacy-id");
    }

    #[test]
    fn playlist_context_uri_accepts_id_or_uri() {
        assert_eq!(
            playlist_context_uri("37i9dQZF1DXcBWIGoYBM5M").unwrap(),
            "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"
        );
        assert_eq!(
            playlist_context_uri("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M").unwrap(),
            "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"
        );
        assert!(playlist_context_uri("spotify:album:abc").is_err());
        assert!(playlist_context_uri("").is_err());
    }

    #[test]
    fn playlist_wait_error_hides_raw_429_json() {
        let message = playlist_wait_error(12, false);
        assert!(message.contains("12 seconds"));
        assert!(!message.contains("429"));
        assert!(!message.contains("API rate limit exceeded"));
        assert!(!message.contains("Spotify playlists query failed"));

        let cached = playlist_wait_error(8, true);
        assert!(cached.contains("Showing saved playlists"));
        assert!(!cached.contains("429"));
    }

    #[test]
    fn official_oauth_requests_streaming_scope() {
        assert_eq!(oauth_scopes(SpotifyAuthMode::Official), "streaming");
        assert!(oauth_scopes(SpotifyAuthMode::Custom).contains("playlist-read-private"));
        assert!(!oauth_scopes(SpotifyAuthMode::Custom)
            .split_whitespace()
            .any(|scope| scope == "streaming"));
    }

    #[test]
    fn refresh_keeps_existing_scope_when_token_omits_it() {
        let existing = SpotifyTokens {
            access_token: "old".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: 1,
            scope: "streaming".to_string(),
        };
        let refreshed = tokens_from_response(
            TokenResponse {
                access_token: "new".to_string(),
                token_type: "Bearer".to_string(),
                expires_in: 3600,
                scope: String::new(),
                refresh_token: None,
            },
            Some(&existing),
        )
        .unwrap();
        assert_eq!(refreshed.access_token, "new");
        assert_eq!(refreshed.refresh_token, "refresh");
        assert_eq!(refreshed.scope, "streaming");
    }
}
