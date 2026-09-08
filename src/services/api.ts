import { invoke } from "@tauri-apps/api/core";
import type { SceneState, PluginConfig } from "../types";
import { logInfo } from "./logger";

const isTauri =
  typeof window !== "undefined" &&
  !!(window as any).__TAURI_INTERNALS__;

export async function getActiveScene(): Promise<SceneState> {
  return invoke<SceneState>("get_active_scene");
}

export async function executeAction(action: string): Promise<void> {
  if (!isTauri) {
    // Browser-only mode (http://localhost:1420): don't call Tauri,
    // just log that this would have executed.
    logInfo(`(browser) would execute action: ${action}`, "Browser actions");
    return;
  }

  return invoke("execute_action", { action });
}

export async function executeActionValue(action: string, value: unknown): Promise<void> {
  if (!isTauri) {
    logInfo(`(browser) would execute action value: ${action} => ${String(value)}`, "Browser actions");
    return;
  }

  return invoke("execute_action_value", { action, value });
}

export async function getPlugins(): Promise<PluginConfig[]> {
  return invoke<PluginConfig[]>("get_plugins");
}

export async function setActiveScene(sceneId: string): Promise<void> {
  if (!isTauri) {
    logInfo(`(browser) would set active scene to: ${sceneId}`, "Browser scenes");
    return;
  }

  return invoke("set_active_scene", { sceneId });
}

export interface OsNowPlaying {
  kind?: string;
  source: string;
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  coverArtUrl?: string | null;
  isPlaying: boolean;
  progressMs?: number | null;
  durationMs?: number | null;
  active?: boolean;
}

export async function getOsNowPlaying(): Promise<OsNowPlaying | null> {
  return invoke<OsNowPlaying | null>("get_os_now_playing");
}

export async function getOutputVolume(): Promise<number> {
  return invoke<number>("get_output_volume");
}

export interface TeamsSetupStatus {
  supported: boolean;
  configured: boolean;
  path?: string | null;
  message: string;
}

export async function getTeamsSetupStatus(): Promise<TeamsSetupStatus> {
  return invoke<TeamsSetupStatus>("get_teams_setup_status");
}

export async function setupTeamsIntegration(): Promise<TeamsSetupStatus> {
  return invoke<TeamsSetupStatus>("setup_teams_integration");
}

export interface TeamsStatus {
  isConnected: boolean;
  isInMeeting: boolean;
  isMuted: boolean;
  isVideoOn: boolean;
  isHandRaised: boolean;
  isSharing: boolean;
  canReact: boolean;
  canToggleChat: boolean;
  canLeave: boolean;
  message: string;
}

export async function getTeamsStatus(): Promise<TeamsStatus> {
  return invoke<TeamsStatus>("get_teams_status");
}

export interface SpotifyTrackPreview {
  itemId: string;
  itemType: string;
  trackName: string;
  artistName?: string | null;
  albumName?: string | null;
  coverArtUrl?: string | null;
  durationMs?: number | null;
}

export interface SpotifyStatus {
  isConfigured: boolean;
  isAuthenticated: boolean;
  hasActiveDevice: boolean;
  activeDeviceName?: string | null;
  currentTrackName?: string | null;
  currentArtistName?: string | null;
  currentCoverArtUrl?: string | null;
  currentAlbumName?: string | null;
  progressMs?: number | null;
  durationMs?: number | null;
  playbackState: string;
  isPlaying: boolean;
  currentVolumePercent?: number | null;
  currentItemType?: string | null;
  currentItemId?: string | null;
  isCurrentTrackSaved?: boolean | null;
  isShuffle?: boolean;
  grantedScopes?: string[];
  nextTrackPreview?: SpotifyTrackPreview | null;
  prevTrackPreview?: SpotifyTrackPreview | null;
  usesWebApi?: boolean;
  message: string;
}

export async function peekSpotifySkipTrack(
  direction: "next" | "prev"
): Promise<SpotifyTrackPreview | null> {
  return invoke<SpotifyTrackPreview | null>("peek_spotify_skip_track", {
    direction,
  });
}

export async function getSpotifyStatus(options?: {
  fresh?: boolean;
}): Promise<SpotifyStatus> {
  return invoke<SpotifyStatus>("get_spotify_status", {
    fresh: options?.fresh ?? false,
  });
}

export type SpotifyAuthMode = "official" | "custom";

export interface SpotifyClientConfig {
  authMode: SpotifyAuthMode;
  clientId: string;
  lockedByEnv: boolean;
}

export async function getSpotifyClientConfig(): Promise<SpotifyClientConfig> {
  return invoke<SpotifyClientConfig>("get_spotify_client_config");
}

export async function setSpotifyClientId(clientId: string): Promise<void> {
  return invoke("set_spotify_client_id", { clientId });
}

export async function setSpotifyAuthMode(authMode: SpotifyAuthMode): Promise<void> {
  return invoke("set_spotify_auth_mode", { authMode });
}

export interface SpotifyPlaylist {
  id: string;
  name: string;
  uri: string;
  imageUrl?: string | null;
  trackCount: number;
  ownerName?: string | null;
}

export interface SpotifyPlaylistPage {
  items: SpotifyPlaylist[];
  offset: number;
  limit: number;
  total: number;
  nextOffset?: number | null;
}

export async function listSpotifyPlaylists(options?: {
  offset?: number;
  limit?: number;
}): Promise<SpotifyPlaylistPage> {
  return invoke<SpotifyPlaylistPage>("list_spotify_playlists", {
    offset: options?.offset ?? 0,
    limit: options?.limit ?? 50,
  });
}

