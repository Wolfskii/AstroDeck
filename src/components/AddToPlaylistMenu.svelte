<script lang="ts">
  import {
    addCurrentYouTubeMusicTrackToPlaylist,
    createSpotifyPlaylist,
    createYouTubeMusicPlaylist,
    getYouTubeMusicLibrary,
    listSpotifyAddPlaylists,
    removeYouTubeMusicTrackFromPlaylist,
    setSpotifyPlaylistTrack,
    type SpotifyAddPlaylist,
    type YouTubeLocalLibrary,
  } from "../services/api";

  export type PlaylistPickerProvider = "spotify" | "youtubeMusic";

  interface PickerItem {
    id: string;
    name: string;
    imageUrl?: string | null;
    trackCount: number;
    containsTrack: boolean;
    isCurrent: boolean;
  }

  let {
    open = false,
    provider,
    trackId = null,
    currentPlaylistId = null,
    onClose,
    onMembershipChange,
  }: {
    open?: boolean;
    provider: PlaylistPickerProvider;
    trackId?: string | null;
    currentPlaylistId?: string | null;
    onClose: () => void;
    onMembershipChange?: (inCurrentOwnedPlaylist: boolean) => void;
  } = $props();

  let items = $state<PickerItem[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let query = $state("");
  let creating = $state(false);
  let newName = $state("");
  let busyId = $state<string | null>(null);

  const visible = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const filtered = !needle
      ? items
      : items.filter((item) => item.name.toLowerCase().includes(needle));
    const saved = filtered.filter((item) => item.containsTrack);
    const rest = filtered.filter((item) => !item.containsTrack);
    return { saved, rest };
  });

  $effect(() => {
    if (!open) {
      query = "";
      creating = false;
      newName = "";
      error = null;
      return;
    }
    trackId;
    currentPlaylistId;
    void loadItems();
  });

  function close() {
    onClose();
  }

  function fromSpotify(playlists: SpotifyAddPlaylist[]): PickerItem[] {
    return playlists.map((playlist) => ({
      id: playlist.id,
      name: playlist.name,
      imageUrl: playlist.imageUrl,
      trackCount: playlist.trackCount,
      containsTrack: playlist.containsTrack,
      isCurrent: playlist.isCurrent,
    }));
  }

  function fromYouTube(library: YouTubeLocalLibrary, id: string | null): PickerItem[] {
    return [...library.playlists]
      .map((playlist) => ({
        id: playlist.id,
        name: playlist.name,
        imageUrl: null,
        trackCount: playlist.trackIds.length,
        containsTrack: id ? playlist.trackIds.includes(id) : false,
        isCurrent: playlist.id === currentPlaylistId,
      }))
      .sort((a, b) => Number(b.isCurrent) - Number(a.isCurrent) || Number(b.containsTrack) - Number(a.containsTrack));
  }

  function publishMembership(next: PickerItem[]) {
    onMembershipChange?.(next.some((item) => item.isCurrent && item.containsTrack));
  }

  async function loadItems() {
    loading = true;
    error = null;
    try {
      if (provider === "spotify") {
        if (!trackId) {
          items = [];
          return;
        }
        items = fromSpotify(await listSpotifyAddPlaylists(trackId));
      } else {
        items = fromYouTube(await getYouTubeMusicLibrary(), trackId);
      }
      publishMembership(items);
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      loading = false;
    }
  }

  async function togglePlaylist(item: PickerItem) {
    if (!trackId || busyId) return;
    busyId = item.id;
    error = null;
    const add = !item.containsTrack;
    try {
      if (provider === "spotify") {
        items = fromSpotify(await setSpotifyPlaylistTrack(item.id, trackId, add));
      } else if (add) {
        const library = await addCurrentYouTubeMusicTrackToPlaylist(item.id);
        items = fromYouTube(library, trackId);
      } else {
        const library = await removeYouTubeMusicTrackFromPlaylist(item.id, trackId);
        items = fromYouTube(library, trackId);
      }
      publishMembership(items);
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      busyId = null;
    }
  }

  async function createPlaylist() {
    const name = newName.trim();
    if (!name || busyId) return;
    busyId = "new";
    error = null;
    try {
      if (provider === "spotify") {
        items = fromSpotify(await createSpotifyPlaylist(name, trackId));
      } else {
        await createYouTubeMusicPlaylist(name);
        const library = await getYouTubeMusicLibrary();
        const created = library.playlists[library.playlists.length - 1];
        if (created && trackId) {
          items = fromYouTube(await addCurrentYouTubeMusicTrackToPlaylist(created.id), trackId);
        } else {
          items = fromYouTube(library, trackId);
        }
      }
      creating = false;
      newName = "";
      publishMembership(items);
    } catch (e) {
      error = String(e).replace(/^Error:\s*/i, "");
    } finally {
      busyId = null;
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (open && event.key === "Escape") close();
  }}
/>

