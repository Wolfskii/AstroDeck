import { writable } from "svelte/store";
import {
  DEFAULT_SCENE_BACKGROUND,
  parseSceneBackgroundId,
  type SceneBackgroundId,
} from "../lib/sceneBackgrounds";
import {
  DEFAULT_CONTROLS_BACKDROP,
  DEFAULT_CONTROLS_OVERLAY_COLOR,
  DEFAULT_CONTROLS_OVERLAY_CUSTOM,
  DEFAULT_CONTROLS_TRANSPARENCY,
  getControlsBackdropEnabled,
  getControlsOverlayColor,
  getControlsOverlayCustom,
  getControlsTransparency,
  getSceneBackground,
  getAudioVisualizerEnabled,
  getAudioVisualizerStatus,
  parseControlsOverlayColor,
  parseControlsOverlayCustom,
  parseControlsTransparency,
  setControlsBackdropEnabled,
  setControlsOverlayColor,
  setControlsOverlayCustom,
  setControlsTransparency,
  setSceneBackground,
  setAudioVisualizerEnabled,
  DEFAULT_AUDIO_VISUALIZER,
} from "../services/prefs";
import { subscribeOsAudioError } from "../lib/osAudioViz";

const STORAGE_KEY = "astrodeck:sceneBackground";
const BACKDROP_KEY = "astrodeck:controlsBackdrop";
const TRANSPARENCY_KEY = "astrodeck:controlsTransparency";
const OVERLAY_COLOR_KEY = "astrodeck:controlsOverlayColor";
const OVERLAY_CUSTOM_KEY = "astrodeck:controlsOverlayCustom";
const AUDIO_VISUALIZER_KEY = "astrodeck:audioVisualizer";

function readStored(): SceneBackgroundId {
  if (typeof window === "undefined") return DEFAULT_SCENE_BACKGROUND;
  try {
    return parseSceneBackgroundId(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_SCENE_BACKGROUND;
  }
}

function readStoredBackdrop(): boolean {
  if (typeof window === "undefined") return DEFAULT_CONTROLS_BACKDROP;
  try {
    const stored = window.localStorage.getItem(BACKDROP_KEY);
    if (stored == null) return DEFAULT_CONTROLS_BACKDROP;
    return stored === "true";
  } catch {
    return DEFAULT_CONTROLS_BACKDROP;
  }
}

function readStoredTransparency(): number {
  if (typeof window === "undefined") return DEFAULT_CONTROLS_TRANSPARENCY;
  try {
    return parseControlsTransparency(window.localStorage.getItem(TRANSPARENCY_KEY));
  } catch {
    return DEFAULT_CONTROLS_TRANSPARENCY;
  }
}

function readStoredOverlayColor(): string {
  if (typeof window === "undefined") return DEFAULT_CONTROLS_OVERLAY_COLOR;
  try {
    return parseControlsOverlayColor(window.localStorage.getItem(OVERLAY_COLOR_KEY));
  } catch {
    return DEFAULT_CONTROLS_OVERLAY_COLOR;
  }
}

function readStoredOverlayCustom(): boolean {
  if (typeof window === "undefined") return DEFAULT_CONTROLS_OVERLAY_CUSTOM;
  try {
    return parseControlsOverlayCustom(
      window.localStorage.getItem(OVERLAY_CUSTOM_KEY),
      window.localStorage.getItem(OVERLAY_COLOR_KEY)
    );
  } catch {
    return DEFAULT_CONTROLS_OVERLAY_CUSTOM;
  }
}

function readStoredAudioVisualizer(): boolean {
  if (typeof window === "undefined") return DEFAULT_AUDIO_VISUALIZER;
  try {
    const stored = window.localStorage.getItem(AUDIO_VISUALIZER_KEY);
    if (stored == null) return DEFAULT_AUDIO_VISUALIZER;
    return stored === "true";
  } catch {
    return DEFAULT_AUDIO_VISUALIZER;
  }
}

export const sceneBackgroundId = writable<SceneBackgroundId>(readStored());
export const controlsBackdropEnabled = writable(readStoredBackdrop());
export const controlsTransparency = writable(readStoredTransparency());
export const controlsOverlayColor = writable(readStoredOverlayColor());
export const controlsOverlayCustom = writable(readStoredOverlayCustom());
export const audioVisualizerEnabled = writable(readStoredAudioVisualizer());
export const audioVisualizerSupported = writable(false);
export const audioVisualizerError = writable<string | null>(null);

sceneBackgroundId.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // ignore
  }
});

controlsBackdropEnabled.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(BACKDROP_KEY, String(value));
  } catch {
    // ignore
  }
});

