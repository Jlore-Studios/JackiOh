// The audio settings store (docs/polish/2-sound.md, Surface; B12, B13): one cached `AudioSettings`,
// persisted as JSON under `localStorage["jackioh.audio.v1"]`. Every storage access sits inside try/catch:
// private windows, blocked site data and quota errors must never break the game. The cached object is
// frozen and replaced on each change, so `useSyncExternalStore` compares snapshots by identity. A
// "storage" event from another tab re-reads the key, so two open games share one mute switch.

import { useSyncExternalStore } from "react";

import { isRecord } from "../wire/json.ts";
import { AUDIO_SETTINGS_KEY, MUSIC_STATIONS } from "./constants.ts";
import type { AudioSettings, MusicStation } from "./types.ts";
import { getStored, setStored } from "../net/storage.ts";

export const DEFAULT_AUDIO_SETTINGS: Readonly<AudioSettings> = Object.freeze({
  master: 0.8,
  sfx: 0.8,
  crowd: 0.5,
  voice: 1,
  muted: false,
  voiceOn: true,
  music: 0.5,
  station: "tavern",
  dynamicMusic: true,
  duckMusic: true,
  playMusicInBackground: false,
});

function level(value: unknown, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback;
  return Math.min(1, Math.max(0, value));
}

function flag(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function station(value: unknown, fallback: MusicStation): MusicStation {
  return MUSIC_STATIONS.find((s) => s === value) ?? fallback;
}

/**
 * A saved `pauseMusicOnBlur` (this switch the other way round, default on) is read inverted, so a player
 * who turned it off keeps their music playing and nobody else hears a change.
 */
function legacyBackground(o: Record<string, unknown>): boolean {
  return typeof o.pauseMusicOnBlur === "boolean" ? !o.pauseMusicOnBlur : DEFAULT_AUDIO_SETTINGS.playMusicInBackground;
}

/** Total: any input → valid settings, each field independently: a finite number is clamped to [0,1];
 *  a value not of its field's kind (a station not in MUSIC_STATIONS) takes the default. */
export function parseAudioSettings(raw: unknown): AudioSettings {
  const o: Record<string, unknown> = isRecord(raw) ? raw : {};
  return {
    master: level(o.master, DEFAULT_AUDIO_SETTINGS.master),
    sfx: level(o.sfx, DEFAULT_AUDIO_SETTINGS.sfx),
    crowd: level(o.crowd, DEFAULT_AUDIO_SETTINGS.crowd),
    voice: level(o.voice, DEFAULT_AUDIO_SETTINGS.voice),
    muted: flag(o.muted, DEFAULT_AUDIO_SETTINGS.muted),
    voiceOn: flag(o.voiceOn, DEFAULT_AUDIO_SETTINGS.voiceOn),
    music: level(o.music, DEFAULT_AUDIO_SETTINGS.music),
    station: station(o.station, DEFAULT_AUDIO_SETTINGS.station),
    dynamicMusic: flag(o.dynamicMusic, DEFAULT_AUDIO_SETTINGS.dynamicMusic),
    duckMusic: flag(o.duckMusic, DEFAULT_AUDIO_SETTINGS.duckMusic),
    playMusicInBackground: flag(o.playMusicInBackground, legacyBackground(o)),
  };
}

let cache: Readonly<AudioSettings> | null = null;

type Subscription = { fn: (s: AudioSettings) => void };
const subscriptions = new Set<Subscription>();
let storageHandler: ((event: Event) => void) | null = null;

function parseStored(text: string): Readonly<AudioSettings> {
  try {
    return Object.freeze(parseAudioSettings(JSON.parse(text)));
  } catch {
    return DEFAULT_AUDIO_SETTINGS;
  }
}

function loadFromStorage(): Readonly<AudioSettings> {
  const text = getStored(AUDIO_SETTINGS_KEY);
  return text === null ? DEFAULT_AUDIO_SETTINGS : parseStored(text);
}

function notify(next: AudioSettings): void {
  for (const sub of [...subscriptions]) {
    try {
      sub.fn(next);
    } catch {
      // A throwing subscriber must not stop the others from hearing the change.
    }
  }
}

/** Cached after the first read. */
export function readAudioSettings(): AudioSettings {
  if (cache === null) cache = loadFromStorage();
  return cache;
}

/** Merges, parses (clamps), caches, persists, and notifies each subscriber once. */
export function writeAudioSettings(patch: Partial<AudioSettings>): AudioSettings {
  const merged: Record<string, unknown> = { ...readAudioSettings() };
  for (const [key, value] of Object.entries(patch)) {
    if (value !== undefined) merged[key] = value;
  }
  const next = Object.freeze(parseAudioSettings(merged));
  cache = next;
  // Storage that refuses it (unavailable or full): the change still holds for this page.
  setStored(AUDIO_SETTINGS_KEY, JSON.stringify(next));
  notify(next);
  return next;
}

function installStorageListener(): void {
  if (storageHandler !== null) return;
  storageHandler = (event: Event) => {
    const key = (event as StorageEvent).key;
    // A null key is `localStorage.clear()` in another tab; any other key is not ours.
    if (key != null && key !== AUDIO_SETTINGS_KEY) return;
    const text = getStored(AUDIO_SETTINGS_KEY);
    const next = text === null ? DEFAULT_AUDIO_SETTINGS : parseStored(text);
    cache = next;
    notify(next);
  };
  window.addEventListener("storage", storageHandler);
}

function removeStorageListener(): void {
  if (storageHandler === null) return;
  window.removeEventListener("storage", storageHandler);
  storageHandler = null;
}

/** The first subscription also installs one window "storage" listener for AUDIO_SETTINGS_KEY that re-reads and notifies. */
export function subscribeAudioSettings(listener: (s: AudioSettings) => void): () => void {
  const sub: Subscription = { fn: listener };
  subscriptions.add(sub);
  installStorageListener();
  return () => {
    subscriptions.delete(sub);
    if (subscriptions.size === 0) removeStorageListener();
  };
}

/** useSyncExternalStore over the store; the snapshot object is referentially stable until a change. */
export function useAudioSettings(): readonly [AudioSettings, (patch: Partial<AudioSettings>) => void] {
  const settings = useSyncExternalStore(subscribeAudioSettings, readAudioSettings, readAudioSettings);
  return [settings, writeAudioSettings] as const;
}

/** Drops the cache, every subscriber and the storage listener. Tests only. */
export function resetAudioSettingsForTests(): void {
  cache = null;
  subscriptions.clear();
  removeStorageListener();
}
