import { useState, useCallback } from "react";
import Sidebar from "./components/Sidebar";
import VideoGrid from "./components/VideoGrid";
import Timeline from "./components/Timeline";
import InfoPanel from "./components/InfoPanel";
import { useTeslaCam } from "./hooks/useTeslaCam";
import type { TeslaCamEvent } from "./types/events";
import "./styles/app.css";

function App() {
  const {
    rootDir,
    events,
    scanning,
    scanResult,
    error,
    selectAndScan,
    deleteEvent,
    backupEvent,
  } = useTeslaCam();

  const [selectedEvent, setSelectedEvent] = useState<TeslaCamEvent | null>(null);
  const [activeCamera, setActiveCamera] = useState<string>("front");
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [playbackRate, setPlaybackRate] = useState(1);

  const handleSelectEvent = useCallback((event: TeslaCamEvent) => {
    setSelectedEvent(event);
    setCurrentTime(0);
    setIsPlaying(false);
  }, []);

  const handleCameraClick = useCallback((camera: string) => {
    setActiveCamera(camera);
  }, []);

  const handleDelete = useCallback(
    async (eventId: number) => {
      await deleteEvent(eventId, true);
      if (selectedEvent?.id === eventId) {
        setSelectedEvent(null);
      }
    },
    [deleteEvent, selectedEvent]
  );

  const handleBackup = useCallback(
    async (eventId: number) => {
      const count = await backupEvent(eventId);
      if (count && count > 0) {
        // 更新 selectedEvent 的 backedUp 狀態
        setSelectedEvent((prev) =>
          prev?.id === eventId ? { ...prev, backedUp: true } : prev
        );
      }
    },
    [backupEvent]
  );

  return (
    <div className="app-layout">
      {/* Top bar */}
      <header className="topbar">
        <div className="topbar-logo">TESLACAM</div>
        <div className="topbar-tabs">
          <button className="tab active">事件檢視</button>
          <button className="tab" disabled>地圖總覽</button>
          <button className="tab" disabled>匯出</button>
        </div>
        <div className="topbar-actions">
          {scanning && <span className="scan-status">掃描中...</span>}
          {scanResult && !scanning && (
            <span className="scan-status">
              {scanResult.total_events} 個事件 · {scanResult.total_clips} 個片段
            </span>
          )}
          <button className="btn" onClick={selectAndScan} disabled={scanning}>
            {rootDir ? "重新掃描" : "選擇 TeslaCam 資料夾"}
          </button>
        </div>
      </header>

      {error && (
        <div className="error-bar">
          {error}
          <button className="error-close" onClick={() => {}}>✕</button>
        </div>
      )}

      {/* Main content */}
      <div className="main-content">
        <Sidebar
          events={events}
          selectedEvent={selectedEvent}
          onSelectEvent={handleSelectEvent}
          rootDir={rootDir}
        />

        <div className="center-panel">
          <VideoGrid
            clips={selectedEvent?.clips ?? []}
            activeCamera={activeCamera}
            onCameraClick={handleCameraClick}
            isPlaying={isPlaying}
            currentTime={currentTime}
            playbackRate={playbackRate}
            onTimeUpdate={setCurrentTime}
            onDurationChange={setDuration}
          />
          <Timeline
            event={selectedEvent}
            currentTime={currentTime}
            duration={duration}
            isPlaying={isPlaying}
            playbackRate={playbackRate}
            onSeek={setCurrentTime}
            onPlayPause={() => setIsPlaying(!isPlaying)}
            onPlaybackRateChange={setPlaybackRate}
          />
        </div>

        <InfoPanel
          event={selectedEvent}
          onDelete={handleDelete}
          onBackup={handleBackup}
        />
      </div>
    </div>
  );
}

export default App;
