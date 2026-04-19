import { useRef, useEffect, useCallback, useMemo, useState, forwardRef, useImperativeHandle } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import type { Clip, CameraAngle, TeslaCamEvent } from "../types/events";

interface TelemetryFrame {
  time_sec: number;
  lat: number;
  lon: number;
  speed_kmh: number;
  heading: number;
}
import "./VideoGrid.css";

export interface VideoGridHandle {
  frameStep: (direction: 1 | -1) => void;
  getVideoRefs: () => Map<string, HTMLVideoElement>;
}

interface VideoGridProps {
  clips: Clip[];
  activeCamera: string;
  onCameraClick: (camera: string) => void;
  isPlaying: boolean;
  currentTime: number;
  playbackRate: number;
  onTimeUpdate?: (time: number) => void;
  onDurationChange?: (duration: number) => void;
  event?: TeslaCamEvent | null;
  telemetryTrack?: TelemetryFrame[];
  onExportCamera?: (camera: CameraAngle) => void;
}

const CAMERA_LABELS: Record<CameraAngle, string> = {
  front: "前方",
  back: "後方",
  left_repeater: "左後",
  right_repeater: "右後",
  left_pillar: "左柱",
  right_pillar: "右柱",
};

// 環景佈局順序
const GRID_ORDER: CameraAngle[] = [
  "left_pillar",
  "front",
  "right_pillar",
  "left_repeater",
  "back",
  "right_repeater",
];

/** 將 clips 按 camera → segment 順序整理 */
function buildCameraSegments(clips: Clip[]) {
  const map = new Map<string, Clip[]>();
  for (const clip of clips) {
    const list = map.get(clip.camera) ?? [];
    list.push(clip);
    map.set(clip.camera, list);
  }
  // 每個 camera 的 clips 按 segmentIndex 排序
  for (const [, list] of map) {
    list.sort((a, b) => a.segmentIndex - b.segmentIndex);
  }
  return map;
}

const FPS = 36;

