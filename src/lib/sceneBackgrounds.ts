export const SCENE_BACKGROUND_IDS = [
  "off",
  "mesh-gradient",
  "static-radial",
  "dithering",
  "neuro-noise",
  "grain-gradient",
  "metaballs",
  "pulsing-border",
  "fluted-glass",
  "water",
  "liquid-gradient",
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
    id: "static-radial",
    label: "Static Radial Gradient",
    description: "Still radial wash based on the cover colors.",
  },
  {
    id: "dithering",
    label: "Dithering",
    description: "Animated dither pattern in cover-art colors.",
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

export function usesCoverImage(id: SceneBackgroundId): boolean {
  return id === "fluted-glass" || id === "water";
}
