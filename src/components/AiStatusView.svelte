<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { performanceFrosted } from "../stores/appearance";

  type ClaudeSession = {
    pid: number;
    sessionId: string;
    name: string;
    cwd: string;
    entrypoint: string;
    status: string;
    version: string;
    startedAt: number;
    updatedAt: number;
  };
  type AiSessions = {
    claudeSessions: ClaudeSession[];
    claudeDesktopRunning: boolean;
    vscodeRunning: boolean;
    copilotInstalled: boolean;
    nowMs: number;
  };
  type UsageWindow = {
    label: string;
    percentUsed: number;
    detail: string | null;
    resetsAt: string | null;
  };
  type UsageReport = { plan: string | null; windows: UsageWindow[]; error: string | null };
  type AiUsage = { claude: UsageReport; copilot: UsageReport; copilotTokenSaved: boolean };

  let sessions = $state<AiSessions | null>(null);
  let usage = $state<AiUsage | null>(null);
  let usageBusy = $state(false);
  let now = $state(Date.now());

  const ENTRYPOINTS: Record<string, string> = {
    "claude-vscode": "VS Code",
    cli: "Terminal",
    "claude-desktop": "Claude Desktop",
    desktop: "Claude Desktop",
  };

  function entrypointLabel(entrypoint: string): string {
    return ENTRYPOINTS[entrypoint] ?? (entrypoint || "Claude Code");
  }

  function isWorking(status: string): boolean {
    return ["busy", "working", "running", "active"].includes(status.toLowerCase());
  }

  function statusLabel(status: string): string {
    if (!status) return "Running";
    if (isWorking(status)) return "Working";
    return status.charAt(0).toUpperCase() + status.slice(1).toLowerCase();
  }

  function shortPath(path: string): string {
    const parts = path.split(/[\\/]/).filter(Boolean);
    return parts.length > 2 ? `…/${parts.slice(-2).join("/")}` : path;
  }

  function ago(ms: number): string {
    if (!ms) return "";
    const seconds = Math.max(0, Math.round((now - ms) / 1000));
    if (seconds < 10) return "just now";
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return hours < 24 ? `${hours}h ago` : `${Math.floor(hours / 24)}d ago`;
  }

  function resetText(iso: string | null): string {
    if (!iso) return "";
    const target = new Date(iso).getTime();
    if (Number.isNaN(target)) return "";
    const minutes = Math.max(0, Math.round((target - now) / 60_000));
    if (minutes < 1) return "Resets any moment";
    if (minutes < 60) return `Resets in ${minutes}m`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `Resets in ${hours}h ${minutes % 60}m`;
    const day = new Date(target).toLocaleDateString(undefined, {
      weekday: "short",
      day: "numeric",
      month: "short",
    });
    return `Resets ${day}`;
  }

  function barLevel(percent: number): string {
    if (percent >= 90) return "high";
    if (percent >= 70) return "mid";
    return "low";
  }

  async function pullSessions() {
    try {
      sessions = await invoke<AiSessions>("get_ai_sessions");
    } catch {
      /* keep the last list */
    }
  }

  async function pullUsage(force = false) {
    usageBusy = true;
    try {
      usage = await invoke<AiUsage>("get_ai_usage", { force });
    } catch {
      /* keep the last numbers */
    } finally {
      usageBusy = false;
    }
  }

  onMount(() => {
    void pullSessions();
    void pullUsage();
    const sessionTimer = window.setInterval(pullSessions, 3000);
    const usageTimer = window.setInterval(() => void pullUsage(), 60_000);
    const clock = window.setInterval(() => (now = Date.now()), 15_000);
    return () => {
      window.clearInterval(sessionTimer);
      window.clearInterval(usageTimer);
      window.clearInterval(clock);
    };
  });

  const claudeSessions = $derived(sessions?.claudeSessions ?? []);
  const workingCount = $derived(claudeSessions.filter((s) => isWorking(s.status)).length);
</script>

