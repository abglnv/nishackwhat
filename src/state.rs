use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::config::Config;

pub type WsTx = mpsc::UnboundedSender<axum::extract::ws::Message>;
pub type WsClients = DashMap<String, WsTx>;

/// Per-teacher sender for the screen-relay WebSocket connections
pub type ScreenTeacherTx = mpsc::UnboundedSender<Vec<u8>>;

pub struct AppState {
    pub config: Config,
    pub redis: ConnectionManager,
    pub start_time: DateTime<Utc>,
    /// Chat / general WS clients (keyed by client id)
    pub ws_clients: WsClients,
    /// Screen-streaming students currently connected (hostname -> WsTx)
    pub screen_students: DashMap<String, WsTx>,
    /// Teacher dashboard connections waiting for screen frames
    pub screen_teachers: Arc<RwLock<Vec<ScreenTeacherTx>>>,
    /// Latest JPEG frame per student (hostname -> bytes) — for instant display
    pub screen_latest: DashMap<String, Vec<u8>>,
}

impl AppState {
    pub fn new(config: Config, redis: ConnectionManager) -> Self {
        Self {
            config,
            redis,
            start_time: Utc::now(),
            ws_clients: DashMap::new(),
            screen_students: DashMap::new(),
            screen_teachers: Arc::new(RwLock::new(Vec::new())),
            screen_latest: DashMap::new(),
        }
    }
}
