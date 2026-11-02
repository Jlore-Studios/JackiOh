// Client routing; §9.4 redirects pending accounts and room claims.
// `pushState` and `replaceState` do not fire `popstate`, so `navigate` dispatches an event for `usePathname`.
// Moves wait on a leave guard (R1754000), cross-fade between menu screens (R2117000) and give an open
// sheet or dialog Back first (`backLayers.ts`, R2127000).

import { useSyncExternalStore } from "react";
import { flushSync } from "react-dom";

import { reducedMotionNow } from "../settings/motion.ts";
import { moveOverBackEntry, popBackLayer } from "./backLayers.ts";

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
 * R1754000: every `navigate` (but a `force`d one) and `popstate` off the page is handed to `hold`, to
 * make if the player lets the work go. Returns the release, called once nothing is at stake.
 */
export function guardLeaving(hold: (leave: () => void) => void): () => void {
  const guard = { hold, here: here() };
  leaveGuard = guard;
  return () => {
    if (leaveGuard === guard) leaveGuard = null;
  };
}

/**
 * R2117000: `testid.board` spelled out: e2e's typecheck loads this file through the Vite config and
 * cannot compile what `game/contract.ts` imports. viewTransition.test.tsx pins the two.
 */
const BOARD_ON_SCREEN = '[data-testid="board"]';

/** R2117000: the pathname the screens were last told of; by a `popstate` the address has moved on. */
let shown = "/";

/** R2117000: every `usePathname` and `useSearch` reader, so one `popstate` tells them all in one transition. */
const readers = new Set<() => void>();

/** R2117000: only between two menu screens; a query or fragment alone is the same screen, and a replay plays on a board in place. */
function crossFades(from: string, to: string): boolean {
  if (from === to || isBoardPath(from) || isBoardPath(to)) return false;
  if (document.querySelector(BOARD_ON_SCREEN) !== null) return false;
  return !reducedMotionNow() && typeof document.startViewTransition === "function";
}

let flushing = false;

/**
 * R2117000: a cross-fading move runs `update` in a view transition, flushed at once so the browser
 * captures the new screen. A move made during that flush (a gate's redirect) joins it: a second
 * transition would skip the first and flush inside React's commit (React #177, spec 14 B37). A
 * skipped transition rejects its `ready`, which is no error.
 */
function showMove(from: string, update: () => void): void {
  const to = currentPath();
  shown = to;
  if (flushing || !crossFades(from, to)) {
    update();
    return;
  }
  const transition = document.startViewTransition(() => {
    flushing = true;
    try {
      flushSync(update);
    } finally {
      flushing = false;
    }
  });
  transition.ready.catch(() => undefined);
}

function go(path: string, replace: boolean): void {
  const from = currentPath();
  if (!moveOverBackEntry(path, replace)) {
    if (replace) window.history.replaceState(null, "", path);
    else window.history.pushState(null, "", path);
  }
  const moved = currentPath() !== from;
  showMove(from, () => {
    if (moved) window.scrollTo(0, 0);
    window.dispatchEvent(new Event(NAVIGATED));
  });
}

/**
 * No stacking when a guard re-renders the current address. A new pathname opens at the top (#1756),
 * scrolled after the push so the entry left keeps its position for Back (`popstate` scroll is the
 * browser's). R1754000: a leave guard holds the move unless `force`d; false when it held it.
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
 * R1754000, R1831000: work that leaves without a `navigate` of its own (the toast's Accept joins a
 * room, then goes) waits on the guard as a move would; false when the guard holds it.
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
 * R1754000: a `popstate` off a guarded page puts its address back before any screen reads it; let go,
 * Back leaves for the entry the player went to (made again after a Forward, whose Back lands here).
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

function tellReaders(): void {
  for (const onChange of [...readers]) onChange();
}

/**
 * One `popstate` listener for every reader: an overlay's Back tells none (R2127000), nor does a held
 * one (R1754000), and any other is one move (R2117000).
 */
