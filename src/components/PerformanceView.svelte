<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { performanceFrosted } from "../stores/appearance";

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
    pull();
    const timer = window.setInterval(pull, 1000);
    return () => {
      stopped = true;
      window.clearInterval(timer);
    };
  });

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
            stroke-dasharray={`${ramPercent} 100`}
          />
        </svg>
        <div class="readout">
          <span class="kicker">Memory</span>
          <strong>{stats ? formatBytes(stats.memoryUsed).replace(" ", "") : "—"}</strong>
          <em>{stats ? `of ${formatBytes(stats.memoryTotal)}` : "RAM"}</em>
        </div>
        </div>
        <div class="ring-row">
          <svg class="ring" viewBox="0 0 36 36" aria-hidden="true">
            <circle cx="18" cy="18" r="14" class="ring-track" />
            <circle
              cx="18"
              cy="18"
              r="14"
              class="ring-level ring-level-down"
              pathLength="100"
              stroke-dasharray={`${ratePercent(stats?.netDownBps ?? 0)} 100`}
            />
            <path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M18 10.2v11.2M13.6 16.8 18 21.4l4.4-4.6" />
          </svg>
          <div>
            <strong>{stats ? formatRate(stats.netDownBps) : "—"}</strong>
            <span>Download</span>
          </div>
        </div>
      </article>

      <article class="gauge gauge-hero">
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
            stroke-dasharray={`${stats?.cpuPercent ?? 0} 100`}
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
            stroke-dasharray={`${gpuPercent} 100`}
          />
        </svg>
        <div class="readout">
          <span class="kicker">GPU</span>
          <strong>{stats?.gpuPercent == null ? "—" : Math.round(stats.gpuPercent)}{#if stats?.gpuPercent != null}<span class="unit">%</span>{/if}</strong>
          <em>{stats?.gpuName ? shortGpuName(stats.gpuName) : "Graphics"}</em>
        </div>
        </div>
        <div class="ring-row">
          <svg class="ring" viewBox="0 0 36 36" aria-hidden="true">
            <circle cx="18" cy="18" r="14" class="ring-track" />
            <circle
              cx="18"
              cy="18"
              r="14"
              class="ring-level"
              pathLength="100"
              stroke-dasharray={`${ratePercent(stats?.netUpBps ?? 0)} 100`}
            />
            <path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M18 25.2V14M13.6 18.6 18 14l4.4 4.6" />
          </svg>
          <div>
            <strong>{stats ? formatRate(stats.netUpBps) : "—"}</strong>
            <span>Upload</span>
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
              <i style={`width: ${diskPercent(disk)}%; background: ${diskColors[index % diskColors.length]}`}></i>
            </div>
          </div>
        {/each}
      {:else}
        <p class="empty">Looking for storage…</p>
      {/if}
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
    display: flex;
    height: auto;
    max-height: calc(100% - 2.8rem);
    min-height: 0;
    flex-direction: column;
    justify-content: center;
    gap: 0.35rem;
    margin: 1.4rem 2.2rem 1.4rem calc(var(--side-inset, 144px) + 1.2rem);
    padding: 0.45rem 2.4rem 3.6rem;
    border-radius: 32px;
    background-color: rgba(10, 14, 22, 0.38);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);
  }

  .stage-opaque {
    background-color: #05060a;
    box-shadow: none;
  }

  .gauges {
    display: grid;
    grid-template-columns: minmax(0, 0.72fr) minmax(0, 1.4fr) minmax(0, 0.72fr);
    align-items: end;
    gap: 0.4rem;
  }

  .gauge {
    position: relative;
    min-width: 0;
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
  }

  .tick {
    stroke: rgba(255, 255, 255, 0.42);
    stroke-width: 0.7;
    stroke-linecap: round;
  }

  .dial {
    position: relative;
  }

  .gauge-hero .dial {
    transform: translateY(-1.1rem) scale(1.13, 1.02);
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
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 3.1rem;
    margin-top: 0.15rem;
  }

  .ring-row > div {
    position: absolute;
    left: calc(50% + 1.65rem + 5px);
    text-align: left;
  }

  .ring {
    width: 3.1rem;
    height: 3.1rem;
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
    transform: rotate(-90deg);
    transform-origin: 18px 18px;
  }

  .ring-level-down {
    stroke: #3dde7a;
  }

  .ring-row strong {
    display: block;
    font-size: clamp(1.15rem, 2vw, 1.55rem);
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .ring-row span {
    color: rgba(244, 247, 251, 0.62);
    font-size: 0.82rem;
    font-weight: 650;
  }

  .disks {
    display: grid;
    grid-template-columns: repeat(var(--disk-count), minmax(0, 1fr));
    width: min(100%, calc(var(--disk-count) * 18rem));
    margin-inline: auto;
    gap: 1.2rem 1.8rem;
  }

  .disk-label {
    color: rgba(244, 247, 251, 0.62);
    font-size: 0.92rem;
    font-weight: 650;
  }

  .disk-track {
    height: 7px;
    overflow: hidden;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.12);
  }

  .disk-track i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: #d7dee8;
  }

  .disk-label {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    margin-bottom: 0.45rem;
  }

  .empty {
    margin: 0;
    color: rgba(244, 247, 251, 0.62);
  }
</style>
