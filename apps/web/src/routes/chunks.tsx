// #2116, R2116000: every lazy screen's `import()` in one map, shared by `main.tsx`'s route
// table and `prefetchRoute(path)`, so a prefetch and the route make one request.
//
// `Screen` exists because a plain `lazy()` shows the fallback once, even over an import that has
// already resolved: once a chunk's code is in, its screen draws at once with no loading panel.
//
// None of this is a rule (CLAUDE.md rule 7): only code is fetched, never game state.

import { lazy, useState, type ComponentType, type ReactElement } from "react";

import { duosIdOf, matchIdOf, paths, seriesIdOf, watchIdOf } from "../net/navigate.ts";
import type { InviteRouteProps } from "./invite.tsx";
import { loadWasm } from "../wasm/index.ts";

const DEV_ONLY = import.meta.env.MODE !== "production";

type ScreenModule<P> = { default: ComponentType<P> };

export type RouteChunk<P extends object> = {
  /** One import, shared; a failed one is forgotten, so the next call asks again. */
  load: () => Promise<ScreenModule<P>>;
  /** Drawn at once when its code is in, else through `lazy()`. */
  Screen: (props: P) => ReactElement;
};

export function routeChunk<P extends object>(importScreen: () => Promise<ScreenModule<P>>): RouteChunk<P> {
  let pending: Promise<ScreenModule<P>> | null = null;
  let arrived: ComponentType<P> | null = null;
  const load = (): Promise<ScreenModule<P>> => {
    pending ??= importScreen().then(
      (module) => {
        arrived = module.default;
        return module;
      },
      (cause: unknown) => {
        pending = null;
        throw cause;
      },
    );
    return pending;
  };
  const Lazy = lazy(load);
  function Screen(props: P): ReactElement {
    // Chosen once per mount: a screen that swapped from `Lazy` to its module mid-life would remount.
    const [Shown] = useState<ComponentType<P>>(() => arrived ?? Lazy);
    return <Shown {...props} />;
  }
  return { load, Screen };
}

export const routeChunks = {
  // Only outside production: a production build serves NotFound at /dev/hotseat, and with the import
  // behind DEV_ONLY the bundler leaves the hotseat chunk (and the developer text in it) out entirely.
  hotseat: DEV_ONLY ? routeChunk(() => import("./dev/hotseat.tsx")) : null,
  // #936 B65: the four-seat Duos hotseat, outside production only, as the 1v1 hotseat is.
  duosDev: DEV_ONLY ? routeChunk(() => import("../duos/DevDuosRoute.tsx")) : null,
  // #936 B61–B64: the networked co-op board.
  duos: routeChunk(() => import("../duos/DuosRoute.tsx")),
  // R768000: the recent games and their replays.
  replays: routeChunk(() => import("./replays.tsx")),
  login: routeChunk(() => import("./login.tsx")),
  resetPassword: routeChunk(() => import("./reset-password.tsx")),
  // The defaulted props infer no `P`, so it is named (its `account` is optional).
  invite: routeChunk<InviteRouteProps>(() => import("./invite.tsx")),
  // Same: `portraitsGated` is optional.
  decks: routeChunk<{ portraitsGated?: boolean }>(() => import("./decks.tsx")),
  play: routeChunk(() => import("./play.tsx")),
  match: routeChunk(() => import("./match.tsx")),
  series: routeChunk(() => import("./series.tsx")),
  practice: routeChunk(() => import("./practice.tsx")),
  // #2705: the Dungeon Run and its stats, played offline on the engine in the browser.
  dungeon: routeChunk(() => import("../dungeon/DungeonPage.tsx")),
  dungeonStats: routeChunk(() => import("../dungeon/DungeonStatsPage.tsx")),
  privacy: routeChunk(() => import("./privacy.tsx")),
  terms: routeChunk(() => import("./terms.tsx")),
  purchases: routeChunk(() => import("./purchases.tsx")),
  accessibility: routeChunk(() => import("./accessibility.tsx")),
  ai: routeChunk(() => import("./ai.tsx")),
  copyright: routeChunk(() => import("./copyright.tsx")),
  patchNotes: routeChunk(() => import("./patch-notes.tsx")),
  almanac: routeChunk(() => import("./almanac.tsx")),
  leaderboard: routeChunk(() => import("./leaderboard.tsx")),
  collection: routeChunk(() => import("./collection.tsx")),
  shop: routeChunk(() => import("./shop.tsx")),
  stats: routeChunk(() => import("./stats.tsx")),
  creator: routeChunk(() => import("./creator.tsx")),
  // #936: Play together and a friend's game watched.
  together: routeChunk(() => import("../together/TogetherRoute.tsx").then((module) => ({ default: module.TogetherRoute }))),
  watch: routeChunk(() => import("../together/WatchRoute.tsx").then((module) => ({ default: module.WatchRoute }))),
};

