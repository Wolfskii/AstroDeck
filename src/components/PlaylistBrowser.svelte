<script lang="ts">
  import {
    executeActionValue,
    getPinnedPlaylists,
    listSpotifyPlaylists,
    playSpotifyPlaylist,
    setPlaylistPinned,
    type SpotifyPlaylist,
  } from "../services/api";

  let {
    open = false,
    preload = false,
    currentPlaylistId = null,
    onClose,
  }: {
    open?: boolean;
    /** Load the list in the background before it is opened, so opening needs no waiting. */
    preload?: boolean;
    /** The playlist Spotify is playing right now, wherever it is playing. */
    currentPlaylistId?: string | null;
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
  let pinnedIds = $state<string[]>([]);
  let playlistListEl = $state<HTMLElement | null>(null);
  let loadSeq = 0;
  let sawOpen = false;

  const isTauriRuntime =
    typeof window !== "undefined" &&
    !!(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

  const visiblePlaylists = $derived.by(() => {
    const query = playlistsQuery.trim().toLowerCase();
    // The list arrives ordered like Spotify's library: pinned first, then recently played.
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
      owned: false,
    },
    {
      id: "demo-liked",
      name: "Liked Songs",
      uri: "spotify:playlist:demo-liked",
      imageUrl: null,
      trackCount: 128,
      ownerName: "You",
      owned: true,
    },
    {
      id: "demo-focus",
      name: "Focus Flow",
      uri: "spotify:playlist:demo-focus",
      imageUrl: null,
      trackCount: 42,
      ownerName: "AstroDeck",
      owned: false,
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

  async function loadPlaylists(reset: boolean, allowAutoRetry = true, refresh = false) {
    if (!reset && (playlistsLoading || playlistsLoadingMore || playlistsNextOffset == null)) return;
    const seq = reset ? ++loadSeq : loadSeq;
    // Refreshing a list that is already on screen happens quietly, without a loading state.
    const quiet = refresh && reset && playlists.length > 0;
    if (reset) {
      if (!quiet) {
        playlistsLoading = true;
        playlistsError = null;
      }
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
      if (reset) {
        playlists = page.items;
      } else {
        const known = new Set(playlists.map((playlist) => playlist.id));
        playlists = [
          ...playlists,
          ...page.items.filter((playlist) => !known.has(playlist.id)),
        ];
      }
      playlistsTotal = page.total;
      playlistsNextOffset = page.nextOffset ?? null;
    } catch (e) {
      if (quiet) return;
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
    if (seq === loadSeq && playlistsQuery.trim() && playlistsNextOffset != null) {
      void loadAllPlaylists();
    }
  }

  async function loadAllPlaylists() {
    if (
      !isTauriRuntime ||
      playlistsLoading ||
      playlistsLoadingMore ||
      playlistsNextOffset == null
    ) {
      return;
    }
    const seq = loadSeq;
    playlistsLoadingMore = true;
    try {
      while (seq === loadSeq && playlistsNextOffset != null) {
        const page = await listSpotifyPlaylists({
          offset: playlistsNextOffset,
          limit: 50,
        });
        if (seq !== loadSeq) return;
        const known = new Set(playlists.map((playlist) => playlist.id));
        playlists = [
          ...playlists,
          ...page.items.filter((playlist) => !known.has(playlist.id)),
        ];
        playlistsTotal = page.total;
        playlistsNextOffset = page.nextOffset ?? null;
      }
    } catch (e) {
      if (seq === loadSeq) playlistsError = String(e);
    } finally {
      if (seq === loadSeq) playlistsLoadingMore = false;
    }
  }

  function maybeLoadMore() {
    if (
      !playlistListEl ||
      playlistsQuery.trim() ||
      playlistsLoading ||
      playlistsLoadingMore ||
      playlistsNextOffset == null
    ) {
      return;
    }
    const remaining =
      playlistListEl.scrollHeight - playlistListEl.scrollTop - playlistListEl.clientHeight;
    if (remaining <= 320) void loadPlaylists(false);
  }

  function handleSearchInput(event: Event) {
    playlistsQuery = (event.currentTarget as HTMLInputElement).value;
    if (playlistsQuery.trim() && playlistsNextOffset != null) {
      void loadAllPlaylists();
    }
  }

  $effect(() => {
    const isOpen = open;
    if (isOpen && !sawOpen) {
      playlistsQuery = "";
      playingPlaylistId = null;
      void getPinnedPlaylists()
        .then((ids) => (pinnedIds = ids))
        .catch(() => {});
      void loadPlaylists(true, true, true);
    }
    if (!isOpen && sawOpen) {
      loadSeq += 1;
    }
    sawOpen = isOpen;
  });

  let preloaded = false;
  $effect(() => {
    if (!preload || !isTauriRuntime || open || preloaded) return;
    preloaded = true;
    const timer = window.setTimeout(() => {
      void getPinnedPlaylists()
        .then((ids) => (pinnedIds = ids))
        .catch(() => {});
      void loadPlaylists(true, false, true);
    }, 1500);
    return () => window.clearTimeout(timer);
  });

  $effect(() => {
    const isOpen = open;
    const loadedCount = playlists.length;
    if (!isOpen || loadedCount === 0 || playlistsQuery.trim()) return;
    const timer = window.setTimeout(maybeLoadMore, 0);
    return () => window.clearTimeout(timer);
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

  // Pinning is done by pressing and holding a card (or right-clicking it), so the cards stay clean.
  const HOLD_TO_PIN_MS = 600;
  const HOLD_MOVE_LIMIT_PX = 12;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;
  let holdStart = { x: 0, y: 0 };
  let heldToPin = false;

  function startHold(event: PointerEvent, playlist: SpotifyPlaylist) {
    heldToPin = false;
    clearTimeout(holdTimer);
    if (event.button === 2) {
      heldToPin = true;
      void togglePin(playlist);
      return;
    }
    holdStart = { x: event.clientX, y: event.clientY };
    holdTimer = setTimeout(() => {
      heldToPin = true;
      void togglePin(playlist);
    }, HOLD_TO_PIN_MS);
  }

  function moveHold(event: PointerEvent) {
    if (
      Math.abs(event.clientX - holdStart.x) > HOLD_MOVE_LIMIT_PX ||
      Math.abs(event.clientY - holdStart.y) > HOLD_MOVE_LIMIT_PX
    ) {
      clearTimeout(holdTimer);
    }
  }

  function endHold() {
    clearTimeout(holdTimer);
  }

  function pressPlaylist(playlist: SpotifyPlaylist) {
    // The press that just pinned a playlist must not also start playing it.
    if (heldToPin) {
      heldToPin = false;
      return;
    }
    void playPlaylist(playlist);
  }

  async function togglePin(playlist: SpotifyPlaylist) {
    const pin = !pinnedIds.includes(playlist.id);
    try {
      pinnedIds = await setPlaylistPinned(playlist.id, pin);
      await loadPlaylists(true, false);
    } catch (e) {
      playlistsError = String(e);
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
            · Press and hold a playlist to pin it
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
          value={playlistsQuery}
          oninput={handleSearchInput}
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
        <p class="playlist-status">
          {playlistsLoadingMore && playlistsQuery.trim()
            ? "Searching all playlists…"
            : "No playlists match that search."}
        </p>
      {:else}
        {#if playlistsError}
          <p class="playlist-status playlist-status-error">{playlistsError}</p>
        {/if}
        <div class="playlist-list" bind:this={playlistListEl} onscroll={maybeLoadMore}>
          {#each visiblePlaylists as playlist (playlist.id)}
            <div class="playlist-row">
            <button
              type="button"
              class="playlist-card"
              class:playlist-card--selected={currentPlaylistId === playlist.id}
              disabled={playingPlaylistId === playlist.id}
              onclick={() => pressPlaylist(playlist)}
              onpointerdown={(event) => startHold(event, playlist)}
              onpointermove={moveHold}
              onpointerup={endHold}
              onpointerleave={endHold}
              onpointercancel={endHold}
              oncontextmenu={(event) => event.preventDefault()}
            >
              <span class="playlist-card-art-wrap">
                {#if playlist.imageUrl}
                  <img class="playlist-art" src={playlist.imageUrl} alt="" />
                {:else}
                  <span class="playlist-art playlist-art-fallback" aria-hidden="true">♪</span>
                {/if}
                {#if currentPlaylistId === playlist.id}
                  <span class="playlist-now-playing">
                    <span class="playlist-equalizer" aria-hidden="true"><i></i><i></i><i></i></span>
                    Now playing
                  </span>
                {/if}
              </span>
              <span class="playlist-copy">
                <strong>{playlist.name}</strong>
                <em>
                  {playlist.ownerName ? `${playlist.ownerName} · ` : ""}{playlist.trackCount}
                  track{playlist.trackCount === 1 ? "" : "s"}
                </em>
              </span>
              {#if playingPlaylistId === playlist.id}
                <span class="playlist-play">Starting…</span>
              {/if}
            </button>
            {#if pinnedIds.includes(playlist.id)}
            <button
              type="button"
              class="playlist-pin"
              aria-label={`Unpin ${playlist.name}`}
              title="Unpin"
              onclick={() => void togglePin(playlist)}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path
                  fill="currentColor"
                  d="M16 3a1 1 0 0 1 .7 1.7L15.4 6l1.6 5.2 2 1.8a1 1 0 0 1-.7 1.7H13v5.3a1 1 0 0 1-2 0v-5.3H5.7a1 1 0 0 1-.7-1.7l2-1.8L8.6 6 7.3 4.7A1 1 0 0 1 8 3h8Z"
                />
              </svg>
            </button>
            {/if}
            </div>
          {/each}
          {#if playlistsLoadingMore}
            <p class="playlist-loading-more">
              {playlistsQuery.trim() ? "Searching all playlists…" : "Loading more playlists…"}
            </p>
          {/if}
        </div>
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
    justify-content: stretch;
    background: #090909;
    padding: 0;
    touch-action: manipulation;
  }

  .playlist-sheet {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
    /* Keep clear of the side menu, which sits on top of the left edge. */
    padding: 28px 32px 28px calc(var(--side-inset, 0px) + 12px);
    background:
      radial-gradient(circle at 10% 0%, rgba(29, 185, 84, 0.12), transparent 32%),
      #141414;
    color: #f8fafc;
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
    font-size: clamp(1.8rem, 3vw, 2.7rem);
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
    margin: 0;
    padding: 0 16px 20px 0;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    align-content: start;
    gap: 18px;
    min-height: 0;
    flex: 1 1 auto;
    overscroll-behavior: contain;
    scrollbar-width: thick;
    scrollbar-color: #77808d #242424;
  }

  .playlist-list::-webkit-scrollbar {
    width: 56px;
  }

  .playlist-list::-webkit-scrollbar-track {
    margin: 2px 0;
    border-radius: 999px;
    background: #242424;
  }

  .playlist-list::-webkit-scrollbar-thumb {
    min-height: 88px;
    border: 5px solid #242424;
    border-radius: 999px;
    background: #77808d;
  }

  .playlist-list::-webkit-scrollbar-thumb:hover {
    background: #aab4c0;
  }

  .playlist-row {
    position: relative;
    display: flex;
    min-width: 0;
  }

  .playlist-pin {
    position: absolute;
    top: 20px;
    left: 20px;
    z-index: 2;
    display: grid;
    width: 46px;
    height: 46px;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.58);
    color: rgba(255, 255, 255, 0.78);
    cursor: pointer;
    touch-action: manipulation;
  }

  .playlist-pin svg {
    width: 24px;
    height: 24px;
    transform: rotate(35deg);
  }

  .playlist-pin:active {
    background: rgba(0, 0, 0, 0.8);
  }

  .playlist-pin {
    background: #1db954;
    color: #04130a;
  }

  .playlist-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    box-sizing: border-box;
    width: 100%;
    min-height: 300px;
    padding: 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    background: #1c1c1c;
    color: inherit;
    text-align: left;
    cursor: pointer;
    touch-action: manipulation;
    overflow: hidden;
  }

  .playlist-card:hover:not(:disabled) {
    background: #262626;
    border-color: rgba(255, 255, 255, 0.18);
  }

  .playlist-card--selected {
    border-color: rgba(29, 185, 84, 0.8);
    box-shadow: 0 0 0 2px rgba(29, 185, 84, 0.16);
  }

  .playlist-loading-more {
    grid-column: 1 / -1;
    margin: 8px 0 4px;
    padding: 18px;
    color: #cbd5e1;
    font-size: 1rem;
    font-weight: 650;
    text-align: center;
  }

  .playlist-card-art-wrap {
    position: relative;
    display: block;
    flex: 0 0 auto;
    width: 100%;
    aspect-ratio: 1;
    margin-bottom: 14px;
    overflow: hidden;
    border-radius: 12px;
  }

  .playlist-art,
  .playlist-art-fallback {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    border-radius: 12px;
    object-fit: cover;
  }

  .playlist-art-fallback {
    display: grid;
    place-items: center;
    background: #2a2a2a;
    color: #94a3b8;
    font-size: 1.3rem;
  }

  .playlist-now-playing {
    position: absolute;
    right: 10px;
    bottom: 10px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 32px;
    padding: 5px 10px;
    border-radius: 999px;
    background: #1db954;
    color: #071b0d;
    font-size: 0.8rem;
    font-weight: 800;
  }

  .playlist-equalizer {
    display: inline-flex;
    align-items: end;
    gap: 2px;
    height: 13px;
  }

  .playlist-equalizer i {
    display: block;
    width: 2px;
    height: 8px;
    border-radius: 2px;
    background: currentColor;
  }

  .playlist-equalizer i:nth-child(2) {
    height: 13px;
  }

  .playlist-equalizer i:nth-child(3) {
    height: 10px;
  }

  .playlist-copy {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 3.8em;
    overflow: hidden;
    flex: 1 1 auto;
  }

  .playlist-copy strong,
  .playlist-copy em {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .playlist-copy strong {
    font-size: 1.15rem;
    line-height: 1.25;
    white-space: normal;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .playlist-copy em {
    font-style: normal;
    color: #94a3b8;
    font-size: 0.95rem;
  }

  .playlist-play {
    align-self: flex-start;
    min-width: 72px;
    margin-top: 14px;
    padding: 10px 14px;
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
      align-items: stretch;
    }

    .playlist-sheet {
      width: 100%;
      padding: 20px 16px;
    }

    .playlist-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 12px;
      padding-right: 12px;
    }

    .playlist-card {
      min-height: 0;
      padding: 9px;
    }

    .playlist-copy strong {
      font-size: 1rem;
    }

    .playlist-play {
      min-width: 62px;
      margin-top: 10px;
      padding-inline: 9px;
    }
  }
</style>
