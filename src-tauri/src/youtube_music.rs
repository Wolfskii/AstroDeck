use rodio::{Decoder, OutputStreamBuilder, Sink};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::{
    Arc, Mutex, OnceLock,
    mpsc::{self, Sender},
};
use std::thread;
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
    pub is_current_track_saved: bool,
    pub is_shuffle: bool,
    pub current_playlist_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug)]
struct YouTubePlayback {
    track: Option<YouTubeSearchTrack>,
    progress_ms: u64,
    progress_at: Option<Instant>,
    is_playing: bool,
    volume_percent: u8,
    shuffle: bool,
    current_playlist_id: Option<String>,
}

impl Default for YouTubePlayback {
    fn default() -> Self {
        Self {
            track: None,
            progress_ms: 0,
            progress_at: None,
            is_playing: false,
            volume_percent: 80,
            shuffle: false,
            current_playlist_id: None,
        }
    }
}

pub struct YouTubeMusicState {
    api: Arc<YtMusic>,
    playback: Mutex<YouTubePlayback>,
    audio_tx: Mutex<Option<Sender<AudioCommand>>>,
    app_handle: Mutex<Option<tauri::AppHandle>>,
    library_path: Mutex<Option<std::path::PathBuf>>,
}

impl Default for YouTubeMusicState {
    fn default() -> Self {
        Self {
            api: Arc::new(YtMusic::anonymous()),
            playback: Mutex::new(YouTubePlayback::default()),
            audio_tx: Mutex::new(None),
            app_handle: Mutex::new(None),
            library_path: Mutex::new(None),
        }
    }
}

enum AudioCommand {
    Play {
        bytes: Vec<u8>,
        volume: u8,
        result: Sender<Result<(), String>>,
    },
    Pause,
    Resume,
    Volume(u8),
    Seek(Duration),
}

fn ensure_audio_worker(state: &YouTubeMusicState) -> Result<Sender<AudioCommand>, String> {
    let mut guard = state.audio_tx.lock().map_err(|error| error.to_string())?;
    if let Some(sender) = guard.as_ref() {
        return Ok(sender.clone());
    }
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("youtube-audio".to_string())
        .spawn(move || audio_worker(receiver))
        .map_err(|error| format!("Audio worker could not start: {error}"))?;
    *guard = Some(sender.clone());
    Ok(sender)
}

