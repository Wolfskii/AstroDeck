import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export async function revealMainWindow(): Promise<void> {
  const win = getCurrentWindow();
  try {
    await invoke("restore_window_show_state");
  } catch {
    // Geometry is still restored at process start; show anyway.
  }
  await win.show();
  await win.unminimize();
  await win.setFocus();
}

export async function persistMainWindowState(): Promise<void> {
  try {
    await invoke("persist_window_state");
  } catch {
    // Ignore: next quit still saves via the window-state plugin.
  }
}
