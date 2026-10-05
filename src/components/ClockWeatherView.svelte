<script lang="ts">
  import ChooseViewButton from "./ChooseViewButton.svelte";
  import WeatherDetail from "./WeatherDetail.svelte";
  import WeatherIcon from "./WeatherIcon.svelte";
  import { clockDigitColor, clockLocation, temperatureUnit } from "../stores/clock";
  import {
    detectLocalPlace,
    fetchWeather,
    searchPlaces,
    weatherIconName,
    type WeatherSnapshot,
  } from "../services/weather";

  let { onOpenSettings }: { onOpenSettings?: () => void } = $props();

  let now = $state(new Date());
  let weather = $state<WeatherSnapshot | null>(null);
  let status = $state("Finding location…");
  let detailOpen = $state(false);

  const parts = $derived(clockParts(now));

  $effect(() => {
    const timer = window.setInterval(() => {
      now = new Date();
    }, 1000);
    return () => window.clearInterval(timer);
  });

  $effect(() => {
    const unit = $temperatureUnit;
    const query = $clockLocation.trim();
    let cancelled = false;

    async function load() {
      try {
        status = query ? "Looking up location…" : "Finding this computer…";
        const place = query ? (await searchPlaces(query))[0] : await detectLocalPlace();
        if (cancelled) return;
        if (!place) {
          weather = null;
          status = query
            ? "No matching place. Set another location in Settings."
            : "Location unavailable. Set one in Settings.";
          return;
        }
        status = "Loading weather…";
        const next = await fetchWeather(place, unit);
        if (cancelled) return;
        weather = next;
        status = "";
      } catch {
        if (!cancelled) {
          weather = null;
          status = "Weather unavailable.";
        }
      }
    }

    void load();
    const refresh = window.setInterval(() => void load(), 15 * 60 * 1000);
    return () => {
      cancelled = true;
      window.clearInterval(refresh);
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

  function degrees(value: number): string {
    return `${Math.round(value)}°`;
  }
</script>

<section class="clock-view">
  {#if onOpenSettings}
    <ChooseViewButton onclick={() => onOpenSettings()} />
  {/if}

  <div class="clock-pane">
    <p
      class="time"
      style={`color: ${$clockDigitColor}`}
      aria-label={`${parts.hours}:${parts.minutes}${parts.meridian ? ` ${parts.meridian}` : ""}`}
    >
      <span class="digit">{parts.hours}</span>
      <span class="colon" aria-hidden="true"><i></i><i></i></span>
      <span class="digit">{parts.minutes}</span>
      {#if parts.meridian}
        <span class="meridian">{parts.meridian}</span>
      {/if}
    </p>
  </div>

  <div class="weather-pane">
    {#if weather}
      <button type="button" class="weather" onclick={() => (detailOpen = true)}>
        <p class="temp">{degrees(weather.temperature)}</p>
        <p class="condition">
          <WeatherIcon name={weatherIconName(weather.code, weather.isDay)} />
          <span>{weather.label}</span>
        </p>
        <p class="place">{weather.place}</p>
      </button>
    {:else}
      <p class="status">{status}</p>
    {/if}
  </div>
  {#if detailOpen && weather}
    <WeatherDetail {weather} onBack={() => (detailOpen = false)} />
  {/if}
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

  .clock-pane,
  .weather-pane {
    display: flex;
    align-items: center;
    min-width: 0;
    min-height: 0;
  }

  .clock-pane {
    justify-content: center;
    padding: 0 2vw 0 6vw;
  }

  .time {
    display: flex;
    align-items: center;
    margin: 0;
    color: #f3d37a;
    font-family: "Arial Narrow", "Segoe UI", sans-serif;
    font-size: min(84vh, 24vw);
    font-weight: 700;
    letter-spacing: -0.01em;
    line-height: 0.8;
    font-variant-numeric: tabular-nums;
  }

  .digit {
    display: block;
    transform: scaleX(1.16) scaleY(1.32);
    transform-origin: center center;
  }

  .colon {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.18em;
    width: 0.22em;
    height: 0.62em;
    margin: 0 0.04em 0.04em;
  }

  .colon i {
    display: block;
    width: 0.16em;
    height: 0.16em;
    border-radius: 50%;
    background: currentColor;
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
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .temp,
  .condition,
  .place,
  .status {
    margin: 0;
  }

  .temp {
    font-size: clamp(7.6rem, 18vw, 13.5rem);
    font-weight: 520;
    letter-spacing: -0.045em;
    line-height: 0.9;
  }

  .condition {
    display: flex;
    align-items: center;
    gap: 0.4em;
    margin-top: 10px;
    font-size: clamp(2rem, 3.8vw, 3.1rem);
    font-weight: 650;
    letter-spacing: -0.02em;
  }

  .place {
    margin-top: 18px;
    color: rgba(245, 245, 247, 0.55);
    font-size: clamp(1.2rem, 2vw, 1.55rem);
    font-weight: 600;
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
