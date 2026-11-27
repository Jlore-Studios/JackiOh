// The viewer's effects settings (docs/polish/1-animations.md, Surface S3): speed (R201, R435), intensity,
// a motion override and Big moments (R2785000), kept per browser under `FX_SETTINGS_KEY`; the runner reads `getFxSettings()`
// at every enqueue. Storage is optional and nothing here throws on it. Every value read back is
// normalized field by field, so a stale or hand-edited entry can never leave R201's range.
//
// R5128000: the panel's Low graphics holds intensity and Big moments off. `getFxSettings()` is what the effects run on,
// with that hold; `getChosenFxSettings()` is what the player chose and the store keeps, which the panel's controls show
// and the account sync carries, so turning Low graphics off gives the player's own choices back.

import { useSyncExternalStore } from "react";

import { localStorageOrNull } from "../net/storage.ts";
import { readSettings as readPanelSettings, subscribeSettings as subscribePanelSettings } from "../settings/store.ts";
import { isRecord, isSafeKey } from "../wire/json.ts";
import { FX_INTENSITY_SCALE, FX_SETTINGS_KEY, FX_SPEED_DEFAULT, FX_SPEED_MAX, FX_SPEED_MIN } from "./constants.ts";

export type FxIntensity = "off" | "low" | "normal" | "high";
export type FxMotion = "system" | "reduce";
/** R2785000: how long a set piece holds the game: its whole beat, a short one, or none. */
export type FxBigMoments = "full" | "short" | "off";
export type FxSettings = { speed: number; intensity: FxIntensity; motion: FxMotion; bigMoments: FxBigMoments };

export const DEFAULT_FX_SETTINGS: FxSettings = {
  speed: FX_SPEED_DEFAULT,
  intensity: "normal",
  motion: "system",
  bigMoments: "full",
};
Object.freeze(DEFAULT_FX_SETTINGS);

export function normalizeSpeed(speed: unknown): number {
  if (typeof speed !== "number" || !Number.isFinite(speed)) return FX_SPEED_DEFAULT;
  return Math.min(FX_SPEED_MAX, Math.max(FX_SPEED_MIN, speed));
}

function normalizeIntensity(intensity: unknown): FxIntensity {
  if (typeof intensity === "string" && Object.prototype.hasOwnProperty.call(FX_INTENSITY_SCALE, intensity)) {
    return intensity as FxIntensity;
  }
  return DEFAULT_FX_SETTINGS.intensity;
}

function normalizeMotion(motion: unknown): FxMotion {
  return motion === "system" || motion === "reduce" ? motion : DEFAULT_FX_SETTINGS.motion;
}

function normalizeBigMoments(value: unknown): FxBigMoments {
  return value === "full" || value === "short" || value === "off" ? value : DEFAULT_FX_SETTINGS.bigMoments;
}

export function normalizeFxSettings(raw: unknown): FxSettings {
  if (!isRecord(raw)) return DEFAULT_FX_SETTINGS;
  return {
    speed: normalizeSpeed(raw.speed),
    intensity: normalizeIntensity(raw.intensity),
    motion: normalizeMotion(raw.motion),
    bigMoments: normalizeBigMoments(raw.bigMoments),
  };
}

/** Absent, throwing or unparsable storage gives the frozen `DEFAULT_FX_SETTINGS` itself; sharing it is safe. */
export function loadFxSettings(storage?: Storage | null): FxSettings {
  try {
    const store = storage === undefined ? localStorageOrNull() : storage;
    if (store === null) return DEFAULT_FX_SETTINGS;
    const text = store.getItem(FX_SETTINGS_KEY);
    if (text === null) return DEFAULT_FX_SETTINGS;
    return normalizeFxSettings(JSON.parse(text));
  } catch {
    return DEFAULT_FX_SETTINGS;
  }
}

function persist(settings: FxSettings): void {
  try {
    const store = localStorageOrNull();
    if (store === null) return;
    store.setItem(FX_SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // Quota, private mode or blocked storage: the setting still holds in memory for this page.
  }
}

/** R5128000: what Low graphics holds while it is on: the effects layer at intensity off and no set piece's hold. */
export const LOW_GRAPHICS_FX: Readonly<Pick<FxSettings, "intensity" | "bigMoments">> = Object.freeze({
  intensity: "off",
  bigMoments: "off",
});

let current: FxSettings | null = null;
/** The held settings for one chosen object, so a snapshot keeps its identity until either changes. */
let held: { chosen: FxSettings; value: FxSettings } | null = null;
const listeners = new Set<(settings: FxSettings) => void>();

/** R5128000: the settings the player chose, which the store keeps, whatever Low graphics holds. */
export function getChosenFxSettings(): FxSettings {
  if (current === null) current = loadFxSettings();
  return current;
}

/** R5128000: the settings the effects run on: the chosen ones, with intensity and Big moments off under Low graphics. */
export function getFxSettings(): FxSettings {
  const chosen = getChosenFxSettings();
  if (!readPanelSettings().lowGraphics) return chosen;
  if (held?.chosen !== chosen) held = { chosen, value: { ...chosen, ...LOW_GRAPHICS_FX } };
  return held.value;
}

/** Writes onto the chosen settings and returns them. */
export function setFxSettings(patch: Partial<FxSettings>): FxSettings {
  const merged: Record<string, unknown> = { ...getChosenFxSettings() };
  for (const [key, value] of Object.entries(patch)) {
    // Issue #4015: a key from outside (the account's copy) never becomes an object key.
    if (!isSafeKey(key)) continue;
    if (value !== undefined) merged[key] = value;
  }
  const next = normalizeFxSettings(merged);
  current = next;
  persist(next);
  for (const listener of [...listeners]) listener(next);
  return next;
}

/** Calls `listener` with the chosen settings after every write to this store; Low graphics is the panel store's. */
export function subscribeFxSettings(listener: (settings: FxSettings) => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** R5128000: this store's writes and the panel's, so a mounted reader follows Low graphics as it turns on or off. */
function subscribeEffective(listener: () => void): () => void {
  const fx = subscribeFxSettings(() => {
    listener();
  });
  const panel = subscribePanelSettings(listener);
  return () => {
    fx();
    panel();
  };
}

/** The settings the effects run on (`getFxSettings`) and the writer. */
export function useFxSettings(): readonly [FxSettings, (patch: Partial<FxSettings>) => void] {
  const settings = useSyncExternalStore(subscribeEffective, getFxSettings, getFxSettings);
  return [settings, setFxSettings] as const;
}

/** R5128000: the chosen settings, which the panel's controls show. */
export function useChosenFxSettings(): FxSettings {
  return useSyncExternalStore(subscribeFxSettings, getChosenFxSettings, getChosenFxSettings);
}

/** Test seam: drops the in-memory value and listeners so the next read reloads storage. */
export function resetFxSettingsForTests(): void {
  current = null;
  held = null;
  listeners.clear();
}
