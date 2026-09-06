import {
  DitheringShapes,
  DitheringTypes,
  GlassDistortionShapes,
  GlassGridShapes,
  GrainGradientShapes,
  PulsingBorderAspectRatios,
  ShaderFitOptions,
  ditheringFragmentShader,
  flutedGlassFragmentShader,
  getShaderColorFromString,
  getShaderNoiseTexture,
  grainGradientFragmentShader,
  meshGradientFragmentShader,
  metaballsFragmentShader,
  neuroNoiseFragmentShader,
  pulsingBorderFragmentShader,
  waterFragmentShader,
  type ShaderMountUniforms,
} from "@paper-design/shaders";
import { DEFAULT_SHADER_COLORS } from "./albumArtColor";
import { auroraFragmentShader } from "./auroraShader";
import { hexToHsv, hsvToHex } from "./color";
import { liquidGradientFragmentShader } from "./liquidGradientShader";
import type { SceneBackgroundId } from "./sceneBackgrounds";
import { kaleidoFragmentShader, plasmaFragmentShader } from "./visualizerShaders";
import { mosaicFragmentShader, silkFragmentShader, vortexFragmentShader } from "./flowShaders";

const meshGradientStableShader = meshGradientFragmentShader.replace(
  "cos(.2 * t + i * 2.4 * smoothstep(.0, 1., uv.y))",
  "cos(i * 2.4 * smoothstep(.0, 1., uv.y) + uv.x * 1.8)"
).replace(
  "fragColor = vec4(color, opacity);",
  `float luma = dot(color, vec3(0.299, 0.587, 0.114));
  float compressed = mix(luma, 0.33, 0.42);
  color *= compressed / max(luma, 0.06);
  fragColor = vec4(color, opacity);`
);

const neuroNoiseStableShader = neuroNoiseFragmentShader.replace(
  "noise = min(1.4, noise);",
  `noise = min(1.4, noise);
  noise = mix(pow(noise, 0.82), noise, 0.28);
  noise = 0.2 + 0.72 * smoothstep(0.08, 1.05, noise);`
);

const sizing = (fit: keyof typeof ShaderFitOptions = "cover") => ({
  u_fit: ShaderFitOptions[fit],
  u_scale: 1,
  u_rotation: 0,
  u_originX: 0.5,
  u_originY: 0.5,
  u_offsetX: 0,
  u_offsetY: 0,
  u_worldWidth: 0,
  u_worldHeight: 0,
});

function rgb3(color: string): [number, number, number] {
  const [r, g, b] = getShaderColorFromString(color);
  return [r, g, b];
}

function scaleRgb(color: string, amount: number): [number, number, number] {
  const [r, g, b] = rgb3(color);
  return [r * amount, g * amount, b * amount];
}

function mixRgb3(
  a: [number, number, number],
  b: [number, number, number],
  amount: number
): [number, number, number] {
  return [
    a[0] * (1 - amount) + b[0] * amount,
    a[1] * (1 - amount) + b[1] * amount,
    a[2] * (1 - amount) + b[2] * amount,
  ];
}

function liftRgb(color: string, targetLuma = 0.48): [number, number, number] {
  const [r, g, b] = rgb3(color);
  const luma = 0.299 * r + 0.587 * g + 0.114 * b;
  if (luma >= targetLuma) return [r, g, b];
  const boost = targetLuma / Math.max(luma, 0.04);
  return [Math.min(1, r * boost), Math.min(1, g * boost), Math.min(1, b * boost)];
}

function chromaOf(color: string): number {
  const [r, g, b] = rgb3(color);
  return Math.max(r, g, b) - Math.min(r, g, b);
}

function vividRgb(color: string, targetLuma = 0.58, minChroma = 0.38): [number, number, number] {
  let [r, g, b] = liftRgb(color, targetLuma);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const chroma = max - min;
  if (chroma < minChroma && max > 0.04) {
    const mid = (r + g + b) / 3;
    const scale = minChroma / Math.max(chroma, 0.05);
    r = Math.min(1, Math.max(0, mid + (r - mid) * scale));
    g = Math.min(1, Math.max(0, mid + (g - mid) * scale));
    b = Math.min(1, Math.max(0, mid + (b - mid) * scale));
  }
  return liftRgb(
    `#${[r, g, b]
      .map((n) =>
        Math.round(n * 255)
          .toString(16)
          .padStart(2, "0")
      )
      .join("")}`,
    targetLuma
  );
}

