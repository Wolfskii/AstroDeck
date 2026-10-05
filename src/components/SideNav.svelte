<script lang="ts">
  import AppGlyph from "./AppGlyph.svelte";
  import ViewIcon from "./ViewIcon.svelte";
  import { BUILTIN_APPS } from "../lib/apps";

  let {
    active,
    customViews = [],
    disabledApps = [],
    onSelect,
  }: {
    active: string;
    customViews?: { id: string; name: string; icon: string; color: string }[];
    disabledApps?: string[];
    onSelect: (id: string) => void;
  } = $props();

  const apps = $derived(BUILTIN_APPS.filter((item) => !disabledApps.includes(item.id)));
  const websites = $derived(customViews.filter((item) => !disabledApps.includes(item.id)));
</script>

<nav class="side-nav" aria-label="Apps">
  <div class="side-nav-views">
    {#each apps as item (item.id)}
      <button
        type="button"
        class="side-nav-btn"
        class:active={active === item.id}
        style={`--nav-accent: ${item.color}`}
        title={item.name}
        aria-label={item.name}
        aria-current={active === item.id ? "page" : undefined}
        onclick={() => onSelect(item.id)}
      >
        <span class="side-nav-glyph">
          <AppGlyph id={item.id} />
        </span>
      </button>
    {/each}
    {#each websites as item (item.id)}
      <button
        type="button"
        class="side-nav-btn"
        class:active={active === item.id}
        style={`--nav-accent: ${item.color}`}
        title={item.name}
        aria-label={item.name}
        aria-current={active === item.id ? "page" : undefined}
        onclick={() => onSelect(item.id)}
      >
        <span class="side-nav-custom-icon">
          <ViewIcon name={item.icon} />
        </span>
      </button>
    {/each}
  </div>
  <button
    type="button"
    class="side-nav-btn"
    class:active={active === "settings"}
    style="--nav-accent: #e8e8e8"
    title="Settings"
    aria-label="Settings"
    aria-current={active === "settings" ? "page" : undefined}
    onclick={() => onSelect("settings")}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="currentColor"
        d="M19.4 13.1a7.7 7.7 0 0 0 .06-1.1 7.7 7.7 0 0 0-.06-1.1l2.02-1.58a.5.5 0 0 0 .12-.64l-1.91-3.3a.5.5 0 0 0-.6-.22l-2.38.96a7.4 7.4 0 0 0-1.9-1.1l-.36-2.53A.5.5 0 0 0 13.9 2h-3.8a.5.5 0 0 0-.49.42l-.36 2.53a7.4 7.4 0 0 0-1.9 1.1l-2.38-.96a.5.5 0 0 0-.6.22l-1.91 3.3a.5.5 0 0 0 .12.64L4.6 10.9c-.04.36-.06.73-.06 1.1s.02.74.06 1.1l-2.02 1.58a.5.5 0 0 0-.12.64l1.91 3.3a.5.5 0 0 0 .6.22l2.38-.96c.58.46 1.22.84 1.9 1.1l.36 2.53a.5.5 0 0 0 .49.42h3.8a.5.5 0 0 0 .49-.42l.36-2.53a7.4 7.4 0 0 0 1.9-1.1l2.38.96a.5.5 0 0 0 .6-.22l1.91-3.3a.5.5 0 0 0-.12-.64L19.4 13.1ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z"
      />
    </svg>
  </button>
</nav>

<style>
  .side-nav {
    position: absolute;
    z-index: 20;
    top: 0;
    bottom: 0;
    left: 0;
    display: flex;
    width: 120px;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 18px 12px;
    border: none;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    transition: background 0.4s ease, box-shadow 0.4s ease;
  }

  .side-nav:hover {
    background: #111113;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.28);
    transition: background 0.18s ease, box-shadow 0.18s ease;
  }

  .side-nav-views {
    display: flex;
    width: 100%;
    min-height: 0;
    flex: 1 1 auto;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .side-nav-views::-webkit-scrollbar {
    display: none;
  }

  .side-nav-btn {
    display: grid;
    width: 88px;
    height: 88px;
    flex: 0 0 auto;
    place-items: center;
    border: none;
    border-radius: 18px;
    background: transparent;
    color: rgba(255, 255, 255, 0.88);
    cursor: pointer;
  }

  .side-nav-btn > svg,
  .side-nav-glyph,
  .side-nav-custom-icon {
    width: 48px;
    height: 48px;
  }

  .side-nav-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .side-nav-btn.active,
  .side-nav-btn.active:hover {
    background: transparent;
    color: var(--nav-accent);
    box-shadow: none;
  }
</style>
