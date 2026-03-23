import type { TeslaCamEvent } from "../types/events";
import "./Timeline.css";

interface TimelineProps {
  event: TeslaCamEvent | null;
  currentTime: number;
  duration?: number;
  isPlaying: boolean;
  playbackRate: number;
  onSeek: (time: number) => void;
  onPlayPause: () => void;
  onPlaybackRateChange: (rate: number) => void;
}

const PLAYBACK_RATES = [0.5, 1, 1.5, 2, 4];

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

export default function Timeline({
  event,
  currentTime,
  duration: durationProp,
  isPlaying,
  playbackRate,
  onSeek,
  onPlayPause,
  onPlaybackRateChange,
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

  // 計算目前絕對時間
  const currentTimestamp = event
    ? (() => {
        const d = new Date(event.timestamp);
        d.setSeconds(d.getSeconds() + currentTime);
        return formatTimestamp(d.toISOString());
      })()
    : "--:--:--";

  return (
    <div className="timeline-area">
      <div className="timeline-controls">
        <button className="play-btn" onClick={onPlayPause} disabled={!event}>
          {isPlaying ? "⏸" : "▶"}
        </button>
        <span className="time-display">
          {currentTimestamp}
        </span>
        <span className="time-display time-elapsed">
          {formatTime(currentTime)} / {formatTime(duration)}
        </span>
        <button className="speed-btn" onClick={cycleRate}>
          {playbackRate}x
        </button>
      </div>
      <div className="timeline-bar" onClick={handleBarClick}>
        <div className="timeline-track">
          <div
            className="timeline-progress"
            style={{ width: `${progress}%` }}
          />
          <div
            className="timeline-playhead"
            style={{ left: `${progress}%` }}
          />
        </div>
      </div>
    </div>
  );
}
