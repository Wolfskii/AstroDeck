use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use ytmusic::YtMusic;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeSearchTrack {
    pub video_id: String,
    pub title: String,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub cover_art_url: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeLocalPlaylist {
    pub id: String,
    pub name: String,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeLocalLibrary {
    pub profile_name: String,
    pub saved_tracks: Vec<YouTubeSearchTrack>,
    pub playlists: Vec<YouTubeLocalPlaylist>,
}

impl Default for YouTubeLocalLibrary {
    fn default() -> Self {
        Self {
            profile_name: "YouTube Music Guest".to_string(),
            saved_tracks: Vec::new(),
            playlists: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeStatus {
    pub provider: &'static str,
    pub is_configured: bool,
    pub is_authenticated: bool,
    pub has_active_device: bool,
    pub current_track_name: Option<String>,
    pub current_artist_name: Option<String>,
    pub current_album_name: Option<String>,
    pub current_cover_art_url: Option<String>,
    pub current_item_id: Option<String>,
    pub progress_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub playback_state: String,
    pub is_playing: bool,
    pub current_volume_percent: u8,
    pub message: String,
}

#[derive(Clone, Debug)]
struct YouTubePlayback {
    track: Option<YouTubeSearchTrack>,
    progress_ms: u64,
    progress_at: Option<Instant>,
    is_playing: bool,
    volume_percent: u8,
}

impl Default for YouTubePlayback {
    fn default() -> Self {
        Self {
            track: None,
            progress_ms: 0,
            progress_at: None,
            is_playing: false,
            volume_percent: 80,
        }
    }
}

pub struct YouTubeMusicState {
    api: Arc<YtMusic>,
    playback: Mutex<YouTubePlayback>,
    output_stream: Mutex<Option<OutputStream>>,
    sink: Mutex<Option<Arc<Sink>>>,
    app_handle: Mutex<Option<tauri::AppHandle>>,
    library_path: Mutex<Option<std::path::PathBuf>>,
}

impl Default for YouTubeMusicState {
    fn default() -> Self {
        Self {
            api: Arc::new(YtMusic::anonymous()),
            playback: Mutex::new(YouTubePlayback::default()),
            output_stream: Mutex::new(None),
            sink: Mutex::new(None),
            app_handle: Mutex::new(None),
            library_path: Mutex::new(None),
        }
    }
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("youtube-music")
            .worker_threads(4)
            .build()
            .expect("YouTube Music runtime")
    })
}

pub fn init(app: &tauri::AppHandle, state: &YouTubeMusicState) {
    if let Ok(mut handle) = state.app_handle.lock() {
        *handle = Some(app.clone());
    }
    if let Ok(base) = app.path().app_local_data_dir() {
        let _ = std::fs::create_dir_all(&base);
        if let Ok(mut path) = state.library_path.lock() {
            *path = Some(base.join("youtube_music_guest_library.json"));
        }
    }
}

fn library_path(state: &YouTubeMusicState) -> Result<std::path::PathBuf, String> {
    state
        .library_path
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "YouTube Music local library is not initialized.".to_string())
}

pub fn get_library(state: &YouTubeMusicState) -> Result<YouTubeLocalLibrary, String> {
    let path = library_path(state)?;
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Ok(YouTubeLocalLibrary::default());
    };
    serde_json::from_str(&contents).map_err(|error| format!("Local YouTube library is invalid: {error}"))
}

fn save_library(state: &YouTubeMusicState, library: &YouTubeLocalLibrary) -> Result<(), String> {
    let path = library_path(state)?;
    let contents = serde_json::to_string_pretty(library).map_err(|error| error.to_string())?;
    std::fs::write(path, contents).map_err(|error| error.to_string())
}

pub fn save_track(
    state: &YouTubeMusicState,
    track: YouTubeSearchTrack,
) -> Result<YouTubeLocalLibrary, String> {
    let mut library = get_library(state)?;
    library.saved_tracks.retain(|saved| saved.video_id != track.video_id);
    library.saved_tracks.insert(0, track);
    save_library(state, &library)?;
    Ok(library)
}

pub fn create_playlist(
    state: &YouTubeMusicState,
    name: String,
) -> Result<YouTubeLocalLibrary, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Playlist name is required.".to_string());
    }
    let mut library = get_library(state)?;
    let id = format!(
        "local-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis()
    );
    library.playlists.push(YouTubeLocalPlaylist {
        id,
        name: name.to_string(),
        track_ids: Vec::new(),
    });
    save_library(state, &library)?;
    Ok(library)
}

