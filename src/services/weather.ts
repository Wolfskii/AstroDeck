import type { TemperatureUnit } from "./prefs";

export type GeoPlace = {
  name: string;
  admin: string;
  country: string;
  latitude: number;
  longitude: number;
};

export type WeatherDay = {
  date: string;
  code: number;
  label: string;
  high: number;
  low: number;
  rainChance: number | null;
};

export type WeatherSnapshot = {
  temperature: number;
  code: number;
  isDay: boolean;
  label: string;
  place: string;
  humidity: number | null;
  rainChance: number | null;
  days: WeatherDay[];
};

export type WeatherScene = "clear" | "partly" | "cloudy" | "fog" | "rain" | "snow" | "thunder";

type GeocodeResult = {
  name?: string;
  admin1?: string;
  country?: string;
  latitude?: number;
  longitude?: number;
};

export function formatPlace(place: GeoPlace): string {
  return [place.name, place.admin, place.country].filter(Boolean).join(", ");
}

export function weatherLabel(code: number): string {
  if (code === 0) return "Clear";
  if (code === 1) return "Mainly Clear";
  if (code === 2) return "Partly Cloudy";
  if (code === 3) return "Overcast";
  if (code === 45 || code === 48) return "Fog";
  if (code >= 51 && code <= 57) return "Drizzle";
  if (code >= 61 && code <= 67) return "Rain";
  if (code >= 71 && code <= 77) return "Snow";
  if (code >= 80 && code <= 82) return "Showers";
  if (code === 85 || code === 86) return "Snow Showers";
  if (code >= 95) return "Thunderstorm";
  return "Weather";
}

export function weatherIconName(code: number, isDay = true): string {
  const night = !isDay;
  if (code === 0) return night ? "clear-night" : "clear-day";
  if (code === 1) return night ? "mostly-clear-night" : "mostly-clear-day";
  if (code === 2) return night ? "partly-cloudy-night" : "partly-cloudy-day";
  if (code === 3) return "overcast";
  if (code === 45 || code === 48) return "fog";
  if (code >= 51 && code <= 57) return "drizzle";
  if (code >= 61 && code <= 67) return "rain";
  if (code >= 71 && code <= 77) return "snow";
  if (code >= 80 && code <= 82) return "overcast-rain";
  if (code === 85 || code === 86) return "overcast-snow";
  if (code >= 95) return "thunderstorms";
  return "cloudy";
}

export function weatherScene(code: number): WeatherScene {
  if (code === 0 || code === 1) return "clear";
  if (code === 2) return "partly";
  if (code === 3) return "cloudy";
  if (code === 45 || code === 48) return "fog";
  if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82)) return "rain";
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) return "snow";
  if (code >= 95) return "thunder";
  return "cloudy";
}

export async function searchPlaces(query: string): Promise<GeoPlace[]> {
  const name = query.trim();
  if (!name) return [];
  const url = new URL("https://geocoding-api.open-meteo.com/v1/search");
  url.searchParams.set("name", name);
  url.searchParams.set("count", "5");
  url.searchParams.set("language", "en");
  url.searchParams.set("format", "json");
  const response = await fetch(url);
  if (!response.ok) throw new Error("Location lookup failed");
  const data = (await response.json()) as { results?: GeocodeResult[] };
  return (data.results ?? [])
    .map((result) => ({
      name: result.name?.trim() ?? "",
      admin: result.admin1?.trim() ?? "",
      country: result.country?.trim() ?? "",
      latitude: Number(result.latitude),
      longitude: Number(result.longitude),
    }))
    .filter(
      (place) =>
        place.name &&
        Number.isFinite(place.latitude) &&
        Number.isFinite(place.longitude)
    );
}

