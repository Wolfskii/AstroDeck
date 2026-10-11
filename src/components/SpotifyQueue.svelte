<script lang="ts">
  import {
    getSpotifyQueue,
    moveSpotifyQueueItem,
    playSpotifyQueueItem,
    removeSpotifyQueueItem,
    type SpotifyQueueView,
    type SpotifyTrackPreview,
  } from "../services/api";

  let {
    open = false,
    trackId = null,
    onClose,
  }: {
    open?: boolean;
    /** The track playing now; the queue is fetched again when it changes. */
    trackId?: string | null;
    onClose: () => void;
  } = $props();

  interface QueueRow {
    uid: number;
    track: SpotifyTrackPreview;
  }

  const PAGE = 50;
  const ROW_GAP_PX = 8;
  const AUTO_SCROLL_EDGE_PX = 72;
  const AUTO_SCROLL_STEP_PX = 14;

  let view = $state<SpotifyQueueView | null>(null);
  let rows = $state<QueueRow[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  let busyUid = $state<number | null>(null);
  let listEl = $state<HTMLElement | null>(null);
  let nextUid = 0;
  let loadSeq = 0;
  let sawOpen = false;
  let shownTrackId: string | null = null;

  let dragIndex = $state<number | null>(null);
  let dragOffset = $state(0);
  let dragPitch = 84;
  let dragStartY = 0;
  let dragStartScroll = 0;
  let dragPointerY = 0;
  let autoScrollFrame = 0;

  const isTauriRuntime =
    typeof window !== "undefined" &&
    !!(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

  const editable = $derived(view?.editable ?? false);
  const dropIndex = $derived.by(() => {
    if (dragIndex == null) return null;
    const target = dragIndex + Math.round(dragOffset / dragPitch);
    return Math.max(0, Math.min(rows.length - 1, target));
  });

  const BROWSER_DEMO_QUEUE: SpotifyQueueView = {
    contextUri: "spotify:playlist:demo",
    contextName: "Focus Flow",
    current: {
      itemId: "demo-now",
      itemType: "track",
      trackName: "Now Playing Song",
      artistName: "AstroDeck",
      durationMs: 201_000,
    },
    upNext: ["Second Song", "Third Song", "Fourth Song", "Fifth Song"].map((name, index) => ({
      itemId: `demo-${index}`,
      itemType: "track",
      trackName: name,
      artistName: "Preview Artist",
      durationMs: 180_000 + index * 13_000,
    })),
    total: 4,
    offset: 0,
    editable: true,
    message: "Preview queue. Connect in the desktop app to see yours.",
  };

  function toRows(tracks: SpotifyTrackPreview[]): QueueRow[] {
    return tracks.map((track) => ({ uid: nextUid++, track }));
  }

  function cleanError(e: unknown): string {
    return String(e).replace(/^Error:\s*/i, "");
  }

  async function load(quiet = false) {
    const seq = ++loadSeq;
    if (!quiet) {
      loading = true;
      error = null;
    }
    try {
      const next = isTauriRuntime ? await getSpotifyQueue({ limit: PAGE }) : BROWSER_DEMO_QUEUE;
      if (seq !== loadSeq) return;
      view = next;
      rows = toRows(next.upNext);
      total = next.total;
      shownTrackId = next.current?.itemId ?? null;
      error = null;
    } catch (e) {
      if (seq === loadSeq) error = cleanError(e);
    } finally {
      if (seq === loadSeq) loading = false;
    }
  }

  async function loadMore() {
    if (!isTauriRuntime || loading || loadingMore || rows.length >= total) return;
    const seq = loadSeq;
    loadingMore = true;
    try {
      const page = await getSpotifyQueue({ offset: rows.length, limit: PAGE });
      if (seq !== loadSeq) return;
      rows = [...rows, ...toRows(page.upNext)];
      total = page.total;
    } catch (e) {
      if (seq === loadSeq) error = cleanError(e);
    } finally {
      loadingMore = false;
    }
  }

  function maybeLoadMore() {
    if (!listEl || dragIndex != null) return;
    const remaining = listEl.scrollHeight - listEl.scrollTop - listEl.clientHeight;
    if (remaining <= 320) void loadMore();
  }

  $effect(() => {
    const isOpen = open;
    if (isOpen && !sawOpen) {
      view = null;
      rows = [];
      void load();
    }
    if (!isOpen && sawOpen) {
      loadSeq += 1;
      endDrag(false);
    }
    sawOpen = isOpen;
  });

  // A new track (skipped, ended, or picked here) moves the queue along, so show it again.
  $effect(() => {
    const id = trackId;
    if (!open || loading || dragIndex != null || id == null || id === shownTrackId) return;
    void load(true);
  });

  function close() {
    endDrag(false);
    onClose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      event.preventDefault();
      close();
    }
  }

  async function moveRow(from: number, to: number) {
    if (from === to || to < 0 || to >= rows.length) return;
    const row = rows[from];
    const reordered = [...rows];
    reordered.splice(from, 1);
    reordered.splice(to, 0, row);
    rows = reordered;
    if (!isTauriRuntime) return;
    try {
      await moveSpotifyQueueItem(from, to, row.track.itemId);
    } catch (e) {
      error = cleanError(e);
      await load(true);
    }
  }

  async function removeRow(index: number) {
    const row = rows[index];
    if (!row) return;
    rows = rows.filter((_, i) => i !== index);
    total = Math.max(0, total - 1);
    if (!isTauriRuntime) return;
    try {
      await removeSpotifyQueueItem(index, row.track.itemId);
    } catch (e) {
      error = cleanError(e);
      await load(true);
    }
  }

  async function playRow(index: number) {
    const row = rows[index];
    if (!row || !editable || busyUid != null || dragIndex != null) return;
    if (!isTauriRuntime) return;
    busyUid = row.uid;
    try {
      await playSpotifyQueueItem(index, row.track.itemId);
      await load(true);
    } catch (e) {
      error = cleanError(e);
      await load(true);
    } finally {
      busyUid = null;
    }
  }

  function startDrag(event: PointerEvent, index: number) {
    if (!editable || event.button > 0) return;
    event.preventDefault();
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    const rowEl = handle.closest<HTMLElement>(".queue-row");
    dragPitch = (rowEl?.offsetHeight ?? 76) + ROW_GAP_PX;
    dragStartY = event.clientY;
    dragPointerY = event.clientY;
    dragStartScroll = listEl?.scrollTop ?? 0;
    dragOffset = 0;
    dragIndex = index;
    autoScrollFrame = requestAnimationFrame(autoScroll);
  }

  function updateDragOffset() {
    const scrolled = (listEl?.scrollTop ?? 0) - dragStartScroll;
    dragOffset = dragPointerY - dragStartY + scrolled;
  }

  function moveDrag(event: PointerEvent) {
    if (dragIndex == null) return;
    dragPointerY = event.clientY;
    updateDragOffset();
  }

  // Holding a dragged row near the top or bottom edge scrolls the list under it.
  function autoScroll() {
    if (dragIndex == null || !listEl) return;
    const rect = listEl.getBoundingClientRect();
    let step = 0;
    if (dragPointerY < rect.top + AUTO_SCROLL_EDGE_PX) step = -AUTO_SCROLL_STEP_PX;
    else if (dragPointerY > rect.bottom - AUTO_SCROLL_EDGE_PX) step = AUTO_SCROLL_STEP_PX;
    if (step !== 0) {
      listEl.scrollTop += step;
      updateDragOffset();
    }
    autoScrollFrame = requestAnimationFrame(autoScroll);
  }

  function endDrag(commit: boolean) {
    cancelAnimationFrame(autoScrollFrame);
    const from = dragIndex;
    const to = dropIndex;
    dragIndex = null;
    dragOffset = 0;
    if (commit && from != null && to != null && from !== to) {
      void moveRow(from, to);
    }
  }

  function handleKeyMove(event: KeyboardEvent, index: number) {
    if (event.key === "ArrowUp") {
      event.preventDefault();
      void moveRow(index, index - 1);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      void moveRow(index, index + 1);
    }
  }

  function rowShift(index: number): number {
    if (dragIndex == null || dropIndex == null || index === dragIndex) return 0;
    if (dragIndex < dropIndex && index > dragIndex && index <= dropIndex) return -dragPitch;
    if (dragIndex > dropIndex && index >= dropIndex && index < dragIndex) return dragPitch;
    return 0;
  }

  function formatTime(ms: number | null | undefined): string {
    if (ms == null) return "";
    const totalSec = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(totalSec / 60)}:${(totalSec % 60).toString().padStart(2, "0")}`;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div
    class="queue-overlay"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Queue"
    onclick={(event) => {
      if (event.target === event.currentTarget) close();
    }}
    onkeydown={(event) => {
      if (event.key === "Escape") close();
    }}
  >
    <div class="queue-sheet">
      <header class="queue-head">
        <div>
          <h2>Queue</h2>
          <p>
            {#if view?.contextName}
              Playing from {view.contextName}
            {:else if total > 0}
              {total} track{total === 1 ? "" : "s"} up next
            {:else}
              What plays next
            {/if}
            {#if editable && rows.length > 1}
              · Drag a track by its handle to move it
            {/if}
          </p>
        </div>
        <button type="button" class="queue-close" aria-label="Close queue" onclick={close}>
          Close
        </button>
      </header>

      {#if view?.message}
        <p class="queue-status">{view.message}</p>
      {/if}
      {#if error}
        <p class="queue-status queue-status-error">{error}</p>
      {/if}

      {#if loading && !view}
        <p class="queue-status">Loading the queue…</p>
      {:else if view}
        <div class="queue-list" bind:this={listEl} onscroll={maybeLoadMore}>
          {#if view.current}
            <h3 class="queue-section">Now playing</h3>
            <div class="queue-row queue-row--current">
              {#if view.current.coverArtUrl}
                <img class="queue-art" src={view.current.coverArtUrl} alt="" />
              {:else}
                <span class="queue-art queue-art-fallback" aria-hidden="true">♪</span>
              {/if}
              <span class="queue-copy">
                <strong>{view.current.trackName}</strong>
                <em>{view.current.artistName ?? ""}</em>
              </span>
              <span class="queue-time">{formatTime(view.current.durationMs)}</span>
            </div>
          {/if}

          <h3 class="queue-section">
            Next up{total > 0 ? ` · ${total}` : ""}
          </h3>
          {#if rows.length === 0}
            <p class="queue-status">Nothing is queued after this track.</p>
          {/if}
          <ol class="queue-rows">
            {#each rows as row, index (row.uid)}
              <li
                class="queue-row"
                class:queue-row--dragging={dragIndex === index}
                class:queue-row--settling={dragIndex != null && dragIndex !== index}
                style={`transform: translateY(${dragIndex === index ? dragOffset : rowShift(index)}px);`}
              >
                {#if editable}
                  <button
                    type="button"
                    class="queue-handle"
                    aria-label={`Move ${row.track.trackName}. Use the arrow keys to move it up or down.`}
                    title="Drag to move"
                    onpointerdown={(event) => startDrag(event, index)}
                    onpointermove={moveDrag}
                    onpointerup={() => endDrag(true)}
                    onpointercancel={() => endDrag(false)}
                    onkeydown={(event) => handleKeyMove(event, index)}
                    oncontextmenu={(event) => event.preventDefault()}
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path fill="currentColor" d="M4 7h16v2H4V7zm0 4h16v2H4v-2zm0 4h16v2H4v-2z" />
                    </svg>
                  </button>
                {/if}
                <button
                  type="button"
                  class="queue-track"
                  disabled={!editable || busyUid != null}
                  title={editable ? "Play this track now" : undefined}
                  onclick={() => void playRow(index)}
                >
                  {#if row.track.coverArtUrl}
                    <img class="queue-art" src={row.track.coverArtUrl} alt="" loading="lazy" />
                  {:else}
                    <span class="queue-art queue-art-fallback" aria-hidden="true">♪</span>
                  {/if}
                  <span class="queue-copy">
                    <strong>{row.track.trackName}</strong>
                    <em>{row.track.artistName ?? ""}</em>
                  </span>
                  <span class="queue-time">
                    {busyUid === row.uid ? "Starting…" : formatTime(row.track.durationMs)}
                  </span>
                </button>
                {#if editable}
                  <button
                    type="button"
                    class="queue-remove"
                    aria-label={`Remove ${row.track.trackName} from the queue`}
                    title="Remove from queue"
                    onclick={() => void removeRow(index)}
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path
                        fill="currentColor"
                        d="M6.4 5 12 10.6 17.6 5 19 6.4 13.4 12l5.6 5.6-1.4 1.4-5.6-5.6L6.4 19 5 17.6l5.6-5.6L5 6.4 6.4 5z"
                      />
                    </svg>
                  </button>
                {/if}
              </li>
            {/each}
          </ol>
          {#if loadingMore}
            <p class="queue-loading-more">Loading more tracks…</p>
          {:else if rows.length < total}
            <button type="button" class="queue-more" onclick={() => void loadMore()}>
              Show more ({total - rows.length} more)
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .queue-overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    background: #090909;
    touch-action: manipulation;
  }

  .queue-sheet {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
    /* Keep clear of the side menu, which sits on top of the left edge. */
    padding: 28px 32px 28px calc(var(--side-inset, 0px) + 12px);
    background:
      radial-gradient(circle at 10% 0%, rgba(29, 185, 84, 0.12), transparent 32%),
      #141414;
    color: #f8fafc;
    overflow: hidden;
  }

  .queue-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .queue-head h2 {
    margin: 0;
    font-size: clamp(1.8rem, 3vw, 2.7rem);
    line-height: 1.1;
  }

  .queue-head p {
    margin: 4px 0 0;
    color: #cbd5e1;
    font-size: 1rem;
  }

  .queue-close,
  .queue-more {
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

  .queue-more {
    align-self: center;
    margin: 8px auto 4px;
  }

  .queue-status {
    margin: 0;
    color: #cbd5e1;
  }

  .queue-status-error {
    color: #fca5a5;
  }

  .queue-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    flex: 1 1 auto;
    overflow: auto;
    padding: 0 16px 20px 0;
    overscroll-behavior: contain;
    scrollbar-width: thick;
    scrollbar-color: #77808d #242424;
  }

  .queue-list::-webkit-scrollbar {
    width: 56px;
  }

  .queue-list::-webkit-scrollbar-track {
    margin: 2px 0;
    border-radius: 999px;
    background: #242424;
  }

  .queue-list::-webkit-scrollbar-thumb {
    min-height: 88px;
    border: 5px solid #242424;
    border-radius: 999px;
    background: #77808d;
  }

  .queue-section {
    margin: 12px 0 2px;
    color: #94a3b8;
    font-size: 0.95rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .queue-rows {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .queue-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 76px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    background: #1c1c1c;
  }

  .queue-row--current {
    gap: 14px;
    padding: 8px 16px 8px 8px;
    border-color: rgba(29, 185, 84, 0.6);
  }

  .queue-row--current strong {
    color: #1ed760;
  }

  .queue-row--settling {
    transition: transform 0.16s ease;
  }

  .queue-row--dragging {
    z-index: 2;
    background: #2a2a2a;
    border-color: rgba(29, 185, 84, 0.8);
    box-shadow: 0 14px 30px rgba(0, 0, 0, 0.5);
  }

  .queue-handle,
  .queue-remove {
    display: grid;
    flex: 0 0 auto;
    width: 56px;
    height: 64px;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: #94a3b8;
    cursor: pointer;
  }

  .queue-handle {
    cursor: grab;
    touch-action: none;
  }

  .queue-row--dragging .queue-handle {
    cursor: grabbing;
    color: #1ed760;
  }

  .queue-handle svg,
  .queue-remove svg {
    width: 26px;
    height: 26px;
  }

  .queue-handle:hover,
  .queue-remove:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.06);
  }

  .queue-track {
    display: flex;
    flex: 1 1 auto;
    align-items: center;
    gap: 14px;
    min-width: 0;
    min-height: 64px;
    padding: 6px 8px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    touch-action: manipulation;
  }

  .queue-track:disabled {
    cursor: default;
  }

  .queue-track:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.05);
  }

  .queue-art {
    flex: 0 0 auto;
    width: 56px;
    height: 56px;
    border-radius: 8px;
    object-fit: cover;
  }

  .queue-art-fallback {
    display: grid;
    place-items: center;
    background: #2a2a2a;
    color: #94a3b8;
  }

  .queue-copy {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .queue-copy strong,
  .queue-copy em {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .queue-copy strong {
    font-size: 1.08rem;
  }

  .queue-copy em {
    color: #94a3b8;
    font-size: 0.95rem;
    font-style: normal;
  }

  .queue-time {
    flex: 0 0 auto;
    color: #94a3b8;
    font-size: 0.95rem;
    font-variant-numeric: tabular-nums;
  }

  .queue-loading-more {
    margin: 8px 0 4px;
    padding: 18px;
    color: #cbd5e1;
    font-weight: 650;
    text-align: center;
  }

  @media (max-width: 640px) {
    .queue-sheet {
      padding: 20px 16px;
    }

    .queue-list {
      padding-right: 8px;
    }

    .queue-handle,
    .queue-remove {
      width: 48px;
    }

    .queue-art {
      width: 48px;
      height: 48px;
    }
  }
</style>
