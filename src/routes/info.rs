use axum::Json;
use serde_json::{json, Value};
use sysinfo::System;

pub async fn info() -> Json<Value> {
    let sys = System::new_all();

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();

    Json(json!({
        "hostname": System::host_name().unwrap_or_default(),
        "os": format!("{} {}", System::name().unwrap_or_default(), System::os_version().unwrap_or_default()),
        "memory": {
            "total_mb": total_mem / 1024 / 1024,
            "used_mb": used_mem / 1024 / 1024,
            "usage_percent": if total_mem > 0 { (used_mem as f64 / total_mem as f64 * 100.0).round() } else { 0.0 }
        },
        "cpu_count": sys.cpus().len(),
        "process_count": sys.processes().len(),
        "username": std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default()
    }))
}