export async function detectLocalPlace(): Promise<GeoPlace | null> {
  try {
    const response = await fetch("https://get.geojs.io/v1/ip/geo.json");
    if (response.ok) {
      const data = (await response.json()) as {
        city?: string;
        region?: string;
        country?: string;
        latitude?: string;
        longitude?: string;
      };
      const latitude = Number(data.latitude);
      const longitude = Number(data.longitude);
      if (Number.isFinite(latitude) && Number.isFinite(longitude)) {
        return {
          name: data.city?.trim() || data.region?.trim() || "This computer",
          admin: data.region?.trim() ?? "",
          country: data.country?.trim() ?? "",
          latitude,
          longitude,
        };
      }
    }
  } catch {
    // Fall through to the browser location prompt.
  }

  if (typeof navigator === "undefined" || !navigator.geolocation) return null;
  return new Promise((resolve) => {
    navigator.geolocation.getCurrentPosition(
      (position) =>
        resolve({
          name: "This computer",
          admin: "",
          country: "",
          latitude: position.coords.latitude,
          longitude: position.coords.longitude,
        }),
      () => resolve(null),
      { enableHighAccuracy: false, timeout: 8000, maximumAge: 30 * 60 * 1000 }
    );
  });
}

export async function fetchWeather(
  place: GeoPlace,
  unit: TemperatureUnit
): Promise<WeatherSnapshot> {
  const url = new URL("https://api.open-meteo.com/v1/forecast");
  url.searchParams.set("latitude", String(place.latitude));
  url.searchParams.set("longitude", String(place.longitude));
  url.searchParams.set("current", "temperature_2m,relative_humidity_2m,weather_code,is_day");
  url.searchParams.set("hourly", "precipitation_probability");
  url.searchParams.set("daily", "weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max");
  url.searchParams.set("temperature_unit", unit);
  url.searchParams.set("timezone", "auto");
  url.searchParams.set("forecast_days", "6");
  const response = await fetch(url);
  if (!response.ok) throw new Error("Weather lookup failed");
  const data = (await response.json()) as {
    current?: {
      time?: string;
      temperature_2m?: number;
      relative_humidity_2m?: number;
      weather_code?: number;
      is_day?: number;
    };
    hourly?: { time?: string[]; precipitation_probability?: Array<number | null> };
    daily?: {
      time?: string[];
      weather_code?: Array<number | null>;
      temperature_2m_max?: Array<number | null>;
      temperature_2m_min?: Array<number | null>;
      precipitation_probability_max?: Array<number | null>;
    };
  };
  const temperature = Number(data.current?.temperature_2m);
  const code = Number(data.current?.weather_code ?? 0);
  if (!Number.isFinite(temperature)) {
    throw new Error("Weather lookup failed");
  }
  const humidity = finiteOrNull(data.current?.relative_humidity_2m);
  const hourKey = data.current?.time?.slice(0, 13) ?? "";
  const hourIndex = data.hourly?.time?.findIndex((time) => time.startsWith(hourKey)) ?? -1;
  const rainChance =
    hourIndex >= 0 ? finiteOrNull(data.hourly?.precipitation_probability?.[hourIndex]) : null;
  const days = (data.daily?.time ?? []).slice(1, 6).map((date, index) => {
    const dayIndex = index + 1;
    const dayCode = Number(data.daily?.weather_code?.[dayIndex] ?? 0);
    return {
      date,
      code: dayCode,
      label: weatherLabel(dayCode),
      high: Number(data.daily?.temperature_2m_max?.[dayIndex]),
      low: Number(data.daily?.temperature_2m_min?.[dayIndex]),
      rainChance: finiteOrNull(data.daily?.precipitation_probability_max?.[dayIndex]),
    };
  }).filter((day) => Number.isFinite(day.high) && Number.isFinite(day.low));
  return {
    temperature,
    code,
    isDay: data.current?.is_day !== 0,
    label: weatherLabel(code),
    place: place.name,
    humidity,
    rainChance,
    days,
  };
}

function finiteOrNull(value: number | null | undefined): number | null {
  const next = Number(value);
  return Number.isFinite(next) ? next : null;
}
