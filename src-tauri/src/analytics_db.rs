use rusqlite::{Connection, params};

/// 目前最新的 schema 版本
const CURRENT_VERSION: i32 = 2;

/// 檢查並執行 schema 遷移
pub fn migrate(conn: &Connection) -> Result<(), String> {
    // 建立 schema_version 表
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);"
    ).map_err(|e| format!("建立 schema_version 失敗: {}", e))?;

    let version: i32 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| row.get(0))
        .unwrap_or(0);

    if version < 2 {
        migrate_v2(conn)?;
    }

    // 更新版本號
    if version < CURRENT_VERSION {
        conn.execute("DELETE FROM schema_version", [])
            .map_err(|e| e.to_string())?;
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", params![CURRENT_VERSION])
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// V2: 新增 trips, daily_stats, telemetry_samples 表
fn migrate_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS trips (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            vehicle_id        INTEGER NOT NULL,
            event_id          INTEGER,
            start_time        TEXT NOT NULL,
            end_time          TEXT NOT NULL,
            duration_sec      REAL NOT NULL,
            distance_km       REAL NOT NULL DEFAULT 0,
            avg_speed_kmh     REAL,
            max_speed_kmh     REAL,
            start_lat         REAL,
            start_lon         REAL,
            end_lat           REAL,
            end_lon           REAL,
            event_count       INTEGER DEFAULT 0,
            hard_brake_count  INTEGER DEFAULT 0,
            hard_accel_count  INTEGER DEFAULT 0,
            sharp_turn_count  INTEGER DEFAULT 0,
            autopilot_pct     REAL DEFAULT 0,
            driving_score     INTEGER,
            FOREIGN KEY (vehicle_id) REFERENCES vehicles(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS daily_stats (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            vehicle_id        INTEGER NOT NULL,
            date              TEXT NOT NULL,
            trip_count        INTEGER DEFAULT 0,
            total_distance_km REAL DEFAULT 0,
            total_duration_sec REAL DEFAULT 0,
            avg_speed_kmh     REAL,
            max_speed_kmh     REAL,
            event_count       INTEGER DEFAULT 0,
            driving_score     INTEGER,
            UNIQUE(vehicle_id, date)
        );

        CREATE TABLE IF NOT EXISTS telemetry_samples (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            trip_id   INTEGER NOT NULL,
            time_sec  REAL NOT NULL,
            lat       REAL NOT NULL,
            lon       REAL NOT NULL,
            speed_kmh REAL,
            heading   REAL,
            FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_trips_vehicle_time ON trips(vehicle_id, start_time);
        CREATE INDEX IF NOT EXISTS idx_daily_vehicle_date ON daily_stats(vehicle_id, date);
        CREATE INDEX IF NOT EXISTS idx_telemetry_trip ON telemetry_samples(trip_id);
        "
    ).map_err(|e| format!("V2 遷移失敗: {}", e))?;

    Ok(())
}
