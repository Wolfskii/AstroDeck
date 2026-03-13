<script lang="ts">
  import type { DeckButtonConfig } from "../types";
  import DeckButton from "./DeckButton.svelte";

  interface Props {
    previous?: DeckButtonConfig | null;
    playPause?: DeckButtonConfig | null;
    next?: DeckButtonConfig | null;
    like?: DeckButtonConfig | null;
    title?: string | null;
    subtitle?: string | null;
    artworkUrl?: string | null;
    playbackState?: string;
    volumeAction?: string | null;
    volumePercent?: number;
    volumeBusy?: boolean;
    volumeEnabled?: boolean;
    source?: string;
    onVolumeCommit?: (value: number) => Promise<void> | void;
  }

  let {
    previous = null,
    playPause = null,
    next = null,
    like = null,
    title = null,
    subtitle = null,
    artworkUrl = null,
    playbackState = "stopped",
    volumeAction = null,
    volumePercent = 50,
    volumeBusy = false,
    volumeEnabled = true,
    source = "media window",
    onVolumeCommit,
  }: Props = $props();

  let localVolume = $state(50);

  $effect(() => {
    localVolume = volumePercent;
  });

  function handleVolumeInput(event: Event) {
    localVolume = Number((event.currentTarget as HTMLInputElement).value);
  }

  async function handleVolumeCommit() {
    if (!volumeAction || !onVolumeCommit) return;
    await onVolumeCommit(localVolume);
  }
</script>

<div class="media-player-view">
  <section class="media-now-playing">
    {#if artworkUrl}
      <img class="media-artwork" src={artworkUrl} alt={title ?? "Current cover art"} />
    {:else}
      <div class="media-artwork media-artwork-placeholder" aria-hidden="true">♪</div>
    {/if}

    <div class="media-meta">
      <div class="media-state-badge media-state-{playbackState}">{playbackState}</div>
      <div class="media-title">{title ?? "Nothing playing"}</div>
      <div class="media-subtitle">{subtitle ?? "No artist information"}</div>
    </div>
  </section>

  <div class="media-player-row">
    <div class="media-slot media-slot-side">
      {#if previous}
        <DeckButton
          label={previous.label}
          emoji={previous.emoji}
          image={previous.image}
          action={previous.action}
          source={source}
        />
      {/if}
    </div>
    <div class="media-slot media-slot-center">
      {#if playPause}
        <DeckButton
          label={playPause.label}
          emoji={playPause.emoji}
          image={playPause.image}
          action={playPause.action}
          source={source}
        />
      {/if}
    </div>
    <div class="media-slot media-slot-side">
      {#if next}
        <DeckButton
          label={next.label}
          emoji={next.emoji}
          image={next.image}
          action={next.action}
          source={source}
        />
      {/if}
    </div>
  </div>

  {#if like}
    <div class="media-player-like">
      <DeckButton
        label={like.label}
        emoji={like.emoji}
        image={like.image}
        action={like.action}
        source={source}
      />
    </div>
  {/if}

  {#if volumeAction}
    <section class="media-volume-panel">
      <div class="media-volume-header">
        <span class="media-volume-label">Volume</span>
        <span class="media-volume-value">{localVolume}%</span>
      </div>
      <input
        class="media-volume-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        value={localVolume}
        oninput={handleVolumeInput}
        onchange={handleVolumeCommit}
        disabled={volumeBusy || !volumeEnabled}
        aria-label="Media volume"
      />
    </section>
  {/if}
</div>

<style>
  .media-player-view {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
    padding: 18px 24px 24px;
    gap: 18px;
    justify-content: center;
  }

  .media-now-playing {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 6px 0;
  }

  .media-artwork {
    width: 112px;
    height: 112px;
    border-radius: 20px;
    object-fit: cover;
    flex: 0 0 auto;
    box-shadow: 0 14px 28px rgba(0, 0, 0, 0.28);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .media-artwork-placeholder {
    display: grid;
    place-items: center;
    font-size: 2.4rem;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-secondary);
  }

  .media-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .media-state-badge {
    display: inline-flex;
    align-self: flex-start;
    padding: 5px 10px;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
  }

  .media-state-playing {
    background: rgba(34, 197, 94, 0.18);
    color: #bbf7d0;
  }

  .media-state-paused {
    background: rgba(245, 158, 11, 0.18);
    color: #fde68a;
  }

  .media-state-stopped {
    background: rgba(148, 163, 184, 0.16);
    color: #cbd5e1;
  }

  .media-title {
    font-size: 1.35rem;
    font-weight: 800;
    line-height: 1.15;
    color: var(--text-primary);
  }

  .media-subtitle {
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .media-player-row {
    display: grid;
    grid-template-columns: 1fr 1.25fr 1fr;
    gap: 16px;
    align-items: stretch;
  }

  .media-slot {
    display: flex;
    align-items: stretch;
  }

  .media-slot :global(.deck-button) {
    width: 100%;
    min-height: 160px;
  }

  .media-slot-center :global(.deck-button) {
    min-height: 190px;
  }

  .media-slot-center :global(.button-emoji) {
    font-size: 2.8rem;
  }

  .media-player-like {
    display: flex;
    justify-content: center;
  }

  .media-player-like :global(.deck-button) {
    min-width: 220px;
  }

  .media-volume-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
  }

  .media-volume-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .media-volume-label {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .media-volume-value {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--accent);
  }

  .media-volume-slider {
    width: min(100%, 720px);
    max-width: 720px;
    height: 96px;
    accent-color: var(--accent);
    cursor: pointer;
  }
</style>
