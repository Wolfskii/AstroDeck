import { get, writable } from "svelte/store";
import {
  DEFAULT_CLOCK_DIGIT_BORDER_COLOR,
  DEFAULT_CLOCK_DIGIT_BORDER_WIDTH,
  DEFAULT_CLOCK_DIGIT_COLOR,
  DEFAULT_TEMPERATURE_UNIT,
  getClockSettings,
  parseClockDigitBorderColor,
  parseClockDigitBorderWidth,
  parseClockDigitColor,
  setClockSettings,
  type ClockSettings,
  type TemperatureUnit,
} from "../services/prefs";

export const temperatureUnit = writable<TemperatureUnit>(DEFAULT_TEMPERATURE_UNIT);
export const clockLocation = writable("");
export const clockDigitColor = writable(DEFAULT_CLOCK_DIGIT_COLOR);
export const clockDigitBorder = writable(false);
export const clockDigitBorderColor = writable(DEFAULT_CLOCK_DIGIT_BORDER_COLOR);
export const clockDigitBorderWidth = writable(DEFAULT_CLOCK_DIGIT_BORDER_WIDTH);

function applyClockSettings(settings: ClockSettings): void {
  temperatureUnit.set(settings.temperatureUnit);
  clockLocation.set(settings.location);
  clockDigitColor.set(settings.digitColor);
  clockDigitBorder.set(settings.digitBorder);
  clockDigitBorderColor.set(settings.digitBorderColor);
  clockDigitBorderWidth.set(settings.digitBorderWidth);
}

function currentClockSettings(overrides: Partial<ClockSettings> = {}): ClockSettings {
  return {
    temperatureUnit: get(temperatureUnit),
    location: get(clockLocation),
    digitColor: get(clockDigitColor),
    digitBorder: get(clockDigitBorder),
    digitBorderColor: get(clockDigitBorderColor),
    digitBorderWidth: get(clockDigitBorderWidth),
    ...overrides,
  };
}

export async function loadClockSettings(): Promise<void> {
  applyClockSettings(await getClockSettings());
}

export async function saveClockSettings(
  unit: TemperatureUnit,
  location: string
): Promise<void> {
  applyClockSettings(await setClockSettings(currentClockSettings({
    temperatureUnit: unit,
    location,
  })));
}

export function previewClockDigitColor(color: string): void {
  clockDigitColor.set(parseClockDigitColor(color));
}

export async function saveClockDigitColor(color: string): Promise<void> {
  const next = parseClockDigitColor(color);
  clockDigitColor.set(next);
  applyClockSettings(await setClockSettings(currentClockSettings({ digitColor: next })));
}

export function previewClockDigitBorderColor(color: string): void {
  clockDigitBorderColor.set(parseClockDigitBorderColor(color));
}

export async function saveClockDigitBorder(border: {
  enabled?: boolean;
  color?: string;
  width?: number;
}): Promise<void> {
  const enabled = border.enabled ?? get(clockDigitBorder);
  const color = parseClockDigitBorderColor(border.color ?? get(clockDigitBorderColor));
  const width = parseClockDigitBorderWidth(border.width ?? get(clockDigitBorderWidth));
  clockDigitBorder.set(enabled);
  clockDigitBorderColor.set(color);
  clockDigitBorderWidth.set(width);
  applyClockSettings(
    await setClockSettings(
      currentClockSettings({
        digitBorder: enabled,
        digitBorderColor: color,
        digitBorderWidth: width,
      })
    )
  );
}
