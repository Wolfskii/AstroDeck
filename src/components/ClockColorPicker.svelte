<script lang="ts">
  import { hexToHsv, hsvToHex, parseHexColor, type Hsv } from "../lib/color";
  import { DEFAULT_CLOCK_DIGIT_COLOR } from "../services/prefs";

  let {
    value,
    onChange,
    onCommit,
  }: {
    value: string;
    onChange: (hex: string) => void;
    onCommit: (hex: string) => void;
  } = $props();

  const presets = [
    { label: "Gold", hex: DEFAULT_CLOCK_DIGIT_COLOR },
    { label: "White", hex: "#f5f5f7" },
    { label: "Blue", hex: "#7eb6ff" },
    { label: "Mint", hex: "#5ee0a0" },
    { label: "Coral", hex: "#ff8a6a" },
  ];

  let hsv = $state<Hsv>(hexToHsv(DEFAULT_CLOCK_DIGIT_COLOR));
  let hexDraft = $state(DEFAULT_CLOCK_DIGIT_COLOR.toUpperCase());

  $effect(() => {
    const incoming = parseHexColor(value, DEFAULT_CLOCK_DIGIT_COLOR);
    hexDraft = incoming.toUpperCase();
    if (incoming === hsvToHex(hsv)) return;
    hsv = hexToHsv(incoming);
  });

  const hex = $derived(hsvToHex(hsv));
  const hueColor = $derived(hsvToHex({ h: hsv.h, s: 1, v: 1 }));

  let planeEl = $state<HTMLDivElement | null>(null);
  let hueEl = $state<HTMLDivElement | null>(null);

  function emit(next: Hsv, commit = false) {
    hsv = next;
    const nextHex = hsvToHex(next);
    hexDraft = nextHex.toUpperCase();
    onChange(nextHex);
    if (commit) onCommit(nextHex);
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

  function drag(event: PointerEvent, read: (event: PointerEvent) => Hsv | null, target: HTMLElement) {
    if (event.button !== 0) return;
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
    if (!target.hasPointerCapture(event.pointerId)) return;
    const next = read(event);
    if (next) emit(next);
  }

  function end(event: PointerEvent, target: HTMLElement) {
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
    onCommit(hsvToHex(hsv));
  }

  function commitDraft() {
    const raw = hexDraft.trim();
    const next = parseHexColor(raw.startsWith("#") ? raw : `#${raw}`, "");
    if (!next) {
      hexDraft = hex.toUpperCase();
      return;
    }
    emit(hexToHsv(next), true);
  }

  function selectPreset(nextHex: string) {
    emit(hexToHsv(nextHex), true);
  }
</script>

<div class="picker">
  <div
    class="plane"
    bind:this={planeEl}
    role="slider"
    tabindex="0"
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
      class="plane-dot"
      style={`left: ${hsv.s * 100}%; top: ${(1 - hsv.v) * 100}%; background: ${hex}`}
      aria-hidden="true"
    ></span>
  </div>

  <div
    class="hue"
    bind:this={hueEl}
    role="slider"
    tabindex="0"
    aria-label="Hue"
    aria-valuemin={0}
    aria-valuemax={360}
    aria-valuenow={Math.round(hsv.h)}
    onpointerdown={(event) => hueEl && drag(event, hsvFromHue, hueEl)}
    onpointermove={(event) => hueEl && move(event, hsvFromHue, hueEl)}
    onpointerup={(event) => hueEl && end(event, hueEl)}
    onpointercancel={(event) => hueEl && end(event, hueEl)}
  >
    <span class="hue-thumb" style={`left: ${(hsv.h / 360) * 100}%`} aria-hidden="true"></span>
  </div>

  <div class="meta">
    <span class="swatch" style={`background: ${hex}`} aria-hidden="true"></span>
    <label class="hex-field">
      <span>Hex</span>
      <input
        type="text"
        spellcheck="false"
        autocomplete="off"
        maxlength="7"
        aria-label="Digit color hex"
        bind:value={hexDraft}
        onblur={commitDraft}
        onkeydown={(event) => {
          if (event.key === "Enter") commitDraft();
        }}
      />
    </label>
  </div>

  <div class="presets" role="listbox" aria-label="Color presets">
    {#each presets as preset (preset.hex)}
      <button
        type="button"
        class="preset"
        class:preset-active={hex === preset.hex}
        style={`--preset: ${preset.hex}`}
        aria-label={preset.label}
        aria-pressed={hex === preset.hex}
        onclick={() => selectPreset(preset.hex)}
      ></button>
    {/each}
  </div>
</div>

<style>
  .picker {
    display: grid;
    gap: 14px;
    max-width: 420px;
    margin-top: 14px;
    padding: 16px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 18px;
    background: #12141a;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
    user-select: none;
    touch-action: none;
  }

  .plane {
    position: relative;
    height: 168px;
    border-radius: 14px;
    cursor: crosshair;
    outline: none;
    background:
      linear-gradient(to top, #000, transparent),
      linear-gradient(to right, #fff, var(--hue));
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .plane:focus-visible,
  .hue:focus-visible,
  .hex-field input:focus-visible,
  .preset:focus-visible {
    outline: 2px solid #8ab4ff;
    outline-offset: 3px;
  }

  .plane-dot,
  .hue-thumb {
    position: absolute;
    border-radius: 50%;
    pointer-events: none;
    transform: translate(-50%, -50%);
  }

  .plane-dot {
    width: 18px;
    height: 18px;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.55), 0 6px 16px rgba(0, 0, 0, 0.35);
  }

  .hue {
    position: relative;
    height: 16px;
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
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .hue-thumb {
    top: 50%;
    width: 22px;
    height: 22px;
    background: #fff;
    border: 3px solid #12141a;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.45);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .swatch {
    width: 42px;
    height: 42px;
    flex: 0 0 auto;
    border-radius: 12px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
  }

  .hex-field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 10px;
    min-width: 0;
    height: 42px;
    padding: 0 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    background: #0c0e13;
    color: rgba(245, 245, 247, 0.55);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .hex-field input {
    width: 100%;
    min-width: 0;
    border: none;
    background: transparent;
    color: #f5f5f7;
    font: inherit;
    letter-spacing: 0.06em;
  }

  .hex-field input:focus {
    outline: none;
  }

  .presets {
    display: flex;
    gap: 10px;
  }

  .preset {
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--preset);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
    cursor: pointer;
  }

  .preset-active {
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.2),
      0 0 0 2px #12141a,
      0 0 0 4px #f5f5f7;
  }
</style>
