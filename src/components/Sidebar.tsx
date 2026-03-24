import { useState } from "react";
import type { TeslaCamEvent, EventType } from "../types/events";
import "./Sidebar.css";

interface SidebarProps {
  events: TeslaCamEvent[];
  selectedEvent: TeslaCamEvent | null;
  onSelectEvent: (event: TeslaCamEvent) => void;
  onSelectFolder?: () => void;
  onDelete?: (eventId: number) => void;
  onBackup?: (eventId: number) => void;
  onExport?: (eventId: number) => void;
  onReport?: (eventId: number) => void;
  rootDir: string | null;
}

const TYPE_CONFIG: Record<EventType, { label: string; dotClass: string; typeLabel: string }> = {
  sentry: { label: "哨兵事件", dotClass: "dot-sentry", typeLabel: "哨兵模式" },
  saved: { label: "手動保存", dotClass: "dot-saved", typeLabel: "手動保存" },
  recent: { label: "行車紀錄", dotClass: "dot-recent", typeLabel: "行車紀錄" },
};

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const min = String(d.getMinutes()).padStart(2, "0");
  return `${mm}/${dd} ${hh}:${min}`;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.round(sec % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

function groupByType(events: TeslaCamEvent[]) {
  const groups: Record<EventType, TeslaCamEvent[]> = { sentry: [], saved: [], recent: [] };
  for (const e of events) groups[e.type].push(e);
  return groups;
}

function uniqueCameras(event: TeslaCamEvent): number {
  return new Set(event.clips.map((c) => c.camera)).size;
}

function segmentCount(event: TeslaCamEvent): number {
  return Math.max(...event.clips.map((c) => c.segmentIndex)) + 1;
}

export default function Sidebar({
  events,
  selectedEvent,
  onSelectEvent,
  onSelectFolder,
  onDelete,
  onBackup,
  onExport,
  onReport,
  rootDir,
}: SidebarProps) {
  const grouped = groupByType(events);
  const [confirming, setConfirming] = useState(false);

  if (!rootDir) {
    return (
      <aside className="sidebar">
        <div className="sidebar-empty" onClick={onSelectFolder} style={{ cursor: "pointer" }}>
          <div className="sidebar-empty-icon">📁</div>
          <p>選擇 TeslaCam 資料夾以開始</p>
          <p className="sidebar-empty-hint">點擊此處或右上角按鈕選擇資料夾</p>
        </div>
      </aside>
    );
  }

  const sel = selectedEvent;
  const totalSize = sel ? sel.clips.reduce((s, c) => s + c.fileSize, 0) : 0;

  return (
    <aside className="sidebar">
      {/* 事件列表 */}
      <div className="sidebar-events">
        {(["sentry", "saved", "recent"] as EventType[]).map((type) => {
          const items = grouped[type];
          if (items.length === 0) return null;
          const config = TYPE_CONFIG[type];
          return (
            <div className="sidebar-section" key={type}>
              <div className="sidebar-title">{config.label} ({items.length})</div>
              {items.map((event) => (
                <div
                  key={event.id}
                  className={`event-item ${sel?.id === event.id ? "active" : ""}`}
                  onClick={() => { onSelectEvent(event); setConfirming(false); }}
                >
                  <div className={`dot ${config.dotClass}`} />
                  <div className="event-meta">
                    <div className="event-time">{formatTimestamp(event.timestamp)}</div>
                    <div className="event-label">
                      {uniqueCameras(event)} 鏡頭 · {formatDuration(event.durationSec)}
                      {segmentCount(event) > 1 && ` · ${segmentCount(event)} 段`}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          );
        })}
      </div>

      {/* 選中事件的資訊面板 */}
      {sel && (
        <div className="sidebar-info">
          <div className="sidebar-title">事件資訊</div>
          <div className="info-row">
            <span className="info-key">類型</span>
            <span className="info-value" style={{ color: sel.type === "sentry" ? "var(--accent-red)" : sel.type === "saved" ? "var(--accent-cyan)" : "#888" }}>
              {TYPE_CONFIG[sel.type].typeLabel}
            </span>
          </div>
          <div className="info-row">
            <span className="info-key">時間</span>
            <span className="info-value mono">{new Date(sel.timestamp).toLocaleString("zh-TW")}</span>
          </div>
          <div className="info-row">
            <span className="info-key">時長</span>
            <span className="info-value mono">{formatDuration(sel.durationSec)}</span>
          </div>
          <div className="info-row">
            <span className="info-key">鏡頭</span>
            <span className="info-value mono">{uniqueCameras(sel)}</span>
          </div>
          {segmentCount(sel) > 1 && (
            <div className="info-row">
              <span className="info-key">片段</span>
              <span className="info-value mono">{segmentCount(sel)} 段</span>
            </div>
          )}
          <div className="info-row">
            <span className="info-key">大小</span>
            <span className="info-value mono">{formatFileSize(totalSize)}</span>
          </div>
          <div className="info-row">
            <span className="info-key">備份</span>
            <span className="info-value" style={{ color: sel.backedUp ? "var(--accent-cyan)" : "var(--text-muted)" }}>
              {sel.backedUp ? "已備份" : "未備份"}
            </span>
          </div>

          <div className="sidebar-actions">
            <button className="action-btn action-btn-export" onClick={() => onExport?.(sel.id)}>匯出六鏡頭影片</button>
            <button className="action-btn" onClick={() => onReport?.(sel.id)}>匯出事件報告</button>
            <button className="action-btn" onClick={() => onBackup?.(sel.id)}>備份到本機</button>
            {!confirming ? (
              <button className="action-btn action-btn-danger" onClick={() => setConfirming(true)}>刪除此事件</button>
            ) : (
              <div className="confirm-group">
                <button className="action-btn action-btn-danger" onClick={() => { onDelete?.(sel.id); setConfirming(false); }}>確認刪除</button>
                <button className="action-btn" onClick={() => setConfirming(false)}>取消</button>
              </div>
            )}
          </div>
        </div>
      )}
    </aside>
  );
}
