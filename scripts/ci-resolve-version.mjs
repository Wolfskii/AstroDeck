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
  const match = String(value).trim().match(/^(\d+)\.(\d+)\.(\d+)(?:-(?:dev|\d+))?$/);
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
  return {
    major: version.major,
    minor: version.minor,
    patch: version.patch + 1,
    raw: `${version.major}.${version.minor}.${version.patch + 1}`,
  };
}

function bumpMinorResetPatch(version) {
  return {
    major: version.major,
    minor: version.minor + 1,
    patch: 0,
    raw: `${version.major}.${version.minor + 1}.0`,
  };
}

function channelTag(versionRaw, releaseChannel) {
  return releaseChannel === "develop" ? `v${versionRaw}-dev` : `v${versionRaw}`;
}

function runGit(command) {
  return execSync(command, { cwd: rootDir, encoding: "utf8" }).trim();
}

function listTags() {
  try {
    // Prefer a fresh remote tag list so local clones/CI checkouts do not miss tags.
    try {
      runGit("git fetch --tags --force");
    } catch {
      // offline / shallow mirrors may not allow fetch; fall back to local tags
    }
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

/** Walk patch numbers until the channel tag is free. */
function nextFreeVersion(start, releaseChannel, existingTags) {
  let current = typeof start === "string" ? parseSemver(start) : start;
  if (!current) {
    throw new Error(`Invalid semver start: ${start}`);
  }
  for (let attempt = 0; attempt < 1000; attempt += 1) {
    const tag = channelTag(current.raw, releaseChannel);
    if (!existingTags.includes(tag)) {
      return { version: current.raw, tag };
    }
    current = bumpPatch(current);
  }
  throw new Error("Could not find a free release version after 1000 bumps");
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
const allVersions = tags.map((tag) => tagToVersion(tag));

const latestStable = maxSemver(stableVersions);
const latestAny = maxSemver(allVersions);
const fallbackBase = latestStable || fileVersion;

let autoStart;
let prerelease;

if (channel === "develop") {
  const base = latestAny || fileVersion;
  autoStart = bumpPatch(base);
  prerelease = true;
} else {
  autoStart = bumpMinorResetPatch(fallbackBase);
  prerelease = false;
}

let candidate = autoStart;
let manualOverride = false;
const wantsManual =
  versionChangedInCommit() && fileVersion.raw !== autoStart.raw;

if (wantsManual) {
  const manualTag = channelTag(fileVersion.raw, channel);
  if (!tags.includes(manualTag)) {
    candidate = fileVersion;
    manualOverride = true;
    console.log(
      `[ci-resolve-version] Using manual VERSION override: ${fileVersion.raw} (auto would be ${autoStart.raw})`
    );
  } else {
    // Stale or reused VERSION — never fail the build; take the next free slot.
    const floor =
      compareSemver(fileVersion, autoStart) >= 0 ? fileVersion : autoStart;
    candidate = floor;
    console.log(
      `[ci-resolve-version] Manual VERSION ${fileVersion.raw} tag ${manualTag} already exists; using next free version from ${floor.raw}`
    );
  }
} else {
  console.log(`[ci-resolve-version] Auto version start: ${autoStart.raw}`);
}

let releaseVersion;
let tagName;
try {
  ({ version: releaseVersion, tag: tagName } = nextFreeVersion(candidate, channel, tags));
} catch (error) {
  console.error(`[ci-resolve-version] ${error instanceof Error ? error.message : error}`);
  process.exit(1);
}

if (releaseVersion !== candidate.raw) {
  console.log(
    `[ci-resolve-version] Advanced ${candidate.raw} → ${releaseVersion} to avoid existing tag`
  );
  // If we had to skip because of collisions, it is no longer a pure manual pin.
  if (manualOverride && releaseVersion !== fileVersion.raw) {
    manualOverride = false;
  }
}

writeFileSync(versionPath, `${releaseVersion}\n`, "utf8");

const manifest = {
  version: releaseVersion,
  tag: tagName,
  prerelease,
  channel,
  manualOverride,
  autoVersion: autoStart.raw,
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
