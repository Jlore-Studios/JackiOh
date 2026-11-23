// A card's flavour line and artist credit (R660), read from the sidecar beside the catalog
// (`@jackioh/cards/flavour.json`, which the aliases resolve to crates/cards/flavour.json): the bundle
// carries the words, and no server is asked. Presentation only (CLAUDE.md rule 7). The sidecar is
// keyed by catalog id, so the hidden sentinel and a match-made definition (a Fuse's, R77) have no
// entry and nothing is shown for them.
//
// A card's flavour line and its artist are words about the card, not the card, so an edit to them
// changes no rule, no def and no patch. They are written in each card's entry of
// crates/cards/card-text.yaml (#2498), which crates/tools/tests/export_card_text.rs turns into
// `flavour.json`; its `"//"` entry says so and is dropped here. The Rust catalog checks hold the file
// to its contract; `flavour.test.tsx` also proves the lines speak no rules words.
//
// #4435: the sidecar arrives in its own chunk beside the engine module, on demand, so the first
// load carries none of it: `main.tsx` starts `loadFlavour()` beside the module, and an inspect
// view opened before it arrives gains its line when it does.

import { useSyncExternalStore } from "react";

/** One card's sidecar entry. Either field may be absent; an artist arrives with its art. */
export type CardFlavour = { readonly flavour?: string; readonly artist?: string };

/** A flavour line's longest length, in characters: one or two short lines under the rules. */
export const FLAVOUR_MAX_CHARS = 120;

/** An artist credit's longest length, in characters. */
export const ARTIST_MAX_CHARS = 60;

type FlavourTable = Readonly<Record<string, CardFlavour>>;

const EMPTY: FlavourTable = {};

let cached: FlavourTable | null = null;
let pendingLoad: Promise<FlavourTable> | null = null;
const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of [...listeners]) {
    try {
      listener();
    } catch {
      // One listener's failure must not stop others.
    }
  }
}

function subscribe(callback: () => void): () => void {
  listeners.add(callback);
  return () => {
    listeners.delete(callback);
  };
}

/** The sidecar once `loadFlavour()` has filled it, else null: every lookup reads English until then. */
export function flavourNow(): FlavourTable | null {
  return cached;
}

/**
 * #4435: loads the sidecar once, in its own chunk beside the engine module. A failed download is
 * not cached, so the next call tries again. Idempotent.
 */
export function loadFlavour(): Promise<FlavourTable> {
  if (cached !== null) {
    const held = cached;
    return Promise.resolve(held);
  }
  if (pendingLoad !== null) return pendingLoad;
  pendingLoad = import("@jackioh/cards/flavour.json")
    .then((mod) => {
      const { "//": _generated, ...entries } = mod.default;
      cached = entries;
      pendingLoad = null;
      notify();
      return cached;
    })
    .catch((cause: unknown) => {
      pendingLoad = null;
      throw cause;
    });
  return pendingLoad;
}

/** The sidecar, starting its load on first mount: a view opened early gains its line when it lands. */
export function useFlavour(): FlavourTable | null {
  if (cached === null) {
    void loadFlavour().catch(() => {});
  }
  return useSyncExternalStore(subscribe, flavourNow, () => null);
}

/** Install or clear the table. Tests only. */
export function setFlavourForTests(table: FlavourTable | null): void {
  cached = table;
  pendingLoad = table !== null ? Promise.resolve(table) : null;
  notify();
}

/** A held flavour load, settled by the test. */
export type DeferredFlavour = {
  /** Install the table and let the held load resolve. */
  resolve(table: FlavourTable): void;
  /** Fail the held load, clearing it so the next `loadFlavour()` tries again. */
  reject(err: unknown): void;
};

/** Hold `loadFlavour()` back until the test settles it. Tests only. */
export function deferFlavourForTests(): DeferredFlavour {
  cached = null;
  let res!: (table: FlavourTable) => void;
  let rej!: (err: unknown) => void;
  pendingLoad = new Promise<FlavourTable>((resolve, reject) => {
    res = resolve;
    rej = reject;
  });
  return {
    resolve(table: FlavourTable) {
      cached = table;
      notify();
      res(table);
    },
    reject(err: unknown) {
      pendingLoad = null;
      rej(err);
    },
  };
}

/** The card's entry when it has a flavour line or an artist, else null. */
export function flavourFor(defId: string, sidecar: FlavourTable = flavourNow() ?? EMPTY): CardFlavour | null {
  if (!Object.hasOwn(sidecar, defId)) return null;
  const entry = sidecar[defId];
  if (entry === undefined || (entry.flavour === undefined && entry.artist === undefined)) return null;
  return entry;
}
