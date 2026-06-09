<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { TrayIcon } from "@tauri-apps/api/tray";
  import { defaultWindowIcon } from "@tauri-apps/api/app";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Menu } from "@tauri-apps/api/menu";
  import { invoke } from "@tauri-apps/api/core";
  import DeckGrid from "./components/DeckGrid.svelte";
  import MediaPlayerView from "./components/MediaPlayerView.svelte";
  import {
    executeActionValue,
    getActiveScene,
    getPlugins,
    getSpotifyClientConfig,
    getSpotifyStatus,
    setActiveScene,
    setSpotifyClientId,
  } from "./services/api";
  import type { SceneState, LayoutConfig, PluginConfig, DeckButtonConfig } from "./types";
  import {
    getBuiltinLayout,
    getBuiltinSceneIds,
    getBuiltinSceneMeta,
  } from "./layouts/layouts";
  import { logs, logInfo, logError, pushExternal, type LogEntry } from "./services/logger";

  type SpotifyStatus = import("./services/api").SpotifyStatus;

  const isTauri = !!(window as any).__TAURI_INTERNALS__ as boolean;

  let trayInitialized = false;
  let trayMenu: Menu | null = null;
  let windowLabel = $state("main");
  let seenScenes = $state<string[]>([]);
  // Tauri: main window can show deck or settings (viewMode). Browser: always settings/debug.
  let viewMode = $state<"deck" | "settings">("deck");
  const isSettingsWindow = $derived(!isTauri || viewMode === "settings");
  let logStatus = $state<"connecting" | "connected" | "disconnected">("connecting");
  let autoScroll = $state(true);
  let logsLinesEl = $state<HTMLElement | null>(null);
  let logBusSocket: WebSocket | null = null;
  let plugins = $state<PluginConfig[]>([]);
  let spotifyStatus = $state<SpotifyStatus | null>(null);
  let spotifyBusy = $state(false);
  let spotifyVolumeBusy = $state(false);
  let spotifySeekTargetMs = $state<number | null>(null);
  let spotifyVolumePercent = $state(50);
  /** Shown under Spotify buttons in the desktop app (Tauri has no debug log panel). */
  let spotifyAuthHint = $state<string | null>(null);
  let spotifyClientIdDraft = $state("");
  let spotifyClientLockedByEnv = $state(false);
  let spotifySavingClientId = $state(false);
  let optimisticSpotifySaved = $state<boolean | null>(null);
  let optimisticSpotifyShuffle = $state<boolean | null>(null);
  let optimisticSpotifyPlaying = $state<boolean | null>(null);
  let spotifyVolumeTarget = $state<number | null>(null);
  let spotifyStatusRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let spotifyStatusLastRefresh = 0;
  const SPOTIFY_STATUS_MIN_INTERVAL_MS = 5000;
  const SPOTIFY_STATUS_POLL_MS = 30000;
  const SPOTIFY_LOW_PRIORITY_REFRESH_ACTIONS = new Set([
    "spotify.togglePlay",
    "spotify.nextTrack",
    "spotify.prevTrack",
    "spotify.setVolume",
    "spotify.seek",
    "spotify.volumeUp",
    "spotify.volumeDown",
  ]);
  /** True when this window is in OS fullscreen (used after our fullscreen + borderless presentation). */
  let deckPresentationFullscreen = $state(false);
  const pluginsById = $derived(new Map(plugins.map((plugin) => [plugin.id, plugin])));
  const currentPlugin = $derived(plugins.find((plugin) => plugin.id === sceneId) ?? null);
  const effectiveSpotifySaved = $derived(
    optimisticSpotifySaved ?? spotifyStatus?.isCurrentTrackSaved ?? null
  );
  const effectiveSpotifyShuffle = $derived(
    optimisticSpotifyShuffle ?? spotifyStatus?.isShuffle ?? false
  );
  const effectiveSpotifyPlaybackState = $derived(
    optimisticSpotifyPlaying !== null
      ? optimisticSpotifyPlaying
        ? "playing"
        : "paused"
      : (spotifyStatus?.playbackState ?? "stopped")
  );
  const currentMediaView = $derived.by(() => {
    const media = currentPlugin?.view?.type === "mediaPlayer"
      ? currentPlugin.view.mediaPlayer
      : null;
    if (!media) return null;

    const withSpotifyLikeState = (button?: DeckButtonConfig | null) => {
      if (!button || button.action !== "spotify.like") return button ?? null;
      return {
        ...button,
        label: effectiveSpotifySaved ? "Remove" : "Like",
        emoji: effectiveSpotifySaved ? "🗑️" : "❤️",
      };
    };

    const withSpotifyPlayState = (button?: DeckButtonConfig | null) => {
      if (!button || button.action !== "spotify.togglePlay") return button ?? null;
      const playing =
        optimisticSpotifyPlaying ?? spotifyStatus?.isPlaying ?? false;
      return {
        ...button,
        label: playing ? "Pause" : "Play",
        emoji: playing ? "⏸️" : "▶️",
      };
    };

    return {
      previous: media.previous ?? null,
      playPause: withSpotifyPlayState(media.playPause),
      next: media.next ?? null,
      like: withSpotifyLikeState(media.like),
      shuffle: media.shuffle ?? null,
      volumeAction: media.volumeAction ?? null,
      seekAction: media.seekAction ?? null,
    };
  });
  const isMediaDeckView = $derived(isTauri && !isSettingsWindow && currentMediaView !== null);
  const displayedLayout = $derived.by(() => {
    if (!layout) return null;
    return layout;
  });

  $effect(() => {
    const next = spotifyStatus?.currentVolumePercent;
    if (next == null || spotifyVolumeBusy) return;
    if (spotifyVolumeTarget != null) {
      if (Math.abs(next - spotifyVolumeTarget) <= 2) {
        spotifyVolumeTarget = null;
      } else {
        return;
      }
    }
    spotifyVolumePercent = next;
  });

  const SPOTIFY_DEV_DASHBOARD = "https://developer.spotify.com/dashboard";

  $effect(() => {
    if (!isTauri || viewMode !== "settings") return;
    void (async () => {
      try {
        const c = await getSpotifyClientConfig();
        spotifyClientIdDraft = c.clientId;
        spotifyClientLockedByEnv = c.lockedByEnv;
      } catch {
        // ignore
      }
    })();
  });

  function openSpotifyDeveloperDashboard() {
    window.open(SPOTIFY_DEV_DASHBOARD, "_blank", "noopener,noreferrer");
  }

  async function syncDeckFullscreenState() {
    if (!isTauri) return;
    try {
      const win = getCurrentWindow();
      const fs = await win.isFullscreen();
      deckPresentationFullscreen = fs;
      if (!fs) {
        await win.setDecorations(true);
      }
    } catch {
      deckPresentationFullscreen = false;
    }
  }

  async function enterDeckPresentationFullscreen() {
    if (!isTauri) return;
    const win = getCurrentWindow();
    try {
      await win.setDecorations(false);
      await win.setFullscreen(true);
    } catch (e) {
      logError(String(e), "Window");
      try {
        await win.setFullscreen(false);
        await win.setDecorations(true);
      } catch {
        // ignore
      }
    }
    await syncDeckFullscreenState();
  }

  async function exitDeckPresentationFullscreen() {
    if (!isTauri) return;
    const win = getCurrentWindow();
    try {
      await win.setFullscreen(false);
      await win.setDecorations(true);
    } catch (e) {
      logError(String(e), "Window");
    }
    await syncDeckFullscreenState();
  }

  async function toggleDeckPresentationFullscreen() {
    if (!isTauri) return;
    try {
      if (await getCurrentWindow().isFullscreen()) {
        await exitDeckPresentationFullscreen();
      } else {
        await enterDeckPresentationFullscreen();
      }
    } catch (e) {
      logError(String(e), "Window");
      await syncDeckFullscreenState();
    }
  }

  async function saveSpotifyClientId() {
    if (!isTauri || spotifyClientLockedByEnv) return;
    spotifySavingClientId = true;
    spotifyAuthHint = null;
    try {
      await setSpotifyClientId(spotifyClientIdDraft.trim());
      await refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      spotifyAuthHint = spotifyClientIdDraft.trim()
        ? "Client ID saved locally."
        : "Cleared saved Client ID.";
    } catch (e) {
      spotifyAuthHint = String(e);
    } finally {
      spotifySavingClientId = false;
    }
  }

  function updateDebugTitle() {
    if (typeof document === "undefined" || isTauri) return;
    let icon = "🟠";
    let text = "connecting…";
    if (logStatus === "connected") {
      icon = "🟢";
      text = "connected";
    } else if (logStatus === "disconnected") {
      icon = "🔴";
      text = "disconnected";
    }
    document.title = `TapTapDeck - Debug panel ${icon} ${text}`;
  }

  function loadSeenScenes() {
    if (typeof window === "undefined") return;
    try {
      const raw = window.localStorage.getItem("taptapdeck:seenScenes");
      if (!raw) return;
      const parsed = JSON.parse(raw) as string[];
      if (Array.isArray(parsed)) {
        seenScenes = Array.from(new Set(parsed));
      }
    } catch {
      // ignore storage errors
    }
  }

  function persistSeenScenes() {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem("taptapdeck:seenScenes", JSON.stringify(seenScenes));
    } catch {
      // ignore storage errors
    }
  }

  function markSceneSeen(id: string) {
    if (!id) return;
    if (!seenScenes.includes(id)) {
      seenScenes = [...seenScenes, id];
      persistSeenScenes();
    }
  }

  async function copyLogs() {
    const entries = $logs;
    const lines = entries.map(
      (e) => `[${e.timestamp}] [${e.source}] ${e.level.toUpperCase()} ${e.message}`
    );
    const text = lines.join("\n") || "No logs yet.";
    try {
      await navigator.clipboard.writeText(text);
      logInfo("Copied logs to clipboard", "Logs");
    } catch (err) {
      logError(`Failed to copy logs: ${String(err)}`, "Logs");
    }
  }

  function clearLogs() {
    logs.set([]);
    logInfo("Cleared logs", "Logs");
  }

  function toggleAutoScroll(event: Event) {
    autoScroll = (event.currentTarget as HTMLInputElement).checked;
  }

  function requestSpotifyStatus() {
    if (logBusSocket?.readyState === WebSocket.OPEN) {
      logBusSocket.send(JSON.stringify({ type: "spotifyStatus" }));
    }
  }

  function requestPlugins() {
    if (logBusSocket?.readyState === WebSocket.OPEN) {
      logBusSocket.send(JSON.stringify({ type: "getPlugins" }));
    }
  }

  async function connectSpotify() {
    if (isTauri) {
      spotifyAuthHint = null;
      spotifyBusy = true;
      try {
        const url = await invoke<string>("start_spotify_auth");
        window.open(url, "_blank", "noopener,noreferrer");
        logInfo("Opened Spotify authorization page", "Spotify");
        spotifyAuthHint =
          "Finish signing in in your browser. When you return here, status updates automatically.";
      } catch (e) {
        const msg = String(e);
        spotifyAuthHint = msg;
        logError(msg, "Spotify");
      } finally {
        spotifyBusy = false;
      }
      return;
    }
    if (logBusSocket?.readyState !== WebSocket.OPEN) {
      logError("Log bus is not connected; cannot start Spotify auth", "Spotify");
      return;
    }
    spotifyBusy = true;
    logBusSocket.send(JSON.stringify({ type: "spotifyAuthStart" }));
  }

  async function disconnectSpotify() {
    if (isTauri) {
      spotifyBusy = true;
      try {
        await invoke("disconnect_spotify");
        spotifyAuthHint = null;
        await refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
        logInfo("Disconnected Spotify", "Spotify");
      } catch (e) {
        spotifyAuthHint = String(e);
        logError(String(e), "Spotify");
      } finally {
        spotifyBusy = false;
      }
      return;
    }
    if (logBusSocket?.readyState !== WebSocket.OPEN) {
      logError("Log bus is not connected; cannot disconnect Spotify", "Spotify");
      return;
    }
    spotifyBusy = true;
    logBusSocket.send(JSON.stringify({ type: "spotifyDisconnect" }));
  }

  async function refreshSpotifyStatusForDesktop(options?: {
    fresh?: boolean;
    immediate?: boolean;
  }) {
    if (!isTauri) return;

    const fresh = options?.fresh ?? false;
    const immediate = options?.immediate ?? false;

    if (!immediate && !fresh) {
      const elapsed = Date.now() - spotifyStatusLastRefresh;
      if (elapsed < SPOTIFY_STATUS_MIN_INTERVAL_MS) {
        if (spotifyStatusRefreshTimer) clearTimeout(spotifyStatusRefreshTimer);
        spotifyStatusRefreshTimer = setTimeout(() => {
          spotifyStatusRefreshTimer = null;
          void refreshSpotifyStatusForDesktop({ fresh });
        }, SPOTIFY_STATUS_MIN_INTERVAL_MS - elapsed);
        return;
      }
    }

    spotifyStatusLastRefresh = Date.now();
    try {
      const next = mergeSpotifyStatusFromServer(
        await getSpotifyStatus({ fresh })
      );
      spotifyStatus = next;
      if (next.isAuthenticated) {
        spotifyAuthHint = null;
      }
    } catch (e) {
      logError(`Failed to fetch Spotify status: ${String(e)}`, "Spotify");
    }
  }

  function mergeSpotifyStatusFromServer(next: SpotifyStatus): SpotifyStatus {
    const sameTrack =
      next.currentItemId != null &&
      next.currentItemId === spotifyStatus?.currentItemId;

    if (!sameTrack) {
      optimisticSpotifySaved = null;
      optimisticSpotifyPlaying = null;
      spotifyVolumeTarget = null;
      spotifySeekTargetMs = null;
    }

    let merged: SpotifyStatus = { ...next };

    if (optimisticSpotifyShuffle !== null) {
      if (next.isShuffle === optimisticSpotifyShuffle) {
        optimisticSpotifyShuffle = null;
      } else {
        merged.isShuffle = optimisticSpotifyShuffle;
      }
    }

    if (spotifySeekTargetMs != null && next.progressMs != null) {
      if (Math.abs(next.progressMs - spotifySeekTargetMs) <= 2500) {
        spotifySeekTargetMs = null;
      } else {
        merged.progressMs = spotifySeekTargetMs;
      }
    }

    if (spotifyVolumeTarget != null) {
      if (
        next.currentVolumePercent != null &&
        Math.abs(next.currentVolumePercent - spotifyVolumeTarget) <= 2
      ) {
        spotifyVolumeTarget = null;
      } else {
        merged.currentVolumePercent = spotifyVolumeTarget;
      }
    }

    if (sameTrack) {
      if (optimisticSpotifySaved !== null) {
        if (
          next.isCurrentTrackSaved != null &&
          next.isCurrentTrackSaved === optimisticSpotifySaved
        ) {
          optimisticSpotifySaved = null;
        } else {
          merged.isCurrentTrackSaved = optimisticSpotifySaved;
        }
      }

      if (optimisticSpotifyPlaying !== null) {
        if (next.isPlaying === optimisticSpotifyPlaying) {
          optimisticSpotifyPlaying = null;
        } else {
          merged.isPlaying = optimisticSpotifyPlaying;
          merged.playbackState = optimisticSpotifyPlaying ? "playing" : "paused";
        }
      }
    }

    return merged;
  }

  async function refreshPluginsForDesktop() {
    if (!isTauri) return;
    try {
      plugins = await getPlugins();
    } catch (e) {
      logError(`Failed to fetch plugins: ${String(e)}`, "Plugins");
    }
  }

  async function commitSceneVolume(action: string, value: number) {
    spotifyVolumePercent = value;
    spotifyVolumeTarget = value;
    try {
      spotifyVolumeBusy = true;
      await executeActionValue(action, value);
    } catch (e) {
      logError(`Failed to set volume via ${action}: ${String(e)}`, `${sceneId} window`);
      spotifyVolumeTarget = null;
    } finally {
      spotifyVolumeBusy = false;
    }
  }

  function commitSceneSeek(action: string, positionMs: number) {
    spotifySeekTargetMs = positionMs;
    if (spotifyStatus) {
      spotifyStatus = { ...spotifyStatus, progressMs: positionMs };
    }
    void (async () => {
      try {
        await executeActionValue(action, positionMs);
      } catch (e) {
        logError(`Failed to seek via ${action}: ${String(e)}`, `${sceneId} window`);
        spotifySeekTargetMs = null;
      }
    })();
  }

  $effect(() => {
    if (!isTauri || sceneId !== "spotify" || !spotifyStatus?.isPlaying) return;
    const id = window.setInterval(() => {
      void refreshSpotifyStatusForDesktop();
    }, SPOTIFY_STATUS_POLL_MS);
    return () => window.clearInterval(id);
  });

  $effect(() => {
    // Track log changes so this effect reruns when new entries arrive.
    const _entries = $logs;
    if (!logsLinesEl || !autoScroll) return;
    // Scroll to bottom whenever log entries change and autoScroll is enabled.
    logsLinesEl.scrollTop = logsLinesEl.scrollHeight;
  });

  async function initTray() {
    if (!isTauri || trayInitialized) return;
    trayInitialized = true;

    try {
      logInfo("Initializing system tray", "Tray");
      const icon = await defaultWindowIcon();
      const win = getCurrentWindow();

      trayMenu = await Menu.new({
        items: [
          {
            id: "toggle",
            text: "Show TapTapDeck",
            action: async () => {
              const visible = await win.isVisible();
              if (visible) {
                await win.hide();
                logInfo("Hid TapTapDeck window from tray menu", "Tray");
                const item = await trayMenu?.get("toggle");
                if (item) await item.setText("Show TapTapDeck");
              } else {
                viewMode = "deck";
                await win.show();
                await win.unminimize();
                await win.setFocus();
                logInfo("Showed TapTapDeck window from tray menu", "Tray");
                const item = await trayMenu?.get("toggle");
                if (item) await item.setText("Hide TapTapDeck");
              }
            },
          },
          {
            id: "settings",
            text: "Settings",
            action: async () => {
              logInfo("Settings requested from tray menu", "Tray");
              viewMode = "settings";
              await win.show();
              await win.unminimize();
              await win.setFocus();
              const item = await trayMenu?.get("toggle");
              if (item) await item.setText("Hide TapTapDeck");
            },
          },
          { item: "Separator" },
          {
            id: "quit",
            text: "Quit TapTapDeck",
            action: async () => {
              logInfo("Quit requested from tray menu", "Tray");
              await invoke("quit_app");
            },
          },
        ],
      });

      const trayOptions: Parameters<typeof TrayIcon.new>[0] = {
        menu: trayMenu,
        // Only show menu on right-click; left-click just shows the window
        menuOnLeftClick: false,
        action: async (event) => {
          if (
            (event.type === "Click" || event.type === "DoubleClick") &&
            // Only react to left-button clicks; right-click should only open the menu
            (event as any).button === "Left"
          ) {
            if (!(await win.isVisible())) {
              await win.show();
              await win.unminimize();
              await win.setFocus();
              logInfo("Tray icon clicked: showing window", "Tray");
              const item = await trayMenu?.get("toggle");
              if (item) await item.setText("Hide TapTapDeck");
            }
          }
        },
      };
      if (icon) {
        trayOptions.icon = icon;
      }
      await TrayIcon.new(trayOptions);
    } catch (e) {
      logError(`Failed to init tray: ${String(e)}`, "Tray");
    }
  }

  let sceneId = $state("default");
  let layout = $state<LayoutConfig | null>(null);
  let availableScenes = $state<string[]>([]);
  let loading = $state(true);
  const settingsSceneIds = $derived(
    Array.from(
      new Set([
        ...getBuiltinSceneIds(),
        ...plugins.map((plugin) => plugin.id),
        ...availableScenes,
        ...seenScenes,
      ])
    )
  );

  function selectScene(id: string) {
    if (!id) return;
    markSceneSeen(id);
    if (isTauri) {
      setActiveScene(id).catch((e) => {
        logError(`Failed to set active scene to ${id}: ${String(e)}`, "Settings window");
      });
    } else {
      if (logBusSocket?.readyState === WebSocket.OPEN) {
        logBusSocket.send(JSON.stringify({ type: "setScene", sceneId: id }));
      } else {
        logError(`Log bus is not connected; cannot set scene to ${id}`, "Settings window");
        const next = getBuiltinLayout(id) ?? getBuiltinLayout("default");
        if (next) {
          sceneId = id;
          layout = next;
        }
      }
    }
    logInfo(`Manually selected scene: ${id}`, "Settings window");
  }

  async function refreshScene() {
    try {
      const state: SceneState = await getActiveScene();
      sceneId = state.activeSceneId;
      layout = state.layout;
      availableScenes = state.availableScenes;
      logInfo(`Active scene: ${state.activeSceneId}`, "Scenes");
      markSceneSeen(state.activeSceneId);
      await refreshPluginsForDesktop();
      if (state.activeSceneId === "spotify") {
        await refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      }
    } catch (e) {
      logError(`Failed to fetch scene state: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    if (isTauri) {
      const win = getCurrentWindow();
      windowLabel = (win as any).label ?? "main";
    } else {
      windowLabel = "browser";
    }
    loadSeenScenes();
    const handleCoreAction = async (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string; label: string }>).detail;
      if (!detail) return;
      const { action, label } = detail;

      if (action === "core.settings") {
        logInfo(`Settings requested from button: ${label}`, "Settings window");
        if (isTauri) {
          viewMode = "settings";
          const win = getCurrentWindow();
          if (!(await win.isVisible())) {
            await win.show();
            await win.unminimize();
            await win.setFocus();
          }
        }
        // In browser debug mode we're already in the settings/debug window.
      } else if (action === "core.refresh") {
        if (isTauri) {
          logInfo("Refresh requested from button", "Scenes");
          refreshScene();
        } else {
          logInfo("Refresh requested from button (no-op in browser mode)", "Scenes");
        }
      } else if (action === "core.plugins" || action === "core.info") {
        logInfo(`Core action from button: ${label} (${action})`, "Core actions");
      }
    };

    const handleActionExecuted = (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string }>).detail;
      if (!detail?.action) return;
      if (detail.action.startsWith("spotify.")) {
        if (isTauri) {
          if (!SPOTIFY_LOW_PRIORITY_REFRESH_ACTIONS.has(detail.action)) {
            void refreshSpotifyStatusForDesktop();
          }
        } else if (!SPOTIFY_LOW_PRIORITY_REFRESH_ACTIONS.has(detail.action)) {
          requestSpotifyStatus();
        }
      }
    };

    const handleActionStarted = (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string }>).detail;
      if (!detail?.action) return;
      if (detail.action === "spotify.like") {
        const nextSaved = !(effectiveSpotifySaved === true);
        optimisticSpotifySaved = nextSaved;
        if (spotifyStatus) {
          spotifyStatus = { ...spotifyStatus, isCurrentTrackSaved: nextSaved };
        }
      }
      if (detail.action === "spotify.toggleShuffle") {
        const nextShuffle = !effectiveSpotifyShuffle;
        optimisticSpotifyShuffle = nextShuffle;
        if (spotifyStatus) {
          spotifyStatus = { ...spotifyStatus, isShuffle: nextShuffle };
        }
      }
      if (detail.action === "spotify.togglePlay") {
        const playing =
          optimisticSpotifyPlaying ?? spotifyStatus?.isPlaying ?? false;
        optimisticSpotifyPlaying = !playing;
      }
    };

    const handleActionFailed = (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string }>).detail;
      if (!detail?.action) return;
      if (detail.action === "spotify.like") {
        optimisticSpotifySaved = null;
      }
      if (detail.action === "spotify.toggleShuffle") {
        optimisticSpotifyShuffle = null;
      }
      if (detail.action === "spotify.togglePlay") {
        optimisticSpotifyPlaying = null;
      }
    };

    window.addEventListener("taptapdeck-core-action", handleCoreAction as EventListener);
    window.addEventListener("taptapdeck-action-started", handleActionStarted as EventListener);
    window.addEventListener("taptapdeck-action-executed", handleActionExecuted as EventListener);
    window.addEventListener("taptapdeck-action-failed", handleActionFailed as EventListener);

    if (!isTauri) {
      // Browser debug mode: start with a static default layout and settings view.
      const debugLayout = getBuiltinLayout("default");
      sceneId = "default";
      layout = debugLayout;
      availableScenes = getBuiltinSceneIds();
      loading = false;
      logStatus = "connecting";
      updateDebugTitle();
      logInfo("Browser mode: loaded builtin default layout", "Browser");
      markSceneSeen("default");

      // Connect to the Rust WebSocket log bus to receive live logs from the Tauri window
      let retryTimer: ReturnType<typeof setTimeout> | null = null;

      function connectLogBus() {
        const port = 3211;
        logStatus = "connecting";
        updateDebugTitle();
        logBusSocket = new WebSocket(`ws://127.0.0.1:${port}`);

        logBusSocket.onopen = () => {
          logStatus = "connected";
          updateDebugTitle();
          logInfo("Connected to TapTapDeck log bus", "Browser");
          spotifyBusy = false;
          requestPlugins();
          requestSpotifyStatus();
        };

        logBusSocket.onmessage = (ev) => {
          try {
            const payload = JSON.parse(ev.data as string) as
              | LogEntry
              | {
                  type?: string;
                  payload?: unknown;
                };
            if ("type" in payload && payload.type) {
              if (payload.type === "spotifyStatus") {
                spotifyStatus = mergeSpotifyStatusFromServer(payload.payload as SpotifyStatus);
                spotifyBusy = false;
              } else if (payload.type === "plugins") {
                plugins = (payload.payload as PluginConfig[]) ?? [];
              } else if (payload.type === "spotifyAuthUrl") {
                spotifyBusy = false;
                const url = (payload.payload as { url?: string })?.url;
                if (url) {
                  window.open(url, "_blank", "noopener,noreferrer");
                  logInfo("Opened Spotify authorization page", "Spotify");
                }
              } else if (payload.type === "spotifyAuthError") {
                spotifyBusy = false;
                const message = (payload.payload as { message?: string })?.message;
                logError(message ?? "Spotify authorization failed", "Spotify");
              }
              return;
            }
            pushExternal(payload as LogEntry);
          } catch {
            // ignore malformed frames
          }
        };

        logBusSocket.onclose = () => {
          logStatus = "disconnected";
          updateDebugTitle();
          spotifyBusy = false;
          logInfo("Log bus disconnected — retrying in 3 s…", "Browser");
          retryTimer = setTimeout(connectLogBus, 3000);
        };

        logBusSocket.onerror = () => {
          // close event fires right after, which triggers the retry
          logStatus = "connecting";
          updateDebugTitle();
          spotifyBusy = false;
        };
      }

      connectLogBus();

      return () => {
        if (retryTimer !== null) clearTimeout(retryTimer);
        logBusSocket?.close();
        logBusSocket = null;
        window.removeEventListener("taptapdeck-core-action", handleCoreAction as EventListener);
        window.removeEventListener("taptapdeck-action-started", handleActionStarted as EventListener);
        window.removeEventListener("taptapdeck-action-executed", handleActionExecuted as EventListener);
        window.removeEventListener("taptapdeck-action-failed", handleActionFailed as EventListener);
      };
    }

    // Tauri windows (main + settings)
    if (isTauri) {
      logInfo(`App mounted in Tauri environment (${windowLabel})`, "App");
      if (windowLabel === "main") {
        initTray();
      }
      void syncDeckFullscreenState();
      window.addEventListener("resize", syncDeckFullscreenState);

      const onEscapeFullscreen = (ev: KeyboardEvent) => {
        if (ev.key !== "Escape") return;
        void (async () => {
          try {
            if (!(await getCurrentWindow().isFullscreen())) return;
            ev.preventDefault();
            await exitDeckPresentationFullscreen();
          } catch {
            // ignore
          }
        })();
      };
      window.addEventListener("keydown", onEscapeFullscreen, true);

      refreshScene();
      refreshPluginsForDesktop();
      if (sceneId === "spotify") {
        void refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      }

      const onWindowFocus = () => {
        void refreshSpotifyStatusForDesktop();
      };
      window.addEventListener("focus", onWindowFocus);

      const unlisten = listen<SceneState>("scene-changed", (event) => {
        sceneId = event.payload.activeSceneId;
        layout = event.payload.layout;
        availableScenes = event.payload.availableScenes;
        logInfo(`Scene changed to: ${event.payload.activeSceneId}`, "Scenes");
        markSceneSeen(event.payload.activeSceneId);
        refreshPluginsForDesktop();
        if (event.payload.activeSceneId === "spotify") {
          void refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
        }
      });

      return () => {
        window.removeEventListener("resize", syncDeckFullscreenState);
        window.removeEventListener("keydown", onEscapeFullscreen, true);
        window.removeEventListener("focus", onWindowFocus);
        unlisten.then((fn) => fn());
        window.removeEventListener("taptapdeck-core-action", handleCoreAction as EventListener);
        window.removeEventListener("taptapdeck-action-started", handleActionStarted as EventListener);
        window.removeEventListener("taptapdeck-action-executed", handleActionExecuted as EventListener);
        window.removeEventListener("taptapdeck-action-failed", handleActionFailed as EventListener);
      };
    }
  });
