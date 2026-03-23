import { useState } from "react";
import type { TeslaCamEvent, EventType } from "../types/events";
import "./InfoPanel.css";

interface InfoPanelProps {
  event: TeslaCamEvent | null;
  onDelete?: (eventId: number) => void;
  onBackup?: (eventId: number) => void;
}

const TYPE_LABELS: Record<EventType, string> = {
  sentry: "哨兵模式",
  saved: "手動保存",
  recent: "行車紀錄",
};

const TYPE_COLORS: Record<EventType, string> = {
  sentry: "var(--accent-red)",
  saved: "var(--accent-cyan)",
  recent: "#888",
};

function formatFileSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export default function InfoPanel({ event, onDelete, onBackup }: InfoPanelProps) {
  const [confirming, setConfirming] = useState(false);

  if (!event) {
    return (
      <aside className="info-panel-container">
        <div className="map-placeholder">
          <div className="map-placeholder-text">地圖</div>
          <div className="map-placeholder-hint">選擇事件後顯示位置</div>
        </div>
      </aside>
    );
  }

  const totalSize = event.clips.reduce((sum, c) => sum + c.fileSize, 0);

  return (
    <aside className="info-panel-container">
      {/* Map area placeholder */}
      <div className="map-placeholder">
        {event.gpsLat && event.gpsLon ? (
          <div className="map-placeholder-text">
            {event.gpsLat.toFixed(4)}, {event.gpsLon.toFixed(4)}
          </div>
        ) : (
          <>
            <div className="map-placeholder-text">地圖</div>
            <div className="map-placeholder-hint">無 GPS 資料</div>
          </>
        )}
      </div>

      {/* Event info */}
      <div className="info-section">
        <div className="info-title">事件資訊</div>
        <div className="info-row">
          <span className="info-key">類型</span>
          <span className="info-value" style={{ color: TYPE_COLORS[event.type] }}>
            {TYPE_LABELS[event.type]}
          </span>
        </div>
        <div className="info-row">
          <span className="info-key">時間</span>
          <span className="info-value mono">
            {new Date(event.timestamp).toLocaleString("zh-TW")}
          </span>
        </div>
        <div className="info-row">
          <span className="info-key">時長</span>
          <span className="info-value mono">
            {Math.floor(event.durationSec / 60)}:{String(Math.round(event.durationSec % 60)).padStart(2, "0")}
          </span>
        </div>
        <div className="info-row">
          <span className="info-key">鏡頭</span>
          <span className="info-value mono">{event.clips.length}</span>
        </div>
        <div className="info-row">
          <span className="info-key">檔案大小</span>
          <span className="info-value mono">{formatFileSize(totalSize)}</span>
        </div>
        {event.avgSpeed !== null && (
          <div className="info-row">
            <span className="info-key">車速</span>
            <span className="info-value mono">{Math.round(event.avgSpeed)} km/h</span>
          </div>
        )}
        <div className="info-row">
          <span className="info-key">備份</span>
          <span className="info-value" style={{ color: event.backedUp ? "var(--accent-cyan)" : "var(--text-muted)" }}>
            {event.backedUp ? "已備份" : "未備份"}
          </span>
        </div>
      </div>

      {/* Quick actions */}
      <div className="info-section">
        <div className="info-title">快速操作</div>
        <div className="quick-actions">
          <button className="action-btn" disabled>
            匯出帶水印影片 (即將推出)
          </button>
          <button
            className="action-btn"
            onClick={() => onBackup?.(event.id)}
          >
            備份到本機
          </button>
          {!confirming ? (
            <button
              className="action-btn action-btn-danger"
              onClick={() => setConfirming(true)}
            >
              刪除此事件
            </button>
          ) : (
            <div className="confirm-group">
              <button
                className="action-btn action-btn-danger"
                onClick={() => {
                  onDelete?.(event.id);
                  setConfirming(false);
                }}
              >
                確認刪除
              </button>
              <button
                className="action-btn"
                onClick={() => setConfirming(false)}
              >
                取消
              </button>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
