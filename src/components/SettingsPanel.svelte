<script lang="ts">
  import ReleaseNotes from "./ReleaseNotes.svelte";
  import appIconUrl from "../assets/app-icon.png";
  import { getBuiltinSceneMeta } from "../layouts/layouts";
  import { SCENE_BACKGROUND_OPTIONS } from "../lib/sceneBackgrounds";
  import SpectrumColorPicker from "./SpectrumColorPicker.svelte";
  import {
    controlsBackdropEnabled,
    controlsOverlayColor,
    controlsOverlayCustom,
    controlsTransparency,
    persistControlsBackdropEnabled,
    persistControlsOverlayColor,
    persistControlsOverlayCustom,
    persistControlsTransparency,
    persistSceneBackground,
    persistAudioVisualizerEnabled,
    persistSettingsTheme,
    previewControlsOverlayColor,
    sceneBackgroundId,
    audioVisualizerEnabled,
    audioVisualizerSupported,
    audioVisualizerError,
    settingsTheme,
    settingsDark,
  } from "../stores/appearance";
  import type { SettingsThemeId } from "../services/prefs";
  import type { PluginConfig } from "../types";
  import type { AppUpdateInfo } from "../services/updater";
  import type { SpotifyAuthMode, SpotifyStatus } from "../services/api";
  import PlaylistBrowser from "./PlaylistBrowser.svelte";

  type SettingsSection = "general" | "appearance" | "updates" | "spotify" | "scenes";

  let {
    isTauri,
    onBackToDeck,
    startOnBoot,
    startOnBootBusy,
    startMinimized,
    startMinimizedBusy,
    startFullscreen,
    startFullscreenBusy,
    showSettingsTerminal,
    showSettingsTerminalBusy,
    startupError,
    onStartOnBootChange,
    onStartMinimizedChange,
    onStartFullscreenChange,
    onShowSettingsTerminalChange,
    appVersion,
    appUpdate,
    appUpdateChecking,
    appUpdateBusy,
    appUpdateError,
    installableUpdate,
    showUpdatePopups,
    showUpdatePopupsBusy,
    onCheckUpdates,
    onInstallUpdate,
    onShowUpdatePopupsChange,
    spotifyStatus,
    spotifyBusy,
    spotifyAuthHint,
    spotifyClientIdDraft = $bindable(""),
    spotifyAuthMode = "official",
    spotifyClientLockedByEnv,
    spotifySavingClientId,
    spotifySavingAuthMode = false,
    onSaveSpotifyClientId,
    onSpotifyAuthModeChange,
    onOpenSpotifyDashboard,
    onConnectSpotify,
    onDisconnectSpotify,
    sceneIds,
    pluginsById,
    sceneId,
    seenScenes,
    autoSwitchScenes = {},
    autoSwitchBusyId = null,
    onSelectScene,
    onAutoSwitchChange,
  }: {
    isTauri: boolean;
    onBackToDeck?: () => void;
    startOnBoot: boolean;
    startOnBootBusy: boolean;
    startMinimized: boolean;
    startMinimizedBusy: boolean;
    startFullscreen: boolean;
    startFullscreenBusy: boolean;
    showSettingsTerminal: boolean;
    showSettingsTerminalBusy: boolean;
    startupError: string | null;
    onStartOnBootChange: (event: Event) => void;
    onStartMinimizedChange: (event: Event) => void;
    onStartFullscreenChange: (event: Event) => void;
    onShowSettingsTerminalChange: (event: Event) => void;
    appVersion: string;
    appUpdate: AppUpdateInfo | null;
    appUpdateChecking: boolean;
    appUpdateBusy: boolean;
    appUpdateError: string | null;
    installableUpdate: boolean;
    showUpdatePopups: boolean;
    showUpdatePopupsBusy: boolean;
    onCheckUpdates: () => void;
    onInstallUpdate: () => void;
    onShowUpdatePopupsChange: (event: Event) => void;
    spotifyStatus: SpotifyStatus | null;
    spotifyBusy: boolean;
    spotifyAuthHint: string | null;
    spotifyClientIdDraft: string;
    spotifyAuthMode: SpotifyAuthMode;
    spotifyClientLockedByEnv: boolean;
    spotifySavingClientId: boolean;
    spotifySavingAuthMode?: boolean;
    onSaveSpotifyClientId: () => void;
    onSpotifyAuthModeChange: (mode: SpotifyAuthMode) => void;
    onOpenSpotifyDashboard: () => void;
    onConnectSpotify: () => void;
    onDisconnectSpotify: () => void;
    sceneIds: string[];
    pluginsById: Map<string, PluginConfig>;
    sceneId: string;
    seenScenes: string[];
    autoSwitchScenes?: Record<string, boolean>;
    autoSwitchBusyId?: string | null;
    onSelectScene: (id: string) => void;
    onAutoSwitchChange?: (id: string, enabled: boolean) => void;
  } = $props();

  let section = $state<SettingsSection>("general");
  let playlistsOpen = $state(false);
  const activeSection = $derived(section);

  const titles: Record<SettingsSection, string> = {
    general: "General",
    appearance: "Appearance",
    updates: "Updates",
    spotify: "Spotify",
    scenes: "Scenes",
  };

  const updateBadge = $derived(installableUpdate ? "1" : null);

  const THEME_OPTIONS: { id: SettingsThemeId; label: string }[] = [
    { id: "system", label: "System" },
    { id: "light", label: "Light" },
    { id: "dark", label: "Dark" },
  ];
</script>

