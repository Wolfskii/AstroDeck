import { invoke } from "@tauri-apps/api/core";
import {
  parseSceneBackgroundId,
  type SceneBackgroundId,
} from "../lib/sceneBackgrounds";

const isTauri =
  typeof window !== "undefined" &&
  !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

const SCENE_BACKGROUND_KEY = "astrodeck:sceneBackground";

export async function getStartMinimized(): Promise<boolean> {
  if (!isTauri) return true;
  return invoke<boolean>("get_start_minimized");
}

export async function setStartMinimized(enabled: boolean): Promise<void> {
  if (!isTauri) return;
  await invoke("set_start_minimized", { enabled });
}

export async function getStartFullscreen(): Promise<boolean> {
  if (!isTauri) return false;
  return invoke<boolean>("get_start_fullscreen");
}

export async function setStartFullscreen(enabled: boolean): Promise<void> {
  if (!isTauri) return;
  await invoke("set_start_fullscreen", { enabled });
}

export async function getSceneBackground(): Promise<SceneBackgroundId> {
  if (!isTauri) {
    try {
      return parseSceneBackgroundId(window.localStorage.getItem(SCENE_BACKGROUND_KEY));
    } catch {
      return parseSceneBackgroundId(null);
    }
  }
  return parseSceneBackgroundId(await invoke<string>("get_scene_background"));
}

export async function setSceneBackground(id: SceneBackgroundId): Promise<void> {
  if (!isTauri) return;
  await invoke("set_scene_background", { id });
}

const SETTINGS_TERMINAL_KEY = "astrodeck:showSettingsTerminal";

export async function getShowSettingsTerminal(): Promise<boolean> {
  if (!isTauri) {
    try {
      const stored = window.localStorage.getItem(SETTINGS_TERMINAL_KEY);
      if (stored == null) return true;
      return stored === "true";
    } catch {
      return true;
    }
  }
  return invoke<boolean>("get_show_settings_terminal");
}

export async function setShowSettingsTerminal(enabled: boolean): Promise<void> {
  if (!isTauri) {
    try {
      window.localStorage.setItem(SETTINGS_TERMINAL_KEY, String(enabled));
    } catch {
      // ignore
    }
    return;
  }
  await invoke("set_show_settings_terminal", { enabled });
}
