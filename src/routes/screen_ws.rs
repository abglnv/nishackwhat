// ─────────────────────────────────────────────────────────────────
//  screen_ws.rs — Live screen relay WebSocket endpoints
//
//  /ws/screen        — Student agents connect here, send JPEG frames
//  /ws/screen/view   — Teacher dashboard connects here, receives frames
// ─────────────────────────────────────────────────────────────────

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::state::AppState;

// ── Student agent endpoint: /ws/screen ──────────────────────────

pub async fn ws_screen_student(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_student_screen(socket, state))
}

async fn handle_student_screen(socket: WebSocket, state: Arc<AppState>) {
    let (_ws_tx, mut ws_rx) = socket.split();
    let mut hostname = String::new();

    // Step 1: Wait for JSON handshake {"role":"student","hostname":"..."}
    if let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            _ => {
                tracing::warn!("Screen WS: expected text handshake, got binary");
                return;
            }
        };

        let parsed: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!("Screen WS: invalid JSON handshake");
                return;
            }
        };

        let role = parsed["role"].as_str().unwrap_or("");
        if role != "student" {
            tracing::warn!("Screen WS: unexpected role '{role}'");
            return;
        }

        hostname = parsed["hostname"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
    } else {
        tracing::warn!("Screen WS: student disconnected before handshake");
        return;
    }

    tracing::info!("🖥️  Screen stream connected: {hostname}");

    // Notify all teacher dashboards that a new student appeared
    {
        let event = build_tagged_event("student_connected", &hostname);
        broadcast_to_screen_teachers(&state, &event).await;
    }

    // Step 2: Receive binary JPEG frames and relay to teachers
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Binary(data) => {
                let data: Vec<u8> = data.into();
                // Cache the latest frame
                state.screen_latest.insert(hostname.clone(), data.clone());

                // Build tagged frame: [1 byte len][hostname bytes][JPEG bytes]
                let tagged = build_tagged_frame(&hostname, &data);

                // Relay to all connected teacher dashboards
                broadcast_to_screen_teachers(&state, &tagged).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup
    tracing::info!("🔌 Screen stream disconnected: {hostname}");
    state.screen_latest.remove(&hostname);

    let event = build_tagged_event("student_disconnected", &hostname);
    broadcast_to_screen_teachers(&state, &event).await;
}

// ── Teacher dashboard endpoint: /ws/screen/view ─────────────────

pub async fn ws_screen_teacher(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_teacher_screen(socket, state))
}

async fn handle_teacher_screen(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Create a channel for sending frames to this teacher
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    // Register this teacher
    {
        let mut teachers = state.screen_teachers.write().await;
        teachers.push(tx.clone());
    }
    tracing::info!("👁️  Screen viewer connected (teachers: {})",
        state.screen_teachers.read().await.len());

    // Send the current student list as a JSON text message
    {
        let student_hostnames: Vec<String> = state
            .screen_latest
            .iter()
            .map(|e| e.key().clone())
            .collect();

        let list_msg = serde_json::json!({
            "type": "student_list",
            "students": student_hostnames,
        });

        if let Err(e) = ws_tx.send(Message::Text(list_msg.to_string().into())).await {
            tracing::warn!("Failed to send student list to teacher: {e}");
        }
    }

    // Send the latest cached frame for each student
    for entry in state.screen_latest.iter() {
        let tagged = build_tagged_frame(entry.key(), entry.value());
        if ws_tx.send(Message::Binary(tagged.into())).await.is_err() {
            break;
        }
    }

    // Spawn a task that forwards channel messages to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if ws_tx.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });

    // Keep alive — read from the teacher socket to detect close
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Close(_) => break,
            _ => {} // Ignore any incoming messages
        }
    }

    // Cleanup — remove this teacher's sender
    {
        let mut teachers = state.screen_teachers.write().await;
        teachers.retain(|t| !t.is_closed());
    }
    tracing::info!("👁️  Screen viewer disconnected (teachers: {})",
        state.screen_teachers.read().await.len());

    send_task.abort();
}

// ── Helpers ─────────────────────────────────────────────────────

/// Build a tagged binary frame: [1 byte hostname_len][hostname bytes][payload]
fn build_tagged_frame(hostname: &str, jpeg_bytes: &[u8]) -> Vec<u8> {
    let hn_bytes = hostname.as_bytes();
    let mut frame = Vec::with_capacity(1 + hn_bytes.len() + jpeg_bytes.len());
    frame.push(hn_bytes.len() as u8);
    frame.extend_from_slice(hn_bytes);
    frame.extend_from_slice(jpeg_bytes);
    frame
}

/// Build a JSON event as binary-tagged data (with a 0-byte JPEG = signal)
/// Actually, send as TEXT so the JS client can distinguish events from frames.
fn build_tagged_event(event_type: &str, hostname: &str) -> Vec<u8> {
    // We'll use a special convention: if the "frame" starts with '{' it's JSON
    let json = serde_json::json!({
        "type": event_type,
        "hostname": hostname,
    });
    // Encode as tagged text: prefix with hostname len + hostname, then JSON
    let json_bytes = json.to_string().into_bytes();
    let hn_bytes = hostname.as_bytes();
    let mut frame = Vec::with_capacity(1 + hn_bytes.len() + json_bytes.len());
    frame.push(0u8); // 0-length hostname = signal that this is a JSON event
    frame.extend_from_slice(&json_bytes);
    frame
}

/// Send data to all connected teacher screen viewers.
async fn broadcast_to_screen_teachers(state: &AppState, data: &[u8]) {
    let teachers = state.screen_teachers.read().await;
    for tx in teachers.iter() {
        let _ = tx.send(data.to_vec());
    }
}
