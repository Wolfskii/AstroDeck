<script lang="ts">
  import {
    getYouTubeMusicLibrary,
    playYouTubeMusic,
    type YouTubeLocalLibrary,
    type YouTubeSearchTrack,
  } from "../services/api";

  let {
    open = false,
    onClose,
  }: {
    open?: boolean;
    onClose: () => void;
  } = $props();

  let library = $state<YouTubeLocalLibrary | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let startingId = $state<string | null>(null);
  let sawOpen = false;

  $effect(() => {
    if (open && !sawOpen) void loadLibrary();
    sawOpen = open;
  });

  async function loadLibrary() {
    loading = true;
    error = null;
    try {
      library = await getYouTubeMusicLibrary();
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      loading = false;
    }
  }

  async function playTrack(track: YouTubeSearchTrack) {
    startingId = track.videoId;
    error = null;
    try {
      await playYouTubeMusic(track);
      onClose();
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      startingId = null;
    }
  }

  function firstPlaylistTrack(trackIds: string[]) {
    return trackIds
      .map((id) => library?.savedTracks.find((track) => track.videoId === id))
      .find((track): track is YouTubeSearchTrack => Boolean(track));
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") onClose();
  }}
/>

{#if open}
  <div class="youtube-library-overlay" role="dialog" aria-modal="true" aria-label="YouTube Music library">
    <div class="youtube-library-sheet">
      <header class="youtube-library-header">
        <div>
          <p>Local guest profile</p>
          <h2>Saved YouTube Music</h2>
        </div>
        <button type="button" onclick={onClose}>Close</button>
      </header>

      {#if loading}
        <p class="youtube-library-status">Loading local library…</p>
      {:else if error}
        <p class="youtube-library-status youtube-library-error">{error}</p>
      {:else if library}
        {#if library.playlists.length > 0}
          <section>
            <h3>Playlists</h3>
            <div class="youtube-library-grid">
              {#each library.playlists as playlist (playlist.id)}
                {@const track = firstPlaylistTrack(playlist.trackIds)}
                <button
                  type="button"
                  class="youtube-library-card"
                  disabled={!track || startingId === track.videoId}
                  onclick={() => track && void playTrack(track)}
                >
                  {#if track?.coverArtUrl}
                    <img src={track.coverArtUrl} alt="" />
                  {:else}
                    <span class="youtube-library-art">♫</span>
                  {/if}
                  <strong>{playlist.name}</strong>
                  <span>{playlist.trackIds.length} saved songs</span>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        <section>
          <h3>Liked songs</h3>
          {#if library.savedTracks.length === 0}
            <p class="youtube-library-status">No saved songs yet. Save tracks from Explore.</p>
          {:else}
            <div class="youtube-library-grid">
              {#each library.savedTracks as track (track.videoId)}
                <button
                  type="button"
                  class="youtube-library-card"
                  disabled={startingId === track.videoId}
                  onclick={() => void playTrack(track)}
                >
                  {#if track.coverArtUrl}
                    <img src={track.coverArtUrl} alt="" />
                  {:else}
                    <span class="youtube-library-art">♫</span>
                  {/if}
                  <strong>{track.title}</strong>
                  <span>{track.artistName}</span>
                </button>
              {/each}
            </div>
          {/if}
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .youtube-library-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    padding: 0;
    background: #090909;
  }

  .youtube-library-sheet {
    display: flex;
    flex-direction: column;
    gap: 22px;
    width: 100%;
    height: 100%;
    padding: 28px clamp(18px, 4vw, 54px);
    overflow: auto;
    color: #fff;
  }

  .youtube-library-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .youtube-library-header p,
  .youtube-library-header h2,
  h3 {
    margin: 0;
  }

  .youtube-library-header p {
    color: #1db954;
    font-size: 0.85rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .youtube-library-header h2 {
    margin-top: 5px;
    font-size: clamp(1.7rem, 3vw, 2.6rem);
  }

  .youtube-library-header button {
    min-height: 48px;
    padding: 10px 17px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 12px;
    background: transparent;
    color: #fff;
    font-weight: 750;
    cursor: pointer;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h3 {
    font-size: 1.15rem;
  }

  .youtube-library-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
  }

  .youtube-library-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    padding: 10px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 15px;
    background: #1a1a1a;
    color: #fff;
    text-align: left;
    cursor: pointer;
  }

  .youtube-library-card:hover:not(:disabled) {
    border-color: #1db954;
    background: #242424;
  }

  .youtube-library-card img,
  .youtube-library-art {
    display: block;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 10px;
    object-fit: cover;
  }

  .youtube-library-art {
    display: grid;
    place-items: center;
    background: #292929;
    color: #1db954;
    font-size: 2rem;
  }

  .youtube-library-card strong,
  .youtube-library-card span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .youtube-library-card span {
    color: #aab4c0;
    font-size: 0.85rem;
  }

  .youtube-library-status {
    color: #cbd5e1;
  }

  .youtube-library-error {
    color: #fca5a5;
  }
</style>
