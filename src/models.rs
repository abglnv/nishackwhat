use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Agent heartbeat ──────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub hostname: String,
    pub ip: String,
    pub port: u16,
    pub os: String,
    pub username: String,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub uptime_secs: u64,
    pub timestamp: DateTime<Utc>,
}

// ── Screenshot from student ──────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub hostname: String,
    pub image_url: String, // base64 data-uri or URL
    pub timestamp: DateTime<Utc>,
}

// ── Notification from student ────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub hostname: String,
    pub title: String,
    pub message: String,
    pub level: String, // info | warning | error
    pub timestamp: DateTime<Utc>,
}

// ── Running applications ─────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub name: String,
    pub pid: u32,
    pub memory_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub browser: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppList {
    pub hostname: String,
    pub applications: Vec<Application>,
    pub browser_tabs: Vec<BrowserTab>,
    pub timestamp: DateTime<Utc>,
}

// ── Violation ────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub hostname: String,
    pub rule: String,
    pub detail: String,
    pub severity: String, // low | medium | high
    pub timestamp: DateTime<Utc>,
}

// ── Student summary (returned by /api/students) ──────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentSummary {
    pub hostname: String,
    pub ip: String,
    pub port: u16,
    pub active: bool,
    pub os: String,
    pub username: String,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub violation_count: i64,
    pub last_seen: Option<DateTime<Utc>>,
}

// ── Full student detail ──────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentDetail {
    pub summary: StudentSummary,
    pub screenshot: Option<Screenshot>,
    pub apps: Option<AppList>,
    pub notifications: Vec<Notification>,
    pub violations: Vec<Violation>,
}

// ── WebSocket chat message ───────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChatMessage {
    #[serde(rename = "type")]
    pub msg_type: String, // identify | chat | system
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub role: String, // teacher | student
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
}
