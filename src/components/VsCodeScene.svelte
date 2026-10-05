<script lang="ts">
  import ChooseViewButton from "./ChooseViewButton.svelte";
  import Icon from "./Icon.svelte";
  import SceneActionButton from "./SceneActionButton.svelte";
  import { sceneBackgroundId } from "../stores/appearance";
  import { usesFullViewBackground } from "../lib/sceneBackgrounds";
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

  const showShader = $derived(usesFullViewBackground($sceneBackgroundId));
</script>

<section class="vscode" class:has-settings={showSettingsButton && onOpenSettings} class:shader={showShader}>
  {#if showSettingsButton && onOpenSettings}
    <ChooseViewButton onclick={() => onOpenSettings()} />
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

  .vscode.shader {
    background: transparent;
  }

  .vscode.has-settings {
    padding-top: 120px;
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
