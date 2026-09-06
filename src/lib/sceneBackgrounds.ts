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
  "silk",
  "vortex",
  "mosaic",
  "rain",
  "embers",
  "lattice",
] as const;

export type SceneBackgroundId = (typeof SCENE_BACKGROUND_IDS)[number];

export const DEFAULT_SCENE_BACKGROUND: SceneBackgroundId = "mesh-gradient";

export const SCENE_BACKGROUND_OPTIONS: {
  id: SceneBackgroundId;
  label: string;
  description: string;
}[] = [
  {
    id: "off",
    label: "Off",
    description: "Solid color from the cover art. Best for performance.",
  },
  {
    id: "mesh-gradient",
    label: "Mesh Gradient",
    description: "Flowing color spots from the cover art.",
  },
  {
    id: "dithering",
    label: "Dithering",
    description: "Swirling dither pattern in cover-art colors.",
  },
  {
    id: "neuro-noise",
    label: "Neuro Noise",
    description: "Glowing web of lines tinted by the cover.",
  },
  {
    id: "grain-gradient",
    label: "Grain Gradient",
    description: "Grainy animated bands from the cover palette.",
  },
  {
    id: "metaballs",
    label: "Metaballs",
    description: "Merging blobs in cover-art colors.",
  },
  {
    id: "pulsing-border",
    label: "Pulsing Border",
    description: "Glowing animated border around the cover art.",
  },
  {
    id: "fluted-glass",
    label: "Fluted Glass",
    description: "Ribbed glass over the cover art, slowly animated.",
  },
  {
    id: "water",
    label: "Water",
    description: "Caustic water over the cover art, tinted by its colors.",
  },
  {
    id: "liquid-gradient",
    label: "Liquid Gradient",
    description: "Flowing liquid color fields from the cover palette.",
  },
  {
    id: "aurora",
    label: "Aurora",
    description: "Night sky with curtains of light and stars, tinted by the cover.",
  },
  {
    id: "cosmos",
    label: "Cosmos",
    description: "A slowly turning galaxy of particles in cover-art colors.",
  },
  {
    id: "warp",
    label: "Warp",
    description: "Hyperspace streaks flying toward you, tinted by the cover.",
  },
  {
    id: "prism",
    label: "Prism",
    description: "Floating crystals lit by the cover palette.",
  },
  {
    id: "horizon",
    label: "Horizon",
    description: "Neon grid and a banded sun, colored from the cover art.",
  },
  {
    id: "helix",
    label: "Helix",
    description: "A double helix of particles twisting in cover-art colors.",
  },
  {
    id: "kaleido",
    label: "Kaleido",
    description: "Mirrored color shards that spin with the cover palette.",
  },
  {
    id: "spectrum",
    label: "Spectrum",
    description: "Circular equalizer bars that rise and spin with the cover palette.",
  },
  {
    id: "bokeh",
    label: "Bokeh",
    description: "Soft glowing orbs that drift through the cover palette.",
  },
  {
    id: "ripple",
    label: "Ripple",
    description: "Shockwave rings expanding from the center of the track.",
  },
  {
    id: "plasma",
    label: "Plasma",
    description: "Classic liquid plasma visualizer tinted by the cover.",
  },
  {
    id: "silk",
    label: "Silk",
    description: "Iridescent folds that drift like fabric in the cover palette.",
  },
  {
    id: "vortex",
    label: "Vortex",
    description: "A spiral tunnel pulling inward, colored from the cover art.",
  },
  {
    id: "mosaic",
    label: "Mosaic",
    description: "Shifting stained-glass cells in cover-art colors.",
  },
  {
    id: "rain",
    label: "Rain",
    description: "Falling streaks tinted by the cover, drifting with a light wind.",
  },
  {
    id: "embers",
    label: "Embers",
    description: "Sparks rising and wandering through the cover palette.",
  },
  {
    id: "lattice",
    label: "Lattice",
    description: "Wireframe crystals turning slowly in cover-art colors.",
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
    id === "helix" ||
    id === "rain" ||
    id === "embers" ||
    id === "lattice"
  );
}

export function usesCoverImage(id: SceneBackgroundId): boolean {
  return id === "fluted-glass" || id === "water";
}

export function usesVisualizerShader(id: SceneBackgroundId): boolean {
  return id === "plasma" || id === "kaleido";
}
