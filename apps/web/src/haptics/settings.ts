// The viewer's vibration switch (R660, issue #259): a presentation preference of this device, so it
// lives in this browser only, under HAPTICS_SETTINGS_KEY in `localStorage`. The settings panel mounts
// it through `SETTINGS_SLOTS`; the haptics player reads it at every buzz, so it applies at once.
//
// Storage is optional. A missing `window`, a throwing `localStorage`, a quota error on write or
// unparsable JSON fall back to the default or to the in-memory value, and nothing here throws.

import { useSyncExternalStore } from "react";

export type HapticsSettings = { vibration: boolean };

export const HAPTICS_SETTINGS_KEY = "jackioh.haptics.v1";

export const DEFAULT_HAPTICS_SETTINGS: Readonly<HapticsSettings> = Object.freeze({ vibration: true });

/** Field by field: a missing or mistyped value is its default. */
export function parseHapticsSettings(raw: unknown): HapticsSettings {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) return { ...DEFAULT_HAPTICS_SETTINGS };
  const vibration = (raw as Record<string, unknown>).vibration;
  return { vibration: typeof vibration === "boolean" ? vibration : DEFAULT_HAPTICS_SETTINGS.vibration };
}

function load(): HapticsSettings {
  try {
    const text = window.localStorage.getItem(HAPTICS_SETTINGS_KEY);
    return text === null ? { ...DEFAULT_HAPTICS_SETTINGS } : parseHapticsSettings(JSON.parse(text));
  } catch {
    return { ...DEFAULT_HAPTICS_SETTINGS };
  }
}

let current: HapticsSettings | null = null;
const listeners = new Set<() => void>();

/** The current value, loaded from storage on first read. */
export function readHapticsSettings(): HapticsSettings {
  current ??= load();
  return current;
}

/** Merges `patch` over the current value, persists it in try/catch and notifies. */
export function writeHapticsSettings(patch: Partial<HapticsSettings>): HapticsSettings {
  const next = parseHapticsSettings({ ...readHapticsSettings(), ...patch });
  current = next;
  try {
    window.localStorage.setItem(HAPTICS_SETTINGS_KEY, JSON.stringify(next));
  } catch {
    // Quota, private mode or blocked storage: the switch still holds in memory for this page.
  }
  for (const listener of [...listeners]) listener();
  return next;
}

export function subscribeHapticsSettings(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The current settings; re-renders on every write. */
export function useHapticsSettings(): HapticsSettings {
  return useSyncExternalStore(subscribeHapticsSettings, readHapticsSettings, readHapticsSettings);
}

/** Test seam: drops the in-memory value and listeners so the next read reloads storage. */
export function resetHapticsSettingsForTests(): void {
  current = null;
  listeners.clear();
}
