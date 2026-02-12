use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::models::*;

type R<T> = redis::RedisResult<T>;

// ── Heartbeat ────────────────────────────────────────────
pub async fn store_heartbeat(
    conn: &mut ConnectionManager,
    prefix: &str,
    hb: &Heartbeat,
    ttl: u64,
) -> R<()> {
    let key = format!("{prefix}:heartbeat:{}", hb.hostname);
    let json = serde_json::to_string(hb).unwrap_or_default();
    conn.set_ex(&key, &json, ttl).await
}

pub async fn get_heartbeat(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
) -> R<Option<Heartbeat>> {
    let key = format!("{prefix}:heartbeat:{hostname}");
    let val: Option<String> = conn.get(&key).await?;
    Ok(val.and_then(|v| serde_json::from_str(&v).ok()))
}

// ── Agent registry ───────────────────────────────────────
pub async fn register_agent(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
    ip: &str,
    port: u16,
) -> R<()> {
    let key = format!("{prefix}:agents");
    let member = format!("{hostname}|{ip}|{port}");
    conn.sadd(&key, &member).await
}

pub async fn get_all_agents(conn: &mut ConnectionManager, prefix: &str) -> R<Vec<String>> {
    let key = format!("{prefix}:agents");
    conn.smembers(&key).await
}

// ── Violations ───────────────────────────────────────────
pub async fn add_violation(
    conn: &mut ConnectionManager,
    prefix: &str,
    v: &Violation,
) -> R<()> {
    let list_key = format!("{prefix}:violations:{}", v.hostname);
    let count_key = format!("{prefix}:violation_count:{}", v.hostname);
    let json = serde_json::to_string(v).unwrap_or_default();
    let _: () = conn.lpush(&list_key, &json).await?;
    let _: () = conn.incr(&count_key, 1i64).await?;
    Ok(())
}

pub async fn get_violations(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
    count: isize,
) -> R<Vec<Violation>> {
    let key = format!("{prefix}:violations:{hostname}");
    let items: Vec<String> = conn.lrange(&key, 0, count - 1).await?;
    Ok(items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect())
}

pub async fn get_violation_count(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
) -> R<i64> {
    let key = format!("{prefix}:violation_count:{hostname}");
    let count: Option<i64> = conn.get(&key).await?;
    Ok(count.unwrap_or(0))
}

// ── Screenshots ──────────────────────────────────────────
pub async fn store_screenshot(
    conn: &mut ConnectionManager,
    prefix: &str,
    ss: &Screenshot,
) -> R<()> {
    let key = format!("{prefix}:screenshot:{}", ss.hostname);
    let json = serde_json::to_string(ss).unwrap_or_default();
    conn.set_ex(&key, &json, 120u64).await // 2 min TTL
}

pub async fn get_screenshot(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
) -> R<Option<Screenshot>> {
    let key = format!("{prefix}:screenshot:{hostname}");
    let val: Option<String> = conn.get(&key).await?;
    Ok(val.and_then(|v| serde_json::from_str(&v).ok()))
}

// ── Notifications ────────────────────────────────────────
pub async fn store_notification(
    conn: &mut ConnectionManager,
    prefix: &str,
    n: &Notification,
) -> R<()> {
    let key = format!("{prefix}:notifications:{}", n.hostname);
    let json = serde_json::to_string(n).unwrap_or_default();
    let _: () = conn.lpush(&key, &json).await?;
    // keep last 200
    let _: () = conn.ltrim(&key, 0, 199).await?;
    Ok(())
}

pub async fn get_notifications(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
    count: isize,
) -> R<Vec<Notification>> {
    let key = format!("{prefix}:notifications:{hostname}");
    let items: Vec<String> = conn.lrange(&key, 0, count - 1).await?;
    Ok(items
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect())
}

// ── App list ─────────────────────────────────────────────
pub async fn store_apps(
    conn: &mut ConnectionManager,
    prefix: &str,
    apps: &AppList,
) -> R<()> {
    let key = format!("{prefix}:apps:{}", apps.hostname);
    let json = serde_json::to_string(apps).unwrap_or_default();
    conn.set_ex(&key, &json, 120u64).await
}

pub async fn get_apps(
    conn: &mut ConnectionManager,
    prefix: &str,
    hostname: &str,
) -> R<Option<AppList>> {
    let key = format!("{prefix}:apps:{hostname}");
    let val: Option<String> = conn.get(&key).await?;
    Ok(val.and_then(|v| serde_json::from_str(&v).ok()))
}

// ── Server IP ────────────────────────────────────────────
pub async fn update_server_ip(
    conn: &mut ConnectionManager,
    prefix: &str,
    ip: &str,
) -> R<()> {
    let key = format!("{prefix}:server:ip");
    conn.set_ex(&key, ip, 360u64).await
}
