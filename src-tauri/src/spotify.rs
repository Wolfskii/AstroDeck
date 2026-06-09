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
const SPOTIFY_LIBRARY_URL: &str = "https://api.spotify.com/v1/me/library";
const SPOTIFY_LIBRARY_CONTAINS_URL: &str = "https://api.spotify.com/v1/me/library/contains";
const SPOTIFY_SCOPES: &str =
    "user-library-modify user-library-read user-read-playback-state user-modify-playback-state";

#[derive(Default)]
pub struct SpotifyState {
    pub config: Mutex<SpotifyConfig>,
    pub tokens: Mutex<Option<SpotifyTokens>>,
    pub auth_session: Mutex<Option<PendingAuth>>,
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
    pub cover_art_url: Option<String>,
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
    item: Option<PlaybackItem>,
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
    #[serde(default)]
    artists: Vec<PlaybackArtist>,
    album: Option<PlaybackAlbum>,
    #[serde(default)]
    images: Vec<PlaybackImage>,
}

#[derive(Debug, Deserialize)]
struct PlaybackAlbum {
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
            playback_state: "stopped".to_string(),
            is_playing: false,
            current_volume_percent: None,
            current_item_type: None,
            current_item_id: None,
            is_current_track_saved: None,
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
            playback_state: "stopped".to_string(),
            is_playing: false,
            current_volume_percent: None,
            current_item_type: None,
            current_item_id: None,
            is_current_track_saved: None,
            granted_scopes,
            message: "Spotify is not connected yet.".to_string(),
        });
    }

    match get_current_playback(spotify) {
        Ok(playback) => {
            let mut message = "Spotify is connected.".to_string();
            let is_current_track_saved = if playback.item_type.as_deref() == Some("track") {
                if has_scope(spotify, "user-library-read")? {
                    match check_track_saved(spotify, playback.item_id.as_deref()) {
                        Ok(saved) => saved,
                        Err(err) => {
                            log::warn!("Spotify saved-state check unavailable: {}", err);
                            message = format!(
                                "Spotify is connected. Saved-state check unavailable: {}",
                                err
                            );
                            None
                        }
                    }
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
            playback_state: "stopped".to_string(),
            is_playing: false,
            current_volume_percent: None,
            current_item_type: None,
            current_item_id: None,
            is_current_track_saved: None,
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
    ensure_scope(spotify, "user-library-read")?;
    let playback = get_current_playback(spotify)?;
    let item_id = playback
        .item_id
        .ok_or_else(|| "Spotify has no current item to save".to_string())?;
    let item_type = playback
        .item_type
        .clone()
        .unwrap_or_else(|| "track".to_string());
    let granted_scopes = current_scopes(spotify)?;
    if item_type != "track" {
        return Err(format!(
            "Spotify like/dislike currently supports tracks only, but the active item type is '{}'",
            item_type
        ));
    }

    let already_saved = check_track_saved(spotify, Some(&item_id))?.unwrap_or(false);

    log::info!(
        "Spotify: attempting track library toggle for item_type='{}' item_id='{}' already_saved='{}' scopes='{}'",
        item_type,
        item_id,
        already_saved,
        granted_scopes.join(" ")
    );

    let item_uri = library_track_uri(&item_id);
    let access_token = get_access_token(spotify)?;
    let client = Client::new();
    let response = if already_saved {
        client
            .delete(SPOTIFY_LIBRARY_URL)
            .bearer_auth(access_token)
            .query(&[("uris", item_uri.as_str())])
            .body("")
            .send()
            .map_err(|e| e.to_string())?
    } else {
        client
            .put(SPOTIFY_LIBRARY_URL)
            .bearer_auth(access_token)
            .query(&[("uris", item_uri.as_str())])
            .body("")
            .send()
            .map_err(|e| e.to_string())?
    };

    if response.status().is_success() {
        Ok(!already_saved)
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!(
            "Spotify track library toggle failed for item_type='{}' item_id='{}' item_uri='{}' already_saved='{}' with scopes='{}': {} {}",
            item_type,
            item_id,
            item_uri,
            already_saved,
            granted_scopes.join(" "),
            status,
            body
        ))
    }
}

pub fn check_track_saved(
    spotify: &SpotifyState,
    track_id: Option<&str>,
) -> Result<Option<bool>, String> {
    let Some(track_id) = track_id else {
        return Ok(None);
    };

    let access_token = get_access_token(spotify)?;
    let track_uri = library_track_uri(track_id);
    let client = Client::new();
    let response = client
        .get(SPOTIFY_LIBRARY_CONTAINS_URL)
        .bearer_auth(access_token)
        .query(&[("uris", track_uri.as_str())])
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Spotify saved-track check failed for track_id='{}' track_uri='{}': {} {}",
            track_id, track_uri, status, body
        ));
    }

    let values: Vec<bool> = response.json().map_err(|e| e.to_string())?;
    Ok(values.into_iter().next())
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
        Ok(volume_percent)
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Spotify volume update failed: {} {}", status, body))
    }
}

pub fn get_current_playback(spotify: &SpotifyState) -> Result<PlaybackSummary, String> {
    let access_token = get_access_token(spotify)?;
    let client = Client::new();
    let response = client
        .get(SPOTIFY_CURRENT_PLAYBACK_URL)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().as_u16() == 204 {
        return Err("Spotify has no active playback device right now.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Spotify playback query failed: {} {}", status, body));
    }

    let playback: PlaybackResponse = response.json().map_err(|e| e.to_string())?;
    let device_id = playback
        .device
        .id
        .ok_or_else(|| "Spotify playback did not expose an active device id".to_string())?;

    Ok(PlaybackSummary {
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
        cover_art_url: playback.item.as_ref().and_then(playback_item_image_url),
    })
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
