use std::time::Duration;
use crate::{
    db,
    models::{Co2Reading, NotifState, Status, compute_status, unix_now},
    ntfy,
    state::AppState,
};

const OFFLINE_SECS: f64 = 300.0;

pub async fn on_new_reading(state: &AppState, reading: &Co2Reading) {
    let (settings, mut ns) = match tokio::join!(
        db::load_settings(&state.pool),
        db::load_notif_state(&state.pool),
    ) {
        (Ok(s), Ok(n)) => (s, n),
        _ => return,
    };

    if ns.offline_alerted {
        ns.offline_alerted = false;
        let _ = db::save_notif_state(&state.pool, &ns).await;
    }

    let new_status = compute_status(reading.co2, settings.warn_threshold, settings.danger_threshold);
    let prev_status = Status::from_str(&ns.last_status);

    if new_status != prev_status {
        match (prev_status, new_status) {
            (Status::Good, Status::Okay) if settings.send_on_warn && !settings.ntfy_topic.is_empty() => {
                let body = format!("CO\u{2082} crossed warn threshold: {:.3}% (warn: {:.3}%)", reading.co2, settings.warn_threshold);
                let _ = ntfy::send(&state.http, &settings.ntfy_server, &settings.ntfy_topic, "CO\u{2082} warning", "default", &body).await;
            }
            (_, Status::Bad) if settings.send_on_danger && !settings.ntfy_topic.is_empty() => {
                let body = format!("CO\u{2082} crossed danger threshold: {:.3}% (danger: {:.3}%)", reading.co2, settings.danger_threshold);
                let _ = ntfy::send(&state.http, &settings.ntfy_server, &settings.ntfy_topic, "CO\u{2082} danger", "high", &body).await;
            }
            _ => {}
        }
        ns.last_status = new_status.as_str().to_string();
        let _ = db::save_notif_state(&state.pool, &ns).await;
    }
}

pub async fn check_offline_once(state: &AppState) -> bool {
    let latest = match db::latest_reading(&state.pool).await {
        Ok(Some(r)) => r,
        _ => return false,
    };

    if unix_now() - latest.time < OFFLINE_SECS {
        return false;
    }

    let ns = match db::load_notif_state(&state.pool).await {
        Ok(n) => n,
        Err(_) => return false,
    };

    if ns.offline_alerted {
        return false;
    }

    let settings = match db::load_settings(&state.pool).await {
        Ok(s) => s,
        Err(_) => return false,
    };

    if !settings.ntfy_topic.is_empty() {
        let mins = ((unix_now() - latest.time) / 60.0) as u64;
        let body = format!("No reading received for {} minutes.", mins);
        let _ = ntfy::send(&state.http, &settings.ntfy_server, &settings.ntfy_topic, "CO\u{2082} sensor offline", "high", &body).await;
    }

    let _ = db::save_notif_state(&state.pool, &NotifState {
        last_status: ns.last_status,
        offline_alerted: true,
    }).await;

    true
}

pub async fn offline_watchdog(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let _ = check_offline_once(&state).await;
    }
}
