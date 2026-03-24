import { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import SteeringWheel from "./SteeringWheel";
import type { Clip } from "../types/events";
import "./TelemetryOverlay.css";

interface TelemetryFrame {
  time_sec: number;
  frame_seq: number;
  speed_kmh: number;
  steering_angle: number;
  gear: string;
  accel_pedal: number;
  brake: boolean;
  blinker_left: boolean;
  blinker_right: boolean;
  autopilot: string;
  lat: number;
  lon: number;
  heading: number;
}

interface TelemetryOverlayProps {
  clips: Clip[];
  currentTime: number;
  visible: boolean;
  onToggle: () => void;
}

export default function TelemetryOverlay({
  clips,
  currentTime,
  visible,
  onToggle,
}: TelemetryOverlayProps) {
  const [telemetryMap, setTelemetryMap] = useState<Map<number, TelemetryFrame[]>>(new Map());
  const [loading, setLoading] = useState(false);
  const [noData, setNoData] = useState(false);

  const frontClips = useMemo(
    () => clips.filter((c) => c.camera === "front").sort((a, b) => a.segmentIndex - b.segmentIndex),
    [clips]
  );

  useEffect(() => {
    if (frontClips.length === 0) { setTelemetryMap(new Map()); setNoData(false); return; }
    let cancelled = false;
    setLoading(true); setNoData(false);
    (async () => {
      const map = new Map<number, TelemetryFrame[]>();
      let hasAny = false;
      for (const clip of frontClips) {
        if (cancelled) break;
        try {
          const frames = await invoke<TelemetryFrame[]>("parse_telemetry", { filePath: clip.filePath });
          if (frames.length > 0) hasAny = true;
          map.set(clip.segmentIndex, frames);
        } catch {
          map.set(clip.segmentIndex, []);
        }
      }
      if (!cancelled) { setTelemetryMap(map); setNoData(!hasAny); setLoading(false); }
    })();
    return () => { cancelled = true; };
  }, [frontClips]);

  const currentFrame = useMemo(() => {
    if (telemetryMap.size === 0) return null;

    // 計算目前在哪個 segment 以及 segment 內的時間
    let remaining = currentTime;
    let segIdx = 0;
    for (const clip of frontClips) {
      if (remaining <= clip.durationSec) {
        segIdx = clip.segmentIndex;
        break;
      }
      remaining -= clip.durationSec;
      segIdx = clip.segmentIndex;
    }

    const frames = telemetryMap.get(segIdx);
    if (!frames || frames.length === 0) return null;

    // 用 time_sec 二分搜尋找到最接近 remaining 的兩個幀，做線性插值
    const targetTime = remaining;
    let lo = 0, hi = frames.length - 1;
    while (lo < hi - 1) {
      const mid = (lo + hi) >> 1;
      if (frames[mid].time_sec <= targetTime) lo = mid;
      else hi = mid;
    }

    const a = frames[lo];
    const b = frames[Math.min(hi, frames.length - 1)];

    // 如果目標時間在第一個 SEI 之前，顯示第一個
    if (targetTime <= a.time_sec) return a;
    // 如果只有一個幀或兩幀相同時間
    if (a.time_sec >= b.time_sec) return a;

    const t = (targetTime - a.time_sec) / (b.time_sec - a.time_sec);
    const tc = Math.max(0, Math.min(1, t));

    return {
      ...a,
      time_sec: targetTime,
      speed_kmh: a.speed_kmh + (b.speed_kmh - a.speed_kmh) * tc,
      steering_angle: a.steering_angle + (b.steering_angle - a.steering_angle) * tc,
      accel_pedal: a.accel_pedal + (b.accel_pedal - a.accel_pedal) * tc,
      heading: a.heading + (b.heading - a.heading) * tc,
      lat: a.lat + (b.lat - a.lat) * tc,
      lon: a.lon + (b.lon - a.lon) * tc,
      brake: tc < 0.5 ? a.brake : b.brake,
      blinker_left: tc < 0.5 ? a.blinker_left : b.blinker_left,
      blinker_right: tc < 0.5 ? a.blinker_right : b.blinker_right,
      gear: tc < 0.5 ? a.gear : b.gear,
      autopilot: tc < 0.5 ? a.autopilot : b.autopilot,
    };
  }, [currentTime, telemetryMap, frontClips]);

  const [minimized, setMinimized] = useState(false);

  if (!visible) {
    return (
      <button className="telemetry-toggle" onClick={onToggle} title="顯示遙測資料">⊕</button>
    );
  }

  const f = currentFrame;

  // 縮小模式：只顯示速度 + 檔位
  if (minimized && f) {
    return (
      <div className="telemetry-mini" onClick={() => setMinimized(false)} title="點擊展開">
        <span className="mini-speed">{Math.round(f.speed_kmh)}</span>
        <span className="mini-unit">km/h</span>
        <span className={`mini-gear ${f.gear === "R" ? "gear-reverse" : ""}`}>{f.gear}</span>
      </div>
    );
  }

  return (
    <div className="telemetry-overlay">
      <div className="telemetry-header">
        <span>遙測資料</span>
        <div className="telemetry-header-btns">
          <button className="telemetry-btn" onClick={() => setMinimized(true)} title="縮小">─</button>
          <button className="telemetry-btn" onClick={onToggle} title="關閉">✕</button>
        </div>
      </div>

      {loading && <div className="telemetry-status">載入 SEI 資料中...</div>}
      {noData && !loading && <div className="telemetry-status">此影片無 SEI 資料<br/><span style={{fontSize:9,color:'#444'}}>需韌體 2025.44.25+ / HW3+</span></div>}
      {!noData && !loading && !f && <div className="telemetry-status">等待遙測資料...</div>}

      {f && (
        <div className="telemetry-data">
          {/* 儀表板：方向盤 + 時速 */}
          <div className="dashboard">
            <div className="dashboard-center">
              <SteeringWheel angle={f.steering_angle} size={120} />
              <div className="dashboard-speed">
                <span className="speed-number">{Math.round(f.speed_kmh)}</span>
                <span className="speed-unit">km/h</span>
              </div>
            </div>

            {/* 檔位 */}
            <div className="dashboard-gear">
              {(["P", "R", "N", "D"] as const).map((g) => (
                <span
                  key={g}
                  className={`gear-letter ${f.gear === g ? "gear-active" : ""} ${f.gear === g && g === "R" ? "gear-reverse" : ""}`}
                >
                  {g}
                </span>
              ))}
            </div>
          </div>

          <div className="tele-divider" />

          {/* 方向燈 + 踏板 */}
          <div className="tele-indicators">
            <span className={`indicator ${f.blinker_left ? "indicator-on" : ""}`}>◄</span>
            <div className="pedal-bars">
              <div className="pedal-row">
                <span className="pedal-label">油門</span>
                <div className="pedal-track">
                  <div className="pedal-fill pedal-accel" style={{ width: `${Math.min(f.accel_pedal, 100)}%` }} />
                </div>
                <span className="pedal-pct">{f.accel_pedal.toFixed(0)}%</span>
              </div>
              <div className="pedal-row">
                <span className="pedal-label">煞車</span>
                <div className="pedal-track">
                  <div className={`pedal-fill ${f.brake ? "pedal-brake-on" : "pedal-brake"}`} style={{ width: f.brake ? "100%" : "0%" }} />
                </div>
                <span className={`pedal-pct ${f.brake ? "pedal-pct-warn" : ""}`}>{f.brake ? "ON" : ""}</span>
              </div>
            </div>
            <span className={`indicator ${f.blinker_right ? "indicator-on" : ""}`}>►</span>
          </div>

          <div className="tele-divider" />

          {/* 詳細資訊 */}
          <div className="tele-details">
            <div className="tele-detail-row">
              <span className="tele-label">方向盤</span>
              <span className="tele-val">{f.steering_angle.toFixed(1)}°</span>
            </div>
            <div className="tele-detail-row">
              <span className="tele-label">航向</span>
              <span className="tele-val">{f.heading.toFixed(1)}°</span>
            </div>
            <div className="tele-detail-row">
              <span className="tele-label">自駕</span>
              <span className={`tele-val ${f.autopilot !== "OFF" ? "tele-val-ap" : ""}`}>
                {f.autopilot}
              </span>
            </div>
            {f.lat !== 0 && (
              <div className="tele-detail-row">
                <span className="tele-label">GPS</span>
                <span className="tele-val tele-val-small">
                  {f.lat.toFixed(6)}, {f.lon.toFixed(6)}
                </span>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
