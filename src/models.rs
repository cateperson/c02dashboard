use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct Co2Reading {
    pub co2: f64,
    pub time: f64,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Co2Reading {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Co2Reading {
            co2: row.try_get("co2")?,
            time: row.try_get("time")?,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub warn_threshold: f64,
    pub danger_threshold: f64,
    pub ntfy_server: String,
    pub ntfy_topic: String,
    pub send_on_warn: bool,
    pub send_on_danger: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            warn_threshold: 0.10,
            danger_threshold: 0.15,
            ntfy_server: "https://ntfy.sh".into(),
            ntfy_topic: String::new(),
            send_on_warn: true,
            send_on_danger: true,
        }
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Settings {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Settings {
            theme: row.try_get("theme")?,
            warn_threshold: row.try_get("warn_threshold")?,
            danger_threshold: row.try_get("danger_threshold")?,
            ntfy_server: row.try_get("ntfy_server")?,
            ntfy_topic: row.try_get("ntfy_topic")?,
            send_on_warn: row.try_get::<i64, _>("send_on_warn")? != 0,
            send_on_danger: row.try_get::<i64, _>("send_on_danger")? != 0,
        })
    }
}

#[derive(Clone)]
pub struct NotifState {
    pub last_status: String,
    pub offline_alerted: bool,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for NotifState {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(NotifState {
            last_status: row.try_get("last_status")?,
            offline_alerted: row.try_get::<i64, _>("offline_alerted")? != 0,
        })
    }
}

#[derive(Clone, Deserialize)]
pub struct SettingsForm {
    pub theme: Option<String>,
    pub warn_threshold: Option<f64>,
    pub danger_threshold: Option<f64>,
    pub ntfy_server: Option<String>,
    pub ntfy_topic: Option<String>,
    pub send_on_warn: Option<String>,
    pub send_on_danger: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Period {
    H1,
    H12,
    H24,
    W1,
    Mo1,
    Y1,
}

impl Period {
    pub fn from_str(s: &str) -> Self {
        match s {
            "1h" => Period::H1,
            "12h" => Period::H12,
            "1w" => Period::W1,
            "1mo" => Period::Mo1,
            "1y" => Period::Y1,
            _ => Period::H24,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Period::H1 => "1h",
            Period::H12 => "12h",
            Period::H24 => "24h",
            Period::W1 => "1w",
            Period::Mo1 => "1mo",
            Period::Y1 => "1y",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Period::H1 => "1 hour",
            Period::H12 => "12 hours",
            Period::H24 => "24 hours",
            Period::W1 => "1 week",
            Period::Mo1 => "1 month",
            Period::Y1 => "1 year",
        }
    }

    pub fn window_secs(&self) -> f64 {
        match self {
            Period::H1 => 3_600.0,
            Period::H12 => 43_200.0,
            Period::H24 => 86_400.0,
            Period::W1 => 604_800.0,
            Period::Mo1 => 2_592_000.0,
            Period::Y1 => 31_536_000.0,
        }
    }

    pub fn window_ms(&self) -> f64 {
        self.window_secs() * 1000.0
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    Good,
    Okay,
    Bad,
}

impl Status {
    pub fn label(&self) -> &'static str {
        match self {
            Status::Good => "GOOD",
            Status::Okay => "OKAY",
            Status::Bad => "BAD",
        }
    }

    pub fn class(&self) -> &'static str {
        match self {
            Status::Good => "ok",
            Status::Okay => "warn",
            Status::Bad => "danger",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Good => "good",
            Status::Okay => "okay",
            Status::Bad => "bad",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "okay" => Status::Okay,
            "bad" => Status::Bad,
            _ => Status::Good,
        }
    }
}

pub fn compute_status(cur: f64, warn: f64, danger: f64) -> Status {
    if cur >= danger {
        Status::Bad
    } else if cur >= warn {
        Status::Okay
    } else {
        Status::Good
    }
}

pub fn unix_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_status_below_warn() {
        assert_eq!(compute_status(0.05, 0.10, 0.15), Status::Good);
    }

    #[test]
    fn compute_status_at_warn_boundary() {
        assert_eq!(compute_status(0.10, 0.10, 0.15), Status::Okay);
    }

    #[test]
    fn compute_status_between_thresholds() {
        assert_eq!(compute_status(0.12, 0.10, 0.15), Status::Okay);
    }

    #[test]
    fn compute_status_at_danger_boundary() {
        assert_eq!(compute_status(0.15, 0.10, 0.15), Status::Bad);
    }

    #[test]
    fn compute_status_above_danger() {
        assert_eq!(compute_status(0.20, 0.10, 0.15), Status::Bad);
    }

    #[test]
    fn period_from_str_all_keys() {
        assert_eq!(Period::from_str("1h").as_str(),  "1h");
        assert_eq!(Period::from_str("12h").as_str(), "12h");
        assert_eq!(Period::from_str("24h").as_str(), "24h");
        assert_eq!(Period::from_str("1w").as_str(),  "1w");
        assert_eq!(Period::from_str("1mo").as_str(), "1mo");
        assert_eq!(Period::from_str("1y").as_str(),  "1y");
    }

    #[test]
    fn period_from_str_unknown_defaults_to_24h() {
        assert_eq!(Period::from_str("garbage").as_str(), "24h");
        assert_eq!(Period::from_str("").as_str(), "24h");
    }

    #[test]
    fn period_window_secs() {
        assert_eq!(Period::H1.window_secs(),  3_600.0);
        assert_eq!(Period::H24.window_secs(), 86_400.0);
        assert_eq!(Period::W1.window_secs(),  604_800.0);
    }

    #[test]
    fn status_from_str_roundtrip() {
        assert_eq!(Status::Good.as_str(),  "good");
        assert_eq!(Status::Okay.as_str(),  "okay");
        assert_eq!(Status::Bad.as_str(),   "bad");
        assert_eq!(Status::from_str("good").as_str(), "good");
        assert_eq!(Status::from_str("okay").as_str(), "okay");
        assert_eq!(Status::from_str("bad").as_str(),  "bad");
    }

    #[test]
    fn status_from_str_unknown_defaults_to_good() {
        assert_eq!(Status::from_str("").as_str(),       "good");
        assert_eq!(Status::from_str("unknown").as_str(),"good");
    }

    #[test]
    fn status_labels_and_classes() {
        assert_eq!(Status::Good.label(), "GOOD");
        assert_eq!(Status::Okay.label(), "OKAY");
        assert_eq!(Status::Bad.label(),  "BAD");
        assert_eq!(Status::Good.class(), "ok");
        assert_eq!(Status::Okay.class(), "warn");
        assert_eq!(Status::Bad.class(),  "danger");
    }
}