pub fn add_track_to_playlist(
    state: &YouTubeMusicState,
    playlist_id: String,
    track_id: String,
) -> Result<YouTubeLocalLibrary, String> {
    let mut library = get_library(state)?;
    let playlist = library
        .playlists
        .iter_mut()
        .find(|playlist| playlist.id == playlist_id)
        .ok_or_else(|| "Local YouTube playlist was not found.".to_string())?;
    if !playlist.track_ids.iter().any(|id| id == &track_id) {
        playlist.track_ids.push(track_id);
    }
    save_library(state, &library)?;
    Ok(library)
}

pub fn search(state: &YouTubeMusicState, query: &str) -> Result<Vec<YouTubeSearchTrack>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let tracks = runtime()
        .block_on(state.api.search_songs(query))
        .map_err(|error| format!("YouTube Music search failed: {error}"))?;
    Ok(tracks
        .into_iter()
        .filter_map(|track| {
            let video_id = track.video_id?;
            let artist_name = track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let cover_art_url = track
                .thumbnails
                .iter()
                .max_by_key(|thumbnail| thumbnail.width)
                .map(|thumbnail| thumbnail.url.clone());
            Some(YouTubeSearchTrack {
                video_id,
                title: track.title,
                artist_name,
                album_name: track.album.map(|album| album.name),
                cover_art_url,
                duration_ms: track.duration.map(|duration| duration.as_millis() as u64),
            })
        })
        .collect())
}

pub fn status(state: &YouTubeMusicState) -> YouTubeStatus {
    let playback = state
        .playback
        .lock()
        .map(|playback| playback.clone())
        .unwrap_or_default();
    let progress_ms = if playback.is_playing {
        Some(
            playback.progress_ms
                + playback
                    .progress_at
                    .map(|started| started.elapsed().as_millis() as u64)
                    .unwrap_or(0),
        )
    } else {
        Some(playback.progress_ms)
    };
    let track = playback.track;
    YouTubeStatus {
        provider: "youtubeMusic",
        is_configured: true,
        is_authenticated: false,
        has_active_device: track.is_some(),
        current_track_name: track.as_ref().map(|track| track.title.clone()),
        current_artist_name: track.as_ref().map(|track| track.artist_name.clone()),
        current_album_name: track.as_ref().and_then(|track| track.album_name.clone()),
        current_cover_art_url: track.as_ref().and_then(|track| track.cover_art_url.clone()),
        current_item_id: track.as_ref().map(|track| track.video_id.clone()),
        progress_ms,
        duration_ms: track.as_ref().and_then(|track| track.duration_ms),
        playback_state: if playback.is_playing {
            "playing".to_string()
        } else if track.is_some() {
            "paused".to_string()
        } else {
            "stopped".to_string()
        },
        is_playing: playback.is_playing,
        current_volume_percent: playback.volume_percent,
        message: "YouTube Music guest playback is ready. Sign-in and account libraries are not enabled."
            .to_string(),
    }
}

pub fn publish_status(state: &YouTubeMusicState) {
    let status = status(state);
    if let Ok(handle) = state.app_handle.lock() {
        if let Some(app) = handle.as_ref() {
            let _ = app.emit("youtube-status", &status);
            if let Some(app_state) = app.try_state::<crate::AppState>() {
                let payload = serde_json::json!({
                    "type": "youtubeStatus",
                    "payload": status,
                });
                let _ = app_state.log_bus.send(payload.to_string());
            }
        }
    }
}

