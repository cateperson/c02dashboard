use axum::{
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};

mod auth;
mod db;
mod handlers;
mod models;
mod notifier;
mod ntfy;
mod state;
mod templates;

use state::AppState;

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./data/co2.db".into());

    let pool = db::init_pool(&db_url).await;
    db::run_migrations(&pool).await;

    let env = Arc::new(templates::build_env());
    let http = reqwest::Client::new();
    let state = AppState { pool, env, http };

    tokio::spawn(notifier::offline_watchdog(state.clone()));

    let app = Router::new()
        .route("/", get(handlers::dashboard))
        .route("/data", get(handlers::get_data).post(handlers::post_data).delete(handlers::delete_data))
        .route("/settings", get(handlers::settings_get).post(handlers::settings_post))
        .route("/settings/regen-topic", post(handlers::settings_regen_topic))
        .route("/settings/test", post(handlers::settings_test))
        .route("/static/:file", get(handlers::static_file))
        .with_state(state);

    let port = parse_port_flag().unwrap_or_else(|| {
        std::env::var("PORT").ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(3000)
    });
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await
        .unwrap_or_else(|e| { eprintln!("error: cannot bind {addr}: {e}"); std::process::exit(1) });
    println!("Listening on http://{}", addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn parse_port_flag() -> Option<u16> {
    let args: Vec<String> = std::env::args().collect();
    let idx = args.iter().position(|a| a == "-p" || a == "--port")?;
    args.get(idx + 1)?.parse().ok()
}
