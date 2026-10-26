// Client routing.
// M5's pathname switch and M6's screens share URL changes; §9.4 redirects pending accounts and room claims.
// `pushState` and `replaceState` do not fire `popstate`, so `navigate` dispatches an event for `usePathname`.
// A page holding work a move would lose registers a leave guard (`guardLeaving`, R1754000): `navigate` and the browser's Back wait for it.

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

/** R1754000: the page that asks before it is left, and its own address, which a held Back puts back. One page at a time. */
let leaveGuard: { hold: (leave: () => void) => void; here: string } | null = null;

function here(): string {
  return window.location.pathname + window.location.search;
}

/**
 * R1754000: while a page holds work a move would lose, every `navigate` off it (but a `force`d one)
 * and every `popstate` off it is held: `hold` is handed the move, to make if the player lets the
 * work go. Returns the release, which the page calls once nothing is at stake.
 */
export function guardLeaving(hold: (leave: () => void) => void): () => void {
  const guard = { hold, here: here() };
  leaveGuard = guard;
  return () => {
    if (leaveGuard === guard) leaveGuard = null;
  };
}

function go(path: string, replace: boolean): void {
  const from = currentPath();
  if (replace) window.history.replaceState(null, "", path);
  else window.history.pushState(null, "", path);
  if (currentPath() !== from) window.scrollTo(0, 0);
  window.dispatchEvent(new Event(NAVIGATED));
}

/**
 * Avoid stacking history when a guard re-renders the current pathname and query. A new pathname opens
 * at the top (#1756), scrolled after the push so the entry left keeps its position for Back; a change
 * of query or fragment alone keeps the scroll, and Back and Forward (`popstate`) are the browser's.
 * R1754000: a page's leave guard holds the move unless `force` (a move the page itself decided);
 * returns false when it held it.
 */
export function navigate(path: string, options: { replace?: boolean; force?: boolean } = {}): boolean {
  if (typeof window === "undefined") return true;
  const target = splitTarget(path);
  if (currentPath() === target.pathname && window.location.search === target.search) return true;
  const replace = options.replace === true;
  const guard = leaveGuard;
  if (guard !== null && options.force !== true) {
    guard.hold(() => {
      go(path, replace);
    });
    return false;
  }
  go(path, replace);
  return true;
}

/**
 * R1754000, R1831000: work that leaves the page without a `navigate` of its own (the toast's Accept
 * joins a room, then goes) waits on the guard as a move would. Runs `then` at once when no page
 * guards; otherwise the guard is handed it, to run if the player lets the work go, and this returns
 * false.
 */
export function leaveFirst(then: () => void): boolean {
  const guard = leaveGuard;
  if (guard === null) {
    then();
    return true;
  }
  guard.hold(then);
  return false;
}

/**
 * R1754000: a `popstate` off a guarded page puts its address back before any screen reads the
 * path, and hands the guard the way out: the browser's own Back, the guard gone, which returns to
 * the entry the player went to (the restored address sits just after it). Had the player pressed
 * Forward, that Back lands on the guarded page instead, so the entry is made again.
 */
function heldPop(): boolean {
  const guard = leaveGuard;
  if (guard === null || here() === guard.here) return false;
  const popped = here();
  window.history.pushState(null, "", guard.here);
  guard.hold(() => {
    if (leaveGuard !== guard) return;
    leaveGuard = null;
    window.addEventListener(
      "popstate",
      () => {
        if (here() !== popped) go(popped, false);
      },
      { once: true },
    );
    window.history.back();
  });
  return true;
}

function subscribe(onChange: () => void): () => void {
  const onPop = (): void => {
    if (!heldPop()) onChange();
  };
  window.addEventListener("popstate", onPop);
  window.addEventListener(NAVIGATED, onChange);
  return () => {
    window.removeEventListener("popstate", onPop);
    window.removeEventListener(NAVIGATED, onChange);
  };
}

export function usePathname(): string {
  return useSyncExternalStore(subscribe, currentPath, currentPath);
}

