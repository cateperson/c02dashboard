mod common;

use axum::{body::Body, http::{Request, StatusCode}};
use ntfy_sender::{db, models::{Co2Reading, unix_now}};

fn json_reading(co2: f64) -> Request<Body> {
    let body = format!(r#"{{"co2":{co2},"time":{}}}"#, unix_now());
    Request::builder()
        .method("POST")
        .uri("/data")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

// ── status text in rendered HTML ──────────────────────────────────────────────

#[tokio::test]
async fn dashboard_shows_good_by_default() {
    let state = common::state_bare().await;
    let app = common::router_for(state);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let resp = common::oneshot(app, req, common::loopback()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let html = common::body_string(resp).await;
    assert!(html.contains("GOOD"), "expected GOOD in: {}", &html[..500.min(html.len())]);
    assert!(html.contains("status-chip--ok"));
}

#[tokio::test]
async fn dashboard_shows_okay_after_warn_reading() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());

    let post = common::oneshot(app.clone(), json_reading(0.12), common::loopback()).await;
    assert_eq!(post.status(), StatusCode::CREATED);

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let html = common::body_string(common::oneshot(app, req, common::loopback()).await).await;
    assert!(html.contains("OKAY"), "expected OKAY in html");
    assert!(html.contains("status-chip--warn"));
}

#[tokio::test]
async fn dashboard_shows_bad_after_danger_reading() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());

    let post = common::oneshot(app.clone(), json_reading(0.17), common::loopback()).await;
    assert_eq!(post.status(), StatusCode::CREATED);

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let html = common::body_string(common::oneshot(app, req, common::loopback()).await).await;
    assert!(html.contains("BAD"), "expected BAD in html");
    assert!(html.contains("status-chip--danger"));
}

#[tokio::test]
async fn dashboard_reverts_to_good_after_low_reading() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());

    common::oneshot(app.clone(), json_reading(0.12), common::loopback()).await;
    common::oneshot(app.clone(), json_reading(0.05), common::loopback()).await;

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let html = common::body_string(common::oneshot(app, req, common::loopback()).await).await;
    assert!(html.contains("GOOD"));
}

// ── auth: loopback vs external ────────────────────────────────────────────────

#[tokio::test]
async fn post_data_loopback_returns_201() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());
    let resp = common::oneshot(app, json_reading(0.08), common::loopback()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(db::reading_count(&state.pool).await, 1);
}

#[tokio::test]
async fn post_data_external_returns_403() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());
    let resp = common::oneshot(app, json_reading(0.08), common::external()).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(db::reading_count(&state.pool).await, 0);
}

#[tokio::test]
async fn delete_data_external_returns_403() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());
    db::insert_reading(&state.pool, &Co2Reading { co2: 0.07, time: unix_now() }).await.unwrap();

    let req = Request::builder().method("DELETE").uri("/data").body(Body::empty()).unwrap();
    let resp = common::oneshot(app, req, common::external()).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(db::reading_count(&state.pool).await, 1, "reading should not be deleted");
}

#[tokio::test]
async fn get_data_is_public() {
    let state = common::state_bare().await;
    let app = common::router_for(state);
    let req = Request::builder().uri("/data").body(Body::empty()).unwrap();
    let resp = common::oneshot(app, req, common::external()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ── data API ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_data_period_filter() {
    let state = common::state_bare().await;
    let now = unix_now();
    db::insert_reading(&state.pool, &Co2Reading { co2: 0.07, time: now - 100.0 }).await.unwrap();
    db::insert_reading(&state.pool, &Co2Reading { co2: 0.08, time: now - 100_000.0 }).await.unwrap();

    let app = common::router_for(state);
    let req = Request::builder().uri("/data?period=1h").body(Body::empty()).unwrap();
    let resp = common::oneshot(app, req, common::loopback()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::body_string(resp).await;
    let readings: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(readings.as_array().unwrap().len(), 1, "only the recent reading should appear");
}

// ── settings round-trip ───────────────────────────────────────────────────────

#[tokio::test]
async fn settings_roundtrip() {
    let state = common::state_bare().await;
    let app = common::router_for(state.clone());

    let form = "theme=dark&warn_threshold=0.08&danger_threshold=0.14&ntfy_server=https%3A%2F%2Fntfy.sh&ntfy_topic=co2-detector-abcde&send_on_warn=1&send_on_danger=1";
    let post_req = Request::builder()
        .method("POST")
        .uri("/settings")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(form))
        .unwrap();
    let resp = common::oneshot(app.clone(), post_req, common::loopback()).await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);

    let s = db::load_settings(&state.pool).await.unwrap();
    assert!((s.warn_threshold - 0.08).abs() < 1e-9);
    assert_eq!(s.ntfy_topic, "co2-detector-abcde");

    let get_req = Request::builder().uri("/settings").body(Body::empty()).unwrap();
    let html = common::body_string(common::oneshot(app, get_req, common::loopback()).await).await;
    assert!(html.contains("0.08"), "warn threshold should appear in settings form");
}
