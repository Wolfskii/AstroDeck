<script lang="ts">
  import DeckGrid from "../components/DeckGrid.svelte";
  import { getBuiltinLayout } from "../layouts/layouts";
  import type { LayoutConfig } from "../types";

  const layout: LayoutConfig = getBuiltinLayout("default")!;
</script>

<div class="mode-container">
  <header class="mode-header">
    <h2>TapTapDeck</h2>
    <span class="mode-badge">Default</span>
  </header>
  <DeckGrid grid={layout.grid} buttons={layout.buttons} />
</div>

<style>
  .mode-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .mode-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .mode-header h2 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .mode-badge {
    font-size: 0.7rem;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--bg-button);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
