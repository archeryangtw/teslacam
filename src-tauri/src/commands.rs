use crate::db::Database;
use crate::scanner;
use crate::sei;
use chrono::TimeZone;
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
    pub segment_index: i64,
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
                "SELECT id, event_id, camera, file_path, file_size, duration_s, has_sei, segment_index
                 FROM clips WHERE event_id = ?1 ORDER BY segment_index, camera",
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
                    segment_index: row.get::<_, Option<i64>>(7)?.unwrap_or(0),
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

/// 解析影片的 SEI 遙測資料（每幀帶精確影片時間）
#[tauri::command]
pub fn parse_telemetry(file_path: String) -> Result<Vec<sei::TelemetryFrame>, String> {
    let raw_frames = sei::parse_sei_from_file(&file_path)?;
    Ok(sei::downsample_by_time(&raw_frames, 0.15))
}

/// 單段匯出：把一個 segment 的六鏡頭合併成環景影片
/// `real_start_epoch` = 該段 trim_start 對應的 Unix 時間戳（秒）
fn export_one_segment(
    cam_map: &std::collections::HashMap<String, String>,
    trim_start: f64,
    trim_end: f64,
    real_start_epoch: f64,
    output_path: &str,
) -> Result<(), String> {
    let cameras = [
        "left_pillar", "front", "right_pillar",
        "left_repeater", "back", "right_repeater",
    ];
    let mut input_args = Vec::new();
    let mut input_cams = Vec::new();

    for cam in &cameras {
        if let Some(path) = cam_map.get(*cam) {
            input_args.push("-i".to_string());
            input_args.push(path.clone());
            input_cams.push(*cam);
        }
    }

    if input_cams.len() < 4 {
        return Err("至少需要 4 個鏡頭".to_string());
    }

    let n = input_cams.len();
    let (cw, ch) = (640, 480);
    let mut fp = Vec::new();

    for i in 0..n {
        let trim = format!("trim=start={trim_start:.3}:end={trim_end:.3},setpts=PTS-STARTPTS");
        if input_cams[i] == "back" {
            fp.push(format!("[{i}:v]{trim},scale={cw}:{ch},hflip[v{i}]"));
        } else {
            fp.push(format!("[{i}:v]{trim},scale={cw}:{ch}[v{i}]"));
        }
    }

    if n >= 6 {
        fp.push("[v0][v1][v2]hstack=inputs=3[top]".into());
        fp.push("[v3][v4][v5]hstack=inputs=3[bottom]".into());
    } else {
        fp.push("[v0][v1]hstack=inputs=2[top]".into());
        fp.push("[v2][v3]hstack=inputs=2[bottom]".into());
    }
    fp.push("[top][bottom]vstack=inputs=2[out]".into());

    // 顯示標準時間：basetime（微秒）+ pts 自動遞增
    let basetime_us = (real_start_epoch * 1_000_000.0) as i64;
    fp.push(format!(
        "[out]drawtext=basetime={basetime_us}:text='%{{localtime\\:%Y-%m-%d %H\\:%M\\:%S}}':fontsize=24:fontcolor=white:borderw=2:bordercolor=black:x=10:y=10[final]"
    ));

    let filter = fp.join(";");

    let output = std::process::Command::new("ffmpeg")
        .args(&input_args)
        .args(&["-filter_complex", &filter, "-map", "[final]",
               "-c:v", "libx264", "-preset", "fast", "-crf", "23", "-y", output_path])
        .output()
        .map_err(|e| format!("ffmpeg 失敗: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg 錯誤: {}", stderr.chars().take(500).collect::<String>()));
    }
    Ok(())
}

