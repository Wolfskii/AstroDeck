const DOWNLOAD_URL = "https://speed.cloudflare.com/__down?bytes=";
const UPLOAD_URL = "https://speed.cloudflare.com/__up";

export type SpeedPhase = "idle" | "ping" | "download" | "upload" | "done" | "error";

export type SpeedSnapshot = {
  phase: SpeedPhase;
  pingMs: number | null;
  downMbps: number | null;
  upMbps: number | null;
  error: string | null;
};

const DOWNLOAD_MS = 7000;
const UPLOAD_MS = 7000;
const DOWNLOAD_BYTES = 12_000_000;
const UPLOAD_BYTES = 1_500_000;

export async function runSpeedTest(
  onUpdate: (snapshot: SpeedSnapshot) => void,
  signal: AbortSignal
): Promise<void> {
  const snapshot: SpeedSnapshot = {
    phase: "ping",
    pingMs: null,
    downMbps: null,
    upMbps: null,
    error: null,
  };
  onUpdate({ ...snapshot });
  try {
    snapshot.pingMs = await measurePing(signal);
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

async function measurePing(signal: AbortSignal): Promise<number> {
  const samples: number[] = [];
  for (let i = 0; i < 6; i += 1) {
    const start = performance.now();
    const response = await fetch(`${DOWNLOAD_URL}0`, { cache: "no-store", signal });
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
