use axum::extract::State;
use axum::Json;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::state::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let uptime = Utc::now()
        .signed_duration_since(state.start_time)
        .num_seconds();

    Json(json!({
        "status": "ok",
        "uptime_secs": uptime,
        "timestamp": Utc::now().to_rfc3339()
    }))
}
