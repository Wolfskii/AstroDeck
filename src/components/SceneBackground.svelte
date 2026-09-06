<script lang="ts">
  import { untrack } from "svelte";
  import { ShaderMount, emptyPixel } from "@paper-design/shaders";
  import { mixHexPalette } from "../lib/color";
  import { usesCoverImage, usesThreeBackground, usesVisualizerShader, type SceneBackgroundId } from "../lib/sceneBackgrounds";
  import { createThreeBackground, type ThreeBackgroundHandle } from "../lib/threeBackground";
  import { synthBeat } from "../lib/visualizerBeat";
  import {
    fragmentForStyle,
    lerpPaletteUniforms,
    paletteUniformsForStyle,
    uniformsForStyle,
  } from "../lib/shaderBackground";

  let {
    style,
    colors,
    imageUrl = null,
    placement = "full",
    playing = true,
  }: {
    style: SceneBackgroundId;
    colors: string[];
    imageUrl?: string | null;
    placement?: "full" | "artwork";
    playing?: boolean;
  } = $props();

  const PALETTE_FADE_MS = 700;

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

  function waitForTexture(image: HTMLImageElement): Promise<void> {
    if (image.complete && image.naturalWidth > 0) return Promise.resolve();
    return new Promise((resolve, reject) => {
      image.addEventListener("load", () => resolve(), { once: true });
      image.addEventListener("error", () => reject(new Error("Failed to load shader texture")), {
        once: true,
      });
    });
  }

  function easeInOut(t: number): number {
    return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
  }

  function palettesEqual(a: string[], b: string[]): boolean {
    if (a.length !== b.length) return false;
    return a.every((color, index) => color === b[index]);
  }

  type LiveShader = {
    style: SceneBackgroundId;
    applyPalette: (palette: string[]) => void;
    applyImage: (image: HTMLImageElement) => void;
    setPlaying: (next: boolean) => void;
  };

  let live: LiveShader | null = null;
  let pendingPalette = [...colors];

  $effect(() => {
    pendingPalette = [...colors];
    live?.applyPalette(pendingPalette);
  });

  $effect(() => {
    const isPlaying = playing;
    untrack(() => live)?.setPlaying(isPlaying);
  });

  $effect(() => {
    const currentStyle = style;
    const url = imageUrl;
    if (!usesCoverImage(currentStyle)) return;
    let cancelled = false;
    void loadImage(url || emptyPixel)
      .catch(() => loadImage(emptyPixel))
      .then((image) => {
        if (cancelled) return;
        live?.applyImage(image);
      })
      .catch(() => {
        // keep the current texture
      });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const el = host;
    const currentStyle = style;
    if (!el) return;

    if (usesThreeBackground(currentStyle)) {
      let three: ThreeBackgroundHandle | null = null;
      try {
        three = createThreeBackground(el, currentStyle, untrack(() => pendingPalette));
        three.setPlaying(untrack(() => playing));
        live = {
          style: currentStyle,
          applyPalette: (palette) => three?.applyPalette(palette),
          applyImage: () => {},
          setPlaying: (next) => three?.setPlaying(next),
        };
      } catch {
        el.replaceChildren();
        live = null;
      }
      return () => {
        if (live?.style === currentStyle) live = null;
        three?.dispose();
      };
    }

    const fragment = fragmentForStyle(currentStyle);
    if (!fragment || currentStyle === "off") {
      el.replaceChildren();
      live = null;
      return;
    }

    let disposed = false;
    let mount: ShaderMount | null = null;
    let raf = 0;
    let lerpRaf = 0;
    let image: HTMLImageElement | undefined;
    let displayed = untrack(() => [...pendingPalette]);
    let lerpFrom = displayed;
    let lerpTo = displayed;
    let lerpStarted = 0;
    let isPlaying = untrack(() => playing);

    function tickLerp(now: number) {
      if (disposed || !mount) return;
      const t = Math.min(1, (now - lerpStarted) / PALETTE_FADE_MS);
      const eased = easeInOut(t);
      displayed = mixHexPalette(lerpFrom, lerpTo, eased);
      mount.setUniforms(
        lerpPaletteUniforms(
          paletteUniformsForStyle(currentStyle, lerpFrom, image),
          paletteUniformsForStyle(currentStyle, lerpTo, image),
          eased
        )
      );
      if (t < 1) lerpRaf = requestAnimationFrame(tickLerp);
    }

    function applyPalette(next: string[]) {
      if (!mount) return;
      if (palettesEqual(next, lerpTo) && palettesEqual(displayed, next)) return;
      lerpFrom = displayed;
      lerpTo = [...next];
      lerpStarted = performance.now();
      cancelAnimationFrame(lerpRaf);
      lerpRaf = requestAnimationFrame(tickLerp);
    }

    function applyImage(nextImage: HTMLImageElement) {
      image = nextImage;
      if (!mount) return;
      mount.setUniforms({ u_image: nextImage });
    }

    void (async () => {
      if (usesCoverImage(currentStyle)) {
        const url = untrack(() => imageUrl);
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
      displayed = untrack(() => [...pendingPalette]);
      lerpFrom = displayed;
      lerpTo = displayed;
      const { uniforms, speed } = uniformsForStyle(currentStyle, displayed, image);
      const textures = Object.values(uniforms).filter(
        (value): value is HTMLImageElement => value instanceof HTMLImageElement
      );
      try {
        await Promise.all(textures.map((texture) => waitForTexture(texture)));
      } catch {
        return;
      }
      if (disposed || !el.isConnected) return;
      const alpha = currentStyle === "pulsing-border";
      try {
        mount = new ShaderMount(
          el,
          fragment,
          uniforms,
          { alpha, premultipliedAlpha: !alpha, antialias: false },
          speed,
          0,
          1
        );
      } catch {
        return;
      }

      live = {
        style: currentStyle,
        applyPalette,
        applyImage,
        setPlaying(next: boolean) {
          isPlaying = next;
        },
      };
      isPlaying = untrack(() => playing);
      const latest = untrack(() => pendingPalette);
      if (!palettesEqual(latest, displayed)) {
        applyPalette(latest);
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

      if (usesVisualizerShader(currentStyle)) {
        const started = performance.now();
        const tick = (now: number) => {
          if (disposed || !mount) return;
          const t = (now - started) / 1000;
          mount.setUniforms({
            u_beat: synthBeat(t, isPlaying),
            u_speed: isPlaying ? 0.85 : 0.18,
          });
          raf = requestAnimationFrame(tick);
        };
        raf = requestAnimationFrame(tick);
      }
    })();

    return () => {
      disposed = true;
      if (live?.style === currentStyle) live = null;
      cancelAnimationFrame(raf);
      cancelAnimationFrame(lerpRaf);
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
    inset: 0;
    z-index: 0;
    overflow: visible;
  }

  .scene-background--artwork :global(canvas) {
    z-index: 0;
  }

  .scene-background :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
