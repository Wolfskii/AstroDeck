import thumbsUp from "../assets/teams/thumbs-up.png";
import heart from "../assets/teams/heart.png";
import clap from "../assets/teams/clap.png";
import laugh from "../assets/teams/laugh.png";
import surprise from "../assets/teams/surprise.png";
import raisedHand from "../assets/teams/raised-hand.png";

/** Microsoft Fluent 3D emoji used by Teams live reactions. */
export const TEAMS_EMOJI_BY_ACTION: Record<string, string> = {
  "teams.reaction.like": thumbsUp,
  "teams.reaction.heart": heart,
  "teams.reaction.clap": clap,
  "teams.reaction.laugh": laugh,
  "teams.reaction.wow": surprise,
  "teams.raiseHand": raisedHand,
};

export function teamsEmojiForAction(action: string): string | null {
  return TEAMS_EMOJI_BY_ACTION[action] ?? null;
}
