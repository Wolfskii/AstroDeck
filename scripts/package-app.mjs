import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "..");
const tauriDir = path.join(rootDir, "src-tauri");
const artifactsRoot = path.join(rootDir, ".artifacts");
const dockerLinuxImage = "taptapdeck-linux-builder:latest";

const hostPlatformMap = {
  win32: "windows",
  darwin: "macos",
  linux: "linux",
};

const defaultTargets = {
  windows: "x86_64-pc-windows-msvc",
  macos: "aarch64-apple-darwin",
  linux: "x86_64-unknown-linux-gnu",
};

function parseArgs(argv) {
  const parsed = {
    platform: hostPlatformMap[process.platform] ?? "windows",
    target: "",
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--platform" && argv[i + 1]) {
      parsed.platform = argv[i + 1];
      i += 1;
    } else if (arg === "--target" && argv[i + 1]) {
      parsed.target = argv[i + 1];
      i += 1;
    }
  }

  return parsed;
}

function run(command, args, options = {}) {
  const extraPathEntries = [];
  const userProfile = process.env.USERPROFILE || process.env.HOME;
  if (userProfile) {
    const cargoBin = path.join(userProfile, ".cargo", "bin");
    if (existsSync(cargoBin)) {
      extraPathEntries.push(cargoBin);
    }
  }

  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: {
      ...process.env,
      CI: "false",
      PATH: [...extraPathEntries, process.env.PATH || ""].filter(Boolean).join(path.delimiter),
    },
    ...options,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function runCapture(command, args, options = {}) {
  return spawnSync(command, args, {
    cwd: rootDir,
    stdio: "pipe",
    encoding: "utf8",
    shell: process.platform === "win32",
    ...options,
  });
}

function resolveBuildTarget(platform, target) {
  if (target) return target;
  if (platform === (hostPlatformMap[process.platform] ?? platform)) {
    return "";
  }
  return defaultTargets[platform] ?? "";
}

function bundleDirForTarget(target) {
  if (target) {
    return path.join(tauriDir, "target", target, "release", "bundle");
  }
  return path.join(tauriDir, "target", "release", "bundle");
}

function copyArtifacts(sourceDir, platform, target) {
  if (!existsSync(sourceDir)) {
    throw new Error(`Expected bundle output at ${sourceDir}, but it was not found.`);
  }

  const destinationDir = path.join(artifactsRoot, platform);
  rmSync(destinationDir, { recursive: true, force: true });
  mkdirSync(destinationDir, { recursive: true });
  cpSync(sourceDir, destinationDir, { recursive: true });

  const metadata = {
    platform,
    target: target || null,
    hostPlatform: process.platform,
    copiedFrom: sourceDir,
    copiedAt: new Date().toISOString(),
  };

  writeFileSync(
    path.join(destinationDir, "build-info.json"),
    `${JSON.stringify(metadata, null, 2)}\n`,
    "utf8"
  );

  return destinationDir;
}

function ensureDependenciesInstalled() {
  const inDocker = process.env.TAPTAPDECK_IN_DOCKER === "1";
  const nodeModulesDir = path.join(rootDir, "node_modules");
  const viteExists = existsSync(path.join(nodeModulesDir, ".bin", "vite"));

  if (!inDocker && existsSync(nodeModulesDir) && viteExists) {
    return;
  }

  if (inDocker) {
    console.log("[package] Running npm ci inside Docker for Linux build");
    run("npm", ["ci"]);
  } else {
    console.log("[package] node_modules missing or incomplete, running npm install");
    run("npm", ["install"]);
  }
}

function fail(message) {
  console.error(`[package] ${message}`);
  process.exit(1);
}

function requireDockerReady(targetPlatform) {
  const version = runCapture("docker", ["version", "--format", "{{.Server.Version}}"]);
  if (version.error) {
    fail(
      `Docker is required to build ${targetPlatform} from ${process.platform}, but 'docker' was not found. Install Docker Desktop and start it first.`
    );
  }

  const info = runCapture("docker", ["info"]);
  if (info.status !== 0) {
    fail(
      `Docker is installed but not ready. Start Docker Desktop before building ${targetPlatform} from ${process.platform}.`
    );
  }

  console.log("[package] Docker is available and running");
}

function buildLinuxViaDocker(platform, target) {
  requireDockerReady(platform);

  console.log("[package] Building Linux artifacts inside Docker");

  run("docker", [
    "build",
    "-f",
    "scripts/docker/linux-build.Dockerfile",
    "-t",
    dockerLinuxImage,
    ".",
  ]);

  run("docker", [
    "run",
    "--rm",
    "-v",
    `${rootDir}:/workspace`,
    "-v",
    "taptapdeck-node-modules:/workspace/node_modules",
    "-w",
    "/workspace",
    "-e",
    "TAPTAPDECK_IN_DOCKER=1",
    dockerLinuxImage,
    "node",
    "scripts/package-app.mjs",
    "--platform",
    platform,
    ...(target ? ["--target", target] : []),
  ]);
}

function main() {
  const { platform, target } = parseArgs(process.argv.slice(2));
  const resolvedTarget = resolveBuildTarget(platform, target);
  const hostPlatform = hostPlatformMap[process.platform];

  console.log(`[package] host=${process.platform} requestedPlatform=${platform}`);
  if (resolvedTarget) {
    console.log(`[package] using target=${resolvedTarget}`);
  } else {
    console.log("[package] using native host target");
  }

  if (!hostPlatform) {
    throw new Error(`Unsupported host platform: ${process.platform}`);
  }

  if (platform === "macos" && hostPlatform !== "macos") {
    fail(
      "macOS bundles cannot be built from this host OS. Docker does not provide a supported macOS packaging environment for Tauri. Use a real macOS machine or macOS CI runner."
    );
  }

  if (
    platform === "linux" &&
    hostPlatform !== "linux" &&
    process.env.TAPTAPDECK_IN_DOCKER !== "1"
  ) {
    buildLinuxViaDocker(platform, resolvedTarget);
    return;
  }

  ensureDependenciesInstalled();
  run("npm", ["run", "build"]);

  const tauriArgs = ["tauri", "build"];
  if (resolvedTarget) {
    tauriArgs.push("--target", resolvedTarget);
  }
  run("npx", tauriArgs);

  const sourceDir = bundleDirForTarget(resolvedTarget);
  const destinationDir = copyArtifacts(sourceDir, platform, resolvedTarget);

  console.log(`[package] bundled artifacts copied to ${destinationDir}`);
  console.log(`[package] native installers/bundles remain in ${sourceDir}`);
}

main();