{#snippet usageBars(report: UsageReport | undefined)}
  {#if !report}
    <p class="note">Loading usage…</p>
  {:else if report.windows.length > 0}
    <ul class="bars">
      {#each report.windows as window (window.label)}
        <li class="bar-row">
          <div class="bar-head">
            <span class="bar-label">{window.label}</span>
            <span class="bar-value">{Math.round(window.percentUsed)}% used</span>
          </div>
          <div
            class="bar"
            role="progressbar"
            aria-label={window.label}
            aria-valuenow={Math.round(window.percentUsed)}
            aria-valuemin="0"
            aria-valuemax="100"
          >
            <div
              class="bar-fill level-{barLevel(window.percentUsed)}"
              style:width="{window.percentUsed}%"
            ></div>
          </div>
          <div class="bar-foot">
            <span>{window.detail ?? ""}</span>
            <span>{resetText(window.resetsAt)}</span>
          </div>
        </li>
      {/each}
    </ul>
  {:else if report.error}
    <p class="note warn">{report.error}</p>
  {:else}
    <p class="note">No usage limits reported.</p>
  {/if}
  {#if report && report.windows.length > 0 && report.error}
    <p class="note warn">{report.error}</p>
  {/if}
{/snippet}

<section class="ai-view">
  <div class="grid">
    <article class="card" class:card-opaque={!$performanceFrosted}>
      <header class="card-head">
        <h2>Claude Code</h2>
        {#if usage?.claude.plan}
          <span class="badge">{usage.claude.plan}</span>
        {/if}
        <span class="spacer"></span>
        <button
          type="button"
          class="refresh"
          disabled={usageBusy}
          onclick={() => void pullUsage(true)}
          aria-label="Refresh usage"
          title="Refresh usage"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" class:spin={usageBusy}>
            <path
              fill="currentColor"
              d="M12 5a7 7 0 0 1 6.3 3.9l-1.9.4a.9.9 0 0 0-.6 1.3.9.9 0 0 0 1 .5l4.1-.9a.9.9 0 0 0 .7-1l-.8-4.2a.9.9 0 0 0-1.7.3l.3 1.6A9 9 0 1 0 21 12a1 1 0 1 0-2 0 7 7 0 1 1-7-7Z"
            />
          </svg>
        </button>
      </header>

      {@render usageBars(usage?.claude)}

      <h3 class="section-title">
        Sessions
        <span class="count">
          {claudeSessions.length}{workingCount > 0 ? ` · ${workingCount} working` : ""}
        </span>
      </h3>
      {#if claudeSessions.length === 0}
        <p class="note">
          {sessions ? "No Claude Code sessions are running." : "Looking for sessions…"}
        </p>
      {:else}
        <ul class="sessions">
          {#each claudeSessions as session (session.pid)}
            <li class="session">
              <span
                class="dot"
                class:dot-working={isWorking(session.status)}
                class:dot-idle={!isWorking(session.status)}
              ></span>
              <div class="session-main">
                <p class="session-name">{session.name}</p>
                <p class="session-meta">{shortPath(session.cwd)}</p>
              </div>
              <div class="session-side">
                <span class="chip">{entrypointLabel(session.entrypoint)}</span>
                <span class="session-meta">
                  {statusLabel(session.status)} · {ago(session.updatedAt)}
                </span>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
      {#if sessions?.claudeDesktopRunning}
        <p class="app-line"><span class="dot dot-idle"></span> Claude Desktop is running</p>
      {/if}
    </article>

    <article class="card" class:card-opaque={!$performanceFrosted}>
      <header class="card-head">
        <h2>GitHub Copilot</h2>
        {#if usage?.copilot.plan}
          <span class="badge">{usage.copilot.plan.replaceAll("_", " ")}</span>
        {/if}
      </header>

      <p class="app-line">
        <span class="dot" class:dot-idle={sessions?.vscodeRunning} class:dot-off={!sessions?.vscodeRunning}
        ></span>
        {#if !sessions}
          Looking for VS Code…
        {:else if !sessions.copilotInstalled}
          Copilot extension not found
        {:else if sessions.vscodeRunning}
          Copilot is available in the running VS Code
        {:else}
          Copilot installed · VS Code is not running
        {/if}
      </p>

      {#if sessions?.copilotInstalled || usage?.copilotTokenSaved || (usage?.copilot.windows.length ?? 0) > 0}
        {@render usageBars(usage?.copilot)}
      {:else}
        <p class="note">Install GitHub Copilot in VS Code to see it here.</p>
      {/if}
    </article>
  </div>
</section>

<style>
  .ai-view {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    width: 100%;
    padding-right: 2.2rem;
    box-sizing: border-box;
    background: transparent;
    color: #f4f7fb;
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
    gap: 1.2rem;
    align-items: start;
    margin: 1.4rem 0 1.4rem 1.2rem;
    min-height: 0;
    max-height: calc(100% - 2.8rem);
  }

  .card {
    display: flex;
    min-width: 0;
    max-height: 100%;
    flex-direction: column;
    gap: 0.9rem;
    padding: 1.3rem 1.6rem 1.5rem;
    border-radius: 28px;
    overflow-y: auto;
    scrollbar-width: none;
    background-color: rgba(10, 14, 22, 0.38);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);
  }

  .card::-webkit-scrollbar {
    display: none;
  }

  .card-opaque {
    background-color: #05060a;
    box-shadow: none;
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }

  h2 {
    margin: 0;
    font-size: 1.35rem;
    font-weight: 700;
  }

  .spacer {
    flex: 1;
  }

  .badge {
    padding: 0.18rem 0.65rem;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.1);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: capitalize;
  }

  .refresh {
    display: grid;
    width: 2.6rem;
    height: 2.6rem;
    place-items: center;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: inherit;
    cursor: pointer;
  }

  .refresh:active:not(:disabled) {
    background: rgba(255, 255, 255, 0.18);
  }

  .refresh:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .refresh svg {
    width: 1.3rem;
    height: 1.3rem;
  }

  .spin {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .bars {
    display: flex;
    flex-direction: column;
    gap: 0.95rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .bar-head,
  .bar-foot {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
  }

  .bar-label {
    font-weight: 650;
  }

  .bar-value {
    font-variant-numeric: tabular-nums;
    font-weight: 650;
  }

  .bar-foot {
    margin-top: 0.3rem;
    min-height: 1.1em;
    font-size: 0.82rem;
    color: rgba(244, 247, 251, 0.62);
  }

  .bar {
    height: 0.7rem;
    margin-top: 0.4rem;
    border-radius: 999px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.1);
  }

  .bar-fill {
    height: 100%;
    border-radius: 999px;
    transition: width 0.5s ease, background-color 0.3s ease;
  }

  .level-low {
    background: #4ade80;
  }

  .level-mid {
    background: #fbbf24;
  }

  .level-high {
    background: #f87171;
  }

  .section-title {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    margin: 0.5rem 0 0;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(244, 247, 251, 0.62);
  }

  .count {
    letter-spacing: 0;
    text-transform: none;
    font-weight: 600;
  }

  .sessions {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .session {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    padding: 0.7rem 0.9rem;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.06);
  }

  .session-main {
    min-width: 0;
    flex: 1;
  }

  .session-name {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 650;
  }

  .session-meta {
    margin: 0.1rem 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    color: rgba(244, 247, 251, 0.62);
  }

  .session-side {
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.2rem;
  }

  .chip {
    padding: 0.1rem 0.55rem;
    border-radius: 999px;
    background: rgba(217, 119, 87, 0.22);
    color: #f3b9a4;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .dot {
    flex: 0 0 auto;
    width: 0.65rem;
    height: 0.65rem;
    border-radius: 50%;
    background: #6b7280;
  }

  .dot-idle {
    background: #4ade80;
  }

  .dot-off {
    background: #6b7280;
  }

  .dot-working {
    background: #fbbf24;
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  .app-line {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0;
    font-size: 0.92rem;
    color: rgba(244, 247, 251, 0.8);
  }

  .note {
    margin: 0;
    font-size: 0.92rem;
    color: rgba(244, 247, 251, 0.62);
  }

  .warn {
    color: #fcd34d;
  }

  @media (max-width: 1100px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
      overflow-y: auto;
    }
  }
</style>
