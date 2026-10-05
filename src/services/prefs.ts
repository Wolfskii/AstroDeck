import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_CONTROLS_OVERLAY_COLOR, parseHexColor } from "../lib/color";
import {
  parseSceneBackgroundId,
  type SceneBackgroundId,
} from "../lib/sceneBackgrounds";
import type { LyricsProviderId } from "./api";

export { DEFAULT_CONTROLS_OVERLAY_COLOR };

export const DEFAULT_LYRICS_PROVIDER_ORDER: LyricsProviderId[] = [
  "lrclib",
  "musixmatch",
  "kugou",
  "netease",
];
const LYRICS_PROVIDER_ORDER_KEY = "astrodeck:lyricsProviderOrder";

export function getLyricsProviderOrder(): LyricsProviderId[] {
  if (typeof window === "undefined") return [...DEFAULT_LYRICS_PROVIDER_ORDER];
  try {
    const parsed = JSON.parse(window.localStorage.getItem(LYRICS_PROVIDER_ORDER_KEY) ?? "null");
    if (!Array.isArray(parsed)) return [...DEFAULT_LYRICS_PROVIDER_ORDER];
    const valid = parsed.filter((id): id is LyricsProviderId =>
      DEFAULT_LYRICS_PROVIDER_ORDER.includes(id)
    );
    return [
      ...valid,
      ...DEFAULT_LYRICS_PROVIDER_ORDER.filter((id) => !valid.includes(id)),
    ];
  } catch {
    return [...DEFAULT_LYRICS_PROVIDER_ORDER];
  }
}

export function setLyricsProviderOrder(order: LyricsProviderId[]): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(LYRICS_PROVIDER_ORDER_KEY, JSON.stringify(order));
}

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

export async function getHideTaskbarIcon(): Promise<boolean> {
  if (!isTauri) return false;
  return invoke<boolean>("get_hide_taskbar_icon");
}

export async function setHideTaskbarIcon(enabled: boolean): Promise<void> {
  if (!isTauri) return;
  await invoke("set_hide_taskbar_icon", { enabled });
}

const CUSTOM_VIEWS_KEY = "astrodeck:customViews";

export type CustomViewPref = {
  id: string;
  name: string;
  url: string;
  icon: string;
  color: string;
};