export type RouteChunkName = keyof typeof routeChunks;

/** Intent toward these starts the engine too. */
const ENGINE_CHUNKS: ReadonlySet<RouteChunkName> = new Set<RouteChunkName>([
  "practice",
  "play",
  "match",
  "series",
  "duos",
  "watch",
  "dungeon",
  "dungeonStats",
]);

/** The chunk drawing `path` (query ignored), or null (the landing, `/account`, a 404). */
export function chunkAt(path: string): RouteChunkName | null {
  const pathname = new URL(path, window.location.origin).pathname.replace(/\/+$/u, "") || "/";
  if (matchIdOf(pathname) !== null) return "match";
  if (seriesIdOf(pathname) !== null) return "series";
  if (watchIdOf(pathname) !== null) return "watch";
  if (duosIdOf(pathname) !== null) return "duos";
  for (const name of Object.keys(routeChunks) as RouteChunkName[]) {
    if (paths[name] === pathname) return name;
  }
  return null;
}

/** Chromium's Network Information field, absent from TypeScript's DOM types. */
type SaveDataNavigator = Navigator & { connection?: { saveData?: boolean } };

function savingData(): boolean {
  return (navigator as SaveDataNavigator).connection?.saveData === true;
}

/** Each chunk once per page load. */
const prefetched = new Set<RouteChunkName>();

/** Test seam: a new page, which has prefetched nothing yet. */
export function resetRoutePrefetchForTests(): void {
  prefetched.clear();
}

/** Starts the code of the screen at `path` (and the engine, for a game's screen); a failure is the route's to report. */
export function prefetchRoute(path: string): void {
  if (savingData()) return;
  const name = chunkAt(path);
  if (name === null || prefetched.has(name)) return;
  prefetched.add(name);
  void routeChunks[name]?.load().catch(() => undefined);
  if (ENGINE_CHUNKS.has(name)) void loadWasm().catch(() => undefined);
}

/**
 * R2630001: a gated screen's own code, and the engine's module when the screen calls it, asked for as
 * its gate mounts, beside `/api/auth/me` rather than after it. The player is going there, so saving
 * data does not hold either back, as it holds back a prefetch on intent. Both loads are shared.
 */
export function startScreen(path: string, engine: boolean): void {
  const name = chunkAt(path);
  if (name !== null) void routeChunks[name]?.load().catch(() => undefined);
  if (engine) void loadWasm().catch(() => undefined);
}

const INTENT_EVENTS = ["pointerenter", "focus", "touchstart"] as const;

/** A same-origin link's path, or a navigating button's `data-route`. */
function routeOf(target: EventTarget | null): string | null {
  if (!(target instanceof Element)) return null;
  const control = target.closest("a[href], [data-route]");
  if (control === null) return null;
  if (control instanceof HTMLElement && control.dataset.route !== undefined) return control.dataset.route;
  if (!(control instanceof HTMLAnchorElement)) return null;
  const url = new URL(control.href);
  return url.origin === window.location.origin ? url.pathname : null;
}

/** Prefetches the route of every control the player shows intent toward (capture phase: `pointerenter` and `focus` do not bubble). Returns the stop. */
export function watchRouteIntent(root: Document): () => void {
  const onIntent = (event: Event): void => {
    const path = routeOf(event.target);
    if (path !== null) prefetchRoute(path);
  };
  for (const type of INTENT_EVENTS) root.addEventListener(type, onIntent, { capture: true, passive: true });
  return () => {
    for (const type of INTENT_EVENTS) root.removeEventListener(type, onIntent, { capture: true });
  };
}
