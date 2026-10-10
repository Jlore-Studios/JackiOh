// The tab title and the canonical link of every path the client serves, with no React and no DOM,
// so the build's static pages (`apps/web/static-pages.ts`) name a page as the app does. `main.tsx`
// re-exports them and writes them into the document (`useDocumentHead`).

import { SITE_ORIGIN, matchIdOf, paths, seriesIdOf } from "./navigate.ts";

/** The name every tab title ends with. */
export const SITE_NAME = "JackiOh";

/** The screen's name, for its tab title; null for a path the client serves nothing at. */
export function screenNameFor(path: string): string | null {
  switch (path) {
    case paths.landing:
      return "";
    case paths.login:
      return "Sign in";
    case paths.resetPassword:
      return "Reset password";
    case paths.invite:
      return "Invite code";
    case paths.decks:
      return "Decks";
    case paths.play:
      return "Play online";
    case paths.account:
      return "Account";
    case paths.practice:
      return "Practice";
    case paths.privacy:
      return "Privacy";
    case paths.terms:
      return "Terms";
    case paths.accessibility:
      return "Accessibility";
    case paths.patchNotes:
      return "Patch notes";
    case paths.almanac:
      return "Almanac";
    case paths.leaderboard:
      return "Leaderboard";
    case paths.stats:
      return "Statistics";
    case paths.hotseat:
      // Read here, not at load: Vite's config loader runs this file with no `import.meta.env`.
      return import.meta.env.MODE !== "production" ? "Hotseat" : null;
  }
  if (matchIdOf(path) !== null) return "Match";
  if (seriesIdOf(path) !== null) return "Conquest";
  return null;
}

/**
 * The tab title for a path: "Sign in · JackiOh", and plain "JackiOh" on the landing page. The
 * match screen replaces it with "Your turn · JackiOh" while it is the player's turn (match.tsx).
 */
export function documentTitleFor(path: string): string {
  const name = screenNameFor(path);
  if (name === null) return `Page not found · ${SITE_NAME}`;
  return name === "" ? SITE_NAME : `${name} · ${SITE_NAME}`;
}

/** The canonical address of a path the client serves, or null for one it does not (a 404). */
export function canonicalUrlFor(path: string): string | null {
  return screenNameFor(path) === null ? null : `${SITE_ORIGIN}${path}`;
}
