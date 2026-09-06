export const SCENE_ACTION_ICONS = {
  "teams.reaction.like": "thumbUp",
  "teams.reaction.heart": "heart",
  "teams.reaction.clap": "clap",
  "teams.reaction.laugh": "laugh",
  "teams.reaction.wow": "wow",
  "teams.raiseHand": "hand",
  "teams.toggleMute": "mic",
  "teams.toggleCamera": "video",
  "teams.shareScreen": "shareScreen",
  "vscode.terminal": "terminal",
  "vscode.run": "play",
  "vscode.debug": "bug",
  "vscode.git": "gitBranch",
  "vscode.search": "search",
  "vscode.extensions": "extensions",
} as const;

export type SceneIconName =
  | (typeof SCENE_ACTION_ICONS)[keyof typeof SCENE_ACTION_ICONS]
  | "teamsMark"
  | "vscodeMark"
  | "letter";

export function iconForAction(action: string): SceneIconName | null {
  return SCENE_ACTION_ICONS[action as keyof typeof SCENE_ACTION_ICONS] ?? null;
}
