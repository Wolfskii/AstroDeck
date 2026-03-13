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
    getSpotifyStatus,
    setActiveScene,
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
  // Desktop (Tauri) windows are for the buttons/scene UI.
  // The browser (`http://localhost:1420/`) acts as the dedicated settings/debug window.
  const isSettingsWindow = $derived(!isTauri);
  let logStatus = $state<"connecting" | "connected" | "disconnected">("connecting");
  let autoScroll = $state(true);
  let logsLinesEl = $state<HTMLElement | null>(null);
  let logBusSocket: WebSocket | null = null;
  let plugins = $state<PluginConfig[]>([]);
  let spotifyStatus = $state<SpotifyStatus | null>(null);
  let spotifyBusy = $state(false);
  let spotifyVolumeBusy = $state(false);
  let spotifyVolumePercent = $state(50);
  let optimisticSpotifySaved = $state<boolean | null>(null);
  const pluginsById = $derived(new Map(plugins.map((plugin) => [plugin.id, plugin])));
  const currentPlugin = $derived(plugins.find((plugin) => plugin.id === sceneId) ?? null);
  const effectiveSpotifySaved = $derived(
    optimisticSpotifySaved ?? spotifyStatus?.isCurrentTrackSaved ?? null
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
      return {
        ...button,
        label: spotifyStatus?.isPlaying ? "Pause" : "Play",
        emoji: spotifyStatus?.isPlaying ? "⏸️" : "▶️",
      };
    };

    return {
      previous: media.previous ?? null,
      playPause: withSpotifyPlayState(media.playPause),
      next: media.next ?? null,
      like: withSpotifyLikeState(media.like),
      volumeAction: media.volumeAction ?? null,
    };
  });
  const displayedLayout = $derived.by(() => {
    if (!layout) return null;
    return layout;
  });

  $effect(() => {
    const next = spotifyStatus?.currentVolumePercent;
    if (next == null || spotifyVolumeBusy) return;
    spotifyVolumePercent = next;
  });

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

  function connectSpotify() {
    if (logBusSocket?.readyState !== WebSocket.OPEN) {
      logError("Log bus is not connected; cannot start Spotify auth", "Spotify");
      return;
    }
    spotifyBusy = true;
    logBusSocket.send(JSON.stringify({ type: "spotifyAuthStart" }));
  }

  function disconnectSpotify() {
    if (logBusSocket?.readyState !== WebSocket.OPEN) {
      logError("Log bus is not connected; cannot disconnect Spotify", "Spotify");
      return;
    }
    spotifyBusy = true;
    logBusSocket.send(JSON.stringify({ type: "spotifyDisconnect" }));
  }

  async function refreshSpotifyStatusForDesktop() {
    if (!isTauri) return;
    try {
      spotifyStatus = await getSpotifyStatus();
      optimisticSpotifySaved = null;
    } catch (e) {
      logError(`Failed to fetch Spotify status: ${String(e)}`, "Spotify");
    }
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
    try {
      spotifyVolumeBusy = true;
      await executeActionValue(action, value);
      if (action.startsWith("spotify.")) {
        spotifyVolumePercent = value;
        await refreshSpotifyStatusForDesktop();
      }
    } catch (e) {
      logError(`Failed to set volume via ${action}: ${String(e)}`, `${sceneId} window`);
    } finally {
      spotifyVolumeBusy = false;
    }
  }

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
                await win.show();
                await win.unminimize();
                await win.setFocus();
                logInfo("Showed TapTapDeck window from tray menu", "Tray");
                const item = await trayMenu?.get("toggle");
                if (item) await item.setText("Hide TapTapDeck");
              }
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
        await refreshSpotifyStatusForDesktop();
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
    const handleCoreAction = (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string; label: string }>).detail;
      if (!detail) return;
      const { action, label } = detail;

      if (action === "core.settings") {
        logInfo(`Settings requested from button: ${label}`, "Settings window");
        if (isTauri) {
          // Open or focus the browser-based debug/settings panel.
          if (typeof window !== "undefined") {
            window.open("http://localhost:1420", "_blank", "noopener,noreferrer");
          }
        } else {
          // In browser debug mode we're already in the settings/debug window.
        }
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
        if (detail.action === "spotify.like") {
          optimisticSpotifySaved = null;
        }
        if (isTauri) {
          refreshSpotifyStatusForDesktop();
        } else {
          requestSpotifyStatus();
        }
      }
    };

    const handleActionStarted = (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string }>).detail;
      if (!detail?.action) return;
      if (detail.action === "spotify.like") {
        optimisticSpotifySaved = !(effectiveSpotifySaved === true);
      }
    };

    const handleActionFailed = (ev: Event) => {
      const detail = (ev as CustomEvent<{ action: string }>).detail;
      if (!detail?.action) return;
      if (detail.action === "spotify.like") {
        optimisticSpotifySaved = null;
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
                spotifyStatus = payload.payload as SpotifyStatus;
                spotifyBusy = false;
                optimisticSpotifySaved = null;
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
      refreshScene();
      refreshPluginsForDesktop();
      if (sceneId === "spotify") {
        refreshSpotifyStatusForDesktop();
      }

      const unlisten = listen<SceneState>("scene-changed", (event) => {
        sceneId = event.payload.activeSceneId;
        layout = event.payload.layout;
        availableScenes = event.payload.availableScenes;
        logInfo(`Scene changed to: ${event.payload.activeSceneId}`, "Scenes");
        markSceneSeen(event.payload.activeSceneId);
        refreshPluginsForDesktop();
        if (event.payload.activeSceneId === "spotify") {
          refreshSpotifyStatusForDesktop();
        }
      });

      return () => {
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
  <header class="app-header">
    <h1 class="app-title">TapTapDeck</h1>
    {#if isTauri && !isSettingsWindow}
      <span class="scene-badge">{sceneId}</span>
    {/if}
  </header>

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
          title={sceneId === "spotify" ? spotifyStatus?.currentTrackName : null}
          subtitle={sceneId === "spotify" ? spotifyStatus?.currentArtistName : null}
          artworkUrl={sceneId === "spotify" ? spotifyStatus?.currentCoverArtUrl : null}
          playbackState={sceneId === "spotify" ? spotifyStatus?.playbackState ?? "stopped" : "stopped"}
          volumeAction={currentMediaView.volumeAction}
          volumePercent={spotifyVolumePercent}
          volumeBusy={spotifyVolumeBusy}
          volumeEnabled={sceneId !== "spotify" || !!spotifyStatus?.hasActiveDevice}
          source={`${sceneId} window`}
          onVolumeCommit={(value) =>
            currentMediaView.volumeAction
              ? commitSceneVolume(currentMediaView.volumeAction, value)
              : Promise.resolve()}
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
    padding: 12px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
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
    flex: 1 1 auto;
    display: flex;
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

  .spotify-actions {
    display: flex;
    align-items: center;
    gap: 10px;
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
