use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::config::Config;
use crate::state::AppState;

pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<Config> {
    Json(state.config.clone())
}

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub banned_sites: Option<Vec<String>>,
    pub banned_apps: Option<Vec<String>>,
    pub sau_mode: Option<bool>,
}

/// PUT /api/config — update banned lists and persist to config.toml + Redis
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateConfigRequest>,
) -> Result<Json<Config>, StatusCode> {
    // Clone current config, apply changes
    let mut cfg = state.config.clone();
    if let Some(sites) = body.banned_sites {
        cfg.banned_sites = sites;
    }
    if let Some(apps) = body.banned_apps {
        cfg.banned_apps = apps;
    }
    if let Some(sau) = body.sau_mode {
        cfg.sau_mode = sau;
    }

    // Persist to config.toml
    let toml_str = toml::to_string_pretty(&cfg).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write("config.toml", &toml_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Publish to Redis so student agents can pick it up
    let mut conn = state.redis.clone();
    let prefix = &state.config.key_prefix;
    let ban_json = serde_json::json!({
        "banned_processes": cfg.banned_apps,
        "banned_domains": cfg.banned_sites,
    });
    let key = format!("{prefix}:ban_config");
    let _: Result<(), _> = redis::AsyncCommands::set::<_, _, ()>(
        &mut conn, &key, ban_json.to_string(),
    ).await;

    // Publish SAU mode to Redis
    let sau_key = format!("{prefix}:sau_mode");
    let _: Result<(), _> = redis::AsyncCommands::set::<_, _, ()>(
        &mut conn, &sau_key, if cfg.sau_mode { "1" } else { "0" },
    ).await;

    tracing::info!("✅ Config updated & pushed to Redis — {} apps, {} sites, SAU={}",
        cfg.banned_apps.len(), cfg.banned_sites.len(), cfg.sau_mode);

    Ok(Json(cfg))
}
