// Client-side preferences (docs/polish/7-mobile-ux.md S8, B19–B22); they never enforce rules (CLAUDE.md rule 7).
// Storage is untrusted, so failed access uses in-memory or default settings.

import { useSyncExternalStore } from "react";
import { isRecord } from "../wire/json.ts";
import { getStored, setStored } from "../net/storage.ts";

export type Settings = {
  dragToPlay: boolean;
  /** Off: e2e specs end turns with one click. */
  confirmEndTurn: boolean;
  /** R82/R345: auto-ends only when ending is left. */
  autoEndTurn: boolean;
  hoverPreviews: boolean;
  /** R2102000: the lethal hint against the practice AI (R2291000). */
  lethalHint: boolean;
  /** R3477000: while an attack is aimed, each legal target shows the engine's forecast of it. */
  attackOutcomes: boolean;
  reduceMotion: boolean;
  /** R644 (§5): kept per device. */
  muteOpponentEmotes: boolean;
  /** R2990004: every match's sideline strip is muted from its start. */
  muteSpectatorEmotes: boolean;
};

export type SettingKey = keyof Settings;

export const SETTINGS_STORAGE_KEY = "jackioh.settings";

export const DEFAULT_SETTINGS: Readonly<Settings> = Object.freeze({
  dragToPlay: true,
  confirmEndTurn: false,
  autoEndTurn: true,
  hoverPreviews: true,
  lethalHint: true,
  attackOutcomes: true,
  reduceMotion: false,
  muteOpponentEmotes: false,
  muteSpectatorEmotes: false,
});

/** The kept keys, in storage order; any other (an older device's `publicStats`) is dropped (R2000000). */
const SETTING_KEYS: readonly SettingKey[] = [
  "dragToPlay",
  "confirmEndTurn",
  "autoEndTurn",
  "hoverPreviews",
  "lethalHint",
  "attackOutcomes",
  "reduceMotion",
  "muteOpponentEmotes",
  "muteSpectatorEmotes",
];

/** B22: settings.css maps this attribute to the reduced-motion scale. */
const REDUCE_MOTION_ATTRIBUTE = "data-reduce-motion";

let snapshot: Settings | null = null;

const subscriptions = new Set<() => void>();

/** Invalid values take their default (text that is not JSON among them); never throws. */
export function parseSettings(raw: unknown): Settings {
  const next: Settings = { ...DEFAULT_SETTINGS };
  try {
    const value: unknown = typeof raw === "string" ? JSON.parse(raw) : raw;
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

/** Neither step throws: `getStored` answers null for storage it cannot read, the defaults' input. */
function loadStored(): Settings {
  return Object.freeze(parseSettings(getStored(SETTINGS_STORAGE_KEY)));
}

function persist(settings: Settings): void {
  // Storage that fails leaves the in-memory settings.
  setStored(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
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
  // The DOM adds one listener once, however many subscribe.
  window.addEventListener("storage", onStorage);
  return () => {
    subscriptions.delete(subscription);
    if (subscriptions.size === 0) window.removeEventListener("storage", onStorage);
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
