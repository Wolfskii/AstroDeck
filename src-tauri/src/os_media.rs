use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

const EVENT_NAME: &str = "os-now-playing";
const SEEK_JUMP_MS: i64 = 1800;
const RESTART_FROM_MS: i64 = 2500;
const RESTART_TO_MS: i64 = 1500;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OsNowPlaying {
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
    (progress - expected).abs() >= SEEK_JUMP_MS
}

fn publish(app: &AppHandle, payload: OsNowPlaying, last: &Mutex<Option<LastPublish>>) {
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

#[cfg(windows)]
mod win {
    use super::{is_spotify_source, publish, OsNowPlaying};
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
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    enum Cmd {
        Resync,
        Snapshot { refresh_cover: bool },
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

    fn run(app: AppHandle) -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
        let (tx, rx) = mpsc::sync_channel::<Cmd>(64);
        let last = Mutex::new(None);

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
                }
            }
            if resync {
                resync_sessions(&manager, &tx, &mut watched);
            }
            if watched.is_empty() {
                continue;
            }
            if let Some(payload) = snapshot_spotify(&mut watched, refresh_cover) {
                publish(&app, payload, &last);
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
            if !is_spotify_source(&source) {
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

    fn snapshot_spotify(watched: &mut [Watched], refresh_cover: bool) -> Option<OsNowPlaying> {
        let mut fallback = None;
        for item in watched.iter_mut() {
            let Some(payload) = read_session(item, refresh_cover) else {
                continue;
            };
            if payload.is_playing && payload.title.is_some() {
                return Some(payload);
            }
            if fallback.is_none() && payload.title.is_some() {
                fallback = Some(payload);
            }
        }
        fallback
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
            source,
            title: nonempty(props.Title().ok()),
            artist: nonempty(props.Artist().ok()),
            album: nonempty(props.AlbumTitle().ok()),
            cover_art_url,
            is_playing,
            progress_ms,
            duration_ms,
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
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{is_spotify_source, publish, OsNowPlaying};
    use media_remote::prelude::*;
    use std::sync::Mutex;
    use tauri::AppHandle;

    pub fn start(app: AppHandle) {
        std::thread::Builder::new()
            .name("os-now-playing".into())
            .spawn(move || {
                let now_playing = NowPlaying::new();
                let last = Mutex::new(None);
                now_playing.subscribe(move |guard| {
                    let Some(info) = guard.as_ref() else {
                        return;
                    };
                    let source = info
                        .bundle_id
                        .clone()
                        .or_else(|| info.bundle_name.clone())
                        .unwrap_or_default();
                    if !is_spotify_source(&source) {
                        return;
                    }
                    let payload = OsNowPlaying {
                        source,
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
                    };
                    if payload.title.is_none() {
                        return;
                    }
                    publish(&app, payload, &last);
                });
                loop {
                    std::thread::park();
                }
            })
            .ok();
    }
}
