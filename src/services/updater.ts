import { invoke } from "@tauri-apps/api/core";

export interface AppUpdateInfo {
  available: boolean;
  currentVersion: string;
  latestVersion: string | null;
  releaseName: string | null;
  releaseNotes: string | null;
  releaseUrl: string | null;
  downloadUrl: string | null;
  installerName: string | null;
}

const isTauri =
  typeof window !== "undefined" &&
  !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

export async function getAppVersion(): Promise<string> {
  if (!isTauri) return "0.0.0";
  return invoke<string>("get_app_version");
}

export async function checkForAppUpdate(): Promise<AppUpdateInfo> {
  if (!isTauri) {
    return {
      available: false,
      currentVersion: "0.0.0",
      latestVersion: null,
      releaseName: null,
      releaseNotes: null,
      releaseUrl: null,
      downloadUrl: null,
      installerName: null,
    };
  }
  return invoke<AppUpdateInfo>("check_for_app_update");
}

export async function downloadAndInstallUpdate(downloadUrl: string): Promise<void> {
  if (!isTauri) return;
  return invoke("download_and_install_update", { downloadUrl });
}
