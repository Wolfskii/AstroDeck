<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { performanceFrosted, speedTestProvider } from "../stores/appearance";
  import {
    emptySpeedSnapshot,
    runSpeedTest,
    speedProviderLabel,
    type SpeedSnapshot,
  } from "../lib/speedTest";

  type DiskStat = { name: string; used: number; total: number };
  type SystemSnapshot = {
    cpuPercent: number;
    cpuGhz: number;
    cpuCores: number;
    cpuName: string;
    memoryUsed: number;
    memoryTotal: number;
    gpuName: string;
    gpuPercent: number | null;
    gpuMemoryTotal: number;
    netDownBps: number;
    netUpBps: number;
    disks: DiskStat[];
  };

  let stats = $state<SystemSnapshot | null>(null);
  let publicIp = $state<string | null>(null);
  let ipCopied = $state(false);
  let speed = $state<SpeedSnapshot>(emptySpeedSnapshot());
  let speedAbort: AbortController | null = null;
  let ipCopyTimer: number | null = null;

  const speedRunning = $derived(
    speed.phase === "ping" || speed.phase === "download" || speed.phase === "upload"
  );

  const ramPercent = $derived(
    stats && stats.memoryTotal > 0 ? (stats.memoryUsed / stats.memoryTotal) * 100 : 0
  );
  const gpuPercent = $derived(stats?.gpuPercent ?? 0);

  onMount(() => {
    let stopped = false;
    const pull = () => {
      void invoke<SystemSnapshot>("get_system_stats")
        .then((next) => {
          if (!stopped) stats = next;
        })
        .catch(() => {});
    };
    const pullIp = () => {
      void fetchPublicIp()
        .then((ip) => {
          if (!stopped && ip) publicIp = ip;
        })
        .catch(() => {});
    };
    pull();
    pullIp();
    const timer = window.setInterval(pull, 1000);
    const ipTimer = window.setInterval(pullIp, 5 * 60_000);
    return () => {
      stopped = true;
      window.clearInterval(timer);
      window.clearInterval(ipTimer);
      if (ipCopyTimer != null) window.clearTimeout(ipCopyTimer);
      speedAbort?.abort();
    };
  });

  const showSpeed = $derived(speed.phase !== "idle");
  const speedWhere = $derived([speed.server, speed.location].filter(Boolean).join(" · "));

  function startSpeedTest() {
    speedAbort?.abort();
    speedAbort = new AbortController();
    void runSpeedTest($speedTestProvider, (next) => {
      speed = next;
    }, speedAbort.signal);
  }

  function closeSpeedTest() {
    speedAbort?.abort();
    speedAbort = null;
    speed = emptySpeedSnapshot();
  }

  async function fetchPublicIp() {
    const response = await fetch("https://1.1.1.1/cdn-cgi/trace", { cache: "no-store" });
    if (!response.ok) return null;
    const text = await response.text();
    const line = text.split("\n").find((row) => row.startsWith("ip="));
    const ip = line?.slice(3).trim() ?? "";
    return ip || null;
  }

  async function copyPublicIp() {
    if (!publicIp) return;
    try {
      await navigator.clipboard.writeText(publicIp);
      ipCopied = true;
      if (ipCopyTimer != null) window.clearTimeout(ipCopyTimer);
      ipCopyTimer = window.setTimeout(() => {
        ipCopied = false;
        ipCopyTimer = null;
      }, 1400);
    } catch {
      // clipboard may be unavailable
    }
  }

  function formatMbps(value: number | null) {
    if (value == null) return "—";
    return value >= 100 ? value.toFixed(0) : value.toFixed(1);
  }

  function mbpsFill(value: number | null) {
    if (value == null) return 0;
    return Math.min(100, (value / 500) * 100);
  }

  function lossFill(value: number | null) {
    if (value == null) return 0;
    return Math.min(100, value);
  }

  function jitterFill(value: number | null) {
    if (value == null) return 0;
    return Math.min(100, (value / 50) * 100);
  }

  function formatLoss(value: number | null) {
    if (value == null) return "—";
    return value < 10 ? `${value.toFixed(1)}%` : `${Math.round(value)}%`;
  }

  function formatJitter(value: number | null) {
    if (value == null) return "—";
    return `${Math.round(value)} ms`;
  }

  function arc(start: number, sweep: number, r = 38, cx = 50, cy = 46) {
    const point = (deg: number) => {
      const rad = (deg * Math.PI) / 180;
      return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)];
    };
    const [x1, y1] = point(start);
    const [x2, y2] = point(start + sweep);
    const large = sweep > 180 ? 1 : 0;
    return `M ${x1.toFixed(2)} ${y1.toFixed(2)} A ${r} ${r} 0 ${large} 1 ${x2.toFixed(2)} ${y2.toFixed(2)}`;
  }

  function ticks(start: number, sweep: number, count: number, r = 38, cx = 50, cy = 46) {
    return Array.from({ length: count }, (_, index) => {
      const deg = start + (sweep * index) / (count - 1);
      const rad = (deg * Math.PI) / 180;
      const outer = r - 2.2;
      const inner = r - 6.4;
      return {
        x1: cx + inner * Math.cos(rad),
        y1: cy + inner * Math.sin(rad),
        x2: cx + outer * Math.cos(rad),
        y2: cy + outer * Math.sin(rad),
      };
    });
  }

  function formatBytes(bytes: number) {
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    const digits = value >= 100 || unit <= 1 ? 0 : 1;
    return `${value.toFixed(digits)} ${units[unit]}`;
  }

  function formatRate(bps: number) {
    return `${formatBytes(bps)}/s`;
  }

  function ratePercent(bps: number) {
    return Math.min(100, (bps / 12_500_000) * 100);
  }

  function shortGpuName(name: string) {
    const cleaned = name.replace(/\(r\)|\(tm\)|®|™/gi, " ").replace(/\s+/g, " ").trim();
    const model = cleaned.match(
      /\b((?:RTX|GTX|RX|Arc|UHD|Iris)\s+[A-Z]?\d{3,5}(?:\s*Ti)?)/i
    );
    if (model) return model[1].replace(/\s+/g, " ");
    const shorter = cleaned
      .replace(/\b(nvidia|amd|intel|geforce|radeon|graphics|super)\b/gi, "")
      .replace(/\s+/g, " ")
      .trim();
    return shorter || "GPU";
  }

  function diskPercent(disk: DiskStat) {
    if (disk.total <= 0) return 0;
    return Math.min(100, (disk.used / disk.total) * 100);
  }

  const sideArc = arc(152, 236, 34, 50, 50);
  const sideTicks = ticks(152, 236, 9, 34, 50, 50);
  const heroArc = arc(155, 230, 34, 50, 50);
  const heroTicks = ticks(155, 230, 11, 34, 50, 50);
  const diskColors = ["#7dffb3", "#8ec5ff", "#ff8fa3", "#f3d37a"];
