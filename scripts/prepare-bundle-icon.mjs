import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const sourceIcon = path.join(rootDir, "app-icon.png");
const bundleIcon = path.join(rootDir, "app-icon-bundle.png");
const tauriIconsDir = path.join(rootDir, "src-tauri", "icons");
const uiIcon = path.join(rootDir, "src", "assets", "app-icon.png");

function backgroundFromStats(stats) {
  return {
    r: Math.round(stats.channels[0].mean),
    g: Math.round(stats.channels[1].mean),
    b: Math.round(stats.channels[2].mean),
    alpha: 255,
  };
}

async function prepareBundleIcon() {
  const trimmed = await sharp(sourceIcon).trim({ threshold: 12 }).toBuffer();
  const meta = await sharp(trimmed).metadata();
  const stats = await sharp(trimmed).stats();
  const background = backgroundFromStats(stats);
  const squareSide = Math.max(meta.width ?? 0, meta.height ?? 0);

  const squared = await sharp(trimmed)
    .resize(squareSide, squareSide, { fit: "contain", background })
    .png()
    .toBuffer();

  // Slight overscale so the squircle reaches tray/taskbar edges without a dark halo.
  const oversize = Math.round(512 * 1.16);
  const cropOffset = Math.floor((oversize - 512) / 2);

  await sharp(squared)
    .resize(oversize, oversize, { fit: "cover", position: "centre" })
    .extract({ left: cropOffset, top: cropOffset, width: 512, height: 512 })
    .flatten({ background })
    .png()
    .toFile(bundleIcon);

  console.log(`[icons] Prepared bundle icon at ${bundleIcon}`);
}

function runTauriIcon() {
  const result = spawnSync(
    "npx",
    ["tauri", "icon", bundleIcon, "-o", tauriIconsDir],
    { cwd: rootDir, stdio: "inherit", shell: process.platform === "win32" }
  );

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

async function syncUiIcon() {
  await sharp(sourceIcon).png().toFile(uiIcon);
  console.log(`[icons] Synced UI icon at ${uiIcon}`);
}

await prepareBundleIcon();
runTauriIcon();
await syncUiIcon();
console.log("[icons] Tray/taskbar and in-app icons updated");
