// Where a sign-in lands (issue #479): the main menu, which is the landing page, unless a gated
// screen sent the player to sign in. Then the sign-in goes back to that screen.
//
// "Play online" while signed out goes through the gate to `/login`, so the gate remembers which
// screen sent the player to sign in. It is kept in `sessionStorage` (this tab, this visit, so it
// outlasts an OAuth provider's round trip, R666) and it is never a URL: only one of the fixed gated
// `paths` values below is ever written or read back, so nothing typed, linked or planted can choose
// where a sign-in lands (B35). A player who goes back to the main menu has left the screen that
// asked, so the landing page forgets it, and a sign-in started from there lands on the main menu.

import { paths } from "./navigate.ts";

export const RETURN_TO_STORAGE_KEY = "jackioh.auth.returnTo";

/** Where a sign-in lands when no gated screen sent the player to sign in: the main menu. */
export const SIGN_IN_HOME = paths.landing;

/** The gated screens a sign-in may go back to. A match is left out: its id is data from a URL. */
const RETURN_TARGETS: readonly string[] = [paths.decks, paths.play, paths.invite, paths.account];

/** The allowed path equal to `path`, or null. */
function returnTarget(path: string | null): string | null {
  if (path === null) return null;
  return RETURN_TARGETS.find((target) => target === path) ?? null;
}

/** The gate sent the player from `path` to sign in; remembered only when it is a fixed gated path. */
export function rememberReturnTo(path: string): void {
  if (typeof window === "undefined") return;
  const target = returnTarget(path);
  try {
    if (target === null) window.sessionStorage.removeItem(RETURN_TO_STORAGE_KEY);
    else window.sessionStorage.setItem(RETURN_TO_STORAGE_KEY, target);
  } catch {
    // Blocked storage: a sign-in lands on the main menu.
  }
}

/** The player is back on the main menu, so no screen is waiting for their sign-in any more. */
export function forgetReturnTo(): void {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.removeItem(RETURN_TO_STORAGE_KEY);
  } catch {
    // Blocked storage holds nothing to forget.
  }
}

/** Where a sign-in goes back to, read once and forgotten: an allowed path, or null. */
export function takeReturnTo(): string | null {
  if (typeof window === "undefined") return null;
  let raw: string | null;
  try {
    raw = window.sessionStorage.getItem(RETURN_TO_STORAGE_KEY);
    window.sessionStorage.removeItem(RETURN_TO_STORAGE_KEY);
  } catch {
    return null;
  }
  return returnTarget(raw);
}

/**
 * Where a successful sign-in goes: the gated screen that sent the player to sign in (read once),
 * else the main menu. Always a fixed `paths` value. A failed sign-in never calls this, so the player
 * stays on the sign-in screen and the screen that asked is still remembered for the next try.
 */
export function signInDestination(): string {
  return takeReturnTo() ?? SIGN_IN_HOME;
}