function hueOf(color: string): number {
  return hexToHsv(color).h;
}

function lumaOf(color: string): number {
  const [r, g, b] = rgb3(color);
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

function hueDelta(a: number, b: number): number {
  const d = Math.abs(a - b) % 360;
  return Math.min(d, 360 - d);
}

function hash01(colors: string[]): number {
  let hash = 2166136261;
  for (const color of colors) {
    for (let i = 0; i < color.length; i += 1) {
      hash ^= color.charCodeAt(i);
      hash = Math.imul(hash, 16777619);
    }
  }
  return (hash >>> 0) / 4294967296;
}

function shiftHue(color: string, degrees: number): string {
  const hsv = hexToHsv(color);
  return hsvToHex({
    h: (hsv.h + degrees + 360) % 360,
    s: Math.max(hsv.s, 0.48),
    v: Math.max(hsv.v, 0.52),
  });
}

function pickAuroraPair(colors: string[]): { base: string; high: string; rng: number } {
  const rng = hash01(colors);
  const ranked = [...colors].sort((a, b) => chromaOf(b) - chromaOf(a));
  const start = Math.min(ranked.length - 1, Math.floor(rng * Math.min(3, ranked.length)));
  let base = ranked[start] ?? ranked[0] ?? DEFAULT_SHADER_COLORS[0];
  let high = ranked[0] ?? base;
  let best = -1;
  for (const color of ranked) {
    if (color === base) continue;
    const score =
      (hueDelta(hueOf(color), hueOf(base)) / 180) * 1.7 +
      chromaOf(color) * 0.85 +
      Math.abs(lumaOf(color) - lumaOf(base)) * 0.9;
    if (score > best) {
      best = score;
      high = color;
    }
  }
  if (hueDelta(hueOf(base), hueOf(high)) < 42) {
    high = shiftHue(high, 72 + rng * 108);
  }
  if (lumaOf(base) > lumaOf(high)) {
    const swap = base;
    base = high;
    high = swap;
  }
  return { base, high, rng };
}

function colorsToVec4(colors: string[]) {
  return colors.map((color) => getShaderColorFromString(color));
}

let noiseTexture: HTMLImageElement | undefined;

function noiseImage(): HTMLImageElement | undefined {
  if (noiseTexture) return noiseTexture;
  noiseTexture = getShaderNoiseTexture();
  return noiseTexture;
}

export function fragmentForStyle(style: SceneBackgroundId): string | null {
  switch (style) {
    case "mesh-gradient":
      return meshGradientStableShader;
    case "dithering":
      return ditheringFragmentShader;
    case "neuro-noise":
      return neuroNoiseStableShader;
    case "grain-gradient":
      return grainGradientFragmentShader;
    case "metaballs":
      return metaballsFragmentShader;
    case "pulsing-border":
      return pulsingBorderFragmentShader;
    case "fluted-glass":
      return flutedGlassFragmentShader;
    case "water":
      return waterFragmentShader;
    case "liquid-gradient":
      return liquidGradientFragmentShader;
    case "aurora":
      return auroraFragmentShader;
    case "plasma":
      return plasmaFragmentShader;
    case "kaleido":
      return kaleidoFragmentShader;
    case "silk":
      return silkFragmentShader;
    case "vortex":
      return vortexFragmentShader;
    case "mosaic":
      return mosaicFragmentShader;
    default:
      return null;
  }
}

export function uniformsForStyle(
  style: SceneBackgroundId,
  palette: string[],
  image?: HTMLImageElement
): { uniforms: ShaderMountUniforms; speed: number } {
  const colors = palette.length ? palette : DEFAULT_SHADER_COLORS;
  const vecs = colorsToVec4(colors);
  const back = getShaderColorFromString(colors[colors.length - 1] ?? "#0a0a0a");
  const front = getShaderColorFromString(colors[0] ?? "#ffffff");
  const mid = getShaderColorFromString(colors[1] ?? colors[0] ?? "#888888");
  const noise = noiseImage();

  switch (style) {
    case "mesh-gradient":
      return {
        speed: 0.22,
        uniforms: {
          ...sizing("cover"),
          u_colors: vecs,
          u_colorsCount: vecs.length,
          u_distortion: 0.72,
          u_swirl: 0.22,
          u_grainMixer: 0.08,
          u_grainOverlay: 0.06,
        },
      };
    case "dithering":
      return {
        speed: 0.35,
        uniforms: {
          ...sizing("cover"),
          u_colorBack: back,
          u_colorFront: front,
          u_shape: DitheringShapes.warp,
          u_type: DitheringTypes["4x4"],
          u_pxSize: 2.4,
          u_scale: 0.85,
        },
      };
    case "neuro-noise":
      return {
        speed: 0.28,
        uniforms: {
          ...sizing("cover"),
          u_colorFront: front,
          u_colorMid: mid,
          u_colorBack: back,
          u_brightness: 0.12,
          u_contrast: 0.28,
          u_scale: 1.1,
        },
      };
    case "grain-gradient":
      return {
        speed: 0.25,
        uniforms: {
          ...sizing("cover"),
          u_colorBack: back,
          u_colors: vecs.slice(0, 7),
          u_colorsCount: Math.min(vecs.length, 7),
          u_softness: 0.7,
          u_intensity: 0.45,
          u_noise: 0.35,
          u_shape: GrainGradientShapes.blob,
          u_noiseTexture: noise,
        },
      };
    case "metaballs":
      return {
        speed: 0.32,
        uniforms: {
          ...sizing("cover"),
          u_colorBack: back,
          u_colors: vecs.slice(0, 8),
          u_colorsCount: Math.min(vecs.length, 8),
          u_count: 12,
          u_size: 0.72,
          u_noiseTexture: noise,
        },
      };
    case "pulsing-border":
      return {
        speed: 0.7,
        uniforms: {
          ...sizing("contain"),
          u_colorBack: [0, 0, 0, 0],
          u_colors: vecs.slice(0, 5),
          u_colorsCount: Math.min(vecs.length, 5),
          u_roundness: 0.03,
          u_thickness: 0.1,
          u_softness: 0.42,
          u_intensity: 0.52,
          u_bloom: 0.5,
          u_spots: 4,
          u_spotSize: 0.5,
          u_pulse: 0.7,
          u_smoke: 0.22,
          u_smokeSize: 0.45,
          u_aspectRatio: PulsingBorderAspectRatios.square,
          u_marginLeft: 0.02,
          u_marginRight: 0.02,
          u_marginTop: 0.02,
          u_marginBottom: 0.02,
          u_scale: 1,
          u_noiseTexture: noise,
        },
      };
    case "fluted-glass":
      return {
        speed: 0,
        uniforms: {
          ...sizing("cover"),
          u_image: image,
          u_colorBack: back,
          u_colorShadow: [0, 0, 0, 1],
          u_colorHighlight: front,
          u_shadows: 0.28,
          u_highlights: 0.16,
          u_size: 0.55,
          u_shape: GlassGridShapes.wave,
          u_distortionShape: GlassDistortionShapes.contour,
          u_distortion: 0.55,
          u_angle: 12,
          u_shift: 0,
          u_stretch: 0.35,
          u_blur: 0.12,
          u_edges: 0.35,
          u_marginLeft: 0,
          u_marginRight: 0,
          u_marginTop: 0,
          u_marginBottom: 0,
          u_grainMixer: 0.06,
          u_grainOverlay: 0.05,
          u_noiseTexture: noise,
        },
      };
    case "water":
      return {
        speed: 0.35,
        uniforms: {
          ...sizing("cover"),
          u_image: image,
          u_colorBack: back,
          u_colorHighlight: front,
          u_highlights: 0.35,
          u_layering: 0.65,
          u_edges: 0.45,
          u_caustic: 0.55,
          u_waves: 0.4,
          u_size: 1.15,
        },
      };
    case "liquid-gradient": {
      const spots = [
        colors[0] ?? DEFAULT_SHADER_COLORS[0],
        colors[colors.length - 1] ?? DEFAULT_SHADER_COLORS[3],
        colors[2] ?? colors[0] ?? DEFAULT_SHADER_COLORS[2],
        colors[Math.max(0, colors.length - 2)] ?? DEFAULT_SHADER_COLORS[1],
        colors[1] ?? colors[0] ?? DEFAULT_SHADER_COLORS[0],
        colors[colors.length - 1] ?? DEFAULT_SHADER_COLORS[3],
      ];
      return {
        speed: 0.45,
        uniforms: {
          ...sizing("cover"),
          u_color1: rgb3(spots[0]),
          u_color2: rgb3(spots[1]),
          u_color3: rgb3(spots[2]),
          u_color4: rgb3(spots[3]),
          u_color5: rgb3(spots[4]),
          u_color6: rgb3(spots[5]),
          u_colorBack: rgb3(colors[colors.length - 1] ?? "#0a0e27"),
          u_speed: 1.5,
          u_intensity: 1.8,
          u_grainIntensity: 0.08,
          u_gradientSize: 0.45,
          u_gradientCount: 12,
          u_color1Weight: 0.5,
          u_color2Weight: 1.8,
        },
      };
    }
    case "aurora": {
      const { base, high, rng } = pickAuroraPair(colors);
      const back = colors[colors.length - 1] ?? DEFAULT_SHADER_COLORS[3];
      const star = high;
      const nightHorizon: [number, number, number] = [0.012, 0.047, 0.11];
      const nightZenith: [number, number, number] = [0.027, 0.059, 0.114];
      return {
        speed: 1,
        uniforms: {
          ...sizing("cover"),
          u_dithering: 0.0228,
          u_speed: 0.48 + rng * 0.42,
          u_seed: 2 + rng * 94,
          u_colorBase: vividRgb(base, 0.4 + rng * 0.1, 0.52),
          u_colorHigh: vividRgb(high, 0.72 + rng * 0.14, 0.58),
          u_skyDark: mixRgb3(nightHorizon, scaleRgb(back, 0.35), 0.18),
          u_skyDeep: mixRgb3(nightZenith, scaleRgb(back, 0.45), 0.18),
          u_starDensity: 0.073,
          u_starSize: 0.92,
          u_starBlinkRate: 6.26,
          u_starIntensity: 0.52,
          u_starColor: vividRgb(star, 0.82, 0.4),
        },
      };
    }
    case "plasma":
    case "kaleido":
    case "silk":
    case "vortex":
    case "mosaic": {
      const speed =
        style === "vortex" ? 0.48 : style === "silk" ? 0.62 : style === "mosaic" ? 0.55 : 0.72;
      return {
        speed,
        uniforms: {
          ...sizing("cover"),
          u_colorA: liftRgb(colors[0] ?? "#7dd3fc", 0.48),
          u_colorB: liftRgb(colors[1] ?? colors[0] ?? "#c084fc", 0.44),
          u_colorC: liftRgb(colors[2] ?? colors[0] ?? "#fb7185", 0.5),
          u_colorBack: rgb3(colors[colors.length - 1] ?? "#071018"),
          u_speed: 0.85,
        },
      };
    }
    default:
      return { speed: 0, uniforms: {} };
  }
}

const PALETTE_UNIFORM_KEY = /^(u_color|u_colors|u_sky)/;

export function paletteUniformsForStyle(
  style: SceneBackgroundId,
  palette: string[],
  image?: HTMLImageElement
): ShaderMountUniforms {
  const { uniforms } = uniformsForStyle(style, palette, image);
  const next: ShaderMountUniforms = {};
  for (const [key, value] of Object.entries(uniforms)) {
    if (
      !(PALETTE_UNIFORM_KEY.test(key) || key === "u_seed" || key === "u_speed") ||
      value instanceof HTMLImageElement
    ) {
      continue;
    }
    next[key] = value;
  }
  return next;
}

function lerpUniformValue(from: unknown, to: unknown, amount: number): unknown {
  if (typeof from === "number" && typeof to === "number") {
    return from + (to - from) * amount;
  }
  if (Array.isArray(from) && Array.isArray(to)) {
    const count = Math.max(from.length, to.length);
    return Array.from({ length: count }, (_, index) =>
      lerpUniformValue(
        from[index] ?? from[from.length - 1],
        to[index] ?? to[to.length - 1],
        amount
      )
    );
  }
  return amount >= 1 ? to : from;
}

export function lerpPaletteUniforms(
  from: ShaderMountUniforms,
  to: ShaderMountUniforms,
  amount: number
): ShaderMountUniforms {
  const t = Math.min(1, Math.max(0, amount));
  const next: ShaderMountUniforms = {};
  const keys = new Set([...Object.keys(from), ...Object.keys(to)]);
  for (const key of keys) {
    if (key === "u_seed" || key === "u_speed") {
      next[key] = (to[key] ?? from[key]) as ShaderMountUniforms[string];
      continue;
    }
    next[key] = lerpUniformValue(from[key], to[key], t) as ShaderMountUniforms[string];
  }
  return next;
}
