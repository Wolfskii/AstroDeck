import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const args = process.argv.slice(2).filter(Boolean);
const bumpDev = args.includes("--bump-dev");
const versionArg = args.find((arg) => arg !== "--bump-dev")?.trim();
const versionPath = path.join(rootDir, "VERSION");
/** MSI only accepts numeric prerelease identifiers (not `-dev`). */
const versionPattern = /^(\d+)\.(\d+)\.(\d+)(?:-(dev|\d+))?$/;

function parseVersion(raw) {
  const match = raw.trim().match(versionPattern);
  if (!match) return null;
  const prereleaseRaw = match[4];
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease:
      prereleaseRaw == null
        ? null
        : prereleaseRaw === "dev"
          ? "dev"
          : Number(prereleaseRaw),
  };
}

function formatVersion(parsed) {
  const core = `${parsed.major}.${parsed.minor}.${parsed.patch}`;
  if (parsed.prerelease == null) return core;
  if (parsed.prerelease === "dev") return `${core}-1`;
  return `${core}-${parsed.prerelease}`;
}

function bumpLocalDevVersion(raw) {
  const parsed = parseVersion(raw);
  if (!parsed) return null;
  if (parsed.prerelease === "dev") {
    return formatVersion({ ...parsed, prerelease: 1 });
  }
  if (typeof parsed.prerelease === "number") {
    return formatVersion({ ...parsed, prerelease: parsed.prerelease + 1 });
  }
  return formatVersion({
    ...parsed,
    patch: parsed.patch + 1,
    prerelease: 1,
  });
}

let version = versionArg || readFileSync(versionPath, "utf8").trim();

if (bumpDev) {
  const next = bumpLocalDevVersion(version);
  if (!next) {
    console.error(`[sync-version] Invalid semver for local bump: ${version}`);
    process.exit(1);
  }
  console.log(`[sync-version] Local deploy bump ${version} → ${next}`);
  version = next;
}

if (!versionPattern.test(version) || version.endsWith("-dev")) {
  const parsed = parseVersion(version);
  if (!parsed) {
    console.error(`[sync-version] Invalid semver: ${version}`);
    process.exit(1);
  }
  version = formatVersion(parsed);
}

writeFileSync(versionPath, `${version}\n`, "utf8");

const packageJsonPath = path.join(rootDir, "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
packageJson.version = version;
writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`, "utf8");

const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");
let cargoToml = readFileSync(cargoTomlPath, "utf8");
cargoToml = cargoToml.replace(/^version = ".*"$/m, `version = "${version}"`);
writeFileSync(cargoTomlPath, cargoToml, "utf8");

const tauriConfPath = path.join(rootDir, "src-tauri", "tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf8"));
tauriConf.version = version;
writeFileSync(tauriConfPath, `${JSON.stringify(tauriConf, null, 2)}\n`, "utf8");

console.log(`[sync-version] Synced version ${version}`);
