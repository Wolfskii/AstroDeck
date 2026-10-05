<script lang="ts">
  import clearDay from "@meteocons/svg-static/monochrome/clear-day.svg?raw";
  import clearNight from "@meteocons/svg-static/monochrome/clear-night.svg?raw";
  import cloudy from "@meteocons/svg-static/monochrome/cloudy.svg?raw";
  import drizzle from "@meteocons/svg-static/monochrome/drizzle.svg?raw";
  import fog from "@meteocons/svg-static/monochrome/fog.svg?raw";
  import mostlyClearDay from "@meteocons/svg-static/monochrome/mostly-clear-day.svg?raw";
  import mostlyClearNight from "@meteocons/svg-static/monochrome/mostly-clear-night.svg?raw";
  import overcast from "@meteocons/svg-static/monochrome/overcast.svg?raw";
  import overcastRain from "@meteocons/svg-static/monochrome/overcast-rain.svg?raw";
  import overcastSnow from "@meteocons/svg-static/monochrome/overcast-snow.svg?raw";
  import partlyCloudyDay from "@meteocons/svg-static/monochrome/partly-cloudy-day.svg?raw";
  import partlyCloudyNight from "@meteocons/svg-static/monochrome/partly-cloudy-night.svg?raw";
  import rain from "@meteocons/svg-static/monochrome/rain.svg?raw";
  import snow from "@meteocons/svg-static/monochrome/snow.svg?raw";
  import thunderstorms from "@meteocons/svg-static/monochrome/thunderstorms.svg?raw";

  const icons: Record<string, string> = {
    "clear-day": clearDay,
    "clear-night": clearNight,
    cloudy,
    drizzle,
    fog,
    "mostly-clear-day": mostlyClearDay,
    "mostly-clear-night": mostlyClearNight,
    overcast,
    "overcast-rain": overcastRain,
    "overcast-snow": overcastSnow,
    "partly-cloudy-day": partlyCloudyDay,
    "partly-cloudy-night": partlyCloudyNight,
    rain,
    snow,
    thunderstorms,
  };

  let { name }: { name: string } = $props();
  const svg = $derived(icons[name] ?? cloudy);
</script>

<span class="weather-icon" aria-hidden="true">{@html svg}</span>

<style>
  .weather-icon {
    display: block;
    flex: 0 0 auto;
    width: 1.15em;
    height: 1.15em;
    color: inherit;
  }

  .weather-icon :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
