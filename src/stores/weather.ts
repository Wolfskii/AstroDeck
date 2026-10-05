import { derived, get, writable } from "svelte/store";
import { clockLocation, temperatureUnit } from "./clock";
import {
  detectLocalPlace,
  fetchWeather,
  searchPlaces,
  type WeatherSnapshot,
} from "../services/weather";

export const weatherSnapshot = writable<WeatherSnapshot | null>(null);
export const weatherStatus = writable("Finding location…");

const weatherQuery = derived(
  [temperatureUnit, clockLocation],
  ([unit, location]) => ({ unit, location: location.trim() })
);

let requestId = 0;

export async function refreshWeatherNow(): Promise<void> {
  const id = ++requestId;
  const { unit, location } = get(weatherQuery);
  try {
    weatherStatus.set(location ? "Looking up location…" : "Finding this computer…");
    const place = location ? (await searchPlaces(location))[0] : await detectLocalPlace();
    if (id !== requestId) return;
    if (!place) {
      weatherSnapshot.set(null);
      weatherStatus.set(
        location
          ? "No matching place. Set another location in Settings."
          : "Location unavailable. Set one in Settings."
      );
      return;
    }
    weatherStatus.set("Loading weather…");
    const next = await fetchWeather(place, unit);
    if (id !== requestId) return;
    weatherSnapshot.set(next);
    weatherStatus.set("");
  } catch {
    if (id !== requestId) return;
    weatherSnapshot.set(null);
    weatherStatus.set("Weather unavailable.");
  }
}

export function startWeatherUpdates(): () => void {
  const stop = weatherQuery.subscribe(() => {
    void refreshWeatherNow();
  });
  const timer = window.setInterval(() => void refreshWeatherNow(), 15 * 60 * 1000);
  return () => {
    stop();
    window.clearInterval(timer);
    requestId += 1;
  };
}
