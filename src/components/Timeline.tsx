import type { TeslaCamEvent } from "../types/events";
import "./Timeline.css";

export interface DetectedEvent {
  event_type: string;
  time_sec: number;
  duration_sec: number;
  description: string;
  severity: number;
}

interface TimelineProps {
  event: TeslaCamEvent | null;
  currentTime: number;
  duration?: number;
  isPlaying: boolean;
  playbackRate: number;
  markIn: number | null;
  markOut: number | null;
  detectedEvents?: DetectedEvent[];
  onSeek: (time: number) => void;
  onPlayPause: () => void;
  onPlaybackRateChange: (rate: number) => void;
  onFrameStep: (direction: 1 | -1) => void;
  onSetMarkIn: () => void;
  onSetMarkOut: () => void;
  onClearMarks: () => void;
}

const PLAYBACK_RATES = [0.5, 1, 1.5, 2, 4];
const FPS = 36;

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  const f = Math.floor((seconds % 1) * FPS);
  return `${m}:${String(s).padStart(2, "0")}:${String(f).padStart(2, "0")}`;
}

function formatTimestamp(iso: string, offsetSec: number): string {
  // iso 格式 "2026-03-23T13:01:18"，視為本地時間（加上本地時區偏移避免 UTC 解析）
  const d = new Date(iso.includes("+") || iso.includes("Z") ? iso : iso + "+08:00");
  d.setSeconds(d.getSeconds() + Math.floor(offsetSec));
  const yy = d.getFullYear();
  const mo = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${yy}-${mo}-${dd} ${hh}:${mm}:${ss}`;
}

const EVENT_COLORS: Record<string, string> = {
  HardBrake: "#e94560",
  HardAccel: "#f0c040",
  SharpTurn: "#ff8c00",
  ReverseGear: "#9b59b6",
  AutopilotChange: "#4ecdc4",
  Stop: "#888",
  SpeedExceed: "#e94560",
};

export default function Timeline({
  event,
  currentTime,
  duration: durationProp,
  isPlaying,
  playbackRate,
  markIn,
  markOut,
  detectedEvents,
  onSeek,
  onPlayPause,
  onPlaybackRateChange,
  onFrameStep,
  onSetMarkIn,
  onSetMarkOut,
  onClearMarks,
}: TimelineProps) {
  const duration = durationProp ?? event?.durationSec ?? 0;
  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;

  const handleBarClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!duration) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    onSeek(Math.max(0, Math.min(duration, ratio * duration)));
  };

  const cycleRate = () => {
    const idx = PLAYBACK_RATES.indexOf(playbackRate);
    const next = PLAYBACK_RATES[(idx + 1) % PLAYBACK_RATES.length];
    onPlaybackRateChange(next);
  };

  const currentTs = event ? formatTimestamp(event.timestamp, currentTime) : "--:--:--";
  const hasMarks = markIn !== null || markOut !== null;

  return (
    <div className="timeline-area">
      {/* 主控制列 */}
      <div className="timeline-controls">
        <button className="tl-btn" onClick={() => onFrameStep(-1)} disabled={!event} title="上一幀">
          ◄◄
        </button>
        <button className="play-btn" onClick={onPlayPause} disabled={!event}>
          {isPlaying ? "⏸" : "▶"}
        </button>
        <button className="tl-btn" onClick={() => onFrameStep(1)} disabled={!event} title="下一幀">
          ►►
        </button>

        <span className="time-display">{currentTs}</span>
        <span className="time-display time-elapsed">
          {formatTime(currentTime)} / {formatTime(duration)}
        </span>

        <div className="tl-spacer" />

        {/* 標記控制 */}
        <button
          className={`tl-btn mark-btn ${markIn !== null ? "mark-active" : ""}`}
          onClick={onSetMarkIn}
          disabled={!event}
          title="設定起點 [I]"
        >
          I
        </button>
        <button
          className={`tl-btn mark-btn ${markOut !== null ? "mark-active" : ""}`}
          onClick={onSetMarkOut}
          disabled={!event}
          title="設定終點 [O]"
        >
          O
        </button>
        {hasMarks && (
          <button className="tl-btn" onClick={onClearMarks} title="清除標記">
            ✕
          </button>
        )}

        {hasMarks && (
          <span className="mark-range">
            {markIn !== null ? formatTime(markIn) : "起"}
            {" → "}
            {markOut !== null ? formatTime(markOut) : "終"}
            {markIn !== null && markOut !== null && (
              <span className="mark-duration">
                ({formatTime(markOut - markIn)})
              </span>
            )}
          </span>
        )}

        <button className="speed-btn" onClick={cycleRate}>
          {playbackRate}x
        </button>
      </div>

      {/* 時間軸 */}
      <div className="timeline-bar" onClick={handleBarClick}>
        <div className="timeline-track">
          {/* 標記範圍高亮 */}
          {markIn !== null && markOut !== null && duration > 0 && (
            <div
              className="timeline-mark-range"
              style={{
                left: `${(markIn / duration) * 100}%`,
                width: `${((markOut - markIn) / duration) * 100}%`,
              }}
            />
          )}
          {/* 起點標記 */}
          {markIn !== null && duration > 0 && (
            <div
              className="timeline-mark timeline-mark-in"
              style={{ left: `${(markIn / duration) * 100}%` }}
            />
          )}
          {/* 終點標記 */}
          {markOut !== null && duration > 0 && (
            <div
              className="timeline-mark timeline-mark-out"
              style={{ left: `${(markOut / duration) * 100}%` }}
            />
          )}
          {/* 偵測到的事件標記 */}
          {detectedEvents && duration > 0 && detectedEvents.map((de, idx) => (
            <div
              key={idx}
              className="timeline-event-dot"
              style={{
                left: `${(de.time_sec / duration) * 100}%`,
                backgroundColor: EVENT_COLORS[de.event_type] ?? "#888",
                width: de.severity >= 3 ? "6px" : de.severity >= 2 ? "4px" : "3px",
                height: de.severity >= 3 ? "6px" : de.severity >= 2 ? "4px" : "3px",
              }}
              title={de.description}
              onClick={(e) => { e.stopPropagation(); onSeek(de.time_sec); }}
            />
          ))}
          <div className="timeline-progress" style={{ width: `${progress}%` }} />
          <div className="timeline-playhead" style={{ left: `${progress}%` }} />
        </div>
      </div>
    </div>
  );
}