<div class="settings-shell" class:theme-dark={$settingsDark}>
  <aside class="settings-nav">
    <div class="settings-brand">
      <img class="settings-brand-icon" src={appIconUrl} alt="" />
      <div>
        <strong>AstroDeck</strong>
        <span>Settings</span>
      </div>
    </div>

    <nav class="settings-nav-list" aria-label="Settings sections">
      <button
        type="button"
        class="settings-nav-item"
        class:active={activeSection === "general"}
        onclick={() => (section = "general")}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M19.14 12.94c.04-.31.06-.63.06-.94s-.02-.63-.06-.94l2.03-1.58a.5.5 0 0 0 .12-.64l-1.92-3.32a.5.5 0 0 0-.6-.22l-2.39.96a7.03 7.03 0 0 0-1.63-.94l-.36-2.54A.5.5 0 0 0 13.9 2h-3.8a.5.5 0 0 0-.49.42l-.36 2.54c-.59.22-1.14.54-1.63.94l-2.39-.96a.5.5 0 0 0-.6.22L2.71 8.48a.5.5 0 0 0 .12.64L4.86 10.7c-.04.31-.06.63-.06.94s.02.63.06.94L2.83 14.16a.5.5 0 0 0-.12.64l1.92 3.32c.14.24.43.34.69.22l2.39-.96c.49.4 1.04.72 1.63.94l.36 2.54c.05.24.26.42.49.42h3.8c.24 0 .44-.18.49-.42l.36-2.54c.59-.22 1.14-.54 1.63-.94l2.39.96c.26.12.55.02.69-.22l1.92-3.32a.5.5 0 0 0-.12-.64zM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7z"
          />
        </svg>
        General
      </button>
      <button
        type="button"
        class="settings-nav-item"
        class:active={activeSection === "appearance"}
        onclick={() => (section = "appearance")}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M12 3a9 9 0 0 0 0 18c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01-.23-.26-.36-.6-.36-.99A1.5 1.5 0 0 1 14.25 16h1.5A5.25 5.25 0 0 0 21 10.75 9 9 0 0 0 12 3zm-5.25 8.25a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zm3-4.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zm4.5 0a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zm3 4.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3z"
          />
        </svg>
        Appearance
      </button>
      <button
        type="button"
        class="settings-nav-item"
        class:active={activeSection === "updates"}
        onclick={() => (section = "updates")}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M12 4a1 1 0 0 1 1 1v5.59l1.3-1.3a1 1 0 1 1 1.4 1.42l-3 3a1 1 0 0 1-1.4 0l-3-3a1 1 0 0 1 1.4-1.42l1.3 1.3V5a1 1 0 0 1 1-1zm-7 9a1 1 0 0 1 1 1 7 7 0 0 0 14 0 1 1 0 1 1 2 0 9 9 0 1 1-18 0 1 1 0 0 1 1-1z"
          />
        </svg>
        Updates
        {#if updateBadge}
          <span class="settings-nav-badge">{updateBadge}</span>
        {/if}
      </button>
      <button
        type="button"
        class="settings-nav-item"
        class:active={activeSection === "spotify"}
        onclick={() => (section = "spotify")}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M12 2a10 10 0 1 0 .01 20.01A10 10 0 0 0 12 2zm4.58 14.45a.75.75 0 0 1-1.03.25c-2.82-1.73-6.38-2.12-10.56-1.16a.75.75 0 0 1-.33-1.46c4.56-1.05 8.5-.6 11.67 1.34.35.22.46.67.25 1.03zm1.23-2.74a.9.9 0 0 1-1.24.3c-3.23-1.98-8.15-2.56-11.96-1.4a.9.9 0 1 1-.52-1.72c4.28-1.3 9.68-.65 13.4 1.63.43.26.56.82.32 1.19zm.1-2.85C14.3 8.9 8.4 8.7 5.17 9.68a1.05 1.05 0 0 1-.62-2.01c3.72-1.14 10.3-.9 14.4 1.53a1.05 1.05 0 0 1-1.04 1.66z"
          />
        </svg>
        Spotify
      </button>
      <button
        type="button"
        class="settings-nav-item"
        class:active={activeSection === "scenes"}
        onclick={() => (section = "scenes")}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M4 5a2 2 0 0 1 2-2h4v18H6a2 2 0 0 1-2-2zm8-2h6a2 2 0 0 1 2 2v4h-8zm0 8h8v6a2 2 0 0 1-2 2h-6z"
          />
        </svg>
        Scenes
      </button>
    </nav>

    {#if isTauri && onBackToDeck}
      <button type="button" class="settings-back" onclick={onBackToDeck}>
        ← Back to deck
      </button>
    {/if}
  </aside>

  <div class="settings-main">
    <section class="settings-content">
      <header class="settings-content-head">
      {#if isTauri && onBackToDeck}
        <button type="button" class="settings-back-inline" onclick={onBackToDeck}>
          ← Back to deck
        </button>
      {/if}
      <h1>{titles[activeSection]}</h1>
    </header>

    {#if activeSection === "general"}
      {#if !isTauri}
        <p class="settings-lead">
          Startup options take effect in the AstroDeck desktop app. Toggles here are a preview.
        </p>
      {/if}
      <div class="settings-card">
        <label class="setting-row" for="start-on-boot">
          <div class="setting-copy">
            <span class="setting-title">Start when this computer starts</span>
            <span class="setting-desc">Launch AstroDeck in the background after you sign in.</span>
          </div>
          <input
            id="start-on-boot"
            type="checkbox"
            checked={startOnBoot}
            disabled={startOnBootBusy}
            onchange={onStartOnBootChange}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
        <label class="setting-row" for="start-minimized">
          <div class="setting-copy">
            <span class="setting-title">Start minimized in the tray</span>
            <span class="setting-desc">
              Stay in the tray until you open AstroDeck yourself. Turn this off to restore the last
              position, size, and mode when VS Code, Cursor, Teams, Spotify, or a local media
              player opens.
            </span>
          </div>
          <input
            id="start-minimized"
            type="checkbox"
            checked={startMinimized}
            disabled={startMinimizedBusy}
            onchange={onStartMinimizedChange}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
        <label class="setting-row" for="start-fullscreen">
          <div class="setting-copy">
            <span class="setting-title">Fullscreen mode</span>
            <span class="setting-desc">
              Fill this display with no window chrome. The app also starts in the mode selected
              here. F11 also toggles this. Press F11 or Esc to leave fullscreen.
            </span>
          </div>
          <input
            id="start-fullscreen"
            type="checkbox"
            checked={startFullscreen}
            disabled={startFullscreenBusy}
            onchange={onStartFullscreenChange}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
        <label class="setting-row" for="show-settings-terminal">
          <div class="setting-copy">
            <span class="setting-title">Show terminal in Settings</span>
            <span class="setting-desc">
              Show the activity log under Settings. Off by default in the installed app.
            </span>
          </div>
          <input
            id="show-settings-terminal"
            type="checkbox"
            checked={showSettingsTerminal}
            disabled={showSettingsTerminalBusy}
            onchange={onShowSettingsTerminalChange}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
      </div>
      {#if startupError}
        <p class="settings-error">{startupError}</p>
      {/if}
    {:else if activeSection === "appearance"}
      <div class="settings-card">
        <div class="setting-row setting-row-theme">
          <div class="setting-copy">
            <span class="setting-title">Settings theme</span>
            <span class="setting-desc">
              Follow this computer's light or dark mode, or lock Settings to Light or Dark.
            </span>
          </div>
          <div class="theme-seg" role="radiogroup" aria-label="Settings theme">
            {#each THEME_OPTIONS as option (option.id)}
              <button
                type="button"
                role="radio"
                aria-checked={$settingsTheme === option.id}
                class:active={$settingsTheme === option.id}
                onclick={() => persistSettingsTheme(option.id)}
              >
                {option.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
      <p class="settings-lead">
        Backgrounds pick colors from the current cover art. They apply to the Spotify player
        first; other views will follow later.
      </p>
      <div class="scene-grid">
        {#each SCENE_BACKGROUND_OPTIONS as option (option.id)}
          <button
            type="button"
            class="scene-card"
            class:active={$sceneBackgroundId === option.id}
            onclick={() => persistSceneBackground(option.id)}
            aria-pressed={$sceneBackgroundId === option.id}
          >
            <div class="scene-card-top">
              <span class="scene-name">{option.label}</span>
              {#if $sceneBackgroundId === option.id}
                <span class="scene-tag current">current</span>
              {/if}
            </div>
            <p class="scene-description">{option.description}</p>
          </button>
        {/each}
      </div>
      <div class="settings-card">
        <label class="setting-row" class:setting-row-disabled={!$audioVisualizerSupported} for="audio-visualizer">
          <div class="setting-copy">
            <span class="setting-title">React to system audio</span>
            <span class="setting-desc">
              Drive the now-playing background from this computer's speaker output. Off uses a
              timed pulse. Nothing is recorded or sent.
            </span>
          </div>
          <input
            id="audio-visualizer"
            type="checkbox"
            checked={$audioVisualizerEnabled}
            disabled={!$audioVisualizerSupported}
            onchange={(event) =>
              persistAudioVisualizerEnabled((event.currentTarget as HTMLInputElement).checked)}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
        {#if !$audioVisualizerSupported}
          <p class="settings-note">
            Speaker capture is available in the Windows desktop app. The browser preview cannot
            tap the system audio mix.
          </p>
        {:else if $audioVisualizerError}
          <p class="settings-error">{$audioVisualizerError}</p>
        {/if}
      </div>
      <div class="settings-card">
        <label class="setting-row" for="controls-backdrop">
          <div class="setting-copy">
            <span class="setting-title">Background behind controls</span>
            <span class="setting-desc">
              Continue the scene background through the playback bar, with a dark overlay.
            </span>
          </div>
          <input
            id="controls-backdrop"
            type="checkbox"
            bind:checked={$controlsBackdropEnabled}
            onchange={(event) =>
              persistControlsBackdropEnabled((event.currentTarget as HTMLInputElement).checked)}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
        <div class="setting-row setting-row-slider" class:setting-row-disabled={!$controlsBackdropEnabled}>
          <div class="setting-copy">
            <span class="setting-title">Control bar transparency</span>
            <span class="setting-desc">
              Higher values let more of the background show through the playback controls.
            </span>
          </div>
          <div class="md-slider">
            <span class="md-slider-value">{$controlsTransparency}%</span>
            <div class="md-slider-ui">
              <div class="md-slider-track" aria-hidden="true"></div>
              <div class="md-slider-fill" style={`width: ${$controlsTransparency}%; background: ${$controlsOverlayCustom ? $controlsOverlayColor : "#000000"}`} aria-hidden="true"></div>
              <input
                id="controls-transparency"
                type="range"
                min="0"
                max="100"
                step="1"
                bind:value={$controlsTransparency}
                disabled={!$controlsBackdropEnabled}
                aria-label="Control bar transparency"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={$controlsTransparency}
                aria-valuetext={`${$controlsTransparency} percent`}
                oninput={(event) =>
                  persistControlsTransparency(Number((event.currentTarget as HTMLInputElement).value))}
              />
            </div>
          </div>
        </div>
        <div class="setting-row setting-row-slider" class:setting-row-disabled={!$controlsBackdropEnabled}>
          <label class="setting-colour-toggle" for="controls-overlay-custom">
            <div class="setting-copy">
              <span class="setting-title">Control bar colour</span>
              <span class="setting-desc">
                Use a custom tint on the playback bar. Off uses a dark overlay.
              </span>
            </div>
            <input
              id="controls-overlay-custom"
              type="checkbox"
              bind:checked={$controlsOverlayCustom}
              disabled={!$controlsBackdropEnabled}
              onchange={(event) =>
                persistControlsOverlayCustom((event.currentTarget as HTMLInputElement).checked)}
            />
            <span class="md-check" aria-hidden="true"></span>
          </label>
          {#if $controlsOverlayCustom}
            <SpectrumColorPicker
              value={$controlsOverlayColor}
              disabled={!$controlsBackdropEnabled}
              onChange={previewControlsOverlayColor}
              onCommit={persistControlsOverlayColor}
            />
          {/if}
        </div>
      </div>
    {:else if activeSection === "updates"}
      <div class="settings-card settings-card-pad">
        <div class="settings-card-toolbar">
          <div>
            <p class="setting-title">Software updates</p>
            <p class="settings-version">
              Current version <strong>{appVersion || "Unknown"}</strong>
            </p>
            <p class="setting-desc">
              AstroDeck checks GitHub for a newer installer when it starts.
            </p>
          </div>
          <button
            type="button"
            class="md-btn"
            disabled={appUpdateChecking}
            onclick={onCheckUpdates}
          >
            {appUpdateChecking ? "Checking…" : "Check for updates"}
          </button>
        </div>
        {#if appUpdateError}
          <p class="settings-error">{appUpdateError}</p>
        {/if}
        {#if installableUpdate && appUpdate}
          <div class="update-available" role="status">
            <strong>Version {appUpdate.latestVersion} is available</strong>
            <ReleaseNotes body={appUpdate.releaseNotes} />
            <button
              type="button"
              class="md-btn md-btn-primary"
              disabled={appUpdateBusy}
              onclick={onInstallUpdate}
            >
              {appUpdateBusy ? "Installing…" : "Update now"}
            </button>
          </div>
        {:else}
          <p class="settings-status" role="status">
            {appUpdateChecking ? "Checking for updates…" : "No update is currently available."}
          </p>
        {/if}
      </div>
      <div class="settings-card">
        <label class="setting-row" for="show-update-popups">
          <div class="setting-copy">
            <span class="setting-title">Show update popups</span>
            <span class="setting-desc">Ask to install when a new version is found at startup.</span>
          </div>
          <input
            id="show-update-popups"
            type="checkbox"
            checked={showUpdatePopups}
            disabled={showUpdatePopupsBusy}
            onchange={onShowUpdatePopupsChange}
          />
          <span class="md-check" aria-hidden="true"></span>
        </label>
      </div>
    {:else if activeSection === "spotify"}
      <div class="settings-card settings-card-pad">
        <div class="settings-card-toolbar">
          <div>
            <p class="setting-title">Account</p>
            <p class="setting-desc">
              {#if spotifyAuthMode === "official"}
                Sign in with your Spotify account. AstroDeck controls the Spotify app with OS media
                keys and the desktop playlist protocol — no developer app and no Web API.
              {:else}
                Connect once to enable track liking, shuffle, and Spotify-device volume through the
                Web API, plus OS media keys.
              {/if}
            </p>
          </div>
          <div class="settings-btn-row">
            <button class="md-btn" onclick={() => (playlistsOpen = true)}>
              Browse playlists
            </button>
            <button class="md-btn md-btn-primary" onclick={onConnectSpotify} disabled={spotifyBusy}>
              {#if spotifyBusy}
                Connecting…
              {:else if spotifyStatus?.isAuthenticated}
                Reconnect
              {:else}
                Connect
              {/if}
            </button>
            <button
              class="md-btn md-btn-danger"
              onclick={onDisconnectSpotify}
              disabled={spotifyBusy || !spotifyStatus?.isAuthenticated}
            >
              Disconnect
            </button>
          </div>
        </div>
        <div class="spotify-status-line">
          <span class="status-dot" class:on={spotifyStatus?.isAuthenticated}></span>
          <span>{spotifyStatus?.message ?? "Waiting for Spotify status…"}</span>
        </div>
        {#if isTauri && spotifyAuthHint}
          <p class="settings-hint" class:ok={spotifyAuthHint.startsWith("Finish")}>
            {spotifyAuthHint}
          </p>
        {/if}
      </div>

      <div class="settings-card settings-card-pad">
        <p class="setting-title">Login method</p>
        <p class="setting-desc">
          Desktop login is the default and does not use Spotify's Web API. Switch to a developer app
          if you want like, shuffle, and Spotify-device volume through the API.
        </p>
        <div class="auth-mode-list" role="radiogroup" aria-label="Spotify login method">
          <button
            type="button"
            class="auth-mode-card"
            class:active={spotifyAuthMode === "official"}
            role="radio"
            aria-checked={spotifyAuthMode === "official"}
            disabled={spotifyClientLockedByEnv || spotifySavingAuthMode}
            onclick={() => onSpotifyAuthModeChange("official")}
          >
            <div class="auth-mode-top">
              <span class="setting-title">Spotify desktop login</span>
              <span class="scene-tag current">default</span>
            </div>
            <p class="setting-desc">
              Sign in with your Spotify account. Playback, playlists, volume, and seek go through
              the Spotify app — not api.spotify.com. Play starts in that app; Spotify may flash
              briefly, then AstroDeck comes back to the front.
            </p>
          </button>
          <button
            type="button"
            class="auth-mode-card"
            class:active={spotifyAuthMode === "custom"}
            role="radio"
            aria-checked={spotifyAuthMode === "custom"}
            disabled={spotifyClientLockedByEnv || spotifySavingAuthMode}
            onclick={() => onSpotifyAuthModeChange("custom")}
          >
            <div class="auth-mode-top">
              <span class="setting-title">Your Spotify developer app</span>
            </div>
            <p class="setting-desc">
              Use a Client ID from the Spotify Developer Dashboard. Enables Web API like, shuffle,
              Spotify-device volume, and playlist play on the active device, plus OS media keys.
            </p>
          </button>
        </div>
        {#if spotifyClientLockedByEnv}
          <p class="setting-desc">
            Using <code>SPOTIFY_CLIENT_ID</code> from the environment, so developer-app login is
            locked. Unset it to change the method here.
          </p>
        {/if}
      </div>

      {#if spotifyAuthMode === "custom"}
        <div class="settings-card settings-card-pad">
          <p class="setting-title">Spotify Client ID</p>
          <p class="setting-desc">
            Create a Spotify app, add redirect URI
            <code>http://127.0.0.1:43821/callback</code>, then paste the Client ID. Or set
            <code>SPOTIFY_CLIENT_ID</code> in the environment.
          </p>
          <div class="settings-field-row">
            <input
              id="spotify-client-id"
              class="md-input"
              type="text"
              autocomplete="off"
              spellcheck="false"
              placeholder="Your Spotify app Client ID"
              bind:value={spotifyClientIdDraft}
              disabled={spotifyClientLockedByEnv || spotifySavingClientId}
            />
            <button type="button" class="md-btn" onclick={onOpenSpotifyDashboard}>
              Open dashboard
            </button>
            <button
              type="button"
              class="md-btn md-btn-primary"
              onclick={onSaveSpotifyClientId}
              disabled={spotifyClientLockedByEnv || spotifySavingClientId}
            >
              {spotifySavingClientId ? "Saving…" : "Save"}
            </button>
          </div>
          {#if spotifyClientLockedByEnv}
            <p class="setting-desc">
              Using <code>SPOTIFY_CLIENT_ID</code> from the environment; unset it to edit the saved
              Client ID here.
            </p>
          {/if}
        </div>
      {/if}

      {#if spotifyStatus?.currentCoverArtUrl || spotifyStatus?.currentTrackName}
        <div class="settings-card settings-card-pad now-playing">
          {#if spotifyStatus?.currentCoverArtUrl}
            <img
              class="cover"
              src={spotifyStatus.currentCoverArtUrl}
              alt={spotifyStatus?.currentTrackName ?? "Current cover art"}
            />
          {/if}
          <div>
            <p class="now-playing-state">{spotifyStatus?.playbackState ?? "stopped"}</p>
            <p class="setting-title">{spotifyStatus?.currentTrackName ?? "Nothing active"}</p>
            <p class="setting-desc">{spotifyStatus?.currentArtistName ?? "No artist information"}</p>
          </div>
        </div>
      {/if}

      <div class="settings-card facts">
        <div class="fact">
          <span>Device</span>
          <strong>{spotifyStatus?.activeDeviceName ?? "No active device"}</strong>
        </div>
        <div class="fact">
          <span>Volume</span>
          <strong>
            {spotifyStatus?.currentVolumePercent != null
              ? `${spotifyStatus.currentVolumePercent}%`
              : "Unavailable"}
          </strong>
        </div>
        <div class="fact">
          <span>Saved</span>
          <strong>
            {spotifyStatus?.isCurrentTrackSaved == null
              ? "Unavailable"
              : spotifyStatus.isCurrentTrackSaved
                ? "Saved"
                : "Not saved"}
          </strong>
        </div>
      </div>
    {:else}
      <p class="settings-lead">
        Tap a scene to preview it. Auto switch opens that view when the app is detected. Teams only
        switches when a Teams window is open. Spotify is preferred when a token is saved; otherwise
        the system media player is used.
      </p>
      {#if sceneIds.length === 0}
        <p class="settings-status">No scenes loaded yet.</p>
      {:else}
        <div class="scene-grid">
          {#each sceneIds as id (id)}
            {@const plugin = pluginsById.get(id)}
            {@const meta = getBuiltinSceneMeta(id)}
            {@const title = plugin?.name ?? meta?.name ?? id}
            {@const description =
              plugin?.description ??
              meta?.description ??
              "Manual scene override for this plugin layout."}
            {@const grid = plugin?.layout.grid ?? meta?.layout.grid ?? [0, 0]}
            {@const buttonCount = plugin?.layout.buttons.length ?? meta?.layout.buttons.length ?? 0}
            <article
              class="scene-card"
              class:active={id === sceneId}
            >
              <button
                type="button"
                class="scene-card-select"
                onclick={() => onSelectScene(id)}
                aria-pressed={id === sceneId}
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
              <p class="scene-description">{description}</p>
              <div class="scene-card-footer">
                <span>{grid[0]}×{grid[1]}</span>
                <span>{buttonCount} buttons</span>
              </div>
              </button>
              <label class="setting-row scene-auto" for={`auto-switch-${id}`}>
                <div class="setting-copy">
                  <span class="setting-title">Auto switch</span>
                  <span class="setting-desc">
                    {#if id === "teams"}
                      Switch here when a Teams window is open.
                    {:else if id === "spotify"}
                      Switch here when Spotify is running and a token is saved.
                    {:else if id === "media"}
                      Switch here for other local media players.
                    {:else}
                      Switch here when this app is detected.
                    {/if}
                  </span>
                </div>
                <input
                  id={`auto-switch-${id}`}
                  type="checkbox"
                  checked={autoSwitchScenes[id] !== false}
                  disabled={!onAutoSwitchChange || autoSwitchBusyId === id}
                  onchange={(event) =>
                    onAutoSwitchChange?.(
                      id,
                      (event.currentTarget as HTMLInputElement).checked
                    )}
                />
                <span class="md-check" aria-hidden="true"></span>
              </label>
            </article>
          {/each}
        </div>
      {/if}
    {/if}
    </section>
  </div>
  <PlaylistBrowser open={playlistsOpen} onClose={() => (playlistsOpen = false)} />
</div>

<style>
  .settings-shell {
    --md-bg: #eceef2;
    --md-surface: #ffffff;
    --md-ink: #111827;
    --md-muted: #1e293b;
    --md-line: rgba(17, 24, 39, 0.14);
    --md-shadow: 0 1px 2px rgba(17, 24, 39, 0.04), 0 14px 40px rgba(17, 24, 39, 0.06);
    --md-ease: background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease,
      box-shadow 0.15s ease, transform 0.15s ease;
    --md-sidebar: #0d0d0d;
    --md-sidebar-text: #d4d4d4;
    --md-sidebar-muted: #a3a3a3;
    --md-sidebar-hover: #1a1a1a;
    --md-sidebar-active: #262626;
    --md-sidebar-strong: #fff;
    --md-blue: #2563eb;
    --md-blue-hover: #1d4ed8;
    --md-blue-soft: rgba(37, 99, 235, 0.16);
    --md-danger: #dc2626;
    --md-row-hover: #f6f7f9;
    --md-chip: #e8eaee;
    --md-chip-hover: #d7dbe3;
    --md-slider-track: #d5d9e1;
    --md-slider-fill: #111827;
    --md-slider-thumb: #fff;
    --md-slider-thumb-border: #111827;
    --md-check-border: #9aa3b2;
    --md-check-border-hover: #64748b;
    --md-check-bg: #fff;
    --md-code-bg: #f3f4f6;
    --md-btn: #c5cad3;
    --md-btn-hover: #b4bac4;
    --md-btn-primary: #111111;
    --md-btn-primary-ink: #fff;
    --md-btn-primary-hover: #2a2a2a;
    --md-input-border: #c5cad3;
    --md-input-border-hover: #9aa3b2;
    --md-input-bg: #fff;
    --md-input-disabled: #f8f9fb;
    --md-error: #b91c1c;
    --md-ok: #15803d;
    --md-scene-border: #c5cad3;
    --md-scene-hover: #f7f8fa;
    --md-scene-active: #eef0f3;
    --md-scene-active-hover: #e7eaee;
    --md-tag-bg: #94a3b8;
    --md-tag-ink: #0f172a;
    --md-tag-current: #111111;
    --md-tag-current-ink: #fff;
    --md-inset: #f3f4f6;
    --md-ease: background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease,
      box-shadow 0.15s ease, transform 0.15s ease;
    color-scheme: light;
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(220px, 248px) 1fr;
    background: var(--md-bg);
    color: var(--md-ink);
    user-select: text;
    -webkit-user-select: text;
  }

  .settings-shell.theme-dark {
    --md-bg: #0b0c10;
    --md-surface: #16171d;
    --md-ink: #e8e8f0;
    --md-muted: #a8aec0;
    --md-line: rgba(255, 255, 255, 0.1);
    --md-shadow: 0 1px 2px rgba(0, 0, 0, 0.24), 0 14px 40px rgba(0, 0, 0, 0.32);
    --md-sidebar: #08090c;
    --md-sidebar-text: #c5c8d4;
    --md-sidebar-muted: #8b90a0;
    --md-sidebar-hover: #15161c;
    --md-sidebar-active: #22232c;
    --md-sidebar-strong: #fff;
    --md-blue: #3b82f6;
    --md-blue-hover: #2563eb;
    --md-blue-soft: rgba(59, 130, 246, 0.22);
    --md-danger: #f87171;
    --md-row-hover: rgba(255, 255, 255, 0.04);
    --md-chip: #1e1f27;
    --md-chip-hover: #2a2b35;
    --md-slider-track: #2a2c36;
    --md-slider-fill: #e8e8f0;
    --md-slider-thumb: #111318;
    --md-slider-thumb-border: #e8e8f0;
    --md-check-border: #6b7280;
    --md-check-border-hover: #9ca3af;
    --md-check-bg: #1a1b22;
    --md-code-bg: #22232c;
    --md-btn: #2a2c36;
    --md-btn-hover: #363846;
    --md-btn-primary: #e8e8f0;
    --md-btn-primary-ink: #111318;
    --md-btn-primary-hover: #f4f4f8;
    --md-input-border: #3a3d4a;
    --md-input-border-hover: #525566;
    --md-input-bg: #1a1b22;
    --md-input-disabled: #121318;
    --md-error: #fca5a5;
    --md-ok: #4ade80;
    --md-scene-border: #3a3d4a;
    --md-scene-hover: #1e1f27;
    --md-scene-active: #25262f;
    --md-scene-active-hover: #2c2d38;
    --md-tag-bg: #4b5563;
    --md-tag-ink: #f8fafc;
    --md-tag-current: #e8e8f0;
    --md-tag-current-ink: #111318;
    --md-inset: #121318;
    color-scheme: dark;
  }

  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 20px 12px 16px;
    background: var(--md-sidebar);
    color: var(--md-sidebar-text);
  }

  .settings-brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 10px 16px;
  }

  .settings-brand-icon {
    width: 36px;
    height: 36px;
    object-fit: contain;
  }

  .settings-brand strong {
    display: block;
    font-size: 1rem;
    font-weight: 600;
    color: var(--md-sidebar-strong);
  }

  .settings-brand span {
    display: block;
    color: var(--md-sidebar-muted);
    font-size: 0.8rem;
  }

  .settings-nav-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .settings-nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    padding: 12px 14px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--md-sidebar-text);
    font-size: 0.95rem;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .settings-nav-item svg {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .settings-nav-item:hover {
    background: var(--md-sidebar-hover);
    color: var(--md-sidebar-strong);
  }

  .settings-nav-item.active {
    background: var(--md-sidebar-active);
    color: var(--md-sidebar-strong);
  }

  .settings-nav-badge {
    margin-left: auto;
    min-width: 20px;
    padding: 1px 6px;
    border-radius: 6px;
    background: var(--md-blue);
    color: var(--md-sidebar-strong);
    font-size: 0.72rem;
    font-weight: 600;
    text-align: center;
  }

  .settings-back {
    margin-top: auto;
    min-height: 44px;
    padding: 12px 14px;
    border: none;
    border-radius: 8px;
    background: var(--md-sidebar-hover);
    color: var(--md-sidebar-text);
    font-weight: 500;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .settings-back:hover {
    background: var(--md-sidebar-active);
    color: var(--md-sidebar-strong);
  }

  .settings-main {
    min-width: 0;
    overflow: auto;
    padding: 20px 24px 24px;
  }

  .settings-content {
    min-height: calc(100% - 0px);
    padding: 28px 32px 36px;
    background: var(--md-surface);
    border: 1px solid var(--md-line);
    border-radius: 16px;
    box-shadow: var(--md-shadow);
  }

  .settings-content-head h1 {
    margin: 0 0 22px;
    font-size: 1.45rem;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--md-ink);
  }

  .settings-back-inline {
    display: none;
    margin: 0 0 12px;
    min-height: 44px;
    padding: 10px 14px;
    border: none;
    border-radius: 8px;
    background: var(--md-chip);
    color: var(--md-ink);
    font-weight: 500;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .settings-back-inline:hover {
    background: var(--md-chip-hover);
  }

  .settings-lead {
    margin: -8px 0 16px;
    color: var(--md-muted);
    font-size: 0.95rem;
  }

  .settings-card {
    background: var(--md-surface);
    border: 1px solid var(--md-line);
    border-radius: 10px;
    overflow: hidden;
    margin-bottom: 14px;
  }

  .settings-card-pad {
    padding: 18px;
  }

  .settings-card-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .setting-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 72px;
    padding: 18px 20px;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .setting-row:hover:not(:has(input:disabled)) {
    background: var(--md-row-hover);
  }

  .setting-row + .setting-row {
    box-shadow: inset 0 1px 0 var(--md-line);
  }

  .setting-row-slider {
    cursor: default;
    align-items: stretch;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
  }

  .setting-row-slider.setting-row-disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  .setting-row.setting-row-disabled {
    opacity: 0.55;
  }

  .setting-colour-toggle {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    cursor: pointer;
  }

  .setting-row-theme {
    flex-wrap: wrap;
    gap: 14px 18px;
  }

  .theme-seg {
    display: flex;
    flex: 0 0 auto;
    padding: 4px;
    border-radius: 10px;
    background: var(--md-chip);
    border: 1px solid var(--md-line);
  }

  .theme-seg button {
    min-height: 36px;
    padding: 8px 14px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--md-muted);
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .theme-seg button:hover {
    color: var(--md-ink);
  }

  .theme-seg button.active {
    background: var(--md-surface);
    color: var(--md-ink);
    box-shadow: 0 1px 2px rgba(17, 24, 39, 0.12);
  }

  .md-slider {
    display: grid;
    grid-template-columns: 4.2rem minmax(0, 1fr);
    align-items: center;
    gap: 14px;
    width: 100%;
  }

  .md-slider-value {
    font-size: 0.95rem;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    color: var(--md-ink);
    text-align: right;
  }

  .md-slider-ui {
    position: relative;
    height: 32px;
    display: flex;
    align-items: center;
  }

  .md-slider-track,
  .md-slider-fill {
    position: absolute;
    left: 0;
    height: 8px;
    border-radius: 999px;
    pointer-events: none;
  }

  .md-slider-track {
    right: 0;
    background: var(--md-slider-track);
    box-shadow: inset 0 1px 2px rgba(17, 24, 39, 0.12);
  }

  .md-slider-fill {
    background: var(--md-slider-fill);
  }

  .md-slider-ui input[type="range"] {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 32px;
    margin: 0;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  .md-slider-ui input[type="range"]:focus-visible {
    outline: none;
  }

  .md-slider-ui:has(input[type="range"]:focus-visible) {
    border-radius: 10px;
    outline: 2px solid var(--md-blue);
    outline-offset: 4px;
  }

  .md-slider-ui input[type="range"]::-webkit-slider-runnable-track {
    height: 8px;
    background: transparent;
  }

  .md-slider-ui input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    width: 22px;
    height: 22px;
    margin-top: -7px;
    border-radius: 50%;
    background: var(--md-slider-thumb);
    border: 2px solid var(--md-slider-thumb-border);
    box-shadow: 0 2px 8px rgba(17, 24, 39, 0.22);
  }

  .md-slider-ui input[type="range"]::-moz-range-track {
    height: 8px;
    background: transparent;
    border: none;
  }

  .md-slider-ui input[type="range"]::-moz-range-thumb {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--md-slider-thumb);
    border: 2px solid var(--md-slider-thumb-border);
    box-shadow: 0 2px 8px rgba(17, 24, 39, 0.22);
  }

  .setting-copy {
    flex: 1 1 auto;
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .setting-title {
    font-size: 0.98rem;
    font-weight: 600;
    color: var(--md-ink);
  }

  .setting-desc {
    margin: 4px 0 0;
    color: var(--md-muted);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .settings-version {
    margin: 6px 0 0;
    color: var(--md-ink);
    font-size: 0.95rem;
  }

  .settings-version strong {
    font-weight: 600;
  }

  .setting-desc code {
    font-size: 0.8em;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--md-code-bg);
  }

  .setting-row input[type="checkbox"] {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }

  .md-check {
    position: relative;
    flex: 0 0 auto;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    border: 2px solid var(--md-check-border);
    background: var(--md-check-bg);
    transition: background-color 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .setting-row:hover:not(:has(input:disabled)) .md-check {
    border-color: var(--md-check-border-hover);
    box-shadow: 0 0 0 4px rgba(37, 99, 235, 0.08);
  }

  .md-check::after {
    content: "";
    position: absolute;
    inset: 0;
    background: center / 14px 14px no-repeat
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath fill='none' stroke='%23fff' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round' d='M3.5 8.5 6.5 11.5 12.5 4.8'/%3E%3C/svg%3E");
    transform: scale(0);
    transform-origin: center;
    transition: transform 0.12s ease;
  }

  .setting-row input[type="checkbox"]:checked + .md-check {
    background: var(--md-blue);
    border-color: var(--md-blue);
  }

  .setting-row:hover:not(:has(input:disabled)) input:checked + .md-check {
    background: var(--md-blue-hover);
    border-color: var(--md-blue-hover);
  }

  .setting-row input[type="checkbox"]:checked + .md-check::after {
    transform: scale(1);
  }

  .setting-row input[type="checkbox"]:disabled + .md-check {
    opacity: 0.55;
  }

  .setting-row:has(input:focus-visible) .md-check {
    outline: 2px solid var(--md-blue);
    outline-offset: 2px;
  }

  .setting-row:has(input:disabled) {
    cursor: default;
  }

  .md-btn {
    min-height: 44px;
    padding: 12px 18px;
    border-radius: 8px;
    border: none;
    background: var(--md-btn);
    color: var(--md-ink);
    font-size: 0.95rem;
    font-weight: 500;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .md-btn:hover:not(:disabled) {
    background: var(--md-btn-hover);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(17, 24, 39, 0.08);
  }

  .md-btn:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: none;
  }

  .md-btn-primary {
    background: var(--md-btn-primary);
    color: var(--md-btn-primary-ink);
  }

  .md-btn-primary:hover:not(:disabled) {
    background: var(--md-btn-primary-hover);
    box-shadow: 0 6px 16px rgba(17, 24, 39, 0.22);
  }

  .md-btn-danger {
    background: var(--md-danger);
    color: #fff;
  }

  .md-btn-danger:hover:not(:disabled) {
    background: var(--md-danger);
    color: #fff;
    filter: brightness(1.08);
  }

  .md-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .md-input {
    flex: 1 1 220px;
    min-width: 0;
    padding: 12px 14px;
    border-radius: 8px;
    border: 1px solid var(--md-input-border);
    background: var(--md-input-bg);
    color: var(--md-ink);
    font-size: 0.95rem;
    min-height: 44px;
    transition: var(--md-ease);
  }

  .md-input:hover:not(:disabled) {
    border-color: var(--md-input-border-hover);
  }

  .md-input:focus {
    outline: none;
    border-color: var(--md-blue);
    box-shadow: 0 0 0 3px var(--md-blue-soft);
  }

  .md-input:disabled {
    cursor: not-allowed;
    background: var(--md-input-disabled);
  }

  .settings-field-row,
  .settings-btn-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin-top: 12px;
  }

  .auth-mode-list {
    display: grid;
    gap: 10px;
    margin-top: 14px;
  }

  .auth-mode-card {
    appearance: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    padding: 14px 16px;
    border: 1px solid var(--md-line);
    border-radius: 10px;
    background: var(--md-surface);
    color: var(--md-ink);
    text-align: left;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .auth-mode-card:hover:not(:disabled) {
    background: var(--md-row-hover);
    border-color: var(--md-input-border-hover);
  }

  .auth-mode-card.active {
    background: var(--md-scene-active);
    border-color: var(--md-input-border-hover);
  }

  .auth-mode-card:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .auth-mode-card .setting-desc {
    margin: 0;
  }

  .auth-mode-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .settings-error {
    margin: 10px 0 0;
    color: var(--md-error);
    font-size: 0.9rem;
  }

  .settings-status {
    margin: 14px 0 0;
    color: var(--md-muted);
    font-size: 0.95rem;
  }

  .settings-hint {
    margin: 10px 0 0;
    color: var(--md-error);
    font-size: 0.88rem;
  }

  .settings-note {
    margin: 0;
    padding: 0 18px 16px;
    color: var(--md-muted);
    font-size: 0.88rem;
  }

  .settings-card .settings-error {
    margin: 0;
    padding: 0 18px 16px;
  }

  .settings-hint.ok {
    color: var(--md-ok);
  }

  .update-available {
    display: grid;
    gap: 12px;
    margin-top: 16px;
    padding-top: 16px;
    box-shadow: inset 0 1px 0 var(--md-line);
    --text-primary: var(--md-ink);
    --text-secondary: var(--md-muted);
    --bg-primary: var(--md-inset);
    --border-subtle: var(--md-line);
    --accent: var(--md-blue);
  }

  .spotify-status-line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
    color: var(--md-muted);
    font-size: 0.9rem;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: #f59e0b;
  }

  .status-dot.on {
    background: #16a34a;
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .cover {
    width: 64px;
    height: 64px;
    border-radius: 10px;
    object-fit: cover;
  }

  .now-playing-state {
    margin: 0 0 4px;
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--md-muted);
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  }

  .fact {
    display: grid;
    gap: 4px;
    padding: 14px 18px;
  }

  .fact + .fact {
    box-shadow: inset 1px 0 0 var(--md-line);
  }

  .fact span {
    color: var(--md-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .fact strong {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .scene-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 12px;
    margin-bottom: 22px;
  }

  .scene-card {
    appearance: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 18px;
    border: 1px solid var(--md-scene-border);
    border-radius: 10px;
    background: var(--md-surface);
    text-align: left;
    color: var(--md-ink);
    min-height: 120px;
    transition: var(--md-ease);
  }

  .scene-card-select {
    appearance: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0;
    border: none;
    background: transparent;
    text-align: left;
    color: inherit;
    cursor: pointer;
  }

  .scene-auto {
    margin-top: 4px;
    padding-top: 12px;
    border-top: 1px solid var(--md-line);
  }

  .scene-auto .setting-desc {
    min-height: 0;
  }

  .scene-card:hover {
    background: var(--md-scene-hover);
    border-color: var(--md-input-border-hover);
    transform: translateY(-2px);
    box-shadow: 0 8px 20px rgba(17, 24, 39, 0.08);
  }

  .scene-card:active {
    transform: translateY(0);
    box-shadow: none;
  }

  .scene-card.active {
    background: var(--md-scene-active);
    border-color: var(--md-input-border-hover);
  }

  .scene-card.active:hover {
    background: var(--md-scene-active-hover);
  }

  .scene-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .scene-name {
    font-weight: 600;
  }

  .scene-id {
    font-size: 0.75rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--md-muted);
  }

  .scene-description {
    margin: 0;
    min-height: 2.5em;
    color: var(--md-muted);
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .scene-card-footer {
    display: flex;
    gap: 8px;
    color: var(--md-muted);
    font-size: 0.78rem;
  }

  .scene-tag {
    font-size: 0.78rem;
    padding: 7px 14px;
    border-radius: 999px;
    background: var(--md-tag-bg);
    color: var(--md-tag-ink);
    font-weight: 600;
    line-height: 1.2;
  }

  .scene-tag.current {
    background: var(--md-tag-current);
    color: var(--md-tag-current-ink);
  }

  @media (max-width: 720px) {
    .settings-shell {
      grid-template-columns: 1fr;
    }

    .settings-nav {
      color: var(--md-sidebar-text);
    }

    .settings-nav-list {
      flex-direction: row;
      overflow-x: auto;
    }

    .settings-back {
      display: none;
    }

    .settings-back-inline {
      display: inline-flex;
    }

    .fact + .fact {
      box-shadow: inset 0 1px 0 var(--md-line);
    }

    .theme-seg {
      width: 100%;
    }

    .theme-seg button {
      flex: 1 1 0;
    }
  }

  .md-btn:focus-visible,
  .theme-seg button:focus-visible,
  .scene-card:focus-visible,
  .settings-nav-item:focus-visible,
  .settings-back:focus-visible,
  .settings-back-inline:focus-visible {
    outline: 2px solid var(--md-blue);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .settings-nav-item,
    .settings-back,
    .settings-back-inline,
    .theme-seg,
    .theme-seg button,
    .md-btn,
    .md-input,
    .md-check,
    .md-check::after,
    .md-slider-fill,
    .md-slider-ui input[type="range"]::-webkit-slider-thumb,
    .scene-card,
    .setting-row {
      transition: none;
    }

    .md-btn:hover:not(:disabled),
    .scene-card:hover,
    .scene-card:active {
      transform: none;
    }
  }
</style>
