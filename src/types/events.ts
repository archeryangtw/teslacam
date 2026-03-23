/** TeslaCam 鏡頭角度 */
export type CameraAngle =
  | "front"
  | "back"
  | "left_repeater"
  | "right_repeater"
  | "left_pillar"
  | "right_pillar";

/** 事件類型 */
export type EventType = "sentry" | "saved" | "recent";

/** 影片片段 */
export interface Clip {
  id: number;
  eventId: number;
  camera: CameraAngle;
  filePath: string;
  fileSize: number;
  durationSec: number;
  hasSei: boolean;
}

/** TeslaCam 事件 */
export interface TeslaCamEvent {
  id: number;
  type: EventType;
  timestamp: string; // ISO 8601
  durationSec: number;
  gpsLat: number | null;
  gpsLon: number | null;
  avgSpeed: number | null;
  maxSpeed: number | null;
  sourceDir: string;
  backedUp: boolean;
  notes: string;
  clips: Clip[];
}

/** SEI 遙測資料 */
export interface TelemetryPoint {
  offsetMs: number;
  speed: number | null;
  steering: number | null;
  gpsLat: number | null;
  gpsLon: number | null;
  driveState: "P" | "D" | "R" | "N" | null;
}

/** 掃描進度 */
export interface ScanProgress {
  total: number;
  scanned: number;
  current: string;
  errors: string[];
}