function onPop(event: PopStateEvent): void {
  if (popBackLayer(event)) return;
  if (!heldPop()) showMove(shown, tellReaders);
}

function subscribe(onChange: () => void): () => void {
  if (readers.size === 0) {
    shown = currentPath();
    window.addEventListener("popstate", onPop);
  }
  readers.add(onChange);
  window.addEventListener(NAVIGATED, onChange);
  return () => {
    window.removeEventListener(NAVIGATED, onChange);
    readers.delete(onChange);
    if (readers.size === 0) window.removeEventListener("popstate", onPop);
  };
}

export function usePathname(): string {
  return useSyncExternalStore(subscribe, currentPath, currentPath);
}

function currentSearch(): string {
  if (typeof window === "undefined") return "";
  return window.location.search;
}

export function useSearch(): string {
  return useSyncExternalStore(subscribe, currentSearch, currentSearch);
}

/**
 * The address `index.html`, `public/robots.txt` and `public/.well-known/security.txt` spell, which
 * `static-pages.ts` replaces with the build's own (#696): staging's, and a build's with no `VITE_SITE_ORIGIN`.
 */
export const SITE_ORIGIN = "https://jackioh.vercel.app";

export function originOf(configured: unknown): string {
  return typeof configured === "string" && configured.length > 0 ? configured.replace(/\/+$/u, "") : SITE_ORIGIN;
}

/**
 * jackioh.com in production (`.env.production`), the Vercel host on staging (`vercel.json`'s build
 * command). Read here, not at load: Vite's config loader runs this file with no `import.meta.env`, so
 * `static-pages.ts` calls `originOf` instead.
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
  /** R2631000: how JackiOh uses AI. */
  ai: "/ai",
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
  /** R654: the statistics page's Cards tab with one card's drill-down open, as the card detail's View full stats links it. */
  statsCard: (cardId: string): string => `/stats?tab=cards&card=${encodeURIComponent(cardId)}`,
  /** #920: the card creator, the account's custom cards (SPEC §12.9). */
  creator: "/creator",
  /** R768000: the recent games and their replays. */
  replays: "/replays",
  /** #936 S8: Play together. */
  together: "/together",
  /** #1307: the account's collection, wallet, tracks and quests. */
  collection: "/collection",
  /** R1579000: a Collection section that opens in place. */
  collectionSection: (section: "cards" | "cosmetics"): string => `/collection?section=${section}`,
  /** #1577, R1580000: the shop. */
  shop: "/shop",
  /** #1665, R1665000: the odds and pity of every pack, public like the terms; the account's counters need a session. */
  purchases: "/purchases",
  hotseat: "/dev/hotseat",
  match: (matchId: string): string => `/match/${matchId}`,
  /** R338: a Conquest series between its games. */
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

export function watchIdOf(path: string): string | null {
  return idUnder(path, "watch");
}

export function duosIdOf(path: string): string | null {
  return idUnder(path, "duos");
}

/** R2117000: the board routes, whose moves the animation queue owns (practice's setup shares its game's path). */
function isBoardPath(path: string): boolean {
  return (
    matchIdOf(path) !== null ||
    watchIdOf(path) !== null ||
    duosIdOf(path) !== null ||
    path === paths.practice ||
    path === paths.hotseat ||
    path === paths.duosDev
  );
}

// Sign-in entry states: `/login` permits only exact expiry and password-reset tokens, so URL input cannot
// carry a destination, message or address (B35, R193). A sign-in always lands on a `paths` value.

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

/**
 * #1991: "Practice with this deck" opens practice's setup with the saved deck at that 1-based place in
 * `GET /api/decks`'s list picked (`practice/decks.ts`); a saved deck never starts a game by itself.
 */
export function practiceDeckPath(place: number): string {
  return `${paths.practice}?deck=saved:${String(place)}`;
}

/** R2118000: the command menu's New deck: the workshop makes and opens one, then drops it from the address. */
export function newDeckPath(): string {
  return `${paths.decks}?new=deck`;
}

export function wantsNewDeck(search: string): boolean {
  return queryValue(search, "new") === "deck";
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