controlsTransparency.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(TRANSPARENCY_KEY, String(value));
  } catch {
    // ignore
  }
});

controlsOverlayColor.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(OVERLAY_COLOR_KEY, value);
  } catch {
    // ignore
  }
});

controlsOverlayCustom.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(OVERLAY_CUSTOM_KEY, String(value));
  } catch {
    // ignore
  }
});

audioVisualizerEnabled.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(AUDIO_VISUALIZER_KEY, String(value));
  } catch {
    // ignore
  }
});

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === STORAGE_KEY && event.newValue) {
      sceneBackgroundId.set(parseSceneBackgroundId(event.newValue));
    }
    if (event.key === BACKDROP_KEY && event.newValue != null) {
      controlsBackdropEnabled.set(event.newValue === "true");
    }
    if (event.key === TRANSPARENCY_KEY && event.newValue != null) {
      controlsTransparency.set(parseControlsTransparency(event.newValue));
    }
    if (event.key === OVERLAY_COLOR_KEY && event.newValue != null) {
      controlsOverlayColor.set(parseControlsOverlayColor(event.newValue));
    }
    if (event.key === OVERLAY_CUSTOM_KEY && event.newValue != null) {
      controlsOverlayCustom.set(parseControlsOverlayCustom(event.newValue));
    }
    if (event.key === AUDIO_VISUALIZER_KEY && event.newValue != null) {
      audioVisualizerEnabled.set(event.newValue === "true");
    }
  });

  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      Promise.all([
        listen<string>("scene-background-changed", (event) => {
          sceneBackgroundId.set(parseSceneBackgroundId(event.payload));
        }),
        listen<{ enabled: boolean; transparency: number; color: string; custom?: boolean }>(
          "controls-backdrop-changed",
          (event) => {
            controlsBackdropEnabled.set(!!event.payload.enabled);
            controlsTransparency.set(parseControlsTransparency(event.payload.transparency));
            controlsOverlayColor.set(parseControlsOverlayColor(event.payload.color));
            controlsOverlayCustom.set(
              parseControlsOverlayCustom(event.payload.custom, event.payload.color)
            );
          }
        ),
        listen<boolean>("audio-visualizer-changed", (event) => {
          audioVisualizerEnabled.set(!!event.payload);
        }),
      ])
    )
    .then(() => {
      subscribeOsAudioError((error) => {
        audioVisualizerError.set(error);
      });
    })
    .catch(() => {
      // browser / unavailable
    });
}

export async function hydrateSceneBackground(): Promise<void> {
  try {
    sceneBackgroundId.set(await getSceneBackground());
  } catch {
    // keep local value
  }
  try {
    controlsBackdropEnabled.set(await getControlsBackdropEnabled());
    controlsTransparency.set(await getControlsTransparency());
    controlsOverlayColor.set(await getControlsOverlayColor());
    controlsOverlayCustom.set(await getControlsOverlayCustom());
  } catch {
    // keep local value
  }
  try {
    const status = await getAudioVisualizerStatus();
    audioVisualizerSupported.set(status.supported);
    audioVisualizerEnabled.set(status.enabled);
    audioVisualizerError.set(status.error ?? null);
  } catch {
    try {
      audioVisualizerEnabled.set(await getAudioVisualizerEnabled());
    } catch {
      // keep local value
    }
  }
}

export function persistSceneBackground(id: SceneBackgroundId): void {
  sceneBackgroundId.set(id);
  void setSceneBackground(id);
}

export function persistControlsBackdropEnabled(enabled: boolean): void {
  controlsBackdropEnabled.set(enabled);
  void setControlsBackdropEnabled(enabled);
}

export function persistControlsTransparency(value: number): void {
  const next = parseControlsTransparency(value);
  controlsTransparency.set(next);
  void setControlsTransparency(next);
}

export function persistControlsOverlayColor(color: string): void {
  const next = parseControlsOverlayColor(color);
  controlsOverlayColor.set(next);
  void setControlsOverlayColor(next);
}

export function persistControlsOverlayCustom(enabled: boolean): void {
  controlsOverlayCustom.set(enabled);
  void setControlsOverlayCustom(enabled);
}

export function persistAudioVisualizerEnabled(enabled: boolean): void {
  audioVisualizerEnabled.set(enabled);
  if (enabled) audioVisualizerError.set(null);
  void setAudioVisualizerEnabled(enabled).then(async () => {
    try {
      const status = await getAudioVisualizerStatus();
      audioVisualizerSupported.set(status.supported);
      audioVisualizerError.set(status.error ?? null);
    } catch {
      // keep current status
    }
  });
}

export function previewControlsOverlayColor(color: string): void {
  controlsOverlayColor.set(parseControlsOverlayColor(color));
}
