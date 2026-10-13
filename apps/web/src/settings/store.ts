// Client-side preferences (docs/polish/7-mobile-ux.md S8, B19–B22).
// They never enforce rules (CLAUDE.md rule 7); `autoEndTurn` is player intent for R82 and R345.
// Storage is untrusted, so failed access uses in-memory or default settings.

import { useSyncExternalStore } from "react";

export type Settings = {
  dragToPlay: boolean;
  /** Gameplay; stays false because e2e specs end turns with one click. */
  confirmEndTurn: boolean;
  /** R82/R345: default true auto-ends only when ending is left. */
  autoEndTurn: boolean;
  hoverPreviews: boolean;
  reduceMotion: boolean;
  /** R644 (§5): mute opponents' emotes, default off per device. */
  muteOpponentEmotes: boolean;
  publicStats: boolean;
};

export type SettingKey = keyof Settings;

export const SETTINGS_STORAGE_KEY = "jackioh.settings";

export const DEFAULT_SETTINGS: Readonly<Settings> = Object.freeze({
  dragToPlay: true,
  confirmEndTurn: false,
  autoEndTurn: true,
  hoverPreviews: true,
  reduceMotion: false,
  muteOpponentEmotes: false,
  publicStats: true,
});

/** The keys `parseSettings` keeps, in the order they are written to storage. */
const SETTING_KEYS: readonly SettingKey[] = [
  "dragToPlay",
  "confirmEndTurn",
  "autoEndTurn",
  "hoverPreviews",
  "reduceMotion",
  "muteOpponentEmotes",
  "publicStats",
];

/** B22: settings.css maps this <html> attribute to the reduced-motion scale. */
const REDUCE_MOTION_ATTRIBUTE = "data-reduce-motion";

let snapshot: Settings | null = null;

const subscriptions = new Set<() => void>();

let storageListening = false;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Tolerant storage parser: invalid values use defaults and never throw. */
export function parseSettings(raw: unknown): Settings {
  const next: Settings = { ...DEFAULT_SETTINGS };
  try {
    let value: unknown = raw;
    if (typeof value === "string") {
      try {
        value = JSON.parse(value) as unknown;
      } catch {
        value = null;
      }
    }
    if (!isRecord(value)) return next;
    for (const key of SETTING_KEYS) {
      const candidate = value[key];
      if (typeof candidate === "boolean") next[key] = candidate;
    }
    return next;
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

function storageOrNull(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

function loadStored(): Settings {
  try {
    const storage = storageOrNull();
    if (storage === null) return DEFAULT_SETTINGS;
    const raw = storage.getItem(SETTINGS_STORAGE_KEY);
    if (raw === null) return DEFAULT_SETTINGS;
    return Object.freeze(parseSettings(raw));
  } catch {
    return DEFAULT_SETTINGS;
  }
}

function persist(settings: Settings): void {
  try {
    storageOrNull()?.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Keep in-memory settings when storage fails.
  }
}

function applyToDocument(settings: Settings): void {
  const root = document.documentElement;
  if (settings.reduceMotion) root.setAttribute(REDUCE_MOTION_ATTRIBUTE, "true");
  else root.removeAttribute(REDUCE_MOTION_ATTRIBUTE);
}

function sameSettings(a: Settings, b: Settings): boolean {
  return SETTING_KEYS.every((key) => a[key] === b[key]);
}

function notify(): void {
  for (const subscription of [...subscriptions]) subscription();
}

/** Preserve identity when unchanged to avoid a useSyncExternalStore re-render; always notify writes. */
function commit(next: Settings): Settings {
  const current = readSettings();
  const settled = sameSettings(current, next) ? current : Object.freeze({ ...next });
  snapshot = settled;
  persist(settled);
  applyToDocument(settled);
  notify();
  return settled;
}

export function readSettings(): Settings {
  if (snapshot === null) {
    snapshot = loadStored();
    applyToDocument(snapshot);
  }
  return snapshot;
}

export function writeSettings(patch: Partial<Settings>): Settings {
  const next: Settings = { ...readSettings() };
  for (const key of SETTING_KEYS) {
    const value: unknown = patch[key];
    if (typeof value === "boolean") next[key] = value;
  }
  return commit(next);
}

export function resetSettings(): Settings {
  return commit({ ...DEFAULT_SETTINGS });
}

function onStorage(event: StorageEvent): void {
  if (typeof event.key === "string" && event.key !== SETTINGS_STORAGE_KEY) return;
  const next =
    typeof event.newValue === "string" ? Object.freeze(parseSettings(event.newValue)) : loadStored();
  const current = snapshot;
  const settled = current !== null && sameSettings(current, next) ? current : next;
  snapshot = settled;
  applyToDocument(settled);
  notify();
}

export function subscribeSettings(listener: () => void): () => void {
  const subscription = (): void => {
    listener();
  };
  subscriptions.add(subscription);
  if (!storageListening) {
    window.addEventListener("storage", onStorage);
    storageListening = true;
  }
  return () => {
    subscriptions.delete(subscription);
    if (subscriptions.size === 0 && storageListening) {
      window.removeEventListener("storage", onStorage);
      storageListening = false;
    }
  };
}

export function useSettings(): Settings {
  return useSyncExternalStore(subscribeSettings, readSettings, readSettings);
}

export function useSetting<K extends SettingKey>(key: K): Settings[K] {
  const read = (): Settings[K] => readSettings()[key];
  return useSyncExternalStore(subscribeSettings, read, read);
}

export function __resetSettingsForTests(): void {
  snapshot = null;
  document.documentElement.removeAttribute(REDUCE_MOTION_ATTRIBUTE);
}
