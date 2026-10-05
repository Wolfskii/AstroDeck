import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export async function revealMainWindow(): Promise<void> {
  const win = getCurrentWindow();
  try {
    await invoke("restore_window_show_state");
  } catch {
    // Geometry is still restored at process start; show anyway.
  }
  try {
    await invoke("show_main_window_no_activate");
  } catch {
    await win.show();
    await win.unminimize();
  }
}

export async function hideMainWindow(): Promise<void> {
  const win = getCurrentWindow();
  await persistMainWindowState();
  await win.hide();
}

export async function persistMainWindowState(): Promise<void> {
  try {
    await invoke("persist_window_state");
  } catch {
    // Ignore: next quit still saves via the window-state plugin.
  }
}
