import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const sourceArt = path.join(rootDir, "docs", "brand", "astrodeck-star-rider.webp");
const bundleIcon = path.join(rootDir, "app-icon-bundle.png");
const tauriIconsDir = path.join(rootDir, "src-tauri", "icons");
const uiIcon = path.join(rootDir, "src", "assets", "app-icon.png");

const PLATE_SIZE = 1024;
const PLATE_FILL = { r: 18, g: 20, b: 28, alpha: 255 };

function plateSvg(size) {
  const inset = Math.round(size * 0.028);
  const stroke = Math.round(size * 0.048);
  const radius = Math.round(size * 0.22);
  const box = size - inset * 2;
  return Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
  <defs>
    <linearGradient id="ring" x1="8%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#7CFFB2"/>
      <stop offset="32%" stop-color="#FFE566"/>
      <stop offset="68%" stop-color="#FF5A6A"/>
      <stop offset="100%" stop-color="#7AD0FF"/>
    </linearGradient>
  </defs>
  <rect x="${inset}" y="${inset}" width="${box}" height="${box}" rx="${radius}" ry="${radius}" fill="rgb(${PLATE_FILL.r},${PLATE_FILL.g},${PLATE_FILL.b})" stroke="url(#ring)" stroke-width="${stroke}"/>
</svg>`);
}

async function renderAppIcon() {
  const plate = await sharp(plateSvg(PLATE_SIZE)).png().toBuffer();
  const mascotBox = Math.round(PLATE_SIZE * 0.7);
  const mascot = await sharp(sourceArt)
    .ensureAlpha()
    .trim({ threshold: 10 })
    .resize(mascotBox, mascotBox, {
      fit: "contain",
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    })
    .png()
    .toBuffer();
  const meta = await sharp(mascot).metadata();
  const left = Math.round((PLATE_SIZE - (meta.width ?? mascotBox)) / 2);
  const top = Math.round((PLATE_SIZE - (meta.height ?? mascotBox)) / 2);

  const composed = await sharp(plate)
    .composite([{ input: mascot, left, top }])
    .png()
    .toBuffer();

  await sharp(composed).png().toFile(uiIcon);
  console.log(`[icons] Wrote in-app icon at ${uiIcon}`);

  await sharp(composed).flatten({ background: PLATE_FILL }).png().toFile(bundleIcon);
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

await renderAppIcon();
runTauriIcon();
console.log("[icons] Tray/taskbar and in-app icons updated");
