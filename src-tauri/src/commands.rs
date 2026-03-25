use crate::analytics_engine;
use crate::db::Database;
use crate::event_detection;
use crate::scanner;
use crate::sei;
use crate::telemetry_overlay;
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
pub fn scan_directory(path: String, vehicle_id: Option<i64>, db: State<'_, Database>) -> Result<scanner::ScanResult, String> {
    let root = std::path::PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("路徑不存在: {}", path));
    }
    if !root.is_dir() {
        return Err(format!("不是目錄: {}", path));
    }

    Ok(scanner::scan_teslacam_dir(&root, &db, vehicle_id.unwrap_or(0)))
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

/// 取得所有車輛
#[tauri::command]
pub fn get_vehicles(db: State<'_, Database>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, root_path, created_at FROM vehicles ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let vehicles: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "rootPath": row.get::<_, String>(2)?,
                "createdAt": row.get::<_, String>(3)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(vehicles)
}

/// 新增車輛
#[tauri::command]
pub fn add_vehicle(name: String, root_path: String, db: State<'_, Database>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO vehicles (name, root_path) VALUES (?1, ?2)",
        rusqlite::params![name, root_path],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// 刪除車輛及其所有事件
#[tauri::command]
pub fn delete_vehicle(vehicle_id: i64, db: State<'_, Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM clips WHERE event_id IN (SELECT id FROM events WHERE vehicle_id = ?1)", [vehicle_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM events WHERE vehicle_id = ?1", [vehicle_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM vehicles WHERE id = ?1", [vehicle_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 生成事件報告 HTML（可列印為 PDF）
#[tauri::command]
pub fn generate_report(
    event_id: i64,
    db: State<'_, Database>,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (etype, timestamp, duration, source_dir): (String, String, i64, String) = conn
        .query_row(
            "SELECT type, timestamp, duration_s, source_dir FROM events WHERE id = ?1",
            [event_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, Option<i64>>(2)?.unwrap_or(0), row.get(3)?)),
        )
        .map_err(|e| e.to_string())?;

    let clip_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM clips WHERE event_id = ?1", [event_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let cam_count: i64 = conn
        .query_row("SELECT COUNT(DISTINCT camera) FROM clips WHERE event_id = ?1", [event_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let total_size: i64 = conn
        .query_row("SELECT COALESCE(SUM(file_size), 0) FROM clips WHERE event_id = ?1", [event_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let seg_count: i64 = conn
        .query_row("SELECT COUNT(DISTINCT segment_index) FROM clips WHERE event_id = ?1", [event_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    // 取得每段的時間戳（用 front 鏡頭的檔名推算）
    let mut seg_stmt = conn
        .prepare("SELECT DISTINCT segment_index, file_path, duration_s FROM clips WHERE event_id = ?1 AND camera = 'front' ORDER BY segment_index")
        .map_err(|e| e.to_string())?;
    let segments: Vec<(i64, String, f64)> = seg_stmt
        .query_map([event_id], |row| Ok((row.get(0)?, row.get::<_, String>(1)?, row.get::<_, Option<f64>>(2)?.unwrap_or(60.0))))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 從檔名提取時間戳
    let extract_ts = |path: &str| -> String {
        path.split('/').last().unwrap_or("")
            .split('-').take(6).collect::<Vec<_>>().join("-")
            .replace(".mp4", "")
            .replacen('_', " ", 1)
            .replacen('-', ":", 3) // 只替換時間部分的 -
    };

    // 哨兵事件的觸發時間 = source_dir 資料夾名
    let trigger_time = source_dir.split('/').last().unwrap_or("")
        .replacen('_', " ", 1)
        .replacen('-', ":", 3);

    let is_sentry = etype == "sentry";
    let type_label = match etype.as_str() {
        "sentry" => "哨兵模式",
        "saved" => "手動保存",
        _ => "行車紀錄",
    };
    let type_color = match etype.as_str() {
        "sentry" => "#e94560",
        "saved" => "#4ecdc4",
        _ => "#888",
    };

    let size_mb = total_size as f64 / (1024.0 * 1024.0);

    // 錄影時間軸
    let mut timeline_html = String::new();
    for (idx, (seg_idx, path, dur)) in segments.iter().enumerate() {
        let seg_ts = extract_ts(path);
        timeline_html.push_str(&format!(
            "<tr><td>片段 {}</td><td>{}</td><td>{:.1} 秒</td></tr>",
            seg_idx + 1, seg_ts, dur
        ));
        // 標記觸發點（哨兵的觸發時間通常是最後一段的開始附近）
        if is_sentry && idx == segments.len() - 1 {
            timeline_html.push_str(&format!(
                "<tr style=\"background:#fff0f0;font-weight:bold\"><td colspan=\"3\">⚠ 哨兵觸發時間點：{}</td></tr>",
                trigger_time
            ));
        }
    }

    // 遙測摘要（只對行車/手動保存有意義）
    let mut telemetry_section = String::new();
    let mut detected_section = String::new();

    let front_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM clips WHERE event_id = ?1 AND camera = 'front' AND segment_index = 0",
            [event_id],
            |row| row.get(0),
        )
        .ok();

    if let Some(path) = &front_path {
        if let Ok(frames) = sei::parse_sei_from_file(path) {
            let has_sei = !frames.is_empty();
            let has_driving = frames.iter().any(|f| f.speed_kmh > 1.0);

            if has_sei && has_driving {
                // 行車數據
                let speeds: Vec<f32> = frames.iter().map(|f| f.speed_kmh).collect();
                let max_speed = speeds.iter().cloned().fold(0.0f32, f32::max);
                let avg_speed = speeds.iter().sum::<f32>() / speeds.len() as f32;
                let brake_count = frames.iter().filter(|f| f.brake).count();

                telemetry_section = format!(
                    r#"<h2>駕駛數據摘要</h2>
<div class="summary">
  <div class="summary-card"><h3>最高車速</h3><div class="value">{max_speed:.0} km/h</div></div>
  <div class="summary-card"><h3>平均車速</h3><div class="value">{avg_speed:.0} km/h</div></div>
  <div class="summary-card"><h3>煞車次數</h3><div class="value">{brake_count}</div></div>
</div>"#
                );

                let detected = event_detection::detect_events(&frames);
                if !detected.is_empty() {
                    let mut rows = String::new();
                    for de in &detected {
                        let cls = match de.severity { 3 => "high", 2 => "medium", _ => "low" };
                        let m = (de.time_sec / 60.0) as u32;
                        let s = (de.time_sec % 60.0) as u32;
                        rows.push_str(&format!(
                            "<tr class=\"{cls}\"><td>{m}:{s:02}</td><td>{}</td><td>{}</td></tr>",
                            de.description, de.severity
                        ));
                    }
                    detected_section = format!(
                        "<h2>偵測到的事件</h2><table><tr><th>時間</th><th>描述</th><th>嚴重度</th></tr>{rows}</table>"
                    );
                }
            } else if has_sei && !has_driving {
                // 停車（哨兵）
                let first_gps = frames.iter().find(|f| f.lat != 0.0);
                let gps_text = first_gps
                    .map(|f| format!("{:.6}, {:.6}", f.lat, f.lon))
                    .unwrap_or_else(|| "無 GPS 資料".to_string());

                telemetry_section = format!(
                    r#"<h2>哨兵模式資訊</h2>
<div class="summary">
  <div class="summary-card"><h3>車輛狀態</h3><div class="value" style="font-size:20px">停車中 (P 檔)</div></div>
  <div class="summary-card"><h3>GPS 位置</h3><div class="value" style="font-size:16px">{gps_text}</div></div>
</div>
<p style="color:#888">哨兵模式在停車狀態下觸發錄影，無行車數據。觸發原因可能為偵測到周邊移動物體或碰撞。</p>"#
                );
            } else {
                telemetry_section = "<h2>遙測資料</h2><p style=\"color:#888\">此影片無 SEI 遙測資料（需韌體 2025.44.25+ / HW3+）</p>".to_string();
            }
        }
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-Hant">
<head>
<meta charset="UTF-8">
<title>TeslaCam 事件報告 — {type_label}</title>
<style>
  body {{ font-family: -apple-system, sans-serif; max-width: 800px; margin: 40px auto; color: #333; }}
  h1 {{ color: {type_color}; border-bottom: 2px solid {type_color}; padding-bottom: 8px; }}
  h2 {{ color: #555; margin-top: 28px; }}
  table {{ width: 100%; border-collapse: collapse; margin: 16px 0; }}
  th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
  th {{ background: #f5f5f5; }}
  .high {{ background: #fff0f0; }}
  .medium {{ background: #fffbf0; }}
  .summary {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin: 16px 0; }}
  .summary-card {{ background: #f9f9f9; border-radius: 8px; padding: 16px; }}
  .summary-card h3 {{ margin: 0 0 8px; font-size: 14px; color: #888; }}
  .summary-card .value {{ font-size: 28px; font-weight: 700; }}
  .badge {{ display: inline-block; padding: 2px 10px; border-radius: 4px; color: #fff; font-size: 13px; }}
  .footer {{ margin-top: 40px; font-size: 12px; color: #999; text-align: center; border-top: 1px solid #eee; padding-top: 16px; }}
</style>
</head>
<body>
<h1>TeslaCam 事件報告</h1>

<table>
<tr><th>事件類型</th><td><span class="badge" style="background:{type_color}">{type_label}</span></td></tr>
{sentry_trigger}
<tr><th>錄影起始時間</th><td>{timestamp}</td></tr>
<tr><th>總時長</th><td>{dur_min} 分 {dur_sec} 秒</td></tr>
<tr><th>鏡頭數</th><td>{cam_count}</td></tr>
<tr><th>片段數</th><td>{seg_count} 段（共 {clip_count} 個檔案）</td></tr>
<tr><th>檔案大小</th><td>{size_mb:.1} MB</td></tr>
<tr><th>來源路徑</th><td style="word-break:break-all;font-size:12px">{source_dir}</td></tr>
</table>

<h2>錄影時間軸</h2>
<table>
<tr><th>片段</th><th>起始時間</th><th>時長</th></tr>
{timeline_html}
</table>

{telemetry_section}
{detected_section}

<div class="footer">
  由 TeslaCam Manager 自動生成 · {gen_time}
</div>
</body></html>"#,
        sentry_trigger = if is_sentry {
            format!("<tr><th>⚠ 哨兵觸發時間</th><td style=\"color:#e94560;font-weight:bold\">{trigger_time}</td></tr>")
        } else { String::new() },
        dur_min = duration / 60,
        dur_sec = duration % 60,
        gen_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    Ok(html)
}

/// 偵測影片中的駕駛事件（急煞車、急轉彎、倒車等）
#[tauri::command]
pub fn detect_events(file_path: String) -> Result<Vec<event_detection::DetectedEvent>, String> {
    let raw_frames = sei::parse_sei_from_file(&file_path)?;
    Ok(event_detection::detect_events(&raw_frames))
}

/// 單段匯出：把一個 segment 的六鏡頭合併成環景影片
fn export_one_segment(
    cam_map: &std::collections::HashMap<String, String>,
    trim_start: f64,
    trim_end: f64,
    real_start_epoch: f64,
    telemetry_frames: Option<&[sei::TelemetryFrame]>,
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

    // 顯示標準時間：pts + epoch → localtime（預設格式 YYYY-MM-DD HH:MM:SS）
    let epoch_sec = real_start_epoch as i64;
    fp.push(format!(
        "[out]drawtext=text='%{{pts\\:localtime\\:{epoch_sec}}}':fontsize=24:fontcolor=white:borderw=2:bordercolor=black:x=10:y=10[timetext]"
    ));

    // 如果有遙測資料，生成 ASS 字幕並疊加
    let tmp_ass;
    let video_w = if n >= 6 { cw * 3 } else { cw * 2 };
    let video_h = ch * 2;

    if let Some(frames) = telemetry_frames {
        tmp_ass = std::env::temp_dir().join(format!("teslacam_tele_{}.ass", std::process::id()));
        telemetry_overlay::generate_ass_overlay(
            frames, trim_start, trim_end, &tmp_ass, video_w as u32, video_h as u32,
        )?;
        let ass_path = tmp_ass.to_string_lossy().to_string().replace('\\', "/").replace(':', "\\:");
        fp.push(format!("[timetext]ass='{ass_path}'[final]"));
    } else {
        tmp_ass = std::path::PathBuf::new();
        // 沒有遙測就直接用時間戳作為最終輸出
        let last = fp.last_mut().unwrap();
        *last = last.replace("[timetext]", "[final]");
    }

    let filter = fp.join(";");

    let output = std::process::Command::new("ffmpeg")
        .args(&input_args)
        .args(&["-filter_complex", &filter, "-map", "[final]",
               "-c:v", "libx264", "-preset", "fast", "-crf", "23", "-y", output_path])
        .output()
        .map_err(|e| format!("ffmpeg 失敗: {}", e))?;

    // 清理暫存 ASS
    if tmp_ass.exists() {
        std::fs::remove_file(&tmp_ass).ok();
    }

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
    with_telemetry: Option<bool>,
    db: State<'_, Database>,
) -> Result<String, String> {
    let show_telemetry = with_telemetry.unwrap_or(true);
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

    // 讀取每段的遙測資料（如果需要）
    let seg_telemetry: Vec<Option<Vec<sei::TelemetryFrame>>> = if show_telemetry {
        segments.iter().map(|(_, cams, _)| {
            cams.get("front")
                .and_then(|path| sei::parse_sei_from_file(path).ok())
        }).collect()
    } else {
        segments.iter().map(|_| None).collect()
    };

    if parts.len() == 1 {
        let (seg_i, ts, te) = parts[0];
        let real_epoch = event_epoch + seg_starts[seg_i] + ts;
        let tele = seg_telemetry[seg_i].as_deref();
        export_one_segment(&segments[seg_i].1, ts, te, real_epoch, tele, &output_path)?;
    } else {
        let tmp_dir = std::env::temp_dir().join("teslacam_export");
        std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

        let mut tmp_files = Vec::new();
        for (idx, (seg_i, ts, te)) in parts.iter().enumerate() {
            let tmp_path = tmp_dir.join(format!("part_{idx}.mp4"));
            let tmp_str = tmp_path.to_string_lossy().to_string();
            let real_epoch = event_epoch + seg_starts[*seg_i] + ts;
            let tele = seg_telemetry[*seg_i].as_deref();
            log::info!("  段 {}: trim {:.3}-{:.3} → {}", seg_i, ts, te, tmp_str);
            export_one_segment(&segments[*seg_i].1, *ts, *te, real_epoch, tele, &tmp_str)?;
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

// ── Analytics Commands ─────────────────────────────────────

/// 計算車輛的駕駛分析數據
#[tauri::command]
pub fn compute_analytics(vehicle_id: i64, db: State<'_, Database>) -> Result<usize, String> {
    analytics_engine::compute_analytics(vehicle_id, &db)
}

/// 取得行程列表
#[tauri::command]
pub fn get_trips(
    vehicle_id: i64,
    date_from: Option<String>,
    date_to: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<analytics_engine::TripInfo>, String> {
    analytics_engine::get_trips(&db, vehicle_id, date_from.as_deref(), date_to.as_deref())
}

/// 取得每日統計
#[tauri::command]
pub fn get_daily_stats(
    vehicle_id: i64,
    date_from: String,
    date_to: String,
    db: State<'_, Database>,
) -> Result<Vec<analytics_engine::DailyStat>, String> {
    analytics_engine::get_daily_stats(&db, vehicle_id, &date_from, &date_to)
}

/// 取得期間摘要（含與前期比較）
#[tauri::command]
pub fn get_period_summary(
    vehicle_id: i64,
    period: String,
    db: State<'_, Database>,
) -> Result<analytics_engine::PeriodSummary, String> {
    analytics_engine::get_period_summary(&db, vehicle_id, &period)
}

/// 取得熱力圖 GPS 資料
#[tauri::command]
pub fn get_heatmap_data(
    vehicle_id: i64,
    date_from: Option<String>,
    date_to: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<analytics_engine::HeatmapPoint>, String> {
    analytics_engine::get_heatmap_data(&db, vehicle_id, date_from.as_deref(), date_to.as_deref())
}
