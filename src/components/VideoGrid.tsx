import { useRef, useEffect, useCallback, useMemo, useState, forwardRef, useImperativeHandle } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Clip, CameraAngle } from "../types/events";
import "./VideoGrid.css";

export interface VideoGridHandle {
  frameStep: (direction: 1 | -1) => void;
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
}: VideoGridProps, ref) {
  const videoRefs = useRef<Map<string, HTMLVideoElement>>(new Map());
  const isSyncing = useRef(false);

  // 逐幀步進：前進或後退一個影格 (1/FPS 秒)
  useImperativeHandle(ref, () => ({
    frameStep(direction: 1 | -1) {
      const frontVideo = videoRefs.current.get("front");
      if (!frontVideo) return;
      frontVideo.pause();
      const step = direction / FPS;
      frontVideo.currentTime = Math.max(0, frontVideo.currentTime + step);
      // 同步其他鏡頭
      const t = frontVideo.currentTime;
      videoRefs.current.forEach((video, cam) => {
        if (cam !== "front") video.currentTime = t;
      });
    },
  }), []);

  // 按 camera 分組，每個 camera 可能有多個 segment
  const cameraSegments = useMemo(() => buildCameraSegments(clips), [clips]);

  // 追蹤每個 camera 目前播放的 segment index
  const [segmentIndexes, setSegmentIndexes] = useState<Map<string, number>>(
    new Map()
  );

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

    const curSeg = segmentIndexes.get(activeCamera) ?? 0;
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

  return (
    <div className={`video-grid grid-${availableCameras.length}`}>
      {availableCameras.map((camera) => {
        const camClips = cameraSegments.get(camera)!;
        const segIdx = segmentIndexes.get(camera) ?? 0;
        const currentClip = camClips[Math.min(segIdx, camClips.length - 1)];
        const isMain = camera === activeCamera;
        const videoSrc = convertFileSrc(currentClip.filePath);

        return (
          <div
            key={camera}
            className={`cam cam-${camera.replace("_", "-")} ${isMain ? "cam-active" : ""}`}
            onClick={() => onCameraClick(camera)}
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
