// Which tab the settings dialog opens on (issue #128): the last one the player used on this device.
//
// It is a screen preference, not a game setting, so it is not in the settings store and no account
// carries it. `localStorage` may be missing or throw (private windows, blocked site data), and a
// hand-edited value can hold anything: every access sits in try/catch and a value that is not one of
// the tabs the dialog has right now is ignored.

import type { SettingsSectionId } from "./slots.ts";

export const SETTINGS_TAB_STORAGE_KEY = "jackioh.settings.tab";

/** The tab the player last used, or null when nothing usable is stored. */
export function readRememberedTab(available: readonly SettingsSectionId[]): SettingsSectionId | null {
  try {
    const raw = window.localStorage.getItem(SETTINGS_TAB_STORAGE_KEY);
    return available.find((id) => id === raw) ?? null;
  } catch {
    return null;
  }
}

/** Remembers the tab; a failed write only means the dialog opens on the default next time. */
export function rememberTab(id: SettingsSectionId): void {
  try {
    window.localStorage.setItem(SETTINGS_TAB_STORAGE_KEY, id);
  } catch {
    // Quota, private mode or blocked storage: this visit is unaffected.
  }
}
