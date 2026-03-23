use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("teslacam.db");
        let conn = Connection::open(db_path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                type        TEXT NOT NULL,
                timestamp   TEXT NOT NULL,
                duration_s  INTEGER,
                gps_lat     REAL,
                gps_lon     REAL,
                avg_speed   REAL,
                max_speed   REAL,
                source_dir  TEXT NOT NULL,
                backed_up   INTEGER DEFAULT 0,
                notes       TEXT
            );

            CREATE TABLE IF NOT EXISTS clips (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id    INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
                camera      TEXT NOT NULL,
                file_path   TEXT NOT NULL,
                file_size   INTEGER,
                duration_s  REAL,
                has_sei     INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS telemetry (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id     INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                offset_ms   INTEGER NOT NULL,
                speed       REAL,
                steering    REAL,
                gps_lat     REAL,
                gps_lon     REAL,
                drive_state TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_type ON events(type);
            CREATE INDEX IF NOT EXISTS idx_clips_event ON clips(event_id);
            CREATE INDEX IF NOT EXISTS idx_telemetry_clip ON telemetry(clip_id);
            ",
        )?;
        Ok(())
    }
}
