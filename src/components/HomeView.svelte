<script lang="ts">
  import WeatherIcon from "./WeatherIcon.svelte";
  import {
    clockDigitBorder,
    clockDigitBorderColor,
    clockDigitBorderWidth,
    clockDigitColor,
    clockWeatherCard,
  } from "../stores/clock";
  import { weatherDetailOpen } from "../stores/navigation";
  import { weatherSnapshot, weatherStatus } from "../stores/weather";
  import { audioVisualizerEnabled, sceneBackgroundId } from "../stores/appearance";
  import {
    acquireOsAudioFrames,
    getOsAudioFrame,
    releaseOsAudioFrames,
    sampleBand,
    type OsAudioFrame,
  } from "../lib/osAudioViz";
  import { usesFullViewBackground } from "../lib/sceneBackgrounds";
  import { weatherIconName } from "../services/weather";
  import { executeAction } from "../services/api";

  let {
    nowPlaying = null,
    onOpenPlaying,
    weatherEnabled = true,
  }: {
    nowPlaying?: {
      sceneId: "spotify" | "youtubeMusic" | "media";
      title: string;
      artist: string;
      playing: boolean;
    } | null;
    onOpenPlaying?: (sceneId: "spotify" | "youtubeMusic" | "media") => void;
    weatherEnabled?: boolean;
  } = $props();

  const nowPlayingLabel = $derived(
    nowPlaying
      ? [nowPlaying.title, nowPlaying.artist].filter(Boolean).join(" - ")
      : ""
  );

  let now = $state(new Date());
  const showShader = $derived(usesFullViewBackground($sceneBackgroundId));

  const parts = $derived(clockParts(now));
  const homeDate = $derived(formatHomeDate(now));
  let timeEl = $state<HTMLParagraphElement | null>(null);
  let dateEdge = $state(0);
  const WAVE_SEGMENTS = 56;
  // Smoothed loudness per point, so the line rises quickly on a hit and settles back gently.
  const waveLevels = new Array<number>(WAVE_SEGMENTS + 1).fill(0.84);

  /**
   * One frame of the line. With speaker analysis the height at each point follows the song's
   * frequency bands (bass in the middle, highs toward the edges); without it the line keeps a
   * steady, gentle movement.
   */
  function renderWave(phase: number, audio: OsAudioFrame | null): string {
    const points: string[] = [];
    for (let index = 0; index <= WAVE_SEGMENTS; index += 1) {
      const x = (index / WAVE_SEGMENTS) * 100;
      const fromCenter = Math.abs(index - WAVE_SEGMENTS / 2) / (WAVE_SEGMENTS / 2);
      let target = 0.84;
      if (audio) {
        const band = sampleBand(audio.bands, fromCenter, 2) ?? 0;
        target = Math.min(1, band * 1.1 + audio.beat * 0.25);
      }
      const level = waveLevels[index];
      waveLevels[index] = level + (target - level) * (target > level ? 0.5 : 0.16);
      const scale = 0.2 + waveLevels[index] * 0.95;
      const t = phase + index * 0.62;
      const wave =
        Math.sin(t) * 6.4 + Math.sin(t * 2.15 + 0.8) * 3.1 + Math.sin(t * 0.45) * 1.4;
      const y = Math.min(23, Math.max(1, 12 + wave * scale));
      points.push(`${x.toFixed(2)},${y.toFixed(2)}`);
    }
    return points.join(" ");
  }

  let wavePoints = $state(renderWave(0, null));
  let wavePhase = 0;

  $effect(() => {
    const timer = window.setInterval(() => {
      now = new Date();
    }, 1000);
    return () => window.clearInterval(timer);
  });

  $effect(() => {
    const el = timeEl;
    parts.hours;
    parts.minutes;
    parts.meridian;
    if (!el) return;
    const measure = () => {
      const box = el.getBoundingClientRect();
      let right = box.right;
      el.querySelectorAll(".digit, .meridian").forEach((node) => {
        right = Math.max(right, node.getBoundingClientRect().right);
      });
      const next = Math.round(box.right - right);
      if (next !== dateEdge) dateEdge = next;
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  });

  $effect(() => {
    if (!nowPlaying?.playing) return;
    // Speaker analysis only runs while "React to system audio" is on.
    let acquired = false;
    const stopWatching = audioVisualizerEnabled.subscribe((enabled) => {
      if (enabled && !acquired) {
        acquireOsAudioFrames();
        acquired = true;
      } else if (!enabled && acquired) {
        releaseOsAudioFrames();
        acquired = false;
      }
    });
    let frame = 0;
    const tick = () => {
      wavePhase += 0.11;
      wavePoints = renderWave(wavePhase, acquired ? getOsAudioFrame() : null);
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => {
      window.cancelAnimationFrame(frame);
      stopWatching();
      if (acquired) releaseOsAudioFrames();
    };
  });

  function clockParts(date: Date): { hours: string; minutes: string; meridian: string } {
    const cycle = new Intl.DateTimeFormat(undefined, { hour: "numeric" }).resolvedOptions()
      .hourCycle;
    const minutes = String(date.getMinutes()).padStart(2, "0");
    if (cycle === "h11" || cycle === "h12") {
      const hour = date.getHours() % 12 || 12;
      return {
        hours: String(hour),
        minutes,
        meridian: date.getHours() < 12 ? "AM" : "PM",
      };
    }
    return {
      hours: String(date.getHours()),
      minutes,
      meridian: "",
    };
  }

  function formatHomeDate(date: Date): string {
    const weekday = new Intl.DateTimeFormat("en-GB", { weekday: "long" }).format(date);
    const month = new Intl.DateTimeFormat("en-GB", { month: "long" }).format(date);
    return `${weekday}, ${ordinalDay(date.getDate())} of ${month}`;
  }

  function ordinalDay(day: number): string {
    const mod100 = day % 100;
    if (mod100 >= 11 && mod100 <= 13) return `${day}th`;
    switch (day % 10) {
      case 1:
        return `${day}st`;
      case 2:
        return `${day}nd`;
      case 3:
        return `${day}rd`;
      default:
        return `${day}th`;
    }
  }

  function degrees(value: number): string {
    return `${Math.round(value)}°`;
  }

  async function runTransport(kind: "prev" | "toggle" | "next") {
    const scene = nowPlaying?.sceneId;
    if (!scene) return;
    const prefix = scene === "media" ? "media" : scene;
    const action =
      kind === "prev"
        ? `${prefix}.prevTrack`
        : kind === "next"
          ? `${prefix}.nextTrack`
          : `${prefix}.togglePlay`;
    const label = kind === "prev" ? "Previous" : kind === "next" ? "Next" : "Play/Pause";
    try {
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-started", { detail: { action, label } })
      );
      await executeAction(action);
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-executed", { detail: { action, label } })
      );
    } catch (e) {
      window.dispatchEvent(
        new CustomEvent("astrodeck-action-failed", {
          detail: { action, label, error: String(e) },
        })
      );
    }
  }
