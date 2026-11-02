// R2127000: the browser's Back, and Android's back gesture, close the topmost open sheet or dialog
// before they leave the screen. Each overlay a player dismisses is a layer while it is open
// (`useBackLayer`), newest on top; a prompt and the mulligan never are. Where the browser has
// `CloseWatcher` the top layer holds one; elsewhere the first layer pushes one history entry copying
// the address, which a layer closed another way leaves spent. None of it reaches the leave guard
// (R1754000) or a view transition (R2117000). Nothing reads the window at load: Vite's config loader
// imports `navigate.ts`, which imports this.

import { useEffect, useLayoutEffect, useRef } from "react";

/** The `history.state` key that marks the layers' entry; it holds the address of the entry under it. */
const BACK_ENTRY = "jackiohBackEntry";

/** What is used of the browser's `CloseWatcher`, which lib.dom does not declare. */
type Watcher = { onclose: ((event: Event) => void) | null; destroy(): void };
type WatcherClass = new () => Watcher;
type Layer = { close: () => void; closable: () => boolean };

/** The open layers, oldest first. */
const layers: Layer[] = [];
let held: { layer: Layer; watcher: Watcher } | null = null;
/**
 * The layers' entry while current, live or spent: its address and the one under it, the same until a
 * replacing move takes it over (`moveOverBackEntry`).
 */
let entry: { at: string; under: string } | null = null;
/** The next `popstate` is this module's own step back off a copied entry that Back or Forward landed on. */
let skipNext = false;
let listening = false;
/** Each `popstate`'s answer: `navigate.ts` asks too, and whichever listener runs first decides it. */
const answered = new WeakMap<Event, boolean>();

function here(): string {
  return window.location.href;
}

function underOf(state: unknown): string | null {
  if (typeof state !== "object" || state === null) return null;
  const under: unknown = Reflect.get(state, BACK_ENTRY);
  return typeof under === "string" ? under : null;
}

function watcherClass(): WatcherClass | null {
  const found: unknown = Reflect.get(globalThis, "CloseWatcher");
  return typeof found === "function" ? (found as WatcherClass) : null;
}

function sync(): void {
  const top = layers.at(-1);
  const Made = watcherClass();
  if (Made !== null) {
    if (held !== null && held.layer !== top) {
      held.watcher.destroy();
      held = null;
    }
    if (top === undefined || held !== null) return;
    const watcher = new Made();
    watcher.onclose = () => {
      if (held?.watcher === watcher) held = null;
      answer(top);
    };
    held = { layer: top, watcher };
    return;
  }
  const under = underOf(window.history.state);
  // Something else replaced the entry (a room link read off the address, a test's `at`).
  if (entry !== null && under === null) entry = null;
  if (top === undefined) return;
  // A spent entry is reused, the one this page load made or one it reloaded on.
  if (under !== null) {
    entry ??= { at: here(), under };
    return;
  }
  const at = here();
  window.history.pushState({ [BACK_ENTRY]: at }, "", at);
  entry = { at, under: at };
}

/** One Back for `layer`: it closes, or, while it cannot (the pack scene's open is out), it stays and answers the next. */
function answer(layer: Layer): void {
  if (layers.at(-1) !== layer) return;
  if (layer.closable()) layer.close();
  else sync();
}

function answerPop(): boolean {
  if (skipNext) {
    // Where the step back off a copy lands: the screen already shown there, or, after a Back from a
    // later page, a move as any other, which the leave guard may hold.
    skipNext = false;
    return false;
  }
  const landed = underOf(window.history.state);
  if (landed !== null) {
    // Back or Forward landed on a layers' entry. A copy of the entry under it is stepped off, so an
    // overlay the address opens (the Almanac's card) never opens again; one a replacing move took
    // over is that page's own entry, and the move to it is shown as any other.
    entry = null;
    if (here() === landed) {
      skipNext = true;
      window.history.back();
      return true;
    }
    entry = { at: here(), under: landed };
    return false;
  }
  if (entry === null) return false;
  const { at, under } = entry;
  entry = null;
  // A Back of more than one step. The entry under holds the entry's own address once a Back stepped past it.
  if (here() !== under && here() !== at) return false;
  // A replacing move took the entry over: its page takes back the entry under it.
  if (at !== under) window.history.replaceState(null, "", at);
  const top = layers.at(-1);
  if (top === undefined) window.history.back(); // a spent entry: on past the address it copied
  else answer(top);
  return true;
}

/**
 * R2127000: whether a `popstate` was the layers' own: a Back that closed the top layer, or a step
 * over their entry. `navigate.ts` asks before anything else, and tells no screen of one.
 */
export function popBackLayer(event: Event): boolean {
  const known = answered.get(event);
  if (known !== undefined) return known;
  const mine = answerPop();
  answered.set(event, mine);
  return mine;
}

/**
 * R2127000: a `navigate` while the layers' entry is the current one. A replacing move takes it over
 * and keeps it theirs; any other takes its place while it is a copy, and goes after it once a
 * replacing move made it a page's own. Returns true when it made the move.
 */
export function moveOverBackEntry(path: string, replace: boolean): boolean {
  const under = underOf(window.history.state);
  entry = null;
  if (under === null) return false;
  if (replace) {
    window.history.replaceState({ [BACK_ENTRY]: under }, "", path);
    entry = { at: here(), under };
    return true;
  }
  if (here() !== under) return false;
  window.history.replaceState(null, "", path);
  return true;
}

/** While a close watcher is held, Escape is left to the layers' own keys, so one Escape never closes two. */
function keepEscape(event: KeyboardEvent): void {
  if (event.key === "Escape" && held !== null) event.preventDefault();
}

function openLayer(layer: Layer): () => void {
  if (!listening) {
    window.addEventListener("popstate", popBackLayer);
    window.addEventListener("keydown", keepEscape, true);
    listening = true;
  }
  layers.push(layer);
  sync();
  return () => {
    const at = layers.indexOf(layer);
    if (at !== -1) layers.splice(at, 1);
    sync();
  };
}

/**
 * R2127000: the mounted overlay is the top layer until another opens; Back calls `close`, as its Close
 * button does, so the focus goes back. While `closable` is false Back closes nothing.
 */
export function useBackLayer(close: () => void, closable = true): void {
  const latest = useRef({ close, closable });
  useLayoutEffect(() => {
    latest.current = { close, closable };
  });
  useEffect(
    () =>
      openLayer({
        close: () => {
          latest.current.close();
        },
        closable: () => latest.current.closable,
      }),
    [],
  );
}

/**
 * R2127000: `useBackLayer` as an element, for an overlay drawn inline while its own state says it is
 * open: mounted with the overlay, it is a layer exactly while the overlay shows, and with it shut Back
 * leaves the screen as before.
 */
export function BackLayer({ onClose }: { onClose: () => void }): null {
  useBackLayer(onClose);
  return null;
}
