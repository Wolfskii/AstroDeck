import type { LayoutConfig } from "../types";

export const IDLE_SCENE_ID = "idle";

export interface BuiltinSceneMeta {
  id: string;
  name: string;
  description: string;
  accent: string;
  layout: LayoutConfig;
}

const builtinScenes: Record<string, BuiltinSceneMeta> = {
  teams: {
    id: "teams",
    name: "Teams Meeting",
    description: "Reactions and in-call controls for meetings.",
    accent: "#6366f1",
    layout: {
      grid: [3, 3],
      buttons: [
        { label: "Like", emoji: "👍", action: "teams.reaction.like" },
        { label: "Love", emoji: "❤️", action: "teams.reaction.heart" },
        { label: "Applause", emoji: "👏", action: "teams.reaction.clap" },
        { label: "Laugh", emoji: "😂", action: "teams.reaction.laugh" },
        { label: "Surprise", emoji: "😮", action: "teams.reaction.wow" },
        { label: "Raise Hand", emoji: "✋", action: "teams.raiseHand" },
        { label: "Mute", emoji: "🎤", action: "teams.toggleMute" },
        { label: "Camera", emoji: "📷", action: "teams.toggleCamera" },
        { label: "Share", emoji: "🖥️", action: "teams.shareScreen" },
      ],
    },
  },
  spotify: {
    id: "spotify",
    name: "Spotify Control",
    description: "Playback and volume controls for Spotify.",
    accent: "#22c55e",
    layout: {
      grid: [2, 2],
      buttons: [
        { label: "Prev Track", emoji: "⏮️", action: "spotify.prevTrack" },
        { label: "Play/Pause", emoji: "⏯️", action: "spotify.togglePlay" },
        { label: "Next Track", emoji: "⏭️", action: "spotify.nextTrack" },
        { label: "Like", emoji: "❤️", action: "spotify.like" },
      ],
    },
  },
  vscode: {
    id: "vscode",
    name: "VS Code",
    description: "Editor, debug, and workspace shortcuts for VS Code and Cursor.",
    accent: "#0ea5e9",
    layout: {
      grid: [2, 3],
      buttons: [
        { label: "Terminal", emoji: "💻", action: "vscode.terminal" },
        { label: "Run", emoji: "▶️", action: "vscode.run" },
        { label: "Debug", emoji: "🐛", action: "vscode.debug" },
        { label: "Git", emoji: "🔀", action: "vscode.git" },
        { label: "Search", emoji: "🔍", action: "vscode.search" },
        { label: "Extensions", emoji: "🧩", action: "vscode.extensions" },
      ],
    },
  },
  media: {
    id: "media",
    name: "System Media",
    description: "Playback controls for local OS media players.",
    accent: "#f59e0b",
    layout: {
      grid: [2, 2],
      buttons: [
        { label: "Prev Track", emoji: "⏮️", action: "media.prevTrack" },
        { label: "Play/Pause", emoji: "⏯️", action: "media.togglePlay" },
        { label: "Next Track", emoji: "⏭️", action: "media.nextTrack" },
      ],
    },
  },
};

export function isIdleScene(id: string | null | undefined): boolean {
  return !id || id === IDLE_SCENE_ID || id === "default";
}

export function getBuiltinLayout(sceneId: string): LayoutConfig | null {
  return builtinScenes[sceneId]?.layout ?? null;
}

export function getBuiltinSceneMeta(sceneId: string): BuiltinSceneMeta | null {
  return builtinScenes[sceneId] ?? null;
}

export function getBuiltinSceneIds(): string[] {
  return Object.keys(builtinScenes);
}
