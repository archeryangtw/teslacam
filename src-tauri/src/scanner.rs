use crate::db::Database;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
fn parse_clip_filename(filename: &str) -> Option<(String, String)> {
    let re = Regex::new(
        r"^(\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})-(front|back|left_repeater|right_repeater|left_pillar|right_pillar)\.mp4$"
    ).ok()?;
    let caps = re.captures(filename)?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

/// 從 MP4 的 moov/mvhd atom 讀取真實時長（秒）
fn read_mp4_duration(path: &Path) -> Option<f64> {
    let mut fp = File::open(path).ok()?;
    let file_size = fp.seek(SeekFrom::End(0)).ok()?;
    fp.seek(SeekFrom::Start(0)).ok()?;

    // 遍歷頂層 atom 找 moov
    while fp.stream_position().ok()? < file_size {
        let mut header = [0u8; 8];
        if fp.read_exact(&mut header).is_err() {
            break;
        }
        let size32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let atom_type = &header[4..8];

        let atom_size = if size32 == 1 {
            let mut ext = [0u8; 8];
            fp.read_exact(&mut ext).ok()?;
            u64::from_be_bytes(ext)
        } else {
            size32 as u64
        };
        let header_size = if size32 == 1 { 16u64 } else { 8u64 };

        if atom_type == b"moov" {
            // 讀取整個 moov 搜尋 mvhd
            let moov_size = (atom_size - header_size) as usize;
            let mut moov_data = vec![0u8; moov_size];
            fp.read_exact(&mut moov_data).ok()?;

            // 搜尋 "mvhd"
            for i in 0..moov_data.len().saturating_sub(20) {
                if &moov_data[i..i + 4] == b"mvhd" {
                    let version = moov_data[i + 4];
                    if version == 0 && i + 24 <= moov_data.len() {
                        let timescale = u32::from_be_bytes([
                            moov_data[i + 16],
                            moov_data[i + 17],
                            moov_data[i + 18],
                            moov_data[i + 19],
                        ]);
                        let duration = u32::from_be_bytes([
                            moov_data[i + 20],
                            moov_data[i + 21],
                            moov_data[i + 22],
                            moov_data[i + 23],
                        ]);
                        if timescale > 0 {
                            return Some(duration as f64 / timescale as f64);
                        }
                    } else if version == 1 && i + 36 <= moov_data.len() {
                        let timescale = u32::from_be_bytes([
                            moov_data[i + 24],
                            moov_data[i + 25],
                            moov_data[i + 26],
                            moov_data[i + 27],
                        ]);
                        let duration = u64::from_be_bytes([
                            moov_data[i + 28],
                            moov_data[i + 29],
                            moov_data[i + 30],
                            moov_data[i + 31],
                            moov_data[i + 32],
                            moov_data[i + 33],
                            moov_data[i + 34],
                            moov_data[i + 35],
                        ]);
                        if timescale > 0 {
                            return Some(duration as f64 / timescale as f64);
                        }
                    }
                    break;
                }
            }
            break;
        }

        if atom_size < header_size {
            break;
        }
        fp.seek(SeekFrom::Current((atom_size - header_size) as i64)).ok()?;
    }
    None
}

/// 公開版本供 commands 使用
pub fn read_mp4_duration_pub(path: &str) -> Option<f64> {
    read_mp4_duration(std::path::Path::new(path))
}

/// "2026-03-22_20-34-32" → "2026-03-22T20:34:32"
fn timestamp_to_iso(ts: &str) -> String {
    ts.replacen('_', "T", 1)
        .replace('-', ":")
        .replacen(':', "-", 2)
}

fn detect_event_type(path: &Path, root: &Path) -> &'static str {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let first = rel
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default();
    match first.as_str() {
        "SentryClips" => "sentry",
        "SavedClips" => "saved",
        _ => "recent",
    }
}

/// 一個片段（同一時間戳的多個鏡頭）
struct Segment {
    timestamp: String,
    clips: Vec<(String, PathBuf, u64)>, // (camera, path, size)
}