export async function playSpotifyPlaylist(playlist: string): Promise<void> {
  return invoke("play_spotify_playlist", { playlist });
}

export interface SpotifyLyricLine {
  startTimeMs: number;
  words: string;
}

export interface SpotifyTrackLyrics {
  trackId: string;
  syncType: string;
  available: boolean;
  lines: SpotifyLyricLine[];
}

export type LyricsProviderId = "lrclib" | "musixmatch" | "kugou" | "netease";

export interface LyricsDocument {
  trackId: string;
  provider: string;
  syncType: string;
  available: boolean;
  lines: SpotifyLyricLine[];
}

const DEMO_LYRICS: SpotifyTrackLyrics = {
  trackId: "demo",
  syncType: "LINE_SYNCED",
  available: true,
  lines: [
    { startTimeMs: 0, words: "Lights down, the room leans in" },
    { startTimeMs: 2500, words: "This is the line that's singing now" },
    { startTimeMs: 5200, words: "And this one waits just underneath" },
    { startTimeMs: 8000, words: "Then it rises as the last one fades" },
    { startTimeMs: 10800, words: "White in the middle, grey above" },
  ],
};

export async function getSpotifyLyrics(trackId: string): Promise<SpotifyTrackLyrics> {
  if (!isTauri) {
    return { ...DEMO_LYRICS, trackId };
  }
  return invoke<SpotifyTrackLyrics>("get_spotify_lyrics", { trackId });
}

export async function getLyrics(options: {
  trackId: string;
  title: string;
  artist: string;
  album?: string | null;
  durationMs?: number | null;
  providerOrder: LyricsProviderId[];
}): Promise<LyricsDocument> {
  if (!isTauri) {
    const demo = await getSpotifyLyrics(options.trackId);
    return { ...demo, provider: "demo" };
  }
  return invoke<LyricsDocument>("get_lyrics", {
    trackId: options.trackId,
    title: options.title,
    artist: options.artist,
    album: options.album ?? null,
    durationMs: options.durationMs ?? null,
    providerOrder: options.providerOrder,
  });
}

export interface YouTubeSearchTrack {
  videoId: string;
  title: string;
  artistName: string;
  albumName?: string | null;
  coverArtUrl?: string | null;
  durationMs?: number | null;
}

export interface YouTubeMusicStatus {
  provider: "youtubeMusic";
  isConfigured: boolean;
  isAuthenticated: boolean;
  hasActiveDevice: boolean;
  currentTrackName?: string | null;
  currentArtistName?: string | null;
  currentAlbumName?: string | null;
  currentCoverArtUrl?: string | null;
  currentItemId?: string | null;
  progressMs?: number | null;
  durationMs?: number | null;
  playbackState: string;
  isPlaying: boolean;
  currentVolumePercent: number;
  isCurrentTrackSaved: boolean;
  isShuffle: boolean;
  message: string;
}

export interface YouTubeLocalPlaylist {
  id: string;
  name: string;
  trackIds: string[];
}

export interface YouTubeLocalLibrary {
  profileName: string;
  savedTracks: YouTubeSearchTrack[];
  playlists: YouTubeLocalPlaylist[];
}

export async function getYouTubeMusicLibrary(): Promise<YouTubeLocalLibrary> {
  return invoke<YouTubeLocalLibrary>("get_youtube_music_library");
}

export async function saveYouTubeMusicTrack(
  track: YouTubeSearchTrack
): Promise<YouTubeLocalLibrary> {
  return invoke<YouTubeLocalLibrary>("save_youtube_music_track", { track });
}

export async function createYouTubeMusicPlaylist(name: string): Promise<YouTubeLocalLibrary> {
  return invoke<YouTubeLocalLibrary>("create_youtube_music_playlist", { name });
}

export async function addYouTubeMusicTrackToPlaylist(
  playlistId: string,
  trackId: string
): Promise<YouTubeLocalLibrary> {
  return invoke<YouTubeLocalLibrary>("add_youtube_music_track_to_playlist", {
    playlistId,
    trackId,
  });
}

export async function searchYouTubeMusic(query: string): Promise<YouTubeSearchTrack[]> {
  return invoke<YouTubeSearchTrack[]>("search_youtube_music", { query });
}

export async function discoverYouTubeMusic(
  category: "trending" | "popular" | "playlists" | "chill"
): Promise<YouTubeSearchTrack[]> {
  return invoke<YouTubeSearchTrack[]>("discover_youtube_music", { category });
}

export async function getYouTubeMusicStatus(): Promise<YouTubeMusicStatus> {
  return invoke<YouTubeMusicStatus>("get_youtube_music_status");
}

export async function playYouTubeMusic(track: YouTubeSearchTrack): Promise<void> {
  return invoke("play_youtube_music", { track });
}

export async function toggleYouTubeMusicPlay(): Promise<void> {
  return invoke("toggle_youtube_music_play");
}

export async function setYouTubeMusicVolume(volumePercent: number): Promise<number> {
  return invoke<number>("set_youtube_music_volume", { volumePercent });
}

export async function seekYouTubeMusic(positionMs: number): Promise<void> {
  return invoke("seek_youtube_music", { positionMs });
}

export async function setSpotifyVolume(volumePercent: number): Promise<number> {
  return invoke<number>("set_spotify_volume", { volumePercent });
}
