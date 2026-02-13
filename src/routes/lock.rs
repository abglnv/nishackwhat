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

#[derive(Deserialize)]
pub struct OpenUrlRequest {
    pub url: String,
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

/// GET /api/apps/:hostname
/// Proxies to student's /apps endpoint to get running processes.
pub async fn get_apps_student(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<Json<Value>, StatusCode> {
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

    let url = format!("http://{ip}:{port}/apps");
    tracing::info!("📋 Fetching apps from {hostname} at {url}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().await.unwrap_or(serde_json::json!({
                "hostname": hostname,
                "applications": [],
                "browser_tabs": [],
            }));
            Ok(Json(body))
        }
        Ok(resp) => {
            let status = resp.status();
            tracing::warn!("Student {hostname} returned {status}");
            Ok(Json(serde_json::json!({
                "hostname": hostname,
                "applications": [],
                "browser_tabs": [],
                "error": format!("Student returned {status}")
            })))
        }
        Err(e) => {
            tracing::warn!("Failed to reach student {hostname}: {e}");
            Ok(Json(serde_json::json!({
                "hostname": hostname,
                "applications": [],
                "browser_tabs": [],
                "error": format!("Cannot reach student: {e}")
            })))
        }
    }
}

/// POST /api/broadcast/open-url
/// Body: { "url": "https://kahoot.it/..." }
/// Opens a URL on ALL active student PCs.
pub async fn broadcast_open_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<OpenUrlRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !body.url.starts_with("http://") && !body.url.starts_with("https://") {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "error": "URL must start with http:// or https://"
        })));
    }

    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    let agents = redis_store::get_all_agents(&mut conn, prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut success = 0usize;
    let mut failed = 0usize;
    let total = agents.len();

    for entry in &agents {
        let parts: Vec<&str> = entry.split('|').collect();
        if parts.len() < 3 { failed += 1; continue; }
        let ip = parts[1];
        let port = parts[2];
        let hostname = parts[0];

        let url = format!("http://{ip}:{port}/open-url");
        match client
            .post(&url)
            .json(&serde_json::json!({ "url": body.url }))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("✅ URL opened on {hostname}");
                success += 1;
            }
            _ => {
                tracing::warn!("❌ Failed to open URL on {hostname}");
                failed += 1;
            }
        }
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "total": total,
        "success": success,
        "failed": failed,
    })))
}

/// POST /api/students/:hostname/open-url
/// Body: { "url": "https://kahoot.it/..." }
/// Opens a URL in the student's default browser.
pub async fn open_url_student(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
    Json(body): Json<OpenUrlRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !body.url.starts_with("http://") && !body.url.starts_with("https://") {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "error": "URL must start with http:// or https://"
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

    let url = format!("http://{ip}:{port}/open-url");
    tracing::info!("🌐 Opening URL on {hostname}: {}", body.url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match client
        .post(&url)
        .json(&serde_json::json!({ "url": body.url }))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().await.unwrap_or(serde_json::json!({"status": "ok"}));
            tracing::info!("✅ URL opened on {hostname}");
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
