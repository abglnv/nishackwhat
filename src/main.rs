use axum::routing::{get, post};
use axum::Router;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod config;
mod models;
mod redis_store;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    let cfg = config::load_config();
    tracing::info!("Config loaded — port {}, redis {}", cfg.port, cfg.redis_url);

    // Redis connection
    let redis_client = redis::Client::open(cfg.redis_url.as_str())
        .expect("Invalid redis_url in config");
    let redis_conn = redis_client
        .get_connection_manager()
        .await
        .expect("Cannot connect to Redis — is it running?");

    let shared = Arc::new(AppState::new(cfg.clone(), redis_conn));

    // Background tasks
    tokio::spawn(ip_update_task(shared.clone()));

    // Routes
    let api = Router::new()
        // Teacher-facing reads
        .route("/health", get(routes::health::health))
        .route("/info", get(routes::info::info))
        .route("/violations", get(routes::violations::violations))
        .route("/config", get(routes::config_route::get_config))
        .route("/students", get(routes::students::list_students))
        .route("/students/active", get(routes::students::list_active))
        .route("/students/{hostname}", get(routes::students::student_detail))
        .route("/students/{hostname}/lock", post(routes::lock::lock_student))
        .route("/students/{hostname}/open-url", post(routes::lock::open_url_student))
        // Agent data ingestion
        .route("/agent/heartbeat", post(routes::agent::heartbeat))
        .route("/agent/screenshot", post(routes::agent::screenshot))
        .route("/agent/notification", post(routes::agent::notification))
        .route("/agent/apps", post(routes::agent::apps))
        .route("/agent/violation", post(routes::agent::violation))
        // Screen streaming info
        .route("/screen/students", get(screen_students_handler));

    let app = Router::new()
        .nest("/api", api)
        .route("/ws", get(routes::ws::ws_handler))
        .route("/ws/screen", get(routes::screen_ws::ws_screen_student))
        .route("/ws/screen/view", get(routes::screen_ws::ws_screen_teacher))
        .fallback_service(ServeDir::new("frontend"))
        .layer(CorsLayer::permissive())
        .with_state(shared);

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Cannot bind address");

    tracing::info!("🚀 NiShack backend listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

/// GET /api/screen/students — list students currently streaming screens
async fn screen_students_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let students: Vec<String> = state
        .screen_latest
        .iter()
        .map(|e| e.key().clone())
        .collect();
    Json(serde_json::json!({
        "count": students.len(),
        "students": students,
    }))
}

/// Every 5 minutes, store this machine's IP in Redis.
async fn ip_update_task(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        match local_ip_address::local_ip() {
            Ok(ip) => {
                let mut conn = state.redis.clone();
                let _ = redis_store::update_server_ip(
                    &mut conn,
                    &state.config.key_prefix,
                    &ip.to_string(),
                )
                .await;
                tracing::info!("Updated server IP in Redis: {ip}");
            }
            Err(e) => tracing::warn!("Could not determine local IP: {e}"),
        }
    }
}
