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
    active = false,
    onAction,
  }: {
    label: string;
    action: string;
    source: string;
    variant?: "reaction" | "call" | "tile";
    icon?: SceneIconName | null;
    tone?: "default" | "accent" | "danger";
    active?: boolean;
    onAction?: () => void;
  } = $props();

  let pressed = $state(false);
  const emojiSrc = $derived(teamsEmojiForAction(action));
  const resolvedIcon = $derived(icon ?? iconForAction(action) ?? "letter");
  const initial = $derived((label.trim()[0] ?? "?").toUpperCase());

  async function handleClick() {
    pressed = true;
    await runDeckAction(action, label, source);
    onAction?.();
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
  class:danger={tone === "danger"}
  class:active={active}
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
    min-width: 148px;
    padding: 12px 10px;
    gap: 12px;
    background: transparent;
    color: #f3f2f1;
  }

  .reaction .glyph {
    width: 132px;
    height: 132px;
    border-radius: 26px;
    background: #2d2c35;
    color: #f3f2f1;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.08),
      0 8px 20px rgba(0, 0, 0, 0.22);
  }

  .reaction:hover .glyph {
    background: #3a3944;
  }

  .emoji {
    display: block;
    width: 56px;
    height: 56px;
    object-fit: contain;
    pointer-events: none;
  }

  .call .emoji {
    width: 72px;
    height: 72px;
  }

  .reaction .caption {
    font-size: 0.92rem;
    color: #d2d0ce;
  }

  .reaction .emoji {
    width: 102px;
    height: 102px;
  }

  .call {
    min-width: 128px;
    padding: 10px;
    background: transparent;
    color: #f3f2f1;
  }

  .call .glyph {
    width: 112px;
    height: 112px;
    border-radius: 24px;
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

  .call.danger .glyph {
    background: #8f2e36;
    color: #fff;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.12),
      0 8px 18px rgba(143, 46, 54, 0.28);
  }

  .call.danger:hover .glyph {
    background: #ad3943;
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