</script>

<section class="clock-view" class:clock-view--shader={showShader}>
  <div class="clock-pane">
    <div class="clock-block">
      <p
        class="time"
        bind:this={timeEl}
        style={`color: ${$clockDigitColor}; --digit-border: ${$clockDigitBorder ? $clockDigitBorderWidth : 0}px; --digit-border-color: ${$clockDigitBorderColor};`}
        aria-label={`${homeDate}. ${parts.hours}:${parts.minutes}${parts.meridian ? ` ${parts.meridian}` : ""}`}
      >
        <span class="home-date" style={`right: ${dateEdge}px`}>{homeDate}</span>
        <span class="digit">{parts.hours}</span>
        <span class="colon" aria-hidden="true"><i></i><i></i></span>
        <span class="digit">{parts.minutes}</span>
        {#if parts.meridian}
          <span class="meridian">{parts.meridian}</span>
        {/if}
        </p>
      {#if nowPlaying}
        <div class="playback-under">
          <svg
            class="sound-wave"
            viewBox="0 0 100 24"
            preserveAspectRatio="none"
            style={`color: ${$clockDigitColor}`}
            aria-hidden="true"
          >
            <polyline
              points={wavePoints}
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linejoin="miter"
              stroke-linecap="butt"
              vector-effect="non-scaling-stroke"
            />
          </svg>
          {#if nowPlaying && nowPlayingLabel}
            <div class="now-playing-row">
              <button
                type="button"
                class="now-playing"
                style={`color: ${$clockDigitColor}`}
                aria-label={`Open ${nowPlayingLabel}`}
                onclick={() => onOpenPlaying?.(nowPlaying.sceneId)}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  {#if nowPlaying.sceneId === "spotify"}
                    <path
                      fill="#1db954"
                      d="M12 2a10 10 0 1 0 .01 20.01A10 10 0 0 0 12 2Zm4.55 14.46a.63.63 0 0 1-.86.21c-2.36-1.44-5.34-1.77-8.84-.97a.63.63 0 0 1-.28-1.23c3.82-.87 7.1-.5 9.77 1.13a.63.63 0 0 1 .21.86Zm1.2-2.68a.78.78 0 0 1-1.07.26c-2.7-1.66-6.82-2.14-10.02-1.17a.78.78 0 1 1-.45-1.5c3.67-1.11 8.23-.57 11.28 1.34a.78.78 0 0 1 .26 1.07Zm.1-2.79a.93.93 0 0 1-1.28.31c-3.09-1.9-8.2-2.07-11.15-1.14a.93.93 0 1 1-.54-1.78c3.4-1.04 9.06-.84 12.66 1.32a.93.93 0 0 1 .31 1.29Z"
                    />
                  {:else if nowPlaying.sceneId === "youtubeMusic"}
                    <path
                      fill="#ff0033"
                      d="M23 12.2s0-3.15-.4-4.55c-.22-.9-.9-1.58-1.8-1.8C19.18 5.46 12 5.46 12 5.46s-7.18 0-8.8.39c-.9.22-1.58.9-1.8 1.8C1 9.05 1 12.2 1 12.2s0 3.15.4 4.55c.22.9.9 1.58 1.8 1.8 1.62.39 8.8.39 8.8.39s7.18 0 8.8-.39c.9-.22 1.58-.9 1.8-1.8.4-1.4.4-4.55.4-4.55ZM9.75 15.57V8.83l6.27 3.37-6.27 3.37Z"
                    />
                  {:else}
                    <path
                      fill="currentColor"
                      d="M12 2.2a9.8 9.8 0 1 0 .01 19.61A9.8 9.8 0 0 0 12 2.2Zm-1.7 13.7V8.1L17.2 12l-6.9 3.9Z"
                    />
                  {/if}
                </svg>
                <span>{nowPlayingLabel}</span>
              </button>
              <div class="now-playing-transport" style={`color: ${$clockDigitColor}`}>
                <button type="button" aria-label="Previous" onclick={() => runTransport("prev")}>
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path fill="currentColor" d="M6 6h2v12H6V6zm3.5 6 8.5 6V6l-8.5 6z" />
                  </svg>
                </button>
                <button
                  type="button"
                  aria-label={nowPlaying.playing ? "Pause" : "Play"}
                  onclick={() => runTransport("toggle")}
                >
                  {#if nowPlaying.playing}
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path fill="currentColor" d="M6 5h4v14H6V5zm8 0h4v14h-4V5z" />
                    </svg>
                  {:else}
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path fill="currentColor" d="M8 5v14l11-7L8 5z" />
                    </svg>
                  {/if}
                </button>
                <button type="button" aria-label="Next" onclick={() => runTransport("next")}>
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path fill="currentColor" d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
                  </svg>
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="weather-pane">
    {#if $weatherSnapshot}
      {@const snapshot = $weatherSnapshot}
      <div
        class="weather"
        class:weather-card={$clockWeatherCard}
        class:weather-open={weatherEnabled}
        style={$clockWeatherCard
          ? "background-color: rgba(10, 14, 22, 0.38); border: none; border-radius: 32px; padding: 5.4rem 8.2rem 5.6rem 5.4rem; box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);"
          : undefined}
        role="button"
        aria-disabled={!weatherEnabled}
        tabindex="0"
        onclick={() => {
          if (weatherEnabled) weatherDetailOpen.set(true);
        }}
        onkeydown={(event) => {
          if (!weatherEnabled) return;
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            weatherDetailOpen.set(true);
          }
        }}
      >
        <p class="condition">
          <WeatherIcon name={weatherIconName(snapshot.code, snapshot.isDay)} />
          <span>{snapshot.label}</span>
        </p>
        <p class="temp">{degrees(snapshot.temperature)}</p>
        <p class="place">{snapshot.city}</p>
      </div>
    {:else}
      <p class="status">{$weatherStatus}</p>
    {/if}
  </div>
</section>

<style>
  .clock-view {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1.55fr) minmax(0, 1fr);
    column-gap: 8vw;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: #000;
    color: #f5f5f7;
  }

  .clock-view--shader {
    background: transparent;
  }

  .clock-pane,
  .weather-pane {
    display: flex;
    align-items: center;
    min-width: 0;
    min-height: 0;
  }

  .clock-pane {
    justify-content: center;
    padding: 0 4vw 0 3vw;
  }

  .clock-block {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    width: max-content;
    max-width: 100%;
  }

  .playback-under {
    position: absolute;
    top: 100%;
    left: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 109%;
    margin-top: 4.2rem;
    gap: 0.85rem;
  }

  .sound-wave {
    display: block;
    width: 100%;
    height: 2.1rem;
    overflow: visible;
  }

  .now-playing-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    width: 100%;
    min-width: 0;
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex: 1 1 auto;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font-size: clamp(1.05rem, 1.7vw, 1.35rem);
    font-weight: 650;
    line-height: 1.2;
  }

  .now-playing svg {
    width: 2.15rem;
    height: 2.15rem;
    flex-shrink: 0;
  }

  .now-playing span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .now-playing:hover span {
    text-decoration: underline;
  }

  .now-playing-transport {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    gap: 0.35rem;
  }

  .now-playing-transport button {
    display: grid;
    place-items: center;
    width: 3.4rem;
    height: 3.4rem;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .now-playing-transport button:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .now-playing-transport svg {
    width: 2.15rem;
    height: 2.15rem;
  }

  .time {
    position: relative;
    display: flex;
    align-items: center;
    margin: 0;
    color: #f3d37a;
    font-family: "Arial Narrow", "Segoe UI", sans-serif;
    font-size: min(60vh, 17vw);
    font-weight: 800;
    -webkit-text-stroke: var(--digit-border, 0) var(--digit-border-color, transparent);
    paint-order: stroke fill;
    letter-spacing: -0.01em;
    line-height: 0.8;
    font-variant-numeric: tabular-nums;
  }

  .home-date {
    position: absolute;
    right: 0;
    bottom: calc(100% + min(60vh, 17vw) * 0.2);
    margin: 0;
    font-family: "Segoe UI", sans-serif;
    font-size: clamp(1.2rem, 2.15vw, 1.8rem);
    font-weight: 450;
    letter-spacing: 0.01em;
    line-height: 1.2;
    white-space: nowrap;
    -webkit-text-stroke: 0;
    paint-order: fill;
    pointer-events: none;
  }

  .digit {
    display: block;
    margin: 0 0.12em;
    transform: scaleX(1.46) scaleY(1.32);
    transform-origin: center center;
  }

  .colon {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.18em;
    width: 0.22em;
    height: 0.62em;
    margin: 0 0.12em 0.04em;
  }

  .colon i {
    display: block;
    width: 0.16em;
    height: 0.16em;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 0 var(--digit-border, 0) var(--digit-border-color, transparent);
  }

  .meridian {
    align-self: flex-end;
    margin: 0 0 0.12em 0.12em;
    font-size: 0.16em;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .weather-pane {
    justify-content: flex-start;
    padding: 48px clamp(28px, 5vw, 88px) 48px 4vw;
  }

  .weather {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: default;
  }

  .weather.weather-open {
    cursor: pointer;
  }

  .weather.weather-card {
    border: none;
    border-radius: 32px;
    padding: 5.4rem 8.2rem 5.6rem 5.4rem;
    background-color: rgba(10, 14, 22, 0.38);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);
  }

  .temp,
  .condition,
  .place,
  .status {
    margin: 0;
  }

  .temp {
    font-size: clamp(7.6rem, 18vw, 13.5rem);
    font-weight: 700;
    letter-spacing: -0.045em;
    line-height: 0.9;
  }

  .condition {
    display: flex;
    align-items: center;
    gap: 0.45em;
    margin-bottom: 0;
    font-size: clamp(1.65rem, 3.1vw, 2.55rem);
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1;
  }

  .condition span {
    line-height: 1;
    align-self: center;
  }

  .condition :global(.weather-icon) {
    width: clamp(3.7rem, 7vw, 5.7rem);
    height: clamp(3.7rem, 7vw, 5.7rem);
  }

  .place {
    width: 100%;
    margin-top: 10px;
    color: rgba(245, 245, 247, 0.55);
    font-size: clamp(1.45rem, 2.5vw, 1.9rem);
    font-weight: 600;
    text-align: left;
    align-self: flex-start;
  }

  .status {
    max-width: 18rem;
    color: rgba(245, 245, 247, 0.72);
    font-size: 1.35rem;
    font-weight: 600;
    line-height: 1.35;
  }

  @media (max-width: 800px) {
    .clock-view {
      grid-template-columns: 1fr;
      grid-template-rows: 1.2fr 0.8fr;
    }

    .weather-pane {
      align-items: flex-start;
      padding: 8px 28px 36px;
    }
  }
</style>
