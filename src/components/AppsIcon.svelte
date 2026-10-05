<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<svg
  class="apps-icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="currentColor"
  aria-hidden="true"
>
  <circle cx="5" cy="5" r="1.75" />
  <circle cx="12" cy="5" r="1.75" />
  <circle cx="19" cy="5" r="1.75" />
  <circle cx="5" cy="12" r="1.75" />
  <circle cx="12" cy="12" r="1.75" />
  <circle cx="19" cy="12" r="1.75" />
  <circle cx="5" cy="19" r="1.75" />
  <circle cx="12" cy="19" r="1.75" />
  <circle cx="19" cy="19" r="1.75" />
</svg>

<style>
  .apps-icon {
    display: block;
    flex-shrink: 0;
  }
</style>