export async function getCustomViews(): Promise<CustomViewPref[]> {
  if (!isTauri) {
    try {
      const parsed = JSON.parse(window.localStorage.getItem(CUSTOM_VIEWS_KEY) ?? "[]");
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }
  return invoke<CustomViewPref[]>("get_custom_views");
}

export async function setCustomViews(views: CustomViewPref[]): Promise<CustomViewPref[]> {
  if (!isTauri) {
    window.localStorage.setItem(CUSTOM_VIEWS_KEY, JSON.stringify(views));
    return views;
  }
  return invoke<CustomViewPref[]>("set_custom_views", { views });
}

const DISABLED_APPS_KEY = "astrodeck:disabledApps";

export async function getDisabledApps(): Promise<string[]> {
  if (!isTauri) {
    try {
      const parsed = JSON.parse(window.localStorage.getItem(DISABLED_APPS_KEY) ?? "[]");
      return Array.isArray(parsed) ? parsed.filter((id) => typeof id === "string") : [];
    } catch {
      return [];
    }
  }
  return invoke<string[]>("get_disabled_apps");
}

export async function setAppEnabled(appId: string, enabled: boolean): Promise<string[]> {
  if (!isTauri) {
    const current = await getDisabledApps();
    const next = current.filter((id) => id !== appId);
    if (!enabled) next.push(appId);
    window.localStorage.setItem(DISABLED_APPS_KEY, JSON.stringify(next));
    return next;
  }
  return invoke<string[]>("set_app_enabled", { appId, enabled });
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

export type TemperatureUnit = "celsius" | "fahrenheit";

export type ClockSettings = {
  temperatureUnit: TemperatureUnit;
  location: string;
  digitColor: string;
  digitBorder: boolean;
  digitBorderColor: string;
  digitBorderWidth: number;
  bgColorCustom: boolean;
  bgColor: string;
  weatherCard: boolean;
  weatherCardColor: string;
};

export const DEFAULT_TEMPERATURE_UNIT: TemperatureUnit = "celsius";
export const DEFAULT_CLOCK_DIGIT_COLOR = "#f3d37a";
export const DEFAULT_CLOCK_DIGIT_BORDER_COLOR = "#111111";
export const DEFAULT_CLOCK_DIGIT_BORDER_WIDTH = 4;
export const DEFAULT_CLOCK_BG_COLOR = "#3b82f6";
export const DEFAULT_CLOCK_WEATHER_CARD_COLOR = "#c5dbe8";
const TEMPERATURE_UNIT_KEY = "astrodeck:temperatureUnit";
const CLOCK_LOCATION_KEY = "astrodeck:clockLocation";
const CLOCK_DIGIT_COLOR_KEY = "astrodeck:clockDigitColor";
const CLOCK_DIGIT_BORDER_KEY = "astrodeck:clockDigitBorder";
const CLOCK_DIGIT_BORDER_COLOR_KEY = "astrodeck:clockDigitBorderColor";
const CLOCK_DIGIT_BORDER_WIDTH_KEY = "astrodeck:clockDigitBorderWidth";
const CLOCK_BG_COLOR_CUSTOM_KEY = "astrodeck:clockBgColorCustom";
const CLOCK_BG_COLOR_KEY = "astrodeck:clockBgColor";
const CLOCK_WEATHER_CARD_KEY = "astrodeck:clockWeatherCard";
const CLOCK_WEATHER_CARD_COLOR_KEY = "astrodeck:clockWeatherCardColor";

export function parseClockDigitColor(value: unknown): string {
  return parseHexColor(value, DEFAULT_CLOCK_DIGIT_COLOR);
}

export function parseClockDigitBorderColor(value: unknown): string {
  return parseHexColor(value, DEFAULT_CLOCK_DIGIT_BORDER_COLOR);
}

export function parseClockDigitBorderWidth(value: unknown): number {
  const width = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(width)) return DEFAULT_CLOCK_DIGIT_BORDER_WIDTH;
  return Math.min(16, Math.max(1, Math.round(width)));
}

export function parseClockBgColor(value: unknown): string {
  return parseHexColor(value, DEFAULT_CLOCK_BG_COLOR);
}

export function parseClockWeatherCardColor(value: unknown): string {
  return parseHexColor(value, DEFAULT_CLOCK_WEATHER_CARD_COLOR);
}

export function parseTemperatureUnit(value: unknown): TemperatureUnit {
  return value === "fahrenheit" ? "fahrenheit" : DEFAULT_TEMPERATURE_UNIT;
}

export async function getClockSettings(): Promise<ClockSettings> {
  if (!isTauri) {
    try {
      return {
        temperatureUnit: parseTemperatureUnit(window.localStorage.getItem(TEMPERATURE_UNIT_KEY)),
        location: window.localStorage.getItem(CLOCK_LOCATION_KEY) ?? "",
        digitColor: parseClockDigitColor(window.localStorage.getItem(CLOCK_DIGIT_COLOR_KEY)),
        digitBorder: window.localStorage.getItem(CLOCK_DIGIT_BORDER_KEY) === "true",
        digitBorderColor: parseClockDigitBorderColor(
          window.localStorage.getItem(CLOCK_DIGIT_BORDER_COLOR_KEY)
        ),
        digitBorderWidth: parseClockDigitBorderWidth(
          window.localStorage.getItem(CLOCK_DIGIT_BORDER_WIDTH_KEY)
        ),
        bgColorCustom: window.localStorage.getItem(CLOCK_BG_COLOR_CUSTOM_KEY) === "true",
        bgColor: parseClockBgColor(window.localStorage.getItem(CLOCK_BG_COLOR_KEY)),
        weatherCard: window.localStorage.getItem(CLOCK_WEATHER_CARD_KEY) === "true",
        weatherCardColor: parseClockWeatherCardColor(
          window.localStorage.getItem(CLOCK_WEATHER_CARD_COLOR_KEY)
        ),
      };
    } catch {
      return {
        temperatureUnit: DEFAULT_TEMPERATURE_UNIT,
        location: "",
        digitColor: DEFAULT_CLOCK_DIGIT_COLOR,
        digitBorder: false,
        digitBorderColor: DEFAULT_CLOCK_DIGIT_BORDER_COLOR,
        digitBorderWidth: DEFAULT_CLOCK_DIGIT_BORDER_WIDTH,
        bgColorCustom: false,
        bgColor: DEFAULT_CLOCK_BG_COLOR,
        weatherCard: false,
        weatherCardColor: DEFAULT_CLOCK_WEATHER_CARD_COLOR,
      };
    }
  }
  const settings = await invoke<ClockSettings>("get_clock_settings");
  return normalizeClockSettings(settings);
}

function normalizeClockSettings(settings: ClockSettings): ClockSettings {
  return {
    temperatureUnit: parseTemperatureUnit(settings.temperatureUnit),
    location: (settings.location ?? "").trim(),
    digitColor: parseClockDigitColor(settings.digitColor),
    digitBorder: !!settings.digitBorder,
    digitBorderColor: parseClockDigitBorderColor(settings.digitBorderColor),
    digitBorderWidth: parseClockDigitBorderWidth(settings.digitBorderWidth),
    bgColorCustom: !!settings.bgColorCustom,
    bgColor: parseClockBgColor(settings.bgColor),
    weatherCard: !!settings.weatherCard,
    weatherCardColor: parseClockWeatherCardColor(settings.weatherCardColor),
  };
}

export async function setClockSettings(settings: ClockSettings): Promise<ClockSettings> {
  const next = normalizeClockSettings(settings);
  if (!isTauri) {
    try {
      window.localStorage.setItem(TEMPERATURE_UNIT_KEY, next.temperatureUnit);
      window.localStorage.setItem(CLOCK_LOCATION_KEY, next.location);
      window.localStorage.setItem(CLOCK_DIGIT_COLOR_KEY, next.digitColor);
      window.localStorage.setItem(CLOCK_DIGIT_BORDER_KEY, String(next.digitBorder));
      window.localStorage.setItem(CLOCK_DIGIT_BORDER_COLOR_KEY, next.digitBorderColor);
      window.localStorage.setItem(CLOCK_DIGIT_BORDER_WIDTH_KEY, String(next.digitBorderWidth));
      window.localStorage.setItem(CLOCK_BG_COLOR_CUSTOM_KEY, String(next.bgColorCustom));
      window.localStorage.setItem(CLOCK_BG_COLOR_KEY, next.bgColor);
      window.localStorage.setItem(CLOCK_WEATHER_CARD_KEY, String(next.weatherCard));
      window.localStorage.setItem(CLOCK_WEATHER_CARD_COLOR_KEY, next.weatherCardColor);
    } catch {
      // ignore
    }
    return next;
  }
  return normalizeClockSettings(
    await invoke<ClockSettings>("set_clock_settings", {
      temperatureUnit: next.temperatureUnit,
      location: next.location,
      digitColor: next.digitColor,
      digitBorder: next.digitBorder,
      digitBorderColor: next.digitBorderColor,
      digitBorderWidth: next.digitBorderWidth,
      bgColorCustom: next.bgColorCustom,
      bgColor: next.bgColor,
      weatherCard: next.weatherCard,
      weatherCardColor: next.weatherCardColor,
    })
  );
}
