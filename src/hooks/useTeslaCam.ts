import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { TeslaCamEvent } from "../types/events";

interface ScanResult {
  total_events: number;
  sentry_count: number;
  saved_count: number;
  recent_count: number;
  total_clips: number;
  total_size_bytes: number;
  errors: string[];
}

interface EventFromBackend {
  id: number;
  type: string;
  timestamp: string;
  duration_sec: number;
  gps_lat: number | null;
  gps_lon: number | null;
  avg_speed: number | null;
  max_speed: number | null;
  source_dir: string;
  backed_up: boolean;
  notes: string;
  clips: {
    id: number;
    event_id: number;
    camera: string;
    file_path: string;
    file_size: number;
    duration_sec: number;
    has_sei: boolean;
  }[];
}

function mapEvent(e: EventFromBackend): TeslaCamEvent {
  return {
    id: e.id,
    type: e.type as TeslaCamEvent["type"],
    timestamp: e.timestamp,
    durationSec: e.duration_sec,
    gpsLat: e.gps_lat,
    gpsLon: e.gps_lon,
    avgSpeed: e.avg_speed,
    maxSpeed: e.max_speed,
    sourceDir: e.source_dir,
    backedUp: e.backed_up,
    notes: e.notes,
    clips: e.clips.map((c) => ({
      id: c.id,
      eventId: c.event_id,
      camera: c.camera as TeslaCamEvent["clips"][0]["camera"],
      filePath: c.file_path,
      fileSize: c.file_size,
      durationSec: c.duration_sec,
      hasSei: c.has_sei,
    })),
  };
}

export function useTeslaCam() {
  const [rootDir, setRootDir] = useState<string | null>(null);
  const [events, setEvents] = useState<TeslaCamEvent[]>([]);
  const [scanning, setScanning] = useState(false);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectAndScan = useCallback(async () => {
    try {
      setError(null);
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "選擇 TeslaCam 資料夾",
      });

      if (!selected) return;

      const dir = selected as string;
      setRootDir(dir);
      setScanning(true);

      const result = await invoke<ScanResult>("scan_directory", { path: dir });
      setScanResult(result);

      const eventsData = await invoke<EventFromBackend[]>("get_events");
      setEvents(eventsData.map(mapEvent));
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  const deleteEvent = useCallback(
    async (eventId: number, deleteFiles: boolean) => {
      try {
        await invoke("delete_event", { eventId, deleteFiles });
        setEvents((prev) => prev.filter((e) => e.id !== eventId));
      } catch (e) {
        setError(String(e));
      }
    },
    []
  );

  const backupEvent = useCallback(async (eventId: number) => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "選擇備份目的地",
      });
      if (!selected) return;

      const count = await invoke<number>("backup_event", {
        eventId,
        targetDir: selected as string,
      });

      setEvents((prev) =>
        prev.map((e) => (e.id === eventId ? { ...e, backedUp: true } : e))
      );

      return count;
    } catch (e) {
      setError(String(e));
      return 0;
    }
  }, []);

  return {
    rootDir,
    events,
    scanning,
    scanResult,
    error,
    selectAndScan,
    deleteEvent,
    backupEvent,
  };
}
