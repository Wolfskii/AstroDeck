export const SCENE_BACKGROUND_IDS = [
  "off",
  "mesh-gradient",
  "dithering",
  "neuro-noise",
  "grain-gradient",
  "metaballs",
  "pulsing-border",
  "fluted-glass",
  "water",
  "liquid-gradient",
  "aurora",
  "cosmos",
  "warp",
  "prism",
  "horizon",
  "spectrum",
  "bokeh",
  "ripple",
  "helix",
  "plasma",
  "kaleido",
] as const;

export type SceneBackgroundId = (typeof SCENE_BACKGROUND_IDS)[number];

export const DEFAULT_SCENE_BACKGROUND: SceneBackgroundId = "mesh-gradient";

export const SCENE_BACKGROUND_OPTIONS: {
  id: SceneBackgroundId;
  label: string;
  description: string;
  group: "scene" | "visualizer";
}[] = [
  {
    id: "off",
    label: "Off",
    description: "Solid color from the cover art. Best for performance.",
    group: "scene",
  },
  {
    id: "mesh-gradient",
    label: "Mesh Gradient",
    description: "Flowing color spots from the cover art.",
    group: "scene",
  },
  {
    id: "dithering",
    label: "Dithering",
    description: "Animated dither pattern in cover-art colors.",
    group: "scene",
  },
  {
    id: "neuro-noise",
    label: "Neuro Noise",
    description: "Glowing web of lines tinted by the cover.",
    group: "scene",
  },
  {
    id: "grain-gradient",
    label: "Grain Gradient",
    description: "Grainy animated bands from the cover palette.",
    group: "scene",
  },
  {
    id: "metaballs",
    label: "Metaballs",
    description: "Merging blobs in cover-art colors.",
    group: "scene",
  },
  {
    id: "pulsing-border",
    label: "Pulsing Border",
    description: "Glowing animated border around the cover art.",
    group: "scene",
  },
  {
    id: "fluted-glass",
    label: "Fluted Glass",
    description: "Ribbed glass over the cover art, slowly animated.",
    group: "scene",
  },
  {
    id: "water",
    label: "Water",
    description: "Caustic water over the cover art, tinted by its colors.",
    group: "scene",
  },
  {
    id: "liquid-gradient",
    label: "Liquid Gradient",
    description: "Flowing liquid color fields from the cover palette.",
    group: "scene",
  },
  {
    id: "aurora",
    label: "Aurora",
    description: "Night sky with curtains of light and stars, tinted by the cover.",
    group: "scene",
  },
  {
    id: "cosmos",
    label: "Cosmos",
    description: "A slowly turning galaxy of particles in cover-art colors.",
    group: "scene",
  },
  {
    id: "warp",
    label: "Warp",
    description: "Hyperspace streaks flying toward you, tinted by the cover.",
    group: "scene",
  },
  {
    id: "prism",
    label: "Prism",
    description: "Floating crystals lit by the cover palette.",
    group: "scene",
  },
  {
    id: "horizon",
    label: "Horizon",
    description: "Neon grid and a banded sun, colored from the cover art.",
    group: "scene",
  },
  {
    id: "helix",
    label: "Helix",
    description: "A double helix of particles twisting in cover-art colors.",
    group: "scene",
  },
  {
    id: "kaleido",
    label: "Kaleido",
    description: "Mirrored color shards that spin with the cover palette.",
    group: "scene",
  },
  {
    id: "spectrum",
    label: "Spectrum",
    description: "Circular equalizer bars that pulse like a music visualizer.",
    group: "visualizer",
  },
  {
    id: "bokeh",
    label: "Bokeh",
    description: "Soft glowing orbs that bloom on the beat.",
    group: "visualizer",
  },
  {
    id: "ripple",
    label: "Ripple",
    description: "Shockwave rings expanding from the center of the track.",
    group: "visualizer",
  },
  {
    id: "plasma",
    label: "Plasma",
    description: "Classic liquid plasma visualizer tinted by the cover.",
    group: "visualizer",
  },
];

export function isSceneBackgroundId(value: string): value is SceneBackgroundId {
  return (SCENE_BACKGROUND_IDS as readonly string[]).includes(value);
}

export function parseSceneBackgroundId(value: string | null | undefined): SceneBackgroundId {
  if (value && isSceneBackgroundId(value)) return value;
  return DEFAULT_SCENE_BACKGROUND;
}

export function usesFullViewBackground(id: SceneBackgroundId): boolean {
  return id !== "off" && id !== "pulsing-border";
}

export function usesThreeBackground(id: SceneBackgroundId): boolean {
  return (
    id === "cosmos" ||
    id === "warp" ||
    id === "prism" ||
    id === "horizon" ||
    id === "spectrum" ||
    id === "bokeh" ||
    id === "ripple" ||
    id === "helix"
  );
}

export function usesCoverImage(id: SceneBackgroundId): boolean {
  return id === "fluted-glass" || id === "water";
}

export function usesVisualizerShader(id: SceneBackgroundId): boolean {
  return id === "plasma" || id === "kaleido";
}
