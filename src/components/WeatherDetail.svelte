<script lang="ts">
  import WeatherAtmosphere from "./WeatherAtmosphere.svelte";
  import WeatherIcon from "./WeatherIcon.svelte";
  import { weatherIconName, weatherScene, type WeatherSnapshot } from "../services/weather";

  let {
    weather,
    onBack,
  }: {
    weather: WeatherSnapshot;
    onBack: () => void;
  } = $props();

  function degrees(value: number): string {
    return `${Math.round(value)}°`;
  }

  function dayParts(date: string): { day: string; weekday: string } {
    const [year, month, day] = date.split("-").map(Number);
    const parsed = new Date(year, (month ?? 1) - 1, day ?? 1);
    return {
      day: String(parsed.getDate()),
      weekday: parsed.toLocaleDateString(undefined, { weekday: "short" }).toUpperCase(),
    };
  }
</script>

<div class="detail">
  <WeatherAtmosphere scene={weatherScene(weather.code)} isDay={weather.isDay} />
  <button type="button" class="back" onclick={onBack}>Home</button>
  <div class="forecast">
    <section class="now">
      <div class="now-copy">
        <p class="place">{weather.place}</p>
        <p class="temp">{degrees(weather.temperature)}</p>
        <p class="label">{weather.label}</p>
        {#if weather.rainChance != null}
          <p class="meta">Chance of rain {Math.round(weather.rainChance)}%</p>
        {/if}
        {#if weather.humidity != null}
          <p class="meta">Humidity {Math.round(weather.humidity)}%</p>
        {/if}
      </div>
      <div class="now-icon">
        <WeatherIcon name={weatherIconName(weather.code, weather.isDay)} />
      </div>
    </section>
    {#each weather.days as day (day.date)}
      {@const parts = dayParts(day.date)}
      <section class="day">
        <p class="when">
          <span>{parts.day}</span>
          <span>{parts.weekday}</span>
        </p>
        <div class="day-icon">
          <WeatherIcon name={weatherIconName(day.code, true)} />
        </div>
        <p class="range">{degrees(day.low)}/{degrees(day.high)}</p>
        <p class="day-label">{day.label}</p>
        {#if day.rainChance != null}
          <p class="day-rain">Rain {Math.round(day.rainChance)}%</p>
        {/if}
      </section>
    {/each}
  </div>
</div>

<style>
  .detail {
    position: absolute;
    inset: 0;
    z-index: 1;
    color: #f5f5f7;
  }

  .back {
    position: absolute;
    top: 44px;
    left: 128px;
    z-index: 4;
    padding: 10px 16px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.28);
    color: inherit;
    font-size: 0.95rem;
    font-weight: 700;
    cursor: pointer;
  }

  .back:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .forecast {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: minmax(16rem, 1.45fr) repeat(5, minmax(0, 1fr));
    align-items: center;
    height: 100%;
    padding: 120px 4vw 48px 5vw;
    gap: clamp(12px, 2vw, 28px);
  }

  .now {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: max-content;
    max-width: 100%;
    gap: 1.1rem;
    min-width: 0;
  }

  .now-copy {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .place,
  .temp,
  .label,
  .meta,
  .when,
  .range,
  .day-label,
  .day-rain {
    margin: 0;
  }

  .place {
    color: rgba(245, 245, 247, 0.72);
    font-size: clamp(1rem, 1.6vw, 1.35rem);
    font-weight: 650;
  }

  .temp {
    margin-top: 6px;
    font-size: clamp(5.5rem, 9vw, 8.5rem);
    font-weight: 560;
    letter-spacing: -0.045em;
    line-height: 0.9;
  }

  .label {
    margin-top: 8px;
    font-size: clamp(1.35rem, 2vw, 1.8rem);
    font-weight: 700;
  }

  .meta {
    margin-top: 6px;
    color: rgba(245, 245, 247, 0.78);
    font-size: clamp(1rem, 1.4vw, 1.2rem);
    font-weight: 600;
  }

  .now-icon,
  .day-icon {
    display: grid;
    place-items: center;
  }

  .now-icon {
    width: clamp(6.5rem, 10vw, 9rem);
    height: clamp(6.5rem, 10vw, 9rem);
    filter: drop-shadow(0 18px 28px rgba(0, 0, 0, 0.35));
  }

  .now-icon :global(.weather-icon),
  .day-icon :global(.weather-icon) {
    width: 100%;
    height: 100%;
  }

  .day {
    display: flex;
    min-width: 0;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
  }

  .when {
    display: flex;
    flex-direction: column;
    font-size: clamp(0.95rem, 1.3vw, 1.15rem);
    font-weight: 750;
    letter-spacing: 0.04em;
  }

  .day-icon {
    width: clamp(3.2rem, 4.6vw, 4.2rem);
    height: clamp(3.2rem, 4.6vw, 4.2rem);
  }

  .range {
    font-size: clamp(1.15rem, 1.7vw, 1.45rem);
    font-weight: 700;
  }

  .day-label,
  .day-rain {
    color: rgba(245, 245, 247, 0.78);
    font-size: clamp(0.85rem, 1.15vw, 1rem);
    font-weight: 600;
  }

  @media (max-width: 980px) {
    .forecast {
      grid-template-columns: 1fr;
      align-content: center;
      overflow: auto;
    }

    .day {
      flex-direction: row;
      justify-content: space-between;
      text-align: left;
    }
  }
</style>
