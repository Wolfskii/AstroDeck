<script lang="ts">
  import AppsIcon from "./AppsIcon.svelte";

  let {
    onclick,
    placement = "corner",
  }: {
    onclick?: () => void;
    placement?: "corner" | "inline";
  } = $props();
</script>

<button
  type="button"
  class="choose-view"
  class:choose-view-inline={placement === "inline"}
  title="Choose view"
  aria-label="Choose view"
  {onclick}
>
  <AppsIcon size={56} />
</button>

<style>
  .choose-view {
    position: absolute;
    top: 36px;
    left: 40px;
    z-index: 2;
    padding: 8px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: inherit;
    line-height: 0;
    cursor: pointer;
  }

  .choose-view-inline {
    position: static;
    top: auto;
    left: auto;
    flex-shrink: 0;
    padding: 8px;
    border-radius: 12px;
  }

  .choose-view:hover {
    background: rgba(255, 255, 255, 0.08);
  }
</style>
