import { invoke } from "@tauri-apps/api/core";
import type { SceneState, PluginConfig } from "../types";
import { logInfo } from "./logger";

const isTauri =
  typeof window !== "undefined" &&
  !!(window as any).__TAURI_INTERNALS__;

export async function getActiveScene(): Promise<SceneState> {
  return invoke<SceneState>("get_active_scene");
}

export async function executeAction(action: string): Promise<void> {
  if (!isTauri) {
    // Browser-only mode (http://localhost:1420): don't call Tauri,
    // just log that this would have executed.
    logInfo(`(browser) would execute action: ${action}`, "Browser actions");
    return;
  }

  return invoke("execute_action", { action });
}

export async function executeActionValue(action: string, value: unknown): Promise<void> {
  if (!isTauri) {
    logInfo(`(browser) would execute action value: ${action} => ${String(value)}`, "Browser actions");
    return;
  }

  return invoke("execute_action_value", { action, value });
}

export async function getPlugins(): Promise<PluginConfig[]> {
  return invoke<PluginConfig[]>("get_plugins");
}

export async function setActiveScene(sceneId: string): Promise<void> {
  if (!isTauri) {
    logInfo(`(browser) would set active scene to: ${sceneId}`, "Browser scenes");
    return;
  }

  return invoke("set_active_scene", { sceneId });
}

export interface OsNowPlaying {
  source: string;
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  coverArtUrl?: string | null;
  isPlaying: boolean;
}

export interface SpotifyTrackPreview {
  itemId: string;
  itemType: string;
  trackName: string;
  artistName?: string | null;
  albumName?: string | null;
  coverArtUrl?: string | null;
  durationMs?: number | null;
}

export interface SpotifyStatus {
  isConfigured: boolean;
  isAuthenticated: boolean;
  hasActiveDevice: boolean;
  activeDeviceName?: string | null;
  currentTrackName?: string | null;
  currentArtistName?: string | null;
  currentCoverArtUrl?: string | null;
  currentAlbumName?: string | null;
  progressMs?: number | null;
  durationMs?: number | null;
  playbackState: string;
  isPlaying: boolean;
  currentVolumePercent?: number | null;
  currentItemType?: string | null;
  currentItemId?: string | null;
  isCurrentTrackSaved?: boolean | null;
  isShuffle?: boolean;
  grantedScopes?: string[];
  nextTrackPreview?: SpotifyTrackPreview | null;
  prevTrackPreview?: SpotifyTrackPreview | null;
  message: string;
}

export async function peekSpotifySkipTrack(
  direction: "next" | "prev"
): Promise<SpotifyTrackPreview | null> {
  return invoke<SpotifyTrackPreview | null>("peek_spotify_skip_track", {
    direction,
  });
}

export async function getSpotifyStatus(options?: {
  fresh?: boolean;
}): Promise<SpotifyStatus> {
  return invoke<SpotifyStatus>("get_spotify_status", {
    fresh: options?.fresh ?? false,
  });
}

export interface SpotifyClientConfig {
  clientId: string;
  lockedByEnv: boolean;
}

export async function getSpotifyClientConfig(): Promise<SpotifyClientConfig> {
  return invoke<SpotifyClientConfig>("get_spotify_client_config");
}

export async function setSpotifyClientId(clientId: string): Promise<void> {
  return invoke("set_spotify_client_id", { clientId });
}

export async function setSpotifyVolume(volumePercent: number): Promise<number> {
  return invoke<number>("set_spotify_volume", { volumePercent });
}
