<script lang="ts">
  import "@fontsource/dseg7-classic/400.css";
  import appIconUrl from "../assets/app-icon.png";
  import type { DeckButtonConfig } from "../types";
  import { executeAction, executeActionValue, getSpotifyLyrics, type SpotifyTrackLyrics } from "../services/api";
  import PlaylistBrowser from "./PlaylistBrowser.svelte";
  import { logError } from "../services/logger";
  import SceneBackground from "./SceneBackground.svelte";
  import { sceneBackgroundId, controlsBackdropEnabled, controlsTransparency, controlsOverlayColor, controlsOverlayCustom } from "../stores/appearance";
  import { DEFAULT_CONTROLS_OVERLAY_COLOR, hexToRgbCss } from "../lib/color";
  import { usesCoverImage, usesFullViewBackground } from "../lib/sceneBackgrounds";
  import {
    DEFAULT_CAR_BACKGROUNDS,
    DEFAULT_SHADER_COLORS,
    extractAlbumPalette,
    paletteToCarThingBackgrounds,
    paletteToShaderColors,
  } from "../lib/albumArtColor";
  import { untrack } from "svelte";

  interface Props {
    previous?: DeckButtonConfig | null;
    playPause?: DeckButtonConfig | null;
    next?: DeckButtonConfig | null;
    like?: DeckButtonConfig | null;
    shuffle?: DeckButtonConfig | null;
    title?: string | null;
    subtitle?: string | null;
    albumName?: string | null;
    artworkUrl?: string | null;
    playbackState?: string;
    progressMs?: number | null;
    durationMs?: number | null;
    volumeAction?: string | null;
    seekAction?: string | null;
    volumePercent?: number;
    volumeBusy?: boolean;
    volumeEnabled?: boolean;
    seekEnabled?: boolean;
    shuffleActive?: boolean;
    trackSaved?: boolean | null;
    lyricsEnabled?: boolean;
    trackId?: string | null;
    source?: string;
    onVolumeCommit?: (value: number) => Promise<void> | void;
    onSeekCommit?: (positionMs: number) => Promise<void> | void;
    showSettingsButton?: boolean;
    onOpenSettings?: () => void;
    playlistsEnabled?: boolean;
  }

  let {
    previous = null,
    playPause = null,
    next = null,
    like = null,
    shuffle = null,
    title = null,
    subtitle = null,
    albumName = null,
    artworkUrl = null,
    playbackState = "stopped",
    progressMs = null,
    durationMs = null,
    volumeAction = null,
    seekAction = null,
    volumePercent = 50,
    volumeBusy = false,
    volumeEnabled = true,
    seekEnabled = true,
    shuffleActive = false,
    trackSaved = null,
    lyricsEnabled = false,
    trackId = null,
    source = "media window",
    onVolumeCommit,
    onSeekCommit,
    showSettingsButton = false,
    onOpenSettings,
    playlistsEnabled = false,
  }: Props = $props();

  let localVolume = $state(50);
  let playbackDisplayMs = $state(0);
  let scrubMs = $state(0);
  let progressDragging = $state(false);
  let progressHovering = $state(false);
  let progressTrackEl = $state<HTMLElement | null>(null);
  let faderTrackEl = $state<HTMLElement | null>(null);
  let faderSlotEl = $state<HTMLElement | null>(null);
  let faderDragging = $state(false);
  const FADER_THUMB_HEIGHT_PX = 78;
  const FADER_THUMB_HALF_PX = FADER_THUMB_HEIGHT_PX / 2;
  let seekHoldMs = $state<number | null>(null);
  let progressTrackStamp = $state("");
  let carBodyBg = $state(DEFAULT_CAR_BACKGROUNDS.body);
  let carFooterBg = $state(DEFAULT_CAR_BACKGROUNDS.footer);
  let shaderColors = $state<string[]>([...DEFAULT_SHADER_COLORS]);
  const backgroundStyle = $derived($sceneBackgroundId);
  const showFullViewShader = $derived(usesFullViewBackground(backgroundStyle));
  const showArtBorder = $derived(backgroundStyle === "pulsing-border");
  const showControlsBackdrop = $derived(showFullViewShader && $controlsBackdropEnabled);
  const controlsOverlayAlpha = $derived(
    showControlsBackdrop ? (100 - $controlsTransparency) / 100 : 1
  );
  const controlsOverlayRgb = $derived(
    hexToRgbCss($controlsOverlayCustom ? $controlsOverlayColor : DEFAULT_CONTROLS_OVERLAY_COLOR)
  );

  const isPlaying = $derived(playbackState === "playing");
  const hasDuration = $derived((durationMs ?? 0) > 0);
  const playbackRatio = $derived(
    hasDuration
      ? Math.min(1, Math.max(0, playbackDisplayMs / (durationMs as number)))
      : 0
  );
  const scrubRatio = $derived(
    hasDuration ? Math.min(1, Math.max(0, scrubMs / (durationMs as number))) : 0
  );
  const progressInteracting = $derived(progressHovering || progressDragging);
  const progressAriaValue = $derived(progressInteracting ? scrubMs : playbackDisplayMs);
  const showProgressPreview = $derived(progressInteracting && scrubRatio > playbackRatio);
  const progressPreviewWidth = $derived(Math.max(0, (scrubRatio - playbackRatio) * 100));
  const faderThumbTop = $derived(
    `calc(${FADER_THUMB_HALF_PX}px + ${(100 - localVolume) / 100} * (100% - ${FADER_THUMB_HEIGHT_PX}px))`
  );

  $effect(() => {
    if (faderDragging || volumeBusy) return;
    localVolume = volumePercent;
  });

  const PROGRESS_SYNC_SLACK_MS = 4000;
  const PROGRESS_SEEK_BACK_MS = 5000;

  $effect(() => {
    if (progressDragging || progressMs == null) return;
    const incoming = progressMs;
    const trackStamp = `${title ?? ""}\0${albumName ?? ""}`;
    const displayed = untrack(() => playbackDisplayMs);
    const hold = untrack(() => seekHoldMs);
    const previousStamp = untrack(() => progressTrackStamp);
    const playingNow = untrack(() => isPlaying);

    if (previousStamp !== trackStamp) {
      progressTrackStamp = trackStamp;
      seekHoldMs = null;
      playbackDisplayMs = incoming;
      return;
    }

    if (hold != null) {
      if (Math.abs(incoming - hold) <= 2500) {
        seekHoldMs = null;
        return;
      }
      if (Math.abs(incoming - displayed) <= PROGRESS_SYNC_SLACK_MS) return;
      seekHoldMs = null;
    }

    if (displayed >= 2500 && incoming <= 1500) {
      playbackDisplayMs = incoming;
      return;
    }

    const delta = incoming - displayed;
    if (playingNow) {
      if (delta >= PROGRESS_SYNC_SLACK_MS || delta <= -PROGRESS_SEEK_BACK_MS) {
        playbackDisplayMs = incoming;
      }
      return;
    }

    if (Math.abs(delta) <= PROGRESS_SYNC_SLACK_MS) return;
    playbackDisplayMs = incoming;
  });

  $effect(() => {
    if (!isPlaying || progressDragging || !hasDuration) return;
    const duration = durationMs as number;
    let last = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      const dt = now - last;
      last = now;
      playbackDisplayMs = Math.min(duration, playbackDisplayMs + dt);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  function applyOptimisticSeek(positionMs: number) {
    seekHoldMs = positionMs;
    playbackDisplayMs = positionMs;
    scrubMs = positionMs;
  }

  function updateScrubFromClientX(clientX: number) {
    if (!progressTrackEl || !hasDuration) return;
    const rect = progressTrackEl.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    scrubMs = Math.round(ratio * (durationMs as number));
  }

  function volumeFromClientY(clientY: number): number {
    if (!faderSlotEl) return localVolume;
    const rect = faderSlotEl.getBoundingClientRect();
    const travel = Math.max(1, rect.height - FADER_THUMB_HEIGHT_PX);
    const y = clientY - rect.top - FADER_THUMB_HALF_PX;
    const ratio = 1 - Math.min(1, Math.max(0, y / travel));
    return Math.round(ratio * 100);
  }

  function handleProgressPointerDown(event: PointerEvent) {
    if (!seekAction || !onSeekCommit || !seekEnabled || !hasDuration) return;
    progressDragging = true;
    progressHovering = true;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    updateScrubFromClientX(event.clientX);
    applyOptimisticSeek(scrubMs);
  }

  function handleProgressPointerMove(event: PointerEvent) {
    if (!progressHovering && !progressDragging) return;
    updateScrubFromClientX(event.clientX);
    if (progressDragging) {
      applyOptimisticSeek(scrubMs);
    }
  }

  function handleProgressPointerUp(event: PointerEvent) {
    const wasDragging = progressDragging;
    if (wasDragging) {
      progressDragging = false;
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
      applyOptimisticSeek(scrubMs);
      if (seekAction && onSeekCommit && seekEnabled) {
        void onSeekCommit(scrubMs);
      }
    }

    const el = progressTrackEl;
    if (el) {
      const rect = el.getBoundingClientRect();
      progressHovering =
        event.clientX >= rect.left &&
        event.clientX <= rect.right &&
        event.clientY >= rect.top &&
        event.clientY <= rect.bottom;
    } else {
      progressHovering = false;
    }
  }

  function handleProgressPointerEnter(event: PointerEvent) {
    if (!seekEnabled || !hasDuration) return;
    progressHovering = true;
    updateScrubFromClientX(event.clientX);
  }

  function handleProgressPointerLeave() {
    if (!progressDragging) progressHovering = false;
  }

  function handleFaderPointerDown(event: PointerEvent) {
    if (!volumeAction || !onVolumeCommit || volumeBusy || !volumeEnabled) return;
    faderDragging = true;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    localVolume = volumeFromClientY(event.clientY);
  }

  function handleFaderPointerMove(event: PointerEvent) {
    if (!faderDragging) return;
    localVolume = volumeFromClientY(event.clientY);
  }

  function handleFaderPointerUp(event: PointerEvent) {
    if (!faderDragging) return;
    faderDragging = false;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    if (!volumeAction || !onVolumeCommit) return;
    void onVolumeCommit(localVolume);
  }

  $effect(() => {
    const url = artworkUrl;
    if (!url) {
      carBodyBg = DEFAULT_CAR_BACKGROUNDS.body;
      carFooterBg = DEFAULT_CAR_BACKGROUNDS.footer;
      shaderColors = [...DEFAULT_SHADER_COLORS];
      return;
    }

    let cancelled = false;
    void extractAlbumPalette(url).then((palette) => {
      if (cancelled) return;
      if (!palette?.[0]) {
        carBodyBg = DEFAULT_CAR_BACKGROUNDS.body;
        carFooterBg = DEFAULT_CAR_BACKGROUNDS.footer;
        shaderColors = [...DEFAULT_SHADER_COLORS];
        return;
      }
      const surfaces = paletteToCarThingBackgrounds(palette[0]);
      carBodyBg = surfaces.body;
      carFooterBg = surfaces.footer;
      shaderColors = paletteToShaderColors(palette);
    });

    return () => {
      cancelled = true;
    };
  });

  let artShown = $state<string | null>(artworkUrl ?? null);
  let artIncoming = $state<string | null>(null);
  let artIncomingReady = $state(false);

  $effect(() => {
    const next = artworkUrl ?? null;
    const shown = untrack(() => artShown);
    const incoming = untrack(() => artIncoming);

    if (next === shown) {
      if (incoming) {
        artIncoming = null;
        artIncomingReady = false;
      }
      return;
    }
    if (!next) {
      artShown = null;
      artIncoming = null;
      artIncomingReady = false;
      return;
    }
    if (!shown) {
      artShown = next;
      return;
    }
    if (next === incoming) return;

    let cancelled = false;
    let settleTimer = 0;
    let revealed = false;
    artIncoming = next;
    artIncomingReady = false;

    function reveal() {
      if (cancelled || revealed) return;
      if (untrack(() => artIncoming) !== next) return;
      revealed = true;
      artIncomingReady = true;
      settleTimer = window.setTimeout(() => {
        settleIncomingArtwork();
      }, 700);
    }

    const img = new Image();
    img.onload = () => {
      if (cancelled) return;
      requestAnimationFrame(() => {
        requestAnimationFrame(reveal);
      });
    };
    img.onerror = () => {
      if (cancelled || untrack(() => artIncoming) !== next) return;
      artShown = next;
      artIncoming = null;
      artIncomingReady = false;
    };
    img.src = next;
    if (img.complete && img.naturalWidth > 0) {
      requestAnimationFrame(() => {
        requestAnimationFrame(reveal);
      });
    }

    return () => {
      cancelled = true;
      if (settleTimer) window.clearTimeout(settleTimer);
    };
  });

  function settleIncomingArtwork() {
    if (!artIncoming || !artIncomingReady) return;
    artShown = artIncoming;
    artIncoming = null;
    artIncomingReady = false;
  }

  async function runTransportAction(action: string, label: string, value?: unknown) {
    try {
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-started", { detail: { action, label } })
      );
      if (value !== undefined) {
        await executeActionValue(action, value);
      } else {
        await executeAction(action);
      }
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-executed", { detail: { action, label } })
      );
    } catch (e) {
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-failed", {
          detail: { action, label, error: String(e) },
        })
      );
      logError(`${label} failed: ${String(e)}`, source);
    }
  }

  let playlistsOpen = $state(false);
  let lyricsOpen = $state(false);
  let lyricsBusy = $state(false);
  let lyricsError = $state<string | null>(null);
  let lyricsTrackId = $state<string | null>(null);
  let lyricsDoc = $state<SpotifyTrackLyrics | null>(null);

  const lyricsLines = $derived(lyricsDoc?.lines ?? []);
  const lyricsSynced = $derived(
    lyricsDoc?.syncType === "LINE_SYNCED" || lyricsDoc?.syncType === "SYLLABLE_SYNCED"
  );
  const currentLyricIndex = $derived.by(() => {
    if (lyricsLines.length === 0) return 0;
    if (!lyricsSynced) return 0;
    const at = playbackDisplayMs;
    let index = 0;
    for (let i = 0; i < lyricsLines.length; i += 1) {
      if (lyricsLines[i].startTimeMs <= at) index = i;
      else break;
    }
    return index;
  });
  const lyricWindow = $derived.by(() => {
    const lines = lyricsLines;
    const index = currentLyricIndex;
    const items: { key: string; role: "exit" | "prev" | "current" | "next"; words: string }[] = [];
    const push = (at: number, role: "exit" | "prev" | "current" | "next") => {
      const line = lines[at];
      if (!line) return;
      items.push({
        key: `${at}:${line.words}`,
        role,
        words: line.words,
      });
    };
    push(index - 2, "exit");
    push(index - 1, "prev");
    push(index, "current");
    push(index + 1, "next");
    return items;
  });

  $effect(() => {
    if (!lyricsOpen) return;
    const id = trackId ?? null;
    if (id && id !== lyricsTrackId) {
      void loadLyrics(id);
    }
  });

  async function toggleLyrics() {
    lyricsOpen = !lyricsOpen;
    if (lyricsOpen) {
      await loadLyrics(trackId ?? null);
    }
  }

  async function loadLyrics(id: string | null) {
    if (!id) {
      lyricsDoc = null;
      lyricsTrackId = null;
      lyricsError = "Nothing is playing";
      return;
    }
    if (lyricsTrackId === id && lyricsDoc) {
      if (!lyricsDoc.available) {
        lyricsError = "Lyrics aren't available for this track";
      }
      return;
    }
    lyricsBusy = true;
    lyricsError = null;
    try {
      const next = await getSpotifyLyrics(id);
      lyricsDoc = next;
      lyricsTrackId = id;
      lyricsError = next.available ? null : "Lyrics aren't available for this track";
    } catch (e) {
      lyricsDoc = null;
      lyricsTrackId = id;
      lyricsError = lyricsUserMessage(e);
    } finally {
      lyricsBusy = false;
    }
  }

  function lyricsUserMessage(error: unknown): string {
    const text = String(error ?? "");
    if (/403|forbidden|permission denied/i.test(text)) {
      return "Spotify wouldn't share lyrics for this track.";
    }
    return text.replace(/^Error:\s*/i, "") || "Spotify lyrics could not be loaded";
  }

  function formatTime(ms: number): string {
    const totalSec = Math.max(0, Math.floor(ms / 1000));
    const min = Math.floor(totalSec / 60);
    const sec = totalSec % 60;
    return `${min}:${sec.toString().padStart(2, "0")}`;
  }
