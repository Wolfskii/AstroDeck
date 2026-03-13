import { writable } from "svelte/store";

export type LogLevel = "info" | "error";

export interface LogEntry {
  level: LogLevel;
  message: string;
  timestamp: string;
  source: string;
}

const MAX_LOGS = 300;

export const logs = writable<LogEntry[]>([]);

const isTauri =
  typeof window !== "undefined" &&
  !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

function push(level: LogLevel, message: string, source: string) {
  const entry: LogEntry = {
    level,
    message,
    timestamp: new Date().toLocaleTimeString(),
    source,
  };

  logs.update((current) => {
    const next = [...current, entry];
    if (next.length > MAX_LOGS) {
      next.splice(0, next.length - MAX_LOGS);
    }
    return next;
  });

  // In Tauri mode: forward to Rust log bus so browser clients receive it over WS
  if (isTauri) {
    import("@tauri-apps/api/core")
      .then(({ invoke }) => invoke("log_to_bus", { entry: JSON.stringify(entry) }))
      .catch(() => {});
  }

  const prefix = `[${source}]`;
  if (level === "error") {
    console.error("[TapTapDeck]", prefix, message);
  } else {
    console.log("[TapTapDeck]", prefix, message);
  }
}

/** Push a log entry that originated externally (e.g. received via WebSocket). */
export function pushExternal(entry: LogEntry) {
  logs.update((current) => {
    const next = [...current, entry];
    if (next.length > MAX_LOGS) {
      next.splice(0, next.length - MAX_LOGS);
    }
    return next;
  });
}

export function logInfo(message: string, source = "App") {
  push("info", message, source);
}

export function logError(message: string, source = "App") {
  push("error", message, source);
}
