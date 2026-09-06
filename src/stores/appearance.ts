import { writable } from "svelte/store";
import {
  DEFAULT_SCENE_BACKGROUND,
  parseSceneBackgroundId,
  type SceneBackgroundId,
} from "../lib/sceneBackgrounds";
import { getSceneBackground, setSceneBackground } from "../services/prefs";

const STORAGE_KEY = "astrodeck:sceneBackground";

function readStored(): SceneBackgroundId {
  if (typeof window === "undefined") return DEFAULT_SCENE_BACKGROUND;
  try {
    return parseSceneBackgroundId(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_SCENE_BACKGROUND;
  }
}

export const sceneBackgroundId = writable<SceneBackgroundId>(readStored());

sceneBackgroundId.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // ignore
  }
});

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key !== STORAGE_KEY || !event.newValue) return;
    sceneBackgroundId.set(parseSceneBackgroundId(event.newValue));
  });

  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<string>("scene-background-changed", (event) => {
        sceneBackgroundId.set(parseSceneBackgroundId(event.payload));
      })
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
}

export function persistSceneBackground(id: SceneBackgroundId): void {
  sceneBackgroundId.set(id);
  void setSceneBackground(id);
}
