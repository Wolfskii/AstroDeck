<script lang="ts">
  type NavId =
    | "clock"
    | "spotify"
    | "youtubeMusic"
    | "media"
    | "teams"
    | "vscode"
    | "weather"
    | "settings";

  let {
    active,
    onSelect,
  }: {
    active: string;
    onSelect: (id: NavId) => void;
  } = $props();

  const views: { id: NavId; label: string; color: string }[] = [
    { id: "clock", label: "Home", color: "#f3d37a" },
    { id: "spotify", label: "Spotify", color: "#1db954" },
    { id: "youtubeMusic", label: "YouTube", color: "#ff0033" },
    { id: "media", label: "System media", color: "#f59e0b" },
    { id: "teams", label: "Teams", color: "#7b83eb" },
    { id: "vscode", label: "VS Code", color: "#3794ff" },
    { id: "weather", label: "Weather", color: "#38bdf8" },
  ];
</script>

<nav class="side-nav" aria-label="Views">
  <div class="side-nav-views">
    {#each views as item (item.id)}
      <button
        type="button"
        class="side-nav-btn"
        class:active={active === item.id}
        style={`--nav-accent: ${item.color}`}
        title={item.label}
        aria-label={item.label}
        aria-current={active === item.id ? "page" : undefined}
        onclick={() => onSelect(item.id)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {#if item.id === "clock"}
            <path
              fill="currentColor"
              d="M4.2 10.6 12 4.2l7.8 6.4V20a1.2 1.2 0 0 1-1.2 1.2h-4.4v-5.6H9.8v5.6H5.4A1.2 1.2 0 0 1 4.2 20v-9.4Z"
            />
          {:else if item.id === "spotify"}
            <path
              fill="currentColor"
              d="M12 2a10 10 0 1 0 .01 20.01A10 10 0 0 0 12 2Zm4.55 14.46a.63.63 0 0 1-.86.21c-2.36-1.44-5.34-1.77-8.84-.97a.63.63 0 0 1-.28-1.23c3.82-.87 7.1-.5 9.77 1.13a.63.63 0 0 1 .21.86Zm1.2-2.68a.78.78 0 0 1-1.07.26c-2.7-1.66-6.82-2.14-10.02-1.17a.78.78 0 1 1-.45-1.5c3.67-1.11 8.23-.57 11.28 1.34a.78.78 0 0 1 .26 1.07Zm.1-2.79a.93.93 0 0 1-1.28.31c-3.09-1.9-8.2-2.07-11.15-1.14a.93.93 0 1 1-.54-1.78c3.4-1.04 9.06-.84 12.66 1.32a.93.93 0 0 1 .31 1.29Z"
            />
          {:else if item.id === "youtubeMusic"}
            <path
              fill="currentColor"
              d="M23 12.2s0-3.15-.4-4.55c-.22-.9-.9-1.58-1.8-1.8C19.18 5.46 12 5.46 12 5.46s-7.18 0-8.8.39c-.9.22-1.58.9-1.8 1.8C1 9.05 1 12.2 1 12.2s0 3.15.4 4.55c.22.9.9 1.58 1.8 1.8 1.62.39 8.8.39 8.8.39s7.18 0 8.8-.39c.9-.22 1.58-.9 1.8-1.8.4-1.4.4-4.55.4-4.55ZM9.75 15.57V8.83l6.27 3.37-6.27 3.37Z"
            />
          {:else if item.id === "media"}
            <path
              fill="currentColor"
              d="M12 2.2a9.8 9.8 0 1 0 .01 19.61A9.8 9.8 0 0 0 12 2.2Zm-1.7 13.7V8.1L17.2 12l-6.9 3.9Z"
            />
          {:else if item.id === "teams"}
            <path
              fill="currentColor"
              d="M14.2 7.2a2.4 2.4 0 1 0 0-4.8 2.4 2.4 0 0 0 0 4.8ZM8.6 8.4a2.7 2.7 0 1 0 0-5.4 2.7 2.7 0 0 0 0 5.4ZM16.7 8.6h-2.2a3.7 3.7 0 0 0-1.7.4 4.6 4.6 0 0 1 1.3 3.2v4.6h3.4A2.5 2.5 0 0 0 20 14.3V11.1a2.5 2.5 0 0 0-2.5-2.5h-.8ZM8.6 9.2C6 9.2 3.2 10.4 3.2 12.8v3.4A2.6 2.6 0 0 0 5.8 18.8h6.3v-6.6c0-1.7-1.4-3-3.5-3Z"
            />
          {:else if item.id === "vscode"}
            <path
              fill="currentColor"
              d="M17.4 2.3 8.7 8.1 5.2 5.4 2.6 6.7v10.6l2.6 1.3 3.5-2.7 8.7 5.8c.7.4 1.6 0 1.6-.9V3.2c0-.9-.9-1.3-1.6-.9ZM8.7 12 5.2 9.3v5.4L8.7 12Zm7.2 4.7-4.6-3.1v-3.2l4.6-3.1v9.4Z"
            />
          {:else}
            <path
              fill="currentColor"
              d="M7.2 16.8h8.4a3.3 3.3 0 0 0 .5-6.55 4.7 4.7 0 0 0-9.1-1.1 3.5 3.5 0 0 0 .2 7.65ZM16.4 6.2a1 1 0 0 1-.3-1.37 4.2 4.2 0 0 1 1.7-1.5 1 1 0 0 1 .8 1.82 2.2 2.2 0 0 0-.9.8 1 1 0 0 1-1.3.25ZM18.9 8.4a1 1 0 0 1 1.2-1.6 4.4 4.4 0 0 1 1.6 1.6 1 1 0 1 1-1.7 1 2.4 2.4 0 0 0-1.1-.9Z"
            />
          {/if}
        </svg>
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
    z-index: 8;
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

  .side-nav-btn svg {
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
