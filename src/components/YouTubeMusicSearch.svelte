<script lang="ts">
  import {
    playYouTubeMusic,
    searchYouTubeMusic,
    type YouTubeSearchTrack,
  } from "../services/api";

  let {
    open = false,
    onClose,
  }: {
    open?: boolean;
    onClose: () => void;
  } = $props();

  let query = $state("");
  let results = $state<YouTubeSearchTrack[]>([]);
  let loading = $state(false);
  let playingId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let sawOpen = false;

  $effect(() => {
    if (open && !sawOpen) {
      query = "";
      results = [];
      error = null;
    }
    sawOpen = open;
  });

  async function search() {
    const nextQuery = query.trim();
    if (!nextQuery) return;
    loading = true;
    error = null;
    try {
      results = await searchYouTubeMusic(nextQuery);
      if (results.length === 0) error = "No YouTube Music results found.";
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      loading = false;
    }
  }

  async function play(track: YouTubeSearchTrack) {
    playingId = track.videoId;
    error = null;
    try {
      await playYouTubeMusic(track);
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-executed", {
          detail: { action: "youtubeMusic.play", label: track.title },
        })
      );
      onClose();
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
      playingId = null;
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") onClose();
  }}
/>

{#if open}
  <div class="youtube-overlay" role="dialog" aria-modal="true" aria-label="YouTube Music search">
    <div class="youtube-sheet">
      <header>
        <div>
          <p class="youtube-eyebrow">Guest playback</p>
          <h2>YouTube Music</h2>
        </div>
        <button type="button" class="youtube-close" onclick={onClose}>Close</button>
      </header>
      <form
        class="youtube-search"
        onsubmit={(event) => {
          event.preventDefault();
          void search();
        }}
      >
        <input bind:value={query} placeholder="Search YouTube Music" autocomplete="off" />
        <button type="submit" disabled={loading || !query.trim()}>
          {loading ? "Searching…" : "Search"}
        </button>
      </form>
      {#if error}
        <p class="youtube-error">{error}</p>
      {/if}
      <div class="youtube-results">
        {#each results as track (track.videoId)}
          <button
            type="button"
            class="youtube-result"
            disabled={playingId === track.videoId}
            onclick={() => void play(track)}
          >
            {#if track.coverArtUrl}
              <img src={track.coverArtUrl} alt="" />
            {:else}
              <span class="youtube-art-fallback">♪</span>
            {/if}
            <span class="youtube-result-copy">
              <strong>{track.title}</strong>
              <span>{track.artistName}{track.albumName ? ` · ${track.albumName}` : ""}</span>
            </span>
            <span class="youtube-play">{playingId === track.videoId ? "Starting…" : "Play"}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .youtube-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.7);
  }

  .youtube-sheet {
    display: flex;
    flex-direction: column;
    gap: 18px;
    width: min(720px, 100%);
    max-height: min(760px, 100%);
    padding: 24px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 20px;
    background: #141414;
    color: #fff;
    box-shadow: 0 22px 80px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: 1.8rem;
  }

  .youtube-eyebrow {
    color: #1db954;
    font-size: 0.85rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .youtube-close,
  .youtube-search button {
    min-height: 48px;
    padding: 10px 16px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 12px;
    background: transparent;
    color: #fff;
    font-weight: 700;
    cursor: pointer;
  }

  .youtube-search {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
  }

  .youtube-search input {
    min-width: 0;
    min-height: 52px;
    padding: 12px 15px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 12px;
    background: #090909;
    color: #fff;
    font-size: 1rem;
  }

  .youtube-search button {
    background: #1db954;
    border-color: #1db954;
    color: #071b0d;
  }

  .youtube-search button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .youtube-results {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    overflow: auto;
  }

  .youtube-result {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    min-height: 80px;
    padding: 8px;
    border: 1px solid transparent;
    border-radius: 14px;
    background: #1d1d1d;
    color: #fff;
    text-align: left;
    cursor: pointer;
  }

  .youtube-result:hover:not(:disabled) {
    border-color: rgba(29, 185, 84, 0.55);
    background: #252525;
  }

  .youtube-result img,
  .youtube-art-fallback {
    width: 64px;
    height: 64px;
    border-radius: 9px;
    object-fit: cover;
  }

  .youtube-art-fallback {
    display: grid;
    place-items: center;
    background: #2b2b2b;
    color: #94a3b8;
    font-size: 1.4rem;
  }

  .youtube-result-copy {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

  .youtube-result-copy strong,
  .youtube-result-copy span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .youtube-result-copy span {
    color: #aab4c0;
    font-size: 0.9rem;
  }

  .youtube-play {
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(29, 185, 84, 0.14);
    color: #1db954;
    font-weight: 800;
  }

  .youtube-error {
    color: #fca5a5;
  }

  @media (max-width: 560px) {
    .youtube-overlay {
      padding: 8px;
    }

    .youtube-sheet {
      padding: 18px;
    }

    .youtube-search {
      grid-template-columns: 1fr;
    }
  }
</style>
