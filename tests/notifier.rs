mod common;

use ntfy_sender::{db, models::{Co2Reading, unix_now}, notifier};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path}};

fn reading(co2: f64) -> Co2Reading {
    Co2Reading { co2, time: unix_now() }
}

async fn mock_server_with_post_expectation(n: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .expect(n)
        .mount(&server)
        .await;
    server
}

fn header_str(req: &wiremock::Request, name: &str) -> String {
    req.headers
        .get(name)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
        .unwrap_or_default()
}

// ── ntfy send on threshold crossings ─────────────────────────────────────────

#[tokio::test]
async fn good_to_okay_fires_warn() {
    let server = mock_server_with_post_expectation(1).await;
    let state = common::state_with_ntfy(&server).await;
    notifier::on_new_reading(&state, &reading(0.12)).await;

    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "okay");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    let title = header_str(req, "title");
    assert!(title.contains("warning"), "title was: {title}");
    assert_eq!(header_str(req, "priority"), "default");
    let body = String::from_utf8(req.body.clone()).unwrap();
    assert!(body.contains("warn threshold"), "body was: {body}");
    assert!(body.contains("0.120"), "body was: {body}");
}

#[tokio::test]
async fn good_to_bad_fires_danger_only() {
    let server = mock_server_with_post_expectation(1).await;
    let state = common::state_with_ntfy(&server).await;
    notifier::on_new_reading(&state, &reading(0.17)).await;

    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "bad");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    let title = header_str(req, "title");
    assert!(title.contains("danger"), "title was: {title}");
    assert_eq!(header_str(req, "priority"), "high");
    let body = String::from_utf8(req.body.clone()).unwrap();
    assert!(body.contains("danger threshold"), "body was: {body}");
}

#[tokio::test]
async fn okay_to_bad_fires_danger() {
    let server = mock_server_with_post_expectation(1).await;
    let state = common::state_with_ntfy(&server).await;
    common::prime_status(&state, "okay").await;

    notifier::on_new_reading(&state, &reading(0.17)).await;

    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "bad");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn bad_to_okay_is_silent() {
    let server = mock_server_with_post_expectation(0).await;
    let state = common::state_with_ntfy(&server).await;
    common::prime_status(&state, "bad").await;

    notifier::on_new_reading(&state, &reading(0.12)).await;

    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "okay");
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn same_status_sends_no_duplicate() {
    let server = mock_server_with_post_expectation(0).await;
    let state = common::state_with_ntfy(&server).await;
    common::prime_status(&state, "okay").await;

    notifier::on_new_reading(&state, &reading(0.12)).await;

    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

// ── suppression flags ─────────────────────────────────────────────────────────

#[tokio::test]
async fn send_on_warn_false_suppresses() {
    let server = mock_server_with_post_expectation(0).await;
    let state = common::state_with_ntfy(&server).await;

    let mut s = db::load_settings(&state.pool).await.unwrap();
    s.send_on_warn = false;
    common::set_settings(&state, &s).await;

    notifier::on_new_reading(&state, &reading(0.12)).await;

    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "okay", "last_status should still update even when suppressed");
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn send_on_danger_false_suppresses() {
    let server = mock_server_with_post_expectation(0).await;
    let state = common::state_with_ntfy(&server).await;

    let mut s = db::load_settings(&state.pool).await.unwrap();
    s.send_on_danger = false;
    common::set_settings(&state, &s).await;

    notifier::on_new_reading(&state, &reading(0.17)).await;

    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "bad", "last_status should still update even when suppressed");
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn empty_topic_skips_http() {
    let state = common::state_bare().await;
    notifier::on_new_reading(&state, &reading(0.17)).await;
    let (last, _) = common::get_notif_state(&state).await;
    assert_eq!(last, "bad");
}

// ── offline detection ─────────────────────────────────────────────────────────

#[tokio::test]
async fn new_reading_clears_offline_flag() {
    let state = common::state_bare().await;
    common::prime_offline_alerted(&state).await;

    notifier::on_new_reading(&state, &reading(0.06)).await;

    let (_, alerted) = common::get_notif_state(&state).await;
    assert!(!alerted, "offline_alerted should be cleared when a new reading arrives");
}

#[tokio::test]
async fn offline_check_fires_once_then_deduplicates() {
    let server = mock_server_with_post_expectation(1).await;

    let state = common::state_with_ntfy(&server).await;

    let stale = Co2Reading { co2: 0.06, time: unix_now() - 600.0 };
    db::insert_reading(&state.pool, &stale).await.unwrap();

    let fired = notifier::check_offline_once(&state).await;
    assert!(fired, "should fire on first check");

    let (_, alerted) = common::get_notif_state(&state).await;
    assert!(alerted);

    let fired_again = notifier::check_offline_once(&state).await;
    assert!(!fired_again, "should not fire a second time");

    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn offline_check_skips_when_reading_is_fresh() {
    let server = mock_server_with_post_expectation(0).await;
    let state = common::state_with_ntfy(&server).await;

    let fresh = Co2Reading { co2: 0.06, time: unix_now() };
    db::insert_reading(&state.pool, &fresh).await.unwrap();

    let fired = notifier::check_offline_once(&state).await;
    assert!(!fired);
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn offline_clears_after_new_reading_allows_next_outage() {
    let server = mock_server_with_post_expectation(1).await;
    let state = common::state_with_ntfy(&server).await;

    let stale = Co2Reading { co2: 0.06, time: unix_now() - 600.0 };
    db::insert_reading(&state.pool, &stale).await.unwrap();
    notifier::check_offline_once(&state).await;

    notifier::on_new_reading(&state, &reading(0.06)).await;

    let (_, alerted) = common::get_notif_state(&state).await;
    assert!(!alerted, "flag should reset after a new reading arrives");
}