</script>

<div class="app">
  {#if !isMediaDeckView}
    <header class="app-header">
      <div class="app-header-left">
        <h1 class="app-title">TapTapDeck</h1>
        {#if isTauri && viewMode === "settings"}
          <button type="button" class="back-to-deck" onclick={() => (viewMode = "deck")}>
            ← Back to deck
          </button>
        {:else if isTauri && !isSettingsWindow}
          <span class="scene-badge">{sceneId}</span>
        {/if}
      </div>
      {#if isTauri}
        <button
          type="button"
          class="header-fullscreen-btn"
          title={deckPresentationFullscreen ? "Exit fullscreen (Esc)" : "Fullscreen on this display"}
          aria-label={deckPresentationFullscreen ? "Exit fullscreen" : "Enter fullscreen"}
          onclick={() => toggleDeckPresentationFullscreen()}
        >
          {deckPresentationFullscreen ? "⤡" : "⛶ Fullscreen"}
        </button>
      {/if}
    </header>
  {/if}

  <main class="app-main">
    {#if isSettingsWindow}
      <div class="settings-root">
        <section class="settings-section">
          <h2>TapTapDeck Settings</h2>
          <div class="settings-block spotify-block">
            <div class="spotify-header">
              <div>
                <h3>Spotify Integration</h3>
                <p class="settings-help">
                  Connect Spotify once to enable track liking and Spotify-only volume control.
                </p>
                {#if isTauri}
                  <div class="spotify-client-block">
                    <label class="spotify-client-label" for="spotify-client-id">Spotify Client ID</label>
                    <p class="settings-help spotify-client-help">
                      Create a Spotify app, add redirect URI <code>http://127.0.0.1:43821/callback</code>,
                      then paste the Client ID here (saved on this PC). Or set <code>SPOTIFY_CLIENT_ID</code> in
                      the environment instead.
                    </p>
                    <div class="spotify-client-row">
                      <input
                        id="spotify-client-id"
                        class="spotify-client-input"
                        type="text"
                        autocomplete="off"
                        spellcheck="false"
                        placeholder="Your Spotify app Client ID"
                        bind:value={spotifyClientIdDraft}
                        disabled={spotifyClientLockedByEnv || spotifySavingClientId}
                      />
                      <button
                        type="button"
                        class="spotify-dashboard-btn"
                        onclick={openSpotifyDeveloperDashboard}
                      >
                        Open dashboard
                      </button>
                      <button
                        type="button"
                        class="spotify-save-client-btn"
                        onclick={saveSpotifyClientId}
                        disabled={spotifyClientLockedByEnv || spotifySavingClientId}
                      >
                        {spotifySavingClientId ? "Saving…" : "Save"}
                      </button>
                    </div>
                    {#if spotifyClientLockedByEnv}
                      <p class="spotify-env-note">
                        Using <code>SPOTIFY_CLIENT_ID</code> from the environment; unset it to edit the saved
                        Client ID here.
                      </p>
                    {/if}
                  </div>
                {/if}
              </div>
              <div class="spotify-actions">
                <button class="spotify-connect" onclick={connectSpotify} disabled={spotifyBusy}>
                  {#if spotifyBusy}
                    Connecting...
                  {:else if spotifyStatus?.isAuthenticated}
                    Reconnect Spotify
                  {:else}
                    Connect Spotify
                  {/if}
                </button>
                <button
                  class="spotify-disconnect"
                  onclick={disconnectSpotify}
                  disabled={spotifyBusy || !spotifyStatus?.isAuthenticated}
                >
                  Disconnect
                </button>
              </div>
            </div>
            {#if isTauri && spotifyAuthHint}
              <p
                class="spotify-auth-hint"
                class:spotify-auth-hint--ok={spotifyAuthHint.startsWith("Finish")}
              >
                {spotifyAuthHint}
              </p>
            {/if}
            <div class="spotify-status-card">
              {#if spotifyStatus?.currentCoverArtUrl || spotifyStatus?.currentTrackName}
                <div class="spotify-now-playing">
                  {#if spotifyStatus?.currentCoverArtUrl}
                    <img
                      class="spotify-cover-art"
                      src={spotifyStatus.currentCoverArtUrl}
                      alt={spotifyStatus?.currentTrackName ?? "Current cover art"}
                    />
                  {/if}
                  <div class="spotify-now-playing-meta">
                    <div class="spotify-playback-state">
                      {spotifyStatus?.playbackState ?? "stopped"}
                    </div>
                    <div class="spotify-track-title">
                      {spotifyStatus?.currentTrackName ?? "Nothing active"}
                    </div>
                    <div class="spotify-track-artist">
                      {spotifyStatus?.currentArtistName ?? "No artist information"}
                    </div>
                  </div>
                </div>
              {/if}
              <div class="spotify-status-top">
                <span
                  class:connected={spotifyStatus?.isAuthenticated}
                  class="spotify-status-dot"
                ></span>
                <span class="spotify-status-text">
                  {spotifyStatus?.message ?? "Waiting for Spotify status..."}
                </span>
              </div>
              <div class="spotify-status-grid">
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Auth</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.isAuthenticated ? "Connected" : "Not connected"}
                  </span>
                </div>
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Device</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.activeDeviceName ?? "No active device"}
                  </span>
                </div>
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Playback</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.playbackState ?? "stopped"}
                  </span>
                </div>
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Track</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.currentTrackName ?? "Nothing active"}
                  </span>
                </div>
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Volume</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.currentVolumePercent != null
                      ? `${spotifyStatus.currentVolumePercent}%`
                      : "Unavailable"}
                  </span>
                </div>
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Item Type</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.currentItemType ?? "Unknown"}
                  </span>
                </div>
                <div class="spotify-stat">
                  <span class="spotify-stat-label">Saved</span>
                  <span class="spotify-stat-value">
                    {spotifyStatus?.isCurrentTrackSaved == null
                      ? "Unavailable"
                      : spotifyStatus.isCurrentTrackSaved
                        ? "Saved"
                        : "Not saved"}
                  </span>
                </div>
                <div class="spotify-stat spotify-stat-wide">
                  <span class="spotify-stat-label">Item Id</span>
                  <span class="spotify-stat-value spotify-selectable">
                    {spotifyStatus?.currentItemId ?? "Unavailable"}
                  </span>
                </div>
              </div>
              <div class="spotify-scopes">
                {#if spotifyStatus?.grantedScopes?.length}
                  {#each spotifyStatus.grantedScopes as scope}
                    <span class="spotify-scope-pill spotify-selectable">{scope}</span>
                  {/each}
                {:else}
                  <span class="spotify-scope-pill muted">No scopes granted yet</span>
                {/if}
              </div>
            </div>
          </div>
          <div class="settings-scenes">
            <h3>Available scenes</h3>
            <p class="settings-help">Choose a scene to manually override the active desktop deck.</p>
            {#if settingsSceneIds.length === 0}
              <p class="settings-empty">No scenes loaded yet.</p>
            {:else}
              <div class="scene-grid">
                {#each settingsSceneIds as id}
                  {@const plugin = pluginsById.get(id)}
                  {@const meta = getBuiltinSceneMeta(id)}
                  {@const title = plugin?.name ?? meta?.name ?? id}
                  {@const description =
                    plugin?.description ??
                    meta?.description ??
                    "Manual scene override for this plugin layout."}
                  {@const accent = meta?.accent ?? "#8b5cf6"}
                  {@const grid = plugin?.layout.grid ?? meta?.layout.grid ?? [0, 0]}
                  {@const buttonCount = plugin?.layout.buttons.length ?? meta?.layout.buttons.length ?? 0}
                  <button
                    class:active={id === sceneId}
                    class="scene-card"
                    onclick={() => selectScene(id)}
                    aria-pressed={id === sceneId}
                    style={`--scene-accent: ${accent}`}
                  >
                    <div class="scene-card-top">
                      <span class="scene-name">{title}</span>
                      {#if id === sceneId}
                        <span class="scene-tag current">current</span>
                      {:else if seenScenes.includes(id)}
                        <span class="scene-tag">seen</span>
                      {:else}
                        <span class="scene-tag">manual</span>
                      {/if}
                    </div>
                    <div class="scene-id">{id}</div>
                    <p class="scene-description">
                      {description}
                    </p>
                    <div class="scene-card-footer">
                      <span class="scene-metrics">
                        {grid[0]}x{grid[1]}
                      </span>
                      <span class="scene-metrics">
                        {buttonCount} buttons
                      </span>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      </div>
    {:else}
      {#if loading}
        <div class="loading">Detecting environment...</div>
      {:else if currentMediaView}
        <MediaPlayerView
          previous={currentMediaView.previous}
          playPause={currentMediaView.playPause}
          next={currentMediaView.next}
          like={currentMediaView.like}
          shuffle={currentMediaView.shuffle}
          shuffleActive={sceneId === "spotify" ? effectiveSpotifyShuffle : false}
          trackSaved={sceneId === "spotify" ? effectiveSpotifySaved : null}
          title={sceneId === "spotify" ? spotifyStatus?.currentTrackName : null}
          subtitle={sceneId === "spotify" ? spotifyStatus?.currentArtistName : null}
          albumName={sceneId === "spotify" ? spotifyStatus?.currentAlbumName : null}
          artworkUrl={sceneId === "spotify" ? spotifyStatus?.currentCoverArtUrl : null}
          playbackState={sceneId === "spotify" ? effectiveSpotifyPlaybackState : "stopped"}
          progressMs={sceneId === "spotify" ? spotifyStatus?.progressMs : null}
          durationMs={sceneId === "spotify" ? spotifyStatus?.durationMs : null}
          volumeAction={currentMediaView.volumeAction}
          seekAction={currentMediaView.seekAction}
          volumePercent={spotifyVolumePercent}
          volumeBusy={spotifyVolumeBusy}
          volumeEnabled={sceneId !== "spotify" || !!spotifyStatus?.hasActiveDevice}
          seekEnabled={sceneId !== "spotify" || !!spotifyStatus?.hasActiveDevice}
          source={`${sceneId} window`}
          onVolumeCommit={(value) =>
            currentMediaView.volumeAction
              ? commitSceneVolume(currentMediaView.volumeAction, value)
              : Promise.resolve()}
          onSeekCommit={(positionMs) =>
            currentMediaView.seekAction
              ? commitSceneSeek(currentMediaView.seekAction, positionMs)
              : Promise.resolve()}
          showFullscreenToggle={isTauri && !deckPresentationFullscreen}
          presentationFullscreen={deckPresentationFullscreen}
          onToggleFullscreen={enterDeckPresentationFullscreen}
        />
      {:else if displayedLayout}
        <DeckGrid
          grid={displayedLayout.grid}
          buttons={displayedLayout.buttons}
          sceneId={sceneId}
        />
      {:else}
        <div class="loading">No active scene</div>
      {/if}
    {/if}
  </main>

  {#if !isTauri}
    <section class="logs-panel">
      <div class="logs-toolbar">
        <label class="logs-autoscroll">
          <input
            type="checkbox"
            checked={autoScroll}
            onchange={toggleAutoScroll}
          />
          Auto-scroll
        </label>
        <button
          class="logs-copy"
          onclick={copyLogs}
          title="Copy logs to clipboard"
          aria-label="Copy logs to clipboard"
        >
          📋
        </button>
      </div>
      <div class="logs-lines" bind:this={logsLinesEl}>
        {#each $logs as entry, i}
          <div class="log-line log-{entry.level}">
            <span class="log-ts">{entry.timestamp}</span>
            <span class="log-source">[{entry.source}]</span>
            <span class="log-msg">{entry.message}</span>
          </div>
        {/each}
        {#if !$logs.length}
          <div class="log-line log-empty">
            <span class="log-msg">No logs yet. Interact with the deck to see activity.</span>
          </div>
        {/if}
      </div>
      <button
        class="logs-clear"
        onclick={clearLogs}
        title="Clear logs"
        aria-label="Clear logs"
      >
        🧹
      </button>
    </section>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px 10px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    flex-shrink: 0;
  }

  .app-header-media {
    justify-content: flex-end;
    padding: 6px 14px 0;
    border-bottom: none;
    background: #0a0a0a;
  }

  .app-header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .header-fullscreen-btn {
    flex-shrink: 0;
    padding: 8px 14px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  .header-fullscreen-btn:hover {
    background: var(--bg-secondary);
  }

  .header-fullscreen-btn-icon {
    padding: 4px;
    border: none;
    background: transparent;
    font-size: 1.65rem;
    line-height: 1;
    color: rgba(255, 255, 255, 0.75);
  }

  .header-fullscreen-btn-icon:hover {
    background: transparent;
    color: rgba(255, 255, 255, 0.95);
  }

  .app-title {
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .scene-badge {
    font-size: 0.7rem;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--accent);
    color: white;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
  }

  .back-to-deck {
    font-size: 0.85rem;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
  }

  .back-to-deck:hover {
    background: var(--bg-secondary);
  }

  .app-main {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    overflow: hidden;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .settings-root {
    flex: 1;
    display: flex;
    padding: 16px 20px;
    gap: 16px;
    overflow: hidden;
  }

  .settings-section {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .settings-section h2 {
    margin: 0;
    font-size: 1.8rem;
    font-weight: 700;
  }

  .settings-section h3 {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    margin: 0 0 6px;
  }

  .settings-help {
    margin: 0 0 12px;
    color: var(--text-secondary);
    font-size: 0.95rem;
  }

  .settings-block {
    border: 1px solid var(--border-subtle);
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.03);
    padding: 16px;
  }

  .spotify-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .spotify-client-block {
    margin-top: 14px;
    max-width: 52rem;
  }

  .spotify-client-label {
    display: block;
    font-size: 0.8rem;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--text-secondary);
  }

  .spotify-client-help {
    margin-top: 0;
    margin-bottom: 10px;
  }

  .spotify-client-help code {
    font-size: 0.78em;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--bg-primary);
  }

  .spotify-client-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .spotify-client-input {
    flex: 1 1 220px;
    min-width: 0;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .spotify-client-input:disabled {
    opacity: 0.65;
  }

  .spotify-dashboard-btn,
  .spotify-save-client-btn {
    padding: 10px 14px;
    border-radius: 8px;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    border: 1px solid var(--border-subtle);
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .spotify-save-client-btn {
    background: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.35);
    color: #bfdbfe;
  }

  .spotify-dashboard-btn:hover,
  .spotify-save-client-btn:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .spotify-dashboard-btn:disabled,
  .spotify-save-client-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .spotify-env-note {
    margin: 10px 0 0;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  .spotify-env-note code {
    font-size: 0.85em;
  }

  .spotify-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .spotify-auth-hint {
    margin: 10px 0 0;
    font-size: 0.85rem;
    line-height: 1.4;
    color: #fecaca;
    max-width: 52ch;
  }

  .spotify-auth-hint--ok {
    color: #a7f3d0;
  }

  .spotify-connect {
    border: 1px solid rgba(34, 197, 94, 0.35);
    background: rgba(34, 197, 94, 0.16);
    color: #dcfce7;
    border-radius: 999px;
    padding: 10px 14px;
    font-weight: 700;
    cursor: pointer;
  }

  .spotify-connect:disabled {
    opacity: 0.65;
    cursor: progress;
  }

  .spotify-disconnect {
    border: 1px solid rgba(248, 113, 113, 0.3);
    background: rgba(248, 113, 113, 0.12);
    color: #fecaca;
    border-radius: 999px;
    padding: 10px 14px;
    font-weight: 700;
    cursor: pointer;
  }

  .spotify-disconnect:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .spotify-status-card {
    margin-top: 12px;
    padding: 14px;
    border-radius: 14px;
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.06);
    user-select: text;
    -webkit-user-select: text;
  }

  .spotify-now-playing {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }

  .spotify-cover-art {
    width: 86px;
    height: 86px;
    border-radius: 16px;
    object-fit: cover;
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.24);
  }

  .spotify-now-playing-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .spotify-playback-state {
    display: inline-flex;
    align-self: flex-start;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgba(34, 197, 94, 0.16);
    color: #bbf7d0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.72rem;
    font-weight: 800;
  }

  .spotify-track-title {
    font-size: 1.08rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .spotify-track-artist {
    color: var(--text-secondary);
    font-size: 0.96rem;
  }

  .spotify-status-top {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }

  .spotify-status-dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: #f59e0b;
    box-shadow: 0 0 10px rgba(245, 158, 11, 0.5);
  }

  .spotify-status-dot.connected {
    background: #22c55e;
    box-shadow: 0 0 10px rgba(34, 197, 94, 0.55);
  }

  .spotify-status-text {
    font-size: 0.95rem;
    color: var(--text-secondary);
  }

  .spotify-status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 10px;
  }

  .spotify-stat {
    padding: 10px 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .spotify-stat-wide {
    grid-column: 1 / -1;
  }

  .spotify-stat-label {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
  }

  .spotify-stat-value {
    font-size: 0.95rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .spotify-selectable {
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
    word-break: break-all;
  }

  .spotify-artist {
    margin-top: 10px;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .spotify-scopes {
    margin-top: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .spotify-scope-pill {
    font-size: 0.75rem;
    padding: 5px 9px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    user-select: text;
    -webkit-user-select: text;
  }

  .spotify-scope-pill.muted {
    opacity: 0.7;
  }

  .scene-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
  }

  .scene-card {
    appearance: none;
    border: 1px solid color-mix(in srgb, var(--scene-accent) 38%, var(--border-subtle));
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--scene-accent) 12%, transparent), transparent 55%),
      var(--bg-surface);
    border-radius: 16px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    text-align: left;
    color: var(--text-primary);
    cursor: pointer;
    transition:
      transform 120ms ease,
      border-color 120ms ease,
      box-shadow 120ms ease,
      background 120ms ease;
  }

  .scene-card:hover {
    transform: translateY(-1px);
    border-color: var(--scene-accent);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
  }

  .scene-card.active {
    border-color: var(--scene-accent);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--scene-accent) 55%, transparent),
      0 10px 30px rgba(0, 0, 0, 0.35);
  }

  .settings-empty {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .scene-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .scene-name {
    font-size: 1rem;
    font-weight: 700;
  }

  .scene-id {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: color-mix(in srgb, var(--scene-accent) 78%, white);
  }

  .scene-description {
    margin: 0;
    min-height: 2.6em;
    font-size: 0.92rem;
    line-height: 1.4;
    color: var(--text-secondary);
  }

  .scene-card-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: auto;
  }

  .scene-metrics {
    font-size: 0.78rem;
    padding: 4px 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
  }

  .scene-tag {
    font-size: 0.72rem;
    padding: 3px 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .scene-tag.current {
    background: color-mix(in srgb, var(--scene-accent) 24%, transparent);
    color: white;
  }

  .logs-panel {
    flex: 0 0 50%;
    background: #050509;
    border-top: 1px solid var(--border-subtle);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New",
      monospace;
    font-size: 1.4rem;
    padding: 6px 10px 30px;
    overflow-y: hidden;
    position: relative;
    display: flex;
    flex-direction: column;
    user-select: text;
    -webkit-user-select: text;
  }

  .logs-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .logs-autoscroll {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .logs-lines {
    flex: 1 1 auto;
    overflow-y: auto;
  }

  .log-line {
    display: flex;
    gap: 8px;
    align-items: baseline;
    padding: 1px 0;
    color: var(--text-secondary);
  }

  .log-line.log-info .log-msg {
    color: #a5b4fc;
  }

  .log-line.log-error .log-msg {
    color: #fca5a5;
  }

  .log-ts {
    opacity: 0.6;
    min-width: 60px;
  }

  .log-source {
    opacity: 0.75;
    min-width: 110px;
  }

  .log-msg {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .log-line.log-empty .log-msg {
    color: var(--text-secondary);
  }

  .logs-copy {
    position: sticky;
    top: 0;
    margin-left: auto;
    margin-bottom: 4px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.7rem;
    padding: 2px 8px;
    cursor: pointer;
  }

  .logs-copy:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .logs-clear {
    position: absolute;
    right: 10px;
    bottom: 6px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.7rem;
    padding: 2px 8px;
    cursor: pointer;
  }

  .logs-clear:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }
</style>
