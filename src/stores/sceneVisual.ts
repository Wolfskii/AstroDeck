import { writable } from "svelte/store";
import { DEFAULT_SHADER_COLORS } from "../lib/albumArtColor";

export const sceneShaderColors = writable<string[]>([...DEFAULT_SHADER_COLORS]);
export const sceneShaderImage = writable<string | null>(null);
