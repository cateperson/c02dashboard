use minijinja::Environment;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub env: Arc<Environment<'static>>,
    pub http: reqwest::Client,
}