fn audio_worker(receiver: mpsc::Receiver<AudioCommand>) {
    let Ok(output_stream) = OutputStreamBuilder::open_default_stream() else {
        while let Ok(command) = receiver.recv() {
            if let AudioCommand::Play { result, .. } = command {
                let _ = result.send(Err("Audio output could not be opened.".to_string()));
            }
        }
        return;
    };
    let mut sink: Option<Sink> = None;
    while let Ok(command) = receiver.recv() {
        match command {
            AudioCommand::Play {
                bytes,
                volume,
                result,
            } => {
                let decoded = Decoder::try_from(Cursor::new(bytes))
                    .map_err(|error| format!("YouTube Music audio could not be decoded: {error}"));
                match decoded {
                    Ok(source) => {
                        let player = Sink::connect_new(output_stream.mixer());
                        player.set_volume(f32::from(volume) / 100.0);
                        player.append(source);
                        player.play();
                        sink = Some(player);
                        let _ = result.send(Ok(()));
                    }
                    Err(error) => {
                        let _ = result.send(Err(error));
                    }
                }
            }
            AudioCommand::Pause => {
                if let Some(player) = sink.as_ref() {
                    player.pause();
                }
            }
            AudioCommand::Resume => {
                if let Some(player) = sink.as_ref() {
                    player.play();
                }
            }
            AudioCommand::Volume(volume) => {
                if let Some(player) = sink.as_ref() {
                    player.set_volume(f32::from(volume) / 100.0);
                }
            }
            AudioCommand::Seek(position) => {
                if let Some(player) = sink.as_ref() {
                    let _ = player.try_seek(position);
                }
            }
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
    let saved_track_id = library.saved_tracks.first().map(|track| track.video_id.clone());
    if let Some(liked) = library.playlists.iter_mut().find(|playlist| playlist.id == "liked") {
        if let Some(track_id) = saved_track_id {
            if !liked.track_ids.iter().any(|id| id == &track_id) {
                liked.track_ids.insert(0, track_id);
            }
        }
    } else {
        library.playlists.insert(
            0,
            YouTubeLocalPlaylist {
                id: "liked".to_string(),
                name: "Liked songs".to_string(),
                track_ids: library
                    .saved_tracks
                    .iter()
                    .map(|track| track.video_id.clone())
                    .collect(),
            },
        );
    }
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
        playlist.track_ids.push(track_id.clone());
    }
    if playlist_id == "liked" {
        if let Some(current) = state
            .playback
            .lock()
            .ok()
            .and_then(|playback| playback.track.clone())
            .filter(|track| track.video_id == track_id)
        {
            if !library.saved_tracks.iter().any(|saved| saved.video_id == track_id) {
                library.saved_tracks.insert(0, current);
            }
        }
    }
    save_library(state, &library)?;
    Ok(library)
}

pub fn remove_track_from_playlist(
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
    playlist.track_ids.retain(|id| id != &track_id);
    if playlist_id == "liked" {
        library.saved_tracks.retain(|saved| saved.video_id != track_id);
    }
    save_library(state, &library)?;
    Ok(library)
}

pub fn add_current_track_to_playlist(
    state: &YouTubeMusicState,
    playlist_id: String,
) -> Result<YouTubeLocalLibrary, String> {
    let track = state
        .playback
        .lock()
        .map_err(|error| error.to_string())?
        .track
        .clone()
        .ok_or_else(|| "YouTube Music has no selected track.".to_string())?;
    let mut library = get_library(state)?;
    if !library
        .saved_tracks
        .iter()
        .any(|saved| saved.video_id == track.video_id)
    {
        library.saved_tracks.insert(0, track.clone());
        save_library(state, &library)?;
    }
    add_track_to_playlist(state, playlist_id, track.video_id)
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
        .filter(|track| track.available && !track.is_video())
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

pub fn discover(
    state: &YouTubeMusicState,
    category: &str,
) -> Result<Vec<YouTubeSearchTrack>, String> {
    let query = match category {
        "popular" => "popular music hits",
        "playlists" => "popular music playlist",
        "chill" => "chill music playlist",
        _ => "trending music",
    };
    search(state, query)
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
    let is_current_track_saved = track.as_ref().and_then(|track| {
        get_library(state).ok().map(|library| {
            library.saved_tracks.iter().any(|saved| saved.video_id == track.video_id)
        })
    }).unwrap_or(false);
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
        is_current_track_saved,
        is_shuffle: playback.shuffle,
        current_playlist_id: playback.current_playlist_id.clone(),
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
    playlist_id: Option<String>,
) -> Result<(), String> {
    if let Some(app) = state
        .app_handle
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
    {
        let app_state = app.state::<crate::AppState>();
        crate::spotify_desktop::stop(&app_state.spotify);
    }
    let (_format, bytes) = runtime()
        .block_on(state.api.load_audio(&track.video_id))
        .map_err(|error| format!("YouTube Music could not play this track: {error}"))?;
    let volume = state
        .playback
        .lock()
        .map(|playback| playback.volume_percent)
        .unwrap_or(80);
    let sender = ensure_audio_worker(state)?;
    let (result_tx, result_rx) = mpsc::channel();
    sender
        .send(AudioCommand::Play {
            bytes,
            volume,
            result: result_tx,
        })
        .map_err(|error| format!("Audio worker is unavailable: {error}"))?;
    result_rx
        .recv_timeout(Duration::from_secs(20))
        .map_err(|error| format!("Audio worker timed out: {error}"))??;
    {
        let mut playback = state.playback.lock().map_err(|error| error.to_string())?;
        playback.track = Some(track);
        playback.progress_ms = 0;
        playback.progress_at = Some(Instant::now());
        playback.is_playing = true;
        playback.current_playlist_id = playlist_id;
    }
    publish_status(state);
    Ok(())
}

pub fn toggle_play(state: &YouTubeMusicState) -> Result<(), String> {
    let mut playback = state.playback.lock().map_err(|error| error.to_string())?;
    if playback.is_playing {
        playback.progress_ms += playback
            .progress_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        playback.progress_at = None;
        playback.is_playing = false;
        ensure_audio_worker(state)?
            .send(AudioCommand::Pause)
            .map_err(|error| error.to_string())?;
    } else {
        playback.progress_at = Some(Instant::now());
        playback.is_playing = true;
        ensure_audio_worker(state)?
            .send(AudioCommand::Resume)
            .map_err(|error| error.to_string())?;
    }
    drop(playback);
    publish_status(state);
    Ok(())
}

pub fn pause(state: &YouTubeMusicState) {
    if let Ok(sender) = ensure_audio_worker(state) {
        let _ = sender.send(AudioCommand::Pause);
    }
    if let Ok(mut playback) = state.playback.lock() {
        if playback.is_playing {
            playback.progress_ms += playback
                .progress_at
                .map(|started| started.elapsed().as_millis() as u64)
                .unwrap_or(0);
            playback.progress_at = None;
            playback.is_playing = false;
        }
    }
    publish_status(state);
}

pub fn set_volume(state: &YouTubeMusicState, volume_percent: u8) -> Result<u8, String> {
    let volume_percent = volume_percent.min(100);
    ensure_audio_worker(state)?
        .send(AudioCommand::Volume(volume_percent))
        .map_err(|error| error.to_string())?;
    if let Ok(mut playback) = state.playback.lock() {
        playback.volume_percent = volume_percent;
    }
    publish_status(state);
    Ok(volume_percent)
}

pub fn set_saved(state: &YouTubeMusicState, saved: bool) -> Result<bool, String> {
    let track = state
        .playback
        .lock()
        .map_err(|error| error.to_string())?
        .track
        .clone()
        .ok_or_else(|| "YouTube Music has no selected track.".to_string())?;
    if saved {
        save_track(state, track)?;
        return Ok(true);
    }
    let mut library = get_library(state)?;
    library.saved_tracks.retain(|item| item.video_id != track.video_id);
    if let Some(liked) = library.playlists.iter_mut().find(|playlist| playlist.id == "liked") {
        liked.track_ids.retain(|id| id != &track.video_id);
    }
    save_library(state, &library)?;
    Ok(false)
}

pub fn toggle_shuffle(state: &YouTubeMusicState) -> Result<bool, String> {
    let mut playback = state.playback.lock().map_err(|error| error.to_string())?;
    playback.shuffle = !playback.shuffle;
    let value = playback.shuffle;
    drop(playback);
    publish_status(state);
    Ok(value)
}

pub fn navigate(state: &YouTubeMusicState, direction: i32) -> Result<(), String> {
    let current_id = state
        .playback
        .lock()
        .map_err(|error| error.to_string())?
        .track
        .as_ref()
        .map(|track| track.video_id.clone())
        .ok_or_else(|| "YouTube Music has no selected track.".to_string())?;
    let library = get_library(state)?;
    let index = library
        .saved_tracks
        .iter()
        .position(|track| track.video_id == current_id)
        .ok_or_else(|| "Save more songs to use previous and next.".to_string())?;
    if library.saved_tracks.is_empty() {
        return Err("Save more songs to use previous and next.".to_string());
    }
    let shuffle = state
        .playback
        .lock()
        .map(|playback| playback.shuffle)
        .unwrap_or(false);
    let next = if shuffle && library.saved_tracks.len() > 1 {
        let mut rng = rand::thread_rng();
        let mut selected = rng.gen_range(0..library.saved_tracks.len());
        while selected == index {
            selected = rng.gen_range(0..library.saved_tracks.len());
        }
        selected
    } else if direction > 0 {
        (index + 1) % library.saved_tracks.len()
    } else if index == 0 {
        library.saved_tracks.len() - 1
    } else {
        index - 1
    };
    play(
        state,
        library.saved_tracks[next].clone(),
        state
            .playback
            .lock()
            .ok()
            .and_then(|playback| playback.current_playlist_id.clone()),
    )
}

pub fn seek(state: &YouTubeMusicState, position_ms: u64) -> Result<(), String> {
    ensure_audio_worker(state)?
        .send(AudioCommand::Seek(Duration::from_millis(position_ms)))
        .map_err(|error| error.to_string())?;
    if let Ok(mut playback) = state.playback.lock() {
        playback.progress_ms = position_ms;
        playback.progress_at = playback.is_playing.then(Instant::now);
    }
    publish_status(state);
    Ok(())
}
