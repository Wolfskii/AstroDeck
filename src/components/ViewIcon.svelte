<script lang="ts">
  import { viewIconSvg } from "../lib/viewIcons";

  let { name }: { name: string } = $props();
  const svg = $derived(viewIconSvg(name));
</script>

<span class="view-icon" aria-hidden="true">{@html svg}</span>

<style>
  .view-icon {
    display: grid;
    width: 100%;
    height: 100%;
    place-items: center;
    color: inherit;
  }

  .view-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
