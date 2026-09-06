import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_CONTROLS_OVERLAY_COLOR, parseHexColor } from "../lib/color";
import {
  parseSceneBackgroundId,
  type SceneBackgroundId,
} from "../lib/sceneBackgrounds";

export { DEFAULT_CONTROLS_OVERLAY_COLOR };

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

const CONTROLS_BACKDROP_KEY = "astrodeck:controlsBackdrop";
const CONTROLS_TRANSPARENCY_KEY = "astrodeck:controlsTransparency";

export const DEFAULT_CONTROLS_BACKDROP = true;
export const DEFAULT_CONTROLS_TRANSPARENCY = 35;

export function parseControlsTransparency(value: unknown): number {
  if (value == null || value === "") return DEFAULT_CONTROLS_TRANSPARENCY;
  const n = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(n)) return DEFAULT_CONTROLS_TRANSPARENCY;
  return Math.min(100, Math.max(0, Math.round(n)));
}

export async function getControlsBackdropEnabled(): Promise<boolean> {
  if (!isTauri) {
    try {
      const stored = window.localStorage.getItem(CONTROLS_BACKDROP_KEY);
      if (stored == null) return DEFAULT_CONTROLS_BACKDROP;
      return stored === "true";
    } catch {
      return DEFAULT_CONTROLS_BACKDROP;
    }
  }
  return invoke<boolean>("get_controls_backdrop_enabled");
}

export async function setControlsBackdropEnabled(enabled: boolean): Promise<void> {
  if (!isTauri) {
    try {
      window.localStorage.setItem(CONTROLS_BACKDROP_KEY, String(enabled));
    } catch {
      // ignore
    }
    return;
  }
  await invoke("set_controls_backdrop_enabled", { enabled });
}

export async function getControlsTransparency(): Promise<number> {
  if (!isTauri) {
    try {
      return parseControlsTransparency(window.localStorage.getItem(CONTROLS_TRANSPARENCY_KEY));
    } catch {
      return DEFAULT_CONTROLS_TRANSPARENCY;
    }
  }
  return parseControlsTransparency(await invoke<number>("get_controls_transparency"));
}

export async function setControlsTransparency(value: number): Promise<void> {
  const transparency = parseControlsTransparency(value);
  if (!isTauri) {
    try {
      window.localStorage.setItem(CONTROLS_TRANSPARENCY_KEY, String(transparency));
    } catch {
      // ignore
    }
    return;
  }
  await invoke("set_controls_transparency", { value: transparency });
}

const CONTROLS_OVERLAY_COLOR_KEY = "astrodeck:controlsOverlayColor";
const CONTROLS_OVERLAY_CUSTOM_KEY = "astrodeck:controlsOverlayCustom";

export const DEFAULT_CONTROLS_OVERLAY_CUSTOM = false;

export function parseControlsOverlayColor(value: unknown): string {
  return parseHexColor(value);
}

export function parseControlsOverlayCustom(value: unknown, color?: unknown): boolean {
  if (value === true || value === "true") return true;
  if (value === false || value === "false") return false;
  return parseControlsOverlayColor(color) !== DEFAULT_CONTROLS_OVERLAY_COLOR;
}

export async function getControlsOverlayColor(): Promise<string> {
  if (!isTauri) {
    try {
      return parseControlsOverlayColor(window.localStorage.getItem(CONTROLS_OVERLAY_COLOR_KEY));
    } catch {
      return DEFAULT_CONTROLS_OVERLAY_COLOR;
    }
  }
  return parseControlsOverlayColor(await invoke<string>("get_controls_overlay_color"));
}

export async function setControlsOverlayColor(color: string): Promise<void> {
  const next = parseControlsOverlayColor(color);
  if (!isTauri) {
    try {
      window.localStorage.setItem(CONTROLS_OVERLAY_COLOR_KEY, next);
    } catch {
      // ignore
    }
    return;
  }
  await invoke("set_controls_overlay_color", { color: next });
}

export async function getControlsOverlayCustom(): Promise<boolean> {
  if (!isTauri) {
    try {
      return parseControlsOverlayCustom(
        window.localStorage.getItem(CONTROLS_OVERLAY_CUSTOM_KEY),
        window.localStorage.getItem(CONTROLS_OVERLAY_COLOR_KEY)
      );
    } catch {
      return DEFAULT_CONTROLS_OVERLAY_CUSTOM;
    }
  }
  return invoke<boolean>("get_controls_overlay_custom");
}

export async function setControlsOverlayCustom(enabled: boolean): Promise<void> {
  if (!isTauri) {
    try {
      window.localStorage.setItem(CONTROLS_OVERLAY_CUSTOM_KEY, String(enabled));
    } catch {
      // ignore
    }
    return;
  }
  await invoke("set_controls_overlay_custom", { enabled });
}

export const DEFAULT_AUDIO_VISUALIZER = false;

export type AudioVisualizerStatus = {
  supported: boolean;
  enabled: boolean;
  running: boolean;
  error: string | null;
};

export async function getAudioVisualizerEnabled(): Promise<boolean> {
  if (!isTauri) return DEFAULT_AUDIO_VISUALIZER;
  return invoke<boolean>("get_audio_visualizer_enabled");
}

export async function setAudioVisualizerEnabled(enabled: boolean): Promise<void> {
  if (!isTauri) return;
  await invoke("set_audio_visualizer_enabled", { enabled });
}

export async function getAudioVisualizerStatus(): Promise<AudioVisualizerStatus> {
  if (!isTauri) {
    return {
      supported: false,
      enabled: DEFAULT_AUDIO_VISUALIZER,
      running: false,
      error: "System audio capture needs the desktop app.",
    };
  }
  return invoke<AudioVisualizerStatus>("get_audio_visualizer_status");
}

export async function setAudioVisualizerEmit(emit: boolean): Promise<void> {
  if (!isTauri) return;
  await invoke("set_audio_visualizer_emit", { emit });
}

export type SettingsThemeId = "system" | "light" | "dark";

export const DEFAULT_SETTINGS_THEME: SettingsThemeId = "system";
export const SETTINGS_THEME_KEY = "astrodeck:settingsTheme";

export function parseSettingsTheme(value: unknown): SettingsThemeId {
  if (value === "light" || value === "dark" || value === "system") return value;
  return DEFAULT_SETTINGS_THEME;
}

export async function getSettingsTheme(): Promise<SettingsThemeId> {
  if (!isTauri) {
    try {
      return parseSettingsTheme(window.localStorage.getItem(SETTINGS_THEME_KEY));
    } catch {
      return DEFAULT_SETTINGS_THEME;
    }
  }
  return parseSettingsTheme(await invoke<string>("get_settings_theme"));
}

export async function setSettingsTheme(theme: SettingsThemeId): Promise<SettingsThemeId> {
  const next = parseSettingsTheme(theme);
  if (!isTauri) {
    try {
      window.localStorage.setItem(SETTINGS_THEME_KEY, next);
    } catch {
      // ignore
    }
    return next;
  }
  return parseSettingsTheme(await invoke<string>("set_settings_theme", { theme: next }));
}

export async function getAutoSwitchScenes(): Promise<Record<string, boolean>> {
  if (!isTauri) return {};
  return invoke<Record<string, boolean>>("get_auto_switch_scenes");
}

export async function setAutoSwitchScene(
  sceneId: string,
  enabled: boolean
): Promise<Record<string, boolean>> {
  if (!isTauri) return { [sceneId]: enabled };
  return invoke<Record<string, boolean>>("set_auto_switch_scene", { sceneId, enabled });
}
