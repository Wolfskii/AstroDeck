use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

const EVENT_NAME: &str = "os-now-playing";
const SEEK_JUMP_MS: i64 = 4000;
const SEEK_BACK_MS: i64 = 5000;
const RESTART_FROM_MS: i64 = 2500;
const RESTART_TO_MS: i64 = 1500;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OsNowPlaying {
    #[serde(default)]
    pub kind: String,
    pub source: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art_url: Option<String>,
    pub is_playing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(default = "default_active")]
    pub active: bool,
}

#[allow(dead_code)]
fn default_active() -> bool {
    true
}

struct LastPublish {
    source: String,
    title: String,
    artist: String,
    album: String,
    has_cover: bool,
    is_playing: bool,
    progress_ms: i64,
    at: Instant,
}

fn is_spotify_source(source: &str) -> bool {
    source.to_ascii_lowercase().contains("spotify")
}

fn ignore_source(source: &str) -> bool {
    let source = source.to_ascii_lowercase();
    source.is_empty()
        || source.contains("astrodeck")
        || source.contains("tap-tap-deck")
        || source.contains("shellexperiencehost")
        || source.contains("textinputhost")
}

fn last_from(payload: &OsNowPlaying) -> LastPublish {
    LastPublish {
        source: payload.source.clone(),
        title: payload.title.clone().unwrap_or_default(),
        artist: payload.artist.clone().unwrap_or_default(),
        album: payload.album.clone().unwrap_or_default(),
        has_cover: payload.cover_art_url.is_some(),
        is_playing: payload.is_playing,
        progress_ms: payload.progress_ms.unwrap_or(0),
        at: Instant::now(),
    }
}

fn identity_changed(last: &LastPublish, payload: &OsNowPlaying) -> bool {
    last.source != payload.source
        || last.title != payload.title.clone().unwrap_or_default()
        || last.artist != payload.artist.clone().unwrap_or_default()
        || last.album != payload.album.clone().unwrap_or_default()
        || last.has_cover != payload.cover_art_url.is_some()
        || last.is_playing != payload.is_playing
}

fn progress_jumped(last: &LastPublish, payload: &OsNowPlaying) -> bool {
    let Some(progress) = payload.progress_ms else {
        return false;
    };
    if last.progress_ms >= RESTART_FROM_MS && progress <= RESTART_TO_MS {
        return true;
    }
    let elapsed = last.at.elapsed().as_millis() as i64;
    let expected = last
        .progress_ms
        .saturating_add(if last.is_playing { elapsed } else { 0 });
    let delta = progress - expected;
    if last.is_playing && delta < 0 && delta > -SEEK_BACK_MS {
        return false;
    }
    delta.abs() >= SEEK_JUMP_MS
}

fn usable_local(payload: &OsNowPlaying) -> bool {
    payload.active && (payload.title.is_some() || payload.is_playing)
}

fn cleared_local() -> OsNowPlaying {
    OsNowPlaying {
        kind: "local".to_string(),
        source: String::new(),
        title: None,
        artist: None,
        album: None,
        cover_art_url: None,
        is_playing: false,
        progress_ms: None,
        duration_ms: None,
        active: false,
    }
}

fn store_local(app: &AppHandle, payload: Option<OsNowPlaying>) -> bool {
    let state = app.state::<crate::AppState>();
    let Ok(mut guard) = state.os_local_media.lock() else {
        return false;
    };
    let was_present = guard.as_ref().is_some_and(usable_local);
    let is_present = payload.as_ref().is_some_and(usable_local);
    *guard = payload.filter(usable_local);
    was_present != is_present
}

fn sync_media_presence(app: &AppHandle, present: bool) {
    let state = app.state::<crate::AppState>();
    let playing = state
        .os_local_media
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .is_some_and(|payload| payload.is_playing);
    let Ok(mut ids) = state.last_matched_ids.lock() else {
        return;
    };
    ids.retain(|id| id != "media");
    let spotify_present = ids.iter().any(|id| id == "spotify");
    if present
        && crate::prefs::auto_switch_enabled(app, "media")
        && (playing || !spotify_present)
    {
        ids.push("media".to_string());
    }
    ids.retain(|id| crate::prefs::auto_switch_enabled(app, id));
    let matched = ids.clone();
    drop(ids);
    crate::mode_engine::resolve(app, &matched);
}

