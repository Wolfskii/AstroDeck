import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const versionArg = process.argv[2]?.trim();
const versionPath = path.join(rootDir, "VERSION");
const version = versionArg || readFileSync(versionPath, "utf8").trim();

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`[sync-version] Invalid semver: ${version}`);
  process.exit(1);
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
