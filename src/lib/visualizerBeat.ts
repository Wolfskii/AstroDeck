import { get } from "svelte/store";
import { getOsAudioFrame } from "./osAudioViz";
import { audioVisualizerEnabled } from "../stores/appearance";

/** Synthetic kick/pulse used when system audio capture is on but silent. */
export function synthBeat(elapsed: number, isPlaying: boolean) {
  if (!isPlaying) return 0.12;
  const kick = Math.pow(0.5 + 0.5 * Math.sin(elapsed * 2.35), 10);
  const pulse = 0.5 + 0.5 * Math.sin(elapsed * 1.17);
  return Math.min(1, 0.18 + kick * 0.72 + pulse * 0.14);
}

export function resolveBeat(elapsed: number, isPlaying: boolean) {
  if (!get(audioVisualizerEnabled)) return 0.12;
  const audio = getOsAudioFrame();
  if (audio) {
    if (audio.rms > 0.016 || audio.beat > 0.18) return audio.beat;
    if (isPlaying) return synthBeat(elapsed, true);
    return audio.beat;
  }
  return synthBeat(elapsed, isPlaying);
}

export function resolvePlaying(isPlaying: boolean) {
  if (!get(audioVisualizerEnabled)) return isPlaying;
  const audio = getOsAudioFrame();
  if (audio && audio.rms > 0.02) return true;
  return isPlaying;
}
