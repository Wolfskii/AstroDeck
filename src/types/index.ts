export interface DeckButtonConfig {
  label: string;
  emoji?: string;
  image?: string;
  action: string;
}

export interface PluginTriggers {
  process?: string;
  windowTitleContains?: string;
}

export interface LayoutConfig {
  grid: [number, number];
  buttons: DeckButtonConfig[];
}

export interface PluginConfig {
  id: string;
  name: string;
  priority: number;
  triggers: PluginTriggers;
  layout: LayoutConfig;
}

export interface SceneState {
  activeSceneId: string;
  layout: LayoutConfig | null;
  availableScenes: string[];
}
