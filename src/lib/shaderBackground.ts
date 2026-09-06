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
  staticRadialGradientFragmentShader,
  waterFragmentShader,
  type ShaderMountUniforms,
} from "@paper-design/shaders";
import { DEFAULT_SHADER_COLORS } from "./albumArtColor";
import { auroraFragmentShader } from "./auroraShader";
import { liquidGradientFragmentShader } from "./liquidGradientShader";
import type { SceneBackgroundId } from "./sceneBackgrounds";

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
      return meshGradientFragmentShader;
    case "static-radial":
      return staticRadialGradientFragmentShader;
    case "dithering":
      return ditheringFragmentShader;
    case "neuro-noise":
      return neuroNoiseFragmentShader;
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
    case "static-radial":
      return {
        speed: 0,
        uniforms: {
          ...sizing("cover"),
          u_colorBack: back,
          u_colors: vecs,
          u_colorsCount: vecs.length,
          u_radius: 1.15,
          u_focalDistance: 0.18,
          u_focalAngle: 40,
          u_falloff: 0.15,
          u_mixing: 0.85,
          u_distortion: 0.12,
          u_distortionShift: 0,
          u_distortionFreq: 4,
          u_grainMixer: 0.1,
          u_grainOverlay: 0.08,
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
          u_thickness: 0.16,
          u_softness: 0.5,
          u_intensity: 0.48,
          u_bloom: 0.55,
          u_spots: 4,
          u_spotSize: 0.5,
          u_pulse: 0.7,
          u_smoke: 0.28,
          u_smokeSize: 0.5,
          u_aspectRatio: PulsingBorderAspectRatios.square,
          u_marginLeft: 0,
          u_marginRight: 0,
          u_marginTop: 0,
          u_marginBottom: 0,
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
      const base = colors[0] ?? DEFAULT_SHADER_COLORS[0];
      const high = colors[1] ?? colors[0] ?? DEFAULT_SHADER_COLORS[2];
      const back = colors[colors.length - 1] ?? DEFAULT_SHADER_COLORS[3];
      const star = colors[2] ?? colors[0] ?? DEFAULT_SHADER_COLORS[0];
      const [br, bg] = rgb3(base);
      const nightHorizon: [number, number, number] = [0.012, 0.047, 0.11];
      const nightZenith: [number, number, number] = [0.027, 0.059, 0.114];
      return {
        speed: 1,
        uniforms: {
          ...sizing("cover"),
          u_dithering: 0.0228,
          u_speed: 0.65,
          u_seed: 14 + br * 18 + bg * 8,
          u_colorBase: liftRgb(base),
          u_colorHigh: liftRgb(high, 0.55),
          u_skyDark: mixRgb3(nightHorizon, scaleRgb(back, 0.35), 0.28),
          u_skyDeep: mixRgb3(nightZenith, scaleRgb(back, 0.45), 0.28),
          u_starDensity: 0.073,
          u_starSize: 0.92,
          u_starBlinkRate: 6.26,
          u_starIntensity: 0.52,
          u_starColor: liftRgb(star, 0.7),
        },
      };
    }
    default:
      return { speed: 0, uniforms: {} };
  }
}
