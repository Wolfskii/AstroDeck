import type { LayoutConfig } from "../types";

export interface BuiltinSceneMeta {
  id: string;
  name: string;
  description: string;
  accent: string;
  layout: LayoutConfig;
}

const builtinScenes: Record<string, BuiltinSceneMeta> = {
  default: {
    id: "default",
    name: "Default Deck",
    description: "Core app actions and general controls.",
    accent: "#8b5cf6",
    layout: {
      grid: [2, 2],
      buttons: [
        { label: "Settings", emoji: "⚙️", action: "core.settings" },
        { label: "Plugins", emoji: "🧩", action: "core.plugins" },
        { label: "Refresh", emoji: "🔄", action: "core.refresh" },
        { label: "Info", emoji: "ℹ️", action: "core.info" },
      ],
    },
  },
  teams: {
    id: "teams",
    name: "Teams Meeting",
    description: "Reactions and in-call controls for meetings.",
    accent: "#6366f1",
    layout: {
      grid: [3, 3],
      buttons: [
        { label: "Like", emoji: "👍", action: "teams.reaction.like" },
        { label: "Heart", emoji: "❤️", action: "teams.reaction.heart" },
        { label: "Clap", emoji: "👏", action: "teams.reaction.clap" },
        { label: "Laugh", emoji: "😂", action: "teams.reaction.laugh" },
        { label: "Wow", emoji: "😮", action: "teams.reaction.wow" },
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
      grid: [2, 3],
      buttons: [
        { label: "Play/Pause", emoji: "⏯️", action: "spotify.togglePlay" },
        { label: "Next Track", emoji: "⏭️", action: "spotify.nextTrack" },
        { label: "Prev Track", emoji: "⏮️", action: "spotify.prevTrack" },
        { label: "Volume Up", emoji: "🔊", action: "spotify.volumeUp" },
        { label: "Volume Down", emoji: "🔉", action: "spotify.volumeDown" },
        { label: "Like", emoji: "❤️", action: "spotify.like" },
      ],
    },
  },
  vscode: {
    id: "vscode",
    name: "VS Code",
    description: "Editor, debug, and workspace shortcuts.",
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
};

export function getBuiltinLayout(sceneId: string): LayoutConfig | null {
  return builtinScenes[sceneId]?.layout ?? null;
}

export function getBuiltinSceneMeta(sceneId: string): BuiltinSceneMeta | null {
  return builtinScenes[sceneId] ?? null;
}

export function getBuiltinSceneIds(): string[] {
  return Object.keys(builtinScenes);
}
