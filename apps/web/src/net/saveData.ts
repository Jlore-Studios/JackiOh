// R2116000's Save-Data signal, shared: while the browser asks to save data, no background
// prefetch runs. The voice prefetch (R4316000) reads the same signal through its own port.

/** Chromium's Network Information field, absent from TypeScript's DOM types. */
type SaveDataNavigator = Navigator & { connection?: { saveData?: boolean } };

/** Whether the browser asks sites to save data (Chrome's Lite mode / Data Saver). False with no `navigator`. */
export function savingData(): boolean {
  if (typeof navigator === "undefined") return false;
  return (navigator as SaveDataNavigator).connection?.saveData === true;
}
