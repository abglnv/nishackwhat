use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use tokio::sync::mpsc;

use crate::config::Config;

pub type WsTx = mpsc::UnboundedSender<axum::extract::ws::Message>;
pub type WsClients = DashMap<String, WsTx>;

pub struct AppState {
    pub config: Config,
    pub redis: ConnectionManager,
    pub start_time: DateTime<Utc>,
    pub ws_clients: WsClients,
}

impl AppState {
    pub fn new(config: Config, redis: ConnectionManager) -> Self {
        Self {
            config,
            redis,
            start_time: Utc::now(),
            ws_clients: DashMap::new(),
        }
    }
}
