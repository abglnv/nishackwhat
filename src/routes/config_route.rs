use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::config::Config;
use crate::state::AppState;

pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<Config> {
    Json(state.config.clone())
}
