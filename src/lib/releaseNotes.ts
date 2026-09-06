export type ReleaseNoteBlock =
  | { kind: "heading"; text: string }
  | { kind: "list"; items: string[] }
  | { kind: "paragraph"; text: string };

export function parseReleaseNotes(body: string): ReleaseNoteBlock[] {
  const blocks: ReleaseNoteBlock[] = [];
  for (const rawLine of body.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    const heading = line.match(/^#{1,6}\s+(.+)$/);
    if (heading) {
      blocks.push({ kind: "heading", text: heading[1] });
      continue;
    }
    const listItem = line.match(/^[-*]\s+(.+)$/);
    if (listItem) {
      const previous = blocks.at(-1);
      if (previous?.kind === "list") {
        previous.items.push(listItem[1]);
      } else {
        blocks.push({ kind: "list", items: [listItem[1]] });
      }
      continue;
    }
    blocks.push({ kind: "paragraph", text: line });
  }
  return blocks;
}
