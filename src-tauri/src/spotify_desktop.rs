use bytes::Bytes;
use http::header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use http::{HeaderMap, Method, Request};
use librespot_core::authentication::Credentials;
use librespot_core::cache::Cache;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_core::spclient::CLIENT_TOKEN;
use librespot_core::{SpotifyId, SpotifyUri};
use librespot_metadata::audio::UniqueFields;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig, VolumeCtrl};
use librespot_playback::mixer::{self, Mixer, MixerConfig};
use librespot_playback::player::{Player, PlayerEvent};
use librespot_protocol::playlist4_external::{
    Add, CreateListReply, Delta, Item, ListAttributes, ListChanges, Op, Rem, SelectedListContent,
};
use librespot_metadata::image::ImageSize;
use librespot_metadata::{Album, Artist, Metadata, Track};
use protobuf::Message;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::Manager;
use tokio::runtime::Runtime;

use crate::spotify::{
    get_access_token, parse_color_lyrics, token_has_streaming_scope, SpotifyPlaylist,
    SpotifyPlaylistPage, SpotifyQueueView, SpotifyState, SpotifyTrackLyrics, TrackPreview,
    OFFICIAL_CLIENT_ID,
};

// Spotify may silently cap large rootlist requests at 250 entries without
// setting `truncated`. Keep requests below that cap so pagination is reliable.
const ROOTLIST_LIMIT: usize = 100;
const ROOTLIST_MAX_PAGES: usize = 100;
const PLAYLIST_PAGE: usize = 200;
const IMAGE_CDN: &str = "https://i.scdn.co/image/";
pub const OFFICIAL_DEVICE_NAME: &str = "AstroDeck";
const PREV_RESTART_MS: u64 = 3_000;
const METADATA_WAIT: Duration = Duration::from_millis(2_000);
const COLLECTION_SET: &str = "collection";
const COLLECTION_CONTENT_TYPE: &str = "application/vnd.collection-v2.spotify.proto";
const COLOR_LYRICS_URL: &str = "https://spclient.wg.spotify.com/color-lyrics/v2/track";
const LYRICS_APP_PLATFORM: &str = "WebPlayer";
/// Progress-only saves of the queue happen at most this often while a track plays.
const QUEUE_PROGRESS_SAVE_INTERVAL: Duration = Duration::from_secs(10);
/// Spotify's history must be this much newer than AstroDeck's saved queue before it wins, so the
/// history entry for a track AstroDeck itself just played does not replace the queue.
pub(crate) const RESUME_HISTORY_SLACK_MS: u64 = 90_000;
const TRACK_META_CACHE_MAX: usize = 4_000;

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("spotify-desktop")
            .worker_threads(4)
            .build()
            .expect("Spotify desktop runtime")
    })
}

#[derive(Clone, Debug)]
pub struct OfficialPlayback {
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub cover_url: Option<String>,
    pub item_id: Option<String>,
    pub item_type: Option<String>,
    pub duration_ms: Option<u64>,
    pub progress_ms: Option<u64>,
    pub progress_at: Option<Instant>,
    pub is_playing: bool,
    pub volume_percent: u8,
    pub shuffle: bool,
    pub player_ready: bool,
    pub metadata_epoch: u64,
}

impl Default for OfficialPlayback {
    fn default() -> Self {
        Self {
            track_name: None,
            artist_name: None,
            album_name: None,
            cover_url: None,
            item_id: None,
            item_type: None,
            duration_ms: None,
            progress_ms: None,
            progress_at: None,
            is_playing: false,
            volume_percent: crate::spotify::DEFAULT_SPOTIFY_VOLUME_PERCENT,
            shuffle: false,
            player_ready: false,
            metadata_epoch: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct PlayQueue {
    tracks: Vec<SpotifyUri>,
    order: Vec<usize>,
    position: usize,
    context_uri: Option<String>,
    context_name: Option<String>,
    /// The player has been given a track from this queue during this run.
    loaded: bool,
    /// The saved queue was already compared with Spotify's listening history this run.
    resume_checked: bool,
    /// Where the current track was when the queue was saved, until the player takes over.
    resume_progress_ms: Option<u64>,
    /// When the restored queue was saved (unix ms).
    saved_at_ms: Option<u64>,
}

/// The queue as written to disk, so a restart can carry on where listening stopped.
#[derive(Serialize, Deserialize)]
struct SavedQueue {
    #[serde(default)]
    context_uri: Option<String>,
    #[serde(default)]
    context_name: Option<String>,
    tracks: Vec<String>,
    order: Vec<usize>,
    position: usize,
    #[serde(default)]
    progress_ms: u64,
    #[serde(default)]
    saved_at_ms: u64,
}

impl PlayQueue {
    fn replace(&mut self, tracks: Vec<SpotifyUri>, shuffle: bool) {
        self.replace_at(tracks, 0, shuffle, None, None);
    }

    fn replace_context(
        &mut self,
        tracks: Vec<SpotifyUri>,
        shuffle: bool,
        context_uri: Option<String>,
        context_name: Option<String>,
    ) {
        self.replace_at(tracks, 0, shuffle, context_uri, context_name);
    }

    /// Replaces the queue with `tracks`, starting at `start`. With shuffle on, only the tracks
    /// after it are shuffled.
    fn replace_at(
        &mut self,
        tracks: Vec<SpotifyUri>,
        start: usize,
        shuffle: bool,
        context_uri: Option<String>,
        context_name: Option<String>,
    ) {
        self.tracks = tracks;
        self.order = (0..self.tracks.len()).collect();
        self.position = start.min(self.order.len().saturating_sub(1));
        self.context_uri = context_uri;
        self.context_name = context_name;
        self.resume_checked = true;
        self.resume_progress_ms = None;
        self.saved_at_ms = None;
        if shuffle {
            self.shuffle_rest();
        }
    }

    pub fn context_uri(&self) -> Option<&str> {
        self.context_uri.as_deref()
    }

    fn upcoming_len(&self) -> usize {
        self.order.len().saturating_sub(self.position + 1)
    }

    fn upcoming_index(&self, index: usize) -> Option<usize> {
        let slot = self.position.checked_add(1)?.checked_add(index)?;
        (slot < self.order.len()).then_some(slot)
    }

    fn upcoming_id(&self, index: usize) -> Option<String> {
        let slot = self.upcoming_index(index)?;
        self.tracks.get(self.order[slot])?.to_id().ok()
    }

    fn upcoming(&self, offset: usize, limit: usize) -> Vec<SpotifyUri> {
        let start = (self.position + 1 + offset).min(self.order.len());
        let end = start.saturating_add(limit).min(self.order.len());
        self.order[start..end]
            .iter()
            .filter_map(|&index| self.tracks.get(index).cloned())
            .collect()
    }

    /// Moves the up-next track at `from` to `to` (both counted from the next track).
    fn move_upcoming(&mut self, from: usize, to: usize) -> bool {
        let (Some(from), Some(to)) = (self.upcoming_index(from), self.upcoming_index(to)) else {
            return false;
        };
        let entry = self.order.remove(from);
        self.order.insert(to, entry);
        true
    }

    fn remove_upcoming(&mut self, index: usize) -> bool {
        let Some(slot) = self.upcoming_index(index) else {
            return false;
        };
        self.order.remove(slot);
        true
    }

    /// Makes the up-next track at `index` the current one; the tracks before it are passed over.
    fn jump_upcoming(&mut self, index: usize) -> bool {
        let Some(slot) = self.upcoming_index(index) else {
            return false;
        };
        self.position = slot;
        true
    }

    fn to_saved(&self, progress_ms: u64) -> SavedQueue {
        SavedQueue {
            context_uri: self.context_uri.clone(),
            context_name: self.context_name.clone(),
            tracks: self
                .tracks
                .iter()
                .filter_map(|uri| uri.to_uri().ok())
                .collect(),
            order: self.order.clone(),
            position: self.position,
            progress_ms,
            // A restored queue that has not played keeps its time, so listening done in the
            // Spotify apps since then still counts as newer on the next start.
            saved_at_ms: if self.loaded {
                unix_now_ms()
            } else {
                self.saved_at_ms.unwrap_or_else(unix_now_ms)
            },
        }
    }

    fn from_saved(saved: SavedQueue) -> Option<Self> {
        let tracks: Vec<SpotifyUri> = saved
            .tracks
            .iter()
            .filter_map(|uri| SpotifyUri::from_uri(uri).ok())
            .collect();
        if tracks.is_empty()
            || tracks.len() != saved.tracks.len()
            || saved.position >= saved.order.len()
            || saved.order.iter().any(|&index| index >= tracks.len())
        {
            return None;
        }
        Some(Self {
            tracks,
            order: saved.order,
            position: saved.position,
            context_uri: saved.context_uri,
            context_name: saved.context_name,
            loaded: false,
            resume_checked: false,
            resume_progress_ms: Some(saved.progress_ms),
            saved_at_ms: Some(saved.saved_at_ms),
        })
    }

    pub fn contains_track_id(&self, track_id: &str) -> bool {
        self.tracks.iter().any(|uri| {
            uri.to_id()
                .ok()
                .is_some_and(|id| id.eq_ignore_ascii_case(track_id))
        })
    }

    fn current(&self) -> Option<&SpotifyUri> {
        let index = *self.order.get(self.position)?;
        self.tracks.get(index)
    }

    fn matches_current(&self, uri: &SpotifyUri) -> bool {
        self.current() == Some(uri)
    }

    fn peek_next(&self) -> Option<&SpotifyUri> {
        let index = *self.order.get(self.position + 1)?;
        self.tracks.get(index)
    }

    fn advance(&mut self) -> Option<&SpotifyUri> {
        if self.position + 1 >= self.order.len() {
            return None;
        }
        self.position += 1;
        self.current()
    }

    fn step_next(&mut self) -> Option<&SpotifyUri> {
        self.advance()
    }

    fn step_prev(&mut self) -> Option<&SpotifyUri> {
        if self.position == 0 {
            return self.current();
        }
        self.position -= 1;
        self.current()
    }

    fn set_shuffle(&mut self, shuffle: bool) {
        let current = self.current().cloned();
        self.order = (0..self.tracks.len()).collect();
        if let Some(current) = current {
            if let Some(index) = self.tracks.iter().position(|track| track == &current) {
                self.position = index;
            }
        }
        if shuffle {
            self.shuffle_rest();
        }
    }

    fn shuffle_rest(&mut self) {
        if self.position + 1 >= self.order.len() {
            return;
        }
        self.order[self.position + 1..].shuffle(&mut rand::thread_rng());
    }
}

pub fn drop_session(spotify: &SpotifyState) {
    let player = take_player(spotify);
    let _mixer = take_mixer(spotify);
    if let Ok(mut queue) = spotify.desktop_queue.lock() {
        *queue = PlayQueue::default();
    }
    drop(player);
    if let Ok(mut guard) = spotify.desktop_session.lock() {
        *guard = None;
    }
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        *playback = OfficialPlayback::default();
    }
}

pub fn clear_credentials(spotify: &SpotifyState) {
    drop_session(spotify);
    if let Some(path) = cache_dir(spotify) {
        let credentials = path.join("credentials.json");
        if credentials.exists() {
            let _ = fs::remove_file(credentials);
        }
    }
    if let Some(path) = queue_path(spotify) {
        let _ = fs::remove_file(path);
    }
}

fn queue_path(spotify: &SpotifyState) -> Option<PathBuf> {
    spotify.config.lock().ok()?.desktop_queue_path.clone()
}

/// Loads the queue saved by the last run, so the playlist it came from is known before anything
/// plays. Nothing is sent to the player until play, next or previous is pressed.
pub fn restore_saved_queue(spotify: &SpotifyState) {
    let Some(path) = queue_path(spotify) else {
        return;
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return;
    };
    let restored = match serde_json::from_str::<SavedQueue>(&raw) {
        Ok(saved) => PlayQueue::from_saved(saved),
        Err(err) => {
            log::warn!("Spotify saved queue could not be read: {err}");
            None
        }
    };
    let Some(restored) = restored else {
        return;
    };
    if let Ok(mut queue) = spotify.desktop_queue.lock() {
        if queue.current().is_none() {
            log::info!(
                "Spotify desktop: restored queue of {} tracks from {}",
                restored.tracks.len(),
                restored.context_uri.as_deref().unwrap_or("single tracks")
            );
            *queue = restored;
        }
    }
}

/// Writes the queue and where the current track is to disk. An empty queue never replaces a saved one.
pub(crate) fn persist_queue(spotify: &SpotifyState) {
    let Some(path) = queue_path(spotify) else {
        return;
    };
    let progress = playback_snapshot(spotify).progress_ms.unwrap_or(0);
    let saved = {
        let Ok(queue) = spotify.desktop_queue.lock() else {
            return;
        };
        if queue.current().is_none() {
            return;
        }
        let progress = if queue.loaded {
            progress
        } else {
            queue.resume_progress_ms.unwrap_or(0)
        };
        queue.to_saved(progress)
    };
    let json = match serde_json::to_string(&saved) {
        Ok(json) => json,
        Err(err) => {
            log::warn!("Spotify queue could not be encoded: {err}");
            return;
        }
    };
    let temp = path.with_extension("json.tmp");
    if let Err(err) = fs::write(&temp, json).and_then(|_| fs::rename(&temp, &path)) {
        log::warn!("Spotify queue could not be saved: {err}");
    }
    if let Ok(mut last) = last_queue_save().lock() {
        *last = Some(Instant::now());
    }
}

fn last_queue_save() -> &'static Mutex<Option<Instant>> {
    static LAST: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(None))
}

