import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const SPEED_PROVIDERS = [
  { id: "cloudflare", label: "Cloudflare" },
  { id: "fast", label: "Fast.com" },
  { id: "bredbandskollen", label: "Bredbandskollen" },
] as const;

export type SpeedProvider = (typeof SPEED_PROVIDERS)[number]["id"];

export function parseSpeedProvider(value: string | null | undefined): SpeedProvider {
  if (value === "fast" || value === "bredbandskollen") return value;
  return "cloudflare";
}

export function speedProviderLabel(provider: SpeedProvider) {
  return SPEED_PROVIDERS.find((option) => option.id === provider)?.label ?? "Cloudflare";
}

const DOWNLOAD_URL = "https://speed.cloudflare.com/__down?bytes=";
const UPLOAD_URL = "https://speed.cloudflare.com/__up";

export type SpeedPhase = "idle" | "ping" | "download" | "upload" | "done" | "error";

export type SpeedSnapshot = {
  phase: SpeedPhase;
  pingMs: number | null;
  downMbps: number | null;
  upMbps: number | null;
  server: string | null;
  location: string | null;
  error: string | null;
};

const DOWNLOAD_MS = 7000;
const UPLOAD_MS = 7000;
const DOWNLOAD_BYTES = 12_000_000;
const UPLOAD_BYTES = 1_500_000;

type SpeedEvent = SpeedSnapshot & { id: string };

export async function runSpeedTest(
  provider: SpeedProvider,
  onUpdate: (snapshot: SpeedSnapshot) => void,
  signal: AbortSignal
): Promise<void> {
  if (provider === "cloudflare") {
    return runCloudflare(onUpdate, signal);
  }
  return runNative(provider, onUpdate, signal);
}

async function runNative(
  provider: SpeedProvider,
  onUpdate: (snapshot: SpeedSnapshot) => void,
  signal: AbortSignal
): Promise<void> {
  const id = crypto.randomUUID();
  let latest: SpeedSnapshot = {
    phase: "ping",
    pingMs: null,
    downMbps: null,
    upMbps: null,
    server: null,
    location: null,
    error: null,
  };
  const unlisten = await listen<SpeedEvent>("speed-test-progress", (event) => {
    if (signal.aborted || event.payload.id !== id) return;
    latest = {
      phase: event.payload.phase,
      pingMs: event.payload.pingMs ?? latest.pingMs,
      downMbps: event.payload.downMbps ?? latest.downMbps,
      upMbps: event.payload.upMbps ?? latest.upMbps,
      server: event.payload.server ?? latest.server,
      location: event.payload.location ?? latest.location,
      error: event.payload.error,
    };
    onUpdate({ ...latest });
  });
  const cancel = () => {
    void invoke("cancel_speed_test", { id });
  };
  signal.addEventListener("abort", cancel, { once: true });
  try {
    await invoke("start_speed_test", { id, provider });
  } catch (error) {
    if (signal.aborted) return;
    onUpdate({
      ...latest,
      phase: "error",
      error: typeof error === "string" ? error : error instanceof Error ? error.message : "Speed test failed",
    });
  } finally {
    signal.removeEventListener("abort", cancel);
    unlisten();
  }
}

async function runCloudflare(
  onUpdate: (snapshot: SpeedSnapshot) => void,
  signal: AbortSignal
): Promise<void> {
  const snapshot: SpeedSnapshot = {
    phase: "ping",
    pingMs: null,
    downMbps: null,
    upMbps: null,
    server: null,
    location: null,
    error: null,
  };
  onUpdate({ ...snapshot });
  try {
    const ping = await measurePing(signal, (place) => {
      snapshot.server = place.server;
      snapshot.location = place.location;
      onUpdate({ ...snapshot });
    });
    snapshot.pingMs = ping;
    snapshot.phase = "download";
    onUpdate({ ...snapshot });
    snapshot.downMbps = await measureDownload(signal, (mbps) => {
      snapshot.downMbps = mbps;
      onUpdate({ ...snapshot });
    });
    snapshot.phase = "upload";
    onUpdate({ ...snapshot });
    snapshot.upMbps = await measureUpload(signal, (mbps) => {
      snapshot.upMbps = mbps;
      onUpdate({ ...snapshot });
    });
    snapshot.phase = "done";
    onUpdate({ ...snapshot });
  } catch (error) {
    if (signal.aborted) return;
    snapshot.phase = "error";
    snapshot.error = error instanceof Error ? error.message : "Speed test failed";
    onUpdate({ ...snapshot });
  }
}

