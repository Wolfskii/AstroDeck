use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicI32, Ordering},
};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const TEAMS_URL: &str = "ws://127.0.0.1:8124";
const PROTOCOL_VERSION: &str = "2.0.0";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamsStatus {
    pub is_connected: bool,
    pub is_in_meeting: bool,
    pub is_muted: bool,
    pub is_video_on: bool,
    pub is_hand_raised: bool,
    pub is_recording_on: bool,
    pub is_background_blurred: bool,
    pub is_sharing: bool,
    pub has_unread_messages: bool,
    pub can_toggle_mute: bool,
    pub can_toggle_video: bool,
    pub can_toggle_hand: bool,
    pub can_toggle_blur: bool,
    pub can_leave: bool,
    pub can_react: bool,
    pub can_toggle_share_tray: bool,
    pub can_toggle_chat: bool,
    pub can_stop_sharing: bool,
    pub can_pair: bool,
    pub message: String,
}

impl Default for TeamsStatus {
    fn default() -> Self {
        Self {
            is_connected: false,
            is_in_meeting: false,
            is_muted: false,
            is_video_on: false,
            is_hand_raised: false,
            is_recording_on: false,
            is_background_blurred: false,
            is_sharing: false,
            has_unread_messages: false,
            can_toggle_mute: false,
            can_toggle_video: false,
            can_toggle_hand: false,
            can_toggle_blur: false,
            can_leave: false,
            can_react: false,
            can_toggle_share_tray: false,
            can_toggle_chat: false,
            can_stop_sharing: false,
            can_pair: false,
            message: "Microsoft Teams is not connected.".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct TeamsState {
    status: Arc<Mutex<TeamsStatus>>,
    command_tx: Arc<Mutex<Option<UnboundedSender<TeamsCommand>>>>,
    request_id: Arc<AtomicI32>,
    token_path: Arc<Mutex<Option<PathBuf>>>,
}

enum TeamsCommand {
    Send {
        action: String,
        parameter: Option<String>,
        request_id: i32,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientMessage {
    action: String,
    parameters: ClientParameters,
    request_id: i32,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ClientParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerMessage {
    #[serde(default)]
    token_refresh: Option<String>,
    #[serde(default)]
    error_msg: Option<String>,
    #[serde(default)]
    meeting_update: Option<MeetingUpdate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeetingUpdate {
    meeting_state: MeetingState,
    meeting_permissions: MeetingPermissions,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MeetingState {
    is_muted: bool,
    is_hand_raised: bool,
    is_in_meeting: bool,
    is_recording_on: bool,
    is_background_blurred: bool,
    is_sharing: bool,
    has_unread_messages: bool,
    is_video_on: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MeetingPermissions {
    can_toggle_mute: bool,
    can_toggle_video: bool,
    can_toggle_hand: bool,
    can_toggle_blur: bool,
    can_leave: bool,
    can_react: bool,
    can_toggle_share_tray: bool,
    can_toggle_chat: bool,
    can_stop_sharing: bool,
    can_pair: bool,
}

impl TeamsState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(TeamsStatus::default())),
            command_tx: Arc::new(Mutex::new(None)),
            request_id: Arc::new(AtomicI32::new(0)),
            token_path: Arc::new(Mutex::new(None)),
        }
    }
}

pub fn init(app: &tauri::AppHandle, state: &TeamsState) {
    if let Ok(base) = app.path().app_local_data_dir() {
        let _ = std::fs::create_dir_all(&base);
        let path = base.join("teams_token.json");
        if let Ok(mut token_path) = state.token_path.lock() {
            *token_path = Some(path.clone());
        }
        let token = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<String>(&text).ok())
            .unwrap_or_default();
        let app_handle = app.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            connection_loop(app_handle, state_clone, token).await;
        });
    }
}

pub fn status(state: &TeamsState) -> TeamsStatus {
    state.status.lock().map(|status| status.clone()).unwrap_or_default()
}

pub fn command(state: &TeamsState, command: &str) -> Result<(), String> {
    let (action, parameter) = match command {
        "toggleMute" => ("toggle-mute", None),
        "toggleCamera" => ("toggle-video", None),
        "raiseHand" => ("toggle-hand", None),
        "shareScreen" => ("toggle-sharing", None),
        "stopSharing" => ("stop-sharing", None),
        "leaveMeeting" => ("leave-call", None),
        "chat" => ("toggle-ui", Some("chat")),
        "shareTray" => ("toggle-ui", Some("share-tray")),
        "reaction.like" => ("send-reaction", Some("like")),
        "reaction.heart" => ("send-reaction", Some("love")),
        "reaction.clap" => ("send-reaction", Some("applause")),
        "reaction.laugh" => ("send-reaction", Some("laugh")),
        "reaction.wow" => ("send-reaction", Some("wow")),
        "goCalendar" | "goActivity" => {
            return Err("Teams third-party API does not control global navigation.".to_string());
        }
        other => return Err(format!("Unknown Teams command: {other}")),
    };
    let status = status(state);
    if !status.is_connected {
        return Err("Microsoft Teams third-party API is not connected.".to_string());
    }
    if !status.is_in_meeting
        && matches!(
            command,
            "toggleMute"
                | "toggleCamera"
                | "raiseHand"
                | "shareScreen"
                | "stopSharing"
                | "leaveMeeting"
                | "chat"
                | "shareTray"
                | "reaction.like"
                | "reaction.heart"
                | "reaction.clap"
                | "reaction.laugh"
                | "reaction.wow"
        )
    {
        return Err("Microsoft Teams is not currently in a meeting.".to_string());
    }
    let request_id = state.request_id.fetch_add(1, Ordering::Relaxed) + 1;
    let sender = state
        .command_tx
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Microsoft Teams command channel is not ready.".to_string())?;
    sender
        .send(TeamsCommand::Send {
            action: action.to_string(),
            parameter: parameter.map(str::to_string),
            request_id,
        })
        .map_err(|_| "Microsoft Teams connection is unavailable.".to_string())
}

async fn connection_loop(app: tauri::AppHandle, state: TeamsState, mut token: String) {
    let mut reconnect_delay = Duration::from_secs(2);
    loop {
        let url = format!(
            "{TEAMS_URL}?token={}&protocol-version={PROTOCOL_VERSION}&manufacturer=AstroDeck&device=AstroDeck&app=AstroDeck&app-version={}",
            urlencoding::encode(&token),
            env!("CARGO_PKG_VERSION")
        );
        match connect_async(&url).await {
            Ok((socket, _)) => {
                reconnect_delay = Duration::from_secs(2);
                mark_connected(&app, &state, true, "Connected to Microsoft Teams.");
                let (mut writer, mut reader) = socket.split();
                let (command_tx, mut command_rx) = unbounded_channel::<TeamsCommand>();
                if let Ok(mut sender) = state.command_tx.lock() {
                    *sender = Some(command_tx);
                }
                let query = ClientMessage {
                    action: "query-state".to_string(),
                    parameters: ClientParameters::default(),
                    request_id: state.request_id.fetch_add(1, Ordering::Relaxed) + 1,
                };
                let _ = writer.send(Message::Text(
                    serde_json::to_string(&query).unwrap_or_default().into(),
                )).await;
                loop {
                    tokio::select! {
                        Some(command) = command_rx.recv() => {
                            let TeamsCommand::Send { action, parameter, request_id } = command;
                            let message = ClientMessage {
                                action,
                                parameters: ClientParameters { r#type: parameter },
                                request_id,
                            };
                            let payload = serde_json::to_string(&message).unwrap_or_default();
                            if writer.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                        incoming = reader.next() => {
                            match incoming {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(message) = serde_json::from_str::<ServerMessage>(&text) {
                                        if let Some(next_token) = message.token_refresh {
                                            token = next_token.clone();
                                            persist_token(&state, &next_token);
                                        }
                                        if let Some(error) = message.error_msg {
                                            publish_error(&app, &state, error);
                                        }
                                        if let Some(update) = message.meeting_update {
                                            apply_update(&app, &state, update);
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                                _ => {}
                            }
                        }
                    }
                }
                if let Ok(mut sender) = state.command_tx.lock() {
                    *sender = None;
                }
                mark_connected(&app, &state, false, "Microsoft Teams connection closed.");
            }
            Err(_) => {
                mark_connected(&app, &state, false, "Microsoft Teams third-party API is unavailable. Enable it in Teams privacy settings.");
            }
        }
        tokio::time::sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(30));
    }
}

fn apply_update(app: &tauri::AppHandle, state: &TeamsState, update: MeetingUpdate) {
    if let Ok(mut status) = state.status.lock() {
        status.is_in_meeting = update.meeting_state.is_in_meeting;
        status.is_muted = update.meeting_state.is_muted;
        status.is_video_on = update.meeting_state.is_video_on;
        status.is_hand_raised = update.meeting_state.is_hand_raised;
        status.is_recording_on = update.meeting_state.is_recording_on;
        status.is_background_blurred = update.meeting_state.is_background_blurred;
        status.is_sharing = update.meeting_state.is_sharing;
        status.has_unread_messages = update.meeting_state.has_unread_messages;
        status.can_toggle_mute = update.meeting_permissions.can_toggle_mute;
        status.can_toggle_video = update.meeting_permissions.can_toggle_video;
        status.can_toggle_hand = update.meeting_permissions.can_toggle_hand;
        status.can_toggle_blur = update.meeting_permissions.can_toggle_blur;
        status.can_leave = update.meeting_permissions.can_leave;
        status.can_react = update.meeting_permissions.can_react;
        status.can_toggle_share_tray = update.meeting_permissions.can_toggle_share_tray;
        status.can_toggle_chat = update.meeting_permissions.can_toggle_chat;
        status.can_stop_sharing = update.meeting_permissions.can_stop_sharing;
        status.can_pair = update.meeting_permissions.can_pair;
        status.message = "Microsoft Teams state synchronized.".to_string();
    }
    publish(app, state);
}

fn mark_connected(app: &tauri::AppHandle, state: &TeamsState, connected: bool, message: &str) {
    if let Ok(mut status) = state.status.lock() {
        status.is_connected = connected;
        status.message = message.to_string();
    }
    publish(app, state);
}

fn publish(app: &tauri::AppHandle, state: &TeamsState) {
    let _ = app.emit("teams-status", status(state));
}

fn publish_error(app: &tauri::AppHandle, state: &TeamsState, error: String) {
    if let Ok(mut status) = state.status.lock() {
        status.message = error;
    }
    publish(app, state);
}

fn persist_token(state: &TeamsState, token: &str) {
    if let Ok(path) = state.token_path.lock() {
        if let Some(path) = path.as_ref() {
            if let Ok(contents) = serde_json::to_string(token) {
                let _ = std::fs::write(path, contents);
            }
        }
    }
}
