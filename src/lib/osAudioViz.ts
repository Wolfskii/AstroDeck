/** Latest speaker-mix analysis. Read from rAF loops; do not subscribe in mount effects. */

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

let latest: OsAudioFrame | null = null;
let listening = false;

export function getOsAudioFrame(): OsAudioFrame | null {
  const frame = latest;
  if (!frame?.active) return null;
  return frame;
}

export function ensureOsAudioListener(): void {
  if (listening || typeof window === "undefined") return;
  listening = true;
  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<OsAudioFrame>("os-audio-viz", (event) => {
        latest = event.payload?.active ? event.payload : null;
      })
    )
    .catch(() => {
      listening = false;
    });
}

export function sampleBand(bands: number[] | null | undefined, index: number, count: number): number | null {
  if (!bands || bands.length === 0) return null;
  const t = (index / Math.max(1, count - 1)) * (bands.length - 1);
  const a = Math.floor(t);
  const b = Math.min(bands.length - 1, a + 1);
  const frac = t - a;
  return (bands[a] ?? 0) * (1 - frac) + (bands[b] ?? 0) * frac;
}
