import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const sourceArt = path.join(rootDir, "docs", "brand", "astrodeck-star-rider.webp");
const bundleIcon = path.join(rootDir, "app-icon-bundle.png");
const tauriIconsDir = path.join(rootDir, "src-tauri", "icons");
const uiIcon = path.join(rootDir, "src", "assets", "app-icon.png");

const PLATE_FILL = { r: 18, g: 20, b: 28, alpha: 255 };
const MASTER_SIZE = 1024;
/** Windows 11 taskbar / titlebar / tray sizes, including 125% and 150% DPI. */
const WINDOWS_ICO_SIZES = [256, 64, 48, 40, 36, 32, 30, 24, 20, 16];

function plateSvg(size) {
  const small = size <= 48;
  const inset = Math.max(1, Math.round(size * (small ? 0.045 : 0.028)));
  const stroke = Math.max(small ? 2 : 3, Math.round(size * (small ? 0.09 : 0.048)));
  const radius = Math.round(size * (small ? 0.2 : 0.22));
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

async function trimmedMascot() {
  return sharp(sourceArt).ensureAlpha().trim({ threshold: 10 }).png().toBuffer();
}

async function renderAt(size, mascotSource) {
  const small = size <= 48;
  const mascotBox = Math.round(size * (small ? 0.78 : 0.7));
  const mascot = await sharp(mascotSource)
    .resize(mascotBox, mascotBox, {
      fit: "contain",
      kernel: size <= 32 ? sharp.kernel.cubic : sharp.kernel.lanczos3,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    })
    .png()
    .toBuffer();
  const meta = await sharp(mascot).metadata();
  const left = Math.round((size - (meta.width ?? mascotBox)) / 2);
  const top = Math.round((size - (meta.height ?? mascotBox)) / 2);
  const plate = await sharp(plateSvg(size)).png().toBuffer();

  let pipeline = sharp(plate).composite([{ input: mascot, left, top }]);
  if (size <= 64) {
    pipeline = pipeline.sharpen({
      sigma: size <= 32 ? 1.1 : 0.85,
      m1: 1,
      m2: 0.5,
    });
  }
  return pipeline.png().toBuffer();
}

function encodeIco(images) {
  const count = images.length;
  const headerBytes = 6 + 16 * count;
  let offset = headerBytes;
  const entries = images.map((image) => {
    const entry = { ...image, offset, bytes: image.png.length };
    offset += image.png.length;
    return entry;
  });
  const buf = Buffer.alloc(offset);
  buf.writeUInt16LE(0, 0);
  buf.writeUInt16LE(1, 2);
  buf.writeUInt16LE(count, 4);
  let cursor = 6;
  for (const entry of entries) {
    buf.writeUInt8(entry.size >= 256 ? 0 : entry.size, cursor);
    buf.writeUInt8(entry.size >= 256 ? 0 : entry.size, cursor + 1);
    buf.writeUInt8(0, cursor + 2);
    buf.writeUInt8(0, cursor + 3);
    buf.writeUInt16LE(1, cursor + 4);
    buf.writeUInt16LE(32, cursor + 6);
    buf.writeUInt32LE(entry.bytes, cursor + 8);
    buf.writeUInt32LE(entry.offset, cursor + 12);
    cursor += 16;
    entry.png.copy(buf, entry.offset);
  }
  return buf;
}

async function writeWindowsIcons(mascotSource) {
  const rendered = [];
  for (const size of WINDOWS_ICO_SIZES) {
    rendered.push({ size, png: await renderAt(size, mascotSource) });
  }

  const icoPath = path.join(tauriIconsDir, "icon.ico");
  fs.writeFileSync(icoPath, encodeIco(rendered));
  console.log(
    `[icons] Wrote Windows ICO (${WINDOWS_ICO_SIZES.join(", ")}) at ${icoPath}`
  );

  const bySize = new Map(rendered.map((image) => [image.size, image.png]));
  const pngWrites = [
    [32, "32x32.png"],
    [64, "64x64.png"],
    [256, "128x128@2x.png"],
  ];
  for (const [size, name] of pngWrites) {
    const png = bySize.get(size) ?? (await renderAt(size, mascotSource));
    await sharp(png).png().toFile(path.join(tauriIconsDir, name));
  }
  await sharp(await renderAt(128, mascotSource))
    .png()
    .toFile(path.join(tauriIconsDir, "128x128.png"));
}

async function renderAppIcon() {
  const mascotSource = await trimmedMascot();
  const master = await renderAt(MASTER_SIZE, mascotSource);
  await sharp(master).png().toFile(uiIcon);
  console.log(`[icons] Wrote in-app icon at ${uiIcon}`);

  await sharp(master).flatten({ background: PLATE_FILL }).png().toFile(bundleIcon);
  console.log(`[icons] Prepared bundle icon at ${bundleIcon}`);
  return mascotSource;
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

const mascotSource = await renderAppIcon();
runTauriIcon();
await writeWindowsIcons(mascotSource);
console.log("[icons] Tray/taskbar and in-app icons updated");
