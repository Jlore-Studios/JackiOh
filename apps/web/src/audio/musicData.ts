// Music data access (SPEC §10.11 "Music", R631).
//
// `music-manifest.json` (the rendered tracks, written by `apps/web/scripts/gen-music.mjs`) and
// `music-cards.json` (which cards play a theme, switch a station or open their play with an intro of
// their own, R1350) are imported here and nowhere else, and checked once at import, so a malformed
// table fails at load rather than playing nothing. A card's entry is looked up by a defId the viewer
// can read: the sentinel is never in the table.

import { MUSIC_ROTATION_KEY, MUSIC_STATIONS } from "./constants.ts";
import rawCards from "./music-cards.json";
import rawManifest from "./music-manifest.json";
import type { MusicCardEntry, MusicManifest, MusicStation, MusicTrack } from "./types.ts";

/** The main menu theme: every screen without a board plays it. */
export const MENU_TRACK = "menu";
/** The result stings, each with its loop for the results screen. */
export const RESULT_TRACKS = { victory: "victory", defeat: "defeat", draw: "draw" } as const;

function fail(file: string, path: string, problem: string): never {
  throw new Error(`${file}: ${path}: ${problem}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function finite(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) fail("music-manifest.json", path, "must be a number ≥ 0");
  return value;
}

function finiteOrNull(value: unknown, path: string): number | null {
  return value === null ? null : finite(value, path);
}

/** Total over the generator's output: every field checked, a loop's span inside its file. */
export function parseMusicManifest(raw: unknown): MusicManifest {
  if (!isRecord(raw) || raw.version !== 1 || !isRecord(raw.files)) fail("music-manifest.json", "$", "must be { version: 1, files }");
  const files: Record<string, MusicTrack> = {};
  for (const [id, entry] of Object.entries(raw.files)) {
    const at = `files.${id}`;
    if (!isRecord(entry)) fail("music-manifest.json", at, "must be an object");
    if (typeof entry.hash !== "string") fail("music-manifest.json", `${at}.hash`, "must be a string");
    if (typeof entry.loop !== "boolean") fail("music-manifest.json", `${at}.loop`, "must be a boolean");
    const track: MusicTrack = {
      hash: entry.hash,
      bytes: finite(entry.bytes, `${at}.bytes`),
      bpm: finite(entry.bpm, `${at}.bpm`),
      beatsPerBar: finite(entry.beatsPerBar, `${at}.beatsPerBar`),
      loop: entry.loop,
      intro: finite(entry.intro, `${at}.intro`),
      duration: finite(entry.duration, `${at}.duration`),
      loopStart: finiteOrNull(entry.loopStart, `${at}.loopStart`),
      loopEnd: finiteOrNull(entry.loopEnd, `${at}.loopEnd`),
      handoff: finiteOrNull(entry.handoff, `${at}.handoff`),
    };
    if (track.bpm <= 0 || track.beatsPerBar <= 0) fail("music-manifest.json", at, "needs a tempo and a metre");
    if (track.loop) {
      if (track.loopStart === null || track.loopEnd === null || track.loopEnd <= track.loopStart || track.loopEnd > track.duration) {
        fail("music-manifest.json", at, "a loop needs loopStart < loopEnd ≤ duration");
      }
    } else if (track.handoff === null || track.handoff > track.duration) {
      fail("music-manifest.json", at, "a sting needs a handoff within its file");
    }
    files[id] = track;
  }
  return { version: 1, format: typeof raw.format === "string" ? raw.format : "", files };
}

/** Every entry names a track the manifest has, and a station that exists. */
export function parseMusicCards(raw: unknown, manifest: MusicManifest): Record<string, MusicCardEntry> {
  if (!isRecord(raw) || raw.version !== 1 || !isRecord(raw.cards)) fail("music-cards.json", "$", "must be { version: 1, cards }");
  const cards: Record<string, MusicCardEntry> = {};
  for (const [defId, entry] of Object.entries(raw.cards)) {
    if (!isRecord(entry)) fail("music-cards.json", defId, "must be an object");
    const out: MusicCardEntry = {};
    if (entry.theme !== undefined) {
      if (typeof entry.theme !== "string" || manifest.files[entry.theme]?.loop !== true) fail("music-cards.json", `${defId}.theme`, "must name a looping track");
      out.theme = entry.theme;
    }
    if (entry.station !== undefined) {
      const station = MUSIC_STATIONS.find((s) => s === entry.station);
      if (station === undefined) fail("music-cards.json", `${defId}.station`, `must be one of ${MUSIC_STATIONS.join(", ")}`);
      out.station = station;
    }
    if (entry.intro !== undefined) {
      // R1352: a card's intro is a sting of its own, which plays once.
      if (typeof entry.intro !== "string" || manifest.files[entry.intro]?.loop !== false) fail("music-cards.json", `${defId}.intro`, "must name a track that plays once");
      out.intro = entry.intro;
    }
    if (out.theme === undefined && out.station === undefined && out.intro === undefined) fail("music-cards.json", defId, "needs a theme, a station or an intro");
    cards[defId] = out;
  }
  return cards;
}

export const MUSIC_MANIFEST: MusicManifest = parseMusicManifest(rawManifest);
export const MUSIC_CARDS: Readonly<Record<string, MusicCardEntry>> = parseMusicCards(rawCards, MUSIC_MANIFEST);

export function musicUrl(id: string): string {
  return `${import.meta.env.BASE_URL}audio/music/${id}.m4a`;
}

/** A station's in-game tracks, `<station>-1`, `<station>-2`, …, in order: the ones a match rotates through. */
export function stationTracks(station: MusicStation, manifest: MusicManifest = MUSIC_MANIFEST): string[] {
  const prefix = `${station}-`;
  return Object.keys(manifest.files)
    .filter((id) => id.startsWith(prefix) && /^\d+$/.test(id.slice(prefix.length)))
    .sort((a, b) => Number(a.slice(prefix.length)) - Number(b.slice(prefix.length)));
}

export function dangerTrack(station: MusicStation): string {
  return `${station}-danger`;
}

export function startTrack(station: MusicStation): string {
  return `${station}-start`;
}

/** A station's own music (in-game or low health), which picks up where it left off within a match. */
export function isStationTrack(id: string): boolean {
  return MUSIC_STATIONS.some((station) => id === dangerTrack(station) || stationTracks(station).includes(id));
}

/** The rotation counter, read inside try/catch: 0 when storage is unavailable. */
export function readRotation(): number {
  try {
    const n = Number(window.localStorage.getItem(MUSIC_ROTATION_KEY));
    return Number.isInteger(n) && n >= 0 ? n : 0;
  } catch {
    return 0;
  }
}

/** Moves the counter on for a new match and returns the value this match uses. */
export function nextRotation(): number {
  const n = readRotation();
  try {
    window.localStorage.setItem(MUSIC_ROTATION_KEY, String(n + 1));
  } catch {
    // Storage is unavailable: every match plays the first track, which is still music.
  }
  return n;
}
