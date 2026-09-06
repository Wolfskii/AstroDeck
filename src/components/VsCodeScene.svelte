<script lang="ts">
  import appIconUrl from "../assets/app-icon.png";
  import Icon from "./Icon.svelte";
  import SceneActionButton from "./SceneActionButton.svelte";
  import type { DeckButtonConfig } from "../types";

  let {
    buttons,
    showSettingsButton = false,
    onOpenSettings,
  }: {
    buttons: DeckButtonConfig[];
    showSettingsButton?: boolean;
    onOpenSettings?: () => void;
  } = $props();
</script>

<section class="vscode" class:has-settings={showSettingsButton && onOpenSettings}>
  {#if showSettingsButton && onOpenSettings}
    <button
      type="button"
      class="settings-btn"
      title="Settings"
      aria-label="Open settings"
      onclick={() => onOpenSettings()}
    >
      <img class="settings-icon" src={appIconUrl} alt="" aria-hidden="true" />
    </button>
  {/if}

  <header class="brand">
    <span class="logo" aria-hidden="true">
      <Icon name="vscodeMark" size={26} />
    </span>
    <div>
      <p class="eyebrow">VS Code / Cursor</p>
      <h1>Workspace</h1>
    </div>
  </header>

  <div class="grid">
    {#each buttons as button (button.action)}
      <SceneActionButton
        label={button.label}
        action={button.action}
        source="VS Code window"
        variant="tile"
      />
    {/each}
  </div>
</section>

<style>
  .vscode {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    padding: 20px 24px 24px;
    background:
      linear-gradient(90deg, #007acc 0 4px, transparent 4px),
      #1e1e1e;
    color: #cccccc;
  }

  .vscode.has-settings {
    padding-top: 92px;
  }

  .settings-btn {
    position: absolute;
    top: 8px;
    left: 12px;
    z-index: 2;
    padding: 4px;
    border: none;
    border-radius: 12px;
    background: transparent;
    line-height: 0;
    cursor: pointer;
  }

  .settings-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .settings-icon {
    display: block;
    width: 72px;
    height: 72px;
    object-fit: contain;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 0 0 18px 8px;
  }

  .logo {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: #007acc;
    color: #fff;
  }

  .eyebrow {
    margin: 0;
    color: #9cdcfe;
    font-size: 0.78rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  h1 {
    margin: 2px 0 0;
    color: #f3f3f3;
    font-size: 1.45rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .grid {
    flex: 1 1 auto;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-auto-rows: minmax(120px, 1fr);
    gap: 14px;
  }

  @media (max-width: 640px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
