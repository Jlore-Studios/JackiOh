// The browser's storage, or null where reading it throws (a sandboxed frame, storage disabled, a
// test without a window), so every caller degrades to "nothing stored" the same way.

/** `localStorage`: what this browser keeps across visits. */
export function localStorageOrNull(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

/** `sessionStorage`: what this tab keeps until it closes. */
export function sessionStorageOrNull(): Storage | null {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}
