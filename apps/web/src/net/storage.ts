// The browser's storage, or null where reading it throws (a sandboxed frame, storage disabled, a
// test without a window), so every caller degrades to "nothing stored" the same way; and the reads,
// writes and removals every module makes through it (#2160).

export function localStorageOrNull(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

export function sessionStorageOrNull(): Storage | null {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

/** `key`'s value in `store` (`localStorage` unless named), or null when it holds none or cannot be read. */
export function getStored(key: string, store: Storage | null = localStorageOrNull()): string | null {
  try {
    return store?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

/**
 * Keeps `value` at `key` in `store` (`localStorage` unless named). False when it was not kept (no
 * storage, blocked, or full); a caller that ignores that keeps the value in memory for this page.
 */
export function setStored(key: string, value: string, store: Storage | null = localStorageOrNull()): boolean {
  if (store === null) return false;
  try {
    store.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

/** Forgets `key` in `store` (`localStorage` unless named); storage that cannot be reached held nothing. */
export function removeStored(key: string, store: Storage | null = localStorageOrNull()): void {
  try {
    store?.removeItem(key);
  } catch {
    // Blocked storage held nothing to forget.
  }
}
