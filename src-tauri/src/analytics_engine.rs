use crate::db::Database;
use crate::event_detection;
use crate::sei::{self, TelemetryFrame};
use chrono::Datelike;
use rusqlite::params;
use serde::Serialize;

// ── 回傳型別 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct TripInfo {
    pub id: i64,
    pub vehicle_id: i64,
    pub start_time: String,
    pub end_time: String,
    pub duration_sec: f64,
    pub distance_km: f64,
    pub avg_speed_kmh: f64,
    pub max_speed_kmh: f64,
    pub event_count: i64,
    pub hard_brake_count: i64,
    pub hard_accel_count: i64,
    pub sharp_turn_count: i64,
    pub autopilot_pct: f64,
    pub driving_score: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyStat {
    pub date: String,
    pub trip_count: i64,
    pub total_distance_km: f64,
    pub total_duration_sec: f64,
    pub avg_speed_kmh: f64,
    pub max_speed_kmh: f64,
    pub event_count: i64,
    pub driving_score: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeriodSummary {
    pub total_distance_km: f64,
    pub total_duration_sec: f64,
    pub trip_count: i64,
    pub event_count: i64,
    pub driving_score: i64,
    pub prev_distance_km: Option<f64>,
    pub prev_duration_sec: Option<f64>,
    pub prev_trip_count: Option<i64>,
    pub prev_event_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapPoint {
    pub lat: f64,
    pub lon: f64,
    pub speed_kmh: f64,
}

// ── 駕駛評分 ──────────────────────────────────────────────

fn compute_driving_score(
    hard_brakes: i64,
    hard_accels: i64,
    sharp_turns: i64,
    speed_exceed_pct: f64,
    distance_km: f64,
) -> i64 {
    if distance_km < 0.1 {
        return 100;
    }
    let per_100km = 100.0 / distance_km;

    let score_brake = rate_to_score((hard_brakes as f64) * per_100km);
    let score_accel = rate_to_score((hard_accels as f64) * per_100km);
    let score_turn = rate_to_score((sharp_turns as f64) * per_100km);
    let score_speed = pct_to_score(speed_exceed_pct);

    (score_brake + score_accel + score_turn + score_speed).min(100).max(20)
}

fn rate_to_score(per_100km: f64) -> i64 {
    if per_100km < 0.5 { 25 }
    else if per_100km < 3.0 { 22 }
    else if per_100km < 6.0 { 18 }
    else if per_100km < 11.0 { 12 }
    else { 5 }
}

fn pct_to_score(pct: f64) -> i64 {
    if pct < 0.5 { 25 }
    else if pct < 5.0 { 22 }
    else if pct < 15.0 { 18 }
    else if pct < 30.0 { 12 }
    else { 5 }
}

// ── GPS 距離計算 ──────────────────────────────────────────

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // km
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

/// 計算一個 trip 的距離（GPS + 速度交叉驗證）
fn compute_distance(frames: &[TelemetryFrame]) -> f64 {
    if frames.len() < 2 {
        return 0.0;
    }

    // GPS 方法：每秒取一個有效 GPS 點
    let mut gps_points: Vec<(f64, f64, f64)> = Vec::new(); // (lat, lon, time)
    let mut last_gps_time = -1.0_f64;

    for f in frames {
        if f.lat == 0.0 && f.lon == 0.0 {
            continue;
        }
        if f.time_sec - last_gps_time < 1.0 {
            continue;
        }
        // 過濾 GPS 跳躍（暗示速度 > 200 km/h 的位移）
        if let Some(last) = gps_points.last() {
            let dt = f.time_sec - last.2;
            if dt > 0.0 {
                let dist = haversine_km(last.0, last.1, f.lat, f.lon);
                let implied_speed = dist / (dt / 3600.0);
                if implied_speed > 200.0 {
                    continue;
                }
            }
        }
        gps_points.push((f.lat, f.lon, f.time_sec));
        last_gps_time = f.time_sec;
    }

    let gps_distance: f64 = gps_points.windows(2)
        .map(|w| haversine_km(w[0].0, w[0].1, w[1].0, w[1].1))
        .sum();

    // 速度積分方法
    let speed_distance: f64 = frames.windows(2)
        .map(|w| {
            let dt = w[1].time_sec - w[0].time_sec;
            if dt <= 0.0 { return 0.0; }
            let avg_speed = (w[0].speed_kmh + w[1].speed_kmh) as f64 / 2.0;
            avg_speed * (dt / 3600.0)
        })
        .sum();

    // 交叉驗證：差異 > 20% 時用速度積分
    if gps_distance > 0.01 && speed_distance > 0.01 {
        let diff_pct = ((gps_distance - speed_distance) / speed_distance).abs() * 100.0;
        if diff_pct > 20.0 {
            speed_distance
        } else {
            gps_distance
        }
    } else if speed_distance > 0.01 {
        speed_distance
    } else {
        gps_distance
    }
}

// ── 分析主流程 ──────────────────────────────────────────

/// 計算單一車輛的所有分析數據
pub fn compute_analytics(vehicle_id: i64, db: &Database) -> Result<usize, String> {
    // 取得所有 recent 類型事件（含 clips）
    let events: Vec<(i64, String, f64, Vec<String>)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        // 跳過 vehicle_id = 0
        if vehicle_id == 0 {
            return Err("vehicle_id 為 0，請先關聯車輛".to_string());
        }

        // 清除舊的分析資料
        conn.execute("DELETE FROM telemetry_samples WHERE trip_id IN (SELECT id FROM trips WHERE vehicle_id = ?1)", params![vehicle_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM trips WHERE vehicle_id = ?1", params![vehicle_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daily_stats WHERE vehicle_id = ?1", params![vehicle_id])
            .map_err(|e| e.to_string())?;

        // 取得 recent 事件及其 front 鏡頭 clips
        let mut stmt = conn.prepare(
            "SELECT e.id, e.timestamp, COALESCE(e.duration_s, 0)
             FROM events e
             WHERE e.vehicle_id = ?1 AND e.type = 'recent'
             ORDER BY e.timestamp"
        ).map_err(|e| e.to_string())?;

        let rows: Vec<(i64, String, f64)> = stmt.query_map(params![vehicle_id], |row| {
            Ok((row.get(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?))
        }).map_err(|e| e.to_string())?
          .filter_map(|r| r.ok())
          .collect();

        let mut result = Vec::new();
        for (event_id, timestamp, duration) in rows {
            let mut clip_stmt = conn.prepare(
                "SELECT file_path FROM clips WHERE event_id = ?1 AND camera = 'front' ORDER BY segment_index"
            ).map_err(|e| e.to_string())?;

            let paths: Vec<String> = clip_stmt.query_map(params![event_id], |row| row.get(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            if !paths.is_empty() {
                result.push((event_id, timestamp, duration, paths));
            }
        }
        result
    };

    let total_events = events.len();
    let mut trip_count = 0;

    for (event_id, timestamp, _duration, front_paths) in &events {
        // 解析所有 segment 的遙測資料
        let mut all_frames: Vec<TelemetryFrame> = Vec::new();
        let mut time_offset = 0.0_f64;

        for path in front_paths {
            match sei::parse_sei_from_file(path) {
                Ok(frames) => {
                    for mut f in frames {
                        f.time_sec += time_offset;
                        all_frames.push(f);
                    }
                    // 取此片段最後一幀的時間作為 offset
                    if let Some(last) = all_frames.last() {
                        time_offset = last.time_sec + 0.033; // ~30fps
                    }
                }
                Err(_) => continue,
            }
        }

        if all_frames.is_empty() {
            continue;
        }

        // Trip 過濾：至少有 gear != P 且 speed > 1 的幀累計 10 秒
        let driving_time: f64 = {
            let mut total = 0.0_f64;
            for w in all_frames.windows(2) {
                if w[0].gear != "P" && w[0].speed_kmh > 1.0 {
                    let dt = w[1].time_sec - w[0].time_sec;
                    if dt > 0.0 && dt < 5.0 {
                        total += dt;
                    }
                }
            }
            total
        };

        if driving_time < 10.0 {
            continue;
        }

        // 計算統計
        let distance_km = compute_distance(&all_frames);
        let speeds: Vec<f32> = all_frames.iter().map(|f| f.speed_kmh).collect();
        let max_speed = speeds.iter().cloned().fold(0.0f32, f32::max);
        let avg_speed = if !speeds.is_empty() {
            speeds.iter().sum::<f32>() / speeds.len() as f32
        } else { 0.0 };

        let total_duration = all_frames.last().map(|f| f.time_sec).unwrap_or(0.0)
            - all_frames.first().map(|f| f.time_sec).unwrap_or(0.0);

        // 事件偵測
        let detected = event_detection::detect_events(&all_frames);
        let hard_brakes = detected.iter()
            .filter(|e| matches!(e.event_type, event_detection::DetectedEventType::HardBrake))
            .count() as i64;
        let hard_accels = detected.iter()
            .filter(|e| matches!(e.event_type, event_detection::DetectedEventType::HardAccel))
            .count() as i64;
        let sharp_turns = detected.iter()
            .filter(|e| matches!(e.event_type, event_detection::DetectedEventType::SharpTurn))
            .count() as i64;

        // 超速百分比
        let speed_exceed_frames = all_frames.iter().filter(|f| f.speed_kmh > 110.0).count();
        let speed_exceed_pct = if !all_frames.is_empty() {
            (speed_exceed_frames as f64 / all_frames.len() as f64) * 100.0
        } else { 0.0 };

        // Autopilot 使用百分比
        let ap_frames = all_frames.iter().filter(|f| f.autopilot != "OFF").count();
        let autopilot_pct = if !all_frames.is_empty() {
            (ap_frames as f64 / all_frames.len() as f64) * 100.0
        } else { 0.0 };

        let driving_score = compute_driving_score(
            hard_brakes, hard_accels, sharp_turns, speed_exceed_pct, distance_km,
        );

        // GPS 起終點
        let first_gps = all_frames.iter().find(|f| f.lat != 0.0 && f.lon != 0.0);
        let last_gps = all_frames.iter().rev().find(|f| f.lat != 0.0 && f.lon != 0.0);

        // 計算結束時間
        let end_time = if let Ok(start_dt) = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S") {
            let end_dt = start_dt + chrono::Duration::seconds(total_duration as i64);
            end_dt.format("%Y-%m-%dT%H:%M:%S").to_string()
        } else {
            timestamp.clone()
        };

        // 寫入 trip（每次一筆，釋放 lock）
        let trip_id = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO trips (vehicle_id, event_id, start_time, end_time, duration_sec, distance_km,
                 avg_speed_kmh, max_speed_kmh, start_lat, start_lon, end_lat, end_lon,
                 event_count, hard_brake_count, hard_accel_count, sharp_turn_count,
                 autopilot_pct, driving_score)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    vehicle_id, event_id, timestamp, end_time, total_duration, distance_km,
                    avg_speed as f64, max_speed as f64,
                    first_gps.map(|f| f.lat), first_gps.map(|f| f.lon),
                    last_gps.map(|f| f.lat), last_gps.map(|f| f.lon),
                    detected.len() as i64, hard_brakes, hard_accels, sharp_turns,
                    autopilot_pct, driving_score,
                ],
            ).map_err(|e| e.to_string())?;
            conn.last_insert_rowid()
        };

        // 寫入 downsampled GPS (每 5 秒一筆)
        {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            let mut last_sample_time = -5.0_f64;
            for f in &all_frames {
                if f.lat == 0.0 && f.lon == 0.0 { continue; }
                if f.time_sec - last_sample_time < 5.0 { continue; }
                conn.execute(
                    "INSERT INTO telemetry_samples (trip_id, time_sec, lat, lon, speed_kmh, heading)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![trip_id, f.time_sec, f.lat, f.lon, f.speed_kmh as f64, f.heading],
                ).map_err(|e| e.to_string())?;
                last_sample_time = f.time_sec;
            }
        }

        trip_count += 1;
        log::info!("分析完成: trip {} / {} (距離 {:.1}km, 評分 {})", trip_count, total_events, distance_km, driving_score);
    }

    // 計算 daily_stats
    aggregate_daily_stats(vehicle_id, db)?;

    Ok(trip_count)
}

/// 從 trips 聚合出 daily_stats
fn aggregate_daily_stats(vehicle_id: i64, db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM daily_stats WHERE vehicle_id = ?1", params![vehicle_id])
        .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO daily_stats (vehicle_id, date, trip_count, total_distance_km, total_duration_sec,
         avg_speed_kmh, max_speed_kmh, event_count, driving_score)
         SELECT vehicle_id,
                DATE(start_time) as date,
                COUNT(*) as trip_count,
                SUM(distance_km) as total_distance_km,
                SUM(duration_sec) as total_duration_sec,
                AVG(avg_speed_kmh) as avg_speed_kmh,
                MAX(max_speed_kmh) as max_speed_kmh,
                SUM(event_count) as event_count,
                AVG(driving_score) as driving_score
         FROM trips
         WHERE vehicle_id = ?1
         GROUP BY vehicle_id, DATE(start_time)",
        params![vehicle_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

// ── 查詢函式 ──────────────────────────────────────────────

pub fn get_trips(db: &Database, vehicle_id: i64, date_from: Option<&str>, date_to: Option<&str>) -> Result<Vec<TripInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT id, vehicle_id, start_time, end_time, duration_sec, distance_km,
                avg_speed_kmh, max_speed_kmh, event_count, hard_brake_count,
                hard_accel_count, sharp_turn_count, autopilot_pct, driving_score
         FROM trips
         WHERE vehicle_id = ?1 {}
         ORDER BY start_time DESC",
        match (date_from, date_to) {
            (Some(_), Some(_)) => "AND start_time >= ?2 AND start_time <= ?3",
            (Some(_), None) => "AND start_time >= ?2",
            (None, Some(_)) => "AND start_time <= ?2",
            (None, None) => "",
        }
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let rows = match (date_from, date_to) {
        (Some(df), Some(dt)) => stmt.query_map(params![vehicle_id, df, dt], map_trip_row),
        (Some(df), None) => stmt.query_map(params![vehicle_id, df], map_trip_row),
        (None, Some(dt)) => stmt.query_map(params![vehicle_id, dt], map_trip_row),
        (None, None) => stmt.query_map(params![vehicle_id], map_trip_row),
    }.map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn map_trip_row(row: &rusqlite::Row) -> rusqlite::Result<TripInfo> {
    Ok(TripInfo {
        id: row.get(0)?,
        vehicle_id: row.get(1)?,
        start_time: row.get(2)?,
        end_time: row.get(3)?,
        duration_sec: row.get(4)?,
        distance_km: row.get(5)?,
        avg_speed_kmh: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
        max_speed_kmh: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
        event_count: row.get::<_, Option<i64>>(8)?.unwrap_or(0),
        hard_brake_count: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
        hard_accel_count: row.get::<_, Option<i64>>(10)?.unwrap_or(0),
        sharp_turn_count: row.get::<_, Option<i64>>(11)?.unwrap_or(0),
        autopilot_pct: row.get::<_, Option<f64>>(12)?.unwrap_or(0.0),
        driving_score: row.get::<_, Option<i64>>(13)?.unwrap_or(100),
    })
}

pub fn get_daily_stats(db: &Database, vehicle_id: i64, date_from: &str, date_to: &str) -> Result<Vec<DailyStat>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT date, trip_count, total_distance_km, total_duration_sec,
                avg_speed_kmh, max_speed_kmh, event_count, driving_score
         FROM daily_stats
         WHERE vehicle_id = ?1 AND date >= ?2 AND date <= ?3
         ORDER BY date"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![vehicle_id, date_from, date_to], |row| {
        Ok(DailyStat {
            date: row.get(0)?,
            trip_count: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
            total_distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            total_duration_sec: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            avg_speed_kmh: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
            max_speed_kmh: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
            event_count: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
            driving_score: row.get::<_, Option<i64>>(7)?.unwrap_or(100),
        })
    }).map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_period_summary(db: &Database, vehicle_id: i64, period: &str) -> Result<PeriodSummary, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let (date_from, prev_from, prev_to) = match period {
        "week" => {
            let now = chrono::Local::now();
            let weekday = now.weekday().num_days_from_monday();
            let start = now - chrono::Duration::days(weekday as i64);
            let prev_start = start - chrono::Duration::days(7);
            let prev_end = start - chrono::Duration::days(1);
            (
                start.format("%Y-%m-%d").to_string(),
                Some(prev_start.format("%Y-%m-%d").to_string()),
                Some(prev_end.format("%Y-%m-%d").to_string()),
            )
        }
        "month" => {
            let now = chrono::Local::now();
            let start = now.format("%Y-%m-01").to_string();
            let prev_month = if now.month() == 1 {
                format!("{}-12-01", now.year() - 1)
            } else {
                format!("{}-{:02}-01", now.year(), now.month() - 1)
            };
            let prev_end_date = chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                .map(|d| (d - chrono::Duration::days(1)).format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            (start, Some(prev_month), Some(prev_end_date))
        }
        _ => ("1970-01-01".to_string(), None, None),
    };

    // 當前期間
    let current = conn.query_row(
        "SELECT COALESCE(SUM(total_distance_km), 0), COALESCE(SUM(total_duration_sec), 0),
                COALESCE(SUM(trip_count), 0), COALESCE(SUM(event_count), 0),
                COALESCE(AVG(driving_score), 100)
         FROM daily_stats
         WHERE vehicle_id = ?1 AND date >= ?2 AND date <= ?3",
        params![vehicle_id, date_from, today],
        |row| Ok((
            row.get::<_, f64>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, f64>(4)?,
        )),
    ).map_err(|e| e.to_string())?;

    // 前期
    let prev = if let (Some(pf), Some(pt)) = (&prev_from, &prev_to) {
        conn.query_row(
            "SELECT COALESCE(SUM(total_distance_km), 0), COALESCE(SUM(total_duration_sec), 0),
                    COALESCE(SUM(trip_count), 0), COALESCE(SUM(event_count), 0)
             FROM daily_stats
             WHERE vehicle_id = ?1 AND date >= ?2 AND date <= ?3",
            params![vehicle_id, pf, pt],
            |row| Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            )),
        ).ok()
    } else {
        None
    };

    Ok(PeriodSummary {
        total_distance_km: current.0,
        total_duration_sec: current.1,
        trip_count: current.2,
        event_count: current.3,
        driving_score: current.4 as i64,
        prev_distance_km: prev.map(|p| p.0),
        prev_duration_sec: prev.map(|p| p.1),
        prev_trip_count: prev.map(|p| p.2),
        prev_event_count: prev.map(|p| p.3),
    })
}

pub fn get_heatmap_data(db: &Database, vehicle_id: i64, date_from: Option<&str>, date_to: Option<&str>) -> Result<Vec<HeatmapPoint>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (df, dt) = (
        date_from.unwrap_or("1970-01-01"),
        date_to.unwrap_or("2099-12-31"),
    );

    let mut stmt = conn.prepare(
        "SELECT ts.lat, ts.lon, ts.speed_kmh
         FROM telemetry_samples ts
         JOIN trips t ON ts.trip_id = t.id
         WHERE t.vehicle_id = ?1 AND t.start_time >= ?2 AND t.start_time <= ?3
         ORDER BY t.start_time, ts.time_sec"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![vehicle_id, df, dt], |row| {
        Ok(HeatmapPoint {
            lat: row.get(0)?,
            lon: row.get(1)?,
            speed_kmh: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
        })
    }).map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
