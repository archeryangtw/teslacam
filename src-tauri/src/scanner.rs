use crate::db::Database;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 掃描結果
#[derive(Debug, Serialize, Clone)]
pub struct ScanResult {
    pub total_events: usize,
    pub sentry_count: usize,
    pub saved_count: usize,
    pub recent_count: usize,
    pub total_clips: usize,
    pub total_size_bytes: u64,
    pub errors: Vec<String>,
}

/// 解析檔名，例如 "2026-03-22_20-34-32-front.mp4"
/// 回傳 (timestamp_str, camera_angle)
fn parse_clip_filename(filename: &str) -> Option<(String, String)> {
    let re = Regex::new(
        r"^(\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})-(front|back|left_repeater|right_repeater|left_pillar|right_pillar)\.mp4$"
    ).ok()?;

    let caps = re.captures(filename)?;
    let ts = caps.get(1)?.as_str().to_string();
    let cam = caps.get(2)?.as_str().to_string();
    Some((ts, cam))
}

/// 將 "2026-03-22_20-34-32" 轉為 ISO 8601 "2026-03-22T20:34:32"
fn timestamp_to_iso(ts: &str) -> String {
    ts.replacen('_', "T", 1)
        .replace('-', ":")
        .replacen(':', "-", 2)
}

/// 偵測事件類型（根據所在資料夾）
fn detect_event_type(path: &Path, root: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let first_component = rel
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default();

    match first_component.as_str() {
        "SentryClips" => "sentry".to_string(),
        "SavedClips" => "saved".to_string(),
        "RecentClips" => "recent".to_string(),
        _ => "recent".to_string(),
    }
}

/// 掃描 TeslaCam 根目錄，建立事件索引
pub fn scan_teslacam_dir(root: &Path, db: &Database) -> ScanResult {
    let mut errors = Vec::new();
    let mut total_clips = 0u64;
    let mut total_size = 0u64;

    // 收集所有 MP4 檔案，按 (event_type, timestamp) 分組
    // key = (source_dir, timestamp), value = Vec<(camera, file_path, file_size)>
    let mut event_groups: HashMap<(String, String), Vec<(String, PathBuf, u64)>> = HashMap::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // 跳過 EncryptedClips
        if path
            .to_string_lossy()
            .contains("EncryptedClips")
        {
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f,
            None => continue,
        };

        // 跳過非 MP4 和縮圖
        if !filename.ends_with(".mp4") {
            continue;
        }

        let (ts, camera) = match parse_clip_filename(filename) {
            Some(v) => v,
            None => {
                errors.push(format!("無法解析檔名: {}", filename));
                continue;
            }
        };

        let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        total_size += file_size;
        total_clips += 1;

        // source_dir: 對 Sentry/Saved 用事件資料夾，對 Recent 用 RecentClips
        let event_type = detect_event_type(path, root);
        let source_dir = if event_type == "recent" {
            root.join("RecentClips").to_string_lossy().to_string()
        } else {
            path.parent()
                .unwrap_or(root)
                .to_string_lossy()
                .to_string()
        };

        let key = (source_dir, ts.clone());
        event_groups
            .entry(key)
            .or_default()
            .push((camera, path.to_path_buf(), file_size));
    }

    // 寫入資料庫
    let conn = db.conn.lock().unwrap();

    // 清除舊資料（重新掃描）
    conn.execute_batch("DELETE FROM telemetry; DELETE FROM clips; DELETE FROM events;")
        .ok();

    let mut sentry_count = 0usize;
    let mut saved_count = 0usize;
    let mut recent_count = 0usize;

    for ((source_dir, ts), clips) in &event_groups {
        let iso_ts = timestamp_to_iso(ts);
        let event_type = if source_dir.contains("SentryClips") {
            "sentry"
        } else if source_dir.contains("SavedClips") {
            "saved"
        } else {
            "recent"
        };

        match event_type {
            "sentry" => sentry_count += 1,
            "saved" => saved_count += 1,
            _ => recent_count += 1,
        }

        // 插入事件
        let result = conn.execute(
            "INSERT INTO events (type, timestamp, duration_s, source_dir) VALUES (?1, ?2, 60, ?3)",
            rusqlite::params![event_type, iso_ts, source_dir],
        );

        let event_id = match result {
            Ok(_) => conn.last_insert_rowid(),
            Err(e) => {
                errors.push(format!("資料庫錯誤: {}", e));
                continue;
            }
        };

        // 插入片段
        for (camera, file_path, file_size) in clips {
            conn.execute(
                "INSERT INTO clips (event_id, camera, file_path, file_size, duration_s) VALUES (?1, ?2, ?3, ?4, 60.0)",
                rusqlite::params![event_id, camera, file_path.to_string_lossy().to_string(), file_size],
            ).ok();
        }
    }

    ScanResult {
        total_events: event_groups.len(),
        sentry_count,
        saved_count,
        recent_count,
        total_clips: total_clips as usize,
        total_size_bytes: total_size,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clip_filename() {
        let (ts, cam) = parse_clip_filename("2026-03-22_20-34-32-front.mp4").unwrap();
        assert_eq!(ts, "2026-03-22_20-34-32");
        assert_eq!(cam, "front");
    }

    #[test]
    fn test_parse_clip_filename_left_repeater() {
        let (ts, cam) = parse_clip_filename("2026-03-22_20-34-32-left_repeater.mp4").unwrap();
        assert_eq!(ts, "2026-03-22_20-34-32");
        assert_eq!(cam, "left_repeater");
    }

    #[test]
    fn test_parse_clip_filename_invalid() {
        assert!(parse_clip_filename("random-file.mp4").is_none());
        assert!(parse_clip_filename("thumb.png").is_none());
    }

    #[test]
    fn test_timestamp_to_iso() {
        assert_eq!(timestamp_to_iso("2026-03-22_20-34-32"), "2026-03-22T20:34:32");
    }
}
