import { rgbToHsv } from "./color";

export type Rgb = { r: number; g: number; b: number };

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("Failed to load album art"));
    img.src = url;
  });
}

function mixRgb(a: Rgb, b: Rgb, amount: number): Rgb {
  return {
    r: Math.round(a.r * (1 - amount) + b.r * amount),
    g: Math.round(a.g * (1 - amount) + b.g * amount),
    b: Math.round(a.b * (1 - amount) + b.b * amount),
  };
}

function rgbToCss({ r, g, b }: Rgb): string {
  return `rgb(${r}, ${g}, ${b})`;
}

export function rgbToHex({ r, g, b }: Rgb): string {
  return `#${[r, g, b].map((n) => n.toString(16).padStart(2, "0")).join("")}`;
}

function keyToRgb(key: number): Rgb {
  return {
    r: ((key >> 8) & 0xf) * 17,
    g: ((key >> 4) & 0xf) * 17,
    b: (key & 0xf) * 17,
  };
}

export async function extractAlbumPalette(imageUrl: string): Promise<Rgb[] | null> {
  try {
    const img = await loadImage(imageUrl);
    const canvas = document.createElement("canvas");
    const size = 64;
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext("2d");
    if (!ctx) return null;
    ctx.drawImage(img, 0, 0, size, size);
    const data = ctx.getImageData(0, 0, size, size).data;
    const buckets = new Map<number, { count: number; chroma: number }>();

    for (let i = 0; i < data.length; i += 4) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      const a = data[i + 3];
      if (a < 128) continue;
      const lum = 0.299 * r + 0.587 * g + 0.114 * b;
      if (lum < 16 || lum > 252) continue;
      const chroma = Math.max(r, g, b) - Math.min(r, g, b);
      const key = ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4);
      const existing = buckets.get(key);
      if (existing) {
        existing.count += 1;
        existing.chroma = Math.max(existing.chroma, chroma);
      } else {
        buckets.set(key, { count: 1, chroma });
      }
    }

    const rankedByCount = [...buckets.entries()].sort((a, b) => b[1].count - a[1].count);
    const rankedByVivid = [...buckets.entries()].sort((a, b) => {
      const score = (entry: { count: number; chroma: number }) =>
        Math.pow(entry.chroma / 255, 1.45) * Math.sqrt(entry.count);
      return score(b[1]) - score(a[1]);
    });
    if (rankedByCount.length === 0) return null;

    const keys: number[] = [];
    const seen = new Set<number>();
    const push = (key: number) => {
      if (seen.has(key)) return;
      seen.add(key);
      keys.push(key);
    };
    const hueOfKey = (key: number) => {
      const rgb = keyToRgb(key);
      return rgbToHsv(rgb.r, rgb.g, rgb.b).h;
    };
    const hueDelta = (a: number, b: number) => {
      const d = Math.abs(a - b) % 360;
      return Math.min(d, 360 - d);
    };

    push(rankedByCount[0][0]);
    const remaining = rankedByVivid.map(([key]) => key).filter((key) => !seen.has(key));
    while (keys.length < 6 && remaining.length > 0) {
      let bestIdx = 0;
      let bestScore = -1;
      for (let i = 0; i < remaining.length; i += 1) {
        const key = remaining[i];
        const chroma = (buckets.get(key)?.chroma ?? 0) / 255;
        const h = hueOfKey(key);
        const minHue = Math.min(...keys.map((picked) => hueDelta(h, hueOfKey(picked))));
        const score = chroma * 1.15 + (minHue / 180) * 1.55;
        if (score > bestScore) {
          bestScore = score;
          bestIdx = i;
        }
      }
      push(remaining.splice(bestIdx, 1)[0]);
    }
    return keys.map(keyToRgb);
  } catch {
    return null;
  }
}

export async function extractDominantAlbumColor(imageUrl: string): Promise<Rgb | null> {
  const palette = await extractAlbumPalette(imageUrl);
  return palette?.[0] ?? null;
}

export function paletteToShaderColors(palette: Rgb[]): string[] {
  if (palette.length === 0) return [...DEFAULT_SHADER_COLORS];
  const black = { r: 0, g: 0, b: 0 };
  const colors = palette.map((rgb) => rgbToHex(mixRgb(rgb, black, 0.18)));
  while (colors.length < 4) {
    colors.push(colors[colors.length - 1] ?? DEFAULT_SHADER_COLORS[0]);
  }
  return colors.slice(0, 5);
}

export function paletteToCarThingBackgrounds(rgb: Rgb): { body: string; footer: string } {
  const black = { r: 0, g: 0, b: 0 };
  return {
    body: rgbToCss(mixRgb(rgb, black, 0.78)),
    footer: rgbToCss(mixRgb(rgb, black, 0.84)),
  };
}

export const DEFAULT_CAR_BACKGROUNDS = {
  body: "#0a0a0a",
  footer: "#0c0808",
};

export const DEFAULT_SHADER_COLORS = ["#3d4a7a", "#1a2a44", "#5a3d6e", "#12141c"];

