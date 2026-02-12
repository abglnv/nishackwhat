use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::models::*;
use crate::redis_store;
use crate::state::AppState;

/// POST /api/agent/heartbeat
pub async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Json(mut hb): Json<Heartbeat>,
) -> Result<Json<Value>, StatusCode> {
    hb.timestamp = Utc::now();
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    redis_store::register_agent(&mut conn, prefix, &hb.hostname, &hb.ip, hb.port)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    redis_store::store_heartbeat(&mut conn, prefix, &hb, state.config.heartbeat_ttl_secs)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "status": "ok" })))
}

/// POST /api/agent/screenshot
pub async fn screenshot(
    State(state): State<Arc<AppState>>,
    Json(mut ss): Json<Screenshot>,
) -> Result<Json<Value>, StatusCode> {
    ss.timestamp = Utc::now();
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    redis_store::store_screenshot(&mut conn, prefix, &ss)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "status": "ok" })))
}

/// POST /api/agent/notification
pub async fn notification(
    State(state): State<Arc<AppState>>,
    Json(mut n): Json<Notification>,
) -> Result<Json<Value>, StatusCode> {
    n.timestamp = Utc::now();
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    redis_store::store_notification(&mut conn, prefix, &n)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Broadcast to teacher dashboard via WS
    if let Some(teacher_tx) = state.ws_clients.get("teacher") {
        let msg = serde_json::json!({
            "type": "notification",
            "hostname": n.hostname,
            "title": n.title,
            "message": n.message,
            "level": n.level,
            "timestamp": n.timestamp.to_rfc3339()
        });
        let _ = teacher_tx.send(axum::extract::ws::Message::Text(msg.to_string().into()));
    }

    Ok(Json(json!({ "status": "ok" })))
}

/// POST /api/agent/apps
pub async fn apps(
    State(state): State<Arc<AppState>>,
    Json(mut app_list): Json<AppList>,
) -> Result<Json<Value>, StatusCode> {
    app_list.timestamp = Utc::now();
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    redis_store::store_apps(&mut conn, prefix, &app_list)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "status": "ok" })))
}

/// POST /api/agent/violation
pub async fn violation(
    State(state): State<Arc<AppState>>,
    Json(mut v): Json<Violation>,
) -> Result<Json<Value>, StatusCode> {
    v.timestamp = Utc::now();
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    redis_store::add_violation(&mut conn, prefix, &v)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Notify teacher via WS
    if let Some(teacher_tx) = state.ws_clients.get("teacher") {
        let msg = serde_json::json!({
            "type": "violation",
            "hostname": v.hostname,
            "rule": v.rule,
            "detail": v.detail,
            "severity": v.severity,
            "timestamp": v.timestamp.to_rfc3339()
        });
        let _ = teacher_tx.send(axum::extract::ws::Message::Text(msg.to_string().into()));
    }

    Ok(Json(json!({ "status": "ok" })))
}