pub fn play(
    state: &YouTubeMusicState,
    track: YouTubeSearchTrack,
) -> Result<(), String> {
    let (_format, bytes) = runtime()
        .block_on(state.api.load_audio(&track.video_id))
        .map_err(|error| format!("YouTube Music could not play this track: {error}"))?;
    let source = Decoder::try_from(Cursor::new(bytes))
        .map_err(|error| format!("YouTube Music audio could not be decoded: {error}"))?;
    let output_stream = OutputStreamBuilder::open_default_stream()
        .map_err(|error| format!("Audio output could not be opened: {error}"))?;
    let sink = Sink::connect_new(output_stream.mixer());
    let volume = state
        .playback
        .lock()
        .map(|playback| playback.volume_percent)
        .unwrap_or(80);
    sink.set_volume(f32::from(volume) / 100.0);
    sink.append(source);
    sink.play();

    *state
        .output_stream
        .lock()
        .map_err(|error| error.to_string())? = Some(output_stream);
    *state.sink.lock().map_err(|error| error.to_string())? = Some(Arc::new(sink));
    {
        let mut playback = state.playback.lock().map_err(|error| error.to_string())?;
        playback.track = Some(track);
        playback.progress_ms = 0;
        playback.progress_at = Some(Instant::now());
        playback.is_playing = true;
    }
    publish_status(state);
    Ok(())
}

pub fn toggle_play(state: &YouTubeMusicState) -> Result<(), String> {
    let sink = {
        let guard = state.sink.lock().map_err(|error| error.to_string())?;
        guard
            .as_ref()
            .ok_or_else(|| "YouTube Music has no selected track.".to_string())?
            .clone()
    };
    let mut playback = state.playback.lock().map_err(|error| error.to_string())?;
    if playback.is_playing {
        playback.progress_ms += playback
            .progress_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        playback.progress_at = None;
        playback.is_playing = false;
        sink.pause();
    } else {
        playback.progress_at = Some(Instant::now());
        playback.is_playing = true;
        sink.play();
    }
    drop(playback);
    publish_status(state);
    Ok(())
}

pub fn set_volume(state: &YouTubeMusicState, volume_percent: u8) -> Result<u8, String> {
    let volume_percent = volume_percent.min(100);
    if let Some(sink) = state
        .sink
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
    {
        sink.set_volume(f32::from(volume_percent) / 100.0);
    }
    if let Ok(mut playback) = state.playback.lock() {
        playback.volume_percent = volume_percent;
    }
    publish_status(state);
    Ok(volume_percent)
}

pub fn stop(state: &YouTubeMusicState) {
    if let Ok(mut sink) = state.sink.lock() {
        sink.take();
    }
    if let Ok(mut stream) = state.output_stream.lock() {
        stream.take();
    }
    if let Ok(mut playback) = state.playback.lock() {
        playback.track = None;
        playback.progress_ms = 0;
        playback.progress_at = None;
        playback.is_playing = false;
    }
    publish_status(state);
}

pub fn seek(state: &YouTubeMusicState, position_ms: u64) -> Result<(), String> {
    let sink = {
        let guard = state.sink.lock().map_err(|error| error.to_string())?;
        guard
            .as_ref()
            .ok_or_else(|| "YouTube Music has no selected track.".to_string())?
            .clone()
    };
    sink.try_seek(Duration::from_millis(position_ms))
        .map_err(|error| format!("YouTube Music seek failed: {error}"))?;
    if let Ok(mut playback) = state.playback.lock() {
        playback.progress_ms = position_ms;
        playback.progress_at = playback.is_playing.then(Instant::now);
    }
    publish_status(state);
    Ok(())
}
