import { useRef, useEffect, useCallback, useMemo } from "react";
import type { CameraAngle } from "../types/events";
import "./BirdEyeView.css";

interface BirdEyeViewProps {
  videoRefs: Map<string, HTMLVideoElement>;
  visible: boolean;
  onToggle: () => void;
}

/**
 * Tesla 鏡頭在車上的位置和朝向
 * x, y: 在俯瞰圖中的位置（0-1 歸一化，車中心=0.5,0.5）
 * rotation: 鏡頭朝向角度（0=上/前，順時針）
 * fov: 大致視角範圍（度）
 */
const CAMERA_CONFIG: Record<string, { x: number; y: number; rotation: number; mirror?: boolean }> = {
  front:          { x: 0.5,  y: 0.08, rotation: 0 },
  left_pillar:    { x: 0.12, y: 0.32, rotation: -45 },    // B柱朝前方
  right_pillar:   { x: 0.88, y: 0.32, rotation: 45 },
  left_repeater:  { x: 0.08, y: 0.25, rotation: -135 },   // 前葉子板朝後方
  right_repeater: { x: 0.92, y: 0.25, rotation: 135 },
  back:           { x: 0.5,  y: 0.92, rotation: 180, mirror: true },
};

const CANVAS_W = 400;
const CANVAS_H = 500;

// 車輛圖示尺寸（在 canvas 中）
const CAR_W = 80;
const CAR_H = 180;
const CAR_X = (CANVAS_W - CAR_W) / 2;
const CAR_Y = (CANVAS_H - CAR_H) / 2;

export default function BirdEyeView({ videoRefs, visible, onToggle }: BirdEyeViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // 清除
    ctx.fillStyle = "#0a0a1a";
    ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);

    // 繪製每個鏡頭的擷取畫面（扇形投影區域）
    const cameras: CameraAngle[] = ["front", "left_pillar", "right_pillar", "left_repeater", "right_repeater", "back"];

    for (const cam of cameras) {
      const video = videoRefs.get(cam);
      const config = CAMERA_CONFIG[cam];
      if (!video || !config || video.readyState < 2) continue;

      const cx = config.x * CANVAS_W;
      const cy = config.y * CANVAS_H;
      const radius = 90;
      const startAngle = ((config.rotation - 60) * Math.PI) / 180;
      const endAngle = ((config.rotation + 60) * Math.PI) / 180;

      ctx.save();
      // 建立扇形裁剪區域
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.arc(cx, cy, radius, startAngle - Math.PI / 2, endAngle - Math.PI / 2);
      ctx.closePath();
      ctx.clip();

      // 繪製影片畫面到扇形區域
      ctx.translate(cx, cy);
      ctx.rotate((config.rotation * Math.PI) / 180);
      if (config.mirror) {
        ctx.scale(-1, 1);
      }

      const vw = video.videoWidth || 640;
      const vh = video.videoHeight || 480;
      const drawSize = radius * 2;
      ctx.drawImage(video, -drawSize / 2, -drawSize * 0.1, drawSize, drawSize * (vh / vw));

      ctx.restore();

      // 扇形邊框
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.arc(cx, cy, radius, startAngle - Math.PI / 2, endAngle - Math.PI / 2);
      ctx.closePath();
      ctx.strokeStyle = "rgba(78, 205, 196, 0.3)";
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    // 繪製車輛圖示（覆蓋在上方）
    drawCar(ctx);

    animRef.current = requestAnimationFrame(draw);
  }, [videoRefs]);

  useEffect(() => {
    if (visible) {
      animRef.current = requestAnimationFrame(draw);
    }
    return () => {
      if (animRef.current) cancelAnimationFrame(animRef.current);
    };
  }, [visible, draw]);

  if (!visible) return null;

  return (
    <div className="birdeye-panel">
      <div className="birdeye-header">
        <span>鳥瞰檢視</span>
        <button className="birdeye-close" onClick={onToggle}>✕</button>
      </div>
      <canvas
        ref={canvasRef}
        width={CANVAS_W}
        height={CANVAS_H}
        className="birdeye-canvas"
      />
    </div>
  );
}

/** 繪製簡化的 Tesla 車輛俯瞰圖 */
function drawCar(ctx: CanvasRenderingContext2D) {
  const x = CAR_X;
  const y = CAR_Y;
  const w = CAR_W;
  const h = CAR_H;
  const r = 12;

  // 車身
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
  ctx.fillStyle = "rgba(30, 35, 55, 0.95)";
  ctx.fill();
  ctx.strokeStyle = "#4a4a6a";
  ctx.lineWidth = 1.5;
  ctx.stroke();

  // 擋風玻璃
  ctx.beginPath();
  ctx.moveTo(x + 8, y + 35);
  ctx.lineTo(x + w - 8, y + 35);
  ctx.lineTo(x + w - 12, y + 55);
  ctx.lineTo(x + 12, y + 55);
  ctx.closePath();
  ctx.fillStyle = "rgba(78, 205, 196, 0.15)";
  ctx.fill();
  ctx.strokeStyle = "rgba(78, 205, 196, 0.4)";
  ctx.lineWidth = 1;
  ctx.stroke();

  // 後擋風
  ctx.beginPath();
  ctx.moveTo(x + 12, y + h - 45);
  ctx.lineTo(x + w - 12, y + h - 45);
  ctx.lineTo(x + w - 8, y + h - 30);
  ctx.lineTo(x + 8, y + h - 30);
  ctx.closePath();
  ctx.fillStyle = "rgba(78, 205, 196, 0.1)";
  ctx.fill();
  ctx.strokeStyle = "rgba(78, 205, 196, 0.3)";
  ctx.stroke();

  // 車頂
  ctx.beginPath();
  ctx.moveTo(x + 12, y + 55);
  ctx.lineTo(x + w - 12, y + 55);
  ctx.lineTo(x + w - 12, y + h - 45);
  ctx.lineTo(x + 12, y + h - 45);
  ctx.closePath();
  ctx.fillStyle = "rgba(40, 45, 65, 0.6)";
  ctx.fill();

  // T 標誌
  ctx.fillStyle = "rgba(78, 205, 196, 0.5)";
  ctx.font = "bold 16px system-ui";
  ctx.textAlign = "center";
  ctx.fillText("T", x + w / 2, y + h / 2 + 5);

  // 車頭方向箭頭
  ctx.beginPath();
  ctx.moveTo(x + w / 2, y + 8);
  ctx.lineTo(x + w / 2 - 6, y + 18);
  ctx.lineTo(x + w / 2 + 6, y + 18);
  ctx.closePath();
  ctx.fillStyle = "#4ecdc4";
  ctx.fill();
}
