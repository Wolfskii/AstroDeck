/** Synthetic kick/pulse — Spotify does not expose an audio FFT. */
export function synthBeat(elapsed: number, isPlaying: boolean) {
  if (!isPlaying) return 0.12;
  const kick = Math.pow(0.5 + 0.5 * Math.sin(elapsed * 2.35), 10);
  const pulse = 0.5 + 0.5 * Math.sin(elapsed * 1.17);
  return Math.min(1, 0.18 + kick * 0.72 + pulse * 0.14);
}