const VideoGrid = forwardRef<VideoGridHandle, VideoGridProps>(function VideoGrid({
  clips,
  activeCamera,
  onCameraClick,
  isPlaying,
  currentTime,
  playbackRate,
  onTimeUpdate,
  onDurationChange,
  event,
  telemetryTrack,
  onExportCamera,
}: VideoGridProps, ref) {
  const videoRefs = useRef<Map<string, HTMLVideoElement>>(new Map());
  const isSyncing = useRef(false);

  useImperativeHandle(ref, () => ({
    frameStep(direction: 1 | -1) {
      const frontVideo = videoRefs.current.get("front");
      if (!frontVideo) return;
      frontVideo.pause();
      const step = direction / FPS;
      frontVideo.currentTime = Math.max(0, frontVideo.currentTime + step);
      const t = frontVideo.currentTime;
      videoRefs.current.forEach((video, cam) => {
        if (cam !== "front") video.currentTime = t;
      });
    },
    getVideoRefs() {
      return videoRefs.current;
    },
  }), []);

  // 按 camera 分組，每個 camera 可能有多個 segment
  const cameraSegments = useMemo(() => buildCameraSegments(clips), [clips]);

  // 追蹤每個 camera 目前播放的 segment index
  const [segmentIndexes, setSegmentIndexes] = useState<Map<string, number>>(
    new Map()
  );

  // 雙擊放大的鏡頭；null = 六鏡頭環景
  const [zoomedCamera, setZoomedCamera] = useState<string | null>(null);

  // clips 切換（換事件）時重設放大狀態，避免放大到不存在的鏡頭
  useEffect(() => {
    setZoomedCamera(null);
  }, [cameraSegments]);

  const handleCamDoubleClick = useCallback((camera: string) => {
    setZoomedCamera((prev) => (prev === camera ? null : camera));
  }, []);

  const handleSnapshot = useCallback(async () => {
    if (!zoomedCamera) return;
    const video = videoRefs.current.get(zoomedCamera);
    if (!video || !video.videoWidth) return;

    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // 先畫影片畫面（後鏡頭要翻轉成正向）
    if (zoomedCamera === "back") {
      ctx.save();
      ctx.translate(canvas.width, 0);
      ctx.scale(-1, 1);
      ctx.drawImage(video, 0, 0);
      ctx.restore();
    } else {
      ctx.drawImage(video, 0, 0);
    }

    // 疊上時間戳 + GPS
    const lines: string[] = [];
    if (event) {
      const iso = event.timestamp;
      const d = new Date(iso.includes("+") || iso.includes("Z") ? iso : iso + "+08:00");
      d.setSeconds(d.getSeconds() + Math.floor(currentTime));
      const pad = (n: number) => String(n).padStart(2, "0");
      lines.push(
        `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
        `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
      );
    }
    if (telemetryTrack && telemetryTrack.length > 0) {
      // 找最接近 currentTime 的 GPS 點
      let closest = telemetryTrack[0];
      let minDt = Math.abs(closest.time_sec - currentTime);
      for (const p of telemetryTrack) {
        const dt = Math.abs(p.time_sec - currentTime);
        if (dt < minDt) {
          minDt = dt;
          closest = p;
        }
      }
      if (closest.lat !== 0 || closest.lon !== 0) {
        lines.push(`${closest.lat.toFixed(6)}, ${closest.lon.toFixed(6)}`);
      }
    }

    if (lines.length > 0) {
      const fs = Math.max(18, Math.round(canvas.height / 32));
      ctx.font = `bold ${fs}px -apple-system, "Helvetica Neue", sans-serif`;
      ctx.textBaseline = "top";
      const pad2 = Math.round(fs * 0.4);
      const lineH = fs + Math.round(fs * 0.25);
      const boxW = Math.max(...lines.map((l) => ctx.measureText(l).width)) + pad2 * 2;
      const boxH = lineH * lines.length + pad2 * 2 - Math.round(fs * 0.25);
      ctx.fillStyle = "rgba(0, 0, 0, 0.55)";
      ctx.fillRect(12, 12, boxW, boxH);
      ctx.fillStyle = "#fff";
      ctx.shadowColor = "rgba(0,0,0,0.8)";
      ctx.shadowBlur = 2;
      lines.forEach((l, i) => {
        ctx.fillText(l, 12 + pad2, 12 + pad2 + i * lineH);
      });
      ctx.shadowBlur = 0;
    }

    const blob: Blob | null = await new Promise((resolve) =>
      canvas.toBlob((b) => resolve(b), "image/png")
    );
    if (!blob) return;
    const bytes = new Uint8Array(await blob.arrayBuffer());

    const ts = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
    const defaultPath = `teslacam-${zoomedCamera}-${ts}.png`;
    try {
      const selected = await saveDialog({
        title: "儲存截圖",
        defaultPath,
        filters: [{ name: "PNG", extensions: ["png"] }],
      });
      if (!selected) return;
      await writeFile(selected, bytes);
      alert(`截圖已儲存：${selected}`);
    } catch (e) {
      alert(`截圖失敗：${e}`);
    }
  }, [zoomedCamera, event, currentTime, telemetryTrack]);

  // 當 clips 改變時重設 segment indexes
  useEffect(() => {
    const init = new Map<string, number>();
    for (const cam of cameraSegments.keys()) {
      init.set(cam, 0);
    }
    setSegmentIndexes(init);
  }, [cameraSegments]);

  const setVideoRef = useCallback(
    (camera: string, el: HTMLVideoElement | null) => {
      if (el) videoRefs.current.set(camera, el);
      else videoRefs.current.delete(camera);
    },
    []
  );

  // 永遠用 front 鏡頭作為時間基準（SEI 遙測來自 front）
  const frontClips = useMemo(
    () => cameraSegments.get("front") ?? [],
    [cameraSegments]
  );

  const totalDuration = useMemo(() => {
    return frontClips.reduce((sum, c) => sum + c.durationSec, 0);
  }, [frontClips]);

  useEffect(() => {
    if (totalDuration > 0) onDurationChange?.(totalDuration);
  }, [totalDuration, onDurationChange]);

  // 以 front 鏡頭的影片時間作為唯一時間基準
  const handleTimeUpdate = useCallback(() => {
    if (isSyncing.current) return;

    // 永遠從 front 影片讀取時間
    const frontVideo = videoRefs.current.get("front");
    if (!frontVideo) return;

    const segIdx = segmentIndexes.get("front") ?? 0;
    let elapsed = 0;
    for (let i = 0; i < segIdx; i++) {
      elapsed += frontClips[i]?.durationSec ?? 60;
    }
    elapsed += frontVideo.currentTime;
    onTimeUpdate?.(elapsed);

    // 同步所有鏡頭到 front 的播放位置
    isSyncing.current = true;
    const t = frontVideo.currentTime;
    videoRefs.current.forEach((video, cam) => {
      if (cam !== "front" && Math.abs(video.currentTime - t) > 0.15) {
        video.currentTime = t;
      }
    });
    isSyncing.current = false;
  }, [onTimeUpdate, segmentIndexes, frontClips]);

  // 當一個 segment 播放結束，自動切到下一個
  const handleEnded = useCallback(
    (camera: string) => {
      const camClips = cameraSegments.get(camera);
      if (!camClips) return;
      const curIdx = segmentIndexes.get(camera) ?? 0;
      const nextIdx = curIdx + 1;

      if (nextIdx < camClips.length) {
        // 切換所有 camera 到下一個 segment
        setSegmentIndexes((prev) => {
          const next = new Map(prev);
          for (const cam of cameraSegments.keys()) {
            const ci = next.get(cam) ?? 0;
            const camList = cameraSegments.get(cam) ?? [];
            if (ci + 1 < camList.length) {
              next.set(cam, ci + 1);
            }
          }
          return next;
        });
      }
    },
    [cameraSegments, segmentIndexes]
  );

  // 新 segment 載入後自動播放
  useEffect(() => {
    if (!isPlaying) return;
    // 短暫延遲讓 video src 更新
    const timer = setTimeout(() => {
      videoRefs.current.forEach((video) => {
        video.play().catch(() => {});
      });
    }, 50);
    return () => clearTimeout(timer);
  }, [segmentIndexes, isPlaying]);

  // 播放/暫停
  useEffect(() => {
    videoRefs.current.forEach((video) => {
      if (isPlaying) video.play().catch(() => {});
      else video.pause();
    });
  }, [isPlaying]);

  // 播放速率
  useEffect(() => {
    videoRefs.current.forEach((video) => {
      video.playbackRate = playbackRate;
    });
  }, [playbackRate]);

  // 外部 seek（用 front 時長基準）
  useEffect(() => {
    if (frontClips.length === 0) return;

    const mainClips = frontClips;
    let remaining = currentTime;
    let targetSeg = 0;
    for (let i = 0; i < mainClips.length; i++) {
      if (remaining <= mainClips[i].durationSec) {
        targetSeg = i;
        break;
      }
      remaining -= mainClips[i].durationSec;
      targetSeg = i + 1;
    }
    if (targetSeg >= mainClips.length) {
      targetSeg = mainClips.length - 1;
      remaining = mainClips[targetSeg]?.durationSec ?? 0;
    }

    const curSeg = segmentIndexes.get("front") ?? 0;
    if (targetSeg !== curSeg) {
      setSegmentIndexes((prev) => {
        const next = new Map(prev);
        for (const cam of cameraSegments.keys()) {
          next.set(cam, targetSeg);
        }
        return next;
      });
    }

    videoRefs.current.forEach((video) => {
      if (Math.abs(video.currentTime - remaining) > 0.3) {
        video.currentTime = remaining;
      }
    });
  }, [currentTime]);

  if (clips.length === 0) {
    return (
      <div className="video-grid-empty">
        <div className="video-grid-empty-text">選擇一個事件來播放影片</div>
      </div>
    );
  }

  const availableCameras = GRID_ORDER.filter((cam) => cameraSegments.has(cam));

  const isZoomed = zoomedCamera !== null && availableCameras.includes(zoomedCamera as CameraAngle);

  return (
    <div className={`video-grid grid-${availableCameras.length}${isZoomed ? " zoomed" : ""}`}>
      {isZoomed && (
        <div className="zoom-toolbar">
          <button
            className="snapshot-btn"
            onClick={handleSnapshot}
            title="截圖目前畫面"
          >
            截圖
          </button>
          {onExportCamera && zoomedCamera && (
            <button
              className="snapshot-btn"
              onClick={() => onExportCamera(zoomedCamera as CameraAngle)}
              title="匯出此鏡頭影片（含時間段標記）"
            >
              匯出此鏡頭
            </button>
          )}
        </div>
      )}
      {availableCameras.map((camera) => {
        const camClips = cameraSegments.get(camera)!;
        const segIdx = Math.min(segmentIndexes.get(camera) ?? 0, camClips.length - 1);
        const currentClip = camClips[Math.max(0, segIdx)];
        if (!currentClip) return null;
        const isMain = camera === activeCamera;
        const isCamZoomed = zoomedCamera === camera;
        const videoSrc = convertFileSrc(currentClip.filePath);

        return (
          <div
            key={camera}
            className={`cam cam-${camera.replace("_", "-")} ${isMain ? "cam-active" : ""} ${isCamZoomed ? "cam-zoomed" : ""}`}
            onClick={() => onCameraClick(camera)}
            onDoubleClick={() => handleCamDoubleClick(camera)}
          >
            <div className="cam-label">
              {CAMERA_LABELS[camera]}
              {camClips.length > 1 && (
                <span className="cam-segment">
                  {" "}
                  {segIdx + 1}/{camClips.length}
                </span>
              )}
            </div>
            <video
              key={`${camera}-${segIdx}`}
              ref={(el) => setVideoRef(camera, el)}
              src={videoSrc}
              muted
              playsInline
              preload="metadata"
              onTimeUpdate={camera === "front" ? handleTimeUpdate : undefined}
              onEnded={() => handleEnded(camera)}
              className={camera === "back" ? "mirror" : ""}
            />
            {isMain && <div className="cam-active-indicator" />}
          </div>
        );
      })}
    </div>
  );
});

export default VideoGrid;
