CREATE TABLE IF NOT EXISTS readings (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    co2   REAL    NOT NULL,
    time  REAL    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_readings_time ON readings(time);

CREATE TABLE IF NOT EXISTS settings (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    theme             TEXT    NOT NULL DEFAULT 'dark',
    warn_threshold    REAL    NOT NULL DEFAULT 0.10,
    danger_threshold  REAL    NOT NULL DEFAULT 0.15,
    ntfy_server       TEXT    NOT NULL DEFAULT 'https://ntfy.sh',
    ntfy_topic        TEXT    NOT NULL DEFAULT '',
    send_on_warn      INTEGER NOT NULL DEFAULT 1,
    send_on_danger    INTEGER NOT NULL DEFAULT 1
);

INSERT OR IGNORE INTO settings (id) VALUES (1);

CREATE TABLE IF NOT EXISTS notif_state (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    last_status      TEXT    NOT NULL DEFAULT 'good',
    offline_alerted  INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO notif_state (id) VALUES (1);
