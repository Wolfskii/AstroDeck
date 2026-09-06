import { writable } from "svelte/store";
import {
  DEFAULT_SCENE_BACKGROUND,
  parseSceneBackgroundId,
  type SceneBackgroundId,
} from "../lib/sceneBackgrounds";
import {
  DEFAULT_CONTROLS_BACKDROP,
  DEFAULT_CONTROLS_TRANSPARENCY,
  getControlsBackdropEnabled,
  getControlsTransparency,
  getSceneBackground,
  parseControlsTransparency,
  setControlsBackdropEnabled,
  setControlsTransparency,
  setSceneBackground,
} from "../services/prefs";

const STORAGE_KEY = "astrodeck:sceneBackground";
const BACKDROP_KEY = "astrodeck:controlsBackdrop";
const TRANSPARENCY_KEY = "astrodeck:controlsTransparency";

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

export const sceneBackgroundId = writable<SceneBackgroundId>(readStored());
export const controlsBackdropEnabled = writable(readStoredBackdrop());
export const controlsTransparency = writable(readStoredTransparency());

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
  });

  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      Promise.all([
        listen<string>("scene-background-changed", (event) => {
          sceneBackgroundId.set(parseSceneBackgroundId(event.payload));
        }),
        listen<{ enabled: boolean; transparency: number }>("controls-backdrop-changed", (event) => {
          controlsBackdropEnabled.set(!!event.payload.enabled);
          controlsTransparency.set(parseControlsTransparency(event.payload.transparency));
        }),
      ])
    )
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
  } catch {
    // keep local value
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
