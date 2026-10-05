export type CustomView = {
  id: string;
  name: string;
  url: string;
  icon: string;
  color: string;
};

export const VIEW_ACCENT_COLORS = [
  "#38bdf8",
  "#f3d37a",
  "#1db954",
  "#ff0033",
  "#f59e0b",
  "#7b83eb",
  "#3794ff",
  "#f472b6",
  "#a78bfa",
  "#34d399",
  "#fb7185",
  "#e8e8e8",
];

export function newCustomViewId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  const suffix = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `view-${suffix}`;
}

export function normalizeEmbedUrl(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > 2000) return null;
  const withScheme = /^[a-z][a-z0-9+.-]*:/i.test(trimmed) ? trimmed : `https://${trimmed}`;
  let url: URL;
  try {
    url = new URL(withScheme);
  } catch {
    return null;
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") return null;
  if (url.username || url.password) return null;
  if (!url.hostname) return null;
  return url.toString();
}
