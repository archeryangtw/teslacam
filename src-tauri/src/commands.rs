use crate::db::Database;
use crate::scanner;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: i64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub duration_sec: i64,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub avg_speed: Option<f64>,
    pub max_speed: Option<f64>,
    pub source_dir: String,
    pub backed_up: bool,
    pub notes: String,
    pub clips: Vec<ClipResponse>,
}

#[derive(Debug, Serialize)]
pub struct ClipResponse {
    pub id: i64,
    pub event_id: i64,
    pub camera: String,
    pub file_path: String,
    pub file_size: i64,
    pub duration_sec: f64,
    pub has_sei: bool,
}

/// 掃描 TeslaCam 資料夾
#[tauri::command]
pub fn scan_directory(path: String, db: State<'_, Database>) -> Result<scanner::ScanResult, String> {
    let root = std::path::PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("路徑不存在: {}", path));
    }
    if !root.is_dir() {
        return Err(format!("不是目錄: {}", path));
    }

    Ok(scanner::scan_teslacam_dir(&root, &db))
}

/// 取得所有事件
#[tauri::command]
pub fn get_events(db: State<'_, Database>) -> Result<Vec<EventResponse>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, type, timestamp, duration_s, gps_lat, gps_lon, avg_speed, max_speed, source_dir, backed_up, notes
             FROM events ORDER BY timestamp DESC",
        )
        .map_err(|e| e.to_string())?;

    let events: Vec<EventResponse> = stmt
        .query_map([], |row| {
            Ok(EventResponse {
                id: row.get(0)?,
                event_type: row.get(1)?,
                timestamp: row.get(2)?,
                duration_sec: row.get::<_, Option<i64>>(3)?.unwrap_or(60),
                gps_lat: row.get(4)?,
                gps_lon: row.get(5)?,
                avg_speed: row.get(6)?,
                max_speed: row.get(7)?,
                source_dir: row.get(8)?,
                backed_up: row.get::<_, Option<i64>>(9)?.unwrap_or(0) != 0,
                notes: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                clips: Vec::new(), // 填入下方
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 載入每個事件的 clips
    let mut result = Vec::with_capacity(events.len());
    for mut event in events {
        let mut clip_stmt = conn
            .prepare(
                "SELECT id, event_id, camera, file_path, file_size, duration_s, has_sei
                 FROM clips WHERE event_id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let clips: Vec<ClipResponse> = clip_stmt
            .query_map([event.id], |row| {
                Ok(ClipResponse {
                    id: row.get(0)?,
                    event_id: row.get(1)?,
                    camera: row.get(2)?,
                    file_path: row.get(3)?,
                    file_size: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                    duration_sec: row.get::<_, Option<f64>>(5)?.unwrap_or(60.0),
                    has_sei: row.get::<_, Option<i64>>(6)?.unwrap_or(0) != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        event.clips = clips;
        result.push(event);
    }

    Ok(result)
}

/// 刪除事件（含所有片段和原始檔案）
#[tauri::command]
pub fn delete_event(event_id: i64, delete_files: bool, db: State<'_, Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    if delete_files {
        // 先取得所有檔案路徑
        let mut stmt = conn
            .prepare("SELECT file_path FROM clips WHERE event_id = ?1")
            .map_err(|e| e.to_string())?;

        let paths: Vec<String> = stmt
            .query_map([event_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        for path in &paths {
            if let Err(e) = std::fs::remove_file(path) {
                log::warn!("無法刪除檔案 {}: {}", path, e);
            }
        }
    }

    conn.execute("DELETE FROM clips WHERE event_id = ?1", [event_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM events WHERE id = ?1", [event_id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 備份事件到指定目錄
#[tauri::command]
pub fn backup_event(event_id: i64, target_dir: String, db: State<'_, Database>) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let target = std::path::PathBuf::from(&target_dir);
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT file_path FROM clips WHERE event_id = ?1")
        .map_err(|e| e.to_string())?;

    let paths: Vec<String> = stmt
        .query_map([event_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut copied = 0;
    for path in &paths {
        let src = std::path::Path::new(path);
        if let Some(filename) = src.file_name() {
            let dest = target.join(filename);
            if let Err(e) = std::fs::copy(src, &dest) {
                log::warn!("備份失敗 {}: {}", path, e);
            } else {
                copied += 1;
            }
        }
    }

    // 標記為已備份
    conn.execute(
        "UPDATE events SET backed_up = 1 WHERE id = ?1",
        [event_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(copied)
}