async function measurePing(
  signal: AbortSignal,
  onPlace: (place: { server: string | null; location: string | null }) => void
): Promise<number> {
  const samples: number[] = [];
  for (let i = 0; i < 6; i += 1) {
    const start = performance.now();
    const response = await fetch(`${DOWNLOAD_URL}0`, { cache: "no-store", signal });
    if (i === 0) onPlace(cloudflarePlace(response));
    await response.arrayBuffer();
    samples.push(performance.now() - start);
  }
  samples.sort((a, b) => a - b);
  return samples[Math.floor(samples.length / 2)] ?? 0;
}

async function measureDownload(
  signal: AbortSignal,
  onProgress: (mbps: number) => void
): Promise<number> {
  return measureTransfer(signal, DOWNLOAD_MS, "Download test failed", onProgress, async (linked) => {
    const response = await fetch(`${DOWNLOAD_URL}${DOWNLOAD_BYTES}`, { cache: "no-store", signal: linked });
    if (!response.ok) throw new Error("Download test failed");
    const buffer = await response.arrayBuffer();
    return buffer.byteLength;
  });
}

async function measureUpload(
  signal: AbortSignal,
  onProgress: (mbps: number) => void
): Promise<number> {
  const payload = new Uint8Array(UPLOAD_BYTES);
  return measureTransfer(signal, UPLOAD_MS, "Upload test failed", onProgress, async (linked) => {
    const response = await fetch(UPLOAD_URL, { method: "POST", body: payload, cache: "no-store", signal: linked });
    if (!response.ok) throw new Error("Upload test failed");
    return payload.byteLength;
  }, 2);
}

async function measureTransfer(
  signal: AbortSignal,
  durationMs: number,
  fallback: string,
  onProgress: (mbps: number) => void,
  transfer: (signal: AbortSignal) => Promise<number>,
  workers = 3
): Promise<number> {
  const stop = new AbortController();
  const linked = AbortSignal.any([signal, stop.signal]);
  const started = performance.now();
  let bytes = 0;
  let failure: Error | null = null;
  const worker = async () => {
    while (performance.now() - started < durationMs && !linked.aborted) {
      try {
        bytes += await transfer(linked);
        const seconds = (performance.now() - started) / 1000;
        if (seconds > 0) onProgress((bytes * 8) / seconds / 1_000_000);
      } catch (error) {
        if (signal.aborted || stop.signal.aborted) return;
        failure = error instanceof Error ? error : new Error(fallback);
        stop.abort();
        return;
      }
    }
  };
  await Promise.all(Array.from({ length: workers }, () => worker()));
  if (failure) throw failure;
  const seconds = (performance.now() - started) / 1000;
  return seconds > 0 ? (bytes * 8) / seconds / 1_000_000 : 0;
}

const CLOUDFLARE_COLOS: Record<string, string> = {
  AMS: "Amsterdam",
  ARN: "Stockholm",
  ATH: "Athens",
  ATL: "Atlanta",
  BCN: "Barcelona",
  BER: "Berlin",
  BOM: "Mumbai",
  BOS: "Boston",
  BRU: "Brussels",
  BUD: "Budapest",
  CDG: "Paris",
  CPH: "Copenhagen",
  DEN: "Denver",
  DFW: "Dallas",
  DUB: "Dublin",
  DUS: "Düsseldorf",
  EWR: "Newark",
  FRA: "Frankfurt",
  GOT: "Gothenburg",
  GRU: "São Paulo",
  HAM: "Hamburg",
  HEL: "Helsinki",
  HKG: "Hong Kong",
  IAD: "Ashburn",
  ICN: "Seoul",
  KEF: "Reykjavík",
  LAX: "Los Angeles",
  LHR: "London",
  LIS: "Lisbon",
  MAD: "Madrid",
  MAN: "Manchester",
  MIA: "Miami",
  MRS: "Marseille",
  MUC: "Munich",
  MXP: "Milan",
  NRT: "Tokyo",
  ORD: "Chicago",
  OSL: "Oslo",
  OTP: "Bucharest",
  PRG: "Prague",
  SEA: "Seattle",
  SIN: "Singapore",
  SJC: "San Jose",
  SOF: "Sofia",
  SYD: "Sydney",
  TLL: "Tallinn",
  VIE: "Vienna",
  WAW: "Warsaw",
  YUL: "Montreal",
  YYZ: "Toronto",
  ZRH: "Zurich",
};

function cloudflarePlace(response: Response) {
  const colo = (response.headers.get("cf-meta-colo") || response.headers.get("colo") || "")
    .trim()
    .toUpperCase();
  if (!colo) return { server: null, location: null };
  return { server: colo, location: CLOUDFLARE_COLOS[colo] ?? null };
}
