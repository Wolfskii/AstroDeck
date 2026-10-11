<script lang="ts">
  import AppGlyph from "./AppGlyph.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import ViewIcon from "./ViewIcon.svelte";
  import { BUILTIN_APPS } from "../lib/apps";
  import { VIEW_ACCENT_COLORS, newCustomViewId, normalizeEmbedUrl, type CustomView } from "../lib/customViews";
  import { VIEW_ICONS } from "../lib/viewIcons";

  let {
    views,
    disabledApps = [],
    busy = false,
    toggleBusyId = null,
    onSave,
    onOpen,
    onSetEnabled,
  }: {
    views: CustomView[];
    disabledApps?: string[];
    busy?: boolean;
    toggleBusyId?: string | null;
    onSave: (views: CustomView[]) => Promise<void>;
    onOpen?: (id: string) => void;
    onSetEnabled?: (id: string, enabled: boolean) => Promise<void>;
  } = $props();

  let formOpen = $state(false);
  let editingId = $state<string | null>(null);
  let name = $state("");
  let url = $state("");
  let icon = $state("globe");
  let color = $state(VIEW_ACCENT_COLORS[0]);
  let iconQuery = $state("");
  let formError = $state<string | null>(null);

  const filteredIcons = $derived.by(() => {
    const query = iconQuery.trim().toLowerCase().replace(/\s+/g, "-");
    if (!query) return VIEW_ICONS;
    return VIEW_ICONS.filter(
      (item) => item.id.includes(query) || item.label.toLowerCase().includes(iconQuery.trim().toLowerCase())
    );
  });

  function resetForm() {
    formOpen = false;
    editingId = null;
    name = "";
    url = "";
    icon = "globe";
    color = VIEW_ACCENT_COLORS[0];
    iconQuery = "";
    formError = null;
  }

  function editView(view: CustomView) {
    formOpen = true;
    editingId = view.id;
    name = view.name;
    url = view.url;
    icon = view.icon;
    color = view.color;
    formError = null;
  }

  async function saveView() {
    const nextName = name.trim();
    const nextUrl = normalizeEmbedUrl(url);
    if (!nextName) {
      formError = "Give the app a name.";
      return;
    }
    if (nextName.length > 40) {
      formError = "Use a name of 40 characters or fewer.";
      return;
    }
    if (!nextUrl) {
      formError = "Enter an http or https website address.";
      return;
    }
    const next: CustomView = {
      id: editingId ?? newCustomViewId(),
      name: nextName,
      url: nextUrl,
      icon,
      color,
    };
    const without = views.filter((view) => view.id !== next.id);
    formError = null;
    try {
      await onSave([...without, next]);
      resetForm();
    } catch (error) {
      formError = error instanceof Error ? error.message : String(error);
    }
  }

  async function removeView(id: string) {
    try {
      await onSave(views.filter((view) => view.id !== id));
      if (editingId === id) resetForm();
    } catch (error) {
      formError = error instanceof Error ? error.message : String(error);
    }
  }
</script>