</script>

<div
  class="car-thing"
  class:car-thing--shader={showFullViewShader}
  class:car-thing--controls-backdrop={showControlsBackdrop}
  style={`--car-body-bg: ${carBodyBg}; --car-footer-bg: ${carFooterBg}; --controls-overlay-alpha: ${controlsOverlayAlpha}; --controls-overlay-rgb: ${controlsOverlayRgb};`}
>
  {#if showFullViewShader}
    <SceneBackground
      style={backgroundStyle}
      colors={shaderColors}
      imageUrl={usesCoverImage(backgroundStyle) ? artworkUrl : null}
      playing={isPlaying}
    />
  {/if}
  <div
    class="car-thing-body"
    class:car-thing-body--has-icon={showSettingsButton && onOpenSettings}
    class:car-thing-body--shader={showFullViewShader}
  >
    {#if showFullViewShader}
      <div class="car-shader-scrim" aria-hidden="true"></div>
    {/if}
    {#if showSettingsButton && onOpenSettings}
      <button
        type="button"
        class="car-settings-btn"
        title="Settings"
        aria-label="Open settings"
        onclick={() => onOpenSettings()}
      >
        <img class="car-app-icon" src={appIconUrl} alt="" aria-hidden="true" />
      </button>
    {/if}
    {#if playlistsEnabled}
      <button
        type="button"
        class="car-playlists-btn"
        title="Playlists"
        aria-label="Browse playlists"
        onclick={() => (playlistsOpen = true)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M4 6h12v2H4V6zm0 5h12v2H4v-2zm0 5h8v2H4v-2zm13-1.5v-6.2c0-.7.6-1.3 1.3-1.3H22v2h-2.7v7.1c0 1.5-1.2 2.7-2.7 2.7s-2.6-1.2-2.6-2.7 1.2-2.6 2.6-2.6c.5 0 .9.1 1.4.3z"
          />
        </svg>
        <span>Playlists</span>
      </button>
    {/if}

    <section class="car-now-playing" class:car-now-playing--lyrics={lyricsOpen}>
      {#if lyricsOpen}
        <div class="car-lyrics" aria-live="polite">
          {#if lyricsBusy}
            <p class="car-lyrics-status">Loading lyrics…</p>
          {:else if !lyricsDoc?.available}
            <p class="car-lyrics-status">{lyricsError ?? "Lyrics aren't available for this track"}</p>
          {:else}
            <div class="car-lyrics-stage">
              {#each lyricWindow as line (line.key)}
                <p
                  class="car-lyrics-line"
                  class:car-lyrics-line--exit={line.role === "exit"}
                  class:car-lyrics-line--prev={line.role === "prev"}
                  class:car-lyrics-line--current={line.role === "current"}
                  class:car-lyrics-line--next={line.role === "next"}
                >
                  {line.words}
                </p>
              {/each}
            </div>
          {/if}
        </div>
      {:else}
      <div class="car-now-playing-inner">
        <div class="car-art-column">
          <div class="car-art-frame" class:car-art-frame--glow={showArtBorder}>
            {#if showArtBorder}
              <SceneBackground
                style="pulsing-border"
                colors={shaderColors}
                placement="artwork"
                playing={isPlaying}
              />
            {/if}
            <div class="car-art-plate">
              {#if artShown}
                <img class="car-artwork" src={artShown} alt={title ?? "Album art"} />
              {:else}
                <div class="car-artwork car-artwork-placeholder" aria-hidden="true">♪</div>
              {/if}
              {#if artIncoming}
                <img
                  class="car-artwork car-artwork-incoming"
                  class:car-artwork-incoming--in={artIncomingReady}
                  src={artIncoming}
                  alt=""
                  aria-hidden="true"
                  ontransitionend={settleIncomingArtwork}
                />
              {/if}
            </div>
          </div>
        </div>

        <div class="car-meta-column">
          {#if albumName}
            <div class="car-album-line">{albumName}</div>
          {/if}
          <h2 class="car-track-title">{title ?? "Nothing playing"}</h2>
          <p class="car-artist-name">{subtitle ?? "No artist information"}</p>
        </div>
      </div>
      {/if}
    </section>
  </div>

  <div
    class="car-progress-wrap"
    class:car-progress-disabled={!seekEnabled || !hasDuration}
    class:car-progress-active={progressInteracting}
    bind:this={progressTrackEl}
    role="slider"
    tabindex="0"
    aria-label="Track progress"
    aria-valuemin={0}
    aria-valuemax={durationMs ?? 0}
    aria-valuenow={progressAriaValue}
    aria-disabled={!seekEnabled || !hasDuration}
    onpointerenter={handleProgressPointerEnter}
    onpointerleave={handleProgressPointerLeave}
    onpointerdown={handleProgressPointerDown}
    onpointermove={handleProgressPointerMove}
    onpointerup={handleProgressPointerUp}
    onpointercancel={handleProgressPointerUp}
  >
    <div class="car-progress-rail">
      <div
        class="car-progress-played"
        class:car-progress-played-hover={progressInteracting}
        style={`width: ${playbackRatio * 100}%`}
      ></div>
      {#if showProgressPreview}
        <div
          class="car-progress-preview"
          style={`left: ${playbackRatio * 100}%; width: ${progressPreviewWidth}%`}
        ></div>
      {/if}
      {#if progressInteracting}
        <div class="car-progress-knob" style={`left: ${playbackRatio * 100}%`}></div>
        <div class="car-progress-tooltip-anchor" style={`left: ${scrubRatio * 100}%`}>
          <span class="car-progress-tooltip">{formatTime(scrubMs)}</span>
        </div>
      {/if}
    </div>
  </div>

  <footer class="car-controls">
    <div class="car-controls-row" class:car-controls-row--lyrics={lyricsEnabled}>
        {#if shuffle}
          <button
            type="button"
            class="car-transport-btn"
            class:car-transport-active={shuffleActive}
            aria-label={shuffle.label}
            aria-pressed={shuffleActive}
            onclick={() => runTransportAction(shuffle.action, shuffle.label)}
          >
            <svg class="car-transport-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.46 20 9.5V4h-5.5zm-.67 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z"
              />
            </svg>
          </button>
        {:else}
          <span class="car-transport-spacer" aria-hidden="true"></span>
        {/if}
        {#if previous}
          <button
            type="button"
            class="car-transport-btn"
            aria-label={previous.label}
            onclick={() => runTransportAction(previous.action, previous.label)}
          >
            <svg class="car-transport-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path fill="currentColor" d="M6 6h2v12H6V6zm3.5 6 8.5 6V6l-8.5 6z" />
            </svg>
          </button>
        {/if}
        {#if playPause}
          <button
            type="button"
            class="car-transport-btn car-transport-center"
            aria-label={playPause.label}
            onclick={() => runTransportAction(playPause.action, playPause.label)}
          >
            {#if isPlaying}
              <svg class="car-transport-icon car-transport-icon-center" viewBox="0 0 24 24" aria-hidden="true">
                <path fill="currentColor" d="M6 5h4v14H6V5zm8 0h4v14h-4V5z" />
              </svg>
            {:else}
              <svg class="car-transport-icon car-transport-icon-center" viewBox="0 0 24 24" aria-hidden="true">
                <path fill="currentColor" d="M8 5v14l11-7L8 5z" />
              </svg>
            {/if}
          </button>
        {/if}
        {#if next}
          <button
            type="button"
            class="car-transport-btn"
            aria-label={next.label}
            onclick={() => runTransportAction(next.action, next.label)}
          >
            <svg class="car-transport-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path fill="currentColor" d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
            </svg>
          </button>
        {/if}
        {#if like}
          <button
            type="button"
            class="car-transport-btn"
            class:car-transport-liked={trackSaved === true}
            aria-label={like.label}
            aria-pressed={trackSaved === true}
            onclick={() => runTransportAction(like.action, like.label, !(trackSaved === true))}
          >
            <svg class="car-transport-icon" viewBox="0 0 24 24" aria-hidden="true">
              {#if trackSaved}
                <path
                  fill="currentColor"
                  d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"
                />
              {:else}
                <path
                  fill="currentColor"
                  d="M16.5 3c-1.74 0-3.41.81-4.5 2.09C10.91 3.81 9.24 3 7.5 3 4.42 3 2 5.42 2 8.5c0 3.78 3.4 6.86 8.55 11.54L12 21.35l1.45-1.32C18.6 15.36 22 12.28 22 8.5 22 5.42 19.58 3 16.5 3zm-4.4 15.55-.1.1-.1-.1C7.14 14.24 4 11.39 4 8.5 4 6.5 5.5 5 7.5 5c1.54 0 3.04.99 3.57 2.36h1.87C13.46 5.99 14.96 5 16.5 5c2 0 3.5 1.5 3.5 3.5 0 2.89-3.14 5.74-7.9 10.05z"
                />
              {/if}
            </svg>
          </button>
        {:else if !lyricsEnabled}
          <span class="car-transport-spacer" aria-hidden="true"></span>
        {/if}
        {#if lyricsEnabled}
          <button
            type="button"
            class="car-transport-btn"
            class:car-transport-lyrics-on={lyricsOpen}
            aria-label={lyricsOpen ? "Hide lyrics" : "Show lyrics"}
            aria-pressed={lyricsOpen}
            disabled={!trackId}
            onclick={() => void toggleLyrics()}
          >
            <svg class="car-transport-icon" viewBox="0 0 24 24" aria-hidden="true">
              <g
                fill="none"
                stroke="currentColor"
                stroke-width="1.85"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <g transform="rotate(-40 13.05 9)">
                  <rect x="10.5" y="2.55" width="5.1" height="11.15" rx="2.55" />
                  <path d="M13.05 13.7v3.05" />
                </g>
              </g>
              <circle cx="8.2" cy="19.2" r="1.55" fill="currentColor" />
            </svg>
          </button>
        {/if}
    </div>
  </footer>

  <PlaylistBrowser open={playlistsOpen} onClose={() => (playlistsOpen = false)} />

  {#if volumeAction}
    <aside class="car-fader" aria-label="Volume">
      <div class="car-volume-display" aria-hidden="true">
        <span
          class="car-volume-led"
          class:car-volume-led-triple={localVolume >= 100}
        >{localVolume}</span>
      </div>
      <div
        class="car-fader-track"
        class:car-fader-locked={volumeBusy}
        class:car-fader-disabled={!volumeEnabled}
        class:car-fader-dragging={faderDragging}
        bind:this={faderTrackEl}
        role="slider"
        tabindex="0"
        aria-label="Volume"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={localVolume}
        aria-disabled={volumeBusy || !volumeEnabled}
        onpointerdown={handleFaderPointerDown}
        onpointermove={handleFaderPointerMove}
        onpointerup={handleFaderPointerUp}
        onpointercancel={handleFaderPointerUp}
      >
        <div class="car-fader-scale car-fader-scale-left">
          {#each Array(21) as _, i}
            <span
              class="car-fader-tick"
              class:car-fader-tick-long={i === 0 || i === 10 || i === 20}
              class:car-fader-tick-mid={i % 5 === 0 && i !== 0 && i !== 10 && i !== 20}
            ></span>
          {/each}
        </div>
        <div class="car-fader-rail">
          <div class="car-fader-slot" bind:this={faderSlotEl}>
            <div class="car-fader-track-line" aria-hidden="true"></div>
            <div
              class="car-fader-thumb"
              style={`top: ${faderThumbTop}`}
              aria-hidden="true"
            >
              <span class="car-fader-thumb-line"></span>
            </div>
          </div>
        </div>
        <div class="car-fader-scale car-fader-scale-right">
          {#each Array(21) as _, i}
            <span
              class="car-fader-tick"
              class:car-fader-tick-long={i === 0 || i === 10 || i === 20}
              class:car-fader-tick-mid={i % 5 === 0 && i !== 0 && i !== 10 && i !== 20}
            ></span>
          {/each}
        </div>
      </div>
    </aside>
  {/if}
</div>

<style>
  .car-thing {
    position: relative;
    display: grid;
    flex: 1;
    min-height: 0;
    width: 100%;
    align-self: stretch;
    overflow: hidden;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto auto;
    background: var(--car-body-bg, #0a0a0a);
    color: #f5f5f5;
    transition: background 0.7s ease;
  }

  .car-thing--shader {
    background: #000;
  }

  .car-thing > :global(.scene-background) {
    z-index: 0;
  }

  .car-thing:has(.car-fader) {
    grid-template-columns: minmax(0, 1fr) 124px;
  }

  .car-thing:has(.car-fader) .car-thing-body {
    padding-right: 8px;
  }

  .car-thing-body {
    grid-column: 1;
    grid-row: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 20px 16px 0 0;
    gap: 0;
    position: relative;
    overflow: hidden;
    background: var(--car-body-bg, #0a0a0a);
    transition: background 0.7s ease;
    z-index: 1;
  }

  .car-thing-body--shader {
    background: transparent;
  }

  .car-shader-scrim {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    background: linear-gradient(
      90deg,
      rgba(0, 0, 0, 0.42) 0%,
      rgba(0, 0, 0, 0.22) 55%,
      rgba(0, 0, 0, 0.28) 100%
    );
  }

  .car-settings-btn {
    position: absolute;
    top: 8px;
    left: 12px;
    z-index: 2;
    padding: 4px;
    border: none;
    border-radius: 12px;
    background: transparent;
    line-height: 0;
    cursor: pointer;
  }

  .car-settings-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .car-playlists-btn {
    position: absolute;
    top: 18px;
    right: 20px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-height: 56px;
    padding: 10px 16px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 999px;
    background: rgba(12, 12, 12, 0.55);
    color: #fff;
    font-size: 1rem;
    font-weight: 650;
    cursor: pointer;
  }

  .car-playlists-btn svg {
    width: 26px;
    height: 26px;
  }

  .car-playlists-btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .car-app-icon {
    display: block;
    width: 96px;
    height: 96px;
    object-fit: contain;
  }

  .car-now-playing {
    position: relative;
    z-index: 1;
    flex: 1 1 0;
    min-height: 0;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    padding: 0 20px 44px clamp(172px, 19vw, 300px);
    overflow: hidden;
  }

  .car-now-playing--lyrics {
    justify-content: center;
    padding-left: 20px;
    padding-right: 20px;
  }

  .car-lyrics {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 8px;
  }

  .car-lyrics-stage {
    position: relative;
    width: min(52rem, 100%);
    height: 10.8em;
    overflow: hidden;
  }

  .car-lyrics-line {
    position: absolute;
    left: 0;
    right: 0;
    margin: 0;
    padding: 0 12px;
    text-align: center;
    line-height: 1.25;
    overflow-wrap: break-word;
    transition:
      top 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.45s ease,
      color 0.35s ease,
      font-size 0.45s cubic-bezier(0.22, 1, 0.36, 1),
      font-weight 0.35s ease;
  }

  .car-lyrics-line--exit {
    top: -1.35em;
    opacity: 0;
    color: rgba(255, 255, 255, 0.28);
    font-size: clamp(1.05rem, 2.2vw, 1.45rem);
    font-weight: 500;
    pointer-events: none;
  }

  .car-lyrics-line--prev {
    top: 0.45em;
    opacity: 0.42;
    color: rgba(232, 232, 232, 0.58);
    font-size: clamp(1.2rem, 2.8vw, 1.85rem);
    font-weight: 600;
  }

  .car-lyrics-line--current {
    top: 3.35em;
    opacity: 1;
    color: #fff;
    font-size: clamp(1.85rem, 4.4vw, 2.85rem);
    font-weight: 800;
    letter-spacing: -0.02em;
  }

  .car-lyrics-line--next {
    top: 7.15em;
    opacity: 0.38;
    color: rgba(232, 232, 232, 0.5);
    font-size: clamp(1.2rem, 2.8vw, 1.85rem);
    font-weight: 600;
  }

  .car-lyrics-status {
    margin: 0;
    color: rgba(255, 255, 255, 0.62);
    font-size: clamp(1.2rem, 2.4vw, 1.7rem);
    font-weight: 600;
    text-align: center;
  }

  .car-thing-body--has-icon .car-now-playing {
    padding-top: 100px;
  }

  .car-now-playing-inner {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    column-gap: 72px;
    align-items: center;
    width: 100%;
    max-height: 100%;
    height: 100%;
    min-width: 0;
  }

  .car-art-column {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    height: 100%;
    align-self: stretch;
  }

  .car-art-frame {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    width: auto;
    max-width: 100%;
    aspect-ratio: 1 / 1;
  }

  .car-art-frame--glow {
    overflow: visible;
    isolation: isolate;
  }

  .car-art-plate {
    position: relative;
    z-index: 1;
    height: 100%;
    width: 100%;
    max-width: 100%;
    aspect-ratio: 1 / 1;
  }

  .car-art-frame--glow .car-art-plate {
    width: 84%;
    height: 84%;
  }

  .car-artwork {
    position: relative;
    z-index: 1;
    display: block;
    height: 100%;
    width: 100%;
    max-width: 100%;
    aspect-ratio: 1 / 1;
    object-fit: contain;
    object-position: center;
    border-radius: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
    background: #1a1a1a;
  }

  .car-artwork-incoming {
    position: absolute;
    inset: 0;
    z-index: 2;
    opacity: 0;
    transition: opacity 0.55s ease;
    pointer-events: none;
  }

  .car-artwork-incoming--in {
    opacity: 1;
  }

  .car-thing-body--has-icon .car-artwork {
    height: 100%;
  }

  .car-artwork-placeholder {
    display: grid;
    place-items: center;
    font-size: 3rem;
    background: #1a1a1a;
    color: #666;
  }

  .car-meta-column {
    min-width: 0;
    max-width: min(36rem, 50vw);
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 14px;
  }

  .car-album-line {
    font-size: clamp(1rem, 1.9vw, 1.35rem);
    font-weight: 500;
    color: #b3b3b3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
  }

  .car-track-title {
    margin: 0;
    font-size: clamp(2.1rem, 5vw, 3.5rem);
    font-weight: 800;
    line-height: 1.05;
    letter-spacing: -0.02em;
    width: 100%;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    overflow: hidden;
    text-overflow: ellipsis;
    overflow-wrap: break-word;
  }

  .car-artist-name {
    margin: 0;
    font-size: clamp(1.35rem, 2.8vw, 2.1rem);
    font-weight: 500;
    color: #e8e8e8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
  }

  .car-progress-wrap {
    grid-column: 1;
    grid-row: 2;
    flex-shrink: 0;
    box-sizing: border-box;
    height: 18px;
    min-height: 18px;
    max-height: 18px;
    padding: 0 0 10px;
    display: flex;
    align-items: flex-end;
    overflow: visible;
    cursor: pointer;
    touch-action: none;
    user-select: none;
    position: relative;
    z-index: 2;
    background: var(--car-footer-bg, #0c0808);
    transition: background 0.7s ease;
  }

  .car-thing--controls-backdrop .car-progress-wrap {
    background: rgba(var(--controls-overlay-rgb, 0, 0, 0), var(--controls-overlay-alpha, 0.65));
  }

  .car-progress-wrap.car-progress-disabled {
    opacity: 0.45;
    cursor: default;
    pointer-events: none;
  }

  .car-progress-rail {
    position: relative;
    width: 100%;
    height: 8px;
    background: rgba(255, 255, 255, 0.14);
    border-radius: 0;
  }

  .car-progress-played {
    position: absolute;
    inset: 0 auto 0 0;
    background: #fff;
    border-radius: 0;
    pointer-events: none;
    transition: background 0.12s ease;
  }

  .car-progress-played.car-progress-played-hover {
    background: #1db954;
  }

  .car-progress-wrap.car-progress-active .car-progress-played {
    border-radius: 0;
  }

  .car-progress-preview {
    position: absolute;
    inset: 0 auto 0 auto;
    background: #fff;
    border-radius: 0;
    pointer-events: none;
  }

  .car-progress-knob {
    position: absolute;
    top: 50%;
    width: 28px;
    height: 28px;
    background: #fff;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    pointer-events: none;
    z-index: 2;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .car-progress-tooltip-anchor {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    pointer-events: none;
    z-index: 3;
  }

  .car-progress-tooltip {
    position: absolute;
    bottom: calc(100% + 14px);
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    background: #282828;
    color: #fff;
    font-size: 1.44rem;
    font-weight: 600;
    line-height: 1;
    border-radius: 6px;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
  }

  .car-progress-tooltip::after {
    content: "";
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border: 6px solid transparent;
    border-top-color: #282828;
  }

  .car-controls {
    grid-column: 1;
    grid-row: 3;
    flex-shrink: 0;
    padding: 12px 24px max(20px, env(safe-area-inset-bottom, 0px));
    min-height: 148px;
    position: relative;
    z-index: 1;
    background: var(--car-footer-bg, #0c0808);
    transition: background 0.7s ease;
  }

  .car-thing--controls-backdrop .car-controls {
    background: rgba(var(--controls-overlay-rgb, 0, 0, 0), var(--controls-overlay-alpha, 0.65));
  }

  .car-controls-row {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    align-items: center;
    justify-items: center;
    gap: 12px;
    max-width: min(100%, 720px);
    margin: 0 auto;
  }

  .car-controls-row--lyrics {
    grid-template-columns: repeat(6, minmax(0, 1fr));
    max-width: min(100%, 860px);
  }

  .car-transport-lyrics-on {
    color: #1db954;
  }

  .car-transport-spacer {
    width: 104px;
    height: 1px;
    visibility: hidden;
    pointer-events: none;
  }

  .car-transport-btn {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 104px;
    height: 104px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: rgba(255, 255, 255, 0.92);
    cursor: pointer;
    transition: color 0.15s ease, transform 0.12s ease;
  }

  .car-transport-btn:hover {
    color: #fff;
  }

  .car-transport-btn:active {
    transform: scale(0.94);
  }

  .car-transport-btn:disabled {
    opacity: 0.35;
    cursor: default;
    transform: none;
  }

  .car-transport-center {
    width: 104px;
    height: 104px;
  }

  .car-transport-icon {
    width: 56px;
    height: 56px;
  }

  .car-transport-icon-center {
    width: 60px;
    height: 60px;
  }

  .car-transport-active {
    color: #1db954;
  }

  .car-transport-liked {
    color: #1db954;
  }

  .car-fader {
    grid-column: 2;
    grid-row: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: stretch;
    gap: 0;
    padding: 20px 20px max(20px, env(safe-area-inset-bottom, 0px));
    position: relative;
    z-index: 1;
    background: var(--car-footer-bg, #0c0808);
    min-height: 0;
    transition: background 0.7s ease;
  }

  .car-thing--shader .car-fader {
    isolation: isolate;
  }

  .car-thing--controls-backdrop .car-fader {
    background: rgba(var(--controls-overlay-rgb, 0, 0, 0), var(--controls-overlay-alpha, 0.65));
  }

  .car-volume-display {
    flex-shrink: 0;
    width: calc(100% - 8px);
    padding: 12px 6px 14px;
    margin: 4px 4px 28px;
    overflow: hidden;
    background: linear-gradient(180deg, #050505 0%, #0a0a0a 100%);
    border: 2px solid #1a1a1a;
    border-radius: 4px;
    box-shadow:
      inset 0 2px 8px rgba(0, 0, 0, 0.9),
      inset 0 0 12px rgba(255, 0, 0, 0.04),
      0 1px 0 rgba(255, 255, 255, 0.04);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .car-volume-led {
    font-family: "DSEG7 Classic", "Courier New", monospace;
    font-size: 1.22rem;
    font-weight: 400;
    line-height: 1.2;
    color: #ff1a1a;
    text-shadow:
      0 0 4px rgba(255, 30, 30, 0.95),
      0 0 12px rgba(255, 20, 20, 0.75),
      0 0 24px rgba(255, 0, 0, 0.45);
    display: block;
    width: 100%;
    text-align: center;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .car-volume-led-triple {
    letter-spacing: -0.03em;
    transform: translateX(-0.2em);
  }

  .car-fader-track {
    --fader-thumb-half: 39px;
    flex: 1 1 auto;
    width: 100%;
    min-height: 0;
    max-height: none;
    margin: 8px 0 32px;
    display: grid;
    grid-template-columns: 14px 1fr 14px;
    gap: 6px;
    align-items: stretch;
    cursor: pointer;
    touch-action: none;
    user-select: none;
  }

  .car-fader-track.car-fader-disabled,
  .car-fader-track.car-fader-locked {
    pointer-events: none;
  }

  .car-fader-track.car-fader-disabled {
    opacity: 0.45;
  }

  .car-fader-track.car-fader-locked {
    cursor: default;
  }

  .car-fader-scale {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: flex-end;
    height: 100%;
    box-sizing: border-box;
    padding-top: var(--fader-thumb-half);
    padding-bottom: var(--fader-thumb-half);
  }

  .car-fader-scale-right {
    align-items: flex-start;
  }

  .car-fader-tick {
    display: block;
    width: 5px;
    height: 1px;
    background: rgba(255, 255, 255, 0.14);
  }

  .car-fader-tick-mid {
    width: 8px;
    background: rgba(255, 255, 255, 0.22);
  }

  .car-fader-tick-long {
    width: 11px;
    background: rgba(255, 255, 255, 0.3);
  }

  .car-thing--shader .car-fader-tick {
    background: #4a4a4a;
  }

  .car-thing--shader .car-fader-tick-mid {
    background: #5a5a5a;
  }

  .car-thing--shader .car-fader-tick-long {
    background: #6a6a6a;
  }

  .car-fader-rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: stretch;
    min-height: 0;
    height: 100%;
  }

  .car-fader-slot {
    position: relative;
    flex: 1;
    width: 51px;
    min-height: 120px;
    background: transparent;
  }

  .car-fader-track-line {
    position: absolute;
    top: var(--fader-thumb-half);
    bottom: var(--fader-thumb-half);
    left: 50%;
    transform: translateX(-50%);
    width: 20px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.16);
    pointer-events: none;
    z-index: 1;
  }

  .car-thing--shader .car-fader-track-line {
    background: #3a3a3a;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
  }

  .car-fader-thumb {
    position: absolute;
    left: 50%;
    width: 51px;
    height: 78px;
    transform: translate(-50%, -50%);
    border-radius: 6px;
    background: #424242;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.38);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    z-index: 2;
    transition: background 0.15s ease, box-shadow 0.15s ease, transform 0.12s ease;
  }

  .car-fader-thumb-line {
    display: block;
    width: 72%;
    height: 2px;
    background: #fff;
    border-radius: 1px;
    opacity: 0.92;
  }

  .car-fader-track.car-fader-dragging .car-fader-thumb {
    background: #4a4a4a;
    box-shadow: 0 3px 8px rgba(0, 0, 0, 0.42);
    transform: translate(-50%, -50%) scale(1.02);
  }
</style>
