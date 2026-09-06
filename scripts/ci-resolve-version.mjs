import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const channel = process.argv[2];

if (channel !== "develop" && channel !== "production") {
  console.error("[ci-resolve-version] Channel must be develop or production");
  process.exit(1);
}

function parseSemver(value) {
  const match = value.trim().match(/^(\d+)\.(\d+)\.(\d+)(?:-dev)?$/);
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

function maxSemver(versions) {
  const parsed = versions.map(parseSemver).filter(Boolean);
  if (parsed.length === 0) return null;
  return parsed.sort(compareSemver).at(-1);
}

function bumpPatch(version) {
  return `${version.major}.${version.minor}.${version.patch + 1}`;
}

function bumpMinorResetPatch(version) {
  return `${version.major}.${version.minor + 1}.0`;
}

function runGit(command) {
  return execSync(command, { cwd: rootDir, encoding: "utf8" }).trim();
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

function versionChangedInCommit() {
  try {
    const diff = runGit("git diff HEAD~1 HEAD -- VERSION");
    return diff.length > 0;
  } catch {
    return false;
  }
}

const versionPath = path.join(rootDir, "VERSION");
const fileVersionRaw = readFileSync(versionPath, "utf8").trim();
const fileVersion = parseSemver(fileVersionRaw);

if (!fileVersion) {
  console.error(`[ci-resolve-version] VERSION file must contain semver x.y.z, got: ${fileVersionRaw}`);
  process.exit(1);
}

const tags = listTags();
const stableVersions = tags
  .filter((tag) => !tag.endsWith("-dev"))
  .map((tag) => tagToVersion(tag));
const devVersions = tags
  .filter((tag) => tag.endsWith("-dev"))
  .map((tag) => tagToVersion(tag));
const allVersions = tags.map((tag) => tagToVersion(tag));

const latestStable = maxSemver(stableVersions);
const latestAny = maxSemver(allVersions);
const fallbackBase = latestStable || fileVersion;

let autoVersion;
let prerelease;
let tagName;

if (channel === "develop") {
  const base = latestAny || fileVersion;
  autoVersion = bumpPatch(base);
  prerelease = true;
  tagName = `v${autoVersion}-dev`;
} else {
  autoVersion = bumpMinorResetPatch(fallbackBase);
  prerelease = false;
  tagName = `v${autoVersion}`;
}

let releaseVersion = autoVersion;
const manualOverride =
  versionChangedInCommit() && fileVersion.raw !== autoVersion;

if (manualOverride) {
  releaseVersion = fileVersion.raw;
  tagName = channel === "develop" ? `v${releaseVersion}-dev` : `v${releaseVersion}`;
  console.log(
    `[ci-resolve-version] Using manual VERSION override: ${releaseVersion} (auto would be ${autoVersion})`
  );
} else {
  console.log(`[ci-resolve-version] Auto version: ${releaseVersion}`);
}

if (tags.includes(tagName)) {
  console.error(`[ci-resolve-version] Tag ${tagName} already exists`);
  process.exit(1);
}

writeFileSync(versionPath, `${releaseVersion}\n`, "utf8");

const manifest = {
  version: releaseVersion,
  tag: tagName,
  prerelease,
  channel,
  manualOverride,
  autoVersion,
};

const manifestPath = path.join(rootDir, "version-manifest.json");
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

const githubOutput = process.env.GITHUB_OUTPUT;
if (githubOutput) {
  const lines = [
    `version=${releaseVersion}`,
    `tag=${tagName}`,
    `prerelease=${prerelease}`,
    `channel=${channel}`,
    `manual_override=${manualOverride}`,
  ];
  writeFileSync(githubOutput, `${lines.join("\n")}\n`, { flag: "a" });
}

console.log(`[ci-resolve-version] channel=${channel} version=${releaseVersion} tag=${tagName}`);