/// 匯出六鏡頭合併影片，支援跨段時間範圍
#[tauri::command]
pub async fn export_surround_video(
    event_id: i64,
    output_path: String,
    start_time: Option<f64>,
    end_time: Option<f64>,
    db: State<'_, Database>,
) -> Result<String, String> {
    // 讀取事件時間戳和所有 segment
    let (event_timestamp, segments): (String, Vec<(i64, std::collections::HashMap<String, String>, f64)>) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        // 取事件時間戳
        let ts: String = conn
            .query_row("SELECT timestamp FROM events WHERE id = ?1", [event_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT segment_index, camera, file_path, duration_s FROM clips WHERE event_id = ?1 ORDER BY segment_index, camera")
            .map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String, String, f64)> = stmt
            .query_map([event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get::<_, Option<f64>>(3)?.unwrap_or(60.0))))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut seg_map: std::collections::BTreeMap<i64, (std::collections::HashMap<String, String>, f64)> = std::collections::BTreeMap::new();
        for (si, cam, path, dur) in rows {
            let e = seg_map.entry(si).or_insert_with(|| (std::collections::HashMap::new(), dur));
            e.0.insert(cam, path);
        }
        (ts, seg_map.into_iter().map(|(i, (c, d))| (i, c, d)).collect())
    };

    if segments.is_empty() {
        return Err("找不到影片片段".to_string());
    }

    // 解析事件時間戳為 Unix epoch（秒）
    // 格式："2026-03-23T13:01:18" — 視為本地時間
    let event_epoch = chrono::NaiveDateTime::parse_from_str(&event_timestamp, "%Y-%m-%dT%H:%M:%S")
        .map(|dt| {
            let local = chrono::Local::now().timezone();
            dt.and_local_timezone(local)
                .single()
                .map(|t| t.timestamp() as f64)
                .unwrap_or(0.0)
        })
        .unwrap_or(0.0);

    // 計算每段的累積起始時間
    let total_dur: f64 = segments.iter().map(|(_, _, d)| d).sum();
    let ss = start_time.unwrap_or(0.0).max(0.0);
    let ee = end_time.unwrap_or(total_dur).min(total_dur);

    let mut seg_starts = Vec::new();
    let mut acc = 0.0f64;
    for (_, _, dur) in &segments {
        seg_starts.push(acc);
        acc += dur;
    }

    // 收集需要匯出的片段：(segment_index, trim_start, trim_end)
    let mut parts: Vec<(usize, f64, f64)> = Vec::new();
    for (i, (_, _, dur)) in segments.iter().enumerate() {
        let seg_begin = seg_starts[i];
        let seg_end_time = seg_begin + dur;

        // 此 segment 與選取範圍有交集嗎？
        if seg_end_time <= ss || seg_begin >= ee {
            continue;
        }

        let trim_start = if ss > seg_begin { ss - seg_begin } else { 0.0 };
        let trim_end = if ee < seg_end_time { ee - seg_begin } else { *dur };
        parts.push((i, trim_start, trim_end));
    }

    if parts.is_empty() {
        return Err("選取的時間範圍內沒有影片".to_string());
    }

    log::info!("匯出: {:.1}s-{:.1}s, 共 {} 段", ss, ee, parts.len());

    if parts.len() == 1 {
        let (seg_i, ts, te) = parts[0];
        let real_epoch = event_epoch + seg_starts[seg_i] + ts;
        export_one_segment(&segments[seg_i].1, ts, te, real_epoch, &output_path)?;
    } else {
        // 多段：各段匯出暫存檔 → concat 合併
        let tmp_dir = std::env::temp_dir().join("teslacam_export");
        std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

        let mut tmp_files = Vec::new();
        for (idx, (seg_i, ts, te)) in parts.iter().enumerate() {
            let tmp_path = tmp_dir.join(format!("part_{idx}.mp4"));
            let tmp_str = tmp_path.to_string_lossy().to_string();
            let real_epoch = event_epoch + seg_starts[*seg_i] + ts;
            log::info!("  段 {}: trim {:.3}-{:.3} → {}", seg_i, ts, te, tmp_str);
            export_one_segment(&segments[*seg_i].1, *ts, *te, real_epoch, &tmp_str)?;
            tmp_files.push(tmp_str);
        }

        // 建立 concat 清單檔
        let list_path = tmp_dir.join("concat_list.txt");
        let list_content: String = tmp_files
            .iter()
            .map(|f| format!("file '{}'", f))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&list_path, &list_content).map_err(|e| e.to_string())?;

        // ffmpeg concat
        let output = std::process::Command::new("ffmpeg")
            .args(&[
                "-f", "concat", "-safe", "0",
                "-i", &list_path.to_string_lossy(),
                "-c", "copy", "-y", &output_path,
            ])
            .output()
            .map_err(|e| format!("concat 失敗: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // 清理暫存
            std::fs::remove_dir_all(&tmp_dir).ok();
            return Err(format!("concat 錯誤: {}", stderr.chars().take(500).collect::<String>()));
        }

        // 清理暫存
        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    Ok(output_path)
}
