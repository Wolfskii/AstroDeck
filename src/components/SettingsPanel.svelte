<script lang="ts">
  import ReleaseNotes from "./ReleaseNotes.svelte";
  import appIconUrl from "../assets/app-icon.png";
  import { getBuiltinSceneMeta } from "../layouts/layouts";
  import type { PluginConfig } from "../types";
  import type { AppUpdateInfo } from "../services/updater";
  import type { SpotifyStatus } from "../services/api";

  type SettingsSection = "general" | "updates" | "spotify" | "scenes";

  let {
    isTauri,
    onBackToDeck,
    startOnBoot,
    startOnBootBusy,
    startMinimized,
    startMinimizedBusy,
    startupError,
    onStartOnBootChange,
    onStartMinimizedChange,
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
    spotifyClientLockedByEnv,
    spotifySavingClientId,
    onSaveSpotifyClientId,
    onOpenSpotifyDashboard,
    onConnectSpotify,
    onDisconnectSpotify,
    sceneIds,
    pluginsById,
    sceneId,
    seenScenes,
    onSelectScene,
  }: {
    isTauri: boolean;
    onBackToDeck?: () => void;
    startOnBoot: boolean;
    startOnBootBusy: boolean;
    startMinimized: boolean;
    startMinimizedBusy: boolean;
    startupError: string | null;
    onStartOnBootChange: (event: Event) => void;
    onStartMinimizedChange: (event: Event) => void;
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
    spotifyClientLockedByEnv: boolean;
    spotifySavingClientId: boolean;
    onSaveSpotifyClientId: () => void;
    onOpenSpotifyDashboard: () => void;
    onConnectSpotify: () => void;
    onDisconnectSpotify: () => void;
    sceneIds: string[];
    pluginsById: Map<string, PluginConfig>;
    sceneId: string;
    seenScenes: string[];
    onSelectScene: (id: string) => void;
  } = $props();

  let section = $state<SettingsSection>("general");
  const activeSection = $derived(section);

  const titles: Record<SettingsSection, string> = {
    general: "General",
    updates: "Updates",
    spotify: "Spotify",
    scenes: "Scenes",
  };

  const updateBadge = $derived(installableUpdate ? "1" : null);
</script>