/// Saves the queue's progress while a track plays, without writing on every position tick.
fn persist_queue_progress(spotify: &SpotifyState) {
    let due = last_queue_save()
        .lock()
        .map(|last| last.is_none_or(|at| at.elapsed() >= QUEUE_PROGRESS_SAVE_INTERVAL))
        .unwrap_or(false);
    if due {
        persist_queue(spotify);
    }
}

fn queue_loaded(spotify: &SpotifyState) -> bool {
    spotify
        .desktop_queue
        .lock()
        .map(|queue| queue.loaded)
        .unwrap_or(false)
}

fn queue_has_current(spotify: &SpotifyState) -> bool {
    spotify
        .desktop_queue
        .lock()
        .map(|queue| queue.current().is_some())
        .unwrap_or(false)
}

/// Before the first track of a run, decides where listening carries on from: AstroDeck's saved
/// queue, or what the Spotify apps played after it according to Spotify's listening history.
fn prepare_resume(spotify: &SpotifyState) {
    let checked = spotify
        .desktop_queue
        .lock()
        .map(|queue| queue.loaded || queue.resume_checked)
        .unwrap_or(true);
    if checked {
        return;
    }
    if !queue_has_current(spotify) {
        restore_saved_queue(spotify);
    }
    let (saved_at, current_id) = {
        let Ok(mut queue) = spotify.desktop_queue.lock() else {
            return;
        };
        queue.resume_checked = true;
        (
            queue.saved_at_ms,
            queue.current().and_then(|uri| uri.to_id().ok()),
        )
    };

    let Some(recent) = crate::spotify::last_played_with_context(spotify) else {
        return;
    };
    let newer = saved_at.is_none_or(|saved| {
        recent.played_at_ms > saved.saturating_add(RESUME_HISTORY_SLACK_MS)
    });
    let same_track = current_id
        .as_deref()
        .is_some_and(|id| id.eq_ignore_ascii_case(&recent.track_id));
    if !newer || same_track {
        return;
    }
    if let Err(err) = queue_from_history(spotify, &recent) {
        log::warn!("Spotify desktop: could not pick up from Spotify history: {err}");
    }
}

/// Builds the queue around the track Spotify last played elsewhere, inside the playlist, album
/// or artist it was played from.
fn queue_from_history(
    spotify: &SpotifyState,
    recent: &crate::spotify::RecentPlay,
) -> Result<(), String> {
    let track = SpotifyUri::from_uri(&format!("spotify:track:{}", recent.track_id))
        .map_err(|e| format!("Spotify track id is invalid: {e}"))?;
    let (mut tracks, name) = match recent.context_uri.as_deref() {
        Some(context) => context_tracks(spotify, context).unwrap_or_else(|err| {
            log::info!("Spotify desktop: {context} could not be loaded for resume: {err}");
            (Vec::new(), None)
        }),
        None => (Vec::new(), None),
    };
    let context = (!tracks.is_empty())
        .then(|| recent.context_uri.clone())
        .flatten();
    let start = match tracks.iter().position(|uri| uri == &track) {
        Some(index) => index,
        None => {
            tracks.insert(0, track);
            0
        }
    };
    let shuffle = spotify
        .desktop_playback
        .lock()
        .map(|playback| playback.shuffle)
        .unwrap_or(false);
    let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
    queue.replace_at(tracks, start, shuffle, context.clone(), name);
    log::info!(
        "Spotify desktop: picking up from Spotify history in {}",
        context.as_deref().unwrap_or("a single track")
    );
    Ok(())
}

/// Hands the restored queue to the player for the first play, next or previous of a run.
fn resume_queue(spotify: &SpotifyState, command: &str) -> Result<(), String> {
    let progress = spotify
        .desktop_queue
        .lock()
        .map_err(|e| e.to_string())?
        .resume_progress_ms
        .unwrap_or(0);
    let position = match command {
        "nextTrack" => {
            spotify
                .desktop_queue
                .lock()
                .map_err(|e| e.to_string())?
                .step_next();
            0
        }
        "prevTrack" => {
            if progress <= PREV_RESTART_MS {
                spotify
                    .desktop_queue
                    .lock()
                    .map_err(|e| e.to_string())?
                    .step_prev();
            }
            0
        }
        _ => progress,
    };
    load_current_at(spotify, true, position)
}

pub fn ensure_session(spotify: &SpotifyState) -> Result<Session, String> {
    {
        let mut guard = spotify.desktop_session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = guard.as_ref() {
            if !session.is_invalid() {
                return Ok(session.clone());
            }
        }
        *guard = None;
    }
    let _ = take_player(spotify);
    let _ = take_mixer(spotify);

    let cache = open_cache(spotify);
    if let Some(stored) = cache.as_ref().and_then(|cache| cache.credentials()) {
        match connect_session(stored, cache.clone()) {
            Ok(session) => {
                store_session(spotify, session.clone());
                return Ok(session);
            }
            Err(err) => {
                log::info!(
                    "Spotify stored desktop credentials failed, trying access token: {err}"
                );
            }
        }
    }

    let session = connect_with_access_token(spotify, cache)?;
    store_session(spotify, session.clone());
    Ok(session)
}

fn player_start_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn ensure_player(spotify: &SpotifyState) -> Result<(), String> {
    let _start = player_start_lock().lock().map_err(|e| e.to_string())?;
    if session_is_live(spotify) && player_is_live(spotify) {
        mark_player_ready(spotify);
        return Ok(());
    }

    if !session_is_live(spotify) {
        let player = take_player(spotify);
        let _mixer = take_mixer(spotify);
        drop(player);
    }

    let session = ensure_session(spotify)?;
    if player_is_live(spotify) {
        mark_player_ready(spotify);
        return Ok(());
    }

    let volume_percent = spotify
        .desktop_playback
        .lock()
        .map(|playback| playback.volume_percent)
        .unwrap_or(50);
    let (player, mixer) = start_player(session, spotify, volume_percent)?;
    *spotify.desktop_player.lock().map_err(|e| e.to_string())? = Some(player);
    *spotify.desktop_mixer.lock().map_err(|e| e.to_string())? = Some(mixer);
    mark_player_ready(spotify);
    Ok(())
}

