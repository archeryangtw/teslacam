import { useRef, useEffect, useCallback } from "react";
import "./BirdEyeView.css";

interface BirdEyeViewProps {
  videoRefs: Map<string, HTMLVideoElement>;
  visible: boolean;
  onToggle: () => void;
}

const CANVAS_W = 400;
const CANVAS_H = 460;
const CX = CANVAS_W / 2;
const CY = CANVAS_H / 2;

// 車輛尺寸
const CAR_W = 56;
const CAR_H = 120;

// 各鏡頭配置：位置、朝向、扇形半徑和角度範圍
const CAMERAS: { cam: string; cx: number; cy: number; startDeg: number; endDeg: number; mirror?: boolean }[] = [
  // 前方：車頭正前方，寬視角
  { cam: "front",          cx: CX,       cy: CY - CAR_H/2 - 5,  startDeg: -60,  endDeg: 60 },
  // 左柱：B柱朝前方偏左
  { cam: "left_pillar",    cx: CX - CAR_W/2 - 5, cy: CY - 15,   startDeg: -150, endDeg: -30 },
  // 右柱：B柱朝前方偏右
  { cam: "right_pillar",   cx: CX + CAR_W/2 + 5, cy: CY - 15,   startDeg: 30,   endDeg: 150 },
  // 左後：前葉子板朝後方偏左
  { cam: "left_repeater",  cx: CX - CAR_W/2 - 5, cy: CY + 15,   startDeg: -210, endDeg: -90 },
  // 右後：前葉子板朝後方偏右
  { cam: "right_repeater", cx: CX + CAR_W/2 + 5, cy: CY + 15,   startDeg: 90,   endDeg: 210 },
  // 後方：車尾正後方
  { cam: "back",           cx: CX,       cy: CY + CAR_H/2 + 5,  startDeg: 120,  endDeg: 240, mirror: true },
];

export default function BirdEyeView({ videoRefs, visible, onToggle }: BirdEyeViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.fillStyle = "#0a0a1a";
    ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);

    const radius = 140;

    for (const cfg of CAMERAS) {
      const video = videoRefs.get(cfg.cam);
      if (!video || video.readyState < 2) continue;

      const startRad = (cfg.startDeg - 90) * Math.PI / 180;
      const endRad = (cfg.endDeg - 90) * Math.PI / 180;

      ctx.save();

      // 扇形裁剪
      ctx.beginPath();
      ctx.moveTo(cfg.cx, cfg.cy);
      ctx.arc(cfg.cx, cfg.cy, radius, startRad, endRad);
      ctx.closePath();
      ctx.clip();

      // 計算鏡頭朝向中心角
      const midDeg = (cfg.startDeg + cfg.endDeg) / 2;
      const midRad = midDeg * Math.PI / 180;

      ctx.translate(cfg.cx, cfg.cy);
      ctx.rotate(midRad);
      if (cfg.mirror) ctx.scale(-1, 1);

      // 將影片畫面畫到扇形內，放大以填滿
      const vw = video.videoWidth || 640;
      const vh = video.videoHeight || 480;
      const aspect = vw / vh;
      const drawH = radius * 1.6;
      const drawW = drawH * aspect;
      ctx.drawImage(video, -drawW / 2, -radius * 0.1, drawW, drawH);

      ctx.restore();

      // 扇形邊框
      ctx.beginPath();
      ctx.moveTo(cfg.cx, cfg.cy);
      ctx.arc(cfg.cx, cfg.cy, radius, startRad, endRad);
      ctx.closePath();
      ctx.strokeStyle = "rgba(78, 205, 196, 0.2)";
      ctx.lineWidth = 0.5;
      ctx.stroke();
    }

    // 車輛圖示（最上層）
    drawCar(ctx, CX - CAR_W / 2, CY - CAR_H / 2, CAR_W, CAR_H);

    animRef.current = requestAnimationFrame(draw);
  }, [videoRefs]);

  useEffect(() => {
    if (visible) {
      animRef.current = requestAnimationFrame(draw);
    }
    return () => { if (animRef.current) cancelAnimationFrame(animRef.current); };
  }, [visible, draw]);

  if (!visible) return null;

  return (
    <div className="birdeye-panel">
      <div className="birdeye-header">
        <span>鳥瞰檢視</span>
        <button className="birdeye-close" onClick={onToggle}>✕</button>
      </div>
      <canvas ref={canvasRef} width={CANVAS_W} height={CANVAS_H} className="birdeye-canvas" />
    </div>
  );
}

function drawCar(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number) {
  const r = 8;
  ctx.beginPath();
  ctx.moveTo(x + r, y); ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
  ctx.fillStyle = "rgba(20, 25, 40, 0.95)";
  ctx.fill();
  ctx.strokeStyle = "#4a4a6a";
  ctx.lineWidth = 1.5;
  ctx.stroke();

  // 擋風玻璃
  ctx.beginPath();
  ctx.moveTo(x + 5, y + 22); ctx.lineTo(x + w - 5, y + 22);
  ctx.lineTo(x + w - 8, y + 36); ctx.lineTo(x + 8, y + 36);
  ctx.closePath();
  ctx.fillStyle = "rgba(78, 205, 196, 0.15)";
  ctx.fill();

  // 後擋風
  ctx.beginPath();
  ctx.moveTo(x + 8, y + h - 30); ctx.lineTo(x + w - 8, y + h - 30);
  ctx.lineTo(x + w - 5, y + h - 20); ctx.lineTo(x + 5, y + h - 20);
  ctx.closePath();
  ctx.fillStyle = "rgba(78, 205, 196, 0.1)";
  ctx.fill();

  // 車頭箭頭
  ctx.beginPath();
  ctx.moveTo(x + w / 2, y + 5);
  ctx.lineTo(x + w / 2 - 4, y + 12);
  ctx.lineTo(x + w / 2 + 4, y + 12);
  ctx.closePath();
  ctx.fillStyle = "#4ecdc4";
  ctx.fill();

  // T
  ctx.fillStyle = "rgba(78, 205, 196, 0.4)";
  ctx.font = "bold 12px system-ui";
  ctx.textAlign = "center";
  ctx.fillText("T", x + w / 2, y + h / 2 + 4);
}
