export const BUILTIN_APPS = [
  { id: "clock", name: "Home", color: "#f3d37a" },
  { id: "spotify", name: "Spotify", color: "#1db954" },
  { id: "youtubeMusic", name: "YouTube", color: "#ff0033" },
  { id: "media", name: "System media", color: "#f59e0b" },
  { id: "teams", name: "Teams", color: "#7b83eb" },
  { id: "vscode", name: "VS Code", color: "#3794ff" },
  { id: "weather", name: "Weather", color: "#38bdf8" },
] as const;

export type BuiltinAppId = (typeof BUILTIN_APPS)[number]["id"];

export function isBuiltinAppId(id: string): id is BuiltinAppId {
  return BUILTIN_APPS.some((app) => app.id === id);
}
