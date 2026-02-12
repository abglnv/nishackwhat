use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::redis_store;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LockRequest {
    /// "soft" (minimize all) or "hard" (lock workstation)
    pub mode: String,
}

/// POST /api/students/:hostname/lock
/// Body: { "mode": "soft" } or { "mode": "hard" }
/// Forwards the lock command to the student agent's HTTP API.
pub async fn lock_student(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
    Json(body): Json<LockRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Validate mode
    if body.mode != "soft" && body.mode != "hard" {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "error": "Invalid mode. Use 'soft' or 'hard'."
        })));
    }

    // Find student's IP:port from Redis agent registry
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    let agents = redis_store::get_all_agents(&mut conn, prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entry = agents
        .iter()
        .find(|e| e.starts_with(&format!("{hostname}|")))
        .ok_or(StatusCode::NOT_FOUND)?;

    let parts: Vec<&str> = entry.split('|').collect();
    if parts.len() < 3 {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let ip = parts[1];
    let port = parts[2];

    // Forward to student agent
    let url = format!("http://{ip}:{port}/lock/{}", body.mode);
    tracing::info!("🔒 Sending {} lock to {hostname} at {url}", body.mode);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match client.post(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().await.unwrap_or(serde_json::json!({"status": "ok"}));
            tracing::info!("✅ Lock command accepted by {hostname}");
            Ok(Json(body))
        }
        Ok(resp) => {
            let status = resp.status();
            tracing::warn!("Student {hostname} returned {status}");
            Ok(Json(serde_json::json!({
                "status": "error",
                "error": format!("Student returned {status}")
            })))
        }
        Err(e) => {
            tracing::warn!("Failed to reach student {hostname}: {e}");
            Ok(Json(serde_json::json!({
                "status": "error",
                "error": format!("Cannot reach student: {e}")
            })))
        }
    }
}
