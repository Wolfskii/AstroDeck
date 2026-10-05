<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { TrayIcon } from "@tauri-apps/api/tray";
  import { defaultWindowIcon } from "@tauri-apps/api/app";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Menu } from "@tauri-apps/api/menu";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import ClockWeatherView from "./components/ClockWeatherView.svelte";
  import PerformanceView from "./components/PerformanceView.svelte";
  import WeatherView from "./components/WeatherView.svelte";
  import DeckGrid from "./components/DeckGrid.svelte";
  import MediaPlayerView from "./components/MediaPlayerView.svelte";
  import ReleaseNotes from "./components/ReleaseNotes.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import EmbedView from "./components/EmbedView.svelte";
  import SideNav from "./components/SideNav.svelte";
  import TeamsScene from "./components/TeamsScene.svelte";
  import VsCodeScene from "./components/VsCodeScene.svelte";
  import SceneBackground from "./components/SceneBackground.svelte";
  import {
    executeActionValue,
    getActiveScene,
    getOsNowPlaying,
    getOutputVolume,
    getPlugins,
    getSpotifyClientConfig,
    getSpotifyStatus,
    getYouTubeMusicStatus,
    getYouTubeMusicLibrary,
    getTeamsSetupStatus,
    getTeamsStatus,
    setupTeamsIntegration,
    peekSpotifySkipTrack,
    setActiveScene,
    setSpotifyAuthMode,
    setSpotifyClientId,
  } from "./services/api";
  import type {
    OsNowPlaying,
    SpotifyAuthMode,
    SpotifyTrackPreview,
    YouTubeMusicStatus,
    YouTubeLocalLibrary,
    TeamsSetupStatus,
    TeamsStatus,
  } from "./services/api";
  import type { SceneState, LayoutConfig, PluginConfig, DeckButtonConfig } from "./types";
  import {
    getBuiltinLayout,
    getBuiltinSceneIds,
    isIdleScene,
  } from "./layouts/layouts";
  import { usesCoverImage, usesFullViewBackground } from "./lib/sceneBackgrounds";
  import { hexToHsv, isNeutralAccent, shaderColorsFromAccent, shaderColorsFromHue } from "./lib/color";
  import type { CustomView } from "./lib/customViews";
  import { extractAlbumPalette, paletteToShaderColors } from "./lib/albumArtColor";
  import { logs, logInfo, logError, pushExternal, type LogEntry } from "./services/logger";
  import {
    checkForAppUpdate,
    downloadAndInstallUpdate,
    getAppVersion,
    getUpdatePopupsEnabled,
    setUpdatePopupsEnabled,
    type AppUpdateInfo,
  } from "./services/updater";
  import { isAutostartEnabled, setAutostartEnabled } from "./services/autostart";
  import {
    getStartMinimized,
    setStartMinimized,
    getHideTaskbarIcon,
    setHideTaskbarIcon,
    getCustomViews,
    setCustomViews,
    getDisabledApps,
    setAppEnabled,
    getStartFullscreen,
    setStartFullscreen,
    getShowSettingsTerminal,
    setShowSettingsTerminal,
    getAutoSwitchScenes,
    setAutoSwitchScene,
    getLyricsProviderOrder,
    setLyricsProviderOrder,
  } from "./services/prefs";
  import { hideMainWindow, persistMainWindowState, revealMainWindow } from "./services/windowState";
  import { hydrateSceneBackground, sceneBackgroundId } from "./stores/appearance";
  import { sceneShaderColors, sceneShaderImage } from "./stores/sceneVisual";
  import { clockBgColor, clockBgColorCustom, clockDigitColor, loadClockSettings } from "./stores/clock";
  import { performanceOpen, weatherDetailOpen } from "./stores/navigation";
  import { startWeatherUpdates } from "./stores/weather";

  type SpotifyStatus = import("./services/api").SpotifyStatus;

  const isTauri = !!(window as any).__TAURI_INTERNALS__ as boolean;

  let trayInitialized = false;
  let appUpdate = $state<AppUpdateInfo | null>(null);
  let appUpdateBusy = $state(false);
  let appUpdateChecking = $state(false);
  let appUpdateError = $state<string | null>(null);
  let appVersion = $state("");
  let updatePopupOpen = $state(false);
  let showUpdatePopups = $state(true);
  let showUpdatePopupsBusy = $state(false);
  const installableUpdate = $derived(
    Boolean(appUpdate?.available && appUpdate.downloadUrl && appUpdate.latestVersion)
  );
  let trayMenu: Menu | null = null;
  let windowLabel = $state("");
  let seenScenes = $state<string[]>([]);
  // Tauri: main window can show deck or settings (viewMode). Browser: always settings/debug.
  let viewMode = $state<"deck" | "settings">("deck");
  let showSettingsTerminal = $state(!isTauri);
  let showSettingsTerminalBusy = $state(false);
  let autoSwitchScenes = $state<Record<string, boolean>>({});
  let autoSwitchBusyId = $state<string | null>(null);
  let customViews = $state<CustomView[]>([]);
  let customViewsBusy = $state(false);
  let disabledApps = $state<string[]>([]);
  let appToggleBusyId = $state<string | null>(null);
  let activeCustomViewId = $state<string | null>(null);
  let keptWebViewIds = $state<string[]>([]);
  const isSettingsWindow = $derived(!isTauri || viewMode === "settings");
  const showGlobalSceneBackground = $derived(
    !isSettingsWindow &&
      usesFullViewBackground($sceneBackgroundId) &&
      !(viewMode === "deck" && !$performanceOpen && !$weatherDetailOpen && activeCustomViewId)
  );
  const showLogsPanel = $derived(isSettingsWindow && showSettingsTerminal);
  let logStatus = $state<"connecting" | "connected" | "disconnected">("connecting");
  let autoScroll = $state(true);
  let logsLinesEl = $state<HTMLElement | null>(null);
  let logBusSocket: WebSocket | null = null;
  let plugins = $state<PluginConfig[]>([]);
  let spotifyStatus = $state<SpotifyStatus | null>(null);
  let youtubeMusicStatus = $state<YouTubeMusicStatus | null>(null);
  let youtubeMusicLibrary = $state<YouTubeLocalLibrary | null>(null);
  let teamsSetupStatus = $state<TeamsSetupStatus | null>(null);
  let teamsStatus = $state<TeamsStatus | null>(null);
  let lyricsProviderOrder = $state(getLyricsProviderOrder());
  let spotifyBusy = $state(false);
  let spotifyVolumeBusy = $state(false);
  let spotifySeekTargetMs = $state<number | null>(null);
  let spotifyVolumePercent = $state(80);
  let spotifyAuthHint = $state<string | null>(null);
  let spotifyClientIdDraft = $state("");
  let spotifyAuthMode = $state<SpotifyAuthMode>("official");
  let spotifyClientLockedByEnv = $state(false);
  let spotifySavingClientId = $state(false);
  let spotifySavingAuthMode = $state(false);
  let startOnBoot = $state(false);
  let startOnBootBusy = $state(false);
  let startOnBootError = $state<string | null>(null);
  let startMinimized = $state(true);
  let startMinimizedBusy = $state(false);
  let hideTaskbarIcon = $state(false);
  let hideTaskbarIconBusy = $state(false);
  let startFullscreen = $state(false);
  let startFullscreenBusy = $state(false);
  let osLocalNowPlaying = $state<OsNowPlaying | null>(null);
  let localProgressAt = Date.now();
  let spotifyProgressAt = Date.now();
  let optimisticMediaPlaying = $state<boolean | null>(null);
  let osVolumePercent = $state(50);
  let osVolumeBusy = $state(false);
  let osVolumeTarget = $state<number | null>(null);
  let optimisticSpotifySaved = $state<boolean | null>(null);
  let optimisticSpotifyShuffle = $state<boolean | null>(null);
  let optimisticSpotifyPlaying = $state<boolean | null>(null);
  let pinnedSpotifyItemId: string | null = null;
  let pinnedSpotifyFromId: string | null = null;
  let pinnedSpotifyUntil = 0;
  let osNowPlayingHold: { previousItemId: string | null } | null = null;
  let localSkipHold: { fromKey: string; until: number } | null = null;
  let spotifyVolumeTarget = $state<number | null>(null);
  let spotifyStatusRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let spotifyTransportRefreshTimers: ReturnType<typeof setTimeout>[] = [];
  let spotifyQueueRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let spotifyQueueRefreshPending = $state(false);
  let spotifyStatusLastRefresh = 0;
  const SPOTIFY_STATUS_MIN_INTERVAL_MS = 5000;
  /** Slow fallback only. Track changes come from the OS now-playing session. */
  const SPOTIFY_STATUS_POLL_MS = 180000;
  /** Spotify restarts the current track on prev when playback is past this point. */
  const SPOTIFY_PREV_RESTART_THRESHOLD_MS = 3000;
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
  const youtubeTrackInCurrentOwnedPlaylist = $derived.by(() => {
    const playlistId = youtubeMusicStatus?.currentPlaylistId;
    const trackId = youtubeMusicStatus?.currentItemId;
    if (!playlistId || !trackId) return false;
    return (
      youtubeMusicLibrary?.playlists.some(
        (playlist) => playlist.id === playlistId && playlist.trackIds.includes(trackId)
      ) ?? false
    );
  });
  const effectiveSpotifyPlaybackState = $derived(
    optimisticSpotifyPlaying !== null
      ? optimisticSpotifyPlaying
        ? "playing"
        : "paused"
      : (spotifyStatus?.playbackState ?? "stopped")
  );
  const effectiveOsMediaPlaybackState = $derived(
    optimisticMediaPlaying !== null
      ? optimisticMediaPlaying
        ? "playing"
        : "paused"
      : osLocalNowPlaying?.isPlaying
        ? "playing"
        : "paused"
  );
  let homePlaybackScene = $state<"spotify" | "youtubeMusic" | "media" | null>(null);
  const homeNowPlaying = $derived.by(() => {
    const spotifyTitle = spotifyStatus?.currentTrackName?.trim() ?? "";
    const spotifyArtist = spotifyStatus?.currentArtistName?.trim() ?? "";
    const youtubeTitle = youtubeMusicStatus?.currentTrackName?.trim() ?? "";
    const youtubeArtist = youtubeMusicStatus?.currentArtistName?.trim() ?? "";
    const mediaTitle = osLocalNowPlaying?.title?.trim() ?? "";
    const mediaArtist = osLocalNowPlaying?.artist?.trim() ?? "";
    const spotify =
      spotifyTitle || spotifyArtist
        ? {
            sceneId: "spotify" as const,
            title: spotifyTitle,
            artist: spotifyArtist,
            coverUrl: spotifyStatus?.currentCoverArtUrl ?? null,
            playing: effectiveSpotifyPlaybackState === "playing",
          }
        : null;
    const youtube =
      youtubeTitle || youtubeArtist
        ? {
            sceneId: "youtubeMusic" as const,
            title: youtubeTitle,
            artist: youtubeArtist,
            coverUrl: youtubeMusicStatus?.currentCoverArtUrl ?? null,
            playing: !!youtubeMusicStatus?.isPlaying,
          }
        : null;
    const media =
      mediaTitle || mediaArtist
        ? {
            sceneId: "media" as const,
            title: mediaTitle,
            artist: mediaArtist,
            coverUrl: osLocalNowPlaying?.coverArtUrl ?? null,
            playing: effectiveOsMediaPlaybackState === "playing",
          }
        : null;
    return spotify?.playing
      ? spotify
      : youtube?.playing
        ? youtube
        : media?.playing
          ? media
          : homePlaybackScene === "spotify"
            ? spotify
            : homePlaybackScene === "youtubeMusic"
              ? youtube
              : homePlaybackScene === "media"
                ? media
                : (spotify ?? youtube ?? media);
  });

  $effect(() => {
    if (effectiveSpotifyPlaybackState === "playing") homePlaybackScene = "spotify";
    else if (youtubeMusicStatus?.isPlaying) homePlaybackScene = "youtubeMusic";
    else if (effectiveOsMediaPlaybackState === "playing") homePlaybackScene = "media";
  });

  const currentMediaView = $derived.by(() => {
    const media = currentPlugin?.view?.type === "mediaPlayer"
      ? currentPlugin.view.mediaPlayer
      : sceneId === "media"
        ? {
            previous: { label: "Previous", emoji: "⏮️", action: "media.prevTrack" },
            playPause: { label: "Play", emoji: "▶️", action: "media.togglePlay" },
            next: { label: "Next", emoji: "⏭️", action: "media.nextTrack" },
            volumeAction: "media.setVolume",
            seekAction: "media.seek",
          }
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

    const withPlayState = (button?: DeckButtonConfig | null) => {
      if (!button) return null;
      if (button.action === "spotify.togglePlay") {
        const playing =
          optimisticSpotifyPlaying ?? spotifyStatus?.isPlaying ?? false;
        return {
          ...button,
          label: playing ? "Pause" : "Play",
          emoji: playing ? "⏸️" : "▶️",
        };
      }
      if (button.action === "media.togglePlay") {
        const playing =
          optimisticMediaPlaying ?? osLocalNowPlaying?.isPlaying ?? false;
        return {
          ...button,
          label: playing ? "Pause" : "Play",
          emoji: playing ? "⏸️" : "▶️",
        };
      }
      return button;
    };

    return {
      previous: media.previous ?? null,
      playPause: withPlayState(media.playPause),
      next: media.next ?? null,
      like: withSpotifyLikeState(media.like),
      shuffle: media.shuffle ?? null,
      volumeAction: media.volumeAction ?? null,
      seekAction: media.seekAction ?? null,
    };
  });
  const isImmersiveDeckView = $derived.by(
    () =>
      isTauri &&
      !isSettingsWindow &&
      ($weatherDetailOpen ||
        $performanceOpen ||
        activeCustomViewId !== null ||
        currentMediaView !== null ||
        sceneId === "teams" ||
        sceneId === "vscode" ||
        sceneId === "clock")
  );
  const displayedLayout = $derived.by(() => {
    if (!layout) return null;
    return layout;
  });

  $effect(() => {
    if (!isTauri || loading || windowLabel !== "main") return;
    const mode = viewMode;
    const id = sceneId;
    if (mode === "deck" && isIdleScene(id) && !activeCustomViewId) {
      void syncWindowToPresence(id);
    }
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
        spotifyAuthMode = c.authMode ?? "official";
        spotifyClientIdDraft = c.clientId;
        spotifyClientLockedByEnv = c.lockedByEnv;
      } catch {
        // ignore
      }
    })();
  });

  $effect(() => {
    if (!isTauri || viewMode !== "settings") return;
    void hydrateSceneBackground();
  });

  $effect(() => {
    if (!isTauri || viewMode !== "settings") return;
    void (async () => {
      try {
        const [boot, minimized, fullscreen, terminal, hideTaskbar] = await Promise.all([
          isAutostartEnabled(),
          getStartMinimized(),
          getStartFullscreen(),
          getShowSettingsTerminal(),
          getHideTaskbarIcon(),
        ]);
        startOnBoot = boot;
        startMinimized = minimized;
        startFullscreen = fullscreen;
        showSettingsTerminal = terminal;
        hideTaskbarIcon = hideTaskbar;
        startOnBootError = null;
      } catch (e) {
        startOnBootError = String(e);
      }
    })();
  });

  async function openExternalUrl(url: string, label = "External link") {
    try {
      if (isTauri) {
        await openUrl(url);
      } else {
        window.open(url, "_blank", "noopener,noreferrer");
      }
    } catch (e) {
      logError(`Failed to open ${label}: ${String(e)}`, "Spotify");
    }
  }

  function openSpotifyDeveloperDashboard() {
    void openExternalUrl(SPOTIFY_DEV_DASHBOARD, "Spotify Developer Dashboard");
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

  async function persistFullscreenPreference(enabled: boolean) {
    startFullscreen = enabled;
    try {
      await setStartFullscreen(enabled);
    } catch (e) {
      logError(`Failed to save fullscreen setting: ${String(e)}`, "Settings");
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
    await persistFullscreenPreference(deckPresentationFullscreen);
    await persistMainWindowState();
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
    await persistFullscreenPreference(deckPresentationFullscreen);
    await persistMainWindowState();
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

  async function onStartOnBootChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const checked = input.checked;
    startOnBootBusy = true;
    startOnBootError = null;
    try {
      await setAutostartEnabled(checked);
      startOnBoot = checked;
      logInfo(
        checked ? "Enabled start on computer sign-in" : "Disabled start on computer sign-in",
        "Settings"
      );
    } catch (e) {
      startOnBoot = !checked;
      input.checked = !checked;
      startOnBootError = String(e);
      logError(`Failed to update start on boot: ${String(e)}`, "Settings");
    } finally {
      startOnBootBusy = false;
    }
  }

  async function onStartMinimizedChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const checked = input.checked;
    startMinimizedBusy = true;
    startOnBootError = null;
    try {
      await setStartMinimized(checked);
      startMinimized = checked;
      logInfo(
        checked
          ? "Stay in the tray until you open AstroDeck yourself"
          : "AstroDeck will open when VS Code, Cursor, Teams, Spotify, or a local media player is running",
        "Settings"
      );
      void syncWindowToPresence();
    } catch (e) {
      startMinimized = !checked;
      input.checked = !checked;
      startOnBootError = String(e);
      logError(`Failed to update start minimized: ${String(e)}`, "Settings");
    } finally {
      startMinimizedBusy = false;
    }
  }

  async function onHideTaskbarIconChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const checked = input.checked;
    hideTaskbarIconBusy = true;
    startOnBootError = null;
    try {
      await setHideTaskbarIcon(checked);
      hideTaskbarIcon = checked;
      logInfo(
        checked
          ? "AstroDeck stays off the Windows taskbar while the window is open"
          : "AstroDeck shows a Windows taskbar button while the window is open",
        "Settings"
      );
    } catch (e) {
      hideTaskbarIcon = !checked;
      input.checked = !checked;
      startOnBootError = String(e);
      logError(`Failed to update taskbar icon: ${String(e)}`, "Settings");
    } finally {
      hideTaskbarIconBusy = false;
    }
  }

  async function onShowSettingsTerminalChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const checked = input.checked;
    showSettingsTerminalBusy = true;
    startOnBootError = null;
    try {
      await setShowSettingsTerminal(checked);
      showSettingsTerminal = checked;
      logInfo(
        checked ? "Enabled Settings terminal" : "Disabled Settings terminal",
        "Settings"
      );
    } catch (e) {
      showSettingsTerminal = !checked;
      input.checked = !checked;
      startOnBootError = String(e);
      logError(`Failed to update Settings terminal: ${String(e)}`, "Settings");
    } finally {
      showSettingsTerminalBusy = false;
    }
  }

  async function onStartFullscreenChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const checked = input.checked;
    startFullscreenBusy = true;
    startOnBootError = null;
    try {
      if (checked) {
        await enterDeckPresentationFullscreen();
      } else {
        await exitDeckPresentationFullscreen();
      }
      logInfo(
        checked ? "Enabled fullscreen mode" : "Disabled fullscreen mode",
        "Settings"
      );
    } catch (e) {
      startFullscreen = !checked;
      input.checked = !checked;
      startOnBootError = String(e);
      logError(`Failed to update fullscreen: ${String(e)}`, "Settings");
    } finally {
      startFullscreenBusy = false;
    }
  }

  async function saveSpotifyClientId() {
    if (!isTauri || spotifyClientLockedByEnv) return;
    spotifySavingClientId = true;
    spotifyAuthHint = null;
    try {
      await setSpotifyClientId(spotifyClientIdDraft.trim());
      spotifyAuthMode = "custom";
      await refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      spotifyAuthHint = spotifyClientIdDraft.trim()
        ? "Client ID saved. Connect Spotify to use your developer app."
        : "Cleared saved Client ID.";
    } catch (e) {
      spotifyAuthHint = String(e);
    } finally {
      spotifySavingClientId = false;
    }
  }

  async function saveSpotifyAuthMode(mode: SpotifyAuthMode) {
    if (spotifyClientLockedByEnv || spotifySavingAuthMode) return;
    if (mode === spotifyAuthMode) return;
    if (!isTauri) {
      spotifyAuthMode = mode;
      return;
    }
    spotifySavingAuthMode = true;
    spotifyAuthHint = null;
    try {
      await setSpotifyAuthMode(mode);
      spotifyAuthMode = mode;
      await refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      spotifyAuthHint =
        mode === "official"
          ? "Using Spotify desktop login. Connect to sign in — no developer app needed."
          : "Using your Spotify developer app. Save a Client ID, then Connect.";
    } catch (e) {
      spotifyAuthHint = String(e);
    } finally {
      spotifySavingAuthMode = false;
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
    document.title = `AstroDeck - Debug panel ${icon} ${text}`;
  }

  function loadSeenScenes() {
    if (typeof window === "undefined") return;
    try {
      const raw = window.localStorage.getItem("astrodeck:seenScenes");
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
      window.localStorage.setItem("astrodeck:seenScenes", JSON.stringify(seenScenes));
    } catch {
      // ignore storage errors
    }
  }

  function markSceneSeen(id: string) {
    if (!id || isIdleScene(id)) return;
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
        await invoke<string>("start_spotify_auth");
        logInfo("Opened Spotify authorization page in the default browser", "Spotify");
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

  function clearSpotifyTransportRefreshTimers() {
    for (const timer of spotifyTransportRefreshTimers) {
      clearTimeout(timer);
    }
    spotifyTransportRefreshTimers = [];
  }

  function scheduleSpotifyTransportRefresh() {
    clearSpotifyTransportRefreshTimers();
    const refresh = () => {
      if (isTauri) {
        void refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      } else {
        requestSpotifyStatus();
      }
    };
    refresh();
    for (const delayMs of [1500, 4000]) {
      spotifyTransportRefreshTimers.push(setTimeout(refresh, delayMs));
    }
  }

  function scheduleSpotifyQueueRefresh() {
    if (spotifyQueueRefreshTimer) clearTimeout(spotifyQueueRefreshTimer);
    const refresh = () => {
      if (isTauri) {
        void refreshSpotifyStatusForDesktop({ immediate: true });
      } else {
        requestSpotifyStatus();
      }
    };
    refresh();
    spotifyQueueRefreshTimer = setTimeout(() => {
      spotifyQueueRefreshTimer = null;
      refresh();
    }, 1500);
  }

  async function waitForSpotifyQueueRefresh(maxMs = 2500) {
    const start = Date.now();
    while (spotifyQueueRefreshPending && Date.now() - start < maxMs) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  function normalizeMediaText(value?: string | null) {
    return (value ?? "").trim().toLowerCase();
  }

  function isSpotifyOsSource(source: string) {
    return source.toLowerCase().includes("spotify");
  }

  function isLocalOsPayload(payload: OsNowPlaying) {
    if (payload.kind === "local") return true;
    if (payload.kind === "spotify") return false;
    return !isSpotifyOsSource(payload.source);
  }

  function friendlyOsSource(source?: string | null) {
    const raw = (source ?? "").trim();
    if (!raw) return "System media";
    const noExe = raw.replace(/\.exe$/i, "");
    const token = noExe.split(/[\\/!.]/).filter(Boolean).pop() ?? noExe;
    return token.replace(/([a-z])([A-Z])/g, "$1 $2");
  }

  const PROGRESS_SYNC_SLACK_MS = 4000;
  const PROGRESS_SEEK_BACK_MS = 5000;

  function osProgressJumped(
    fromMs?: number | null,
    toMs?: number | null,
    fromAtMs = Date.now(),
    wasPlaying = false
  ) {
    if (toMs == null || !Number.isFinite(toMs)) return false;
    const from = fromMs ?? 0;
    if (from >= 2500 && toMs <= 1500) return true;
    const elapsed = wasPlaying ? Math.max(0, Date.now() - fromAtMs) : 0;
    const expected = from + elapsed;
    const delta = toMs - expected;
    if (wasPlaying && delta < 0 && delta > -PROGRESS_SEEK_BACK_MS) return false;
    return Math.abs(delta) >= PROGRESS_SYNC_SLACK_MS;
  }

  function sameOsTrack(previous: OsNowPlaying | null, next: OsNowPlaying) {
    if (!previous) return false;
    return (
      previous.source === next.source &&
      (previous.title ?? "") === (next.title ?? "") &&
      (previous.artist ?? "") === (next.artist ?? "") &&
      (previous.album ?? "") === (next.album ?? "")
    );
  }

  function osTrackKey(payload: OsNowPlaying | null | undefined) {
    if (!payload) return "";
    return `${payload.source}\0${payload.title ?? ""}\0${payload.artist ?? ""}\0${payload.album ?? ""}`;
  }

  function isSpotifySkipPinned() {
    return pinnedSpotifyItemId != null && Date.now() < pinnedSpotifyUntil;
  }

  function beginLocalSkipHold() {
    if (!osLocalNowPlaying) {
      localSkipHold = null;
      return;
    }
    localSkipHold = {
      fromKey: osTrackKey(osLocalNowPlaying),
      until: Date.now() + 4000,
    };
  }

  function applyOsNowPlaying(payload: OsNowPlaying) {
    if (isLocalOsPayload(payload)) {
      if (payload.active === false) {
        osLocalNowPlaying = null;
        optimisticMediaPlaying = null;
        localSkipHold = null;
        return;
      }
      if (localSkipHold && Date.now() > localSkipHold.until) {
        localSkipHold = null;
      }
      if (localSkipHold && osTrackKey(payload) === localSkipHold.fromKey) {
        if (
          optimisticMediaPlaying !== null &&
          payload.isPlaying === optimisticMediaPlaying
        ) {
          optimisticMediaPlaying = null;
        }
        return;
      }
      if (localSkipHold) {
        localSkipHold = null;
      }
      const previous = osLocalNowPlaying;
      const keepProgress =
        sameOsTrack(previous, payload) &&
        !osProgressJumped(
          previous?.progressMs,
          payload.progressMs,
          localProgressAt,
          !!previous?.isPlaying
        );
      osLocalNowPlaying = {
        ...payload,
        progressMs: keepProgress
          ? (previous?.progressMs ?? payload.progressMs)
          : payload.progressMs,
        coverArtUrl: payload.coverArtUrl || previous?.coverArtUrl || null,
      };
      if (!keepProgress) localProgressAt = Date.now();
      if (
        optimisticMediaPlaying !== null &&
        payload.isPlaying === optimisticMediaPlaying
      ) {
        optimisticMediaPlaying = null;
      }
      return;
    }

    if (!isSpotifyOsSource(payload.source) || !spotifyStatus?.isAuthenticated) return;

    const nextTitle = payload.title?.trim() || null;
    const titleChanged =
      nextTitle != null &&
      normalizeMediaText(nextTitle) !==
        normalizeMediaText(spotifyStatus.currentTrackName);

    const osCover = payload.coverArtUrl?.trim() || null;
    const osProgress =
      payload.progressMs != null && Number.isFinite(payload.progressMs)
        ? Math.max(0, Math.round(payload.progressMs))
        : null;
    const osDuration =
      payload.durationMs != null && payload.durationMs > 0
        ? Math.round(payload.durationMs)
        : null;
    const progressJumped = osProgressJumped(
      spotifyStatus.progressMs,
      osProgress,
      spotifyProgressAt,
      !!spotifyStatus.isPlaying
    );

    if (titleChanged) {
      if (isSpotifySkipPinned()) {
        scheduleSpotifyTransportRefresh();
        return;
      }
      optimisticSpotifySaved = null;
      osNowPlayingHold = {
        previousItemId: spotifyStatus.currentItemId ?? null,
      };
      spotifySeekTargetMs = osProgress;
      spotifyProgressAt = Date.now();
      spotifyStatus = {
        ...spotifyStatus,
        currentTrackName: nextTitle,
        currentArtistName: payload.artist?.trim() || spotifyStatus.currentArtistName,
        currentAlbumName: payload.album?.trim() || spotifyStatus.currentAlbumName,
        currentCoverArtUrl: osCover || spotifyStatus.currentCoverArtUrl,
        progressMs: osProgress ?? 0,
        durationMs: osDuration ?? spotifyStatus.durationMs,
        isPlaying: payload.isPlaying,
        playbackState: payload.isPlaying ? "playing" : "paused",
      };
    } else {
      const nextStatus = { ...spotifyStatus };
      let changed = false;
      if (osCover && osCover !== spotifyStatus.currentCoverArtUrl) {
        nextStatus.currentCoverArtUrl = osCover;
        changed = true;
      }
      if (payload.isPlaying !== spotifyStatus.isPlaying) {
        optimisticSpotifyPlaying = null;
        nextStatus.isPlaying = payload.isPlaying;
        nextStatus.playbackState = payload.isPlaying ? "playing" : "paused";
        changed = true;
      }
      if (progressJumped && osProgress != null) {
        spotifySeekTargetMs = osProgress;
        nextStatus.progressMs = osProgress;
        if (osDuration != null) nextStatus.durationMs = osDuration;
        spotifyProgressAt = Date.now();
        changed = true;
      }
      if (changed) {
        spotifyStatus = nextStatus;
      }
    }

    scheduleSpotifyTransportRefresh();
  }

  function pinSpotifyItem(itemId: string, fromItemId?: string | null) {
    pinnedSpotifyItemId = itemId;
    pinnedSpotifyFromId = fromItemId ?? null;
    pinnedSpotifyUntil = Date.now() + 5000;
  }

  function clearPinnedSpotifyItem() {
    pinnedSpotifyItemId = null;
    pinnedSpotifyFromId = null;
    pinnedSpotifyUntil = 0;
  }

  function applySpotifyTrackPreview(preview: SpotifyTrackPreview) {
    optimisticSpotifySaved = null;
    spotifySeekTargetMs = null;
    const fromId =
      spotifyStatus?.currentItemId && spotifyStatus.currentItemId !== preview.itemId
        ? spotifyStatus.currentItemId
        : pinnedSpotifyFromId;
    pinSpotifyItem(preview.itemId, fromId);
    if (!spotifyStatus) return;
    spotifyProgressAt = Date.now();
    spotifyStatus = {
      ...spotifyStatus,
      currentItemId: preview.itemId,
      currentItemType: preview.itemType,
      currentTrackName: preview.trackName,
      currentArtistName: preview.artistName ?? null,
      currentAlbumName: preview.albumName ?? null,
      currentCoverArtUrl: preview.coverArtUrl ?? null,
      durationMs: preview.durationMs ?? null,
      progressMs: 0,
      isCurrentTrackSaved: null,
    };
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
    const pinActive =
      pinnedSpotifyItemId != null && Date.now() < pinnedSpotifyUntil;
    if (pinActive && next.currentItemId === pinnedSpotifyItemId) {
      clearPinnedSpotifyItem();
    } else if (pinActive && next.currentItemId && next.currentItemId === pinnedSpotifyFromId) {
      if (spotifyStatus) {
        next = {
          ...next,
          currentItemId: spotifyStatus.currentItemId,
          currentItemType: spotifyStatus.currentItemType,
          currentTrackName: spotifyStatus.currentTrackName,
          currentArtistName: spotifyStatus.currentArtistName,
          currentAlbumName: spotifyStatus.currentAlbumName,
          currentCoverArtUrl: spotifyStatus.currentCoverArtUrl,
          durationMs: spotifyStatus.durationMs,
          progressMs: spotifyStatus.progressMs,
          isCurrentTrackSaved: spotifyStatus.isCurrentTrackSaved,
        };
      }
    } else if (pinActive && next.currentItemId && next.currentItemId !== pinnedSpotifyItemId) {
      clearPinnedSpotifyItem();
    } else if (pinActive && spotifyStatus) {
      next = {
        ...next,
        currentItemId: spotifyStatus.currentItemId,
        currentItemType: spotifyStatus.currentItemType,
        currentTrackName: spotifyStatus.currentTrackName,
        currentArtistName: spotifyStatus.currentArtistName,
        currentAlbumName: spotifyStatus.currentAlbumName,
        currentCoverArtUrl: spotifyStatus.currentCoverArtUrl,
        durationMs: spotifyStatus.durationMs,
        progressMs: spotifyStatus.progressMs,
        isCurrentTrackSaved: spotifyStatus.isCurrentTrackSaved,
      };
    } else if (!pinActive) {
      clearPinnedSpotifyItem();
    }

    if (osNowPlayingHold && spotifyStatus) {
      const apiId = next.currentItemId ?? null;
      const previousId = osNowPlayingHold.previousItemId;
      const apiMovedOn =
        Boolean(apiId && previousId && apiId !== previousId) ||
        Boolean(apiId && !previousId);
      if (apiMovedOn) {
        osNowPlayingHold = null;
      } else {
        next = {
          ...next,
          currentItemId: spotifyStatus.currentItemId,
          currentItemType: spotifyStatus.currentItemType,
          currentTrackName: spotifyStatus.currentTrackName,
          currentArtistName: spotifyStatus.currentArtistName,
          currentAlbumName: spotifyStatus.currentAlbumName,
          currentCoverArtUrl: spotifyStatus.currentCoverArtUrl,
          durationMs: spotifyStatus.durationMs,
          progressMs: spotifyStatus.progressMs,
          isCurrentTrackSaved: spotifyStatus.isCurrentTrackSaved,
        };
      }
    }

    const sameTrack =
      next.currentItemId != null &&
      next.currentItemId === spotifyStatus?.currentItemId;

    if (!sameTrack) {
      optimisticSpotifySaved = null;
      optimisticSpotifyPlaying = null;
      spotifyVolumeTarget = null;
      spotifySeekTargetMs = null;
      spotifyProgressAt = Date.now();
    }

    let merged: SpotifyStatus = { ...next };

    if (sameTrack && spotifyStatus && merged.progressMs != null) {
      if (
        !osProgressJumped(
          spotifyStatus.progressMs,
          merged.progressMs,
          spotifyProgressAt,
          !!spotifyStatus.isPlaying
        )
      ) {
        merged.progressMs = spotifyStatus.progressMs;
      } else {
        spotifyProgressAt = Date.now();
      }
    }

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

  async function refreshOsVolume() {
    if (!isTauri) return;
    try {
      const next = await getOutputVolume();
      if (osVolumeBusy) return;
      if (osVolumeTarget != null) {
        if (Math.abs(next - osVolumeTarget) <= 2) {
          osVolumeTarget = null;
        } else {
          return;
        }
      }
      osVolumePercent = next;
    } catch {
      // System volume is optional; keep the last known fader value.
    }
  }

  async function hydrateOsLocalMedia() {
    if (!isTauri) return;
    try {
      osLocalNowPlaying = await getOsNowPlaying();
    } catch {
      osLocalNowPlaying = null;
    }
    await refreshOsVolume();
  }

  async function commitSceneVolume(action: string, value: number) {
    const useOsVolume = action.startsWith("media.");
    const useYouTubeVolume = action.startsWith("youtubeMusic.");
    if (useOsVolume) {
      osVolumePercent = value;
      osVolumeTarget = value;
      try {
        osVolumeBusy = true;
        await executeActionValue(action, value);
      } catch (e) {
        logError(`Failed to set volume via ${action}: ${String(e)}`, `${sceneId} window`);
        osVolumeTarget = null;
      } finally {
        osVolumeBusy = false;
      }
      return;
    }
    if (useYouTubeVolume) {
      try {
        await executeActionValue(action, value);
      } catch (e) {
        logError(`Failed to set volume via ${action}: ${String(e)}`, `${sceneId} window`);
      }
      return;
    }
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
    if (action.startsWith("media.")) {
      if (osLocalNowPlaying) {
        osLocalNowPlaying = { ...osLocalNowPlaying, progressMs: positionMs };
        localProgressAt = Date.now();
      }
      void (async () => {
        try {
          await executeActionValue(action, positionMs);
        } catch (e) {
          logError(`Failed to seek via ${action}: ${String(e)}`, `${sceneId} window`);
        }
      })();
      return;
    }
    if (action.startsWith("youtubeMusic.")) {
      void executeActionValue(action, positionMs).catch((e) => {
        logError(`Failed to seek via ${action}: ${String(e)}`, `${sceneId} window`);
      });
      return;
    }
    spotifySeekTargetMs = positionMs;
    if (spotifyStatus) {
      spotifyStatus = { ...spotifyStatus, progressMs: positionMs };
      spotifyProgressAt = Date.now();
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
    if (!isTauri || sceneId !== "media") return;
    void refreshOsVolume();
    const id = window.setInterval(() => {
      void refreshOsVolume();
    }, 2000);
    return () => window.clearInterval(id);
  });

  $effect(() => {
    if (!isTauri || sceneId !== "spotify") return;
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
      if (icon) {
        try {
          await win.setIcon(icon);
        } catch (error) {
          logError(`Failed to set window icon: ${String(error)}`, "Tray");
        }
      }
      const initiallyVisible = await win.isVisible();

      trayMenu = await Menu.new({
        items: [
          {
            id: "toggle",
            text: initiallyVisible ? "Hide AstroDeck" : "Show AstroDeck",
            action: async () => {
              const visible = await win.isVisible();
              if (visible) {
                await persistMainWindowState();
                await win.hide();
                logInfo("Hid AstroDeck window from tray menu", "Tray");
                const item = await trayMenu?.get("toggle");
                if (item) await item.setText("Show AstroDeck");
              } else {
                await showAstroDeckFromTray();
              }
            },
          },
          {
            id: "settings",
            text: "Settings",
            action: async () => {
              logInfo("Settings requested from tray menu", "Tray");
              viewMode = "settings";
              await revealMainWindow();
              const item = await trayMenu?.get("toggle");
              if (item) await item.setText("Hide AstroDeck");
            },
          },
          { item: "Separator" },
          {
            id: "quit",
            text: "Quit AstroDeck",
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
              await showAstroDeckFromTray();
              logInfo("Tray icon clicked: showing window", "Tray");
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

  let sceneId = $state("clock");
  let returnHomeFromWeather = $state(false);
  const sideNavActive = $derived(
    viewMode === "settings"
      ? "settings"
      : activeCustomViewId
        ? activeCustomViewId
        : $performanceOpen
        ? "performance"
        : $weatherDetailOpen
          ? "weather"
          : sceneId
  );
  const activeCustomView = $derived(
    customViews.find((view) => view.id === activeCustomViewId) ?? null
  );
  const keptWebViews = $derived(
    customViews.filter((view) => keptWebViewIds.includes(view.id))
  );
  const activeWebViewId = $derived(
    viewMode === "deck" && !$performanceOpen && !$weatherDetailOpen ? activeCustomViewId : null
  );

  function closeWeatherToHome() {
    viewMode = "deck";
    performanceOpen.set(false);
    if (sceneId === "clock") {
      weatherDetailOpen.set(false);
      return;
    }
    returnHomeFromWeather = true;
    selectScene("clock");
  }

  function appEnabled(id: string) {
    return !disabledApps.includes(id);
  }

  function ensureShownAppEnabled() {
    if (viewMode === "settings") return;
    if (activeCustomViewId && !appEnabled(activeCustomViewId)) activeCustomViewId = null;
    if ($weatherDetailOpen && !appEnabled("weather")) weatherDetailOpen.set(false);
    if ($performanceOpen && !appEnabled("performance")) performanceOpen.set(false);
    const shown = activeCustomViewId ?? ($weatherDetailOpen ? "weather" : sceneId);
    if (appEnabled(shown)) return;
    if (shown !== "clock" && appEnabled("clock")) {
      activeCustomViewId = null;
      weatherDetailOpen.set(false);
      performanceOpen.set(false);
      sceneId = "clock";
      selectScene("clock");
      return;
    }
    viewMode = "settings";
  }

  function openCustomView(id: string) {
    if (!customViews.some((view) => view.id === id) || !appEnabled(id)) return;
    weatherDetailOpen.set(false);
    performanceOpen.set(false);
    viewMode = "deck";
    if (!keptWebViewIds.includes(id)) keptWebViewIds = [...keptWebViewIds, id];
    activeCustomViewId = id;
  }

  async function saveCustomViews(next: CustomView[]) {
    customViewsBusy = true;
    try {
      customViews = await setCustomViews(next);
      keptWebViewIds = keptWebViewIds.filter((id) => next.some((view) => view.id === id));
      if (activeCustomViewId && !customViews.some((view) => view.id === activeCustomViewId)) {
        activeCustomViewId = null;
      }
    } finally {
      customViewsBusy = false;
    }
  }

  async function onSetAppEnabled(id: string, enabled: boolean) {
    appToggleBusyId = id;
    try {
      disabledApps = await setAppEnabled(id, enabled);
      if (!enabled) ensureShownAppEnabled();
    } finally {
      appToggleBusyId = null;
    }
  }

  function onSideNavSelect(id: string) {
    if (id === "settings") {
      weatherDetailOpen.set(false);
      performanceOpen.set(false);
      viewMode = "settings";
      return;
    }
    if (id.startsWith("view-")) {
      openCustomView(id);
      return;
    }
    activeCustomViewId = null;
    if (id === "performance") {
      weatherDetailOpen.set(false);
      performanceOpen.set(true);
      viewMode = "deck";
      return;
    }
    if (id === "weather") {
      performanceOpen.set(false);
      weatherDetailOpen.set(true);
      viewMode = "deck";
      return;
    }
    if (id === "clock") {
      closeWeatherToHome();
      return;
    }
    weatherDetailOpen.set(false);
    performanceOpen.set(false);
    selectScene(id);
  }

  $effect(() => {
    if (!returnHomeFromWeather || sceneId !== "clock") return;
    returnHomeFromWeather = false;
    weatherDetailOpen.set(false);
  });
  let layout = $state<LayoutConfig | null>(null);
  let availableScenes = $state<string[]>([]);
  let loading = $state(true);
  const isHomeScene = $derived(sceneId === "clock");
  let homeHue = $state(Math.floor(Math.random() * 360));
  const homePalette = $derived(
    $clockBgColorCustom
      ? shaderColorsFromHue(hexToHsv($clockBgColor).h)
      : isNeutralAccent($clockDigitColor)
        ? shaderColorsFromHue(homeHue, true)
        : shaderColorsFromAccent($clockDigitColor)
  );
  const globalSceneColors = $derived(
    !isHomeScene || homeNowPlaying?.playing ? $sceneShaderColors : homePalette
  );

  $effect(() => {
    if (!isHomeScene || !homeNowPlaying?.playing) return;
    const url = homeNowPlaying.coverUrl;
    if (!url) return;
    let cancelled = false;
    void extractAlbumPalette(url).then((palette) => {
      if (cancelled || !palette?.[0]) return;
      sceneShaderColors.set(paletteToShaderColors(palette));
      sceneShaderImage.set(url);
    });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (!isHomeScene || homeNowPlaying?.playing || $clockBgColorCustom || !isNeutralAccent($clockDigitColor)) return;
    const timer = window.setInterval(() => {
      const jump = 50 + Math.floor(Math.random() * 160);
      homeHue = (homeHue + jump) % 360;
    }, 18000);
    return () => window.clearInterval(timer);
  });
  const isSpotifyScene = $derived(sceneId === "spotify");
  const isYouTubeMusicScene = $derived(sceneId === "youtubeMusic");
  const isOsMediaScene = $derived(sceneId === "media");
  const isYouTubeMusicActive = $derived(
    isYouTubeMusicScene && youtubeMusicStatus?.currentItemId != null
  );
  const mediaPlayerTitle = $derived.by(() => {
    if (isSpotifyScene) {
      return (
        spotifyStatus?.currentTrackName ??
        (spotifyStatus?.isConfigured === false
          ? "Spotify setup required"
          : spotifyStatus?.isAuthenticated === false
            ? "Connect Spotify"
            : "Nothing playing")
      );
    }
    if (isYouTubeMusicScene) {
      return youtubeMusicStatus?.currentTrackName ?? "Search YouTube Music";
    }
    if (isOsMediaScene) {
      return osLocalNowPlaying?.title?.trim() || "Nothing playing";
    }
    return null;
  });
  const mediaPlayerSubtitle = $derived.by(() => {
    if (isSpotifyScene) {
      return (
        spotifyStatus?.currentArtistName ??
        (spotifyStatus?.message ??
          "Tray → Settings: Connect Spotify")
      );
    }
    if (isYouTubeMusicScene) {
      return youtubeMusicStatus?.currentArtistName ?? "Guest playback";
    }
    if (isOsMediaScene) {
      return (
        osLocalNowPlaying?.artist?.trim() ||
        friendlyOsSource(osLocalNowPlaying?.source)
      );
    }
    return null;
  });
  const settingsSceneIds = $derived(
    Array.from(
      new Set([
        ...getBuiltinSceneIds(),
        ...plugins.map((plugin) => plugin.id),
        ...availableScenes,
        ...seenScenes,
      ])
    ).filter((id) => !isIdleScene(id))
  );

  async function setTrayToggleVisible(visible: boolean) {
    const item = await trayMenu?.get("toggle");
    if (item) await item.setText(visible ? "Hide AstroDeck" : "Show AstroDeck");
  }

  async function showAstroDeckFromTray() {
    viewMode = "deck";
    await revealMainWindow();
    await setTrayToggleVisible(true);
    logInfo(`Showed AstroDeck window from tray (${sceneId})`, "Tray");
  }

  async function syncWindowToPresence(nextSceneId = sceneId) {
    if (!isTauri || windowLabel !== "main") return;
    const idle = isIdleScene(nextSceneId);
    const win = getCurrentWindow();
    const visible = await win.isVisible();
    if (idle) {
      if (viewMode === "settings") return;
      if (visible) {
        await hideMainWindow();
        await setTrayToggleVisible(false);
        logInfo("Hid AstroDeck; no supported app or local media is running", "Scenes");
      }
      return;
    }
    if (startMinimized && !visible) return;
    if (!visible) {
      viewMode = "deck";
      await revealMainWindow();
      await setTrayToggleVisible(true);
      logInfo(`Opened AstroDeck for ${nextSceneId}`, "Scenes");
    }
  }

  async function checkForUpdates(showPopup = false) {
    if (!isTauri) return null;
    appUpdateChecking = true;
    appUpdateError = null;
    try {
      const update = await checkForAppUpdate();
      appUpdate = update;
      if (update.available && update.latestVersion) {
        logInfo(
          `Update available: ${update.latestVersion} (current ${update.currentVersion})`,
          "Updater"
        );
        if (showPopup && showUpdatePopups) {
          updatePopupOpen = true;
          await revealMainWindow();
          const item = await trayMenu?.get("toggle");
          if (item) await item.setText("Hide AstroDeck");
        }
      }
      return update;
    } catch (e) {
      appUpdateError = String(e);
      logError(`Update check failed: ${String(e)}`, "Updater");
      return null;
    } finally {
      appUpdateChecking = false;
    }
  }

  async function installAppUpdate() {
    if (!appUpdate?.available || !appUpdate.downloadUrl || !appUpdate.latestVersion) return;
    if (appUpdateBusy) return;

    appUpdateBusy = true;
    appUpdateError = null;
    try {
      logInfo(`Downloading update from ${appUpdate.installerName ?? "release asset"}`, "Updater");
      updatePopupOpen = false;
      await downloadAndInstallUpdate(appUpdate.downloadUrl);
    } catch (e) {
      appUpdateBusy = false;
      updatePopupOpen = true;
      appUpdateError = String(e);
      logError(`Update install failed: ${String(e)}`, "Updater");
    }
  }

  function openUpdatePopup() {
    if (!installableUpdate) return;
    updatePopupOpen = true;
  }

  async function onShowUpdatePopupsChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const checked = input.checked;
    showUpdatePopupsBusy = true;
    appUpdateError = null;
    try {
      await setUpdatePopupsEnabled(checked);
      showUpdatePopups = checked;
    } catch (e) {
      showUpdatePopups = !checked;
      input.checked = !checked;
      appUpdateError = String(e);
      logError(`Failed to update popup setting: ${String(e)}`, "Settings");
    } finally {
      showUpdatePopupsBusy = false;
    }
  }

  function selectScene(id: string) {
    if (!id || id.startsWith("view-") || !appEnabled(id)) return;
    activeCustomViewId = null;
    markSceneSeen(id);
    if (isTauri) {
      viewMode = "deck";
      setActiveScene(id).catch((e) => {
        logError(`Failed to set active scene to ${id}: ${String(e)}`, "Settings window");
      });
    } else {
      if (logBusSocket?.readyState === WebSocket.OPEN) {
        logBusSocket.send(JSON.stringify({ type: "setScene", sceneId: id }));
      } else {
        logError(`Log bus is not connected; cannot set scene to ${id}`, "Settings window");
        const next = getBuiltinLayout(id);
        if (next) {
          sceneId = id;
          layout = next;
        }
      }
    }
    logInfo(`Manually selected scene: ${id}`, "Settings window");
  }

  async function onAutoSwitchChange(sceneIdToToggle: string, enabled: boolean) {
    autoSwitchBusyId = sceneIdToToggle;
    try {
      autoSwitchScenes = await setAutoSwitchScene(sceneIdToToggle, enabled);
      logInfo(
        enabled
          ? `Auto-switch on for ${sceneIdToToggle}`
          : `Auto-switch off for ${sceneIdToToggle}`,
        "Settings"
      );
    } catch (e) {
      logError(`Failed to update auto-switch for ${sceneIdToToggle}: ${String(e)}`, "Settings");
    } finally {
      autoSwitchBusyId = null;
    }
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
      if (state.activeSceneId === "media") {
        await hydrateOsLocalMedia();
      }
      await syncWindowToPresence(state.activeSceneId);
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
    void hydrateSceneBackground();
    let weatherUpdatesStopped = false;
    let stopWeatherUpdates = () => {
      weatherUpdatesStopped = true;
    };
    void loadClockSettings().finally(() => {
      if (weatherUpdatesStopped) return;
      stopWeatherUpdates = startWeatherUpdates();
    });
    const handleCoreAction = async (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string; label: string }>).detail;
      if (!detail) return;
      const { action, label } = detail;

      if (action === "core.settings") {
        logInfo(`Settings requested from button: ${label}`, "Settings window");
        if (isTauri) {
          weatherDetailOpen.set(false);
          performanceOpen.set(false);
          viewMode = "settings";
          const win = getCurrentWindow();
          if (!(await win.isVisible())) {
            await revealMainWindow();
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
        if (detail.action === "spotify.toggleShuffle") {
          spotifyQueueRefreshPending = false;
          scheduleSpotifyQueueRefresh();
        } else if (detail.action === "spotify.playPlaylist") {
          void refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
        } else if (
          detail.action === "spotify.nextTrack" ||
          detail.action === "spotify.prevTrack"
        ) {
          scheduleSpotifyTransportRefresh();
        } else if (isTauri) {
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
        spotifyQueueRefreshPending = true;
        if (spotifyStatus) {
          spotifyStatus = {
            ...spotifyStatus,
            isShuffle: nextShuffle,
            nextTrackPreview: null,
          };
        }
      }
      if (detail.action === "spotify.togglePlay") {
        const playing =
          optimisticSpotifyPlaying ?? spotifyStatus?.isPlaying ?? false;
        optimisticSpotifyPlaying = !playing;
      }
      if (detail.action === "media.togglePlay") {
        const playing =
          optimisticMediaPlaying ?? osLocalNowPlaying?.isPlaying ?? false;
        optimisticMediaPlaying = !playing;
      }
      if (detail.action === "media.nextTrack" || detail.action === "media.prevTrack") {
        beginLocalSkipHold();
      }
      if (detail.action === "spotify.nextTrack") {
        if (isTauri) {
          void (async () => {
            if (spotifyQueueRefreshPending) {
              await waitForSpotifyQueueRefresh();
            }
            const cachedPreview = spotifyStatus?.nextTrackPreview;
            if (cachedPreview) {
              applySpotifyTrackPreview(cachedPreview);
            }
            try {
              const preview = await peekSpotifySkipTrack("next");
              if (preview) {
                applySpotifyTrackPreview(preview);
              } else if (!cachedPreview && spotifyStatus) {
                spotifyStatus = { ...spotifyStatus, progressMs: 0 };
              }
            } catch {
              if (!cachedPreview && spotifyStatus) {
                spotifyStatus = { ...spotifyStatus, progressMs: 0 };
              }
            }
          })();
        } else {
          const cachedPreview = spotifyStatus?.nextTrackPreview;
          if (cachedPreview) {
            applySpotifyTrackPreview(cachedPreview);
          } else if (spotifyStatus) {
            spotifyStatus = { ...spotifyStatus, progressMs: 0 };
          }
        }
      }
      if (detail.action === "spotify.prevTrack") {
        const progressMs =
          spotifySeekTargetMs ?? spotifyStatus?.progressMs ?? 0;
        const willSkipToPrevious =
          progressMs <= SPOTIFY_PREV_RESTART_THRESHOLD_MS;

        if (willSkipToPrevious) {
          const cachedPreview = spotifyStatus?.prevTrackPreview;
          if (cachedPreview) {
            applySpotifyTrackPreview(cachedPreview);
          }
          if (isTauri) {
            void (async () => {
              try {
                const preview = await peekSpotifySkipTrack("prev");
                if (preview) {
                  applySpotifyTrackPreview(preview);
                } else if (!cachedPreview && spotifyStatus) {
                  spotifyStatus = { ...spotifyStatus, progressMs: 0 };
                }
              } catch {
                if (!cachedPreview && spotifyStatus) {
                  spotifyStatus = { ...spotifyStatus, progressMs: 0 };
                }
              }
            })();
          } else if (!cachedPreview && spotifyStatus) {
            spotifyStatus = { ...spotifyStatus, progressMs: 0 };
          }
        } else {
          spotifySeekTargetMs = 0;
          if (spotifyStatus) {
            spotifyStatus = { ...spotifyStatus, progressMs: 0 };
          }
        }
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
        spotifyQueueRefreshPending = false;
      }
      if (detail.action === "spotify.togglePlay") {
        optimisticSpotifyPlaying = null;
      }
      if (detail.action === "media.togglePlay") {
        optimisticMediaPlaying = null;
      }
      if (detail.action === "media.nextTrack" || detail.action === "media.prevTrack") {
        localSkipHold = null;
      }
      if (
        detail.action === "spotify.nextTrack" ||
        detail.action === "spotify.prevTrack"
      ) {
        clearPinnedSpotifyItem();
        if (isTauri) {
          void refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
        } else {
          requestSpotifyStatus();
        }
      }
    };

    window.addEventListener("astrodeck-core-action", handleCoreAction as EventListener);
    window.addEventListener("astrodeck-action-started", handleActionStarted as EventListener);
    window.addEventListener("astrodeck-action-executed", handleActionExecuted as EventListener);
    window.addEventListener("astrodeck-action-failed", handleActionFailed as EventListener);

    if (!isTauri) {
      // Browser debug mode: preview a built-in app scene in Settings.
      const debugLayout = getBuiltinLayout("vscode");
      sceneId = "vscode";
      layout = debugLayout;
      availableScenes = getBuiltinSceneIds();
      loading = false;
      logStatus = "connecting";
      updateDebugTitle();
      logInfo("Browser mode: loaded builtin VS Code layout", "Browser");
      markSceneSeen("vscode");
      void getAppVersion()
        .then((version) => {
          appVersion = version;
        })
        .catch(() => {
          appVersion = "";
        });
      void getShowSettingsTerminal()
        .then((enabled) => {
          showSettingsTerminal = enabled;
        })
        .catch(() => {
          showSettingsTerminal = true;
        });

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
          logInfo("Connected to AstroDeck log bus", "Browser");
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
                  void openExternalUrl(url, "Spotify authorization page");
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
        stopWeatherUpdates();
        clearSpotifyTransportRefreshTimers();
        if (retryTimer !== null) clearTimeout(retryTimer);
        logBusSocket?.close();
        logBusSocket = null;
        window.removeEventListener("astrodeck-core-action", handleCoreAction as EventListener);
        window.removeEventListener("astrodeck-action-started", handleActionStarted as EventListener);
        window.removeEventListener("astrodeck-action-executed", handleActionExecuted as EventListener);
        window.removeEventListener("astrodeck-action-failed", handleActionFailed as EventListener);
      };
    }

    // Tauri windows (main + settings)
    if (isTauri) {
      logInfo(`App mounted in Tauri environment (${windowLabel})`, "App");
      if (windowLabel === "main") {
        initTray();
      }
      void syncDeckFullscreenState();
      void getStartFullscreen()
        .then((enabled) => {
          startFullscreen = enabled;
        })
        .catch(() => {
          startFullscreen = false;
        });
      window.addEventListener("resize", syncDeckFullscreenState);

      const onDeckPresentationKeydown = (ev: KeyboardEvent) => {
        if (ev.key === "F11") {
          ev.preventDefault();
          void toggleDeckPresentationFullscreen();
          return;
        }
        if (ev.key !== "Escape") return;
        if (updatePopupOpen) {
          ev.preventDefault();
          updatePopupOpen = false;
          return;
        }
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
      window.addEventListener("keydown", onDeckPresentationKeydown, true);

      void (async () => {
        try {
          startMinimized = await getStartMinimized();
        } catch {
          startMinimized = true;
        }
        try {
          hideTaskbarIcon = await getHideTaskbarIcon();
        } catch {
          hideTaskbarIcon = false;
        }
        try {
          customViews = await getCustomViews();
        } catch {
          customViews = [];
        }
        try {
          disabledApps = await getDisabledApps();
        } catch {
          disabledApps = [];
        }
        try {
          autoSwitchScenes = await getAutoSwitchScenes();
        } catch {
          autoSwitchScenes = {};
        }
        await refreshScene();
      })();
      void (async () => {
        try {
          appVersion = await getAppVersion();
        } catch {
          appVersion = "";
        }
        try {
          showUpdatePopups = await getUpdatePopupsEnabled();
        } catch {
          showUpdatePopups = true;
        }
        try {
          showSettingsTerminal = await getShowSettingsTerminal();
        } catch {
          showSettingsTerminal = false;
        }
        await checkForUpdates(true);
      })();
      refreshPluginsForDesktop();
      if (sceneId === "spotify") {
        void refreshSpotifyStatusForDesktop({ fresh: true, immediate: true });
      }
      void hydrateOsLocalMedia();
      void getYouTubeMusicStatus()
        .then((status) => {
          youtubeMusicStatus = status;
        })
        .catch((error) => logError(`Failed to fetch YouTube Music status: ${String(error)}`, "YouTube Music"));
      void getYouTubeMusicLibrary()
        .then((library) => {
          youtubeMusicLibrary = library;
        })
        .catch((error) => logError(`Failed to load YouTube Music local library: ${String(error)}`, "YouTube Music"));
      void getTeamsSetupStatus()
        .then((status) => {
          teamsSetupStatus = status;
        })
        .catch((error) => logError(`Failed to read Teams setup status: ${String(error)}`, "Teams"));
      void getTeamsStatus()
        .then((status) => {
          teamsStatus = status;
        })
        .catch(() => {});

      const onWindowFocus = () => {
        void refreshSpotifyStatusForDesktop();
        if (sceneId === "media") {
          void refreshOsVolume();
        }
      };
      window.addEventListener("focus", onWindowFocus);

      const unlistenOsNowPlaying = listen<OsNowPlaying>("os-now-playing", (event) => {
        applyOsNowPlaying(event.payload);
      });
      const unlistenSpotifyStatus = listen<SpotifyStatus>("spotify-status", (event) => {
        spotifyStatus = mergeSpotifyStatusFromServer(event.payload);
        if (event.payload.isAuthenticated) {
          spotifyAuthHint = null;
        }
      });
      const unlistenYouTubeStatus = listen<YouTubeMusicStatus>("youtube-status", (event) => {
        youtubeMusicStatus = event.payload;
      });
      const unlistenTeamsStatus = listen<TeamsStatus>("teams-status", (event) => {
        teamsStatus = event.payload;
      });
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
        if (event.payload.activeSceneId === "media") {
          void hydrateOsLocalMedia();
        }
        if (!activeCustomViewId) {
          void syncWindowToPresence(event.payload.activeSceneId);
        }
        if (
          viewMode !== "settings" &&
          !activeCustomViewId &&
          !$weatherDetailOpen &&
          !appEnabled(event.payload.activeSceneId)
        ) {
          ensureShownAppEnabled();
        }
      });
      const unlistenTerminal = listen<boolean>("settings-terminal-changed", (event) => {
        showSettingsTerminal = event.payload;
      });
      const unlistenAutoSwitch = listen<Record<string, boolean>>(
        "auto-switch-scenes-changed",
        (event) => {
          autoSwitchScenes = event.payload ?? {};
        }
      );

      const skipTextTypes = new Set([
        "button",
        "checkbox",
        "radio",
        "range",
        "file",
        "submit",
        "reset",
        "image",
        "color",
        "hidden",
      ]);
      let textInputHeld = false;
      const textFieldFrom = (target: EventTarget | null): HTMLElement | null => {
        const start = target instanceof Element ? target : null;
        const field = start?.closest("input, textarea, select, [contenteditable='true']");
        if (!(field instanceof HTMLElement)) return null;
        if (field instanceof HTMLInputElement && skipTextTypes.has(field.type)) return null;
        if (field instanceof HTMLInputElement && field.disabled) return null;
        if (field instanceof HTMLTextAreaElement && field.disabled) return null;
        return field;
      };
      const onTextPointerDown = (event: PointerEvent) => {
        const field = textFieldFrom(event.target);
        if (!field) return;
        void (async () => {
          if (!textInputHeld) {
            textInputHeld = true;
            try {
              await invoke("begin_text_input");
            } catch {
              textInputHeld = false;
              return;
            }
          }
          field.focus();
        })();
      };
      const onTextFocusOut = () => {
        window.setTimeout(() => {
          if (textFieldFrom(document.activeElement)) return;
          if (!textInputHeld) return;
          textInputHeld = false;
          void invoke("end_text_input");
        }, 0);
      };
      window.addEventListener("pointerdown", onTextPointerDown, true);
      window.addEventListener("focusout", onTextFocusOut);

      return () => {
        stopWeatherUpdates();
        clearSpotifyTransportRefreshTimers();
        window.removeEventListener("pointerdown", onTextPointerDown, true);
        window.removeEventListener("focusout", onTextFocusOut);
        if (textInputHeld) void invoke("end_text_input");
        window.removeEventListener("resize", syncDeckFullscreenState);
        window.removeEventListener("keydown", onDeckPresentationKeydown, true);
        window.removeEventListener("focus", onWindowFocus);
        unlisten.then((fn) => fn());
        unlistenOsNowPlaying.then((fn) => fn());
        unlistenSpotifyStatus.then((fn) => fn());
        unlistenYouTubeStatus.then((fn) => fn());
        unlistenTeamsStatus.then((fn) => fn());
        unlistenTerminal.then((fn) => fn());
        unlistenAutoSwitch.then((fn) => fn());
        window.removeEventListener("astrodeck-core-action", handleCoreAction as EventListener);
        window.removeEventListener("astrodeck-action-started", handleActionStarted as EventListener);
        window.removeEventListener("astrodeck-action-executed", handleActionExecuted as EventListener);
        window.removeEventListener("astrodeck-action-failed", handleActionFailed as EventListener);
      };
    }
  });
</script>

<div class="app" class:app-settings={isSettingsWindow} class:has-side-nav={isTauri}>
  {#if isTauri}
    <SideNav
      active={sideNavActive}
      customViews={customViews}
      disabledApps={disabledApps}
      onSelect={onSideNavSelect}
    />
  {/if}
  {#if showGlobalSceneBackground}
    <div class="app-scene-bg">
      <SceneBackground
        style={$sceneBackgroundId}
        colors={globalSceneColors}
        imageUrl={
          usesCoverImage($sceneBackgroundId) && (!isHomeScene || homeNowPlaying?.playing)
            ? $sceneShaderImage
            : null
        }
        playing={true}
        fadeMs={isHomeScene && homeNowPlaying ? 6500 : isHomeScene && !$clockBgColorCustom && isNeutralAccent($clockDigitColor) ? 7000 : 700}
      />
    </div>
  {/if}
  {#if !isImmersiveDeckView && !isSettingsWindow}
    <header class="app-header">
      <div class="app-header-left">
        <h1 class="app-title">AstroDeck</h1>
        {#if isTauri && !isSettingsWindow}
          <span class="scene-badge">{sceneId}</span>
        {/if}
      </div>
      {#if isTauri}
        <div class="app-header-actions">
          {#if appUpdate?.available}
            <button
              type="button"
              class="header-update-btn"
              title={appUpdate.latestVersion
                ? `Update to ${appUpdate.latestVersion}`
                : "Update available"}
              aria-label={appUpdate.latestVersion
                ? `Update to ${appUpdate.latestVersion}`
                : "Update available"}
              disabled={appUpdateBusy}
              onclick={() => openUpdatePopup()}
            >
              <svg class="header-update-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path
                  fill="currentColor"
                  d="M12 4a1 1 0 0 1 1 1v5.59l1.3-1.3a1 1 0 1 1 1.4 1.42l-3 3a1 1 0 0 1-1.4 0l-3-3a1 1 0 0 1 1.4-1.42l1.3 1.3V5a1 1 0 0 1 1-1zm-7 9a1 1 0 0 1 1 1 7 7 0 0 0 14 0 1 1 0 1 1 2 0 9 9 0 1 1-18 0 1 1 0 0 1 1-1z"
                />
              </svg>
              {#if appUpdate.latestVersion}
                <span class="header-update-label">{appUpdate.latestVersion}</span>
              {/if}
            </button>
          {/if}
        </div>
      {/if}
    </header>
  {/if}

  {#if isTauri && isImmersiveDeckView && appUpdate?.available}
    <button
      type="button"
      class="update-fab"
      title={appUpdate.latestVersion ? `Update to ${appUpdate.latestVersion}` : "Update available"}
      aria-label={appUpdate.latestVersion ? `Update to ${appUpdate.latestVersion}` : "Update available"}
      disabled={appUpdateBusy}
      onclick={() => openUpdatePopup()}
    >
      <svg class="update-fab-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path
          fill="currentColor"
          d="M12 4a1 1 0 0 1 1 1v5.59l1.3-1.3a1 1 0 1 1 1.4 1.42l-3 3a1 1 0 0 1-1.4 0l-3-3a1 1 0 0 1 1.4-1.42l1.3 1.3V5a1 1 0 0 1 1-1zm-7 9a1 1 0 0 1 1 1 7 7 0 0 0 14 0 1 1 0 1 1 2 0 9 9 0 1 1-18 0 1 1 0 0 1 1-1z"
        />
      </svg>
      {#if appUpdate.latestVersion}
        <span class="update-fab-label">{appUpdate.latestVersion}</span>
      {/if}
    </button>
  {/if}

  <main class="app-main">
    {#each keptWebViews as view (view.id)}
      <div class="kept-web" class:kept-web-on={activeWebViewId === view.id} inert={activeWebViewId !== view.id}>
        <EmbedView url={view.url} title={view.name} />
      </div>
    {/each}
    {#if isSettingsWindow}
      <SettingsPanel
        isTauri={isTauri}
        onBackToDeck={isTauri ? () => (viewMode = "deck") : undefined}
        startOnBoot={startOnBoot}
        startOnBootBusy={startOnBootBusy}
        startMinimized={startMinimized}
        startMinimizedBusy={startMinimizedBusy}
        hideTaskbarIcon={hideTaskbarIcon}
        hideTaskbarIconBusy={hideTaskbarIconBusy}
        startFullscreen={startFullscreen}
        startFullscreenBusy={startFullscreenBusy}
        showSettingsTerminal={showSettingsTerminal}
        showSettingsTerminalBusy={showSettingsTerminalBusy}
        startupError={startOnBootError}
        onStartOnBootChange={onStartOnBootChange}
        onStartMinimizedChange={onStartMinimizedChange}
        onHideTaskbarIconChange={onHideTaskbarIconChange}
        onStartFullscreenChange={onStartFullscreenChange}
        onShowSettingsTerminalChange={onShowSettingsTerminalChange}
        appVersion={appVersion}
        appUpdate={appUpdate}
        appUpdateChecking={appUpdateChecking}
        appUpdateBusy={appUpdateBusy}
        appUpdateError={appUpdateError}
        installableUpdate={installableUpdate}
        showUpdatePopups={showUpdatePopups}
        showUpdatePopupsBusy={showUpdatePopupsBusy}
        onCheckUpdates={() => void checkForUpdates()}
        onInstallUpdate={() => void installAppUpdate()}
        onShowUpdatePopupsChange={onShowUpdatePopupsChange}
        spotifyStatus={spotifyStatus}
        youtubeMusicStatus={youtubeMusicStatus}
        youtubeMusicLibrary={youtubeMusicLibrary}
        teamsSetupStatus={teamsSetupStatus}
        onSetupTeams={async () => {
          teamsSetupStatus = await setupTeamsIntegration();
        }}
        lyricsProviderOrder={lyricsProviderOrder}
        onLyricsProviderOrderChange={(order) => {
          lyricsProviderOrder = order;
          setLyricsProviderOrder(order);
        }}
        spotifyBusy={spotifyBusy}
        spotifyAuthHint={spotifyAuthHint}
        bind:spotifyClientIdDraft
        spotifyAuthMode={spotifyAuthMode}
        spotifyClientLockedByEnv={spotifyClientLockedByEnv}
        spotifySavingClientId={spotifySavingClientId}
        spotifySavingAuthMode={spotifySavingAuthMode}
        onSaveSpotifyClientId={() => void saveSpotifyClientId()}
        onSpotifyAuthModeChange={(mode) => void saveSpotifyAuthMode(mode)}
        onOpenSpotifyDashboard={openSpotifyDeveloperDashboard}
        onConnectSpotify={() => void connectSpotify()}
        onDisconnectSpotify={() => void disconnectSpotify()}
        sceneIds={settingsSceneIds}
        pluginsById={pluginsById}
        sceneId={sceneId}
        seenScenes={seenScenes}
        autoSwitchScenes={autoSwitchScenes}
        autoSwitchBusyId={autoSwitchBusyId}
        onSelectScene={selectScene}
        onAutoSwitchChange={onAutoSwitchChange}
        customViews={customViews}
        customViewsBusy={customViewsBusy}
        onSaveCustomViews={saveCustomViews}
        onOpenCustomView={openCustomView}
        disabledApps={disabledApps}
        appToggleBusyId={appToggleBusyId}
        onSetAppEnabled={onSetAppEnabled}
      />
    {:else}
      {#if activeCustomView}
        <!-- The website stays mounted in .kept-web so leaving and returning does not reload it. -->
      {:else if $performanceOpen}
        <PerformanceView />
      {:else if $weatherDetailOpen}
        <WeatherView />
      {:else if loading}
        <div class="loading">Detecting environment...</div>
      {:else if currentMediaView}
        <MediaPlayerView
          previous={currentMediaView.previous}
          playPause={isYouTubeMusicActive
            ? {
                label: youtubeMusicStatus?.isPlaying ? "Pause" : "Play",
                emoji: youtubeMusicStatus?.isPlaying ? "⏸️" : "▶️",
                action: "youtubeMusic.togglePlay",
              }
            : currentMediaView.playPause}
          next={currentMediaView.next}
          like={isOsMediaScene ? null : currentMediaView.like}
          shuffle={isOsMediaScene ? null : currentMediaView.shuffle}
          shuffleActive={isYouTubeMusicActive
            ? youtubeMusicStatus?.isShuffle ?? false
            : isSpotifyScene ? effectiveSpotifyShuffle : false}
          trackSaved={isYouTubeMusicActive
            ? youtubeMusicStatus?.isCurrentTrackSaved ?? false
            : isSpotifyScene ? effectiveSpotifySaved : null}
          lyricsEnabled={
            (isSpotifyScene || isYouTubeMusicScene) &&
            !!(spotifyStatus?.currentItemId || youtubeMusicStatus?.currentItemId)
          }
          trackId={isYouTubeMusicActive
            ? youtubeMusicStatus?.currentItemId ?? null
            : isSpotifyScene
              ? spotifyStatus?.currentItemId ?? null
              : null}
          title={mediaPlayerTitle}
          subtitle={mediaPlayerSubtitle}
          albumName={isYouTubeMusicActive
            ? youtubeMusicStatus?.currentAlbumName
            : isSpotifyScene
              ? spotifyStatus?.currentAlbumName
            : isOsMediaScene
              ? osLocalNowPlaying?.album
              : null}
          artworkUrl={isYouTubeMusicActive
            ? youtubeMusicStatus?.currentCoverArtUrl
            : isSpotifyScene
              ? spotifyStatus?.currentCoverArtUrl
            : isOsMediaScene
              ? osLocalNowPlaying?.coverArtUrl
              : null}
          playbackState={isYouTubeMusicActive
            ? (youtubeMusicStatus?.playbackState ?? "stopped")
            : isSpotifyScene
              ? effectiveSpotifyPlaybackState
            : isOsMediaScene
              ? effectiveOsMediaPlaybackState
              : "stopped"}
          progressMs={isYouTubeMusicActive
            ? youtubeMusicStatus?.progressMs
            : isSpotifyScene
              ? spotifyStatus?.progressMs
            : isOsMediaScene
              ? osLocalNowPlaying?.progressMs
              : null}
          durationMs={isYouTubeMusicActive
            ? youtubeMusicStatus?.durationMs
            : isSpotifyScene
              ? spotifyStatus?.durationMs
            : isOsMediaScene
              ? osLocalNowPlaying?.durationMs
              : null}
          volumeAction={isYouTubeMusicActive ? "youtubeMusic.setVolume" : currentMediaView.volumeAction}
          seekAction={isYouTubeMusicActive ? "youtubeMusic.seek" : currentMediaView.seekAction}
          volumePercent={isYouTubeMusicActive
            ? youtubeMusicStatus?.currentVolumePercent ?? 80
            : isOsMediaScene ? osVolumePercent : spotifyVolumePercent}
          volumeBusy={isYouTubeMusicActive ? false : isOsMediaScene ? osVolumeBusy : spotifyVolumeBusy}
          volumeLiveEnabled={!(isSpotifyScene && spotifyAuthMode === "custom")}
          volumeEnabled={isYouTubeMusicActive || isOsMediaScene || !isSpotifyScene || !!spotifyStatus?.hasActiveDevice}
          seekEnabled={isYouTubeMusicActive || isOsMediaScene || !isSpotifyScene || !!spotifyStatus?.hasActiveDevice}
          source={`${sceneId} window`}
          onVolumeCommit={(value) =>
            (isYouTubeMusicActive ? "youtubeMusic.setVolume" : currentMediaView.volumeAction)
              ? commitSceneVolume(
                  isYouTubeMusicActive ? "youtubeMusic.setVolume" : currentMediaView.volumeAction!,
                  value
                )
              : Promise.resolve()}
          onSeekCommit={(positionMs) =>
            (isYouTubeMusicActive ? "youtubeMusic.seek" : currentMediaView.seekAction)
              ? commitSceneSeek(
                  isYouTubeMusicActive ? "youtubeMusic.seek" : currentMediaView.seekAction!,
                  positionMs
                )
              : Promise.resolve()}
          playlistsEnabled={isSpotifyScene}
          youtubeMusicEnabled={isYouTubeMusicScene}
          youtubeMusicLibraryEnabled={isYouTubeMusicScene}
          addToPlaylistEnabled={isSpotifyScene || isYouTubeMusicScene}
          playlistProvider={isYouTubeMusicScene ? "youtubeMusic" : isSpotifyScene ? "spotify" : null}
          currentPlaylistId={isYouTubeMusicActive
            ? youtubeMusicStatus?.currentPlaylistId ?? null
            : isSpotifyScene
              ? spotifyStatus?.currentPlaylistId ?? null
              : null}
          trackInCurrentOwnedPlaylist={isYouTubeMusicActive
            ? youtubeTrackInCurrentOwnedPlaylist
            : isSpotifyScene
              ? spotifyStatus?.currentTrackInOwnedPlaylist ?? false
              : false}
          onPlaylistMembershipChange={() => {
            if (isYouTubeMusicScene) {
              void getYouTubeMusicLibrary()
                .then((library) => {
                  youtubeMusicLibrary = library;
                })
                .catch(() => {});
            }
            if (isSpotifyScene) requestSpotifyStatus();
          }}
          lyricsProviderOrder={lyricsProviderOrder}
          useLyricsFallback={isYouTubeMusicScene}
        />
      {:else if sceneId === "clock"}
        <ClockWeatherView
          nowPlaying={homeNowPlaying}
          weatherEnabled={appEnabled("weather")}
          onOpenPlaying={(sceneIdToOpen) => selectScene(sceneIdToOpen)}
        />
      {:else if sceneId === "teams" && displayedLayout}
        <TeamsScene
          buttons={displayedLayout.buttons}
          teamsStatus={teamsStatus}
        />
      {:else if sceneId === "vscode" && displayedLayout}
        <VsCodeScene buttons={displayedLayout.buttons} />
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

  {#if showLogsPanel}
    <section class="logs-panel">
      <div class="logs-card">
        <div class="logs-toolbar">
          <div class="logs-heading">
            <strong>Logs</strong>
            <span class="logs-status" class:on={isTauri || logStatus === "connected"}>
              {#if isTauri}
                Live
              {:else if logStatus === "connected"}
                Connected
              {:else if logStatus === "disconnected"}
                Disconnected
              {:else}
                Connecting
              {/if}
            </span>
          </div>
          <label class="logs-autoscroll">
            <input
              type="checkbox"
              checked={autoScroll}
              onchange={toggleAutoScroll}
            />
            <span class="logs-check" aria-hidden="true"></span>
            Auto-scroll
          </label>
          <button
            type="button"
            class="logs-btn"
            onclick={copyLogs}
            title="Copy logs to clipboard"
          >
            Copy
          </button>
          <button
            type="button"
            class="logs-btn"
            onclick={clearLogs}
            title="Clear logs"
          >
            Clear
          </button>
        </div>
        <div class="logs-lines" bind:this={logsLinesEl}>
          {#each $logs as entry, i}
            <div class="log-line log-{entry.level}">
              <span class="log-ts">{entry.timestamp}</span>
              <span class="log-source">{entry.source}</span>
              <span class="log-msg">{entry.message}</span>
            </div>
          {/each}
          {#if !$logs.length}
            <div class="log-line log-empty">
              <span class="log-msg">No logs yet. Interact with the deck to see activity.</span>
            </div>
          {/if}
        </div>
      </div>
    </section>
  {/if}

  {#if isTauri && updatePopupOpen && installableUpdate && appUpdate}
    <div class="update-modal-backdrop">
      <div
        class="update-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-modal-title"
      >
        <div class="update-modal-heading">
          <div>
            <p class="update-modal-eyebrow">Software update</p>
            <h2 id="update-modal-title">Version {appUpdate.latestVersion} is available</h2>
          </div>
          <button
            type="button"
            class="update-modal-close"
            aria-label="Close update"
            onclick={() => (updatePopupOpen = false)}
          >
            ×
          </button>
        </div>
        <p>Would you like to install it now? AstroDeck will close and run the installer.</p>
        <ReleaseNotes body={appUpdate.releaseNotes} />
        {#if appUpdateError}
          <p class="settings-error">{appUpdateError}</p>
        {/if}
        <div class="update-modal-actions">
          <button
            type="button"
            class="settings-secondary-btn"
            onclick={() => (updatePopupOpen = false)}
          >
            No, later
          </button>
          <button
            type="button"
            class="settings-primary-btn"
            disabled={appUpdateBusy}
            onclick={() => void installAppUpdate()}
          >
            {appUpdateBusy ? "Installing…" : "Yes, update now"}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .app {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .app.has-side-nav {
    --side-inset: 144px;
  }

  .app.has-side-nav .app-header,
  .app.has-side-nav .logs-panel,
  .app.has-side-nav .app-main > :global(*:not(.car-thing):not(.embed-view):not(.perf-view):not(.kept-web)) {
    padding-left: var(--side-inset);
  }

  .kept-web {
    position: absolute;
    inset: 0;
    z-index: 0;
    visibility: hidden;
    pointer-events: none;
  }

  .kept-web-on {
    z-index: 2;
    visibility: visible;
    pointer-events: auto;
  }

  .app-scene-bg {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
  }

  .app.app-settings {
    background: var(--settings-page-bg);
    color-scheme: light;
  }

  :global(html[data-settings-theme="dark"]) .app.app-settings {
    color-scheme: dark;
  }

  .app-header {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px 10px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-primary);
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

  .app-header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .header-update-btn,
  .update-fab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid rgba(29, 185, 84, 0.45);
    background: rgba(29, 185, 84, 0.14);
    color: #1ed760;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .header-update-btn {
    padding: 6px 10px;
    border-radius: 999px;
    font-size: 0.78rem;
    font-weight: 700;
    line-height: 1;
  }

  .header-update-btn:hover:not(:disabled),
  .update-fab:hover:not(:disabled) {
    background: rgba(29, 185, 84, 0.24);
    border-color: rgba(29, 185, 84, 0.7);
  }

  .header-update-btn:disabled,
  .update-fab:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .header-update-icon,
  .update-fab-icon {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .header-update-label,
  .update-fab-label {
    white-space: nowrap;
  }

  .update-fab {
    position: fixed;
    top: 12px;
    right: 148px;
    z-index: 5;
    padding: 8px 12px;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 700;
    line-height: 1;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
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

  .app-main {
    position: relative;
    z-index: 1;
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    overflow: hidden;
    background: transparent;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .settings-error {
    margin: 10px 0 0;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .settings-primary-btn,
  .settings-secondary-btn {
    padding: 10px 14px;
    border: none;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .settings-primary-btn {
    background: #111111;
    color: #fff;
  }

  .settings-secondary-btn {
    background: #262626;
    color: var(--text-primary);
  }

  .settings-primary-btn:hover:not(:disabled),
  .settings-secondary-btn:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .settings-primary-btn:disabled,
  .settings-secondary-btn:disabled {
    opacity: 0.55;
    cursor: progress;
  }

  .update-modal-backdrop {
    position: fixed;
    z-index: 40;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(8, 8, 14, 0.62);
  }

  .update-modal {
    width: min(560px, 100%);
    max-height: min(680px, calc(100vh - 48px));
    overflow: auto;
    padding: 24px;
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    background: var(--bg-surface);
    box-shadow: 0 22px 60px rgba(0, 0, 0, 0.45);
  }

  .update-modal-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .update-modal-eyebrow {
    margin: 0 0 6px;
    font-size: 0.75rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .update-modal h2 {
    margin: 0;
    font-size: 1.35rem;
    font-weight: 700;
  }

  .update-modal > p {
    margin: 16px 0 12px;
    color: var(--text-secondary);
    font-size: 0.95rem;
  }

  .update-modal-close {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
  }

  .update-modal-close:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .update-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 16px;
  }

  .logs-panel {
    flex: 0 0 46%;
    min-height: 0;
    background: #0d0d0d;
    padding: 12px 16px 16px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    user-select: text;
    -webkit-user-select: text;
  }

  .logs-card {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #171717;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2), 0 14px 40px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }

  .logs-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 12px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .logs-heading {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-right: auto;
  }

  .logs-heading strong {
    color: #f5f5f5;
    font-size: 0.98rem;
    font-weight: 600;
  }

  .logs-status {
    color: #a3a3a3;
    font-size: 0.8rem;
    font-weight: 500;
  }

  .logs-status.on {
    color: #4ade80;
  }

  .logs-autoscroll {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 0 8px;
    color: #e5e5e5;
    font-size: 0.9rem;
    cursor: pointer;
    user-select: none;
  }

  .logs-autoscroll input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }

  .logs-check {
    position: relative;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    border: 2px solid #737373;
    background: #262626;
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .logs-check::after {
    content: "";
    position: absolute;
    inset: 0;
    background: center / 12px 12px no-repeat
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath fill='none' stroke='%23fff' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round' d='M3.5 8.5 6.5 11.5 12.5 4.8'/%3E%3C/svg%3E");
    transform: scale(0);
    transform-origin: center;
    transition: transform 0.12s ease;
  }

  .logs-autoscroll:hover .logs-check {
    border-color: #a3a3a3;
  }

  .logs-autoscroll input:checked + .logs-check {
    background: #2563eb;
    border-color: #2563eb;
  }

  .logs-autoscroll input:checked + .logs-check::after {
    transform: scale(1);
  }

  .logs-autoscroll:has(input:focus-visible) .logs-check {
    outline: 2px solid #2563eb;
    outline-offset: 2px;
  }

  .logs-btn {
    min-height: 44px;
    padding: 10px 16px;
    border-radius: 8px;
    border: none;
    background: #262626;
    color: #f5f5f5;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s ease, transform 0.15s ease;
  }

  .logs-btn:hover {
    background: #333333;
    transform: translateY(-1px);
  }

  .logs-btn:active {
    transform: translateY(0);
  }

  .logs-lines {
    flex: 1 1 auto;
    overflow-y: auto;
    padding: 10px 14px 16px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New",
      monospace;
    font-size: 0.8rem;
    line-height: 1.55;
  }

  .log-line {
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 3px 0;
    color: #a3a3a3;
  }

  .log-line.log-info .log-msg {
    color: #93c5fd;
  }

  .log-line.log-error .log-msg {
    color: #fca5a5;
  }

  .log-ts {
    flex: 0 0 auto;
    min-width: 72px;
    color: #a3a3a3;
  }

  .log-source {
    flex: 0 0 auto;
    min-width: 88px;
    color: #d4d4d4;
    font-weight: 500;
  }

  .log-msg {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .log-line.log-empty .log-msg {
    color: #a3a3a3;
  }
</style>
