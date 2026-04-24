use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;
use crate::models::{Co2Reading, NotifState, Settings};

pub async fn init_pool(url: &str) -> SqlitePool {
    if url.starts_with("sqlite://") {
        let path = &url[9..];
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
    }
    let opts = SqliteConnectOptions::from_str(url)
        .unwrap()
        .create_if_missing(true);
    SqlitePool::connect_with(opts).await.unwrap()
}

pub async fn run_migrations(pool: &SqlitePool) {
    sqlx::migrate!("./migrations").run(pool).await.unwrap();
}

pub async fn insert_reading(pool: &SqlitePool, r: &Co2Reading) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO readings (co2, time) VALUES (?, ?)")
        .bind(r.co2)
        .bind(r.time)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn readings_since(pool: &SqlitePool, cutoff: f64) -> Result<Vec<Co2Reading>, sqlx::Error> {
    sqlx::query_as::<_, Co2Reading>(
        "SELECT co2, time FROM readings WHERE time >= ? ORDER BY time ASC",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await
}

pub async fn latest_reading(pool: &SqlitePool) -> Result<Option<Co2Reading>, sqlx::Error> {
    sqlx::query_as::<_, Co2Reading>(
        "SELECT co2, time FROM readings ORDER BY time DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

pub async fn delete_all_readings(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM readings").execute(pool).await?;
    Ok(())
}

pub async fn load_settings(pool: &SqlitePool) -> Result<Settings, sqlx::Error> {
    sqlx::query_as::<_, Settings>(
        "SELECT theme, warn_threshold, danger_threshold, ntfy_server, ntfy_topic, send_on_warn, send_on_danger FROM settings WHERE id = 1",
    )
    .fetch_one(pool)
    .await
}

pub async fn save_settings(pool: &SqlitePool, s: &Settings) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE settings SET theme=?, warn_threshold=?, danger_threshold=?, ntfy_server=?, ntfy_topic=?, send_on_warn=?, send_on_danger=? WHERE id=1",
    )
    .bind(&s.theme)
    .bind(s.warn_threshold)
    .bind(s.danger_threshold)
    .bind(&s.ntfy_server)
    .bind(&s.ntfy_topic)
    .bind(s.send_on_warn as i64)
    .bind(s.send_on_danger as i64)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_notif_state(pool: &SqlitePool) -> Result<NotifState, sqlx::Error> {
    sqlx::query_as::<_, NotifState>(
        "SELECT last_status, offline_alerted FROM notif_state WHERE id = 1",
    )
    .fetch_one(pool)
    .await
}

pub async fn save_notif_state(pool: &SqlitePool, ns: &NotifState) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE notif_state SET last_status=?, offline_alerted=? WHERE id=1",
    )
    .bind(&ns.last_status)
    .bind(ns.offline_alerted as i64)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn reading_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM readings")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}
