export interface DeckButtonConfig {
  id?: string;
  label: string;
  emoji?: string;
  image?: string;
  action: string;
  actionSpec?: ActionSpec;
}

export interface PluginTriggers {
  process?: string;
  processGlob?: string;
  processesAny?: string[];
  processesAll?: string[];
  excludeProcesses?: string[];
  windowTitleContains?: string;
  windowTitleGlob?: string;
  windowTitlesAny?: string[];
}

export interface LayoutConfig {
  grid: [number, number];
  buttons: DeckButtonConfig[];
}

export interface ActionSpecOpenUrl {
  kind: "openUrl";
  url: string;
}

export interface ActionSpecOpenPath {
  kind: "openPath";
  path: string;
}

export interface ActionSpecLaunch {
  kind: "launch";
  program: string;
  args?: string[];
}

export type ActionSpec = ActionSpecOpenUrl | ActionSpecOpenPath | ActionSpecLaunch;

export interface MediaPlayerTemplateConfig {
  previous?: DeckButtonConfig;
  playPause?: DeckButtonConfig;
  next?: DeckButtonConfig;
  like?: DeckButtonConfig;
  volumeAction?: string;
}

export interface PluginViewConfig {
  type?: string;
  mediaPlayer?: MediaPlayerTemplateConfig;
}

export interface PluginSourceMeta {
  kind: string;
  manifestPath?: string | null;
  baseDir?: string | null;
}

export interface PluginConfig {
  id: string;
  name: string;
  description?: string | null;
  schemaVersion?: number;
  priority: number;
  triggers: PluginTriggers;
  view?: PluginViewConfig;
  layout: LayoutConfig;
  source?: PluginSourceMeta;
}

export interface SceneState {
  activeSceneId: string;
  layout: LayoutConfig | null;
  availableScenes: string[];
}
