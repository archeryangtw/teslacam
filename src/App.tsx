import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import Sidebar from "./components/Sidebar";
import VideoGrid, { type VideoGridHandle } from "./components/VideoGrid";
import Timeline from "./components/Timeline";
import TelemetryOverlay from "./components/TelemetryOverlay";
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
  const [showTelemetry, setShowTelemetry] = useState(true);
  const [markIn, setMarkIn] = useState<number | null>(null);
  const [markOut, setMarkOut] = useState<number | null>(null);
  const [exporting, setExporting] = useState(false);

  const videoGridRef = useRef<VideoGridHandle>(null);

  const handleSelectEvent = useCallback((event: TeslaCamEvent) => {
    setSelectedEvent(event);
    setCurrentTime(0);
    setIsPlaying(false);
    setMarkIn(null);
    setMarkOut(null);
  }, []);

  const handleCameraClick = useCallback((camera: string) => {
    setActiveCamera(camera);
  }, []);

  const handleFrameStep = useCallback((direction: 1 | -1) => {
    setIsPlaying(false);
    videoGridRef.current?.frameStep(direction);
  }, []);

  const handleSetMarkIn = useCallback(() => {
    setMarkIn(currentTime);
    if (markOut !== null && currentTime >= markOut) setMarkOut(null);
  }, [currentTime, markOut]);

  const handleSetMarkOut = useCallback(() => {
    setMarkOut(currentTime);
    if (markIn !== null && currentTime <= markIn) setMarkIn(null);
  }, [currentTime, markIn]);

  const handleClearMarks = useCallback(() => {
    setMarkIn(null);
    setMarkOut(null);
  }, []);

  const handleDelete = useCallback(
    async (eventId: number) => {
      await deleteEvent(eventId, true);
      if (selectedEvent?.id === eventId) setSelectedEvent(null);
    },
    [deleteEvent, selectedEvent]
  );

  const handleBackup = useCallback(
    async (eventId: number) => {
      const count = await backupEvent(eventId);
      if (count && count > 0) {
        setSelectedEvent((prev) =>
          prev?.id === eventId ? { ...prev, backedUp: true } : prev
        );
      }
    },
    [backupEvent]
  );

  const handleExport = useCallback(
    async (eventId: number) => {
      const event = events.find((e) => e.id === eventId);
      if (!event) return;

      // 如果有標記時間段，用標記的範圍
      const startTime = markIn ?? 0;
      const endTime = markOut ?? duration;
      const rangeText = markIn !== null || markOut !== null
        ? `_${Math.floor(startTime)}s-${Math.floor(endTime)}s`
        : "";

      const selected = await saveDialog({
        title: markIn !== null || markOut !== null
          ? `匯出選定時間段 (${(endTime - startTime).toFixed(1)}秒)`
          : "匯出六鏡頭環景影片",
        defaultPath: `teslacam-${event.timestamp.replace(/[T:]/g, "-")}${rangeText}.mp4`,
        filters: [{ name: "MP4", extensions: ["mp4"] }],
      });
      if (!selected) return;

      setExporting(true);
      try {
        await invoke("export_surround_video", {
          eventId,
          outputPath: selected,
          startTime: markIn !== null ? startTime : null,
          endTime: markOut !== null ? endTime : null,
        });
        alert(`匯出完成：${selected}`);
      } catch (e) {
        alert(`匯出失敗：${e}`);
      } finally {
        setExporting(false);
      }
    },
    [events, markIn, markOut, duration]
  );

  return (
    <div className="app-layout">
      <header className="topbar">
        <div className="topbar-logo">TESLACAM</div>
        <div className="topbar-actions">
          {exporting && <span className="scan-status" style={{ color: "var(--accent-cyan)" }}>匯出中...</span>}
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

      {error && <div className="error-bar">{error}</div>}

      <div className="main-content">
        <Sidebar
          events={events}
          selectedEvent={selectedEvent}
          onSelectEvent={handleSelectEvent}
          onSelectFolder={selectAndScan}
          onDelete={handleDelete}
          onBackup={handleBackup}
          onExport={handleExport}
          rootDir={rootDir}
        />

        <div className="center-panel">
          <div className="video-area">
            <VideoGrid
              ref={videoGridRef}
              clips={selectedEvent?.clips ?? []}
              activeCamera={activeCamera}
              onCameraClick={handleCameraClick}
              isPlaying={isPlaying}
              currentTime={currentTime}
              playbackRate={playbackRate}
              onTimeUpdate={setCurrentTime}
              onDurationChange={setDuration}
            />
            <TelemetryOverlay
              clips={selectedEvent?.clips ?? []}
              currentTime={currentTime}
              visible={showTelemetry}
              onToggle={() => setShowTelemetry(!showTelemetry)}
            />
          </div>
          <Timeline
            event={selectedEvent}
            currentTime={currentTime}
            duration={duration}
            isPlaying={isPlaying}
            playbackRate={playbackRate}
            markIn={markIn}
            markOut={markOut}
            onSeek={setCurrentTime}
            onPlayPause={() => setIsPlaying(!isPlaying)}
            onPlaybackRateChange={setPlaybackRate}
            onFrameStep={handleFrameStep}
            onSetMarkIn={handleSetMarkIn}
            onSetMarkOut={handleSetMarkOut}
            onClearMarks={handleClearMarks}
          />
        </div>
      </div>
    </div>
  );
}

export default App;
