# TeslaCam Manager

A cross-platform desktop application for managing Tesla dashcam footage. Built with [Tauri](https://tauri.app/) (Rust + React + TypeScript).

Browse, replay, analyze, and export your TeslaCam videos with synchronized 6-camera surround playback, real-time telemetry overlay, GPS tracking, and intelligent event detection.

## Features

### Playback
- **6-Camera Synchronized Playback** — Surround view layout matching actual Tesla camera positions (front, back, left/right pillar, left/right repeater)
- **Multi-Segment Continuous Play** — Automatically merges consecutive clips; Sentry/Saved event folders play as one continuous event
- **Frame Stepping** — Step forward/backward one frame at a time (arrow keys)
- **Playback Speed Control** — 0.5x, 1x, 1.5x, 2x, 4x
- **Real-Time Recording Timestamp** — Displays the actual recording date and time (YYYY-MM-DD HH:MM:SS) during playback

### Telemetry (SEI Data)
- **Real-Time Dashboard** — Tesla Yoke steering wheel animation, speed, gear (P/R/N/D), throttle/brake bars, turn signals with blink animation
- **Autopilot Status** — Shows OFF / FSD / Autosteer / TACC
- **GPS Coordinates** — Latitude, longitude, heading
- **Minimizable** — Collapse to a compact speed + gear display
- **Precise Sync** — Uses MP4 sample table for frame-accurate telemetry-to-video alignment

### Map
- **GPS Track Visualization** — Real-time driving route on OpenStreetMap via MapLibre GL
- **Current Position Marker** — Follows playback with heading indicator
- **Event Markers** — Click to jump to events on the map
- **Minimizable** — Collapse to a map icon, expand on demand

### Bird's Eye View
- **Surround Projection** — All 6 cameras projected into fan-shaped regions around a car diagram
- **Real-Time Updates** — Synchronized with video playback via `requestAnimationFrame`

### Event Detection (Rule Engine)
- **Hard Brake** — Deceleration > 15 km/h/s with brake applied
- **Hard Acceleration** — Acceleration > 15 km/h/s
- **Sharp Turn** — Steering rate > 90 deg/s above 20 km/h
- **Reverse Gear** — Gear change to R
- **Autopilot Changes** — Mode transitions
- **Stops** — Speed < 1 km/h for > 3 seconds
- **Speeding** — Above 110 km/h
- **Timeline Markers** — Color-coded dots on the timeline; click to jump

### Export
- **6-Camera Surround Video** — 3x2 grid layout via ffmpeg (1920x960 H.264)
- **Time Range Selection** — Mark in/out points (I/O keys), frame-accurate trimming
- **Cross-Segment Export** — Automatically handles exports spanning multiple clips
- **Standard Timestamp Watermark** — Real recording time (not relative)
- **Telemetry Overlay** — Speed, gear, steering, throttle/brake burned into the export via ASS subtitles
- **Event Report** — HTML report with driving summary, max/avg speed, detected events; Sentry reports show trigger time, timeline, GPS location, and parked status

### Management
- **Multi-Vehicle Support** — Add, switch, and remove multiple Tesla vehicles via dropdown
- **Backup & Delete** — Backup events to local folder, delete with confirmation
- **Auto-Merge** — Consecutive RecentClips within 65s gap merged into sessions

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `Left Arrow` | Previous frame |
| `Right Arrow` | Next frame |
| `I` | Set mark-in point |
| `O` | Set mark-out point |
| `Esc` | Clear marks |

## Requirements

- **Tesla Firmware** 2025.44.25 or later (for SEI telemetry data)
- **Hardware** HW3 or above
- **ffmpeg** installed on your system (for video export)

## Installation

### Pre-built Binaries

Download from the [Releases](https://github.com/archeryangtw/teslacam/releases) page.

### Build from Source

**Prerequisites:**
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.77+
- [pnpm](https://pnpm.io/) 8+
- [ffmpeg](https://ffmpeg.org/) (for export features)

**macOS:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install pnpm
npm install -g pnpm

# Install ffmpeg
brew install ffmpeg

# Clone and build
git clone https://github.com/archeryangtw/teslacam.git
cd teslacam
pnpm install
pnpm tauri dev        # Development mode
pnpm tauri build      # Production build (outputs .dmg)
```

**Windows:**
```powershell
# Install Rust
winget install Rustlang.Rustup

# Install pnpm
npm install -g pnpm

# Install ffmpeg (add to PATH)
winget install Gyan.FFmpeg

# Clone and build
git clone https://github.com/archeryangtw/teslacam.git
cd teslacam
pnpm install
pnpm tauri dev        # Development mode
pnpm tauri build      # Production build (outputs .msi)
```

**Linux (Debian/Ubuntu):**
```bash
# Install system dependencies
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev ffmpeg

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install pnpm
npm install -g pnpm

# Clone and build
git clone https://github.com/archeryangtw/teslacam.git
cd teslacam
pnpm install
pnpm tauri dev        # Development mode
pnpm tauri build      # Production build (outputs .deb / .AppImage)
```

## Usage

1. Launch the app
2. Click **"Add Vehicle"** or click the empty sidebar area
3. Select your TeslaCam folder (typically on a USB drive under `TeslaCam/`)
4. Browse events in the sidebar — Sentry, Saved, and Recent clips are automatically categorized
5. Click an event to start playback
6. Use the telemetry panel (top-right), map (bottom-left), and bird's eye view (bottom-right) for analysis
7. Set in/out marks and export surround video with timestamps and telemetry

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | Tauri 2.x |
| Frontend | React 19 + TypeScript |
| Backend | Rust |
| Video | HTML5 Video API |
| Map | MapLibre GL JS + OpenStreetMap |
| Database | SQLite (rusqlite) |
| SEI Parser | Custom Rust parser (protobuf via prost) |
| Video Export | ffmpeg (system-installed) |
| i18n | react-i18next (zh-TW / en) |

## Project Structure

```
src/                    # Frontend (React + TypeScript)
  components/           # UI components
  hooks/                # Custom React hooks
  styles/               # CSS
  types/                # TypeScript types
  i18n.ts               # Internationalization

src-tauri/              # Backend (Rust)
  src/
    commands.rs          # Tauri IPC commands
    db.rs                # SQLite database
    scanner.rs           # TeslaCam folder scanner
    sei.rs               # H.264 SEI metadata parser
    event_detection.rs   # Rule-based event detection
    telemetry_overlay.rs # ASS subtitle generation for export
```

## License

MIT

---

[正體中文版本 (Traditional Chinese)](#teslacam-manager---正體中文)

---

# TeslaCam Manager - 正體中文

跨平台 Tesla 行車紀錄管理桌面應用程式。使用 [Tauri](https://tauri.app/)（Rust + React + TypeScript）開發。

瀏覽、重播、分析、匯出你的 TeslaCam 影片——六鏡頭同步環景播放、即時遙測資料覆蓋、GPS 軌跡追蹤、智慧事件偵測。

## 功能特色

### 播放
- **六鏡頭同步播放** — 環景佈局對應 Tesla 實際鏡頭位置（前方、後方、左右柱、左右後）
- **多段連續播放** — 自動合併連續片段；哨兵/手動保存事件資料夾自動合併為完整事件
- **逐幀步進** — 前進/後退一個影格（方向鍵）
- **播放速度控制** — 0.5x、1x、1.5x、2x、4x
- **即時錄影時間** — 播放時顯示實際錄影日期時間（YYYY-MM-DD HH:MM:SS）

### 遙測資料（SEI）
- **即時儀表板** — Tesla Yoke 方向盤動畫、時速、檔位（P/R/N/D）、油門/煞車進度條、方向燈閃爍
- **自駕狀態** — 顯示 OFF / FSD / Autosteer / TACC
- **GPS 座標** — 緯度、經度、航向
- **可縮小** — 收合為迷你速度 + 檔位顯示
- **精確同步** — 使用 MP4 sample table 做幀級精確的遙測對影片對齊

### 地圖
- **GPS 軌跡** — 透過 MapLibre GL + OpenStreetMap 即時顯示行車路線
- **即時位置標記** — 隨播放移動，三角形指示車頭方向
- **事件標記** — 點擊跳到地圖上的事件位置
- **可最小化** — 收合為地圖圖標，點擊展開

### 鳥瞰檢視
- **環景投影** — 六鏡頭投影到車輛俯瞰圖周圍的扇形區域
- **即時更新** — 透過 requestAnimationFrame 與影片同步

### 事件偵測（規則引擎）
- **急煞車** — 減速 > 15 km/h/s 且煞車踩下
- **急加速** — 加速 > 15 km/h/s
- **急轉彎** — 方向盤轉速 > 90°/s 且車速 > 20km/h
- **倒車** — 切換到 R 檔
- **自駕變化** — 模式切換
- **停車** — 車速 < 1 km/h 超過 3 秒
- **超速** — 超過 110 km/h
- **時間軸標記** — 彩色圓點標記，點擊跳轉

### 匯出
- **六鏡頭環景影片** — 3x2 佈局，ffmpeg 合併（1920x960 H.264）
- **時間段選取** — I/O 鍵標記起止點，逐幀精確裁切
- **跨段匯出** — 自動處理跨越多個片段的匯出
- **標準時間水印** — 顯示真實錄影時間
- **遙測覆蓋** — 車速、檔位、方向盤、油門/煞車燒錄到匯出影片
- **事件報告** — HTML 報告：行車事件含駕駛摘要與偵測事件；哨兵事件含觸發時間、錄影時間軸、GPS 位置、停車狀態

### 管理
- **多車管理** — 新增、切換、刪除不同 Tesla 車輛
- **備份與刪除** — 備份事件到本機資料夾，刪除前確認
- **自動合併** — RecentClips 連續片段（間隔 ≤ 65 秒）自動合併為行車 session

### 鍵盤快捷鍵

| 按鍵 | 功能 |
|------|------|
| `空白鍵` | 播放 / 暫停 |
| `←` | 上一幀 |
| `→` | 下一幀 |
| `I` | 設定起點 |
| `O` | 設定終點 |
| `Esc` | 清除標記 |

## 系統需求

- **Tesla 韌體** 2025.44.25 以上（才有 SEI 遙測資料）
- **硬體** HW3 以上
- **ffmpeg** 需安裝在系統上（匯出功能需要）

## 安裝方式

### 預編譯版本

從 [Releases](https://github.com/archeryangtw/teslacam/releases) 頁面下載。

### 從原始碼編譯

**前置需求：**
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.77+
- [pnpm](https://pnpm.io/) 8+
- [ffmpeg](https://ffmpeg.org/)（匯出功能需要）

**macOS：**
```bash
# 安裝 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安裝 pnpm
npm install -g pnpm

# 安裝 ffmpeg
brew install ffmpeg

# 取得原始碼並編譯
git clone https://github.com/archeryangtw/teslacam.git
cd teslacam
pnpm install
pnpm tauri dev        # 開發模式
pnpm tauri build      # 正式版（產生 .dmg）
```

**Windows：**
```powershell
# 安裝 Rust
winget install Rustlang.Rustup

# 安裝 pnpm
npm install -g pnpm

# 安裝 ffmpeg（需加入 PATH）
winget install Gyan.FFmpeg

# 取得原始碼並編譯
git clone https://github.com/archeryangtw/teslacam.git
cd teslacam
pnpm install
pnpm tauri dev        # 開發模式
pnpm tauri build      # 正式版（產生 .msi）
```

**Linux（Debian/Ubuntu）：**
```bash
# 安裝系統依賴
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev ffmpeg

# 安裝 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安裝 pnpm
npm install -g pnpm

# 取得原始碼並編譯
git clone https://github.com/archeryangtw/teslacam.git
cd teslacam
pnpm install
pnpm tauri dev        # 開發模式
pnpm tauri build      # 正式版（產生 .deb / .AppImage）
```

## 使用方式

1. 啟動應用程式
2. 點擊 **「新增車輛」** 或點擊左側空白區域
3. 選擇你的 TeslaCam 資料夾（通常在 USB 磁碟的 `TeslaCam/` 目錄下）
4. 在左側欄瀏覽事件——哨兵、手動保存、行車紀錄自動分類
5. 點擊事件開始播放
6. 使用遙測面板（右上）、地圖（左下）、鳥瞰檢視（右下）進行分析
7. 設定起止點後匯出帶時間戳和遙測資料的環景影片

## 授權

MIT
