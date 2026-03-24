use prost::Message;
use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

/// Protobuf: SeiMetadata
#[derive(Clone, PartialEq, Message)]
pub struct SeiMetadataProto {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(enumeration = "Gear", tag = "2")]
    pub gear_state: i32,
    #[prost(uint64, tag = "3")]
    pub frame_seq_no: u64,
    #[prost(float, tag = "4")]
    pub vehicle_speed_mps: f32,
    #[prost(float, tag = "5")]
    pub accelerator_pedal_position: f32,
    #[prost(float, tag = "6")]
    pub steering_wheel_angle: f32,
    #[prost(bool, tag = "7")]
    pub blinker_on_left: bool,
    #[prost(bool, tag = "8")]
    pub blinker_on_right: bool,
    #[prost(bool, tag = "9")]
    pub brake_applied: bool,
    #[prost(enumeration = "AutopilotState", tag = "10")]
    pub autopilot_state: i32,
    #[prost(double, tag = "11")]
    pub latitude_deg: f64,
    #[prost(double, tag = "12")]
    pub longitude_deg: f64,
    #[prost(double, tag = "13")]
    pub heading_deg: f64,
    #[prost(double, tag = "14")]
    pub linear_accel_x: f64,
    #[prost(double, tag = "15")]
    pub linear_accel_y: f64,
    #[prost(double, tag = "16")]
    pub linear_accel_z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum Gear { Park = 0, Drive = 1, Reverse = 2, Neutral = 3 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum AutopilotState { None = 0, SelfDriving = 1, Autosteer = 2, Tacc = 3 }

#[derive(Debug, Clone, Serialize)]
pub struct TelemetryFrame {
    /// 此幀在影片中的精確時間（秒）
    pub time_sec: f64,
    pub frame_seq: u64,
    pub speed_kmh: f32,
    pub steering_angle: f32,
    pub gear: String,
    pub accel_pedal: f32,
    pub brake: bool,
    pub blinker_left: bool,
    pub blinker_right: bool,
    pub autopilot: String,
    pub lat: f64,
    pub lon: f64,
    pub heading: f64,
}

fn gear_str(g: i32) -> &'static str {
    match g { 0 => "P", 1 => "D", 2 => "R", 3 => "N", _ => "?" }
}

fn autopilot_str(a: i32) -> &'static str {
    match a { 1 => "FSD", 2 => "Autosteer", 3 => "TACC", _ => "OFF" }
}

// ─── MP4 moov 解析 ───

struct Mp4Info {
    mdat_offset: u64,
    mdat_size: u64,
    sample_sizes: Vec<u32>,   // stsz
    stts_entries: Vec<(u32, u32)>, // (sample_count, sample_delta)
    timescale: u32,           // mvhd timescale
}