<div class="settings-shell">
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
              Keep the window hidden until you open it. Turn this off to restore the last display,
              position, size, and maximized or fullscreen state.
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
      </div>
      {#if startupError}
        <p class="settings-error">{startupError}</p>
      {/if}
    {:else if activeSection === "updates"}
      <div class="settings-card settings-card-pad">
        <div class="settings-card-toolbar">
          <div>
            <p class="setting-title">Software updates</p>
            <p class="setting-desc">
              {#if appVersion}Current version {appVersion}. {/if}
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
              Connect once to enable track liking and Spotify-only volume control.
            </p>
          </div>
          <div class="settings-btn-row">
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

      {#if isTauri}
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
      <p class="settings-lead">Choose a scene to manually override the active desktop deck.</p>
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
            <button
              type="button"
              class="scene-card"
              class:active={id === sceneId}
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
          {/each}
        </div>
      {/if}
    {/if}
    </section>
  </div>
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
    --md-blue: #2563eb;
    --md-danger: #dc2626;
    flex: 1 1 auto;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(220px, 248px) 1fr;
    background: var(--md-bg);
    color: var(--md-ink);
    user-select: text;
    -webkit-user-select: text;
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
    border-radius: 10px;
  }

  .settings-brand strong {
    display: block;
    font-size: 1rem;
    font-weight: 600;
    color: #fff;
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
    color: #fff;
  }

  .settings-nav-item.active {
    background: var(--md-sidebar-active);
    color: #fff;
  }

  .settings-nav-badge {
    margin-left: auto;
    min-width: 20px;
    padding: 1px 6px;
    border-radius: 6px;
    background: var(--md-blue);
    color: #fff;
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
    color: #fff;
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
    background: #e8eaee;
    color: var(--md-ink);
    font-weight: 500;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .settings-back-inline:hover {
    background: #d7dbe3;
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
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 72px;
    padding: 18px 20px;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .setting-row:hover:not(:has(input:disabled)) {
    background: #f6f7f9;
  }

  .setting-row + .setting-row {
    box-shadow: inset 0 1px 0 var(--md-line);
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

  .setting-desc code {
    font-size: 0.8em;
    padding: 1px 5px;
    border-radius: 4px;
    background: #f3f4f6;
  }

  .setting-row input {
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
    border: 2px solid #9aa3b2;
    background: #fff;
    transition: background-color 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .setting-row:hover:not(:has(input:disabled)) .md-check {
    border-color: #64748b;
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

  .setting-row input:checked + .md-check {
    background: var(--md-blue);
    border-color: var(--md-blue);
  }

  .setting-row:hover:not(:has(input:disabled)) input:checked + .md-check {
    background: #1d4ed8;
    border-color: #1d4ed8;
  }

  .setting-row input:checked + .md-check::after {
    transform: scale(1);
  }

  .setting-row input:disabled + .md-check {
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
    background: #e8eaee;
    color: var(--md-ink);
    font-size: 0.95rem;
    font-weight: 500;
    cursor: pointer;
    transition: var(--md-ease);
  }

  .md-btn:hover:not(:disabled) {
    background: #d7dbe3;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(17, 24, 39, 0.08);
  }

  .md-btn:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: none;
  }

  .md-btn-primary {
    background: #111111;
    color: #fff;
  }

  .md-btn-primary:hover:not(:disabled) {
    background: #2a2a2a;
    box-shadow: 0 6px 16px rgba(17, 24, 39, 0.22);
  }

  .md-btn-danger {
    background: #ef4444;
    color: #fff;
  }

  .md-btn-danger:hover:not(:disabled) {
    background: #dc2626;
    color: #fff;
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
    border: 1px solid #c5cad3;
    background: #fff;
    color: var(--md-ink);
    font-size: 0.95rem;
    min-height: 44px;
    transition: var(--md-ease);
  }

  .md-input:hover:not(:disabled) {
    border-color: #9aa3b2;
  }

  .md-input:focus {
    outline: none;
    border-color: var(--md-blue);
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.16);
  }

  .md-input:disabled {
    cursor: not-allowed;
    background: #f8f9fb;
  }

  .settings-field-row,
  .settings-btn-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin-top: 12px;
  }

  .settings-error {
    margin: 10px 0 0;
    color: #b91c1c;
    font-size: 0.9rem;
  }

  .settings-status {
    margin: 14px 0 0;
    color: var(--md-muted);
    font-size: 0.95rem;
  }

  .settings-hint {
    margin: 10px 0 0;
    color: #b91c1c;
    font-size: 0.88rem;
  }

  .settings-hint.ok {
    color: #15803d;
  }

  .update-available {
    display: grid;
    gap: 12px;
    margin-top: 16px;
    padding-top: 16px;
    box-shadow: inset 0 1px 0 var(--md-line);
    --text-primary: #111827;
    --text-secondary: #1e293b;
    --bg-primary: #f3f4f6;
    --border-subtle: rgba(17, 24, 39, 0.08);
    --accent: #2563eb;
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
  }

  .scene-card {
    appearance: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 18px;
    border: 1px solid #c5cad3;
    border-radius: 10px;
    background: var(--md-surface);
    text-align: left;
    color: var(--md-ink);
    cursor: pointer;
    min-height: 120px;
    transition: var(--md-ease);
  }

  .scene-card:hover {
    background: #f7f8fa;
    border-color: #9aa3b2;
    transform: translateY(-2px);
    box-shadow: 0 8px 20px rgba(17, 24, 39, 0.08);
  }

  .scene-card:active {
    transform: translateY(0);
    box-shadow: none;
  }

  .scene-card.active {
    background: #eef0f3;
    border-color: #9aa3b2;
  }

  .scene-card.active:hover {
    background: #e7eaee;
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
    background: #94a3b8;
    color: #0f172a;
    font-weight: 600;
    line-height: 1.2;
  }

  .scene-tag.current {
    background: #111111;
    color: #fff;
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
  }

  .md-btn:focus-visible,
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
    .md-btn,
    .md-input,
    .md-check,
    .md-check::after,
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
