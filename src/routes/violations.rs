use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::redis_store;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ViolationQuery {
    pub hostname: Option<String>,
    pub count: Option<isize>,
}

pub async fn violations(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ViolationQuery>,
) -> Result<Json<Value>, StatusCode> {
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;
    let count = q.count.unwrap_or(50);

    if let Some(hostname) = &q.hostname {
        let viols = redis_store::get_violations(&mut conn, prefix, hostname, count)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let total = redis_store::get_violation_count(&mut conn, prefix, hostname)
            .await
            .unwrap_or(0);
        Ok(Json(serde_json::json!({
            "hostname": hostname,
            "total": total,
            "violations": viols
        })))
    } else {
        // Return violations for all known agents
        let agents = redis_store::get_all_agents(&mut conn, prefix)
            .await
            .unwrap_or_default();

        let mut all = Vec::new();
        for entry in &agents {
            let hostname = entry.split('|').next().unwrap_or(entry);
            let viols = redis_store::get_violations(&mut conn, prefix, hostname, count)
                .await
                .unwrap_or_default();
            for v in viols {
                all.push(v);
            }
        }
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all.truncate(count as usize);

        Ok(Json(serde_json::json!({ "violations": all })))
    }
}
