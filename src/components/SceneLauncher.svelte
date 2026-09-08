<script lang="ts">
  import { getBuiltinSceneMeta } from "../layouts/layouts";
  import type { PluginConfig } from "../types";

  let {
    sceneIds,
    pluginsById,
    sceneId,
    seenScenes,
    onSelectScene,
    onOpenSettings,
  }: {
    sceneIds: string[];
    pluginsById: Map<string, PluginConfig>;
    sceneId: string;
    seenScenes: string[];
    onSelectScene: (id: string) => void;
    onOpenSettings: () => void;
  } = $props();
</script>

<section class="launcher">
  <button class="launcher-settings" type="button" aria-label="Open settings" onclick={onOpenSettings}>
    <span aria-hidden="true">⚙</span>
  </button>
  <header class="launcher-header">
    <p>AstroDeck</p>
    <h1>Choose a view</h1>
  </header>
  <div class="launcher-grid">
    {#each sceneIds as id (id)}
      {@const plugin = pluginsById.get(id)}
      {@const meta = getBuiltinSceneMeta(id)}
      {@const title = plugin?.name ?? meta?.name ?? id}
      {@const description =
        plugin?.description ?? meta?.description ?? "Open this AstroDeck view."}
      <button
        type="button"
        class="launcher-card"
        class:launcher-card--active={id === sceneId}
        onclick={() => onSelectScene(id)}
      >
        <span class="launcher-card-icon" aria-hidden="true">
          {id === "spotify" ? "♫" : id === "youtubeMusic" ? "▶" : id === "media" ? "◉" : "✦"}
        </span>
        <span class="launcher-card-title">{title}</span>
        <span class="launcher-card-description">{description}</span>
        {#if id === sceneId}
          <span class="launcher-card-current">Current</span>
        {:else if seenScenes.includes(id)}
          <span class="launcher-card-current">Open</span>
        {/if}
      </button>
    {/each}
  </div>
</section>

<style>
  .launcher {
    position: relative;
    min-height: 100%;
    overflow: auto;
    padding: 42px clamp(24px, 6vw, 96px) 64px;
    background:
      radial-gradient(circle at 50% 0%, rgba(29, 185, 84, 0.13), transparent 42%),
      #101010;
    color: #fff;
  }

  .launcher-settings {
    position: absolute;
    top: 28px;
    left: 28px;
    display: grid;
    place-items: center;
    width: 58px;
    height: 58px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
    font-size: 1.7rem;
    cursor: pointer;
  }

  .launcher-settings:hover {
    background: rgba(255, 255, 255, 0.16);
  }

  .launcher-header {
    max-width: 1200px;
    margin: 0 auto 28px;
    text-align: center;
  }

  .launcher-header p {
    margin: 0 0 8px;
    color: #1db954;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .launcher-header h1 {
    margin: 0;
    font-size: clamp(2rem, 5vw, 4rem);
  }

  .launcher-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 20px;
    max-width: 1200px;
    margin: 0 auto;
  }

  .launcher-card {
    position: relative;
    display: flex;
    min-height: 220px;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-end;
    gap: 8px;
    padding: 22px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 20px;
    background: linear-gradient(145deg, #252525, #151515);
    color: #fff;
    text-align: left;
    cursor: pointer;
    transition: transform 0.15s ease, border-color 0.15s ease, background 0.15s ease;
  }

  .launcher-card:hover,
  .launcher-card--active {
    border-color: rgba(29, 185, 84, 0.8);
    background: linear-gradient(145deg, #263b2d, #151515);
    transform: translateY(-3px);
  }

  .launcher-card-icon {
    position: absolute;
    top: 22px;
    right: 22px;
    display: grid;
    place-items: center;
    width: 74px;
    height: 74px;
    border-radius: 20px;
    background: rgba(29, 185, 84, 0.16);
    color: #1db954;
    font-size: 2.5rem;
  }

  .launcher-card-title {
    font-size: 1.45rem;
    font-weight: 800;
  }

  .launcher-card-description {
    max-width: 90%;
    color: #b7c0c8;
    line-height: 1.35;
  }

  .launcher-card-current {
    margin-top: 8px;
    color: #1db954;
    font-size: 0.82rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  @media (max-width: 600px) {
    .launcher {
      padding: 110px 16px 32px;
    }

    .launcher-settings {
      top: 22px;
      left: 20px;
    }

    .launcher-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 12px;
    }

    .launcher-card {
      min-height: 180px;
      padding: 15px;
    }

    .launcher-card-icon {
      top: 15px;
      right: 15px;
      width: 52px;
      height: 52px;
      border-radius: 15px;
      font-size: 1.7rem;
    }

    .launcher-card-title {
      font-size: 1.05rem;
    }

    .launcher-card-description {
      font-size: 0.8rem;
    }
  }
</style>
