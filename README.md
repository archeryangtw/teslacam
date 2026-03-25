# TeslaCam Manager

A cross-platform desktop application for managing Tesla dashcam footage. Built with [Tauri](https://tauri.app/) (Rust + React + TypeScript).

Browse, replay, analyze, and export your TeslaCam videos with synchronized 6-camera surround playback, real-time telemetry overlay, GPS tracking, intelligent event detection, and a driving analytics dashboard with trip statistics and driving score.

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

### Driving Analytics Dashboard
- **Trip Detection** — Automatically identifies driving trips from RecentClips using SEI telemetry (gear, speed, duration)
- **Period Summary** — View total distance, drive time, trip count, and detected events for the week, month, or all time
- **Period Comparison** — Percentage delta vs. previous period (e.g., +12% distance vs. last week)
- **Daily Distance Chart** — Bar chart showing distance driven per day
- **Event Breakdown** — Donut chart of hard brakes, hard accelerations, sharp turns, and other events
- **Trip List** — Each trip shows distance, duration, avg/max speed, and color-coded event dots
- **Driving Score** — 0–100 composite score across 4 dimensions: smooth braking, steady acceleration, smooth cornering, and speed compliance
- **Route Heatmap (placeholder)** — GPS points stored for future MapLibre GL heatmap visualization
- **Distance Calculation** — GPS-based with speed-integration cross-validation; filters invalid GPS (tunnels, garages)
- **Shareable** — Designed for screenshot-sharing in Tesla owner communities

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

### Quick Install (Recommended)

Go to the [Releases](https://github.com/archeryangtw/teslacam/releases) page and download the installer for your platform:

| Platform | File | How to Install |
|----------|------|----------------|
| **macOS (Apple Silicon)** | `TeslaCam Manager_x.x.x_aarch64.dmg` | Open the `.dmg`, drag **TeslaCam Manager** into the **Applications** folder. If macOS says "unidentified developer", go to **System Settings > Privacy & Security** and click **Open Anyway**. |
| **macOS (Intel)** | `TeslaCam Manager_x.x.x_x64.dmg` | Same as above. |
| **Windows** | `TeslaCam Manager_x.x.x_x64-setup.exe` or `.msi` | Double-click to run the installer and follow the prompts. If Windows SmartScreen warns, click **More info > Run anyway**. |

#### Installing ffmpeg (required for video export)

The app works without ffmpeg for playback and analysis, but **video export** requires ffmpeg:

- **macOS**: Open Terminal and run: `brew install ffmpeg` (requires [Homebrew](https://brew.sh/))
- **Windows**: Download from [ffmpeg.org](https://ffmpeg.org/download.html), extract, and add the `bin` folder to your system PATH. Or run: `winget install Gyan.FFmpeg`

### Build from Source (Advanced)

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
| Charts | Recharts |
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
    analytics_engine.rs  # Driving analytics (trips, scores, aggregation)
    analytics_db.rs      # Analytics schema migration
    telemetry_overlay.rs # ASS subtitle generation for export
```

## License

MIT

---

[正體中文版本 (Traditional Chinese)](#teslacam-manager---正體中文)

---

# TeslaCam Manager - 正體中文

跨平台 Tesla 行車紀錄管理桌面應用程式。使用 [Tauri](https://tauri.app/)（Rust + React + TypeScript）開發。

瀏覽、重播、分析、匯出你的 TeslaCam 影片——六鏡頭同步環景播放、即時遙測資料覆蓋、GPS 軌跡追蹤、智慧事件偵測、駕駛分析儀表板（行程統計與駕駛評分）。

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

### 駕駛分析儀表板
- **行程偵測** — 從 RecentClips 的 SEI 遙測資料（檔位、車速、時長）自動識別行車行程
- **期間摘要** — 查看本週、本月或全部時間的總里程、駕駛時間、行程數、偵測事件數
- **期間比較** — 與前期百分比差異（例如：vs 上週 +12% 里程）
- **每日里程圖表** — 長條圖顯示每天行駛里程
- **事件分布** — 甜甜圈圖呈現急煞車、急加速、急轉彎及其他事件比例
- **行程列表** — 每趟行程顯示里程、時間、平均/最高車速、彩色事件標記
- **駕駛評分** — 0–100 綜合評分，涵蓋四個面向：平穩煞車、平穩加速、平穩過彎、速度合規
- **路線熱力圖（預留）** — GPS 點已儲存，未來將整合 MapLibre GL 熱力圖顯示
- **距離計算** — GPS 定位搭配速度積分交叉驗證；過濾無效 GPS（隧道、地下停車場）
- **可分享** — 專為截圖分享到 Tesla 車主社群而設計

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

### 快速安裝（推薦）

到 [Releases](https://github.com/archeryangtw/teslacam/releases) 頁面下載對應你電腦的安裝檔：

| 平台 | 檔案 | 安裝方式 |
|------|------|----------|
| **macOS (Apple Silicon)** | `TeslaCam Manager_x.x.x_aarch64.dmg` | 開啟 `.dmg`，將 **TeslaCam Manager** 拖入 **Applications** 資料夾。若 macOS 顯示「未識別的開發者」，請到 **系統設定 > 隱私權與安全性**，點擊 **強制開啟**。 |
| **macOS (Intel)** | `TeslaCam Manager_x.x.x_x64.dmg` | 同上。 |
| **Windows** | `TeslaCam Manager_x.x.x_x64-setup.exe` 或 `.msi` | 雙擊執行安裝程式，依提示操作。若 Windows SmartScreen 出現警告，點擊 **其他資訊 > 仍要執行**。 |

#### 安裝 ffmpeg（匯出影片功能需要）

不安裝 ffmpeg 也可以正常播放和分析影片，但**匯出影片**功能需要 ffmpeg：

- **macOS**：打開終端機執行 `brew install ffmpeg`（需先安裝 [Homebrew](https://brew.sh/)）
- **Windows**：從 [ffmpeg.org](https://ffmpeg.org/download.html) 下載，解壓縮後將 `bin` 資料夾加入系統 PATH。或執行 `winget install Gyan.FFmpeg`

### 從原始碼編譯（進階）

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
