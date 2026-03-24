import { useEffect, useRef, useMemo } from "react";
import maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import type { TeslaCamEvent } from "../types/events";
import "./MapPanel.css";

interface TelemetryPoint {
  time_sec: number;
  lat: number;
  lon: number;
  speed_kmh: number;
  heading: number;
}

interface MapPanelProps {
  events: TeslaCamEvent[];
  selectedEvent: TeslaCamEvent | null;
  telemetryTrack: TelemetryPoint[];
  currentTime: number;
  visible: boolean;
  onToggle: () => void;
  onSelectEvent?: (event: TeslaCamEvent) => void;
}

export default function MapPanel({
  events,
  selectedEvent,
  telemetryTrack,
  currentTime,
  visible,
  onSelectEvent,
}: MapPanelProps) {
  const mapContainer = useRef<HTMLDivElement>(null);
  const mapRef = useRef<maplibregl.Map | null>(null);
  const markerRef = useRef<maplibregl.Marker | null>(null);

  // 初始化地圖
  useEffect(() => {
    if (!mapContainer.current || !visible) return;
    if (mapRef.current) return; // 已初始化

    const map = new maplibregl.Map({
      container: mapContainer.current,
      style: {
        version: 8,
        sources: {
          osm: {
            type: "raster",
            tiles: ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
            tileSize: 256,
            attribution: "&copy; OpenStreetMap",
          },
        },
        layers: [
          {
            id: "osm",
            type: "raster",
            source: "osm",
          },
        ],
      },
      center: [121.5, 25.0], // 預設台灣
      zoom: 14,
    });

    map.addControl(new maplibregl.NavigationControl(), "top-left");
    mapRef.current = map;

    return () => {
      map.remove();
      mapRef.current = null;
    };
  }, [visible]);

  // 事件標記
  const eventMarkers = useRef<maplibregl.Marker[]>([]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;

    // 清除舊標記
    eventMarkers.current.forEach((m) => m.remove());
    eventMarkers.current = [];

    // 從 SEI 或事件中取 GPS 座標的事件
    for (const event of events) {
      if (!event.gpsLat || !event.gpsLon) continue;

      const el = document.createElement("div");
      el.className = `map-event-marker ${event.type === "sentry" ? "marker-sentry" : event.type === "saved" ? "marker-saved" : "marker-recent"}`;
      el.title = `${event.timestamp} (${event.type})`;

      const marker = new maplibregl.Marker({ element: el })
        .setLngLat([event.gpsLon, event.gpsLat])
        .addTo(map);

      el.addEventListener("click", () => onSelectEvent?.(event));
      eventMarkers.current.push(marker);
    }
  }, [events, onSelectEvent]);

  // 軌跡線 + 目前位置標記
  const currentPos = useMemo(() => {
    if (telemetryTrack.length === 0) return null;

    // 用 currentTime 找最近的點
    let best = telemetryTrack[0];
    for (const pt of telemetryTrack) {
      if (Math.abs(pt.time_sec - currentTime) < Math.abs(best.time_sec - currentTime)) {
        best = pt;
      }
    }
    return best.lat !== 0 && best.lon !== 0 ? best : null;
  }, [telemetryTrack, currentTime]);

  // 更新軌跡線
  useEffect(() => {
    const map = mapRef.current;
    if (!map || !map.isStyleLoaded()) return;

    const coords = telemetryTrack
      .filter((p) => p.lat !== 0 && p.lon !== 0)
      .map((p) => [p.lon, p.lat] as [number, number]);

    if (coords.length < 2) {
      if (map.getLayer("track-line")) map.removeLayer("track-line");
      if (map.getSource("track")) map.removeSource("track");
      return;
    }

    const geojson: GeoJSON.Feature = {
      type: "Feature",
      properties: {},
      geometry: { type: "LineString", coordinates: coords },
    };

    if (map.getSource("track")) {
      (map.getSource("track") as maplibregl.GeoJSONSource).setData(geojson);
    } else {
      map.addSource("track", { type: "geojson", data: geojson });
      map.addLayer({
        id: "track-line",
        type: "line",
        source: "track",
        paint: {
          "line-color": "#4ecdc4",
          "line-width": 3,
          "line-opacity": 0.8,
        },
      });
    }

    // 置中到軌跡
    const bounds = coords.reduce(
      (b, c) => b.extend(c as maplibregl.LngLatLike),
      new maplibregl.LngLatBounds(coords[0], coords[0])
    );
    map.fitBounds(bounds, { padding: 40, maxZoom: 17 });
  }, [telemetryTrack]);

  // 目前位置標記
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;

    if (currentPos) {
      if (!markerRef.current) {
        const el = document.createElement("div");
        el.className = "map-current-marker";
        markerRef.current = new maplibregl.Marker({ element: el }).setLngLat([0, 0]).addTo(map);
      }
      markerRef.current.setLngLat([currentPos.lon, currentPos.lat]);

      // 旋轉標記表示車頭方向
      const el = markerRef.current.getElement();
      el.style.transform = `${el.style.transform} rotate(${currentPos.heading}deg)`;
    } else if (markerRef.current) {
      markerRef.current.remove();
      markerRef.current = null;
    }
  }, [currentPos]);

  if (!visible) return null;

  return (
    <div className="map-panel">
      <button className="map-close" onClick={onToggle} title="最小化地圖">─</button>
      <div ref={mapContainer} className="map-container" />
      {currentPos && (
        <div className="map-info">
          <span>{currentPos.speed_kmh.toFixed(0)} km/h</span>
          <span>{currentPos.lat.toFixed(6)}, {currentPos.lon.toFixed(6)}</span>
        </div>
      )}
    </div>
  );
}
