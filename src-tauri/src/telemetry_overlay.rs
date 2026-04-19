use crate::sei::TelemetryFrame;
use std::io::Write;
use std::path::Path;

/// 生成 ASS 字幕檔，在影片上顯示遙測資料
pub fn generate_ass_overlay(
    frames: &[TelemetryFrame],
    trim_start: f64,
    trim_end: f64,
    output_path: &Path,
    video_width: u32,
    video_height: u32,
) -> Result<(), String> {
    let mut file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;

    // ASS 檔頭
    write!(
        file,
        r#"[Script Info]
Title: TeslaCam Telemetry
ScriptType: v4.00+
PlayResX: {video_width}
PlayResY: {video_height}
WrapStyle: 0

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Gps,SF Mono,24,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,7,20,20,55,1
Style: Info,SF Mono,24,&H00CCCCCC,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,1,20,20,20,1
Style: Gear,SF Mono,36,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,3,0,9,20,20,20,1
Style: Brake,SF Mono,28,&H004560E9,&H000000FF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,2,0,3,20,20,20,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"#
    )
    .map_err(|e| e.to_string())?;

    // 篩選在 trim 範圍內的幀，每 0.5 秒一筆
    let mut last_time = -1.0f64;

    for frame in frames {
        if frame.time_sec < trim_start || frame.time_sec > trim_end {
            continue;
        }

        let rel_time = frame.time_sec - trim_start;
        if rel_time - last_time < 0.5 {
            continue;
        }
        last_time = rel_time;

        let start = format_ass_time(rel_time);
        let end = format_ass_time(rel_time + 0.5);

        // GPS 經緯度（左上，接在 drawtext 時間戳下方）
        if frame.lat != 0.0 || frame.lon != 0.0 {
            writeln!(
                file,
                "Dialogue: 0,{start},{end},Gps,,0,0,0,,{:.6}, {:.6}",
                frame.lat, frame.lon
            )
            .ok();
        }

        // 檔位（右上）
        let gear_color = if frame.gear == "R" { "\\c&H004560E9&" } else { "" };
        writeln!(
            file,
            "Dialogue: 0,{start},{end},Gear,,0,0,0,,{{{gear_color}}}{}", frame.gear
        )
        .ok();

        // 方向盤 + 油門（左下資訊列）
        let steer_dir = if frame.steering_angle > 5.0 {
            "→"
        } else if frame.steering_angle < -5.0 {
            "←"
        } else {
            "↑"
        };
        let brake_text = if frame.brake { "  BRAKE" } else { "" };
        writeln!(
            file,
            "Dialogue: 0,{start},{end},Info,,0,0,0,,{steer_dir} {:.0}°  |  油門 {:.0}%{brake_text}",
            frame.steering_angle.abs(),
            frame.accel_pedal,
        )
        .ok();

        // 煞車警示（畫面中央偏下）
        if frame.brake && frame.speed_kmh > 10.0 {
            writeln!(
                file,
                "Dialogue: 1,{start},{end},Brake,,0,0,0,,BRAKE"
            )
            .ok();
        }
    }

    Ok(())
}

fn format_ass_time(seconds: f64) -> String {
    let h = (seconds / 3600.0) as u32;
    let m = ((seconds % 3600.0) / 60.0) as u32;
    let s = (seconds % 60.0) as u32;
    let cs = ((seconds % 1.0) * 100.0) as u32;
    format!("{h}:{m:02}:{s:02}.{cs:02}")
}
