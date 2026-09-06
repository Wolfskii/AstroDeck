<script lang="ts">
  import { hexToHsv, hsvToHex, type Hsv } from "../lib/color";

  let {
    value,
    disabled = false,
    onChange,
    onCommit,
  }: {
    value: string;
    disabled?: boolean;
    onChange: (hex: string) => void;
    onCommit?: (hex: string) => void;
  } = $props();

  let hsv = $state<Hsv>(hexToHsv(value));
  let open = $state(false);

  $effect(() => {
    const incoming = value;
    if (incoming === hsvToHex(hsv)) return;
    hsv = hexToHsv(incoming);
  });

  $effect(() => {
    if (disabled) open = false;
  });

  const hex = $derived(hsvToHex(hsv));
  const hueColor = $derived(hsvToHex({ h: hsv.h, s: 1, v: 1 }));

  let planeEl = $state<HTMLDivElement | null>(null);
  let hueEl = $state<HTMLDivElement | null>(null);

  function emit(next: Hsv, commit = false) {
    hsv = next;
    const nextHex = hsvToHex(next);
    onChange(nextHex);
    if (commit) onCommit?.(nextHex);
  }

  function hsvFromPlane(event: PointerEvent): Hsv | null {
    const el = planeEl;
    if (!el) return null;
    const rect = el.getBoundingClientRect();
    const s = Math.min(1, Math.max(0, (event.clientX - rect.left) / Math.max(rect.width, 1)));
    const v = 1 - Math.min(1, Math.max(0, (event.clientY - rect.top) / Math.max(rect.height, 1)));
    return { h: hsv.h, s, v };
  }

  function hsvFromHue(event: PointerEvent): Hsv | null {
    const el = hueEl;
    if (!el) return null;
    const rect = el.getBoundingClientRect();
    const h = Math.min(1, Math.max(0, (event.clientX - rect.left) / Math.max(rect.width, 1))) * 360;
    return { h, s: hsv.s, v: hsv.v };
  }

  function drag(
    event: PointerEvent,
    read: (event: PointerEvent) => Hsv | null,
    target: HTMLElement
  ) {
    if (disabled || event.button !== 0) return;
    event.preventDefault();
    const next = read(event);
    if (next) emit(next);
    try {
      target.setPointerCapture(event.pointerId);
    } catch {
      // Synthetic events may not support capture.
    }
  }

  function move(event: PointerEvent, read: (event: PointerEvent) => Hsv | null, target: HTMLElement) {
    if (disabled || !target.hasPointerCapture(event.pointerId)) return;
    const next = read(event);
    if (next) emit(next);
  }

  function end(event: PointerEvent, target: HTMLElement) {
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
    onCommit?.(hsvToHex(hsv));
    open = false;
  }

  function toggleOpen() {
    if (disabled) return;
    open = !open;
  }
</script>

<div class="spectrum" class:spectrum-disabled={disabled} class:spectrum-open={open}>
  <div class="spectrum-head">
    <button
      type="button"
      class="spectrum-swatch"
      style={`background: ${hex}`}
      aria-label="Toggle colour picker"
      aria-expanded={open}
      disabled={disabled}
      onclick={toggleOpen}
    ></button>
    <span class="spectrum-hex">{hex}</span>
  </div>

  {#if open}
    <div
      class="spectrum-plane"
      bind:this={planeEl}
      role="slider"
      tabindex={disabled ? -1 : 0}
      aria-label="Saturation and brightness"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(hsv.s * 100)}
      aria-valuetext={`Saturation ${Math.round(hsv.s * 100)}%, brightness ${Math.round(hsv.v * 100)}%`}
      style={`--hue: ${hueColor}`}
      onpointerdown={(event) => planeEl && drag(event, hsvFromPlane, planeEl)}
      onpointermove={(event) => planeEl && move(event, hsvFromPlane, planeEl)}
      onpointerup={(event) => planeEl && end(event, planeEl)}
      onpointercancel={(event) => planeEl && end(event, planeEl)}
    >
      <span
        class="spectrum-dot"
        style={`left: ${hsv.s * 100}%; top: ${(1 - hsv.v) * 100}%; background: ${hex}`}
        aria-hidden="true"
      ></span>
    </div>

    <div
      class="spectrum-hue"
      bind:this={hueEl}
      role="slider"
      tabindex={disabled ? -1 : 0}
      aria-label="Hue"
      aria-valuemin={0}
      aria-valuemax={360}
      aria-valuenow={Math.round(hsv.h)}
      onpointerdown={(event) => hueEl && drag(event, hsvFromHue, hueEl)}
      onpointermove={(event) => hueEl && move(event, hsvFromHue, hueEl)}
      onpointerup={(event) => hueEl && end(event, hueEl)}
      onpointercancel={(event) => hueEl && end(event, hueEl)}
    >
      <span class="spectrum-hue-thumb" style={`left: ${(hsv.h / 360) * 100}%`} aria-hidden="true"></span>
    </div>
  {/if}
</div>

<style>
  .spectrum {
    display: grid;
    gap: 12px;
    width: 100%;
    user-select: none;
    touch-action: none;
  }

  .spectrum-disabled {
    pointer-events: none;
  }

  .spectrum-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .spectrum-swatch {
    width: 36px;
    height: 36px;
    padding: 0;
    border-radius: 10px;
    border: 1px solid rgba(17, 24, 39, 0.16);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.35);
    flex: 0 0 auto;
    cursor: pointer;
    background: #000;
  }

  .spectrum-swatch:focus-visible {
    outline: 2px solid var(--md-blue, #2563eb);
    outline-offset: 3px;
  }

  .spectrum-open .spectrum-swatch {
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.35),
      0 0 0 2px var(--md-blue, #2563eb);
  }

  .spectrum-hex {
    font-size: 0.92rem;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
    color: var(--md-ink, #111827);
    text-transform: uppercase;
  }

  .spectrum-plane {
    position: relative;
    height: 132px;
    border-radius: 12px;
    cursor: crosshair;
    outline: none;
    background:
      linear-gradient(to top, #000, transparent),
      linear-gradient(to right, #fff, var(--hue));
    box-shadow: inset 0 0 0 1px rgba(17, 24, 39, 0.12);
  }

  .spectrum-plane:focus-visible,
  .spectrum-hue:focus-visible {
    outline: 2px solid var(--md-blue, #2563eb);
    outline-offset: 3px;
  }

  .spectrum-dot {
    position: absolute;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px rgba(17, 24, 39, 0.45), 0 2px 8px rgba(17, 24, 39, 0.28);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .spectrum-hue {
    position: relative;
    height: 18px;
    border-radius: 999px;
    cursor: pointer;
    outline: none;
    background: linear-gradient(
      to right,
      #ff0000,
      #ffff00,
      #00ff00,
      #00ffff,
      #0000ff,
      #ff00ff,
      #ff0000
    );
    box-shadow: inset 0 0 0 1px rgba(17, 24, 39, 0.12);
  }

  .spectrum-hue-thumb {
    position: absolute;
    top: 50%;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #fff;
    border: 2px solid #111827;
    box-shadow: 0 2px 8px rgba(17, 24, 39, 0.22);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }
</style>
