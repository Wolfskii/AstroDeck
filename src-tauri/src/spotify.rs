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
use std::sync::Mutex;
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
#[derive(Serialize)]
struct LibraryUrisBody {
    uris: Vec<String>,
}
const SPOTIFY_SCOPES: &str =
    "user-library-modify user-library-read user-read-playback-state user-modify-playback-state";

#[derive(Default)]
struct SavedTrackCache {
    track_id: Option<String>,
    saved: Option<bool>,
    checked_at: Option<SystemTime>,
    /// Track id we already attempted a library-contains lookup for (avoids 429 spam).
    saved_lookup_track_id: Option<String>,
}

#[derive(Default)]
struct ApiRateLimitState {
    limited_until: Option<SystemTime>,
}

#[derive(Default)]
struct PlaybackCache {
    summary: Option<PlaybackSummary>,
    fetched_at: Option<SystemTime>,
}

const SPOTIFY_HTTP_TIMEOUT: Duration = Duration::from_secs(12);
const SAVED_TRACK_CACHE_TTL: Duration = Duration::from_secs(120);
/// Reuse /me/player responses for routine status polls (UI extrapolates progress locally).
const PLAYBACK_CACHE_TTL: Duration = Duration::from_secs(30);
/// Default backoff when Spotify omits Retry-After on a 429.
const SPOTIFY_DEFAULT_RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(60);
/// Max wait before a single automatic retry (user-initiated actions).
const SPOTIFY_MAX_RETRY_WAIT: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct SpotifyState {
    pub config: Mutex<SpotifyConfig>,
    pub tokens: Mutex<Option<SpotifyTokens>>,
    pub auth_session: Mutex<Option<PendingAuth>>,
    saved_track_cache: Mutex<SavedTrackCache>,
    api_rate_limit: Mutex<ApiRateLimitState>,
    playback_cache: Mutex<PlaybackCache>,
}

