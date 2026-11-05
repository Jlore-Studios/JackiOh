// A media query the stylesheet also reads, followed live: one copy for every hook that follows one
// (`phoneBar.ts`, `useOsReducedMotion.ts`, the Patch notes legend, the shortcut hints). A host without
// `matchMedia` has no list and nothing to follow. Nothing caches the list: a test swaps
// `window.matchMedia` between renders.

/** `text`'s `MediaQueryList`, or null where the host has no `matchMedia` or it throws. */
export function queryList(text: string): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(text);
  } catch {
    return null;
  }
}

/** Calls `onChange` whenever `text` flips; the unsubscribe `useSyncExternalStore` takes. */
export function subscribeTo(text: string, onChange: () => void): () => void {
  const query = queryList(text);
  if (query === null || typeof query.addEventListener !== "function") return () => undefined;
  query.addEventListener("change", onChange);
  return () => {
    query.removeEventListener("change", onChange);
  };
}
