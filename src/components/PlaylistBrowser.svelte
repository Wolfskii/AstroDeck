<script lang="ts">
  import {
    executeActionValue,
    listSpotifyPlaylists,
    playSpotifyPlaylist,
    type SpotifyPlaylist,
  } from "../services/api";

  let {
    open = false,
    onClose,
  }: {
    open?: boolean;
    onClose: () => void;
  } = $props();

  let playlists = $state<SpotifyPlaylist[]>([]);
  let playlistsTotal = $state(0);
  let playlistsNextOffset = $state<number | null>(null);
  let playlistsLoading = $state(false);
  let playlistsLoadingMore = $state(false);
  let playlistsError = $state<string | null>(null);
  let playlistsQuery = $state("");
  let playingPlaylistId = $state<string | null>(null);
  let loadSeq = 0;
  let sawOpen = false;

  const isTauriRuntime =
    typeof window !== "undefined" &&
    !!(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

  const visiblePlaylists = $derived.by(() => {
    const query = playlistsQuery.trim().toLowerCase();
    if (!query) return playlists;
    return playlists.filter((playlist) => {
      const owner = playlist.ownerName?.toLowerCase() ?? "";
      return playlist.name.toLowerCase().includes(query) || owner.includes(query);
    });
  });

  const BROWSER_DEMO_PLAYLISTS: SpotifyPlaylist[] = [
    {
      id: "demo-daily",
      name: "Daily Mix",
      uri: "spotify:playlist:demo-daily",
      imageUrl: null,
      trackCount: 50,
      ownerName: "Spotify",
    },
    {
      id: "demo-liked",
      name: "Liked Songs",
      uri: "spotify:playlist:demo-liked",
      imageUrl: null,
      trackCount: 128,
      ownerName: "You",
    },
    {
      id: "demo-focus",
      name: "Focus Flow",
      uri: "spotify:playlist:demo-focus",
      imageUrl: null,
      trackCount: 42,
      ownerName: "AstroDeck",
    },
  ];

  function playlistRateLimitWaitMs(message: string): number | null {
    const about = message.match(/about (\d+) seconds/i);
    if (about) {
      const seconds = Number(about[1]);
      if (Number.isFinite(seconds) && seconds > 0) {
        return Math.min(seconds, 12) * 1000;
      }
    }
    if (/rate-limit/i.test(message) || /Too Many Requests/i.test(message)) {
      return 4000;
    }
    return null;
  }

  async function loadPlaylists(reset: boolean, allowAutoRetry = true) {
    const seq = reset ? ++loadSeq : loadSeq;
    if (reset) {
      playlistsLoading = true;
      playlistsError = null;
    } else {
      playlistsLoadingMore = true;
    }
    try {
      if (!isTauriRuntime) {
        playlists = BROWSER_DEMO_PLAYLISTS;
        playlistsTotal = BROWSER_DEMO_PLAYLISTS.length;
        playlistsNextOffset = null;
        playlistsError = "Preview list. Connect in the desktop app to play your playlists.";
        return;
      }
      const page = await listSpotifyPlaylists({
        offset: reset ? 0 : (playlistsNextOffset ?? playlists.length),
        limit: 50,
      });
      if (seq !== loadSeq) return;
      playlists = reset ? page.items : [...playlists, ...page.items];
      playlistsTotal = page.total;
      playlistsNextOffset = page.nextOffset ?? null;
    } catch (e) {
      const message = String(e);
      const waitMs = allowAutoRetry && reset ? playlistRateLimitWaitMs(message) : null;
      if (waitMs != null && seq === loadSeq) {
        playlistsError = message;
        await new Promise((resolve) => setTimeout(resolve, waitMs));
        if (seq !== loadSeq) return;
        await loadPlaylists(true, false);
        return;
      }
      if (seq === loadSeq) {
        playlistsError = message;
      }
    } finally {
      if (seq === loadSeq) {
        playlistsLoading = false;
        playlistsLoadingMore = false;
      }
    }
  }

  $effect(() => {
    const isOpen = open;
    if (isOpen && !sawOpen) {
      playlistsQuery = "";
      playingPlaylistId = null;
      void loadPlaylists(true);
    }
    if (!isOpen && sawOpen) {
      loadSeq += 1;
    }
    sawOpen = isOpen;
  });

  function close() {
    playingPlaylistId = null;
    onClose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
    }
  }

  async function playPlaylist(playlist: SpotifyPlaylist) {
    playingPlaylistId = playlist.id;
    try {
      if (isTauriRuntime) {
        await playSpotifyPlaylist(playlist.uri);
      } else {
        await executeActionValue("spotify.playPlaylist", playlist.uri);
      }
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-executed", {
          detail: { action: "spotify.playPlaylist", label: playlist.name },
        })
      );
      close();
    } catch (e) {
      playlistsError = String(e);
      playingPlaylistId = null;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div
    class="playlist-overlay"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Playlists"
    onclick={(event) => {
      if (event.target === event.currentTarget) close();
    }}
    onkeydown={(event) => {
      if (event.key === "Escape") close();
    }}
  >
    <div class="playlist-sheet">
      <header class="playlist-sheet-head">
        <div>
          <h2>Playlists</h2>
          <p>
            {playlistsTotal > 0
              ? `${playlistsTotal} playlist${playlistsTotal === 1 ? "" : "s"}`
              : "Choose a playlist to play"}
          </p>
        </div>
        <button type="button" class="playlist-close" aria-label="Close playlists" onclick={close}>
          Close
        </button>
      </header>
      <label class="playlist-search">
        <span class="sr-only">Search playlists</span>
        <input
          type="search"
          placeholder="Search playlists"
          bind:value={playlistsQuery}
          autocomplete="off"
        />
      </label>
      {#if playlistsLoading}
        <p class="playlist-status">
          {playlistsError
            ? playlistsError
            : "Loading playlists… Spotify may need a few seconds right after sign-in."}
        </p>
      {:else if playlistsError && playlists.length === 0}
        <p class="playlist-status playlist-status-error">{playlistsError}</p>
        <button
          type="button"
          class="playlist-more"
          onclick={() => void loadPlaylists(true)}
        >
          Try again
        </button>
      {:else if visiblePlaylists.length === 0}
        <p class="playlist-status">No playlists match that search.</p>
      {:else}
        {#if playlistsError}
          <p class="playlist-status playlist-status-error">{playlistsError}</p>
        {/if}
        <ul class="playlist-list">
          {#each visiblePlaylists as playlist (playlist.id)}
            <li>
              <button
                type="button"
                class="playlist-row"
                disabled={playingPlaylistId === playlist.id}
                onclick={() => void playPlaylist(playlist)}
              >
                {#if playlist.imageUrl}
                  <img class="playlist-art" src={playlist.imageUrl} alt="" />
                {:else}
                  <span class="playlist-art playlist-art-fallback" aria-hidden="true">♪</span>
                {/if}
                <span class="playlist-copy">
                  <strong>{playlist.name}</strong>
                  <em>
                    {playlist.ownerName ? `${playlist.ownerName} · ` : ""}{playlist.trackCount}
                    track{playlist.trackCount === 1 ? "" : "s"}
                  </em>
                </span>
                <span class="playlist-play">
                  {playingPlaylistId === playlist.id ? "Starting…" : "Play"}
                </span>
              </button>
            </li>
          {/each}
        </ul>
        {#if playlistsNextOffset != null && !playlistsQuery.trim()}
          <button
            type="button"
            class="playlist-more"
            disabled={playlistsLoadingMore}
            onclick={() => void loadPlaylists(false)}
          >
            {playlistsLoadingMore ? "Loading…" : "Load more"}
          </button>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<style>
  .playlist-overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: stretch;
    justify-content: flex-end;
    background: rgba(0, 0, 0, 0.52);
    padding: 12px;
    touch-action: manipulation;
  }

  .playlist-sheet {
    width: min(680px, 100%);
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    border-radius: 20px;
    background: #141414;
    color: #f8fafc;
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  .playlist-sheet-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .playlist-sheet-head h2 {
    margin: 0;
    font-size: 1.8rem;
    line-height: 1.1;
  }

  .playlist-sheet-head p {
    margin: 4px 0 0;
    color: #cbd5e1;
    font-size: 1rem;
  }

  .playlist-close,
  .playlist-more {
    min-height: 52px;
    padding: 10px 18px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 12px;
    background: transparent;
    color: #fff;
    font-size: 1rem;
    font-weight: 650;
    cursor: pointer;
    touch-action: manipulation;
  }

  .playlist-search input {
    width: 100%;
    min-height: 56px;
    padding: 12px 16px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 14px;
    background: #0d0d0d;
    color: #fff;
    font-size: 1.1rem;
  }

  .playlist-list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    flex: 1 1 auto;
    overscroll-behavior: contain;
    scrollbar-width: auto;
    scrollbar-color: #77808d #242424;
  }

  .playlist-list::-webkit-scrollbar {
    width: 18px;
  }

  .playlist-list::-webkit-scrollbar-track {
    margin: 2px 0;
    border-radius: 999px;
    background: #242424;
  }

  .playlist-list::-webkit-scrollbar-thumb {
    min-height: 72px;
    border: 4px solid #242424;
    border-radius: 999px;
    background: #77808d;
  }

  .playlist-list::-webkit-scrollbar-thumb:hover {
    background: #aab4c0;
  }

  .playlist-row {
    display: grid;
    grid-template-columns: 76px minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 84px;
    padding: 10px;
    border: none;
    border-radius: 14px;
    background: #1c1c1c;
    color: inherit;
    text-align: left;
    cursor: pointer;
    touch-action: manipulation;
  }

  .playlist-row:hover:not(:disabled) {
    background: #262626;
  }

  .playlist-art,
  .playlist-art-fallback {
    width: 76px;
    height: 76px;
    border-radius: 10px;
    object-fit: cover;
  }

  .playlist-art-fallback {
    display: grid;
    place-items: center;
    background: #2a2a2a;
    color: #94a3b8;
    font-size: 1.3rem;
  }

  .playlist-copy {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .playlist-copy strong,
  .playlist-copy em {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .playlist-copy strong {
    font-size: 1.05rem;
    line-height: 1.2;
  }

  .playlist-copy em {
    font-style: normal;
    color: #94a3b8;
    font-size: 0.95rem;
  }

  .playlist-play {
    min-width: 58px;
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(34, 197, 94, 0.1);
    font-weight: 700;
    color: #22c55e;
    text-align: center;
  }

  .playlist-status {
    margin: 12px 0 0;
    color: #cbd5e1;
  }

  .playlist-status-error {
    color: #fca5a5;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 640px) {
    .playlist-overlay {
      align-items: flex-end;
      padding: 8px;
    }

    .playlist-sheet {
      width: 100%;
      max-height: calc(100% - 8px);
      padding: 18px;
      border-radius: 20px 20px 14px 14px;
    }

    .playlist-row {
      grid-template-columns: 64px minmax(0, 1fr) auto;
      gap: 12px;
      min-height: 72px;
    }

    .playlist-art,
    .playlist-art-fallback {
      width: 64px;
      height: 64px;
    }

    .playlist-play {
      min-width: 52px;
      padding-inline: 8px;
    }
  }
</style>
