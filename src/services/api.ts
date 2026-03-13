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

export interface SpotifyStatus {
  isConfigured: boolean;
  isAuthenticated: boolean;
  hasActiveDevice: boolean;
  activeDeviceName?: string | null;
  currentTrackName?: string | null;
  currentArtistName?: string | null;
  currentVolumePercent?: number | null;
  currentItemType?: string | null;
  currentItemId?: string | null;
  isCurrentTrackSaved?: boolean | null;
  grantedScopes?: string[];
  message: string;
}

export async function getSpotifyStatus(): Promise<SpotifyStatus> {
  return invoke<SpotifyStatus>("get_spotify_status");
}

export async function setSpotifyVolume(volumePercent: number): Promise<number> {
  return invoke<number>("set_spotify_volume", { volumePercent });
}
