<script lang="ts">
  import { ShaderMount, emptyPixel } from "@paper-design/shaders";
  import { usesCoverImage, type SceneBackgroundId } from "../lib/sceneBackgrounds";
  import { fragmentForStyle, uniformsForStyle } from "../lib/shaderBackground";

  let {
    style,
    colors,
    imageUrl = null,
    placement = "full",
  }: {
    style: SceneBackgroundId;
    colors: string[];
    imageUrl?: string | null;
    placement?: "full" | "artwork";
  } = $props();

  let host = $state<HTMLDivElement | null>(null);

  function loadImage(url: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.crossOrigin = "anonymous";
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error("Failed to load shader image"));
      img.src = url;
    });
  }

  $effect(() => {
    const el = host;
    const currentStyle = style;
    const palette = colors;
    const url = imageUrl;
    if (!el) return;

    const fragment = fragmentForStyle(currentStyle);
    if (!fragment || currentStyle === "off") {
      el.replaceChildren();
      return;
    }

    let disposed = false;
    let mount: ShaderMount | null = null;
    let raf = 0;

    void (async () => {
      let image: HTMLImageElement | undefined;
      if (usesCoverImage(currentStyle)) {
        try {
          image = await loadImage(url || emptyPixel);
        } catch {
          try {
            image = await loadImage(emptyPixel);
          } catch {
            image = undefined;
          }
        }
      }
      if (disposed || !el.isConnected) return;
      el.replaceChildren();
      const { uniforms, speed } = uniformsForStyle(currentStyle, palette, image);
      const alpha = currentStyle === "pulsing-border";
      try {
        mount = new ShaderMount(
          el,
          fragment,
          uniforms,
          { alpha, premultipliedAlpha: true, antialias: false },
          speed,
          0,
          1
        );
      } catch {
        return;
      }

      if (currentStyle === "fluted-glass") {
        const started = performance.now();
        const tick = (now: number) => {
          if (disposed || !mount) return;
          const t = (now - started) / 1000;
          mount.setUniforms({
            u_shift: Math.sin(t * 0.45) * 0.22,
            u_angle: 10 + Math.sin(t * 0.18) * 14,
          });
          raf = requestAnimationFrame(tick);
        };
        raf = requestAnimationFrame(tick);
      }
    })();

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      mount?.dispose();
      mount = null;
    };
  });
</script>

<div
  class="scene-background"
  class:scene-background--artwork={placement === "artwork"}
  bind:this={host}
  aria-hidden="true"
></div>

<style>
  .scene-background {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .scene-background--artwork {
    inset: -22px;
  }

  .scene-background :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
