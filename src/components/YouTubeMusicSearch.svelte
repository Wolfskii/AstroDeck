<script lang="ts">
  import {
    addYouTubeMusicTrackToPlaylist,
    createYouTubeMusicPlaylist,
    discoverYouTubeMusic,
    getYouTubeMusicLibrary,
    playYouTubeMusic,
    saveYouTubeMusicTrack,
    searchYouTubeMusic,
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

  let query = $state("");
  let results = $state<YouTubeSearchTrack[]>([]);
  let loading = $state(false);
  let playingId = $state<string | null>(null);
  let savingId = $state<string | null>(null);
  let addingId = $state<string | null>(null);
  let library = $state<YouTubeLocalLibrary | null>(null);
  let newPlaylistName = $state("");
  let selectedPlaylistByTrack = $state<Record<string, string>>({});
  let discoveryFilter = $state<"trending" | "popular" | "playlists" | "chill">("trending");
  let discoveryMenuOpen = $state(false);
  let error = $state<string | null>(null);
  let sawOpen = false;

  $effect(() => {
    if (open && !sawOpen) {
      query = "";
      results = [];
      error = null;
      void getYouTubeMusicLibrary().then((next) => {
        library = next;
        if (next.savedTracks.length === 0 && next.playlists.length === 0) {
          void loadDiscovery();
        }
      }).catch(() => {});
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

  async function loadDiscovery() {
    loading = true;
    error = null;
    try {
      results = await discoverYouTubeMusic(discoveryFilter);
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      loading = false;
    }
  }

  const discoveryLabels = {
    trending: "Trending now",
    popular: "Most popular",
    playlists: "Popular playlists",
    chill: "Chill playlists",
  } as const;

  function chooseDiscoveryFilter(
    filter: "trending" | "popular" | "playlists" | "chill"
  ) {
    discoveryFilter = filter;
    discoveryMenuOpen = false;
    void loadDiscovery();
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

  async function save(track: YouTubeSearchTrack) {
    savingId = track.videoId;
    error = null;
    try {
      library = await saveYouTubeMusicTrack(track);
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      savingId = null;
    }
  }

  async function createPlaylist() {
    if (!newPlaylistName.trim()) return;
    try {
      library = await createYouTubeMusicPlaylist(newPlaylistName);
      newPlaylistName = "";
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    }
  }

  async function addToPlaylist(track: YouTubeSearchTrack) {
    const playlistId = selectedPlaylistByTrack[track.videoId];
    if (!playlistId) return;
    addingId = track.videoId;
    try {
      library = await addYouTubeMusicTrackToPlaylist(playlistId, track.videoId);
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      addingId = null;
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
      <div class="youtube-discovery-filter">
        <span>Discover</span>
        <div class="youtube-discovery-select">
          <button
            type="button"
            class="youtube-discovery-trigger"
            aria-haspopup="listbox"
            aria-expanded={discoveryMenuOpen}
            onclick={() => (discoveryMenuOpen = !discoveryMenuOpen)}
          >
            <span>{discoveryLabels[discoveryFilter]}</span>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="m6 9 6 6 6-6" />
            </svg>
          </button>
          {#if discoveryMenuOpen}
            <div class="youtube-discovery-menu" role="listbox" aria-label="Discovery filter">
              {#each Object.entries(discoveryLabels) as [value, label]}
                <button
                  type="button"
                  role="option"
                  aria-selected={discoveryFilter === value}
                  class:active={discoveryFilter === value}
                  onclick={() =>
                    chooseDiscoveryFilter(
                      value as "trending" | "popular" | "playlists" | "chill"
                    )}
                >
                  {label}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <div class="youtube-library-bar">
        <span>
          Local guest library:
          {library?.savedTracks.length ?? 0} saved songs ·
          {library?.playlists.length ?? 0} playlists
        </span>
        <form
          onsubmit={(event) => {
            event.preventDefault();
            void createPlaylist();
          }}
        >
          <input bind:value={newPlaylistName} placeholder="New local playlist" />
          <button type="submit" disabled={!newPlaylistName.trim()}>Create</button>
        </form>
      </div>
      {#if error}
        <p class="youtube-error">{error}</p>
      {/if}
      <div class="youtube-results">
        {#each results as track (track.videoId)}
          <div class="youtube-result">
            <button
              type="button"
              class="youtube-result-main"
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
            <button
              type="button"
              class="youtube-save"
              disabled={savingId === track.videoId}
              onclick={() => void save(track)}
            >
              {savingId === track.videoId ? "Saving…" : "Save"}
            </button>
            {#if library?.playlists.length}
              <select
                aria-label={`Choose local playlist for ${track.title}`}
                value={selectedPlaylistByTrack[track.videoId] ?? ""}
                onchange={(event) => {
                  selectedPlaylistByTrack[track.videoId] =
                    (event.currentTarget as HTMLSelectElement).value;
                }}
              >
                <option value="">Add to playlist…</option>
                {#each library.playlists as playlist (playlist.id)}
                  <option value={playlist.id}>{playlist.name}</option>
                {/each}
              </select>
              <button
                type="button"
                class="youtube-add"
                disabled={addingId === track.videoId || !selectedPlaylistByTrack[track.videoId]}
                onclick={() => void addToPlaylist(track)}
              >{addingId === track.videoId ? "Adding…" : "Add"}</button>
            {/if}
          </div>
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

  .youtube-discovery-filter {
    display: flex;
    align-items: center;
    gap: 12px;
    color: #cbd5e1;
    font-size: 1rem;
    font-weight: 750;
  }

  .youtube-discovery-select {
    flex: 1;
    position: relative;
  }

  .youtube-discovery-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    min-height: 54px;
    padding: 10px 16px 10px 15px;
    border: 1px solid rgba(29, 185, 84, 0.5);
    border-radius: 14px;
    outline: none;
    background: #0c0c0c;
    color: #fff;
    font-size: 1.05rem;
    font-weight: 750;
    cursor: pointer;
  }

  .youtube-discovery-trigger svg {
    flex: 0 0 auto;
    width: 22px;
    height: 22px;
    margin-left: 14px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.4;
  }

  .youtube-discovery-trigger:focus-visible {
    box-shadow: 0 0 0 3px rgba(29, 185, 84, 0.2);
  }

  .youtube-discovery-menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    left: 0;
    z-index: 4;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 7px;
    border: 1px solid rgba(29, 185, 84, 0.55);
    border-radius: 14px;
    background: #151515;
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.55);
  }

  .youtube-discovery-menu button {
    min-height: 48px;
    padding: 9px 12px;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: #e8edf2;
    font-size: 1rem;
    font-weight: 700;
    text-align: left;
    cursor: pointer;
  }

  .youtube-discovery-menu button:hover,
  .youtube-discovery-menu button.active {
    background: rgba(29, 185, 84, 0.18);
    color: #1db954;
  }

  .youtube-library-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: #aab4c0;
    font-size: 0.88rem;
  }

  .youtube-library-bar form {
    display: flex;
    gap: 6px;
  }

  .youtube-library-bar input {
    width: 150px;
    min-height: 38px;
    padding: 7px 9px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    background: #090909;
    color: #fff;
  }

  .youtube-library-bar button,
  .youtube-add {
    min-height: 38px;
    padding: 7px 10px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    background: rgba(29, 185, 84, 0.14);
    color: #1db954;
    font-weight: 750;
    cursor: pointer;
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
    grid-template-columns: minmax(0, 1fr) auto;
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

  .youtube-result:hover {
    border-color: rgba(29, 185, 84, 0.55);
    background: #252525;
  }

  .youtube-result-main {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .youtube-result-main img,
  .youtube-result-main .youtube-art-fallback {
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

  .youtube-save {
    min-height: 42px;
    padding: 9px 12px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 10px;
    background: transparent;
    color: #cbd5e1;
    font-weight: 750;
    cursor: pointer;
  }

  .youtube-save:hover:not(:disabled) {
    border-color: #1db954;
    color: #1db954;
  }

  .youtube-result select {
    max-width: 150px;
    min-height: 38px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    background: #090909;
    color: #cbd5e1;
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

    .youtube-library-bar {
      align-items: stretch;
      flex-direction: column;
    }

    .youtube-library-bar form {
      width: 100%;
    }

    .youtube-library-bar input {
      flex: 1;
      width: auto;
    }
  }
</style>