pub fn playback_snapshot(spotify: &SpotifyState) -> OfficialPlayback {
    let Ok(guard) = spotify.desktop_playback.lock() else {
        return OfficialPlayback::default();
    };
    let mut snapshot = guard.clone();
    snapshot.progress_ms = interpolated_progress(&snapshot);
    snapshot
}

pub fn should_show_spotify_scene(spotify: &SpotifyState) -> bool {
    let Ok(playback) = spotify.desktop_playback.lock() else {
        return false;
    };
    playback.is_playing || playback.track_name.is_some()
}

pub fn play_context_uri(spotify: &SpotifyState, context_uri: &str) -> Result<(), String> {
    ensure_player(spotify)?;
    let (tracks, name) = context_tracks(spotify, context_uri)?;
    if tracks.is_empty() {
        return Err("This playlist has no playable tracks.".to_string());
    }
    start_context(spotify, tracks, context_uri, name)
}

/// Plays the user's Liked Songs, which is a library collection rather than a playlist.
pub fn play_liked_songs(spotify: &SpotifyState) -> Result<(), String> {
    ensure_player(spotify)?;
    let (tracks, name) = context_tracks(spotify, crate::spotify::LIKED_SONGS_URI)?;
    if tracks.is_empty() {
        return Err("Liked Songs has no playable tracks.".to_string());
    }
    start_context(spotify, tracks, crate::spotify::LIKED_SONGS_URI, name)
}

/// The tracks of a playlist, album, artist (their top tracks) or Liked Songs, with its name.
fn context_tracks(
    spotify: &SpotifyState,
    context_uri: &str,
) -> Result<(Vec<SpotifyUri>, Option<String>), String> {
    if context_uri == crate::spotify::LIKED_SONGS_URI {
        let ids = crate::spotify::liked_track_ids(spotify)?;
        let tracks = ids
            .iter()
            .filter_map(|id| SpotifyUri::from_uri(&format!("spotify:track:{id}")).ok())
            .collect();
        return Ok((tracks, Some("Liked Songs".to_string())));
    }
    let session = ensure_session(spotify)?;
    if context_uri.starts_with("spotify:album:") {
        return fetch_album_tracks(&session, context_uri);
    }
    if context_uri.starts_with("spotify:artist:") {
        return fetch_artist_tracks(&session, context_uri);
    }
    fetch_context_tracks(&session, context_uri)
}

fn fetch_album_tracks(
    session: &Session,
    context_uri: &str,
) -> Result<(Vec<SpotifyUri>, Option<String>), String> {
    let uri = SpotifyUri::from_uri(context_uri)
        .map_err(|e| format!("Spotify album id is invalid: {e}"))?;
    runtime().block_on(async {
        let album = tokio::time::timeout(Duration::from_secs(20), Album::get(session, &uri))
            .await
            .map_err(|_| "Spotify album timed out".to_string())?
            .map_err(|e| format!("Spotify album could not be loaded: {e}"))?;
        let tracks = album.tracks().cloned().collect();
        Ok((tracks, Some(album.name).filter(|name| !name.is_empty())))
    })
}

fn fetch_artist_tracks(
    session: &Session,
    context_uri: &str,
) -> Result<(Vec<SpotifyUri>, Option<String>), String> {
    let uri = SpotifyUri::from_uri(context_uri)
        .map_err(|e| format!("Spotify artist id is invalid: {e}"))?;
    let country = session.country();
    runtime().block_on(async {
        let artist = tokio::time::timeout(Duration::from_secs(20), Artist::get(session, &uri))
            .await
            .map_err(|_| "Spotify artist timed out".to_string())?
            .map_err(|e| format!("Spotify artist could not be loaded: {e}"))?;
        let tracks = artist.top_tracks.for_country(&country).0;
        Ok((tracks, Some(artist.name).filter(|name| !name.is_empty())))
    })
}

fn start_context(
    spotify: &SpotifyState,
    tracks: Vec<SpotifyUri>,
    context_uri: &str,
    context_name: Option<String>,
) -> Result<(), String> {
    let shuffle = spotify
        .desktop_playback
        .lock()
        .map(|playback| playback.shuffle)
        .unwrap_or(false);
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        queue.replace_context(tracks, shuffle, Some(context_uri.to_string()), context_name);
    }

    let epoch = playback_snapshot(spotify).metadata_epoch;
    load_current(spotify, true)?;
    wait_for_metadata(spotify, epoch);
    crate::spotify::publish_status(spotify);
    log::info!("Spotify desktop: streaming {context_uri} in AstroDeck");
    Ok(())
}

pub fn set_track_saved(
    spotify: &SpotifyState,
    track_id: &str,
    saved: bool,
) -> Result<(), String> {
    let session = ensure_session(spotify)?;
    let username = session.username();
    if username.trim().is_empty() {
        return Err("Spotify liked-songs collection is not ready yet.".to_string());
    }
    let uri = format!("spotify:track:{track_id}");
    let added_at = if saved { unix_now_i32() } else { 0 };
    let body = encode_collection_write(
        &username,
        &uri,
        saved,
        added_at,
        &random_client_update_id(),
    );
    collection_post(&session, "/collection/v2/write", &body)?;
    Ok(())
}

pub fn is_track_saved(spotify: &SpotifyState, track_id: &str) -> Result<Option<bool>, String> {
    let session = ensure_session(spotify)?;
    let username = session.username();
    if username.trim().is_empty() {
        return Ok(None);
    }
    let uri = format!("spotify:track:{track_id}");
    let body = encode_collection_contains(&username, &uri);
    let response = collection_post(&session, "/collection/v2/contains", &body)?;
    Ok(parse_contains_response(&response))
}

pub fn fetch_lyrics(spotify: &SpotifyState, track_id: &str) -> Result<SpotifyTrackLyrics, String> {
    let session = ensure_session(spotify)?;
    let id = SpotifyId::from_base62(track_id)
        .map_err(|e| format!("Spotify track id is invalid: {e}"))?;
    let id62 = id
        .to_base62()
        .map_err(|e| format!("Spotify track id is invalid: {e}"))?;

    match lyrics_get(&session, &lyrics_request_uri(&id62)) {
        Ok(bytes) => parse_color_lyrics(track_id, &bytes),
        Err(err) if lyrics_missing(&err) => {
            if let Some(cover) = playback_snapshot(spotify)
                .cover_url
                .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
            {
                match lyrics_get(&session, &lyrics_image_uri(&id62, &cover)) {
                    Ok(bytes) => parse_color_lyrics(track_id, &bytes),
                    Err(image_err) if lyrics_missing(&image_err) => {
                        Ok(SpotifyTrackLyrics::unavailable(track_id))
                    }
                    Err(image_err) => Err(image_err),
                }
            } else {
                Ok(SpotifyTrackLyrics::unavailable(track_id))
            }
        }
        Err(err) => Err(err),
    }
}

fn lyrics_request_uri(track_id: &str) -> String {
    format!("{COLOR_LYRICS_URL}/{track_id}?format=json&vocalRemoval=false&market=from_token")
}

fn lyrics_image_uri(track_id: &str, cover_url: &str) -> String {
    format!(
        "{COLOR_LYRICS_URL}/{track_id}/image/{}?format=json&vocalRemoval=false&market=from_token",
        encode_lyrics_image_path(cover_url)
    )
}

fn lyrics_missing(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("404") || lower.contains("not found")
}

fn lyrics_get(session: &Session, uri: &str) -> Result<Vec<u8>, String> {
    runtime().block_on(async {
        let token = tokio::time::timeout(Duration::from_secs(20), session.login5().auth_token())
            .await
            .map_err(|_| "Spotify lyrics timed out".to_string())?
            .map_err(|e| format!("Spotify lyrics could not be loaded: {e}"))?;
        let client_token =
            tokio::time::timeout(Duration::from_secs(20), session.spclient().client_token())
                .await
                .map_err(|_| "Spotify lyrics timed out".to_string())?
                .map_err(|e| format!("Spotify lyrics could not be loaded: {e}"))?;
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header(ACCEPT, "application/json")
            .header("app-platform", LYRICS_APP_PLATFORM)
            .header(
                AUTHORIZATION,
                format!("{} {}", token.token_type, token.access_token),
            )
            .header(CLIENT_TOKEN, client_token)
            .body(Bytes::new())
            .map_err(|e| format!("Spotify lyrics could not be loaded: {e}"))?;
        let body = tokio::time::timeout(
            Duration::from_secs(20),
            session.http_client().request_body(request),
        )
        .await
        .map_err(|_| "Spotify lyrics timed out".to_string())?
        .map_err(|e| format!("Spotify lyrics could not be loaded: {e}"))?;
        Ok(body.to_vec())
    })
}