pub fn scan_teslacam_dir(root: &Path, db: &Database, vehicle_id: i64) -> ScanResult {
    let mut errors = Vec::new();
    let mut total_clips = 0u64;
    let mut total_size = 0u64;

    // 第一步：收集所有 MP4，按 (event_key, timestamp) 分組
    // event_key: Sentry/Saved = 事件資料夾路徑, Recent = 每個時間戳獨立
    // 用 BTreeMap 讓 timestamp 自動排序
    let mut event_map: HashMap<String, BTreeMap<String, Vec<(String, PathBuf, u64)>>> =
        HashMap::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.to_string_lossy().contains("EncryptedClips") {
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f,
            None => continue,
        };
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

        let event_type = detect_event_type(path, root);

        // Sentry/Saved: 同一事件資料夾 → 同一事件（多個時間段）
        // Recent: 每個時間戳獨立一個事件
        let event_key = if event_type == "recent" {
            format!("recent::{}", ts)
        } else {
            path.parent()
                .unwrap_or(root)
                .to_string_lossy()
                .to_string()
        };

        event_map
            .entry(event_key)
            .or_default()
            .entry(ts)
            .or_default()
            .push((camera, path.to_path_buf(), file_size));
    }

    // 第二步：寫入資料庫
    let conn = db.conn.lock().unwrap();
    conn.execute_batch("DELETE FROM telemetry; DELETE FROM clips; DELETE FROM events;")
        .ok();

    let mut sentry_count = 0usize;
    let mut saved_count = 0usize;
    let mut recent_count = 0usize;

    for (event_key, segments_map) in &event_map {
        // 收集所有片段（已按時間排序，BTreeMap 保證）
        let segments: Vec<Segment> = segments_map
            .iter()
            .map(|(ts, clips)| Segment {
                timestamp: ts.clone(),
                clips: clips.clone(),
            })
            .collect();

        if segments.is_empty() {
            continue;
        }

        let first_ts = &segments[0].timestamp;
        let iso_ts = timestamp_to_iso(first_ts);
        let total_duration = segments.len() as i64 * 60; // 每段約 60 秒

        let event_type = if event_key.contains("SentryClips") {
            "sentry"
        } else if event_key.contains("SavedClips") {
            "saved"
        } else {
            "recent"
        };

        let source_dir = if event_key.starts_with("recent::") {
            root.join("RecentClips").to_string_lossy().to_string()
        } else {
            event_key.clone()
        };

        match event_type {
            "sentry" => sentry_count += 1,
            "saved" => saved_count += 1,
            _ => recent_count += 1,
        }

        let result = conn.execute(
            "INSERT INTO events (vehicle_id, type, timestamp, duration_s, source_dir) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![vehicle_id, event_type, iso_ts, total_duration, source_dir],
        );

        let event_id = match result {
            Ok(_) => conn.last_insert_rowid(),
            Err(e) => {
                errors.push(format!("資料庫錯誤: {}", e));
                continue;
            }
        };

        // 插入所有片段，segment_index 表示在事件中的順序
        // 讀取真實時長用於精確的時間同步
        let mut real_total_duration = 0.0f64;
        for (seg_idx, segment) in segments.iter().enumerate() {
            // 用同一 segment 中任意一個檔案讀取真實時長
            let real_duration = segment
                .clips
                .first()
                .and_then(|(_, path, _)| read_mp4_duration(path))
                .unwrap_or(60.0);
            real_total_duration += real_duration;

            for (camera, file_path, file_size) in &segment.clips {
                conn.execute(
                    "INSERT INTO clips (event_id, camera, file_path, file_size, duration_s, segment_index) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        event_id,
                        camera,
                        file_path.to_string_lossy().to_string(),
                        file_size,
                        real_duration,
                        seg_idx as i64
                    ],
                )
                .ok();
            }
        }

        // 更新事件總時長為真實時長
        conn.execute(
            "UPDATE events SET duration_s = ?1 WHERE id = ?2",
            rusqlite::params![real_total_duration as i64, event_id],
        )
        .ok();
    }

    ScanResult {
        total_events: event_map.len(),
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
    }

    #[test]
    fn test_timestamp_to_iso() {
        assert_eq!(
            timestamp_to_iso("2026-03-22_20-34-32"),
            "2026-03-22T20:34:32"
        );
    }
}
