import { useRef, useEffect, useCallback } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Clip, CameraAngle } from "../types/events";
import "./VideoGrid.css";

interface VideoGridProps {
  clips: Clip[];
  activeCamera: string;
  onCameraClick: (camera: string) => void;
  isPlaying: boolean;
  currentTime: number;
  playbackRate: number;
  onTimeUpdate?: (time: number) => void;
  onDurationChange?: (duration: number) => void;
}

const CAMERA_LABELS: Record<CameraAngle, string> = {
  front: "前方",
  back: "後方",
  left_repeater: "左後",
  right_repeater: "右後",
  left_pillar: "左柱",
  right_pillar: "右柱",
};

const GRID_ORDER: CameraAngle[] = [
  "left_repeater",
  "front",
  "right_repeater",
  "left_pillar",
  "back",
  "right_pillar",
];

export default function VideoGrid({
  clips,
  activeCamera,
  onCameraClick,
  isPlaying,
  currentTime,
  playbackRate,
  onTimeUpdate,
  onDurationChange,
}: VideoGridProps) {
  const videoRefs = useRef<Map<string, HTMLVideoElement>>(new Map());
  const isSyncing = useRef(false);

  const setVideoRef = useCallback((camera: string, el: HTMLVideoElement | null) => {
    if (el) {
      videoRefs.current.set(camera, el);
    } else {
      videoRefs.current.delete(camera);
    }
  }, []);

  // 主鏡頭 timeupdate → 同步其他鏡頭 + 通知外部
  const handleTimeUpdate = useCallback(() => {
    if (isSyncing.current) return;
    const masterVideo = videoRefs.current.get(activeCamera);
    if (!masterVideo) return;

    const t = masterVideo.currentTime;
    onTimeUpdate?.(t);

    // 同步其他鏡頭（允許 ≤100ms 偏移）
    isSyncing.current = true;
    videoRefs.current.forEach((video, cam) => {
      if (cam !== activeCamera && Math.abs(video.currentTime - t) > 0.1) {
        video.currentTime = t;
      }
    });
    isSyncing.current = false;
  }, [activeCamera, onTimeUpdate]);

  // 主鏡頭 loadedmetadata → 通知時長
  const handleLoadedMetadata = useCallback(
    (e: React.SyntheticEvent<HTMLVideoElement>) => {
      onDurationChange?.(e.currentTarget.duration);
    },
    [onDurationChange]
  );

  // 播放/暫停同步
  useEffect(() => {
    videoRefs.current.forEach((video) => {
      if (isPlaying) {
        video.play().catch(() => {});
      } else {
        video.pause();
      }
    });
  }, [isPlaying]);

  // 播放速率同步
  useEffect(() => {
    videoRefs.current.forEach((video) => {
      video.playbackRate = playbackRate;
    });
  }, [playbackRate]);

  // 外部 seek（來自 Timeline 點擊）
  useEffect(() => {
    videoRefs.current.forEach((video) => {
      if (Math.abs(video.currentTime - currentTime) > 0.3) {
        video.currentTime = currentTime;
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

  const clipMap = new Map(clips.map((c) => [c.camera, c]));
  const availableCameras = GRID_ORDER.filter((cam) => clipMap.has(cam));

  return (
    <div className={`video-grid grid-${availableCameras.length}`}>
      {availableCameras.map((camera) => {
        const clip = clipMap.get(camera)!;
        const isMain = camera === activeCamera;
        const videoSrc = convertFileSrc(clip.filePath);

        return (
          <div
            key={camera}
            className={`cam ${camera === "front" ? "cam-front" : ""} ${isMain ? "cam-active" : ""}`}
            onClick={() => onCameraClick(camera)}
          >
            <div className="cam-label">{CAMERA_LABELS[camera]}</div>
            <video
              ref={(el) => setVideoRef(camera, el)}
              src={videoSrc}
              muted
              playsInline
              preload="metadata"
              onTimeUpdate={isMain ? handleTimeUpdate : undefined}
              onLoadedMetadata={isMain ? handleLoadedMetadata : undefined}
            />
            {isMain && <div className="cam-active-indicator" />}
          </div>
        );
      })}
    </div>
  );
}
