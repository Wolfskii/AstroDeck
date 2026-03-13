<script lang="ts">
  import { executeAction } from "../services/api";
  import { logError, logInfo } from "../services/logger";

  interface Props {
    label: string;
    emoji?: string;
    image?: string;
    action: string;
    source?: string;
  }

  let { label, emoji, image, action, source = "Buttons window" }: Props = $props();
  let pressed = $state(false);

  async function handleClick() {
    pressed = true;
    try {
      if (action.startsWith("core.")) {
        window.dispatchEvent(
          new CustomEvent("taptapdeck-core-action", {
            detail: { action, label },
          })
        );
      }
      logInfo(`Clicked ${label} (${action})`, source);
      await executeAction(action);
      window.dispatchEvent(
        new CustomEvent("taptapdeck-action-executed", {
          detail: { action, label },
        })
      );
    } catch (e) {
      logError(`Action failed: ${action} (${String(e)})`, source);
    } finally {
      setTimeout(() => (pressed = false), 200);
    }
  }
</script>

<button
  class="deck-button"
  class:pressed
  onclick={handleClick}
  aria-label={label}
>
  {#if image}
    <img src={image} alt={label} class="button-icon" />
  {:else if emoji}
    <span class="button-emoji">{emoji}</span>
  {/if}
  <span class="button-label">{label}</span>
</button>

<style>
  .deck-button {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: var(--button-min-size);
    min-height: var(--button-min-size);
    padding: 16px 12px;
    background: var(--bg-button);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-button);
    color: var(--text-primary);
    cursor: pointer;
    transition:
      background var(--transition-fast),
      transform var(--transition-fast),
      box-shadow var(--transition-normal);
    touch-action: manipulation;
    -webkit-tap-highlight-color: transparent;
  }

  .deck-button:hover {
    background: var(--bg-button-hover);
    box-shadow: 0 0 12px var(--accent-glow);
  }

  .deck-button:active,
  .deck-button.pressed {
    background: var(--bg-button-active);
    transform: scale(0.93);
    box-shadow: 0 0 20px var(--accent-glow), 0 0 40px rgba(99, 102, 241, 0.15);
  }

  .button-emoji {
    font-size: 2rem;
    line-height: 1;
  }

  .button-icon {
    width: 40px;
    height: 40px;
    object-fit: contain;
  }

  .button-label {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
    text-align: center;
    line-height: 1.2;
  }
</style>
