<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { ShaderMount, emptyPixel } from "@paper-design/shaders";
  import { mixHexPalette } from "../lib/color";
  import { usesCoverImage, usesThreeBackground, type SceneBackgroundId } from "../lib/sceneBackgrounds";
  import { createThreeBackground, type ThreeBackgroundHandle } from "../lib/threeBackground";
  import { acquireOsAudioFrames, releaseOsAudioFrames } from "../lib/osAudioViz";
  import { audioVisualizerEnabled } from "../stores/appearance";
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
  const PLAYING_OFF_DELAY_MS = 350;

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
    getFrameMs: () => number;
  };

  let live: LiveShader | null = null;
  let pendingPalette = [...colors];
  let animFrameMs = 0;
  let animStyle: SceneBackgroundId | null = null;
  let renderPlaying = $state(playing);

  $effect(() => {
    pendingPalette = [...colors];
    live?.applyPalette(pendingPalette);
  });

  $effect(() => {
    if (playing) {
      renderPlaying = true;
      return;
    }
    const timer = window.setTimeout(() => {
      renderPlaying = false;
    }, PLAYING_OFF_DELAY_MS);
    return () => window.clearTimeout(timer);
  });

  $effect(() => {
    const isPlaying = renderPlaying;
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

  onMount(() => {
    let acquired = false;
    const stop = audioVisualizerEnabled.subscribe((enabled) => {
      if (enabled && !acquired) {
        acquireOsAudioFrames();
        acquired = true;
      } else if (!enabled && acquired) {
        releaseOsAudioFrames();
        acquired = false;
      }
    });
    return () => {
      stop();
      if (acquired) releaseOsAudioFrames();
    };
  });

  function prepareAnimClock(nextStyle: SceneBackgroundId) {
    if (animStyle !== nextStyle) {
      animStyle = nextStyle;
      animFrameMs = 0;
    }
  }

  $effect(() => {
    const el = host;
    const currentStyle = style;
    if (!el) return;

    if (usesThreeBackground(currentStyle)) {
      let three: ThreeBackgroundHandle | null = null;
      try {
        prepareAnimClock(currentStyle);
        three = createThreeBackground(
          el,
          currentStyle,
          untrack(() => pendingPalette),
          animFrameMs / 1000
        );
        three.setPlaying(untrack(() => renderPlaying));
        live = {
          style: currentStyle,
          applyPalette: (palette) => three?.applyPalette(palette),
          applyImage: () => {},
          setPlaying: (next) => three?.setPlaying(next),
          getFrameMs: () => (three ? three.getElapsed() * 1000 : animFrameMs),
        };
      } catch {
        el.replaceChildren();
        live = null;
      }
      return () => {
        if (three) animFrameMs = three.getElapsed() * 1000;
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
      prepareAnimClock(currentStyle);
      let shaderPlaying = untrack(() => renderPlaying);
      try {
        // Always mount paused: we own u_time so song position / SMTC jitter
        // cannot rewind or pulse the shader.
        mount = new ShaderMount(
          el,
          fragment,
          uniforms,
          { alpha, premultipliedAlpha: !alpha, antialias: false },
          0,
          animFrameMs,
          1
        );
      } catch {
        return;
      }

      function applyFluted(tSec: number) {
        if (currentStyle !== "fluted-glass" || !mount) return;
        mount.setUniforms({
          u_shift: Math.sin(tSec * 0.45) * 0.22,
          u_angle: 10 + Math.sin(tSec * 0.18) * 14,
        });
      }

      applyFluted(animFrameMs / 1000);

      let clockLast = performance.now();
      const tickClock = (now: number) => {
        if (disposed || !mount) return;
        const dt = Math.min(50, now - clockLast);
        clockLast = now;
        if (shaderPlaying) {
          animFrameMs += dt * speed;
          mount.setFrame(animFrameMs);
          applyFluted(animFrameMs / 1000);
        }
        raf = requestAnimationFrame(tickClock);
      };
      raf = requestAnimationFrame(tickClock);

      live = {
        style: currentStyle,
        applyPalette,
        applyImage,
        setPlaying(next) {
          shaderPlaying = next;
        },
        getFrameMs: () => (mount ? mount.getCurrentFrame() : animFrameMs),
      };
      const latest = untrack(() => pendingPalette);
      if (!palettesEqual(latest, displayed)) {
        applyPalette(latest);
      }
    })();

    return () => {
      disposed = true;
      if (mount) animFrameMs = mount.getCurrentFrame();
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
