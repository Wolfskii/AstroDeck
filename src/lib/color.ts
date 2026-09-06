export const DEFAULT_CONTROLS_OVERLAY_COLOR = "#000000";

const HEX_PATTERN = /^#([0-9a-f]{6})$/i;

export function parseHexColor(value: unknown, fallback = DEFAULT_CONTROLS_OVERLAY_COLOR): string {
  if (typeof value !== "string") return fallback;
  const trimmed = value.trim();
  if (!HEX_PATTERN.test(trimmed)) return fallback;
  return `#${trimmed.slice(1).toLowerCase()}`;
}

export function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const parsed = parseHexColor(hex);
  return {
    r: Number.parseInt(parsed.slice(1, 3), 16),
    g: Number.parseInt(parsed.slice(3, 5), 16),
    b: Number.parseInt(parsed.slice(5, 7), 16),
  };
}

export function rgbToHex(r: number, g: number, b: number): string {
  const toByte = (n: number) =>
    Math.min(255, Math.max(0, Math.round(n)))
      .toString(16)
      .padStart(2, "0");
  return `#${toByte(r)}${toByte(g)}${toByte(b)}`;
}

export type Hsv = { h: number; s: number; v: number };

export function rgbToHsv(r: number, g: number, b: number): Hsv {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const delta = max - min;
  let h = 0;
  if (delta !== 0) {
    if (max === rn) h = ((gn - bn) / delta) % 6;
    else if (max === gn) h = (bn - rn) / delta + 2;
    else h = (rn - gn) / delta + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  const s = max === 0 ? 0 : delta / max;
  return { h, s, v: max };
}

export function hsvToRgb(h: number, s: number, v: number): { r: number; g: number; b: number } {
  const hh = ((h % 360) + 360) % 360;
  const c = v * s;
  const x = c * (1 - Math.abs(((hh / 60) % 2) - 1));
  const m = v - c;
  let rn = 0;
  let gn = 0;
  let bn = 0;
  if (hh < 60) {
    rn = c;
    gn = x;
  } else if (hh < 120) {
    rn = x;
    gn = c;
  } else if (hh < 180) {
    gn = c;
    bn = x;
  } else if (hh < 240) {
    gn = x;
    bn = c;
  } else if (hh < 300) {
    rn = x;
    bn = c;
  } else {
    rn = c;
    bn = x;
  }
  return {
    r: Math.round((rn + m) * 255),
    g: Math.round((gn + m) * 255),
    b: Math.round((bn + m) * 255),
  };
}

export function hsvToHex(hsv: Hsv): string {
  const { r, g, b } = hsvToRgb(hsv.h, hsv.s, hsv.v);
  return rgbToHex(r, g, b);
}

export function hexToHsv(hex: string): Hsv {
  const { r, g, b } = hexToRgb(hex);
  return rgbToHsv(r, g, b);
}

export function hexToRgbCss(hex: string): string {
  const { r, g, b } = hexToRgb(hex);
  return `${r}, ${g}, ${b}`;
}

export function mixHex(from: string, to: string, amount: number): string {
  const t = Math.min(1, Math.max(0, amount));
  const a = hexToRgb(from);
  const b = hexToRgb(to);
  return rgbToHex(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t);
}

export function mixHexPalette(from: string[], to: string[], amount: number): string[] {
  const count = Math.max(from.length, to.length, 1);
  const mixed: string[] = [];
  for (let i = 0; i < count; i += 1) {
    mixed.push(
      mixHex(
        from[i] ?? from[from.length - 1] ?? "#000000",
        to[i] ?? to[to.length - 1] ?? "#000000",
        amount
      )
    );
  }
  return mixed;
}
