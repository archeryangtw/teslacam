use crate::sei::TelemetryFrame;
use serde::Serialize;

/// 偵測到的事件類型
#[derive(Debug, Clone, Serialize)]
pub enum DetectedEventType {
    HardBrake,        // 急煞車
    HardAccel,        // 急加速
    SharpTurn,        // 急轉彎
    ReverseGear,      // 倒車
    AutopilotChange,  // 自駕狀態變化
    Stop,             // 停車
    SpeedExceed,      // 超過特定速度
}

/// 偵測到的事件
#[derive(Debug, Clone, Serialize)]
pub struct DetectedEvent {
    pub event_type: DetectedEventType,
    pub time_sec: f64,
    pub duration_sec: f64,
    pub description: String,
    pub severity: u8, // 1-3, 3=最嚴重
}

/// 從遙測資料偵測事件
pub fn detect_events(frames: &[TelemetryFrame]) -> Vec<DetectedEvent> {
    let mut events = Vec::new();

    if frames.len() < 2 {
        return events;
    }

    let mut prev_speed = frames[0].speed_kmh;
    let mut prev_gear = frames[0].gear.clone();
    let mut prev_autopilot = frames[0].autopilot.clone();
    let mut stop_start: Option<f64> = None;

    for i in 1..frames.len() {
        let f = &frames[i];
        let dt = f.time_sec - frames[i - 1].time_sec;
        if dt <= 0.0 {
            continue;
        }

        let speed_change = f.speed_kmh - prev_speed;
        let decel = speed_change / dt as f32; // km/h per second

        // 急煞車：減速 > 15 km/h/s 且煞車踩下
        if decel < -15.0 && f.brake {
            let severity = if decel < -30.0 { 3 } else if decel < -20.0 { 2 } else { 1 };
            events.push(DetectedEvent {
                event_type: DetectedEventType::HardBrake,
                time_sec: f.time_sec,
                duration_sec: dt,
                description: format!("急煞車 {:.0} → {:.0} km/h", prev_speed, f.speed_kmh),
                severity,
            });
        }

        // 急加速：加速 > 15 km/h/s
        if decel > 15.0 && prev_speed > 5.0 {
            let severity = if decel > 30.0 { 3 } else if decel > 20.0 { 2 } else { 1 };
            events.push(DetectedEvent {
                event_type: DetectedEventType::HardAccel,
                time_sec: f.time_sec,
                duration_sec: dt,
                description: format!("急加速 {:.0} → {:.0} km/h", prev_speed, f.speed_kmh),
                severity,
            });
        }

        // 急轉彎：方向盤角度變化 > 90°/s 且車速 > 20km/h
        let steer_rate = (f.steering_angle - frames[i - 1].steering_angle).abs() / dt as f32;
        if steer_rate > 90.0 && f.speed_kmh > 20.0 {
            events.push(DetectedEvent {
                event_type: DetectedEventType::SharpTurn,
                time_sec: f.time_sec,
                duration_sec: dt,
                description: format!("急轉彎 方向盤 {:.0}°/s @ {:.0}km/h", steer_rate, f.speed_kmh),
                severity: if steer_rate > 180.0 { 3 } else { 2 },
            });
        }

        // 倒車
        if f.gear == "R" && prev_gear != "R" {
            events.push(DetectedEvent {
                event_type: DetectedEventType::ReverseGear,
                time_sec: f.time_sec,
                duration_sec: 0.0,
                description: "切換到倒車檔".to_string(),
                severity: 1,
            });
        }

        // 自駕狀態變化
        if f.autopilot != prev_autopilot {
            events.push(DetectedEvent {
                event_type: DetectedEventType::AutopilotChange,
                time_sec: f.time_sec,
                duration_sec: 0.0,
                description: format!("自駕: {} → {}", prev_autopilot, f.autopilot),
                severity: 2,
            });
        }

        // 停車偵測
        if f.speed_kmh < 1.0 && prev_speed >= 1.0 {
            stop_start = Some(f.time_sec);
        }
        if f.speed_kmh >= 1.0 && prev_speed < 1.0 {
            if let Some(start) = stop_start {
                let dur = f.time_sec - start;
                if dur > 3.0 {
                    events.push(DetectedEvent {
                        event_type: DetectedEventType::Stop,
                        time_sec: start,
                        duration_sec: dur,
                        description: format!("停車 {:.0} 秒", dur),
                        severity: 1,
                    });
                }
            }
            stop_start = None;
        }

        // 超速 (> 110 km/h)
        if f.speed_kmh > 110.0 && prev_speed <= 110.0 {
            events.push(DetectedEvent {
                event_type: DetectedEventType::SpeedExceed,
                time_sec: f.time_sec,
                duration_sec: 0.0,
                description: format!("車速超過 110 km/h ({:.0})", f.speed_kmh),
                severity: 2,
            });
        }

        prev_speed = f.speed_kmh;
        prev_gear = f.gear.clone();
        prev_autopilot = f.autopilot.clone();
    }

    // 去重：相同類型在 2 秒內只保留最嚴重的
    dedup_events(&mut events);

    events
}

fn dedup_events(events: &mut Vec<DetectedEvent>) {
    if events.len() < 2 {
        return;
    }
    events.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap());

    let mut result = Vec::new();
    let mut i = 0;
    while i < events.len() {
        let mut best = events[i].clone();
        let mut j = i + 1;
        while j < events.len() {
            let same_type = std::mem::discriminant(&events[j].event_type)
                == std::mem::discriminant(&best.event_type);
            if same_type && (events[j].time_sec - best.time_sec).abs() < 2.0 {
                if events[j].severity > best.severity {
                    best = events[j].clone();
                }
                j += 1;
            } else {
                break;
            }
        }
        result.push(best);
        i = j;
    }
    *events = result;
}
