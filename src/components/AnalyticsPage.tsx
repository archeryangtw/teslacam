import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend,
} from "recharts";
import "./AnalyticsPage.css";

interface TripInfo {
  id: number;
  start_time: string;
  end_time: string;
  duration_sec: number;
  distance_km: number;
  avg_speed_kmh: number;
  max_speed_kmh: number;
  event_count: number;
  hard_brake_count: number;
  hard_accel_count: number;
  sharp_turn_count: number;
  autopilot_pct: number;
  driving_score: number;
}

interface DailyStat {
  date: string;
  trip_count: number;
  total_distance_km: number;
  total_duration_sec: number;
  avg_speed_kmh: number;
  max_speed_kmh: number;
  event_count: number;
  driving_score: number;
}

interface PeriodSummary {
  total_distance_km: number;
  total_duration_sec: number;
  trip_count: number;
  event_count: number;
  driving_score: number;
  prev_distance_km: number | null;
  prev_duration_sec: number | null;
  prev_trip_count: number | null;
  prev_event_count: number | null;
}

interface HeatmapPoint {
  lat: number;
  lon: number;
  speed_kmh: number;
}

const PERIOD_OPTIONS = ["week", "month", "all"] as const;
type Period = (typeof PERIOD_OPTIONS)[number];

const EVENT_COLORS = ["#7c8cf8", "#a78bfa", "#60a5fa", "#555"];

interface Props {
  vehicleId: number | null;
  visible: boolean;
}