function currentSearch(): string {
  if (typeof window === "undefined") return "";
  return window.location.search;
}

/** The query string (`?a=b`, or empty), re-read on every `navigate` and every `popstate`. */
export function useSearch(): string {
  return useSyncExternalStore(subscribe, currentSearch, currentSearch);
}

/**
 * The address the source files spell: `index.html`, `public/robots.txt` and
 * `public/.well-known/security.txt` name it, and `static-pages.ts` replaces it with the build's
 * own (#696). It is staging's host, and the address of any build that sets no `VITE_SITE_ORIGIN`.
 */
export const SITE_ORIGIN = "https://jackioh.vercel.app";

/** A build's address from its `VITE_SITE_ORIGIN`, without a trailing slash; `SITE_ORIGIN` when unset or empty. */
export function originOf(configured: unknown): string {
  return typeof configured === "string" && configured.length > 0 ? configured.replace(/\/+$/u, "") : SITE_ORIGIN;
}

/**
 * The address this bundle is served from: jackioh.com in production (`.env.production`), the Vercel
 * host on staging (`vercel.json`'s build command). Read here, not at load: Vite's config loader runs
 * this file with no `import.meta.env`, so `static-pages.ts` calls `originOf` instead.
 */
export function siteOrigin(): string {
  const configured: unknown = import.meta.env.VITE_SITE_ORIGIN;
  return originOf(configured);
}

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
  /** Issue #757: the copyright and takedown page. */
  copyright: "/copyright",
  /** R388: patch notes and touched cards. */
  patchNotes: "/patch-notes",
  /** R630: browsable cards, including tokens. */
  almanac: "/almanac",
  /** The Almanac with one card's detail open, as the statistics drill-down links it. */
  almanacCard: (cardId: string): string => `/almanac?card=${encodeURIComponent(cardId)}`,
  /** R608, R612: the account-gated ranked ladder. */
  leaderboard: "/leaderboard",
  /** R654: public card and player statistics. */
  stats: "/stats",
  /** #920: the card creator, the account's custom cards (SPEC §12.9). */
  creator: "/creator",
  /** R768000: the recent games and their replays. */
  replays: "/replays",
  /** #936 S8: Play together: friends, requests, challenges, chat and shared replays. */
  together: "/together",
  /** #1307: the account's collection, wallet, tracks and quests. */
  collection: "/collection",
  /** R1579000: a Collection section that opens in place. */
  collectionSection: (section: "cards" | "cosmetics"): string => `/collection?section=${section}`,
  /** #1577, R1580000: the shop: buying and opening packs, pity, the tokens and the cosmetic packs. */
  shop: "/shop",
  /** #1665, R1665000: the odds and pity of every pack, public like the terms; the account's counters need a session. */
  purchases: "/purchases",
  hotseat: "/dev/hotseat",
  match: (matchId: string): string => `/match/${matchId}`,
  /** R338: a Conquest series between its games: the score, the sealed picks and the pick clock. */
  series: (seriesId: string): string => `/series/${seriesId}`,
  /** #936 S8 (B50): a friend's live game, watched from their seat; the friend rides on `?friend=`. */
  watch: (matchId: string): string => "/watch/" + matchId,
  /** #936 B61–B64: a Duos co-op match against the AI team. */
  duos: (matchId: string): string => `/duos/${matchId}`,
  /** #936 B65: the four-seat Duos hotseat over WASM; production serves a 404 there, as at `hotseat`. */
  duosDev: "/dev/duos",
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

/** #936 S8: the match a `/watch/:matchId` path watches, or null. */
export function watchIdOf(path: string): string | null {
  return idUnder(path, "watch");
}

/** #936: the match id of a `/duos/:matchId` path, or null. */
export function duosIdOf(path: string): string | null {
  return idUnder(path, "duos");
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

/** #921: the creator's "Try it": `/practice?try=<id>` deals a random deck around that custom card (R915005). */
export function tryPath(customId: string): string {
  return `${paths.practice}?try=${encodeURIComponent(customId)}`;
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
