<script lang="ts">
  import Icon from "./Icon.svelte";
  import { runDeckAction } from "../lib/runDeckAction";
  import { iconForAction, type SceneIconName } from "../lib/sceneIcons";
  import { teamsEmojiForAction } from "../lib/teamsEmojis";

  let {
    label,
    action,
    source,
    variant = "tile",
    icon = null,
    tone = "default",
  }: {
    label: string;
    action: string;
    source: string;
    variant?: "reaction" | "call" | "tile";
    icon?: SceneIconName | null;
    tone?: "default" | "accent";
  } = $props();

  let pressed = $state(false);
  const emojiSrc = $derived(teamsEmojiForAction(action));
  const resolvedIcon = $derived(icon ?? iconForAction(action) ?? "letter");
  const initial = $derived((label.trim()[0] ?? "?").toUpperCase());

  async function handleClick() {
    pressed = true;
    await runDeckAction(action, label, source);
    setTimeout(() => (pressed = false), 180);
  }
</script>

<button
  type="button"
  class="action"
  class:reaction={variant === "reaction"}
  class:call={variant === "call"}
  class:tile={variant === "tile"}
  class:accent={tone === "accent"}
  class:pressed
  class:tint-run={action === "vscode.run"}
  class:tint-debug={action === "vscode.debug"}
  class:tint-git={action === "vscode.git"}
  class:tint-search={action === "vscode.search"}
  class:tint-terminal={action === "vscode.terminal"}
  class:tint-extensions={action === "vscode.extensions"}
  onclick={handleClick}
  aria-label={label}
>
  <span class="glyph" aria-hidden="true">
    {#if emojiSrc}
      <img class="emoji" src={emojiSrc} alt="" />
    {:else if resolvedIcon === "letter"}
      <span class="letter">{initial}</span>
    {:else}
      <Icon name={resolvedIcon} size={variant === "call" ? 32 : variant === "tile" ? 34 : 26} />
    {/if}
  </span>
  <span class="caption">{label}</span>
</button>

<style>
  .action {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: none;
    color: inherit;
    cursor: pointer;
    touch-action: manipulation;
    -webkit-tap-highlight-color: transparent;
    transition:
      transform 120ms ease,
      background-color 120ms ease,
      box-shadow 160ms ease;
  }

  .action:active,
  .action.pressed {
    transform: scale(0.94);
  }

  .glyph {
    display: grid;
    place-items: center;
  }

  .letter {
    font-size: 1.15rem;
    font-weight: 700;
  }

  .caption {
    font-weight: 600;
    text-align: center;
    line-height: 1.15;
  }

  .reaction {
    min-width: 76px;
    padding: 4px;
    background: transparent;
    color: #f3f2f1;
  }

  .reaction .glyph {
    width: 72px;
    height: 72px;
    border-radius: 20px;
    background: transparent;
    color: #f3f2f1;
  }

  .reaction:hover .glyph {
    background: rgba(255, 255, 255, 0.08);
  }

  .emoji {
    display: block;
    width: 56px;
    height: 56px;
    object-fit: contain;
    pointer-events: none;
  }

  .call .emoji {
    width: 48px;
    height: 48px;
  }

  .reaction .caption {
    font-size: 0.78rem;
    color: #d2d0ce;
  }

  .call {
    min-width: 92px;
    padding: 4px;
    background: transparent;
    color: #f3f2f1;
  }

  .call .glyph {
    width: 84px;
    height: 84px;
    border-radius: 50%;
    background: #2d2c35;
    color: #fff;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
  }

  .call:hover .glyph {
    background: #3a3944;
  }

  .call.accent .glyph {
    background: #5b5fc7;
  }

  .call.accent:hover .glyph {
    background: #6e74d8;
  }

  .call .caption {
    font-size: 0.82rem;
    color: #c8c6c4;
  }

  .tile {
    min-height: 120px;
    padding: 18px 12px;
    border-radius: 18px;
    background: #252526;
    color: #cccccc;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.05);
  }

  .tile:hover {
    background: #2d2d2d;
    color: #fff;
  }

  .tile .glyph {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    background: rgba(0, 122, 204, 0.16);
    color: #4fc1ff;
  }

  .tile .caption {
    font-size: 0.9rem;
    color: #e0e0e0;
  }

  .tile.tint-run .glyph {
    color: #89d185;
    background: rgba(137, 209, 133, 0.16);
  }

  .tile.tint-debug .glyph {
    color: #f14c4c;
    background: rgba(241, 76, 76, 0.16);
  }

  .tile.tint-git .glyph {
    color: #f05133;
    background: rgba(240, 81, 51, 0.16);
  }

  .tile.tint-search .glyph {
    color: #75beff;
    background: rgba(117, 190, 255, 0.16);
  }

  .tile.tint-terminal .glyph {
    color: #d7ba7d;
    background: rgba(215, 186, 125, 0.16);
  }

  .tile.tint-extensions .glyph {
    color: #c586c0;
    background: rgba(197, 134, 192, 0.16);
  }
</style>