export default function AnalyticsPage({ vehicleId, visible }: Props) {
  const { t } = useTranslation();
  const [period, setPeriod] = useState<Period>("week");
  const [computing, setComputing] = useState(false);
  const [summary, setSummary] = useState<PeriodSummary | null>(null);
  const [dailyStats, setDailyStats] = useState<DailyStat[]>([]);
  const [trips, setTrips] = useState<TripInfo[]>([]);
  const [hasData, setHasData] = useState(false);

  const loadData = useCallback(async () => {
    if (!vehicleId) return;

    try {
      const s = await invoke<PeriodSummary>("get_period_summary", { vehicleId, period });
      setSummary(s);
      setHasData(s.trip_count > 0);

      if (s.trip_count === 0) return;

      // 日期範圍
      const now = new Date();
      let dateFrom: string;
      const dateTo = now.toISOString().split("T")[0];

      if (period === "week") {
        const d = new Date(now);
        d.setDate(d.getDate() - d.getDay() + 1); // Monday
        dateFrom = d.toISOString().split("T")[0];
      } else if (period === "month") {
        dateFrom = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-01`;
      } else {
        dateFrom = "1970-01-01";
      }

      const [daily, tripList] = await Promise.all([
        invoke<DailyStat[]>("get_daily_stats", { vehicleId, dateFrom, dateTo }),
        invoke<TripInfo[]>("get_trips", { vehicleId, dateFrom, dateTo: dateTo + "T23:59:59" }),
      ]);

      setDailyStats(daily);
      setTrips(tripList);
    } catch (e) {
      console.error("Analytics load failed:", e);
    }
  }, [vehicleId, period]);

  useEffect(() => {
    if (visible && vehicleId) loadData();
  }, [visible, vehicleId, period, loadData]);

  const handleCompute = async () => {
    if (!vehicleId) return;
    setComputing(true);
    try {
      await invoke("compute_analytics", { vehicleId });
      await loadData();
    } catch (e) {
      console.error("Compute failed:", e);
    } finally {
      setComputing(false);
    }
  };

  if (!visible) return null;

  // 格式化 delta
  const fmtDelta = (curr: number, prev: number | null | undefined) => {
    if (prev == null || prev === 0) return null;
    const pct = ((curr - prev) / prev) * 100;
    const sign = pct >= 0 ? "+" : "";
    return { text: `${sign}${pct.toFixed(0)}%`, positive: pct <= 0 || curr === prev };
  };

  // 事件分布資料
  const eventBreakdown = trips.length > 0 ? [
    { name: t("analytics.hardBrake"), value: trips.reduce((s, t) => s + t.hard_brake_count, 0) },
    { name: t("analytics.hardAccel"), value: trips.reduce((s, t) => s + t.hard_accel_count, 0) },
    { name: t("analytics.sharpTurn"), value: trips.reduce((s, t) => s + t.sharp_turn_count, 0) },
    { name: t("analytics.other"), value: Math.max(0, (summary?.event_count ?? 0) - trips.reduce((s, t) => s + t.hard_brake_count + t.hard_accel_count + t.sharp_turn_count, 0)) },
  ].filter(d => d.value > 0) : [];

  // 每日里程圖表資料
  const dayLabels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const barData = dailyStats.map((d) => ({
    date: d.date.slice(5), // MM-DD
    day: dayLabels[new Date(d.date + "T00:00:00").getDay() === 0 ? 6 : new Date(d.date + "T00:00:00").getDay() - 1] || d.date.slice(5),
    distance: Number(d.total_distance_km.toFixed(1)),
  }));

  // 駕駛評分子分數（從 trips 平均）
  const scoreBreakdown = trips.length > 0 ? (() => {
    const totalDist = trips.reduce((s, t) => s + t.distance_km, 0);
    const per100 = totalDist > 0 ? 100 / totalDist : 0;
    const brakes = trips.reduce((s, t) => s + t.hard_brake_count, 0);
    const accels = trips.reduce((s, t) => s + t.hard_accel_count, 0);
    const turns = trips.reduce((s, t) => s + t.sharp_turn_count, 0);
    const rateToScore = (r: number) => r < 0.5 ? 25 : r < 3 ? 22 : r < 6 ? 18 : r < 11 ? 12 : 5;
    return [
      { name: t("analytics.smoothBraking"), score: rateToScore(brakes * per100), max: 25 },
      { name: t("analytics.steadyAccel"), score: rateToScore(accels * per100), max: 25 },
      { name: t("analytics.smoothCornering"), score: rateToScore(turns * per100), max: 25 },
      { name: t("analytics.speedCompliance"), score: Math.max(5, 25 - Math.floor((summary?.event_count ?? 0) * per100 / 2)), max: 25 },
    ];
  })() : [];

  const totalScore = summary?.driving_score ?? 100;

  return (
    <div className="analytics-page">
      {/* Period selector */}
      <div className="analytics-header">
        <div className="period-tabs">
          {PERIOD_OPTIONS.map((p) => (
            <button
              key={p}
              className={`period-tab ${period === p ? "active" : ""}`}
              onClick={() => setPeriod(p)}
            >
              {t(`analytics.${p}`)}
            </button>
          ))}
        </div>
        <div className="analytics-actions">
          <button
            className="btn compute-btn"
            onClick={handleCompute}
            disabled={computing || !vehicleId}
          >
            {computing ? t("analytics.computing") : hasData ? t("analytics.recompute") : t("analytics.compute")}
          </button>
        </div>
      </div>

      {!hasData && !computing && (
        <div className="analytics-empty">
          <p>{t("analytics.noData")}</p>
          {vehicleId && (
            <button className="btn compute-btn" onClick={handleCompute}>
              {t("analytics.compute")}
            </button>
          )}
        </div>
      )}

      {hasData && summary && (
        <>
          {/* Summary cards */}
          <div className="summary-row">
            <SummaryCard
              label={t("analytics.totalDistance")}
              value={summary.total_distance_km.toFixed(1)}
              unit={t("analytics.km")}
              delta={fmtDelta(summary.total_distance_km, summary.prev_distance_km)}
            />
            <SummaryCard
              label={t("analytics.driveTime")}
              value={(summary.total_duration_sec / 3600).toFixed(1)}
              unit={t("analytics.hrs")}
              delta={fmtDelta(summary.total_duration_sec, summary.prev_duration_sec)}
            />
            <SummaryCard
              label={t("analytics.trips")}
              value={String(summary.trip_count)}
              delta={fmtDelta(summary.trip_count, summary.prev_trip_count)}
            />
            <SummaryCard
              label={t("analytics.eventsDetected")}
              value={String(summary.event_count)}
              delta={fmtDelta(summary.event_count, summary.prev_event_count)}
              invertDelta
            />
          </div>

          {/* Charts */}
          <div className="charts-grid">
            <div className="chart-box">
              <h3>{t("analytics.dailyDistance")}</h3>
              <ResponsiveContainer width="100%" height={160}>
                <BarChart data={barData}>
                  <XAxis dataKey="day" tick={{ fill: "#888", fontSize: 11 }} />
                  <YAxis tick={{ fill: "#888", fontSize: 11 }} width={30} />
                  <Tooltip
                    contentStyle={{ background: "#1e1e3a", border: "1px solid #333", borderRadius: 6 }}
                    labelStyle={{ color: "#aaa" }}
                  />
                  <Bar dataKey="distance" fill="#5b5bbd" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>

            <div className="chart-box">
              <h3>{t("analytics.eventBreakdown")}</h3>
              {eventBreakdown.length > 0 ? (
                <ResponsiveContainer width="100%" height={160}>
                  <PieChart>
                    <Pie
                      data={eventBreakdown}
                      cx="40%"
                      cy="50%"
                      innerRadius={30}
                      outerRadius={55}
                      dataKey="value"
                      stroke="none"
                    >
                      {eventBreakdown.map((_, i) => (
                        <Cell key={i} fill={EVENT_COLORS[i % EVENT_COLORS.length]} />
                      ))}
                    </Pie>
                    <Legend
                      layout="vertical"
                      align="right"
                      verticalAlign="middle"
                      iconSize={8}
                      wrapperStyle={{ fontSize: 11, color: "#aaa" }}
                      formatter={(value, entry) => `${value} (${(entry.payload as { value: number }).value})`}
                    />
                  </PieChart>
                </ResponsiveContainer>
              ) : (
                <div className="no-events">No events</div>
              )}
            </div>
          </div>

          {/* Trip list */}
          <div className="trips-section">
            <div className="section-header">
              <h3>{t("analytics.recentTrips")}</h3>
              <span className="trip-count">
                {t("analytics.showing")} {Math.min(trips.length, 10)} {t("analytics.of")} {trips.length}
              </span>
            </div>
            <div className="trip-list">
              {trips.slice(0, 10).map((trip) => (
                <TripCard key={trip.id} trip={trip} t={t} />
              ))}
            </div>
          </div>

          {/* Driving Score + Heatmap placeholder */}
          <div className="bottom-grid">
            <div className="chart-box heatmap-placeholder">
              <h3>{t("analytics.routeHeatmap")}</h3>
              <div className="heatmap-content">
                <div className="heatmap-blob a" />
                <div className="heatmap-blob b" />
                <div className="heatmap-blob c" />
                <span className="heatmap-label">MapLibre GL (coming soon)</span>
              </div>
            </div>

            <div className="chart-box score-card">
              <h3>{t("analytics.drivingScore")}</h3>
              <div className="score-ring" data-score={totalScore}>
                <svg viewBox="0 0 120 120" width="120" height="120">
                  <circle cx="60" cy="60" r="52" fill="none" stroke="#333" strokeWidth="8" />
                  <circle
                    cx="60" cy="60" r="52" fill="none"
                    stroke={totalScore >= 80 ? "#4ade80" : totalScore >= 60 ? "#fbbf24" : "#f87171"}
                    strokeWidth="8"
                    strokeDasharray={`${(totalScore / 100) * 327} 327`}
                    strokeLinecap="round"
                    transform="rotate(-90 60 60)"
                  />
                  <text x="60" y="68" textAnchor="middle" fill={totalScore >= 80 ? "#4ade80" : totalScore >= 60 ? "#fbbf24" : "#f87171"} fontSize="28" fontWeight="700">
                    {totalScore}
                  </text>
                </svg>
              </div>
              <div className="score-breakdown">
                {scoreBreakdown.map((item) => (
                  <div key={item.name} className="score-item">
                    <span className="score-name">{item.name}</span>
                    <span className="score-pts">{item.score}/{item.max}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function SummaryCard({ label, value, unit, delta, invertDelta }: {
  label: string;
  value: string;
  unit?: string;
  delta: { text: string; positive: boolean } | null;
  invertDelta?: boolean;
}) {
  return (
    <div className="summary-card">
      <div className="summary-label">{label}</div>
      <div className="summary-value">
        {value} {unit && <span className="summary-unit">{unit}</span>}
      </div>
      {delta && (
        <div className={`summary-delta ${(invertDelta ? !delta.positive : delta.positive) ? "positive" : "negative"}`}>
          {delta.text}
        </div>
      )}
    </div>
  );
}

function TripCard({ trip, t }: { trip: TripInfo; t: (key: string) => string }) {
  const startDate = new Date(trip.start_time);
  const timeStr = `${String(startDate.getHours()).padStart(2, "0")}:${String(startDate.getMinutes()).padStart(2, "0")}`;
  const endDate = new Date(trip.end_time);
  const endTimeStr = `${String(endDate.getHours()).padStart(2, "0")}:${String(endDate.getMinutes()).padStart(2, "0")}`;
  const dateStr = `${startDate.getFullYear()}/${String(startDate.getMonth() + 1).padStart(2, "0")}/${String(startDate.getDate()).padStart(2, "0")}`;
  const dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

  return (
    <div className="trip-card">
      <div className="trip-time-col">
        <div className="trip-time">{timeStr} - {endTimeStr}</div>
        <div className="trip-date">{dateStr} ({dayNames[startDate.getDay()]})</div>
      </div>
      <div className="trip-stats">
        <div className="trip-stat"><div className="val">{trip.distance_km.toFixed(1)}</div><div className="lbl">{t("analytics.km")}</div></div>
        <div className="trip-stat"><div className="val">{Math.round(trip.duration_sec / 60)}</div><div className="lbl">{t("analytics.min")}</div></div>
        <div className="trip-stat"><div className="val">{Math.round(trip.avg_speed_kmh)}</div><div className="lbl">{t("analytics.avgKmh")}</div></div>
        <div className="trip-stat"><div className="val">{Math.round(trip.max_speed_kmh)}</div><div className="lbl">{t("analytics.maxKmh")}</div></div>
      </div>
      <div className="trip-events">
        {Array.from({ length: trip.hard_brake_count }).map((_, i) => <span key={`b${i}`} className="event-dot brake" />)}
        {Array.from({ length: trip.hard_accel_count }).map((_, i) => <span key={`a${i}`} className="event-dot accel" />)}
        {Array.from({ length: trip.sharp_turn_count }).map((_, i) => <span key={`t${i}`} className="event-dot turn" />)}
      </div>
    </div>
  );
}
