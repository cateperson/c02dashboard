use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use minijinja::context;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

use crate::{
    auth,
    db,
    models::{Co2Reading, Period, SettingsForm, compute_status, unix_now},
    notifier,
    ntfy,
    state::AppState,
};

#[derive(Deserialize)]
pub struct PeriodQuery {
    period: Option<String>,
}

pub async fn dashboard(
    State(state): State<AppState>,
    Query(q): Query<PeriodQuery>,
) -> Response {
    let period = Period::from_str(q.period.as_deref().unwrap_or("1h"));
    let cutoff = unix_now() - period.window_secs();

    let (readings, settings) = match tokio::join!(
        db::readings_since(&state.pool, cutoff),
        db::load_settings(&state.pool),
    ) {
        (Ok(r), Ok(s)) => (r, s),
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let (current, min, max, avg) = if readings.is_empty() {
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64)
    } else {
        let cur = readings.last().unwrap().co2;
        let mn = readings.iter().map(|r| r.co2).fold(f64::INFINITY, f64::min);
        let mx = readings.iter().map(|r| r.co2).fold(f64::NEG_INFINITY, f64::max);
        let av = readings.iter().map(|r| r.co2).sum::<f64>() / readings.len() as f64;
        (cur, mn, mx, av)
    };

    let status = compute_status(current, settings.warn_threshold, settings.danger_threshold);
    let now = unix_now();
    let updated_at = {
        let secs = now as u64;
        let h = (secs % 86400) / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    };

    let colors = theme_colors(&settings.theme);
    let bootstrap_json = json!({
        "readings": readings,
        "windowMs": period.window_ms(),
        "warn": settings.warn_threshold,
        "danger": settings.danger_threshold,
        "period": period.as_str(),
        "colors": colors,
    })
    .to_string();

    let periods_list: Vec<_> = [
        Period::H1, Period::H12, Period::H24, Period::W1, Period::Mo1, Period::Y1,
    ]
    .iter()
    .map(|p| json!({ "key": p.as_str(), "label": p.label() }))
    .collect();

    let max_class = if max >= settings.warn_threshold { "text-warn" } else { "text-ink" };

    let tmpl = state.env.get_template("dashboard.html").unwrap();
    let rendered = tmpl.render(context! {
        theme => &settings.theme,
        period => period.as_str(),
        period_label => period.label(),
        periods => periods_list,
        current => format!("{:.3}", current),
        min => format!("{:.3}", min),
        max => format!("{:.3}", max),
        avg => format!("{:.3}", avg),
        max_class => max_class,
        status_label => status.label(),
        status_class => status.class(),
        updated_at => updated_at,
        warn_threshold => settings.warn_threshold,
        danger_threshold => settings.danger_threshold,
        bootstrap_json => bootstrap_json,
    })
    .unwrap();

    Html(rendered).into_response()
}

pub async fn get_data(
    State(state): State<AppState>,
    Query(q): Query<PeriodQuery>,
) -> Response {
    let period = Period::from_str(q.period.as_deref().unwrap_or("1h"));
    let cutoff = unix_now() - period.window_secs();
    match db::readings_since(&state.pool, cutoff).await {
        Ok(r) => Json(r).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn post_data(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(reading): Json<Co2Reading>,
) -> Response {
    if let Err(code) = auth::guard(addr) {
        return code.into_response();
    }
    if db::insert_reading(&state.pool, &reading).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    notifier::on_new_reading(&state, &reading).await;
    StatusCode::CREATED.into_response()
}

pub async fn delete_data(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Response {
    if let Err(code) = auth::guard(addr) {
        return code.into_response();
    }
    match db::delete_all_readings(&state.pool).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn settings_get(State(state): State<AppState>) -> Response {
    let settings = match db::load_settings(&state.pool).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let tmpl = state.env.get_template("settings.html").unwrap();
    let rendered = tmpl.render(context! {
        theme => &settings.theme,
        warn_threshold => settings.warn_threshold,
        danger_threshold => settings.danger_threshold,
        ntfy_server => &settings.ntfy_server,
        ntfy_topic => &settings.ntfy_topic,
        send_on_warn => settings.send_on_warn,
        send_on_danger => settings.send_on_danger,
    })
    .unwrap();
    Html(rendered).into_response()
}

pub async fn settings_post(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Form(form): Form<SettingsForm>,
) -> Response {
    if let Err(code) = auth::guard(addr) {
        return code.into_response();
    }
    let mut settings = match db::load_settings(&state.pool).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if let Some(t) = form.theme {
        if t == "dark" || t == "light" {
            settings.theme = t;
        }
    }
    if let Some(v) = form.warn_threshold { settings.warn_threshold = v; }
    if let Some(v) = form.danger_threshold { settings.danger_threshold = v; }
    if let Some(v) = form.ntfy_server { settings.ntfy_server = v; }
    if let Some(v) = form.ntfy_topic { settings.ntfy_topic = v; }
    settings.send_on_warn = form.send_on_warn.is_some();
    settings.send_on_danger = form.send_on_danger.is_some();
    let _ = db::save_settings(&state.pool, &settings).await;
    Redirect::to("/settings").into_response()
}

pub async fn settings_regen_topic(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Response {
    if let Err(code) = auth::guard(addr) {
        return code.into_response();
    }
    let mut settings = match db::load_settings(&state.pool).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    settings.ntfy_topic = {
        let mut rng = rand::thread_rng();
        let suffix: String = (0..5)
            .map(|_| {
                let n = rng.gen_range(0..36u8);
                if n < 10 { (b'0' + n) as char } else { (b'a' + n - 10) as char }
            })
            .collect();
        format!("co2-detector-{}", suffix)
    };
    let _ = db::save_settings(&state.pool, &settings).await;
    Redirect::to("/settings").into_response()
}

pub async fn settings_test(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Response {
    if let Err(code) = auth::guard(addr) {
        return code.into_response();
    }
    let settings = match db::load_settings(&state.pool).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let _ = ntfy::send(
        &state.http,
        &settings.ntfy_server,
        &settings.ntfy_topic,
        "CO\u{2082} test",
        "default",
        "Hello from co2dashboard.",
    )
    .await;
    Redirect::to("/settings").into_response()
}

pub async fn static_file(Path(file): Path<String>) -> Response {
    match file.as_str() {
        "output.css" => {
            match tokio::fs::read("static/output.css").await {
                Ok(bytes) => (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    bytes,
                )
                    .into_response(),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn theme_colors(theme: &str) -> serde_json::Value {
    if theme == "light" {
        json!({
            "accent": "#3a8a98",
            "accentRgb": "58, 138, 152",
            "warn": "#b07a2a",
            "warnRgb": "176, 122, 42",
            "danger": "#b8412e",
            "dangerRgb": "184, 65, 46",
            "ok": "#4a8a64",
            "line": "#e2e1dc",
            "inkSoft": "#6b7178",
            "panel": "#ffffff",
            "panel2": "#faf8f3",
            "ink": "#1a1d20",
            "bg": "#f6f5f1",
        })
    } else {
        json!({
            "accent": "#5fb8c4",
            "accentRgb": "95, 184, 196",
            "warn": "#d4a24a",
            "warnRgb": "212, 162, 74",
            "danger": "#d75a4a",
            "dangerRgb": "215, 90, 74",
            "ok": "#6fb88a",
            "line": "#262c32",
            "inkSoft": "#8a929b",
            "panel": "#14181c",
            "panel2": "#1a1f24",
            "ink": "#eef1f4",
            "bg": "#0d0f11",
        })
    }
}
