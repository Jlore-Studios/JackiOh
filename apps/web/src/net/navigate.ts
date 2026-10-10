// Client routing.
// M5's pathname switch and M6's screens share URL changes; §9.4 redirects pending accounts and room claims.
// `pushState` and `replaceState` do not fire `popstate`, so `navigate` dispatches an event for `usePathname`.

import { useSyncExternalStore } from "react";

const NAVIGATED = "jackioh:navigated";

export function currentPath(): string {
  if (typeof window === "undefined") return "/";
  return window.location.pathname.replace(/\/+$/, "") || "/";
}

function splitTarget(path: string): { pathname: string; search: string } {
  const at = path.indexOf("?");
  const rawPath = at === -1 ? path : path.slice(0, at);
  const query = at === -1 ? "" : path.slice(at + 1);
  return {
    pathname: rawPath.replace(/\/+$/, "") || "/",
    search: query.length === 0 ? "" : `?${query}`,
  };
}

/** Avoid stacking history when a guard re-renders the current pathname and query. */
export function navigate(path: string, options: { replace?: boolean } = {}): void {
  if (typeof window === "undefined") return;
  const target = splitTarget(path);
  if (currentPath() === target.pathname && window.location.search === target.search) return;
  if (options.replace === true) {
    window.history.replaceState(null, "", path);
  } else {
    window.history.pushState(null, "", path);
  }
  window.dispatchEvent(new Event(NAVIGATED));
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener("popstate", onChange);
  window.addEventListener(NAVIGATED, onChange);
  return () => {
    window.removeEventListener("popstate", onChange);
    window.removeEventListener(NAVIGATED, onChange);
  };
}

export function usePathname(): string {
  return useSyncExternalStore(subscribe, currentPath, currentPath);
}

/**
 * The runtime canonical link and static pages derive from this address. Static `index.html`,
 * `public/robots.txt`, and `public/.well-known/security.txt` must spell it too.
 */
export const SITE_ORIGIN = "https://jackioh.vercel.app";

/**
 * Client routes live in one table. `vercel.json` and `apps/web/vercel.json` must match it or
 * `net/deploy-routes.test.ts` fails; other paths are real 404s.
 */
export const paths = {
  landing: "/",
  login: "/login",
  resetPassword: "/reset-password",
  invite: "/invite",
  decks: "/decks",
  play: "/play",
  account: "/account",
  practice: "/practice",
  privacy: "/privacy",
  terms: "/terms",
  accessibility: "/accessibility",
  /** R388: patch notes and touched cards. */
  patchNotes: "/patch-notes",
  /** R630: browsable cards, including tokens. */
  almanac: "/almanac",
  /** R608, R612: the account-gated ranked ladder. */
  leaderboard: "/leaderboard",
  /** R654: public card and player statistics. */
  stats: "/stats",
  hotseat: "/dev/hotseat",
  match: (matchId: string): string => `/match/${matchId}`,
  /** R338: a Conquest series between its games: the score, the sealed picks and the pick clock. */
  series: (seriesId: string): string => `/series/${seriesId}`,
} as const;

function idUnder(path: string, first: string): string | null {
  const parts = path.split("/").filter((part) => part.length > 0);
  if (parts.length !== 2 || parts[0] !== first) return null;
  return parts[1] ?? null;
}

export function matchIdOf(path: string): string | null {
  return idUnder(path, "match");
}

export function seriesIdOf(path: string): string | null {
  return idUnder(path, "series");
}

// Sign-in screen entry states.
// `/login` permits only exact expiry and password-reset tokens, so URL input cannot carry a destination,
// message or address (B35, R193). A sign-in always lands on a `paths` value.

export type LoginReason = "expired";
export type LoginEntryMode = "forgot";

export function loginPath(options: { reason?: LoginReason; mode?: LoginEntryMode } = {}): string {
  const query = new URLSearchParams();
  if (options.reason === "expired") query.set("reason", "expired");
  if (options.mode === "forgot") query.set("mode", "forgot");
  const search = query.toString();
  return search.length === 0 ? paths.login : `${paths.login}?${search}`;
}

function queryValue(search: string, key: string): string | null {
  return new URLSearchParams(search).get(key);
}

export function loginReasonOf(search: string): LoginReason | null {
  return queryValue(search, "reason") === "expired" ? "expired" : null;
}

export function loginModeOf(search: string): LoginEntryMode | null {
  return queryValue(search, "mode") === "forgot" ? "forgot" : null;
}
