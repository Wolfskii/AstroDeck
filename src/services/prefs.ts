import { invoke } from "@tauri-apps/api/core";

const isTauri =
  typeof window !== "undefined" &&
  !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

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
