import "./SteeringWheel.css";

interface SteeringWheelProps {
  angle: number;
  size?: number;
}

export default function SteeringWheel({ angle, size = 120 }: SteeringWheelProps) {
  // Tesla steering_wheel_angle: 正值=左轉(逆時針), 需取反給 CSS
  const rotation = -angle;

  return (
    <div className="steering-wheel" style={{ width: size, height: size }}>
      <svg
        viewBox="0 0 200 200"
        className="steering-svg"
        style={{ transform: `rotate(${rotation}deg)` }}
      >
        {/* Tesla Yoke 方向盤 — 蝶形/U形 */}
        {/* 左握把 */}
        <path
          d="M 30,75 C 20,85 16,100 20,115 C 24,130 35,140 50,145 L 55,130 C 42,126 34,118 32,108 C 30,98 34,88 42,82 Z"
          fill="#444"
          stroke="#555"
          strokeWidth="1"
        />
        {/* 右握把 */}
        <path
          d="M 170,75 C 180,85 184,100 180,115 C 176,130 165,140 150,145 L 145,130 C 158,126 166,118 168,108 C 170,98 166,88 158,82 Z"
          fill="#444"
          stroke="#555"
          strokeWidth="1"
        />

        {/* 主框架 - 上橫樑（平頂） */}
        <path
          d="M 50,68 L 150,68 C 156,68 160,72 160,78 L 160,82 L 40,82 L 40,78 C 40,72 44,68 50,68 Z"
          fill="#3a3a3a"
          stroke="#555"
          strokeWidth="1"
        />

        {/* 左輻條 */}
        <rect x="40" y="82" width="18" height="35" rx="4" fill="#333" stroke="#444" strokeWidth="1" />
        {/* 右輻條 */}
        <rect x="142" y="82" width="18" height="35" rx="4" fill="#333" stroke="#444" strokeWidth="1" />

        {/* 下橫樑（平底） */}
        <path
          d="M 55,130 L 145,130 C 150,130 152,126 152,122 L 152,117 L 48,117 L 48,122 C 48,126 50,130 55,130 Z"
          fill="#3a3a3a"
          stroke="#555"
          strokeWidth="1"
        />

        {/* 中心區域 */}
        <rect x="58" y="82" width="84" height="35" rx="6" fill="#2a2a2a" stroke="#3a3a3a" strokeWidth="1" />

        {/* Tesla T 標誌 */}
        <text
          x="100" y="105"
          textAnchor="middle"
          fill="#666"
          fontSize="20"
          fontWeight="800"
          fontFamily="system-ui"
        >
          T
        </text>

        {/* 左按鈕區 (滾輪) */}
        <circle cx="66" cy="96" r="5" fill="#2a2a2a" stroke="#444" strokeWidth="1" />
        {/* 右按鈕區 (滾輪) */}
        <circle cx="134" cy="96" r="5" fill="#2a2a2a" stroke="#444" strokeWidth="1" />

        {/* 頂部中心標記（12點鐘位置） */}
        <rect x="95" y="62" width="10" height="4" rx="2" fill="var(--accent-red)" />
      </svg>
    </div>
  );
}
