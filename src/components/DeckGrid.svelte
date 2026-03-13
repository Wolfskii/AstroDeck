<script lang="ts">
  import type { DeckButtonConfig } from "../types";
  import DeckButton from "./DeckButton.svelte";

  interface Props {
    grid: [number, number];
    buttons: DeckButtonConfig[];
  }

  let { grid, buttons, sceneId = "Buttons" }: Props & { sceneId?: string } = $props();

  let gridStyle = $derived(
    `grid-template-columns: repeat(${grid[1]}, 1fr); grid-template-rows: repeat(${grid[0]}, 1fr);`
  );
</script>

<div class="deck-grid" style={gridStyle}>
  {#each buttons as btn (btn.action)}
    <DeckButton
      label={btn.label}
      emoji={btn.emoji}
      image={btn.image}
      action={btn.action}
      source={`${sceneId} window`}
    />
  {/each}
</div>

<style>
  .deck-grid {
    display: grid;
    gap: 12px;
    padding: 16px;
    flex: 1;
    align-content: center;
    justify-content: center;
  }
</style>
