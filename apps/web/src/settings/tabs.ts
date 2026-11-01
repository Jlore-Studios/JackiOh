// The settings dialog opens on the tab last used on this device (#128), a screen preference no account
// carries. Storage may be missing or throw, and a stored value that is not a tab now is ignored.

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