fn publish_spotify(app: &AppHandle, payload: OsNowPlaying, last: &Mutex<Option<LastPublish>>) {
    {
        let mut guard = match last.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(previous) = guard.as_ref() {
            if !identity_changed(previous, &payload) && !progress_jumped(previous, &payload) {
                return;
            }
        }
        *guard = Some(last_from(&payload));
    }

    let state = app.state::<crate::AppState>();
    crate::spotify::invalidate_playback_cache(&state.spotify);
    crate::spotify::invalidate_queue_cache(&state.spotify);

    if let Err(err) = app.emit(EVENT_NAME, &payload) {
        log::debug!("Failed to emit {EVENT_NAME}: {err}");
        return;
    }

    log::info!(
        "OS now-playing: {} — {} ({} @ {}ms)",
        payload.title.as_deref().unwrap_or("unknown"),
        payload.artist.as_deref().unwrap_or("unknown"),
        if payload.is_playing { "playing" } else { "paused" },
        payload.progress_ms.unwrap_or(0)
    );
}

fn publish_local(app: &AppHandle, payload: OsNowPlaying, last: &Mutex<Option<LastPublish>>) {
    let should_sync;
    {
        let mut guard = match last.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(previous) = guard.as_ref() {
            let identity = identity_changed(previous, &payload);
            if !identity && !progress_jumped(previous, &payload) {
                return;
            }
            should_sync = identity;
        } else {
            should_sync = true;
        }
        *guard = Some(last_from(&payload));
    }

    let presence_changed = store_local(app, Some(payload.clone()));
    if let Err(err) = app.emit(EVENT_NAME, &payload) {
        log::debug!("Failed to emit {EVENT_NAME}: {err}");
        return;
    }

    log::info!(
        "OS local media: {} — {} from {} ({} @ {}ms)",
        payload.title.as_deref().unwrap_or("unknown"),
        payload.artist.as_deref().unwrap_or("unknown"),
        payload.source,
        if payload.is_playing { "playing" } else { "paused" },
        payload.progress_ms.unwrap_or(0)
    );

    if presence_changed || should_sync {
        sync_media_presence(app, usable_local(&payload));
    }
}

fn publish_local_cleared(app: &AppHandle, last: &Mutex<Option<LastPublish>>) {
    {
        let mut guard = match last.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if guard.is_none() {
            return;
        }
        *guard = None;
    }

    let presence_changed = store_local(app, None);
    let payload = cleared_local();
    if let Err(err) = app.emit(EVENT_NAME, &payload) {
        log::debug!("Failed to emit {EVENT_NAME}: {err}");
    }
    if presence_changed {
        sync_media_presence(app, false);
    }
}

pub fn has_local_session(app: &AppHandle) -> bool {
    app.state::<crate::AppState>()
        .os_local_media
        .lock()
        .ok()
        .map(|guard| guard.as_ref().is_some_and(usable_local))
        .unwrap_or(false)
}

pub fn current_local(state: &crate::AppState) -> Option<OsNowPlaying> {
    state.os_local_media.lock().ok().and_then(|guard| guard.clone())
}

pub fn start(app: AppHandle) {
    #[cfg(windows)]
    win::start(app);

    #[cfg(target_os = "macos")]
    macos::start(app);

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = app;
        log::info!(
            "OS now-playing listener is not available on this platform; using Spotify polling"
        );
    }
}

pub fn handle(command: &str, _state: &crate::AppState) -> Result<(), String> {
    match command {
        "togglePlay" => control_playback(ControlOp::TogglePlay),
        "nextTrack" => control_playback(ControlOp::Next),
        "prevTrack" => control_playback(ControlOp::Prev),
        _ => Err(format!("Unknown media command: {command}")),
    }
}

pub fn handle_value(
    command: &str,
    value: serde_json::Value,
    _state: &crate::AppState,
) -> Result<(), String> {
    match command {
        "setVolume" => {
            let volume = value
                .as_u64()
                .ok_or_else(|| "media.setVolume expects a numeric value".to_string())?
                .clamp(0, 100) as u8;
            set_output_volume(volume)
        }
        "seek" => {
            let position_ms = value.as_u64().ok_or_else(|| {
                "media.seek expects a numeric position in milliseconds".to_string()
            })?;
            control_playback(ControlOp::Seek(position_ms as i64))?;
            Ok(())
        }
        _ => Err(format!(
            "Media action '{}' does not support a value payload",
            command
        )),
    }
}

#[derive(Debug, Clone)]
enum ControlOp {
    TogglePlay,
    Next,
    Prev,
    Seek(i64),
}

