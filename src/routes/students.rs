use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;
use std::sync::Arc;

use crate::models::{StudentDetail, StudentSummary};
use crate::redis_store;
use crate::state::AppState;

/// Build a StudentSummary from an agent registry entry like "hostname|ip|port"
async fn build_summary(
    conn: &mut redis::aio::ConnectionManager,
    prefix: &str,
    entry: &str,
    heartbeat_ttl: u64,
) -> Option<StudentSummary> {
    let parts: Vec<&str> = entry.split('|').collect();
    if parts.len() < 3 {
        return None;
    }
    let hostname = parts[0];
    let ip = parts[1];
    let port: u16 = parts[2].parse().unwrap_or(0);

    let hb = redis_store::get_heartbeat(conn, prefix, hostname).await.ok()?;
    let violation_count = redis_store::get_violation_count(conn, prefix, hostname)
        .await
        .unwrap_or(0);

    let (active, os, username, cpu_usage, ram_usage, last_seen) = match hb {
        Some(h) => {
            let age = chrono::Utc::now()
                .signed_duration_since(h.timestamp)
                .num_seconds();
            (
                age < heartbeat_ttl as i64,
                h.os,
                h.username,
                h.cpu_usage,
                h.ram_usage,
                Some(h.timestamp),
            )
        }
        None => (false, String::new(), String::new(), 0.0, 0.0, None),
    };

    Some(StudentSummary {
        hostname: hostname.to_string(),
        ip: ip.to_string(),
        port,
        active,
        os,
        username,
        cpu_usage,
        ram_usage,
        violation_count,
        last_seen,
    })
}

/// GET /api/students — all registered students
pub async fn list_students(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    let agents = redis_store::get_all_agents(&mut conn, prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut students = Vec::new();
    for entry in &agents {
        if let Some(s) =
            build_summary(&mut conn, prefix, entry, state.config.heartbeat_ttl_secs).await
        {
            students.push(s);
        }
    }
    students.sort_by(|a, b| a.hostname.cmp(&b.hostname));

    Ok(Json(serde_json::json!({
        "count": students.len(),
        "students": students
    })))
}

/// GET /api/students/active — only students with live heartbeat
pub async fn list_active(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    let agents = redis_store::get_all_agents(&mut conn, prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut active = Vec::new();
    for entry in &agents {
        if let Some(s) =
            build_summary(&mut conn, prefix, entry, state.config.heartbeat_ttl_secs).await
        {
            if s.active {
                active.push(s);
            }
        }
    }
    active.sort_by(|a, b| a.hostname.cmp(&b.hostname));

    Ok(Json(serde_json::json!({
        "count": active.len(),
        "students": active
    })))
}

/// GET /api/students/:hostname — full details for one student
pub async fn student_detail(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<Json<StudentDetail>, StatusCode> {
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;

    // Look up agent entry
    let agents = redis_store::get_all_agents(&mut conn, prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entry = agents
        .iter()
        .find(|e| e.starts_with(&format!("{hostname}|")))
        .ok_or(StatusCode::NOT_FOUND)?;

    let summary =
        build_summary(&mut conn, prefix, entry, state.config.heartbeat_ttl_secs)
            .await
            .ok_or(StatusCode::NOT_FOUND)?;

    let screenshot = redis_store::get_screenshot(&mut conn, prefix, &hostname)
        .await
        .unwrap_or(None);
    let apps = redis_store::get_apps(&mut conn, prefix, &hostname)
        .await
        .unwrap_or(None);
    let notifications =
        redis_store::get_notifications(&mut conn, prefix, &hostname, 50)
            .await
            .unwrap_or_default();
    let violations = redis_store::get_violations(&mut conn, prefix, &hostname, 50)
        .await
        .unwrap_or_default();

    Ok(Json(StudentDetail {
        summary,
        screenshot,
        apps,
        notifications,
        violations,
    }))
}
