<script lang="ts">
  let { url, title }: { url: string; title: string } = $props();
</script>

<div class="embed-view">
  <iframe
    class="embed-frame"
    {title}
    src={url}
    referrerpolicy="no-referrer"
    allow="autoplay; fullscreen; picture-in-picture; encrypted-media; clipboard-write"
    sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox allow-presentation allow-downloads allow-modals allow-top-navigation-by-user-activation"
  ></iframe>
</div>

<style>
  .embed-view {
    position: relative;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    background: #000;
  }

  .embed-frame {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: #fff;
  }
</style>