fn encode_lyrics_image_path(url: &str) -> String {
    let mut encoded = String::new();
    for byte in url.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub fn handle_transport(spotify: &SpotifyState, command: &str) -> Result<(), String> {
    ensure_player(spotify)?;
    if !queue_loaded(spotify) {
        prepare_resume(spotify);
        if queue_has_current(spotify) {
            return resume_queue(spotify, command);
        }
    }
    match command {
        "togglePlay" => {
            let playing = spotify
                .desktop_playback
                .lock()
                .map(|playback| playback.is_playing)
                .unwrap_or(false);
            if playing {
                with_player(spotify, |player| {
                    player.pause();
                    Ok(())
                })
            } else {
                let queue_has_current = spotify
                    .desktop_queue
                    .lock()
                    .map(|queue| queue.current().is_some())
                    .unwrap_or(false);
                if queue_has_current {
                    with_player(spotify, |player| {
                        player.play();
                        Ok(())
                    })
                } else {
                    resume_last_played(spotify)
                }
            }
        }
        "nextTrack" => play_next(spotify),
        "prevTrack" => play_prev(spotify),
        other => Err(format!("Unknown Spotify player command: {other}")),
    }
}

pub fn stop(spotify: &SpotifyState) {
    if let Ok(guard) = spotify.desktop_player.lock() {
        if let Some(player) = guard.as_ref() {
            player.stop();
        }
    }
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.is_playing = false;
        playback.progress_at = None;
    }
    crate::spotify::publish_status(spotify);
}

pub fn toggle_shuffle(spotify: &SpotifyState) -> Result<bool, String> {
    ensure_player(spotify)?;
    let next = {
        let playback = spotify.desktop_playback.lock().map_err(|e| e.to_string())?;
        !playback.shuffle
    };
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        queue.set_shuffle(next);
    }
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.shuffle = next;
    }
    preload_next(spotify);
    persist_queue(spotify);
    Ok(next)
}

pub fn set_volume(spotify: &SpotifyState, volume_percent: u8) -> Result<u8, String> {
    ensure_player(spotify)?;
    let volume_percent = volume_percent.min(100);
    let mixer = {
        let guard = spotify.desktop_mixer.lock().map_err(|e| e.to_string())?;
        guard
            .clone()
            .ok_or_else(|| "Spotify player is not running in AstroDeck".to_string())?
    };
    mixer.set_volume(percent_to_volume(volume_percent));
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.volume_percent = volume_percent;
    }
    crate::spotify::persist_volume(spotify, volume_percent);
    Ok(volume_percent)
}

pub fn adjust_volume(spotify: &SpotifyState, delta: i32) -> Result<u8, String> {
    let current = spotify
        .desktop_playback
        .lock()
        .map(|playback| playback.volume_percent)
        .unwrap_or(crate::spotify::DEFAULT_SPOTIFY_VOLUME_PERCENT) as i32;
    set_volume(spotify, (current + delta).clamp(0, 100) as u8)
}

pub fn seek(spotify: &SpotifyState, position_ms: u64) -> Result<(), String> {
    ensure_player(spotify)?;
    let position = u32::try_from(position_ms).unwrap_or(u32::MAX);
    with_player(spotify, |player| {
        player.seek(position);
        Ok(())
    })?;
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.progress_ms = Some(position_ms);
        playback.progress_at = Some(Instant::now());
    }
    Ok(())
}

fn take_player(spotify: &SpotifyState) -> Option<Arc<Player>> {
    spotify
        .desktop_player
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

fn take_mixer(spotify: &SpotifyState) -> Option<Arc<dyn Mixer>> {
    spotify
        .desktop_mixer
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

fn session_is_live(spotify: &SpotifyState) -> bool {
    spotify
        .desktop_session
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|session| !session.is_invalid()))
        .unwrap_or(false)
}

fn player_is_live(spotify: &SpotifyState) -> bool {
    spotify
        .desktop_player
        .lock()
        .ok()
        .map(|guard| guard.as_ref().map(|player| !player.is_invalid()).unwrap_or(false))
        .unwrap_or(false)
}

fn mark_player_ready(spotify: &SpotifyState) {
    if let Ok(mut playback) = spotify.desktop_playback.lock() {
        playback.player_ready = true;
    }
}

fn store_session(spotify: &SpotifyState, session: Session) {
    if let Ok(mut guard) = spotify.desktop_session.lock() {
        *guard = Some(session);
    }
}

fn cache_dir(spotify: &SpotifyState) -> Option<PathBuf> {
    spotify
        .config
        .lock()
        .ok()?
        .desktop_cache_path
        .clone()
}

fn open_cache(spotify: &SpotifyState) -> Option<Cache> {
    let path = cache_dir(spotify)?;
    let files = path.join("files");
    Cache::new(Some(path), None::<PathBuf>, Some(files), None).ok()
}

fn connect_with_access_token(
    spotify: &SpotifyState,
    cache: Option<Cache>,
) -> Result<Session, String> {
    if token_has_streaming_scope(spotify)? == Some(false) {
        return Err(reconnect_for_desktop_session_error());
    }

    let access_token = get_access_token(spotify)?;
    connect_session(Credentials::with_access_token(access_token), cache)
        .map_err(desktop_session_login_error)
}

fn session_config() -> SessionConfig {
    let mut config = SessionConfig::default();
    config.client_id = OFFICIAL_CLIENT_ID.to_string();
    config
}

fn connect_session(credentials: Credentials, cache: Option<Cache>) -> Result<Session, String> {
    runtime().block_on(async {
        let session = Session::new(session_config(), cache);
        session
            .connect(credentials, true)
            .await
            .map_err(|e| format!("{e}"))?;
        Ok(session)
    })
}

fn start_player(
    session: Session,
    spotify: &SpotifyState,
    volume_percent: u8,
) -> Result<(Arc<Player>, Arc<dyn Mixer>), String> {
    let mixer_builder = mixer::find(None)
        .ok_or_else(|| player_start_error("audio mixer is unavailable"))?;
    let mixer = mixer_builder(MixerConfig::default()).map_err(player_start_error)?;
    mixer.set_volume(percent_to_volume(volume_percent));

    let sink_builder = audio_backend::find(None)
        .ok_or_else(|| player_start_error("audio output is unavailable"))?;
    let audio_format = AudioFormat::default();
    let mut player_config = PlayerConfig::default();
    player_config.position_update_interval = Some(Duration::from_secs(1));

    let player = Player::new(
        player_config,
        session,
        mixer.get_soft_volume(),
        move || sink_builder(None, audio_format),
    );
    let mut events = player.get_player_event_channel();
    let playback = spotify.desktop_playback.clone();
    let queue = spotify.desktop_queue.clone();
    let player_slot = spotify.desktop_player.clone();
    let app_handle = spotify
        .app_handle
        .lock()
        .ok()
        .and_then(|guard| guard.clone());

    runtime().spawn(async move {
        while let Some(event) = events.recv().await {
            let finished = match &event {
                PlayerEvent::EndOfTrack { track_id, .. }
                | PlayerEvent::Unavailable { track_id, .. } => Some(track_id.clone()),
                _ => None,
            };
            let publish = player_event_publishes_status(&event);
            let position_tick = matches!(event, PlayerEvent::PositionChanged { .. });
            apply_player_event(&playback, event);
            if publish {
                publish_status_from_app(app_handle.clone());
            } else if position_tick {
                if let Some(app) = app_handle.as_ref() {
                    persist_queue_progress(&app.state::<crate::AppState>().spotify);
                }
            }
            if let Some(track_id) = finished {
                continue_if_current(&player_slot, &queue, &track_id);
            }
        }
    });

    Ok((player, mixer))
}

fn with_player<R>(
    spotify: &SpotifyState,
    f: impl FnOnce(&Player) -> Result<R, String>,
) -> Result<R, String> {
    let guard = spotify.desktop_player.lock().map_err(|e| e.to_string())?;
    let player = guard
        .as_ref()
        .ok_or_else(|| "Spotify player is not running in AstroDeck".to_string())?;
    f(player)
}

fn load_current(spotify: &SpotifyState, start_playing: bool) -> Result<(), String> {
    load_current_at(spotify, start_playing, 0)
}

fn load_current_at(
    spotify: &SpotifyState,
    start_playing: bool,
    position_ms: u64,
) -> Result<(), String> {
    let uri = {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        let uri = queue
            .current()
            .cloned()
            .ok_or_else(|| "Spotify queue is empty".to_string())?;
        queue.loaded = true;
        queue.resume_progress_ms = None;
        uri
    };
    seed_playback_from_uri(spotify, &uri, start_playing, position_ms);
    let position = u32::try_from(position_ms).unwrap_or(0);
    with_player(spotify, |player| {
        player.load(uri, start_playing, position);
        Ok(())
    })?;
    preload_next(spotify);
    persist_queue(spotify);
    Ok(())
}

fn resume_last_played(spotify: &SpotifyState) -> Result<(), String> {
    let track_id = crate::spotify::last_played_track_id(spotify)
        .ok_or_else(|| "No last played Spotify track is available to resume.".to_string())?;
    let uri = SpotifyUri::from_uri(&format!("spotify:track:{track_id}"))
        .map_err(|e| format!("Last played Spotify track is invalid: {e}"))?;
    let shuffle = spotify
        .desktop_playback
        .lock()
        .map(|playback| playback.shuffle)
        .unwrap_or(false);
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        queue.replace(vec![uri], shuffle);
    }
    load_current(spotify, true)
}

fn seed_playback_from_uri(
    spotify: &SpotifyState,
    uri: &SpotifyUri,
    start_playing: bool,
    position_ms: u64,
) {
    let Ok(mut playback) = spotify.desktop_playback.lock() else {
        return;
    };
    if let Ok(id) = uri.to_id() {
        playback.item_id = Some(id);
        playback.item_type = Some("track".to_string());
    }
    playback.is_playing = start_playing;
    playback.progress_ms = Some(position_ms);
    playback.progress_at = Some(Instant::now());
}