pub fn invalidate_playback_cache(spotify: &SpotifyState) {
    let mut cache = spotify.playback_cache.lock().unwrap();
    cache.summary = None;
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

fn store_playback_cache(spotify: &SpotifyState, summary: &PlaybackSummary) {
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

fn set_api_rate_limit(spotify: &SpotifyState, backoff: Duration) {
    let until = SystemTime::now() + backoff;
    let mut state = spotify.api_rate_limit.lock().unwrap();
    state.limited_until = Some(match state.limited_until {
        Some(existing) if existing > until => existing,
        _ => until,
    });
}

fn clear_api_rate_limit(spotify: &SpotifyState) {
    let mut state = spotify.api_rate_limit.lock().unwrap();
    state.limited_until = None;
}

fn api_rate_limit_wait_remaining(spotify: &SpotifyState) -> Option<Duration> {
    let state = spotify.api_rate_limit.lock().unwrap();
    state
        .limited_until
        .and_then(|until| until.duration_since(SystemTime::now()).ok())
        .filter(|duration| !duration.is_zero())
}

fn is_api_rate_limited(spotify: &SpotifyState) -> bool {
    api_rate_limit_wait_remaining(spotify).is_some()
}

fn api_rate_limited_error() -> String {
    "Spotify API is rate-limited. Wait for Retry-After, then try again.".to_string()
}

fn apply_api_rate_limit_from_response(spotify: &SpotifyState, response: &reqwest::blocking::Response) {
    if response.status().as_u16() != 429 {
        return;
    }
    let backoff = retry_after_from_response(response);
    set_api_rate_limit(spotify, backoff);
    log::warn!(
        "Spotify API returned 429; backing off for {} seconds (Retry-After)",
        backoff.as_secs()
    );
}

/// Wait until Retry-After expires. Caps sleep for user-initiated retries.
fn wait_for_api_rate_limit(spotify: &SpotifyState, cap: Option<Duration>) {
    let Some(mut remaining) = api_rate_limit_wait_remaining(spotify) else {
        return;
    };
    if let Some(max) = cap {
        if remaining > max {
            log::info!(
                "Spotify Retry-After is {}s; waiting {}s before retry",
                remaining.as_secs(),
                max.as_secs()
            );
            remaining = max;
        }
    } else {
        log::info!(
            "Spotify API rate-limited; waiting {}s per Retry-After",
            remaining.as_secs()
        );
    }
    std::thread::sleep(remaining);
}

fn ensure_api_available_for_background(spotify: &SpotifyState) -> Result<(), String> {
    if is_api_rate_limited(spotify) {
        let seconds = api_rate_limit_wait_remaining(spotify)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        return Err(format!(
            "{} Retry again in ~{} seconds.",
            api_rate_limited_error(),
            seconds
        ));
    }
    Ok(())
}

#[derive(Default, Clone)]
pub struct SpotifyConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub token_path: Option<PathBuf>,
    /// `app_local_data_dir/spotify_client.json` — used when `SPOTIFY_CLIENT_ID` is unset.
    pub client_store_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpotifyClientFile {
    client_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyClientConfigResponse {
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
    scope: String,
    refresh_token: Option<String>,
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

fn read_stored_client_id(path: &PathBuf) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let parsed: SpotifyClientFile = serde_json::from_str(&raw).ok()?;
    let id = parsed.client_id.trim().to_string();
    (!id.is_empty()).then_some(id)
}

fn env_client_id() -> String {
    std::env::var("SPOTIFY_CLIENT_ID")
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub fn init(app: &tauri::AppHandle, spotify: &SpotifyState) -> Result<(), String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;

    let client_store_path = base.join("spotify_client.json");
    let token_path = base.join("spotify_tokens.json");

    let from_env = env_client_id();
    let from_file = read_stored_client_id(&client_store_path).unwrap_or_default();
    let client_id = if !from_env.is_empty() {
        from_env
    } else {
        from_file
    };

    let redirect_uri = std::env::var("SPOTIFY_REDIRECT_URI")
        .unwrap_or_else(|_| "http://127.0.0.1:43821/callback".to_string());

    {
        let mut config = spotify.config.lock().map_err(|e| e.to_string())?;
        config.client_id = client_id;
        config.redirect_uri = redirect_uri;
        config.token_path = Some(token_path.clone());
        config.client_store_path = Some(client_store_path);
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
    let client_id = spotify
        .config
        .lock()
        .map(|c| c.client_id.clone())
        .unwrap_or_default();
    SpotifyClientConfigResponse {
        client_id,
        locked_by_env,
    }
}

pub fn set_client_id_from_settings(
    spotify: &SpotifyState,
    client_id: &str,
) -> Result<(), String> {
    if !env_client_id().is_empty() {
        return Err(
            "SPOTIFY_CLIENT_ID is set in the environment; unset it to save a Client ID from Settings."
                .to_string(),
        );
    }

    let store_path = {
        let guard = spotify.config.lock().map_err(|e| e.to_string())?;
        guard
            .client_store_path
            .clone()
            .ok_or_else(|| "Spotify paths not initialized.".to_string())?
    };

    let trimmed = client_id.trim();
    if trimmed.is_empty() {
        let _ = fs::remove_file(&store_path);
        let mut cfg = spotify.config.lock().map_err(|e| e.to_string())?;
        cfg.client_id.clear();
        return Ok(());
    }

    let file = SpotifyClientFile {
        client_id: trimmed.to_string(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    fs::write(&store_path, json).map_err(|e| e.to_string())?;

    let mut cfg = spotify.config.lock().map_err(|e| e.to_string())?;
    cfg.client_id = trimmed.to_string();
    Ok(())
}

pub fn get_status(spotify: &SpotifyState) -> Result<SpotifyStatus, String> {
    build_status(spotify, false)
}

pub fn get_status_fresh(spotify: &SpotifyState) -> Result<SpotifyStatus, String> {
    build_status(spotify, true)
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
            message: "Save your Spotify Client ID in Settings (desktop app), or set SPOTIFY_CLIENT_ID. Use Open Spotify Developer Dashboard to create an app and copy the Client ID."
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
            message: "Spotify is not connected yet.".to_string(),
        });
    }

    let playback_result = if fresh_playback {
        refresh_current_playback(spotify)
    } else {
        get_current_playback(spotify)
    };

    match playback_result {
        Ok(playback) => {
            let mut message = "Spotify is connected.".to_string();
            let is_current_track_saved = if playback.item_type.as_deref() == Some("track") {
                if has_scope(spotify, "user-library-read")? {
                    playback
                        .item_id
                        .as_deref()
                        .map(|track_id| {
                            resolve_display_saved_state(spotify, track_id, fresh_playback)
                        })
                        .unwrap_or(None)
                } else {
                    message = "Spotify is connected. Reconnect Spotify to enable saved-track status."
                        .to_string();
                    None
                }
            } else {
                None
            };

            Ok(SpotifyStatus {
                is_configured: true,
                is_authenticated: true,
                has_active_device: true,
                active_device_name: Some(playback.device_name),
                current_track_name: playback.item_name,
                current_artist_name: playback.artist_name,
                current_cover_art_url: playback.cover_art_url,
                current_album_name: playback.album_name,
                progress_ms: playback.progress_ms,
                duration_ms: playback.duration_ms,
                playback_state: if playback.is_playing {
                    "playing".to_string()
                } else {
                    "paused".to_string()
                },
                is_playing: playback.is_playing,
                current_volume_percent: Some(playback.volume_percent),
                current_item_type: playback.item_type,
                current_item_id: playback.item_id,
                is_current_track_saved,
                is_shuffle: playback.shuffle_state,
                granted_scopes,
                message,
            })
        }
        Err(err) => Ok(SpotifyStatus {
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
            message: err,
        }),
    }
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

    Ok(())
}

pub fn start_auth_flow(spotify: &SpotifyState) -> Result<String, String> {
    let config = spotify.config.lock().map_err(|e| e.to_string())?.clone();
    if config.client_id.is_empty() {
        return Err("Missing SPOTIFY_CLIENT_ID in environment configuration.".to_string());
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
        .append_pair("scope", SPOTIFY_SCOPES)
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
    Ok(())
}

pub fn toggle_current_track_saved(spotify: &SpotifyState) -> Result<bool, String> {
    ensure_scope(spotify, "user-library-modify")?;
    let playback = get_current_playback(spotify)?;
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
    ensure_scope(spotify, "user-library-modify")?;
    let (item_id, item_type) = match cached_playback_item_id(spotify) {
        Some(item_id) => (item_id, "track".to_string()),
        None => {
            let playback = get_current_playback(spotify)?;
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
            clear_api_rate_limit(spotify);
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
    wait_for_api_rate_limit(spotify, Some(SPOTIFY_MAX_RETRY_WAIT));

    let response = send_library_update_once(client, access_token, track_uri, should_save)?;
    if response.status().is_success() {
        return Ok(());
    }

    let status_code = response.status().as_u16();
    if status_code == 429 {
        apply_api_rate_limit_from_response(spotify, &response);
        let _body = response.text().unwrap_or_default();
        log::info!(
            "Spotify library update rate-limited; retrying once after Retry-After for track_id='{}'",
            track_id
        );
        wait_for_api_rate_limit(spotify, Some(SPOTIFY_MAX_RETRY_WAIT));

        let retry = send_library_update_once(client, access_token, track_uri, should_save)?;
        if retry.status().is_success() {
            return Ok(());
        }
        let retry_status = retry.status().as_u16();
        if retry_status == 429 {
            apply_api_rate_limit_from_response(spotify, &retry);
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

    if !allow_network_fetch || is_api_rate_limited(spotify) {
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
    if fresh || is_api_rate_limited(spotify) {
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
        if is_api_rate_limited(spotify) {
            return Ok(cache
                .track_id
                .as_deref()
                .filter(|id| *id == track_id)
                .and_then(|_| cache.saved));
        }
    } else if is_api_rate_limited(spotify) {
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
    ensure_api_available_for_background(spotify)?;

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
    apply_api_rate_limit_from_response(spotify, &response);
    let body = response.text().unwrap_or_default();
    Err(format!(
        "Spotify saved-track check failed for track_id='{}' track_uri='{}': {} {}",
        track_id, track_uri, status, body
    ))
}

pub fn adjust_volume(spotify: &SpotifyState, delta: i32) -> Result<u8, String> {
    let playback = get_current_playback(spotify)?;
    let device_id = playback.device_id;
    let current = playback.volume_percent as i32;
    let next_volume = (current + delta).clamp(0, 100) as u8;
    set_volume(spotify, next_volume, Some(device_id))
}

pub fn set_volume(spotify: &SpotifyState, volume_percent: u8, device_id: Option<String>) -> Result<u8, String> {
    let device_id = match device_id {
        Some(id) => id,
        None => get_current_playback(spotify)?.device_id,
    };
    let access_token = get_access_token(spotify)?;

    let client = Client::new();
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
    let playback = get_current_playback(spotify)?;
    let access_token = get_access_token(spotify)?;
    let client = Client::new();
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
    } else if is_api_rate_limited(spotify) {
        if let Some(cached) = read_playback_cache(spotify, Duration::from_secs(300)) {
            return Ok(cached);
        }
    }

    fetch_current_playback(spotify)
}

fn fetch_current_playback(spotify: &SpotifyState) -> Result<PlaybackSummary, String> {
    ensure_api_available_for_background(spotify)?;
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
        apply_api_rate_limit_from_response(spotify, &response);
        let body = response.text().unwrap_or_default();
        return Err(format!("Spotify playback query failed: {} {}", status, body));
    }

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
    store_playback_cache(spotify, &summary);
    Ok(summary)
}

pub fn toggle_shuffle(spotify: &SpotifyState) -> Result<bool, String> {
    let playback = get_current_playback(spotify)?;
    let next_state = !playback.shuffle_state;
    let access_token = get_access_token(spotify)?;
    let client = Client::new();
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
        Ok(next_state)
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Spotify shuffle update failed: {} {}", status, body))
    }
}

fn playback_item_image_url(item: &PlaybackItem) -> Option<String> {
    item.album
        .as_ref()
        .and_then(|album| album.images.first())
        .map(|image| image.url.clone())
        .or_else(|| item.images.first().map(|image| image.url.clone()))
}

fn get_access_token(spotify: &SpotifyState) -> Result<String, String> {
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
    let tokens = tokens_from_response(refreshed, Some(current_tokens.refresh_token))?;
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
    existing_refresh_token: Option<String>,
) -> Result<SpotifyTokens, String> {
    if response.token_type.to_lowercase() != "bearer" {
        return Err(format!("Unexpected Spotify token type: {}", response.token_type));
    }

    Ok(SpotifyTokens {
        access_token: response.access_token,
        refresh_token: response
            .refresh_token
            .or(existing_refresh_token)
            .ok_or_else(|| "Spotify did not return a refresh token".to_string())?,
        expires_at: now_unix_seconds() + response.expires_in,
        scope: response.scope,
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
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>TapTapDeck Spotify</title></head><body style=\"font-family: sans-serif; background:#0b1020; color:#f8fafc; padding:32px;\"><h1>{}</h1><p>{}</p></body></html>",
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

fn current_scopes(spotify: &SpotifyState) -> Result<Vec<String>, String> {
    let tokens = spotify
        .tokens
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Spotify is not authenticated yet.".to_string())?;
    Ok(parse_scopes(&tokens.scope))
}
