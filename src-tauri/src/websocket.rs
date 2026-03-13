/// WebSocket log bus -- broadcasts JSON-encoded LogEntry from the Tauri window
/// to any browser client connected to ws://127.0.0.1:WEBSOCKET_PORT.
///
/// Also accepts simple control commands from the browser settings panel.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};

type WsSender = futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "setScene")]
    SetScene { #[serde(rename = "sceneId")] scene_id: String },
    #[serde(rename = "spotifyStatus")]
    SpotifyStatus,
    #[serde(rename = "spotifyAuthStart")]
    SpotifyAuthStart,
    #[serde(rename = "spotifyDisconnect")]
    SpotifyDisconnect,
}

#[derive(Debug, Serialize)]
struct ServerEnvelope<T: Serialize> {
    #[serde(rename = "type")]
    kind: &'static str,
    payload: T,
}

pub async fn start_log_bus(port: u16, app_handle: tauri::AppHandle, tx: broadcast::Sender<String>) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            log::info!("WebSocket log bus listening on ws://{}", addr);
            l
        }
        Err(e) => {
            log::error!("Failed to bind WebSocket log bus on {}: {}", addr, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                log::debug!("Log bus: client connected from {}", peer);
                let rx = tx.subscribe();
                let app = app_handle.clone();
                tokio::spawn(handle_client(stream, app, rx));
            }
            Err(e) => log::error!("Log bus accept error: {}", e),
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    app_handle: tauri::AppHandle,
    mut rx: broadcast::Receiver<String>,
) {
    let ws = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::warn!("Log bus: WS handshake failed: {}", e);
            return;
        }
    };

    let (mut sender, mut receiver) = ws.split();

    loop {
        tokio::select! {
            broadcast_msg = rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        if sender.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("Log bus: receiver lagged {} messages", n);
                    }
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::SetScene { scene_id }) => {
                                let state = app_handle.state::<crate::AppState>();
                                if let Err(e) = crate::apply_scene_change(&app_handle, &state, scene_id.clone(), true) {
                                    log::error!("Log bus: failed to set scene to {}: {}", scene_id, e);
                                } else {
                                    log::info!("Log bus: browser requested scene override -> {}", scene_id);
                                }
                            }
                            Ok(ClientMessage::SpotifyStatus) => {
                                let app = app_handle.clone();
                                let status_result = tokio::task::spawn_blocking(move || {
                                    let state = app.state::<crate::AppState>();
                                    crate::spotify::get_status(&state.spotify)
                                })
                                .await
                                .map_err(|e| e.to_string());

                                match status_result {
                                    Ok(Ok(status)) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyStatus",
                                            payload: status,
                                        }).await;
                                    }
                                    Ok(Err(e)) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyAuthError",
                                            payload: serde_json::json!({ "message": e }),
                                        }).await;
                                    }
                                    Err(e) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyAuthError",
                                            payload: serde_json::json!({ "message": e }),
                                        }).await;
                                    }
                                }
                            }
                            Ok(ClientMessage::SpotifyAuthStart) => {
                                let state = app_handle.state::<crate::AppState>();
                                match crate::launch_spotify_auth(&app_handle, &state) {
                                    Ok(status) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyAuthUrl",
                                            payload: serde_json::json!({ "url": status }),
                                        }).await;
                                    }
                                    Err(e) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyAuthError",
                                            payload: serde_json::json!({ "message": e }),
                                        }).await;
                                    }
                                }
                            }
                            Ok(ClientMessage::SpotifyDisconnect) => {
                                let app = app_handle.clone();
                                let disconnect_result = tokio::task::spawn_blocking(move || {
                                    let state = app.state::<crate::AppState>();
                                    crate::spotify::disconnect(&state.spotify)?;
                                    crate::spotify::get_status(&state.spotify)
                                })
                                .await
                                .map_err(|e| e.to_string());

                                match disconnect_result {
                                    Ok(Ok(status)) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyStatus",
                                            payload: status,
                                        }).await;
                                    }
                                    Ok(Err(e)) | Err(e) => {
                                        let _ = send_json(&mut sender, &ServerEnvelope {
                                            kind: "spotifyAuthError",
                                            payload: serde_json::json!({ "message": e }),
                                        }).await;
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("Log bus: invalid client message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        log::warn!("Log bus: client read error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    log::debug!("Log bus: client disconnected");
}

async fn send_json<T: Serialize>(sender: &mut WsSender, payload: &T) -> Result<(), String> {
    let text = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    sender
        .send(Message::Text(text.into()))
        .await
        .map_err(|e| e.to_string())
}