fn wait_for_metadata(spotify: &SpotifyState, epoch_before: u64) {
    let deadline = Instant::now() + METADATA_WAIT;
    while Instant::now() < deadline {
        if playback_snapshot(spotify).metadata_epoch != epoch_before {
            return;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

fn player_event_publishes_status(event: &PlayerEvent) -> bool {
    matches!(
        event,
        PlayerEvent::TrackChanged { .. }
            | PlayerEvent::Playing { .. }
            | PlayerEvent::Paused { .. }
            | PlayerEvent::Stopped { .. }
            | PlayerEvent::Seeked { .. }
            | PlayerEvent::VolumeChanged { .. }
            | PlayerEvent::ShuffleChanged { .. }
    )
}

fn publish_status_from_app(app: Option<tauri::AppHandle>) {
    let Some(app) = app else {
        return;
    };
    std::thread::spawn(move || {
        let state = app.state::<crate::AppState>();
        crate::spotify::publish_status(&state.spotify);
    });
}

fn preload_next(spotify: &SpotifyState) {
    let Some(uri) = spotify
        .desktop_queue
        .lock()
        .ok()
        .and_then(|queue| queue.peek_next().cloned())
    else {
        return;
    };
    let _ = with_player(spotify, |player| {
        player.preload(uri);
        Ok(())
    });
}

fn play_next(spotify: &SpotifyState) -> Result<(), String> {
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        if queue.step_next().is_none() {
            return Ok(());
        }
    }
    load_current(spotify, true)
}

fn play_prev(spotify: &SpotifyState) -> Result<(), String> {
    let progress = interpolated_progress(&playback_snapshot(spotify)).unwrap_or(0);
    if progress > PREV_RESTART_MS {
        return seek(spotify, 0);
    }
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        queue.step_prev();
    }
    load_current(spotify, true)
}

fn continue_if_current(
    player_slot: &Mutex<Option<Arc<Player>>>,
    queue: &Mutex<PlayQueue>,
    ended: &SpotifyUri,
) {
    let next = {
        let Ok(mut queue) = queue.lock() else {
            return;
        };
        if !queue.matches_current(ended) {
            return;
        }
        queue.advance().cloned()
    };
    let Some(next) = next else {
        return;
    };
    let Ok(guard) = player_slot.lock() else {
        return;
    };
    let Some(player) = guard.as_ref() else {
        return;
    };
    player.load(next, true, 0);
    if let Some(following) = queue.lock().ok().and_then(|queue| queue.peek_next().cloned()) {
        player.preload(following);
    }
}

fn apply_player_event(playback: &Mutex<OfficialPlayback>, event: PlayerEvent) {
    let Ok(mut playback) = playback.lock() else {
        return;
    };
    match event {
        PlayerEvent::TrackChanged { audio_item } => {
            playback.track_name = Some(audio_item.name);
            playback.cover_url = audio_item.covers.first().map(|cover| cover.url.clone());
            playback.duration_ms = Some(u64::from(audio_item.duration_ms));
            playback.item_id = audio_item.track_id.to_id().ok();
            let (artist, album, item_type) = metadata_from_unique_fields(audio_item.unique_fields);
            playback.artist_name = artist;
            playback.album_name = album;
            playback.item_type = Some(item_type.to_string());
            playback.metadata_epoch = playback.metadata_epoch.wrapping_add(1);
        }
        PlayerEvent::Playing { position_ms, .. }
        | PlayerEvent::PositionCorrection { position_ms, .. } => {
            playback.is_playing = true;
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::Paused { position_ms, .. } => {
            playback.is_playing = false;
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::Seeked { position_ms, .. } => {
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::PositionChanged { position_ms, .. } => {
            set_progress(&mut playback, position_ms);
        }
        PlayerEvent::Stopped { .. } | PlayerEvent::EndOfTrack { .. } => {
            playback.is_playing = false;
        }
        PlayerEvent::VolumeChanged { volume } => {
            playback.volume_percent = volume_to_percent(volume);
        }
        PlayerEvent::ShuffleChanged { shuffle } => {
            playback.shuffle = shuffle;
        }
        PlayerEvent::Unavailable { track_id, denied, .. } => {
            log::warn!("Spotify track unavailable ({track_id:?}, denied={denied})");
            if denied {
                playback.is_playing = false;
            }
        }
        _ => {}
    }
}

fn metadata_from_unique_fields(
    fields: UniqueFields,
) -> (Option<String>, Option<String>, &'static str) {
    match fields {
        UniqueFields::Track { artists, album, .. } => {
            let names: Vec<String> = artists.0.into_iter().map(|artist| artist.name).collect();
            let artist = if names.is_empty() {
                None
            } else {
                Some(names.join(", "))
            };
            (artist, Some(album), "track")
        }
        UniqueFields::Local {
            artists, album, ..
        } => (artists, album, "track"),
        UniqueFields::Episode { show_name, .. } => {
            (Some(show_name.clone()), Some(show_name), "episode")
        }
    }
}

fn set_progress(playback: &mut OfficialPlayback, position_ms: u32) {
    playback.progress_ms = Some(u64::from(position_ms));
    playback.progress_at = Some(Instant::now());
}

fn interpolated_progress(playback: &OfficialPlayback) -> Option<u64> {
    let progress = playback.progress_ms?;
    if !playback.is_playing {
        return Some(progress);
    }
    let elapsed = playback
        .progress_at
        .map(|at| at.elapsed().as_millis() as u64)
        .unwrap_or(0);
    Some(match playback.duration_ms {
        Some(duration) => (progress + elapsed).min(duration),
        None => progress + elapsed,
    })
}

fn percent_to_volume(percent: u8) -> u16 {
    let max = u32::from(VolumeCtrl::MAX_VOLUME);
    let pct = u32::from(percent.min(100));
    ((pct * max + 50) / 100) as u16
}

fn volume_to_percent(volume: u16) -> u8 {
    let max = u32::from(VolumeCtrl::MAX_VOLUME);
    ((u32::from(volume) * 100 + max / 2) / max) as u8
}

fn reconnect_for_desktop_session_error() -> String {
    "Spotify desktop playlists need a fresh login. Disconnect and connect again so the browser prompt can grant desktop access.".to_string()
}

fn desktop_session_login_error(err: String) -> String {
    let lower = err.to_ascii_lowercase();
    if lower.contains("bad credentials") || lower.contains("login failed") {
        reconnect_for_desktop_session_error()
    } else {
        format!("Spotify desktop session failed: {err}")
    }
}

fn player_start_error(err: impl std::fmt::Display) -> String {
    let message = err.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("invalid_credentials")
        || lower.contains("bad credentials")
        || lower.contains("premium")
    {
        "Spotify Premium is required to play in AstroDeck. Reconnect a Premium account, or switch to a developer app if you want the Spotify desktop app to handle playback.".to_string()
    } else {
        format!("Spotify player failed to start: {message}")
    }
}

pub fn list_playlists(
    spotify: &SpotifyState,
    offset: u32,
    limit: u32,
) -> Result<SpotifyPlaylistPage, String> {
    let session = match ensure_session(spotify) {
        Ok(session) => session,
        Err(err) => {
            drop_session(spotify);
            return Err(err);
        }
    };

    match fetch_rootlist_page(&session, offset, limit) {
        Ok(page) => Ok(page),
        Err(err) => {
            drop_session(spotify);
            let session = ensure_session(spotify)?;
            fetch_rootlist_page(&session, offset, limit).map_err(|retry| {
                format!("{retry} (after reconnect; first error: {err})")
            })
        }
    }
}

fn fetch_context_tracks(
    session: &Session,
    context_uri: &str,
) -> Result<(Vec<SpotifyUri>, Option<String>), String> {
    let playlist_id = playlist_id_from_uri(context_uri)
        .ok_or_else(|| format!("Unsupported Spotify context: {context_uri}"))?;
    let playlist_id = SpotifyId::from_base62(playlist_id)
        .map_err(|e| format!("Spotify playlist id is invalid: {e}"))?;

    runtime().block_on(async {
        let mut tracks = Vec::new();
        let mut name = None;
        let mut from = 0usize;
        for _ in 0..ROOTLIST_MAX_PAGES {
            let endpoint = format!(
                "/playlist/v2/playlist/{}?from={from}&length={PLAYLIST_PAGE}",
                playlist_id
                    .to_base62()
                    .map_err(|e| format!("Spotify playlist id is invalid: {e}"))?
            );
            let body = tokio::time::timeout(
                Duration::from_secs(20),
                session
                    .spclient()
                    .request(&Method::GET, &endpoint, None, None),
            )
            .await
            .map_err(|_| "Spotify playlist timed out".to_string())?
            .map_err(|e| format!("Spotify playlist could not be loaded: {e}"))?;

            let content = SelectedListContent::parse_from_bytes(&body)
                .map_err(|e| format!("Spotify playlist could not be decoded: {e}"))?;
            if name.is_none() {
                name = content
                    .attributes
                    .as_ref()
                    .map(|attributes| attributes.name().to_string())
                    .filter(|name| !name.is_empty());
            }
            let Some(contents) = content.contents.as_ref() else {
                break;
            };
            let page_len = contents.items.len();
            for item in &contents.items {
                let uri = item.uri();
                if !uri.starts_with("spotify:track:") {
                    continue;
                }
                match SpotifyUri::from_uri(uri) {
                    Ok(parsed) => tracks.push(parsed),
                    Err(err) => log::debug!("Skipping unreadable playlist item {uri}: {err}"),
                }
            }
            if page_len == 0 || !contents.truncated() {
                break;
            }
            from += page_len;
        }
        Ok((tracks, name))
    })
}

fn fetch_rootlist_page(
    session: &Session,
    offset: u32,
    limit: u32,
) -> Result<SpotifyPlaylistPage, String> {
    runtime().block_on(async {
        let username = session.username();
        let mut items = Vec::new();
        let mut from = 0usize;

        loop {
            let body = tokio::time::timeout(
                Duration::from_secs(20),
                session.spclient().get_rootlist(from, Some(ROOTLIST_LIMIT)),
            )
            .await
            .map_err(|_| "Spotify desktop playlist list timed out".to_string())?
            .map_err(|e| format!("Spotify desktop playlist list failed: {e}"))?;

            let root = SelectedListContent::parse_from_bytes(&body)
                .map_err(|e| format!("Spotify desktop playlist list could not be decoded: {e}"))?;
            let page_len = root
                .contents
                .as_ref()
                .map(|contents| contents.items.len())
                .unwrap_or(0);
            items.extend(playlists_from_rootlist(&root, &username));

            let truncated = root
                .contents
                .as_ref()
                .map(|contents| contents.truncated())
                .unwrap_or(false);
            if page_len == 0 || (!truncated && page_len < ROOTLIST_LIMIT) {
                break;
            }
            from += page_len;
        }

        Ok(page_from_playlists(items, offset, limit))
    })
}

fn playlists_from_rootlist(root: &SelectedListContent, username: &str) -> Vec<SpotifyPlaylist> {
    let Some(contents) = root.contents.as_ref() else {
        return Vec::new();
    };
    let meta = &contents.meta_items;
    let mut items = Vec::new();
    for (index, item) in contents.items.iter().enumerate() {
        let Some(id) = playlist_id_from_uri(item.uri()) else {
            continue;
        };
        let meta = meta.get(index);
        let name = meta
            .map(|entry| entry.attributes.name())
            .filter(|name| !name.is_empty())
            .unwrap_or("Playlist")
            .to_string();
        let owner_name = meta
            .map(|entry| entry.owner_username())
            .filter(|owner| !owner.is_empty())
            .map(|owner| {
                if owner.eq_ignore_ascii_case(username) {
                    "You".to_string()
                } else {
                    owner.to_string()
                }
            });
        let owned = owner_name.as_deref() == Some("You");
        items.push(SpotifyPlaylist {
            id: id.to_string(),
            name,
            uri: format!("spotify:playlist:{id}"),
            image_url: meta.and_then(|entry| playlist_cover(&entry.attributes)),
            track_count: meta
                .map(|entry| entry.length().max(0) as u32)
                .unwrap_or(0),
            owner_name,
            owned,
        });
    }
    items
}

fn playlist_id_from_uri(uri: &str) -> Option<&str> {
    let id = uri
        .strip_prefix("spotify:playlist:")
        .or_else(|| uri.rsplit_once(":playlist:").map(|(_, id)| id))?;
    (!id.is_empty() && !id.contains(':')).then_some(id)
}

fn page_from_playlists(
    items: Vec<SpotifyPlaylist>,
    offset: u32,
    limit: u32,
) -> SpotifyPlaylistPage {
    let total = items.len() as u32;
    let start = (offset as usize).min(items.len());
    let end = (start + limit as usize).min(items.len());
    let sliced = items[start..end].to_vec();
    let next_offset = (end as u32 != total).then_some(end as u32);

    SpotifyPlaylistPage {
        items: sliced,
        offset,
        limit,
        total,
        next_offset,
    }
}

fn playlist_cover(
    attributes: &librespot_protocol::playlist4_external::ListAttributes,
) -> Option<String> {
    for target in ["xlarge", "large", "default", "small"] {
        if let Some(url) = attributes
            .picture_size
            .iter()
            .find(|size| size.target_name() == target)
            .map(|size| size.url())
            .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        {
            return Some(url.to_string());
        }
    }
    attributes
        .picture_size
        .first()
        .map(|size| size.url())
        .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        .map(str::to_string)
        .or_else(|| image_url(attributes.picture()))
}

fn collection_post(session: &Session, endpoint: &str, body: &[u8]) -> Result<Vec<u8>, String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(COLLECTION_CONTENT_TYPE),
    );
    runtime().block_on(async {
        let bytes = tokio::time::timeout(
            Duration::from_secs(20),
            session
                .spclient()
                .request(&Method::POST, endpoint, Some(headers), Some(body)),
        )
        .await
        .map_err(|_| "Spotify collection request timed out".to_string())?
        .map_err(|e| format!("Spotify liked-songs update failed: {e}"))?;
        Ok(bytes.to_vec())
    })
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn unix_now_i32() -> i32 {
    i32::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(0)
}

fn random_client_update_id() -> String {
    let mut rng = rand::thread_rng();
    format!("{:016x}{:016x}", rng.gen::<u64>(), rng.gen::<u64>())
}

fn encode_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn encode_key(out: &mut Vec<u8>, field: u32, wire: u8) {
    encode_varint(out, u64::from((field << 3) | u32::from(wire)));
}

fn encode_string_field(out: &mut Vec<u8>, field: u32, value: &str) {
    encode_key(out, field, 2);
    encode_varint(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

fn encode_int32_field(out: &mut Vec<u8>, field: u32, value: i32) {
    encode_key(out, field, 0);
    encode_varint(out, u64::from(value as u32));
}

fn encode_bool_field(out: &mut Vec<u8>, field: u32, value: bool) {
    encode_key(out, field, 0);
    out.push(u8::from(value));
}

fn encode_collection_item(uri: &str, saved: bool, added_at: i32) -> Vec<u8> {
    let mut item = Vec::new();
    encode_string_field(&mut item, 1, uri);
    if saved && added_at != 0 {
        encode_int32_field(&mut item, 2, added_at);
    }
    if !saved {
        encode_bool_field(&mut item, 3, true);
    }
    item
}

fn encode_collection_write(
    username: &str,
    uri: &str,
    saved: bool,
    added_at: i32,
    client_update_id: &str,
) -> Vec<u8> {
    let item = encode_collection_item(uri, saved, added_at);
    let mut body = Vec::new();
    encode_string_field(&mut body, 1, username);
    encode_string_field(&mut body, 2, COLLECTION_SET);
    encode_key(&mut body, 3, 2);
    encode_varint(&mut body, item.len() as u64);
    body.extend_from_slice(&item);
    encode_string_field(&mut body, 4, client_update_id);
    body
}

fn encode_collection_contains(username: &str, uri: &str) -> Vec<u8> {
    let mut body = Vec::new();
    encode_string_field(&mut body, 1, username);
    encode_string_field(&mut body, 2, COLLECTION_SET);
    encode_string_field(&mut body, 3, uri);
    body
}

fn parse_contains_response(bytes: &[u8]) -> Option<bool> {
    let mut index = 0usize;
    let mut first = None;
    while index < bytes.len() {
        let (tag, next) = read_varint(bytes, index)?;
        index = next;
        let field = (tag >> 3) as u32;
        let wire = (tag & 7) as u8;
        if field == 1 && wire == 0 {
            let (value, next) = read_varint(bytes, index)?;
            index = next;
            if first.is_none() {
                first = Some(value != 0);
            }
        } else if field == 1 && wire == 2 {
            let (len, next) = read_varint(bytes, index)?;
            index = next;
            let end = index.saturating_add(len as usize);
            if end > bytes.len() {
                return None;
            }
            if first.is_none() && index < end {
                first = Some(bytes[index] != 0);
            }
            index = end;
        } else if !skip_field(bytes, &mut index, wire) {
            return None;
        }
    }
    first
}

fn read_varint(bytes: &[u8], mut index: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while index < bytes.len() {
        let byte = bytes[index];
        index += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, index));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
    None
}

fn skip_field(bytes: &[u8], index: &mut usize, wire: u8) -> bool {
    match wire {
        0 => {
            let Some((_, next)) = read_varint(bytes, *index) else {
                return false;
            };
            *index = next;
            true
        }
        1 => {
            *index = index.saturating_add(8);
            *index <= bytes.len()
        }
        2 => {
            let Some((len, next)) = read_varint(bytes, *index) else {
                return false;
            };
            *index = next.saturating_add(len as usize);
            *index <= bytes.len()
        }
        5 => {
            *index = index.saturating_add(4);
            *index <= bytes.len()
        }
        _ => false,
    }
}

fn image_url(file_id: &[u8]) -> Option<String> {
    if file_id.is_empty() {
        return None;
    }
    let hex = file_id.iter().fold(String::new(), |mut hex, byte| {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    Some(format!("{IMAGE_CDN}{hex}"))
}

pub fn current_playlist_id(spotify: &SpotifyState) -> Option<String> {
    let queue = spotify.desktop_queue.lock().ok()?;
    let uri = queue.context_uri()?;
    if uri == crate::spotify::LIKED_SONGS_URI {
        return Some(crate::spotify::LIKED_SONGS_ID.to_string());
    }
    playlist_id_from_uri(uri).map(str::to_string)
}

pub fn current_playlist_contains_track(spotify: &SpotifyState, track_id: &str) -> bool {
    let Ok(queue) = spotify.desktop_queue.lock() else {
        return false;
    };
    queue.contains_track_id(track_id)
}

/// Every track id in a playlist, fetched from Spotify.
pub fn fetch_playlist_track_ids(
    spotify: &SpotifyState,
    playlist_id: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let session = ensure_session(spotify)?;
    let (tracks, _) = fetch_context_tracks(&session, &format!("spotify:playlist:{playlist_id}"))?;
    Ok(tracks.iter().filter_map(|uri| uri.to_id().ok()).collect())
}

/// What plays now and next in AstroDeck's own player, `limit` up-next tracks from `offset`.
pub fn queue_view(
    spotify: &SpotifyState,
    offset: usize,
    limit: usize,
) -> Result<SpotifyQueueView, String> {
    if !queue_loaded(spotify) {
        prepare_resume(spotify);
    }
    let (current, upcoming, total, context_uri, context_name) = {
        let queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        (
            queue.current().cloned(),
            queue.upcoming(offset, limit),
            queue.upcoming_len(),
            queue.context_uri.clone(),
            queue.context_name.clone(),
        )
    };
    let mut uris: Vec<SpotifyUri> = current.iter().cloned().collect();
    uris.extend(upcoming);
    let mut previews = if uris.is_empty() {
        Vec::new()
    } else {
        let session = ensure_session(spotify)?;
        track_previews(spotify, &session, &uris)
    };
    let current = current.is_some().then(|| previews.remove(0));
    Ok(SpotifyQueueView {
        context_uri,
        context_name,
        current,
        up_next: previews,
        total: total as u32,
        offset: offset as u32,
        editable: true,
        message: None,
    })
}

/// Checks that the up-next track at `index` is still `item_id`, so an edit made while the
/// queue moved on does not hit the wrong track.
fn check_upcoming(queue: &PlayQueue, index: usize, item_id: &str) -> Result<(), String> {
    match queue.upcoming_id(index) {
        Some(id) if id.eq_ignore_ascii_case(item_id) => Ok(()),
        _ => Err("The queue changed. Showing it again.".to_string()),
    }
}

pub fn move_queue_item(
    spotify: &SpotifyState,
    from: usize,
    to: usize,
    item_id: &str,
) -> Result<(), String> {
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        check_upcoming(&queue, from, item_id)?;
        if !queue.move_upcoming(from, to) {
            return Err("That spot is outside the queue.".to_string());
        }
    }
    after_queue_edit(spotify);
    Ok(())
}

pub fn remove_queue_item(spotify: &SpotifyState, index: usize, item_id: &str) -> Result<(), String> {
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        check_upcoming(&queue, index, item_id)?;
        queue.remove_upcoming(index);
    }
    after_queue_edit(spotify);
    Ok(())
}

pub fn play_queue_item(spotify: &SpotifyState, index: usize, item_id: &str) -> Result<(), String> {
    ensure_player(spotify)?;
    {
        let mut queue = spotify.desktop_queue.lock().map_err(|e| e.to_string())?;
        check_upcoming(&queue, index, item_id)?;
        queue.jump_upcoming(index);
    }
    let epoch = playback_snapshot(spotify).metadata_epoch;
    load_current(spotify, true)?;
    wait_for_metadata(spotify, epoch);
    crate::spotify::publish_status(spotify);
    Ok(())
}

fn after_queue_edit(spotify: &SpotifyState) {
    if queue_loaded(spotify) {
        preload_next(spotify);
    }
    persist_queue(spotify);
}

/// Names, artists and covers for queue rows, from Spotify's metadata service. Results are kept,
/// so scrolling back through the queue does not ask again.
fn track_previews(
    spotify: &SpotifyState,
    session: &Session,
    uris: &[SpotifyUri],
) -> Vec<TrackPreview> {
    let ids: Vec<String> = uris.iter().map(|uri| uri.to_id().unwrap_or_default()).collect();
    let missing: Vec<SpotifyUri> = {
        let cache = spotify.desktop_track_meta.lock();
        uris.iter()
            .zip(&ids)
            .filter(|(_, id)| cache.as_ref().map_or(true, |cache| !cache.contains_key(*id)))
            .map(|(uri, _)| uri.clone())
            .collect()
    };
    if !missing.is_empty() {
        let fetched = runtime().block_on(async {
            let requests = missing.iter().map(|uri| async move {
                tokio::time::timeout(Duration::from_secs(20), Track::get(session, uri))
                    .await
                    .ok()
                    .and_then(Result::ok)
            });
            futures_util::future::join_all(requests).await
        });
        if let Ok(mut cache) = spotify.desktop_track_meta.lock() {
            if cache.len() > TRACK_META_CACHE_MAX {
                cache.clear();
            }
            for track in fetched.into_iter().flatten() {
                if let Some(preview) = preview_from_track(track) {
                    cache.insert(preview.item_id.clone(), preview);
                }
            }
        }
    }
    let cache = spotify.desktop_track_meta.lock().ok();
    ids.iter()
        .map(|id| {
            cache
                .as_ref()
                .and_then(|cache| cache.get(id).cloned())
                .unwrap_or_else(|| TrackPreview {
                    item_id: id.clone(),
                    item_type: "track".to_string(),
                    track_name: "Unavailable track".to_string(),
                    artist_name: None,
                    album_name: None,
                    cover_art_url: None,
                    duration_ms: None,
                })
        })
        .collect()
}

fn preview_from_track(track: Track) -> Option<TrackPreview> {
    let item_id = track.id.to_id().ok()?;
    let artists: Vec<String> = track
        .artists
        .iter()
        .map(|artist| artist.name.clone())
        .filter(|name| !name.is_empty())
        .collect();
    let covers = &track.album.covers;
    let cover = covers
        .iter()
        .find(|image| image.size == ImageSize::DEFAULT)
        .or_else(|| covers.first())
        .and_then(|image| image.id.to_base16().ok())
        .map(|hex| format!("{IMAGE_CDN}{hex}"));
    Some(TrackPreview {
        item_id,
        item_type: "track".to_string(),
        track_name: track.name,
        artist_name: (!artists.is_empty()).then(|| artists.join(", ")),
        album_name: Some(track.album.name).filter(|name| !name.is_empty()),
        cover_art_url: cover,
        duration_ms: u64::try_from(track.duration).ok(),
    })
}

pub fn set_playlist_track(
    spotify: &SpotifyState,
    playlist_id: &str,
    track_id: &str,
    add: bool,
) -> Result<(), String> {
    let session = ensure_session(spotify)?;
    let playlist_uri = format!("spotify:playlist:{playlist_id}");
    apply_playlist_track_change(&session, &playlist_uri, track_id, add)?;
    Ok(())
}

pub fn create_playlist(spotify: &SpotifyState, name: &str) -> Result<SpotifyPlaylist, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Playlist name is required.".to_string());
    }
    let session = ensure_session(spotify)?;
    let username = session.username();
    if username.trim().is_empty() {
        return Err("Spotify desktop playlists need a fresh login.".to_string());
    }

    runtime().block_on(async {
        let mut attributes = ListAttributes::new();
        attributes.set_name(name.to_string());
        let created = tokio::time::timeout(
            Duration::from_secs(20),
            session.spclient().request_with_protobuf(
                &Method::POST,
                "/playlist/v2/playlist",
                None,
                &attributes,
            ),
        )
        .await
        .map_err(|_| "Spotify playlist create timed out".to_string())?
        .map_err(|e| format!("Spotify playlist could not be created: {e}"))?;
        let reply = CreateListReply::parse_from_bytes(&created)
            .map_err(|e| format!("Spotify playlist create could not be decoded: {e}"))?;
        let uri = reply.uri().to_string();
        let id = playlist_id_from_uri(&uri)
            .ok_or_else(|| "Spotify created playlist did not return an id.".to_string())?
            .to_string();

        let root_body = tokio::time::timeout(
            Duration::from_secs(20),
            session.spclient().get_rootlist(0, Some(1)),
        )
        .await
        .map_err(|_| "Spotify playlist list timed out".to_string())?
        .map_err(|e| format!("Spotify playlist list failed: {e}"))?;
        let root = SelectedListContent::parse_from_bytes(&root_body)
            .map_err(|e| format!("Spotify playlist list could not be decoded: {e}"))?;
        let revision = root.revision().to_vec();
        if !revision.is_empty() {
            let mut item = Item::new();
            item.set_uri(uri.clone());
            let mut add = Add::new();
            add.set_add_first(true);
            add.items.push(item);
            let changes = playlist_changes(revision, librespot_protocol::playlist4_external::op::Kind::ADD, add, None);
            let endpoint = format!("/playlist/v2/user/{username}/rootlist/changes");
            let _ = tokio::time::timeout(
                Duration::from_secs(20),
                session
                    .spclient()
                    .request_with_protobuf(&Method::POST, &endpoint, None, &changes),
            )
            .await;
        }

        Ok(SpotifyPlaylist {
            id,
            name: name.to_string(),
            uri,
            image_url: None,
            track_count: 0,
            owner_name: Some("You".to_string()),
            owned: true,
        })
    })
}

fn apply_playlist_track_change(
    session: &Session,
    context_uri: &str,
    track_id: &str,
    add: bool,
) -> Result<(), String> {
    let playlist_id = playlist_id_from_uri(context_uri)
        .ok_or_else(|| format!("Unsupported Spotify context: {context_uri}"))?;
    let playlist_id = SpotifyId::from_base62(playlist_id)
        .map_err(|e| format!("Spotify playlist id is invalid: {e}"))?;
    let track_uri = format!("spotify:track:{track_id}");

    runtime().block_on(async {
        let id62 = playlist_id
            .to_base62()
            .map_err(|e| format!("Spotify playlist id is invalid: {e}"))?;
        let endpoint = format!("/playlist/v2/playlist/{id62}");
        let body = tokio::time::timeout(
            Duration::from_secs(20),
            session.spclient().request(&Method::GET, &endpoint, None, None),
        )
        .await
        .map_err(|_| "Spotify playlist timed out".to_string())?
        .map_err(|e| format!("Spotify playlist could not be loaded: {e}"))?;
        let content = SelectedListContent::parse_from_bytes(&body)
            .map_err(|e| format!("Spotify playlist could not be decoded: {e}"))?;
        let revision = content.revision().to_vec();
        if revision.is_empty() {
            return Err("Spotify playlist revision is missing.".to_string());
        }

        let mut item = Item::new();
        item.set_uri(track_uri);
        let kind = if add {
            librespot_protocol::playlist4_external::op::Kind::ADD
        } else {
            librespot_protocol::playlist4_external::op::Kind::REM
        };
        let mut add_op = Add::new();
        let mut rem_op = Rem::new();
        let changes = if add {
            add_op.set_add_last(true);
            add_op.items.push(item);
            playlist_changes(revision, kind, add_op, None)
        } else {
            rem_op.set_items_as_key(true);
            rem_op.items.push(item);
            playlist_changes(revision, kind, Add::new(), Some(rem_op))
        };
        let change_endpoint = format!("/playlist/v2/playlist/{id62}/changes");
        tokio::time::timeout(
            Duration::from_secs(20),
            session
                .spclient()
                .request_with_protobuf(&Method::POST, &change_endpoint, None, &changes),
        )
        .await
        .map_err(|_| "Spotify playlist update timed out".to_string())?
        .map_err(|e| format!("Spotify playlist could not be updated: {e}"))?;
        Ok(())
    })
}

fn playlist_changes(
    revision: Vec<u8>,
    kind: librespot_protocol::playlist4_external::op::Kind,
    add: Add,
    rem: Option<Rem>,
) -> ListChanges {
    let mut operation = Op::new();
    operation.set_kind(kind);
    if kind == librespot_protocol::playlist4_external::op::Kind::ADD {
        operation.add = protobuf::MessageField::some(add);
    } else if let Some(rem) = rem {
        operation.rem = protobuf::MessageField::some(rem);
    }

    let mut delta = Delta::new();
    delta.set_base_version(revision.clone());
    delta.ops.push(operation);

    let mut changes = ListChanges::new();
    changes.set_base_revision(revision);
    changes.deltas.push(delta);
    changes.set_want_resulting_revisions(true);
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(n: u8) -> SpotifyUri {
        let ids = [
            "4uLU6hMCjMI75M1A2tKUQC",
            "0VjIjW4GlUZAMYd2vXMi3b",
            "7qiZfU4dY1lWllzX7mPBI3",
        ];
        let id = ids[(n.saturating_sub(1) as usize).min(ids.len() - 1)];
        SpotifyUri::from_uri(&format!("spotify:track:{id}")).expect("track uri")
    }

    #[test]
    fn bad_credentials_asks_to_reconnect() {
        let message = desktop_session_login_error(
            "Permission denied { Login failed with reason: Bad credentials }".to_string(),
        );
        assert!(message.contains("Disconnect and connect again"));
        assert!(!message.contains("Bad credentials"));
    }

    #[test]
    fn volume_percent_roundtrip_stays_near_original() {
        for pct in [0_u8, 1, 25, 50, 75, 99, 100] {
            let back = volume_to_percent(percent_to_volume(pct));
            assert!(
                (i16::from(back) - i16::from(pct)).abs() <= 1,
                "{pct} mapped to {back}"
            );
        }
    }

    #[test]
    fn playlist_id_accepts_owned_and_standard_uris() {
        assert_eq!(
            playlist_id_from_uri("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"),
            Some("37i9dQZF1DXcBWIGoYBM5M")
        );
        assert_eq!(
            playlist_id_from_uri("spotify:user:alice:playlist:abc123"),
            Some("abc123")
        );
        assert_eq!(playlist_id_from_uri("spotify:folder:abc123"), None);
    }

    #[test]
    fn playlists_keep_library_order_when_paged() {
        let playlist = |id: &str, owner: &str| SpotifyPlaylist {
            id: id.to_string(),
            name: id.to_string(),
            uri: format!("spotify:playlist:{id}"),
            image_url: None,
            track_count: 1,
            owner_name: Some(owner.to_string()),
            owned: owner == "You",
        };
        let page = page_from_playlists(
            vec![
                playlist("followed", "someone"),
                playlist("owned", "You"),
            ],
            0,
            1,
        );
        assert_eq!(page.items[0].id, "followed");
        assert_eq!(page.total, 2);
        assert_eq!(page.next_offset, Some(1));
    }

    #[test]
    fn interpolated_progress_advances_while_playing() {
        let playback = OfficialPlayback {
            progress_ms: Some(1_000),
            progress_at: Some(Instant::now() - Duration::from_millis(500)),
            is_playing: true,
            duration_ms: Some(10_000),
            ..OfficialPlayback::default()
        };
        let next = interpolated_progress(&playback).expect("progress");
        assert!(next >= 1_400, "got {next}");
        assert!(next <= 1_700, "got {next}");
    }

    #[test]
    fn player_start_maps_keymaster_failure() {
        let message = player_start_error("Audio key: INVALID_CREDENTIALS");
        assert!(message.contains("Premium"));
        assert!(!message.contains("INVALID_CREDENTIALS"));
    }

    #[test]
    fn queue_advances_only_for_the_current_track() {
        let mut queue = PlayQueue::default();
        queue.replace(vec![track(1), track(2), track(3)], false);
        assert!(queue.matches_current(&track(1)));
        assert_eq!(queue.advance(), Some(&track(2)));
        assert!(!queue.matches_current(&track(1)));
        assert!(queue.matches_current(&track(2)));
    }

    #[test]
    fn queue_prev_stays_on_first_track() {
        let mut queue = PlayQueue::default();
        queue.replace(vec![track(1), track(2)], false);
        assert_eq!(queue.step_prev(), Some(&track(1)));
        assert_eq!(queue.step_next(), Some(&track(2)));
        assert_eq!(queue.step_prev(), Some(&track(1)));
    }

    #[test]
    fn queue_edits_only_touch_up_next() {
        let mut queue = PlayQueue::default();
        queue.replace(vec![track(1), track(2), track(3)], false);
        assert_eq!(queue.upcoming(0, 10), vec![track(2), track(3)]);
        assert!(queue.move_upcoming(1, 0));
        assert_eq!(queue.upcoming(0, 10), vec![track(3), track(2)]);
        assert!(queue.matches_current(&track(1)));
        assert!(!queue.move_upcoming(0, 2));
        assert!(queue.remove_upcoming(0));
        assert_eq!(queue.upcoming(0, 10), vec![track(2)]);
        assert!(queue.jump_upcoming(0));
        assert!(queue.matches_current(&track(2)));
        assert_eq!(queue.upcoming_len(), 0);
    }

    #[test]
    fn queue_starts_at_a_chosen_track() {
        let mut queue = PlayQueue::default();
        queue.replace_at(vec![track(1), track(2), track(3)], 1, false, None, None);
        assert!(queue.matches_current(&track(2)));
        assert_eq!(queue.step_prev(), Some(&track(1)));
    }

    #[test]
    fn saved_queue_restores_order_position_and_progress() {
        let mut queue = PlayQueue::default();
        queue.replace_context(
            vec![track(1), track(2), track(3)],
            false,
            Some("spotify:album:abc".to_string()),
            Some("Album".to_string()),
        );
        queue.move_upcoming(1, 0);
        queue.step_next();
        let saved = queue.to_saved(42_000);
        let json = serde_json::to_string(&saved).expect("encode");
        let restored = PlayQueue::from_saved(serde_json::from_str(&json).expect("decode"))
            .expect("restore");
        assert!(restored.matches_current(&track(3)));
        assert_eq!(restored.upcoming(0, 10), vec![track(2)]);
        assert_eq!(restored.resume_progress_ms, Some(42_000));
        assert_eq!(restored.context_name.as_deref(), Some("Album"));
        assert!(!restored.loaded && !restored.resume_checked);
    }

    #[test]
    fn broken_saved_queue_is_ignored() {
        let saved = SavedQueue {
            context_uri: None,
            context_name: None,
            tracks: vec!["spotify:track:4uLU6hMCjMI75M1A2tKUQC".to_string()],
            order: vec![0, 3],
            position: 0,
            progress_ms: 0,
            saved_at_ms: 0,
        };
        assert!(PlayQueue::from_saved(saved).is_none());
    }

    #[test]
    fn collection_write_encodes_like_and_unlike() {
        let liked = encode_collection_write(
            "alice",
            "spotify:track:abc",
            true,
            1_700_000_000,
            "cid",
        );
        assert!(payload_contains(&liked, "alice"));
        assert!(payload_contains(&liked, "collection"));
        assert!(payload_contains(&liked, "spotify:track:abc"));
        assert!(payload_contains(&liked, "cid"));
        assert!(!liked.windows(2).any(|window| window == [0x18, 0x01]));

        let unliked = encode_collection_write("alice", "spotify:track:abc", false, 0, "cid");
        assert!(unliked.windows(2).any(|window| window == [0x18, 0x01]));
    }

    #[test]
    fn collection_contains_response_reads_bool_and_packed() {
        assert_eq!(parse_contains_response(&[0x08, 0x01]), Some(true));
        assert_eq!(parse_contains_response(&[0x08, 0x00]), Some(false));
        assert_eq!(parse_contains_response(&[0x0a, 0x01, 0x01]), Some(true));
        assert_eq!(parse_contains_response(&[]), None);
    }

    #[test]
    fn lyrics_request_uses_web_player_host_and_query() {
        assert_eq!(
            lyrics_request_uri("4uLU6hMCjMI75M1A2tKUQC"),
            "https://spclient.wg.spotify.com/color-lyrics/v2/track/4uLU6hMCjMI75M1A2tKUQC?format=json&vocalRemoval=false&market=from_token"
        );
        assert_eq!(
            encode_lyrics_image_path("https://i.scdn.co/image/ab"),
            "https%3A%2F%2Fi.scdn.co%2Fimage%2Fab"
        );
        assert!(lyrics_missing("not found"));
        assert!(lyrics_missing("Response status code: 404"));
        assert!(!lyrics_missing("403 Forbidden"));
    }

    fn payload_contains(bytes: &[u8], value: &str) -> bool {
        let needle = value.as_bytes();
        bytes.windows(needle.len()).any(|window| window == needle)
    }
}
