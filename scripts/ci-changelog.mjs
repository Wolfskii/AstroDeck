import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");

function runGit(command) {
  return execSync(command, { cwd: rootDir, encoding: "utf8" }).trim();
}

function parseSemver(value) {
  const match = value.trim().match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    raw: `${match[1]}.${match[2]}.${match[3]}`,
  };
}

function compareSemver(a, b) {
  if (a.major !== b.major) return a.major - b.major;
  if (a.minor !== b.minor) return a.minor - b.minor;
  return a.patch - b.patch;
}

function listTags() {
  try {
    const output = runGit('git tag -l "v*"');
    return output ? output.split("\n").filter(Boolean) : [];
  } catch {
    return [];
  }
}

function tagToVersion(tag) {
  return tag.replace(/^v/, "").replace(/-dev$/, "");
}

function previousTag(channel) {
  const tags = listTags();
  const filtered =
    channel === "develop"
      ? tags.filter((tag) => tag.endsWith("-dev"))
      : tags.filter((tag) => !tag.endsWith("-dev"));

  const sorted = filtered
    .map((tag) => ({ tag, version: parseSemver(tagToVersion(tag)) }))
    .filter((entry) => entry.version)
    .sort((a, b) => compareSemver(a.version, b.version));

  return sorted.at(-1)?.tag ?? "";
}

function formatCommits(commits) {
  if (commits.length === 0) {
    return ["- No commit messages found for this range."];
  }

  return commits.map((commit) => {
    const [hash, subject, author, date] = commit.split("|");
    const shortHash = hash.slice(0, 7);
    const cleanSubject = subject.trim();
    return `- ${cleanSubject} (\`${shortHash}\`, ${author}, ${date})`;
  });
}

const manifestPath = path.join(rootDir, "version-manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const { version, tag, prerelease, channel } = manifest;
const sinceTag = previousTag(channel);
const range = sinceTag ? `${sinceTag}..HEAD` : "HEAD";

let logOutput = "";
try {
  logOutput = runGit(
    `git log ${range} --pretty=format:"%H|%s|%an|%ad" --date=short --no-merges`
  );
} catch {
  logOutput = "";
}

const commits = logOutput ? logOutput.split("\n").filter(Boolean) : [];
commits.sort((a, b) => {
  const dateA = a.split("|")[3] ?? "";
  const dateB = b.split("|")[3] ?? "";
  if (dateA !== dateB) return dateB.localeCompare(dateA);
  return a.localeCompare(b);
});

const releaseLabel = prerelease
  ? "Development pre-release"
  : "Production release";
const sinceLine = sinceTag
  ? `Changes since \`${sinceTag}\`:`
  : "Initial release changes:";

const body = [
  `## AstroDeck v${version}`,
  "",
  `**${releaseLabel}** from \`${channel}\` branch.`,
  "",
  `### Changes`,
  "",
  sinceLine,
  "",
  ...formatCommits(commits),
  "",
  `### Artifacts`,
  "",
  "Installers and portable builds are attached below, grouped by platform.",
  "",
  "| Platform | Installers | Portable |",
  "| --- | --- | --- |",
  "| Windows | `.msi`, NSIS `.exe` setup | standalone `.exe` |",
  "| Linux | `.deb` | `.AppImage` |",
  "| macOS | `.dmg` | `.app` bundle |",
  "",
].join("\n");

const outputPath = path.join(rootDir, "release-notes.md");
writeFileSync(outputPath, `${body}\n`, "utf8");
console.log(`[ci-changelog] Wrote ${outputPath} for ${tag}`);