<div class="custom-views">
  <div class="card">
    <p class="title">Apps</p>
    <p class="desc">Turn an app off to take it out of the sidebar. Settings stays available so you can turn it back on.</p>
    <ul class="app-list">
      {#each BUILTIN_APPS as app (app.id)}
        <li>
          <span class="view-mark" style={`color: ${app.color}`}>
            <AppGlyph id={app.id} />
          </span>
          <span class="view-copy">
            <strong>{app.name}</strong>
          </span>
          <ToggleSwitch
            checked={!disabledApps.includes(app.id)}
            disabled={toggleBusyId === app.id}
            label={`${disabledApps.includes(app.id) ? "Show" : "Hide"} ${app.name}`}
            onchange={(enabled) => void onSetEnabled?.(app.id, enabled)}
          />
        </li>
      {/each}
      {#each views as view (view.id)}
        <li>
          <span class="view-mark" style={`color: ${view.color}`}>
            <ViewIcon name={view.icon} />
          </span>
          <span class="view-copy">
            <strong>{view.name}</strong>
            <span>{view.url}</span>
          </span>
          <span class="view-actions">
            <button type="button" class="btn" disabled={busy} onclick={() => onOpen?.(view.id)}>Open</button>
            <button type="button" class="btn" disabled={busy} onclick={() => editView(view)}>Edit</button>
            <button type="button" class="btn danger" disabled={busy} onclick={() => void removeView(view.id)}>
              Remove
            </button>
          </span>
          <ToggleSwitch
            checked={!disabledApps.includes(view.id)}
            disabled={toggleBusyId === view.id}
            label={`${disabledApps.includes(view.id) ? "Show" : "Hide"} ${view.name}`}
            onchange={(enabled) => void onSetEnabled?.(view.id, enabled)}
          />
        </li>
      {/each}
    </ul>
  </div>

  {#if !formOpen}
    <div class="card add-row">
      <div class="add-copy">
        <p class="title">Add a website</p>
        <p class="desc">Show any site as its own app in the sidebar.</p>
      </div>
      <button type="button" class="btn primary" disabled={busy} onclick={() => (formOpen = true)}>
        + Add website
      </button>
    </div>
  {:else}
  <div class="card">
    <p class="title">{editingId ? "Edit app" : "Add a website"}</p>
    <p class="desc">
      Add a site by address. It shows up in the list above and in the sidebar, filling the window
      with the sidebar still on top. Some sites refuse to be embedded and will stay blank.
    </p>
    <label class="field">
      <span>Name</span>
      <input class="input" type="text" maxlength="40" placeholder="News" bind:value={name} disabled={busy} />
    </label>
    <label class="field">
      <span>Website address</span>
      <input
        class="input"
        type="text"
        inputmode="url"
        spellcheck="false"
        autocomplete="off"
        placeholder="https://example.com"
        bind:value={url}
        disabled={busy}
      />
    </label>
    <div class="field">
      <span>Sidebar color</span>
      <div class="swatches">
        {#each VIEW_ACCENT_COLORS as swatch (swatch)}
          <button
            type="button"
            class="swatch"
            class:selected={color === swatch}
            style={`background: ${swatch}`}
            aria-label={swatch}
            disabled={busy}
            onclick={() => (color = swatch)}
          ></button>
        {/each}
        <input class="color-input" type="color" bind:value={color} disabled={busy} aria-label="Custom color" />
      </div>
    </div>
    <div class="field">
      <span>Icon</span>
      <input
        class="input"
        type="search"
        placeholder="Search {VIEW_ICONS.length} icons"
        bind:value={iconQuery}
        disabled={busy}
      />
      <div class="icon-grid" role="listbox" aria-label="View icons">
        {#each filteredIcons as item (item.id)}
          <button
            type="button"
            class="icon-choice"
            class:selected={icon === item.id}
            style={`color: ${color}`}
            role="option"
            title={item.label}
            aria-label={item.label}
            aria-selected={icon === item.id}
            disabled={busy}
            onclick={() => (icon = item.id)}
          >
            <ViewIcon name={item.id} />
          </button>
        {/each}
      </div>
      {#if filteredIcons.length === 0}
        <p class="empty">No icons match that search.</p>
      {/if}
    </div>
    {#if formError}
      <p class="error">{formError}</p>
    {/if}
    <div class="actions">
      <button type="button" class="btn" disabled={busy} onclick={resetForm}>Cancel</button>
      <button type="button" class="btn primary" disabled={busy} onclick={() => void saveView()}>
        {editingId ? "Save app" : "Add app"}
      </button>
    </div>
  </div>
  {/if}
</div>

<style>
  .custom-views {
    display: grid;
    gap: 14px;
  }

  .card {
    padding: 18px;
    border: 1px solid var(--md-line);
    border-radius: 10px;
    background: var(--md-surface);
  }

  .add-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .add-copy .desc {
    margin-top: 4px;
  }

  .add-row .btn {
    flex: 0 0 auto;
  }

  .title {
    margin: 0;
    font-size: 0.98rem;
    font-weight: 600;
    color: var(--md-ink);
  }

  .desc,
  .empty,
  .error {
    margin: 8px 0 0;
    color: var(--md-muted);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .error {
    color: var(--md-danger, #b42318);
  }

  .app-list {
    display: grid;
    gap: 10px;
    margin: 14px 0 0;
    padding: 0;
    list-style: none;
  }

  .app-list li {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .view-mark {
    width: 36px;
    height: 36px;
    flex: 0 0 auto;
  }

  .view-copy {
    display: grid;
    gap: 2px;
    min-width: 0;
    flex: 1 1 auto;
  }

  .view-copy strong {
    color: var(--md-ink);
    font-weight: 600;
  }

  .view-copy span {
    overflow: hidden;
    color: var(--md-muted);
    font-size: 0.82rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .view-actions {
    display: flex;
    flex: 0 0 auto;
    gap: 8px;
  }

  .field {
    display: grid;
    gap: 8px;
    margin-top: 14px;
  }

  .field > span {
    color: var(--md-ink);
    font-size: 0.9rem;
    font-weight: 600;
  }

  .input {
    width: 100%;
    min-height: 44px;
    padding: 12px 14px;
    border: 1px solid var(--md-input-border);
    border-radius: 8px;
    background: var(--md-input-bg);
    color: var(--md-ink);
    font-size: 0.95rem;
  }

  .input:focus {
    outline: none;
    border-color: var(--md-blue);
    box-shadow: 0 0 0 3px var(--md-blue-soft);
  }

  .swatches {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .swatch {
    width: 28px;
    height: 28px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 999px;
    cursor: pointer;
  }

  .swatch.selected {
    border-color: var(--md-ink);
  }

  .color-input {
    width: 42px;
    height: 32px;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
  }

  .icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(52px, 1fr));
    gap: 6px;
    max-height: 280px;
    overflow: auto;
    padding: 2px;
  }

  .icon-choice {
    display: grid;
    width: 100%;
    aspect-ratio: 1;
    place-items: center;
    padding: 8px;
    border: 1px solid var(--md-line);
    border-radius: 10px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .icon-choice.selected {
    border-color: currentColor;
    background: color-mix(in srgb, currentColor 16%, transparent);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 16px;
  }

  .btn {
    min-height: 40px;
    padding: 10px 14px;
    border: none;
    border-radius: 8px;
    background: var(--md-btn);
    color: var(--md-ink);
    font-size: 0.92rem;
    cursor: pointer;
  }

  .btn.primary {
    background: var(--md-btn-primary);
    color: var(--md-btn-primary-ink);
  }

  .btn.danger {
    background: transparent;
    color: var(--md-danger, #b42318);
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
