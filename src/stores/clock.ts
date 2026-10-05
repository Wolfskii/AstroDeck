import { get, writable } from "svelte/store";
import {
  DEFAULT_CLOCK_DIGIT_COLOR,
  DEFAULT_TEMPERATURE_UNIT,
  getClockSettings,
  parseClockDigitColor,
  setClockSettings,
  type TemperatureUnit,
} from "../services/prefs";

export const temperatureUnit = writable<TemperatureUnit>(DEFAULT_TEMPERATURE_UNIT);
export const clockLocation = writable("");
export const clockDigitColor = writable(DEFAULT_CLOCK_DIGIT_COLOR);

export async function loadClockSettings(): Promise<void> {
  const settings = await getClockSettings();
  temperatureUnit.set(settings.temperatureUnit);
  clockLocation.set(settings.location);
  clockDigitColor.set(settings.digitColor);
}

export async function saveClockSettings(
  unit: TemperatureUnit,
  location: string
): Promise<void> {
  const settings = await setClockSettings(unit, location, get(clockDigitColor));
  temperatureUnit.set(settings.temperatureUnit);
  clockLocation.set(settings.location);
  clockDigitColor.set(settings.digitColor);
}

export function previewClockDigitColor(color: string): void {
  clockDigitColor.set(parseClockDigitColor(color));
}

export async function saveClockDigitColor(color: string): Promise<void> {
  const next = parseClockDigitColor(color);
  clockDigitColor.set(next);
  const settings = await setClockSettings(get(temperatureUnit), get(clockLocation), next);
  temperatureUnit.set(settings.temperatureUnit);
  clockLocation.set(settings.location);
  clockDigitColor.set(settings.digitColor);
}
