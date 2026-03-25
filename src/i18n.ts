import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  "zh-TW": {
    translation: {
      app: { title: "TESLACAM", rescan: "重新掃描", addVehicle: "新增車輛", exporting: "匯出中..." },
      sidebar: {
        sentry: "哨兵事件", saved: "手動保存", recent: "行車紀錄",
        cameras: "鏡頭", segments: "段", info: "事件資訊",
        type: "類型", time: "時間", duration: "時長", size: "大小", backup: "備份",
        backedUp: "已備份", notBackedUp: "未備份",
        exportVideo: "匯出六鏡頭影片", exportReport: "匯出事件報告",
        backupLocal: "備份到本機", deleteEvent: "刪除此事件", confirmDelete: "確認刪除", cancel: "取消",
      },
      player: { selectEvent: "選擇一個事件來播放影片", selectFolder: "選擇 TeslaCam 資料夾以開始", clickHere: "點擊此處或右上角按鈕選擇資料夾" },
      telemetry: { title: "遙測資料", loading: "載入 SEI 資料中...", noData: "此影片無 SEI 資料", waiting: "等待遙測資料...", accel: "油門", brake: "煞車", steering: "方向盤", heading: "航向", autopilot: "自駕" },
      birdeye: { title: "鳥瞰檢視" },
      analytics: {
        title: "駕駛分析", day: "日", week: "週", month: "月", all: "全部",
        totalDistance: "總里程", driveTime: "駕駛時間", trips: "行程數", eventsDetected: "偵測事件",
        dailyDistance: "每日里程 (km)", eventBreakdown: "事件分布",
        recentTrips: "近期行程", drivingScore: "駕駛評分", routeHeatmap: "路線熱力圖",
        smoothBraking: "平穩煞車", steadyAccel: "平穩加速", smoothCornering: "平穩過彎", speedCompliance: "速度合規",
        hardBrake: "急煞車", hardAccel: "急加速", sharpTurn: "急轉彎", other: "其他",
        exportPng: "匯出 PNG", exportPdf: "匯出 PDF", exportHint: "將儀表板匯出為可分享的圖片",
        computing: "分析計算中...", noData: "尚無分析資料，請先掃描 TeslaCam 資料夾",
        compute: "開始分析", recompute: "重新計算",
        vsLastWeek: "vs 上週", vsLastMonth: "vs 上月",
        showing: "顯示", of: "/",
        km: "km", hrs: "hrs", min: "分鐘", avgKmh: "平均 km/h", maxKmh: "最高 km/h",
      },
      keys: { space: "播放/暫停", arrows: "逐幀", io: "標記起止點", esc: "清除標記" },
    },
  },
  en: {
    translation: {
      app: { title: "TESLACAM", rescan: "Rescan", addVehicle: "Add Vehicle", exporting: "Exporting..." },
      sidebar: {
        sentry: "Sentry Events", saved: "Saved Clips", recent: "Recent Clips",
        cameras: "cameras", segments: "segments", info: "Event Info",
        type: "Type", time: "Time", duration: "Duration", size: "Size", backup: "Backup",
        backedUp: "Backed up", notBackedUp: "Not backed up",
        exportVideo: "Export Surround Video", exportReport: "Export Report",
        backupLocal: "Backup to Local", deleteEvent: "Delete Event", confirmDelete: "Confirm Delete", cancel: "Cancel",
      },
      player: { selectEvent: "Select an event to play", selectFolder: "Select TeslaCam folder to start", clickHere: "Click here or the button above" },
      telemetry: { title: "Telemetry", loading: "Loading SEI data...", noData: "No SEI data", waiting: "Waiting for telemetry...", accel: "Throttle", brake: "Brake", steering: "Steering", heading: "Heading", autopilot: "Autopilot" },
      birdeye: { title: "Bird Eye View" },
      analytics: {
        title: "Analytics", day: "Day", week: "Week", month: "Month", all: "All",
        totalDistance: "Total Distance", driveTime: "Drive Time", trips: "Trips", eventsDetected: "Events Detected",
        dailyDistance: "Daily Distance (km)", eventBreakdown: "Event Breakdown",
        recentTrips: "Recent Trips", drivingScore: "Driving Score", routeHeatmap: "Route Heatmap",
        smoothBraking: "Smooth Braking", steadyAccel: "Steady Acceleration", smoothCornering: "Smooth Cornering", speedCompliance: "Speed Compliance",
        hardBrake: "Hard Brake", hardAccel: "Hard Accel", sharpTurn: "Sharp Turn", other: "Other",
        exportPng: "Export PNG", exportPdf: "Export PDF", exportHint: "Export dashboard as a shareable image",
        computing: "Computing analytics...", noData: "No analytics data yet. Scan a TeslaCam folder first.",
        compute: "Compute Analytics", recompute: "Recompute",
        vsLastWeek: "vs last week", vsLastMonth: "vs last month",
        showing: "Showing", of: "/",
        km: "km", hrs: "hrs", min: "min", avgKmh: "avg km/h", maxKmh: "max km/h",
      },
      keys: { space: "Play/Pause", arrows: "Frame step", io: "Mark in/out", esc: "Clear marks" },
    },
  },
};

i18n.use(initReactI18next).init({
  resources,
  lng: "zh-TW",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