fn find_atom(data: &[u8], name: &[u8; 4]) -> Option<usize> {
    data.windows(4).position(|w| w == name)
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

fn parse_mp4_info(fp: &mut File) -> io::Result<Mp4Info> {
    let file_size = fp.seek(SeekFrom::End(0))?;
    fp.seek(SeekFrom::Start(0))?;

    let mut mdat_offset = 0u64;
    let mut mdat_size = 0u64;
    let mut moov_data: Option<Vec<u8>> = None;

    // 找 mdat 和 moov
    while fp.stream_position()? < file_size {
        let mut header = [0u8; 8];
        if fp.read_exact(&mut header).is_err() { break; }
        let size32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let atom_type = &header[4..8];
        let (atom_size, header_size) = if size32 == 1 {
            let mut ext = [0u8; 8];
            fp.read_exact(&mut ext)?;
            (u64::from_be_bytes(ext), 16u64)
        } else {
            (size32 as u64, 8u64)
        };

        if atom_type == b"mdat" {
            mdat_offset = fp.stream_position()?;
            mdat_size = if atom_size > 0 { atom_size - header_size } else { 0 };
            // 跳過 mdat 資料以繼續尋找 moov
            if mdat_size > 0 {
                fp.seek(SeekFrom::Current(mdat_size as i64))?;
            }
        } else if atom_type == b"moov" {
            let sz = (atom_size - header_size) as usize;
            let mut buf = vec![0u8; sz];
            fp.read_exact(&mut buf)?;
            moov_data = Some(buf);
        } else {
            if atom_size < header_size { break; }
            fp.seek(SeekFrom::Current((atom_size - header_size) as i64))?;
        }
    }

    let moov = moov_data.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no moov"))?;

    // 解析 mvhd timescale
    let timescale = if let Some(pos) = find_atom(&moov, b"mvhd") {
        let ver = moov[pos + 4];
        if ver == 0 { read_u32_be(&moov, pos + 16) } else { read_u32_be(&moov, pos + 24) }
    } else { 10000 };

    // 解析 stsz
    let sample_sizes = if let Some(pos) = find_atom(&moov, b"stsz") {
        let count = read_u32_be(&moov, pos + 12) as usize;
        (0..count).map(|i| read_u32_be(&moov, pos + 16 + i * 4)).collect()
    } else { Vec::new() };

    // 解析 stts
    let stts_entries = if let Some(pos) = find_atom(&moov, b"stts") {
        let count = read_u32_be(&moov, pos + 8) as usize;
        (0..count).map(|i| {
            let sc = read_u32_be(&moov, pos + 12 + i * 8);
            let sd = read_u32_be(&moov, pos + 16 + i * 8);
            (sc, sd)
        }).collect()
    } else { Vec::new() };

    Ok(Mp4Info { mdat_offset, mdat_size, sample_sizes, stts_entries, timescale })
}

/// 用 stts 計算每個 sample 的起始時間（秒）
fn build_sample_times(info: &Mp4Info) -> Vec<f64> {
    let ts = info.timescale as f64;
    let mut times = Vec::with_capacity(info.sample_sizes.len());
    let mut t = 0u64;
    for &(count, delta) in &info.stts_entries {
        for _ in 0..count {
            times.push(t as f64 / ts);
            t += delta as u64;
        }
    }
    // 如果 stts 不夠，用最後一個 delta 補
    let last_delta = info.stts_entries.last().map(|e| e.1 as u64).unwrap_or(1);
    while times.len() < info.sample_sizes.len() {
        times.push(t as f64 / ts);
        t += last_delta;
    }
    times
}

fn strip_emulation_prevention(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut zero_count = 0u32;
    for &b in data {
        if zero_count >= 2 && b == 0x03 { zero_count = 0; continue; }
        out.push(b);
        zero_count = if b == 0 { zero_count + 1 } else { 0 };
    }
    out
}

fn extract_proto_payload(nal: &[u8]) -> Option<Vec<u8>> {
    if nal.len() < 5 { return None; }
    for i in 3..nal.len().saturating_sub(1) {
        let b = nal[i];
        if b == 0x42 { continue; }
        if b == 0x69 && i > 2 {
            return Some(strip_emulation_prevention(&nal[i + 1..nal.len() - 1]));
        }
        break;
    }
    None
}

/// 從 MP4 解析 SEI，用 sample table 精確對應每個 SEI 的影片時間
pub fn parse_sei_from_file(path: &str) -> Result<Vec<TelemetryFrame>, String> {
    let mut fp = File::open(path).map_err(|e| format!("無法開啟: {}", e))?;
    let info = parse_mp4_info(&mut fp).map_err(|e| format!("MP4 解析失敗: {}", e))?;

    if info.sample_sizes.is_empty() {
        return Ok(Vec::new());
    }

    let sample_times = build_sample_times(&info);

    fp.seek(SeekFrom::Start(info.mdat_offset)).map_err(|e| e.to_string())?;

    let mut frames = Vec::new();
    let mut consumed = 0u64;
    let mut sample_idx: usize = 0;
    let mut sample_bytes_consumed: u64 = 0;

    while info.mdat_size == 0 || consumed < info.mdat_size {
        let mut size_buf = [0u8; 4];
        if fp.read_exact(&mut size_buf).is_err() { break; }
        let nal_size = u32::from_be_bytes(size_buf) as usize;
        consumed += 4;

        if nal_size < 2 {
            if fp.seek(SeekFrom::Current(nal_size as i64)).is_err() { break; }
            consumed += nal_size as u64;
            // 非 SEI 的小 NAL 算入 sample
            sample_bytes_consumed += 4 + nal_size as u64;
            while sample_idx < info.sample_sizes.len()
                && sample_bytes_consumed >= info.sample_sizes[sample_idx] as u64
            {
                sample_bytes_consumed -= info.sample_sizes[sample_idx] as u64;
                sample_idx += 1;
            }
            continue;
        }

        let mut first_two = [0u8; 2];
        if fp.read_exact(&mut first_two).is_err() { break; }

        const NAL_ID_SEI: u8 = 6;
        const NAL_SEI_USER_DATA: u8 = 5;
        let is_sei = (first_two[0] & 0x1F) == NAL_ID_SEI && first_two[1] == NAL_SEI_USER_DATA;

        let mut rest = vec![0u8; nal_size - 2];
        if fp.read_exact(&mut rest).is_err() { break; }
        consumed += nal_size as u64;

        if is_sei {
            // SEI NAL：取得當前 sample 的時間
            let time_sec = if sample_idx < sample_times.len() {
                sample_times[sample_idx]
            } else {
                // fallback: 用 sample_idx 估算
                sample_idx as f64 / 36.0
            };

            let mut full_nal = Vec::with_capacity(nal_size);
            full_nal.extend_from_slice(&first_two);
            full_nal.extend_from_slice(&rest);

            if let Some(payload) = extract_proto_payload(&full_nal) {
                if let Ok(meta) = SeiMetadataProto::decode(payload.as_slice()) {
                    frames.push(TelemetryFrame {
                        time_sec,
                        frame_seq: meta.frame_seq_no,
                        speed_kmh: meta.vehicle_speed_mps * 3.6,
                        steering_angle: meta.steering_wheel_angle,
                        gear: gear_str(meta.gear_state).to_string(),
                        accel_pedal: meta.accelerator_pedal_position,
                        brake: meta.brake_applied,
                        blinker_left: meta.blinker_on_left,
                        blinker_right: meta.blinker_on_right,
                        autopilot: autopilot_str(meta.autopilot_state).to_string(),
                        lat: meta.latitude_deg,
                        lon: meta.longitude_deg,
                        heading: meta.heading_deg,
                    });
                }
            }

            // SEI 如果不計入 sample size，就不加 sample_bytes
            // 如果計入 sample size，就加上
            // 先假設計入（根據分析 sum of stsz = mdat payload）
            sample_bytes_consumed += 4 + nal_size as u64;
        } else {
            // 非 SEI NAL：計入 sample
            sample_bytes_consumed += 4 + nal_size as u64;
        }

        // 推進 sample 邊界
        while sample_idx < info.sample_sizes.len()
            && sample_bytes_consumed >= info.sample_sizes[sample_idx] as u64
        {
            sample_bytes_consumed -= info.sample_sizes[sample_idx] as u64;
            sample_idx += 1;
        }
    }

    Ok(frames)
}

/// 取樣到目標頻率
pub fn downsample_by_time(frames: &[TelemetryFrame], interval_sec: f64) -> Vec<TelemetryFrame> {
    if frames.is_empty() { return Vec::new(); }
    let mut result = Vec::new();
    let mut next_time = frames[0].time_sec;
    for f in frames {
        if f.time_sec >= next_time {
            result.push(f.clone());
            next_time = f.time_sec + interval_sec;
        }
    }
    result
}
