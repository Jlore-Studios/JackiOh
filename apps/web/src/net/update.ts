// R4605000: a tab left open across a deploy notices the new build. The production build writes
// `dist/version.json` naming its entry script (static-pages.ts `VERSION_FILE`), and this module
// fetches that one static file when the tab wakes and remembers when the deployed entry differs
// from the running one. Presentation only (CLAUDE.md rule 7): it reads no game state.
// This module imports nothing: `net/navigate.ts` reads it, and the config chain (`vite.config.ts`
// `static-pages.ts` `navigate.ts`) bundles without the `@jackioh/*` aliases, so an import of
// `game/net.ts` here would break the build's own config load. The wake wiring lives in `main.tsx`.

/** How often a woken tab may ask the server for the next build: a wake inside the window asks nothing. */
export const UPDATE_CHECK_MIN_MS = 10 * 60 * 1000;

/** Where the build writes its entry name, served `no-cache` (public/_headers). */
export const VERSION_PATH = "/version.json";

/** What `sessionStorage` holds the entry a chunk failure already reloaded for, so it reloads once. */
export const UPDATE_RELOAD_KEY = "jackioh:update-reloaded-entry";

/** The deployed entry newer than the running one, or null while this build is current. */
let waiting: string | null = null;

/** When the last check ran, so a wake inside `UPDATE_CHECK_MIN_MS` sends no request. */
let lastCheckMs = 0;

/** The entry script this page loaded, or null outside a document or without a module script. */
export function runningEntry(): string | null {
  if (typeof document === "undefined") return null;
  return document.querySelector('script[type="module"]')?.getAttribute("src") ?? null;
}

/** The entry the deployed build names, or null when the fetch fails, misses, or names none. Never throws. */
export async function readDeployedEntry(): Promise<string | null> {
  try {
    const response = await fetch(VERSION_PATH, { cache: "no-store" });
    if (!response.ok) return null;
    const parsed: unknown = await response.json();
    if (typeof parsed !== "object" || parsed === null) return null;
    const entry = (parsed as { entry?: unknown }).entry;
    return typeof entry === "string" && entry.length > 0 ? entry : null;
  } catch {
    return null;
  }
}

/** Whether a newer build waits: the next menu move loads it instead of moving in place. */
export function updateWaiting(): boolean {
  return waiting !== null;
}

/**
 * R4605000: checks for a newer build, at most once per `UPDATE_CHECK_MIN_MS`. Outside a
 * production build (the dev server, a preview) nothing is checked and nothing changes. A failed
 * fetch, a missing file or a missing running entry changes nothing. Returns whether an update waits.
 */
export async function checkForUpdate(now: number = Date.now(), mode: string = import.meta.env.MODE): Promise<boolean> {
  if (mode !== "production") return updateWaiting();
  if (now - lastCheckMs < UPDATE_CHECK_MIN_MS) return updateWaiting();
  lastCheckMs = now;
  const deployed = await readDeployedEntry();
  const running = runningEntry();
  if (deployed !== null && running !== null && deployed !== running) waiting = deployed;
  return updateWaiting();
}

/**
 * R4605000: remembers that a failure already reloaded for `entry`. True when it did, so the
 * boundary shows its panel instead of reloading again; false the first time. A storage error
 * fails closed to false, so the panel shows and nothing loops.
 */
export function noteReloadedEntry(entry: string): boolean {
  try {
    if (window.sessionStorage.getItem(UPDATE_RELOAD_KEY) === entry) return true;
    window.sessionStorage.setItem(UPDATE_RELOAD_KEY, entry);
    return false;
  } catch {
    return false;
  }
}

/** Test seam: a tab with no update waiting and no check behind it. */
export function resetUpdateForTests(): void {
  waiting = null;
  lastCheckMs = 0;
}

/**
 * R4605000: leaves this build for `path`: what a menu move becomes with an update waiting, landing
 * on the screen asked for up to date. A `replace` move replaces instead of pushing, as in place.
 */
export function fullLoadPage(path: string, replace: boolean): void {
  if (replace) window.location.replace(path);
  else window.location.assign(path);
}

/** R4605000: takes the reloaded build: what a screen that failed on a stale chunk becomes. */
export function reloadPage(): void {
  window.location.reload();
}
