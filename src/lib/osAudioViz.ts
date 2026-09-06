/** Latest speaker-mix analysis. Read from rAF loops; do not subscribe in mount effects. */

import { setAudioVisualizerEmit } from "../services/prefs";

export type OsAudioFrame = {
  active: boolean;
  beat: number;
  rms: number;
  bass: number;
  mid: number;
  treble: number;
  bands: number[];
  error?: string | null;
};

type ErrorListener = (error: string | null) => void;

let latest: OsAudioFrame | null = null;
let listening = false;
let frameConsumers = 0;
let lastError: string | null = null;
let emitQueue: Promise<void> = Promise.resolve();
const errorListeners = new Set<ErrorListener>();

function errorFromFrame(frame: OsAudioFrame | null | undefined): string | null {
  const value = frame?.error;
  return typeof value === "string" && value.length > 0 ? value : null;
}

function publishError(next: string | null): void {
  if (next === lastError) return;
  lastError = next;
  for (const listener of errorListeners) listener(next);
}

function syncEmit(): void {
  emitQueue = emitQueue.then(() => setAudioVisualizerEmit(frameConsumers > 0)).catch(() => {});
}

export function getOsAudioFrame(): OsAudioFrame | null {
  const frame = latest;
  if (!frame?.active) return null;
  return frame;
}

export function subscribeOsAudioError(listener: ErrorListener): () => void {
  errorListeners.add(listener);
  return () => {
    errorListeners.delete(listener);
  };
}

export function ensureOsAudioListener(): void {
  if (listening || typeof window === "undefined") return;
  listening = true;
  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<OsAudioFrame>("os-audio-viz", (event) => {
        const payload = event.payload;
        latest = payload?.active ? payload : null;
        publishError(errorFromFrame(payload));
      })
    )
    .catch(() => {
      listening = false;
    });
}

/** Start speaker capture/events only while a now-playing background is mounted. */
export function acquireOsAudioFrames(): void {
  ensureOsAudioListener();
  frameConsumers += 1;
  if (frameConsumers === 1) syncEmit();
}

export function releaseOsAudioFrames(): void {
  frameConsumers = Math.max(0, frameConsumers - 1);
  if (frameConsumers === 0) {
    latest = null;
    syncEmit();
  }
}

export function sampleBand(bands: number[] | null | undefined, index: number, count: number): number | null {
  if (!bands || bands.length === 0) return null;
  const t = (index / Math.max(1, count - 1)) * (bands.length - 1);
  const a = Math.floor(t);
  const b = Math.min(bands.length - 1, a + 1);
  const frac = t - a;
  return (bands[a] ?? 0) * (1 - frac) + (bands[b] ?? 0) * frac;
}