fn control_playback(op: ControlOp) -> Result<(), String> {
    #[cfg(windows)]
    {
        return win::dispatch_control(op);
    }

    #[cfg(target_os = "macos")]
    {
        return macos::dispatch_control(op);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = op;
        Err("OS media controls are not available on this platform".to_string())
    }
}

pub fn get_output_volume() -> Result<u8, String> {
    #[cfg(windows)]
    {
        return win::get_output_volume();
    }

    #[cfg(target_os = "macos")]
    {
        return macos::get_output_volume();
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("System volume is not available on this platform".to_string())
    }
}

pub fn seek_to(position_ms: u64) -> Result<(), String> {
    control_playback(ControlOp::Seek(position_ms as i64))?;
    Ok(())
}

pub fn set_output_volume(percent: u8) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::set_output_volume(percent)?;
        log::info!("OS media: set system volume to {percent}%");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        macos::set_output_volume(percent)?;
        log::info!("OS media: set system volume to {percent}%");
        Ok(())
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = percent;
        Err("System volume is not available on this platform".to_string())
    }
}

#[cfg(windows)]
mod win {
    use super::{
        ignore_source, is_spotify_source, publish_local, publish_local_cleared, publish_spotify,
        ControlOp, OsNowPlaying,
    };
    use std::sync::mpsc::{self, SyncSender};
    use std::sync::Mutex;
    use std::time::Duration;
    use tauri::AppHandle;
    use windows::core::HSTRING;
    use windows::Foundation::TypedEventHandler;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus, MediaPropertiesChangedEventArgs,
        PlaybackInfoChangedEventArgs, SessionsChangedEventArgs, TimelinePropertiesChangedEventArgs,
    };
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    enum Cmd {
        Resync,
        Snapshot { refresh_cover: bool },
        Control {
            op: ControlOp,
            reply: mpsc::Sender<Result<(), String>>,
        },
    }

    struct Watched {
        source: String,
        session: GlobalSystemMediaTransportControlsSession,
        media_token: windows::Foundation::EventRegistrationToken,
        playback_token: windows::Foundation::EventRegistrationToken,
        timeline_token: Option<windows::Foundation::EventRegistrationToken>,
        last_cover: Option<String>,
    }

    impl Drop for Watched {
        fn drop(&mut self) {
            let _ = self.session.RemoveMediaPropertiesChanged(self.media_token);
            let _ = self
                .session
                .RemovePlaybackInfoChanged(self.playback_token);
            if let Some(token) = self.timeline_token {
                let _ = self.session.RemoveTimelinePropertiesChanged(token);
            }
        }
    }

    static CONTROL_TX: Mutex<Option<SyncSender<Cmd>>> = Mutex::new(None);

    pub fn start(app: AppHandle) {
        std::thread::Builder::new()
            .name("os-now-playing".into())
            .spawn(move || {
                if let Err(err) = run(app) {
                    log::warn!("OS now-playing listener failed: {err}");
                }
            })
            .ok();
    }

    pub fn dispatch_control(op: ControlOp) -> Result<(), String> {
        let tx = CONTROL_TX
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .ok_or_else(|| "OS media listener is not running".to_string())?;
        let (reply_tx, reply_rx) = mpsc::channel();
        tx.send(Cmd::Control {
            op,
            reply: reply_tx,
        })
        .map_err(|_| "OS media listener is not running".to_string())?;
        reply_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "OS media control timed out".to_string())?
    }

    fn run(app: AppHandle) -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
        let (tx, rx) = mpsc::sync_channel::<Cmd>(64);
        if let Ok(mut guard) = CONTROL_TX.lock() {
            *guard = Some(tx.clone());
        }
        let last_spotify = Mutex::new(None);
        let last_local = Mutex::new(None);

        let tx_sessions = tx.clone();
        manager.SessionsChanged(&TypedEventHandler::<
            GlobalSystemMediaTransportControlsSessionManager,
            SessionsChangedEventArgs,
        >::new(move |_, _| {
            let _ = tx_sessions.try_send(Cmd::Resync);
            Ok(())
        }))?;

        let tx_current = tx.clone();
        manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx_current.try_send(Cmd::Resync);
            Ok(())
        }))?;

        let mut watched: Vec<Watched> = Vec::new();
        let _ = tx.try_send(Cmd::Resync);

        loop {
            let (cmd, timed_out) = match rx.recv_timeout(Duration::from_millis(750)) {
                Ok(cmd) => (cmd, false),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    (Cmd::Snapshot { refresh_cover: false }, true)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            };
            if !timed_out {
                std::thread::sleep(Duration::from_millis(80));
            }
            let mut resync = matches!(cmd, Cmd::Resync);
            let mut refresh_cover =
                resync || matches!(cmd, Cmd::Snapshot { refresh_cover: true });
            let mut controls: Vec<(ControlOp, mpsc::Sender<Result<(), String>>)> = Vec::new();
            if let Cmd::Control { op, reply } = cmd {
                controls.push((op, reply));
            }
            while let Ok(extra) = rx.try_recv() {
                match extra {
                    Cmd::Resync => {
                        resync = true;
                        refresh_cover = true;
                    }
                    Cmd::Snapshot {
                        refresh_cover: true,
                    } => {
                        refresh_cover = true;
                    }
                    Cmd::Snapshot { .. } => {}
                    Cmd::Control { op, reply } => controls.push((op, reply)),
                }
            }
            if resync {
                resync_sessions(&manager, &tx, &mut watched);
            }
            for (op, reply) in controls {
                let _ = reply.send(apply_control(&watched, &op));
            }
            if let Some(payload) = pick_session(&mut watched, refresh_cover, true) {
                publish_spotify(&app, payload, &last_spotify);
            }
            if let Some(payload) = pick_session(&mut watched, refresh_cover, false) {
                publish_local(&app, payload, &last_local);
            } else {
                publish_local_cleared(&app, &last_local);
            }
        }

        Ok(())
    }

    fn resync_sessions(
        manager: &GlobalSystemMediaTransportControlsSessionManager,
        tx: &SyncSender<Cmd>,
        watched: &mut Vec<Watched>,
    ) {
        let Ok(sessions) = manager.GetSessions() else {
            watched.clear();
            return;
        };
        let Ok(size) = sessions.Size() else {
            return;
        };

        let mut next: Vec<Watched> = Vec::new();
        for index in 0..size {
            let Ok(session) = sessions.GetAt(index) else {
                continue;
            };
            let Ok(source) = session.SourceAppUserModelId() else {
                continue;
            };
            let source = source.to_string();
            if ignore_source(&source) {
                continue;
            }
            if let Some(existing) = watched.iter().position(|item| item.source == source) {
                next.push(watched.swap_remove(existing));
            } else if let Some(created) = watch_session(session, source, tx) {
                log::info!("Watching OS media session: {}", created.source);
                next.push(created);
            }
        }

        watched.clear();
        watched.extend(next);
    }

    fn watch_session(
        session: GlobalSystemMediaTransportControlsSession,
        source: String,
        tx: &SyncSender<Cmd>,
    ) -> Option<Watched> {
        let tx_media = tx.clone();
        let media_token = session
            .MediaPropertiesChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSession,
                MediaPropertiesChangedEventArgs,
            >::new(move |_, _| {
                let _ = tx_media.try_send(Cmd::Snapshot { refresh_cover: true });
                Ok(())
            }))
            .ok()?;
        let tx_playback = tx.clone();
        let playback_token = session
            .PlaybackInfoChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSession,
                PlaybackInfoChangedEventArgs,
            >::new(move |_, _| {
                let _ = tx_playback.try_send(Cmd::Snapshot { refresh_cover: false });
                Ok(())
            }))
            .ok()?;
        let tx_timeline = tx.clone();
        let timeline_token = session
            .TimelinePropertiesChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSession,
                TimelinePropertiesChangedEventArgs,
            >::new(move |_, _| {
                let _ = tx_timeline.try_send(Cmd::Snapshot { refresh_cover: false });
                Ok(())
            }))
            .ok();
        Some(Watched {
            source,
            session,
            media_token,
            playback_token,
            timeline_token,
            last_cover: None,
        })
    }

    fn pick_session(
        watched: &mut [Watched],
        refresh_cover: bool,
        spotify: bool,
    ) -> Option<OsNowPlaying> {
        let mut fallback = None;
        for item in watched.iter_mut() {
            if is_spotify_source(&item.source) != spotify {
                continue;
            }
            let Some(payload) = read_session(item, refresh_cover) else {
                continue;
            };
            if !spotify && payload.title.is_none() && !payload.is_playing {
                continue;
            }
            if payload.is_playing && payload.title.is_some() {
                return Some(payload);
            }
            if fallback.is_none() && (payload.title.is_some() || payload.is_playing) {
                fallback = Some(payload);
            }
        }
        fallback
    }

    fn local_session(
        watched: &[Watched],
    ) -> Option<&GlobalSystemMediaTransportControlsSession> {
        let mut fallback = None;
        for item in watched {
            if is_spotify_source(&item.source) {
                continue;
            }
            let playing = item
                .session
                .GetPlaybackInfo()
                .ok()
                .and_then(|info| info.PlaybackStatus().ok())
                == Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);
            if playing {
                return Some(&item.session);
            }
            if fallback.is_none() {
                fallback = Some(&item.session);
            }
        }
        fallback
    }

    fn apply_control(watched: &[Watched], op: &ControlOp) -> Result<(), String> {
        let session = local_session(watched)
            .ok_or_else(|| "No local media session is available".to_string())?;
        let accepted = match op {
            ControlOp::TogglePlay => session
                .TryTogglePlayPauseAsync()
                .map_err(|err| err.to_string())?
                .get()
                .map_err(|err| err.to_string())?,
            ControlOp::Next => session
                .TrySkipNextAsync()
                .map_err(|err| err.to_string())?
                .get()
                .map_err(|err| err.to_string())?,
            ControlOp::Prev => session
                .TrySkipPreviousAsync()
                .map_err(|err| err.to_string())?
                .get()
                .map_err(|err| err.to_string())?,
            ControlOp::Seek(position_ms) => {
                let ticks = position_ms.saturating_mul(10_000);
                session
                    .TryChangePlaybackPositionAsync(ticks)
                    .map_err(|err| err.to_string())?
                    .get()
                    .map_err(|err| err.to_string())?
            }
        };
        if accepted {
            Ok(())
        } else {
            Err("The current player rejected that media command".to_string())
        }
    }

    fn read_session(item: &mut Watched, refresh_cover: bool) -> Option<OsNowPlaying> {
        let session = &item.session;
        let source = session.SourceAppUserModelId().ok()?.to_string();
        let is_playing = session
            .GetPlaybackInfo()
            .ok()
            .and_then(|info| info.PlaybackStatus().ok())
            == Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);
        let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let (progress_ms, duration_ms) = read_timeline(session);
        let cover_art_url = if refresh_cover {
            let next = thumbnail_data_url(&props);
            if next.is_some() {
                item.last_cover = next.clone();
            }
            next.or_else(|| item.last_cover.clone())
        } else {
            item.last_cover.clone()
        };
        Some(OsNowPlaying {
            kind: if is_spotify_source(&source) {
                "spotify".to_string()
            } else {
                "local".to_string()
            },
            source,
            title: nonempty(props.Title().ok()),
            artist: nonempty(props.Artist().ok()),
            album: nonempty(props.AlbumTitle().ok()),
            cover_art_url,
            is_playing,
            progress_ms,
            duration_ms,
            active: true,
        })
    }

    fn timespan_ms(span: windows::Foundation::TimeSpan) -> i64 {
        span.Duration / 10_000
    }

    fn read_timeline(
        session: &GlobalSystemMediaTransportControlsSession,
    ) -> (Option<i64>, Option<i64>) {
        let Ok(timeline) = session.GetTimelineProperties() else {
            return (None, None);
        };
        let start = timeline.StartTime().ok().map(timespan_ms).unwrap_or(0);
        let progress = timeline
            .Position()
            .ok()
            .map(timespan_ms)
            .map(|position| (position - start).max(0));
        let duration = timeline
            .EndTime()
            .ok()
            .map(timespan_ms)
            .map(|end| (end - start).max(0))
            .filter(|value| *value > 0);
        (progress, duration)
    }

    fn nonempty(value: Option<HSTRING>) -> Option<String> {
        value.map(|text| text.to_string()).filter(|text| !text.is_empty())
    }

    fn thumbnail_data_url(
        props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> Option<String> {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        use windows::core::Interface;
        use windows::Storage::Streams::{DataReader, IInputStream};

        let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.get().ok()?;
        let size = stream.Size().ok()?;
        if size == 0 || size > 4_000_000 {
            return None;
        }
        let input = Interface::cast::<IInputStream>(&stream).ok()?;
        let reader = DataReader::CreateDataReader(&input).ok()?;
        let loaded = reader.LoadAsync(size as u32).ok()?.get().ok()?;
        if loaded == 0 {
            return None;
        }
        let mut bytes = vec![0u8; loaded as usize];
        reader.ReadBytes(&mut bytes).ok()?;
        let mime = stream
            .ContentType()
            .ok()
            .map(|value| value.to_string())
            .filter(|value| value.starts_with("image/"))
            .unwrap_or_else(|| "image/jpeg".to_string());
        Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
    }

    fn endpoint_volume() -> Result<IAudioEndpointVolume, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|err| {
                    format!("Failed to create audio device enumerator: {err}")
                })?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|err| format!("Failed to get default audio endpoint: {err}"))?;
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|err| format!("Failed to activate endpoint volume: {err}"))
        }
    }

    pub fn get_output_volume() -> Result<u8, String> {
        let endpoint = endpoint_volume()?;
        let level = unsafe {
            endpoint
                .GetMasterVolumeLevelScalar()
                .map_err(|err| format!("Failed to read system volume: {err}"))?
        };
        Ok((level * 100.0).round().clamp(0.0, 100.0) as u8)
    }

    pub fn set_output_volume(percent: u8) -> Result<(), String> {
        let endpoint = endpoint_volume()?;
        let level = (percent as f32 / 100.0).clamp(0.0, 1.0);
        unsafe {
            endpoint
                .SetMasterVolumeLevelScalar(level, std::ptr::null())
                .map_err(|err| format!("Failed to set system volume: {err}"))?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{
        is_spotify_source, publish_local, publish_local_cleared, publish_spotify, ControlOp,
        OsNowPlaying,
    };
    use media_remote::prelude::*;
    use std::sync::Mutex;
    use tauri::AppHandle;

    pub fn start(app: AppHandle) {
        std::thread::Builder::new()
            .name("os-now-playing".into())
            .spawn(move || {
                let now_playing = NowPlaying::new();
                let last_spotify = Mutex::new(None);
                let last_local = Mutex::new(None);
                now_playing.subscribe(move |guard| {
                    let Some(info) = guard.as_ref() else {
                        publish_local_cleared(&app, &last_local);
                        return;
                    };
                    let source = info
                        .bundle_id
                        .clone()
                        .or_else(|| info.bundle_name.clone())
                        .unwrap_or_default();
                    let payload = OsNowPlaying {
                        kind: if is_spotify_source(&source) {
                            "spotify".to_string()
                        } else {
                            "local".to_string()
                        },
                        source: source.clone(),
                        title: info.title.clone(),
                        artist: info.artist.clone(),
                        album: info.album.clone(),
                        cover_art_url: None,
                        is_playing: info.is_playing.unwrap_or(false),
                        progress_ms: info
                            .elapsed_time
                            .map(|seconds| (seconds.max(0.0) * 1000.0).round() as i64),
                        duration_ms: info
                            .duration
                            .map(|seconds| (seconds.max(0.0) * 1000.0).round() as i64)
                            .filter(|value| *value > 0),
                        active: true,
                    };
                    if payload.title.is_none() && !payload.is_playing {
                        if !is_spotify_source(&source) {
                            publish_local_cleared(&app, &last_local);
                        }
                        return;
                    }
                    if is_spotify_source(&source) {
                        publish_spotify(&app, payload, &last_spotify);
                        publish_local_cleared(&app, &last_local);
                    } else {
                        publish_local(&app, payload, &last_local);
                    }
                });
                loop {
                    std::thread::park();
                }
            })
            .ok();
    }

    pub fn dispatch_control(op: ControlOp) -> Result<(), String> {
        let now_playing = NowPlaying::new();
        let ok = match op {
            ControlOp::TogglePlay => now_playing.toggle(),
            ControlOp::Next => now_playing.next(),
            ControlOp::Prev => now_playing.previous(),
            ControlOp::Seek(_) => {
                return Err("Seek is not available for this macOS media session".to_string())
            }
        };
        if ok {
            Ok(())
        } else {
            Err("Failed to send macOS media command".to_string())
        }
    }

    pub fn get_output_volume() -> Result<u8, String> {
        let output = std::process::Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"])
            .output()
            .map_err(|err| err.to_string())?;
        if !output.status.success() {
            return Err("Failed to read macOS output volume".to_string());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        text.trim()
            .parse::<u8>()
            .map_err(|_| "Failed to parse macOS output volume".to_string())
    }

    pub fn set_output_volume(percent: u8) -> Result<(), String> {
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("set volume output volume {percent}"))
            .status()
            .map_err(|err| err.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("osascript exited with status {status}"))
        }
    }
}
