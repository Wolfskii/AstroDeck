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

export async function extractDominantAlbumColor(imageUrl: string): Promise<Rgb | null> {
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
    const buckets = new Map<number, number>();

    for (let i = 0; i < data.length; i += 4) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      const a = data[i + 3];
      if (a < 128) continue;
      const lum = 0.299 * r + 0.587 * g + 0.114 * b;
      if (lum < 28 || lum > 235) continue;
      const key = ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4);
      buckets.set(key, (buckets.get(key) ?? 0) + 1);
    }

    let bestKey = 0;
    let bestCount = 0;
    for (const [key, count] of buckets) {
      if (count > bestCount) {
        bestCount = count;
        bestKey = key;
      }
    }
    if (bestCount === 0) return null;

    return {
      r: ((bestKey >> 8) & 0xf) * 17,
      g: ((bestKey >> 4) & 0xf) * 17,
      b: (bestKey & 0xf) * 17,
    };
  } catch {
    return null;
  }
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
