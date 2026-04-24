#![allow(dead_code, unused_imports)]

use std::{net::SocketAddr, sync::Arc};
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::Request,
    routing::{get, post},
    Router,
};
use ntfy_sender::{db, handlers, models::Settings, state::AppState, templates};
use tower::ServiceExt;
use wiremock::MockServer;

pub async fn setup_pool() -> sqlx::SqlitePool {
    let pool = db::init_pool("sqlite::memory:").await;
    db::run_migrations(&pool).await;
    pool
}

pub async fn state_with_ntfy(server: &MockServer) -> AppState {
    let pool = setup_pool().await;
    let mut s = db::load_settings(&pool).await.unwrap();
    s.ntfy_server = server.uri();
    s.ntfy_topic = "test".to_string();
    db::save_settings(&pool, &s).await.unwrap();
    AppState {
        pool,
        env: Arc::new(templates::build_env()),
        http: reqwest::Client::new(),
    }
}

pub async fn state_bare() -> AppState {
    let pool = setup_pool().await;
    AppState {
        pool,
        env: Arc::new(templates::build_env()),
        http: reqwest::Client::new(),
    }
}

pub async fn set_settings(state: &AppState, s: &Settings) {
    db::save_settings(&state.pool, s).await.unwrap();
}

pub async fn get_notif_state(state: &AppState) -> (String, bool) {
    let ns = db::load_notif_state(&state.pool).await.unwrap();
    (ns.last_status, ns.offline_alerted)
}

pub async fn prime_status(state: &AppState, status: &str) {
    let ns = ntfy_sender::models::NotifState {
        last_status: status.to_string(),
        offline_alerted: false,
    };
    db::save_notif_state(&state.pool, &ns).await.unwrap();
}

pub async fn prime_offline_alerted(state: &AppState) {
    let ns = ntfy_sender::models::NotifState {
        last_status: "good".to_string(),
        offline_alerted: true,
    };
    db::save_notif_state(&state.pool, &ns).await.unwrap();
}

pub fn router_for(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::dashboard))
        .route("/data", get(handlers::get_data).post(handlers::post_data).delete(handlers::delete_data))
        .route("/settings", get(handlers::settings_get).post(handlers::settings_post))
        .route("/settings/regen-topic", post(handlers::settings_regen_topic))
        .route("/settings/test", post(handlers::settings_test))
        .route("/static/:file", get(handlers::static_file))
        .with_state(state)
}

pub async fn oneshot(app: Router, mut req: Request<Body>, from: SocketAddr) -> axum::response::Response {
    req.extensions_mut().insert(ConnectInfo(from));
    app.oneshot(req).await.unwrap()
}

pub async fn body_string(resp: axum::response::Response) -> String {
    use http_body_util::BodyExt;
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

pub fn loopback() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 12345))
}

pub fn external() -> SocketAddr {
    SocketAddr::from(([192, 168, 1, 5], 12345))
}
