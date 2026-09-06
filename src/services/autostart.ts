import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

const isTauri =
  typeof window !== "undefined" &&
  !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

export async function isAutostartEnabled(): Promise<boolean> {
  if (!isTauri) return false;
  return isEnabled();
}

export async function setAutostartEnabled(enabled: boolean): Promise<void> {
  if (!isTauri) return;
  if (enabled) {
    await enable();
  } else {
    await disable();
  }
}
