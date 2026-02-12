use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Forward channel → WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut client_id = String::new();

    while let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            Message::Text(ref t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let parsed: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match parsed["type"].as_str() {
            Some("identify") => {
                client_id = parsed["id"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                state.ws_clients.insert(client_id.clone(), tx.clone());

                let sys_msg = serde_json::json!({
                    "type": "system",
                    "content": format!("{} joined the chat", client_id),
                    "timestamp": Utc::now().to_rfc3339()
                });
                broadcast(&state, &sys_msg.to_string(), &client_id);
                tracing::info!("WS client identified: {client_id}");
            }

            Some("chat") => {
                let to = parsed["to"].as_str().unwrap_or("all");
                let content = parsed["content"].as_str().unwrap_or("");

                let out = serde_json::json!({
                    "type": "chat",
                    "from": client_id,
                    "to": to,
                    "content": content,
                    "timestamp": Utc::now().to_rfc3339()
                });
                let out_str = out.to_string();

                if to == "all" {
                    broadcast(&state, &out_str, "");
                } else {
                    // Send to target
                    if let Some(target_tx) = state.ws_clients.get(to) {
                        let _ = target_tx.send(Message::Text(out_str.clone().into()));
                    }
                    // Echo back to sender
                    let _ = tx.send(Message::Text(out_str.into()));
                }
            }

            _ => {}
        }
    }

    // Cleanup
    if !client_id.is_empty() {
        state.ws_clients.remove(&client_id);
        let sys_msg = serde_json::json!({
            "type": "system",
            "content": format!("{} left the chat", client_id),
            "timestamp": Utc::now().to_rfc3339()
        });
        broadcast(&state, &sys_msg.to_string(), &client_id);
        tracing::info!("WS client disconnected: {client_id}");
    }

    send_task.abort();
}

fn broadcast(state: &AppState, msg: &str, exclude: &str) {
    for entry in state.ws_clients.iter() {
        if entry.key() != exclude {
            let _ = entry.value().send(Message::Text(msg.to_string().into()));
        }
    }
}