{#if open}
  <div
    class="add-overlay"
    role="dialog"
    aria-modal="true"
    aria-label="Add to playlist"
    onclick={(event) => {
      if (event.target === event.currentTarget) close();
    }}
  >
    <div class="add-sheet">
      <header class="add-head">
        <h2>Add to playlist</h2>
        <button type="button" class="add-close" onclick={close}>Close</button>
      </header>

      <label class="add-search">
        <span class="sr-only">Find a playlist</span>
        <input
          type="search"
          placeholder="Find a playlist"
          bind:value={query}
          autocomplete="off"
        />
      </label>

      {#if creating}
        <form
          class="add-create-form"
          onsubmit={(event) => {
            event.preventDefault();
            void createPlaylist();
          }}
        >
          <input bind:value={newName} placeholder="Playlist name" />
          <button type="submit" disabled={!newName.trim() || busyId === "new"}>Create</button>
        </form>
      {:else}
        <button type="button" class="add-new" onclick={() => (creating = true)}>
          <span class="add-new-icon" aria-hidden="true">+</span>
          New playlist
        </button>
      {/if}

      {#if loading}
        <p class="add-status">Loading your playlists…</p>
      {:else if error && items.length === 0}
        <p class="add-status add-error">{error}</p>
      {:else if visible.saved.length === 0 && visible.rest.length === 0}
        <p class="add-status">No playlists yet. Create one to save this song.</p>
      {:else}
        {#if error}
          <p class="add-status add-error">{error}</p>
        {/if}
        {#if visible.saved.length > 0}
          <h3>Saved in</h3>
          <div class="add-list">
            {#each visible.saved as item (item.id)}
              <button
                type="button"
                class="add-row"
                class:add-row-current={item.isCurrent}
                disabled={busyId === item.id}
                onclick={() => void togglePlaylist(item)}
              >
                <span class="add-art" aria-hidden="true">
                  {#if item.imageUrl}
                    <img src={item.imageUrl} alt="" />
                  {:else}
                    ♫
                  {/if}
                </span>
                <span class="add-copy">
                  <strong>{item.name}</strong>
                  <em>{item.trackCount} songs{item.isCurrent ? " · Now playing" : ""}</em>
                </span>
                <span class="add-check" aria-hidden="true">
                  <svg viewBox="0 0 24 24">
                    <path
                      fill="currentColor"
                      d="M9.2 16.2 5.5 12.5l1.4-1.4 2.3 2.3 6-6 1.4 1.4-7.4 7.4z"
                    />
                  </svg>
                </span>
              </button>
            {/each}
          </div>
        {/if}
        {#if visible.rest.length > 0}
          <h3>{visible.saved.length > 0 ? "Your playlists" : "Playlists"}</h3>
          <div class="add-list">
            {#each visible.rest as item (item.id)}
              <button
                type="button"
                class="add-row"
                disabled={busyId === item.id}
                onclick={() => void togglePlaylist(item)}
              >
                <span class="add-art" aria-hidden="true">
                  {#if item.imageUrl}
                    <img src={item.imageUrl} alt="" />
                  {:else}
                    ♫
                  {/if}
                </span>
                <span class="add-copy">
                  <strong>{item.name}</strong>
                  <em>{item.trackCount} songs</em>
                </span>
                <span class="add-empty" aria-hidden="true"></span>
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<style>
  .add-overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: end center;
    padding: 18px;
    background: rgba(0, 0, 0, 0.55);
  }

  .add-sheet {
    width: min(520px, 100%);
    max-height: min(78vh, 760px);
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 18px 18px;
    overflow: hidden;
    border-radius: 22px;
    background: #121212;
    color: #fff;
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.45);
  }

  .add-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .add-head h2,
  h3 {
    margin: 0;
  }

  .add-head h2 {
    font-size: 1.35rem;
  }

  h3 {
    color: #b3b3b3;
    font-size: 0.82rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .add-close,
  .add-create-form button {
    min-height: 44px;
    padding: 8px 14px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 12px;
    background: transparent;
    color: #fff;
    font-weight: 700;
    cursor: pointer;
  }

  .add-search input,
  .add-create-form input {
    width: 100%;
    min-height: 52px;
    padding: 12px 16px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 999px;
    background: #2a2a2a;
    color: #fff;
    font-size: 1.05rem;
  }

  .add-new,
  .add-row {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    min-height: 64px;
    padding: 10px 8px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    touch-action: manipulation;
  }

  .add-new {
    font-size: 1.05rem;
    font-weight: 750;
  }

  .add-new-icon {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 10px;
    background: #2a2a2a;
    font-size: 1.8rem;
    line-height: 1;
  }

  .add-create-form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
  }

  .add-create-form button {
    background: #1db954;
    border-color: #1db954;
    color: #071b0d;
  }

  .add-list {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
    padding-right: 8px;
    scrollbar-width: thick;
    scrollbar-color: #77808d #242424;
  }

  .add-row:hover,
  .add-new:hover {
    background: #1a1a1a;
  }

  .add-row-current {
    background: rgba(29, 185, 84, 0.08);
  }

  .add-art {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    overflow: hidden;
    border-radius: 8px;
    background: #2a2a2a;
    color: #94a3b8;
    flex: 0 0 auto;
  }

  .add-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .add-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1 1 auto;
  }

  .add-copy strong,
  .add-copy em {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .add-copy em {
    font-style: normal;
    color: #b3b3b3;
    font-size: 0.9rem;
  }

  .add-check,
  .add-empty {
    width: 28px;
    height: 28px;
    flex: 0 0 auto;
    border-radius: 50%;
  }

  .add-check {
    display: grid;
    place-items: center;
    background: #1db954;
    color: #121212;
  }

  .add-check svg {
    width: 20px;
    height: 20px;
  }

  .add-empty {
    border: 2px solid #7a7a7a;
  }

  .add-status {
    margin: 8px 0 0;
    color: #cbd5e1;
  }

  .add-error {
    color: #fca5a5;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }
</style>
