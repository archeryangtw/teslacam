import type { TeslaCamEvent, EventType } from "../types/events";
import "./Sidebar.css";

interface SidebarProps {
  events: TeslaCamEvent[];
  selectedEvent: TeslaCamEvent | null;
  onSelectEvent: (event: TeslaCamEvent) => void;
  rootDir: string | null;
}

const TYPE_CONFIG: Record<EventType, { label: string; dotClass: string }> = {
  sentry: { label: "哨兵事件", dotClass: "dot-sentry" },
  saved: { label: "手動保存", dotClass: "dot-saved" },
  recent: { label: "行車紀錄", dotClass: "dot-recent" },
};

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const min = String(d.getMinutes()).padStart(2, "0");
  return `${mm}/${dd} ${hh}:${min}`;
}

function groupByType(events: TeslaCamEvent[]) {
  const groups: Record<EventType, TeslaCamEvent[]> = {
    sentry: [],
    saved: [],
    recent: [],
  };
  for (const e of events) {
    groups[e.type].push(e);
  }
  return groups;
}

export default function Sidebar({ events, selectedEvent, onSelectEvent, rootDir }: SidebarProps) {
  const grouped = groupByType(events);

  if (!rootDir) {
    return (
      <aside className="sidebar">
        <div className="sidebar-empty">
          <div className="sidebar-empty-icon">📁</div>
          <p>選擇 TeslaCam 資料夾以開始</p>
          <p className="sidebar-empty-hint">
            通常位於 USB 磁碟的 TeslaCam 目錄
          </p>
        </div>
      </aside>
    );
  }

  return (
    <aside className="sidebar">
      {(["sentry", "saved", "recent"] as EventType[]).map((type) => {
        const items = grouped[type];
        if (items.length === 0) return null;
        const config = TYPE_CONFIG[type];

        return (
          <div className="sidebar-section" key={type}>
            <div className="sidebar-title">
              {config.label} ({items.length})
            </div>
            {items.map((event) => (
              <div
                key={event.id}
                className={`event-item ${selectedEvent?.id === event.id ? "active" : ""}`}
                onClick={() => onSelectEvent(event)}
              >
                <div className={`dot ${config.dotClass}`} />
                <div className="event-meta">
                  <div className="event-time">{formatTimestamp(event.timestamp)}</div>
                  <div className="event-label">
                    {event.clips.length} 鏡頭 · {Math.round(event.durationSec / 60) || 1} 分鐘
                  </div>
                </div>
                {event.avgSpeed !== null && (
                  <span className="speed-badge">{Math.round(event.avgSpeed)} km/h</span>
                )}
              </div>
            ))}
          </div>
        );
      })}
    </aside>
  );
}