</script>

<section class="perf-view">
  <div class="stage" class:stage-opaque={!$performanceFrosted}>
    <div class="speed-actions">
      {#if !showSpeed}
        <button type="button" class="speed-btn" onclick={startSpeedTest}>Test speed</button>
      {:else if speedRunning}
        <span class="speed-btn is-quiet">Testing…</span>
        <button type="button" class="speed-btn speed-btn-ghost" onclick={closeSpeedTest}>Back</button>
      {:else}
        <button type="button" class="speed-btn" onclick={startSpeedTest}>Run again</button>
        <button type="button" class="speed-btn speed-btn-ghost" onclick={closeSpeedTest}>Back</button>
      {/if}
    </div>
    <div class="swap" class:swap-hidden={showSpeed} inert={showSpeed}>
    <div class="gauges">
      <article class="gauge">
        <div class="dial">
        <svg viewBox="0 0 100 78" aria-hidden="true">
          <path d={sideArc} class="track" />
          {#each sideTicks as tick}
            <line x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} class="tick" />
          {/each}
          <path
            d={sideArc}
            class="level"
            pathLength="100"
            style:stroke-dasharray={`${ramPercent} 100`}
          />
        </svg>
        <div class="readout">
          <span class="kicker">Memory</span>
          <strong>{stats ? formatBytes(stats.memoryUsed).replace(" ", "") : "—"}</strong>
          <em>{stats ? `of ${formatBytes(stats.memoryTotal)}` : "RAM"}</em>
        </div>
        </div>
        <div class="ring-row">
          <div class="ring-item">
            <svg class="ring" viewBox="0 0 36 36" aria-hidden="true">
              <circle cx="18" cy="18" r="14" class="ring-track" />
              <circle
                cx="18"
                cy="18"
                r="14"
                class="ring-level ring-level-down"
                pathLength="100"
                style:stroke-dasharray={`${ratePercent(stats?.netDownBps ?? 0)} 100`}
              />
              <path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M18 10.2v11.2M13.6 16.8 18 21.4l4.4-4.6" />
            </svg>
            <div>
              <strong>{stats ? formatRate(stats.netDownBps) : "—"}</strong>
              <span>Down</span>
            </div>
          </div>
          <div class="ring-item">
            <svg class="ring" viewBox="0 0 36 36" aria-hidden="true">
              <circle cx="18" cy="18" r="14" class="ring-track" />
              <circle
                cx="18"
                cy="18"
                r="14"
                class="ring-level"
                pathLength="100"
                style:stroke-dasharray={`${ratePercent(stats?.netUpBps ?? 0)} 100`}
              />
              <path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M18 25.2V14M13.6 18.6 18 14l4.4 4.6" />
            </svg>
            <div>
              <strong>{stats ? formatRate(stats.netUpBps) : "—"}</strong>
              <span>Up</span>
            </div>
          </div>
        </div>
      </article>

      <article class="gauge gauge-hero">
        {#if publicIp}
          <button
            type="button"
            class="public-ip"
            class:copied={ipCopied}
            title={ipCopied ? "Copied" : "Copy IP"}
            onclick={copyPublicIp}
          >
            {ipCopied ? "Copied" : publicIp}
          </button>
        {/if}
        <div class="dial">
        <svg viewBox="0 0 100 78" aria-hidden="true">
          <path d={heroArc} class="track" />
          {#each heroTicks as tick}
            <line x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} class="tick" />
          {/each}
          <path
            d={heroArc}
            class="level"
            pathLength="100"
            style:stroke-dasharray={`${stats?.cpuPercent ?? 0} 100`}
          />
        </svg>
        <div class="readout readout-hero">
          <span class="kicker">CPU</span>
          <strong>{stats ? Math.round(stats.cpuPercent) : "—"}{#if stats}<span class="unit">%</span>{/if}</strong>
          <em>{stats ? `${stats.cpuGhz.toFixed(2)} GHz · ${stats.cpuCores} cores` : "percent"}</em>
        </div>
        </div>
      </article>

      <article class="gauge">
        <div class="dial">
        <svg viewBox="0 0 100 78" aria-hidden="true">
          <path d={sideArc} class="track" />
          {#each sideTicks as tick}
            <line x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} class="tick" />
          {/each}
          <path
            d={sideArc}
            class="level"
            pathLength="100"
            style:stroke-dasharray={`${gpuPercent} 100`}
          />
        </svg>
        <div class="readout">
          <span class="kicker">GPU</span>
          <strong>{stats?.gpuPercent == null ? "—" : Math.round(stats.gpuPercent)}{#if stats?.gpuPercent != null}<span class="unit">%</span>{/if}</strong>
          <em>{stats?.gpuName ? shortGpuName(stats.gpuName) : "Graphics"}</em>
        </div>
        </div>
      </article>
    </div>

    <div class="disks" style={`--disk-count: ${Math.max(stats?.disks.length ?? 1, 1)}`}>
      {#if stats && stats.disks.length > 0}
        {#each stats.disks as disk, index (`${disk.name}-${index}`)}
          <div class="disk">
            <div class="disk-label">
              <span>{disk.name}</span>
              <span>{formatBytes(disk.used)} / {formatBytes(disk.total)}</span>
            </div>
            <div class="disk-track">
              <i style:width={`${diskPercent(disk)}%`} style:background={diskColors[index % diskColors.length]}></i>
            </div>
          </div>
        {/each}
      {:else}
        <p class="empty">Looking for storage…</p>
      {/if}
    </div>
    </div>

    <div class="swap" class:swap-hidden={!showSpeed} inert={!showSpeed}>
      <div class="gauges">
        <article class="gauge" class:speed-live={speed.phase === "ping"}>
          <div class="dial">
            <svg viewBox="0 0 100 78" aria-hidden="true">
              <path d={sideArc} class="track" />
              {#each sideTicks as tick}
                <line x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} class="tick" />
              {/each}
              <path d={sideArc} class="level" pathLength="100" style:stroke-dasharray={`${speed.pingMs == null ? 0 : Math.min(100, Math.max(8, 120 - speed.pingMs))} 100`} />
            </svg>
            <div class="readout">
              <span class="kicker">Ping</span>
              <strong>{speed.pingMs == null ? "—" : Math.round(speed.pingMs)}{#if speed.pingMs != null}<span class="unit">ms</span>{/if}</strong>
            </div>
          </div>
          <div class="ring-row">
            <div class="ring-item">
              <svg class="ring" viewBox="0 0 36 36" aria-hidden="true">
                <circle cx="18" cy="18" r="14" class="ring-track" />
                <circle
                  cx="18"
                  cy="18"
                  r="14"
                  class="ring-level ring-level-loss"
                  pathLength="100"
                  style:stroke-dasharray={`${lossFill(speed.lossPercent)} 100`}
                />
              </svg>
              <div>
                <strong>{formatLoss(speed.lossPercent)}</strong>
                <span>Loss</span>
              </div>
            </div>
            <div class="ring-item">
              <svg class="ring" viewBox="0 0 36 36" aria-hidden="true">
                <circle cx="18" cy="18" r="14" class="ring-track" />
                <circle
                  cx="18"
                  cy="18"
                  r="14"
                  class="ring-level ring-level-jitter"
                  pathLength="100"
                  style:stroke-dasharray={`${jitterFill(speed.jitterMs)} 100`}
                />
              </svg>
              <div>
                <strong>{formatJitter(speed.jitterMs)}</strong>
                <span>Jitter</span>
              </div>
            </div>
          </div>
        </article>
        <article class="gauge gauge-hero" class:speed-live={speed.phase === "download"}>
          <div class="dial">
            <svg viewBox="0 0 100 78" aria-hidden="true">
              <path d={heroArc} class="track" />
              {#each heroTicks as tick}
                <line x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} class="tick" />
              {/each}
              <path d={heroArc} class="level" pathLength="100" style:stroke-dasharray={`${mbpsFill(speed.downMbps)} 100`} />
            </svg>
            <div class="readout">
              <span class="kicker">Download</span>
              <strong>{formatMbps(speed.downMbps)}</strong>
              <em>Mbps</em>
            </div>
          </div>
        </article>
        <article class="gauge" class:speed-live={speed.phase === "upload"}>
          <div class="dial">
            <svg viewBox="0 0 100 78" aria-hidden="true">
              <path d={sideArc} class="track" />
              {#each sideTicks as tick}
                <line x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} class="tick" />
              {/each}
              <path d={sideArc} class="level" pathLength="100" style:stroke-dasharray={`${mbpsFill(speed.upMbps)} 100`} />
            </svg>
            <div class="readout">
              <span class="kicker">Upload</span>
              <strong>{formatMbps(speed.upMbps)}</strong>
              <em>Mbps</em>
            </div>
          </div>
        </article>
      </div>
      {#if speedWhere}
        <p class="speed-where">{speedWhere}</p>
      {/if}
      <p class="speed-status" class:speed-error={speed.phase === "error"}>
        {speed.phase === "ping"
          ? "Measuring ping"
          : speed.phase === "download"
            ? "Measuring download"
            : speed.phase === "upload"
              ? "Measuring upload"
              : speed.phase === "error"
                ? speed.error
                : `Measured with ${speedProviderLabel($speedTestProvider)}`}
      </p>
    </div>
  </div>
</section>

<style>
  .perf-view {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    width: 100%;
    background: transparent;
    color: #f4f7fb;
  }

  .stage {
    position: relative;
    display: grid;
    height: auto;
    min-height: 0;
    margin: 1.4rem 2.2rem 1.4rem calc(var(--side-inset, 144px) + 1.2rem);
    padding: 1.15rem 2.4rem 2.8rem;
    border-radius: 32px;
    overflow: hidden;
    background-color: rgba(10, 14, 22, 0.38);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);
  }

  .stage-opaque {
    background-color: #05060a;
    box-shadow: none;
  }

  .swap {
    display: flex;
    grid-area: 1 / 1;
    min-width: 0;
    flex-direction: column;
    justify-content: flex-start;
    gap: 0.2rem;
    transition: opacity 0.38s ease, transform 0.38s ease;
  }

  .swap-hidden {
    position: absolute;
    inset: 0;
    opacity: 0;
    transform: translateY(10px);
    pointer-events: none;
    overflow: hidden;
  }

  .speed-actions {
    position: absolute;
    top: 0.85rem;
    right: 1.1rem;
    z-index: 2;
    display: flex;
    gap: 0.45rem;
  }

  .speed-btn {
    min-height: 2.1rem;
    padding: 0.35rem 0.85rem;
    border: none;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.14);
    color: #f4f7fb;
    font-size: 0.82rem;
    font-weight: 650;
    letter-spacing: 0.02em;
    cursor: pointer;
  }

  .speed-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.22);
  }

  .speed-btn-ghost {
    background: transparent;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.28);
  }

  .is-quiet {
    opacity: 0.7;
    cursor: default;
  }

  .speed-live .level {
    stroke: #f3d37a;
  }

  .speed-where {
    margin: 0.85rem 0 0;
    color: rgba(244, 247, 251, 0.86);
    font-size: 0.98rem;
    font-weight: 650;
    text-align: center;
  }

  .swap:not(:has(.disks)) > .gauges + .speed-where,
  .swap:not(:has(.disks)) > .gauges + .speed-status {
    margin-top: -3.2rem;
  }

  .speed-status {
    margin: 0.4rem 0 1.8rem;
    color: rgba(244, 247, 251, 0.62);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-align: center;
    text-transform: uppercase;
  }

  .speed-error {
    color: #ffb4b4;
    text-transform: none;
    letter-spacing: 0;
  }

  .gauges {
    display: grid;
    grid-template-columns: minmax(0, 0.72fr) minmax(0, 1.4fr) minmax(0, 0.72fr);
    align-items: start;
    gap: 0.4rem;
  }

  .gauge {
    position: relative;
    min-width: 0;
  }

  .gauges > .gauge:not(.gauge-hero) {
    transform: translateY(8rem);
  }

  .gauges > .gauge:not(.gauge-hero) .dial {
    transform: none;
  }

  .gauge > .dial > svg {
    display: block;
    width: 100%;
    overflow: visible;
  }

  .track {
    fill: none;
    stroke: rgba(255, 255, 255, 0.14);
    stroke-width: 3.2;
    stroke-linecap: round;
  }

  .level {
    fill: none;
    stroke: #f7f8fb;
    stroke-width: 3.2;
    stroke-linecap: round;
    stroke-dasharray: 0 100;
    transition: stroke-dasharray 1s cubic-bezier(0.22, 0.61, 0.36, 1);
  }

  .tick {
    stroke: rgba(255, 255, 255, 0.42);
    stroke-width: 0.7;
    stroke-linecap: round;
  }

  .dial {
    position: relative;
  }

  .public-ip {
    position: relative;
    z-index: 2;
    display: block;
    margin: 0.35rem auto -2.35rem;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    color: rgba(244, 247, 251, 0.88);
    font-size: 1.8rem;
    font-weight: 650;
    letter-spacing: 0.02em;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }

  .public-ip:hover {
    background: transparent;
    color: #f4f7fb;
  }

  .public-ip.copied {
    color: #7dffb3;
  }

  .gauge-hero .dial {
    transform: translateY(-2rem) scale(1.05, 0.98);
    transform-origin: center center;
  }

  .readout {
    position: absolute;
    left: 50%;
    top: 57%;
    display: grid;
    width: 70%;
    justify-items: center;
    text-align: center;
    transform: translate(-50%, -50%);
  }

  .gauge-hero .readout {
    top: 60%;
    width: 76%;
    transform: translate(-50%, -46%);
  }

  .kicker {
    color: rgba(244, 247, 251, 0.55);
    font-size: 0.78rem;
    font-weight: 650;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .gauge:not(.gauge-hero) .kicker {
    font-size: 0.68rem;
  }

  .gauge:not(.gauge-hero) .readout strong {
    font-size: clamp(1.85rem, 3.6vw, 3.15rem);
  }

  .gauge:not(.gauge-hero) .readout em {
    font-size: clamp(0.72rem, 1.15vw, 0.9rem);
  }

  .readout strong {
    font-weight: 650;
    letter-spacing: -0.04em;
    line-height: 0.9;
    margin-bottom: 0.22em;
  }

  .gauge-hero .readout strong {
    font-size: clamp(3.4rem, 7vw, 6.4rem);
  }

  .unit {
    margin-left: 0.02em;
    font-weight: 650;
    letter-spacing: 0;
  }

  .readout em {
    max-width: 16rem;
    overflow: hidden;
    color: rgba(244, 247, 251, 0.72);
    font-size: clamp(0.85rem, 1.4vw, 1.05rem);
    font-style: normal;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ring-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.65rem;
    width: 100%;
    min-height: 3.1rem;
    margin-top: 0.15rem;
  }

  .ring-item {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }

  .ring-item > div {
    text-align: left;
  }

  .ring {
    width: 2.7rem;
    height: 2.7rem;
    flex: 0 0 auto;
    color: #f4f7fb;
  }

  .ring-track,
  .ring-level {
    fill: none;
    stroke-width: 3.4;
    stroke-linecap: round;
  }

  .ring-track {
    stroke: rgba(255, 255, 255, 0.16);
  }

  .ring-level {
    stroke: #f4f7fb;
    stroke-dasharray: 0 100;
    transform: rotate(-90deg);
    transform-origin: 18px 18px;
    transition: stroke-dasharray 1s cubic-bezier(0.22, 0.61, 0.36, 1);
  }

  .ring-level-down {
    stroke: #3dde7a;
  }

  .ring-level-loss {
    stroke: #ff8fa3;
  }

  .ring-level-jitter {
    stroke: #8ec5ff;
  }

  .ring-item strong {
    display: block;
    font-size: clamp(0.95rem, 1.6vw, 1.25rem);
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .ring-item span {
    color: rgba(244, 247, 251, 0.62);
    font-size: 0.72rem;
    font-weight: 650;
  }

  .disks {
    display: grid;
    grid-template-columns: repeat(var(--disk-count), minmax(0, 1fr));
    width: min(100%, calc(var(--disk-count) * 18rem));
    margin: -3.4rem auto 0;
    gap: 1.2rem 1.8rem;
  }

  .disk-label {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    margin-bottom: 0.5rem;
    color: rgba(244, 247, 251, 0.72);
    font-size: 1.05rem;
    font-weight: 650;
  }

  .disk-track {
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.12);
  }

  .disk-track i {
    display: block;
    width: 0;
    height: 100%;
    border-radius: inherit;
    background: #d7dee8;
    transition: width 1s cubic-bezier(0.22, 0.61, 0.36, 1);
  }

  .empty {
    margin: 0;
    color: rgba(244, 247, 251, 0.62);
  }
</style>
