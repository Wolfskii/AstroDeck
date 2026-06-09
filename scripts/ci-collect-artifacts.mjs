import { execSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const tauriDir = path.join(rootDir, "src-tauri");
const platform = process.argv[2];
const target = process.argv[3] || "";

if (!platform) {
  console.error("[ci-collect-artifacts] Usage: node ci-collect-artifacts.mjs <windows|linux|macos> [rust-target]");
  process.exit(1);
}

const targetRoot = target
  ? path.join(tauriDir, "target", target, "release")
  : path.join(tauriDir, "target", "release");

const bundleDir = path.join(targetRoot, "bundle");
const outputDir = path.join(rootDir, "release-artifacts", platform);

function ensureDir(dir) {
  mkdirSync(dir, { recursive: true });
}

function copyFile(source, destinationName) {
  if (!existsSync(source)) return false;
  cpSync(source, path.join(outputDir, destinationName));
  console.log(`[ci-collect-artifacts] ${destinationName}`);
  return true;
}

function copyNewestMatching(dir, pattern, destinationPrefix) {
  if (!existsSync(dir)) return;
  const entries = readdirSync(dir)
    .filter((name) => pattern.test(name))
    .map((name) => {
      const fullPath = path.join(dir, name);
      return { name, fullPath, mtime: statSync(fullPath).mtimeMs };
    })
    .sort((a, b) => b.mtime - a.mtime);

  for (const entry of entries) {
    copyFile(entry.fullPath, `${destinationPrefix}-${entry.name}`);
  }
}

ensureDir(outputDir);

const binaryName = platform === "windows" ? "astrodeck.exe" : "astrodeck";
copyFile(path.join(targetRoot, binaryName), `portable-${binaryName}`);

if (!existsSync(bundleDir)) {
  console.error(`[ci-collect-artifacts] Bundle directory not found: ${bundleDir}`);
  process.exit(1);
}

copyNewestMatching(path.join(bundleDir, "msi"), /\.msi$/i, "installer");
copyNewestMatching(path.join(bundleDir, "nsis"), /\.exe$/i, "installer");
copyNewestMatching(path.join(bundleDir, "deb"), /\.deb$/i, "installer");
copyNewestMatching(path.join(bundleDir, "rpm"), /\.rpm$/i, "installer");
copyNewestMatching(path.join(bundleDir, "appimage"), /\.AppImage$/i, "portable");
copyNewestMatching(path.join(bundleDir, "dmg"), /\.dmg$/i, "installer");

const macosBundleDir = path.join(bundleDir, "macos");
if (existsSync(macosBundleDir)) {
  const appBundles = readdirSync(macosBundleDir)
    .filter((name) => name.endsWith(".app"))
    .map((name) => {
      const fullPath = path.join(macosBundleDir, name);
      return { name, fullPath, mtime: statSync(fullPath).mtimeMs };
    })
    .sort((a, b) => b.mtime - a.mtime);

  if (appBundles[0]) {
    const { name, fullPath } = appBundles[0];
    const zipName = `portable-${name}.zip`;
    const zipPath = path.join(outputDir, zipName);
    if (process.platform === "darwin") {
      execSync(`ditto -c -k --sequesterRsrc --keepParent "${fullPath}" "${zipPath}"`, {
        stdio: "inherit",
      });
    } else {
      cpSync(fullPath, path.join(outputDir, name), { recursive: true });
    }
    console.log(`[ci-collect-artifacts] ${zipName}`);
  }
}

const copied = readdirSync(outputDir);
if (copied.length === 0) {
  console.error("[ci-collect-artifacts] No artifacts were collected");
  process.exit(1);
}

console.log(`[ci-collect-artifacts] Collected ${copied.length} file(s) for ${platform}`);
