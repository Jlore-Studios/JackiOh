// Dev decks for hotseat (BUILD §2, SPEC §8). Query over catalog; legality authority is engine (§2.6).

import { setShips, type CardCost, type CardDef, type CardDefs, type SetName } from "@jackioh/shared";

/** SPEC §2.6 (BUILD §2): re-exported from engine config constants. */
import { DECK_SIZE, MAX_COPIES } from "@jackioh/engine/config";

export { DECK_SIZE, MAX_COPIES };

export type DeckSource = {
  id: string;
  label: string;
  /** The card ids this deck resolves to, in library order. May be shorter than `DECK_SIZE`. */
  resolve(catalog: CardDefs): string[];
};

export type ResolvedDeck = { deck: string[] } | { error: string };

/**
 * R65 read out of play: an X-cost card's printed cost is 0 and an embiggen card's is its base
 * price. `cheap20` sorts on this, so it agrees with the engine's own out-of-play cost.
 */
export function printedCost(cost: CardCost): number {
  if (typeof cost === "number") return cost;
  if (cost === "X") return 0;
  return cost.base;
}

/**
 * `validateDeck`'s L3, mirrored: not a Token (`def.token || def.tags.includes("Token")`, §2.6) and
 * not a card of a set that has not shipped (R1420), unless the engine was told to preview that set
 * (`preview`, the E2E injection's, `EnginePort.previewSets`). A deck built from anything else is refused.
 */
function deckable(def: CardDef, preview: readonly SetName[] = []): boolean {
  return !def.token && !def.tags.includes("Token") && (setShips(def.set) || preview.includes(def.set));
}

/**
 * SPEC §5 index order. Indices are mostly numeric strings ("1".."100") but tokens use "51.1" and
 * "T-rush", so a numeric compare with a lexicographic fallback and an id tie-break keeps the order
 * total — a deck must resolve the same way on every run or the M5-T3 replay hash moves.
 */
export function byIndex(a: CardDef, b: CardDef): number {
  const na = Number.parseFloat(a.index);
  const nb = Number.parseFloat(b.index);
  const aNum = Number.isFinite(na);
  const bNum = Number.isFinite(nb);
  if (aNum && bNum && na !== nb) return na - nb;
  if (aNum !== bNum) return aNum ? -1 : 1;
  if (a.index !== b.index) return a.index < b.index ? -1 : 1;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

function candidates(catalog: CardDefs): CardDef[] {
  return Object.values(catalog)
    .filter((def) => deckable(def))
    .sort(byIndex);
}

/** Dev decks satisfying DECK_SIZE and MAX_COPIES; ids kept for #1705 compatibility. */
export const DEV_DECKS: readonly DeckSource[] = [
  {
    id: "first20",
    label: `First ${String(DECK_SIZE)} (by §5 index)`,
    resolve: (catalog) =>
      candidates(catalog)
        .slice(0, DECK_SIZE)
        .map((def) => def.id),
  },
  {
    id: "cheap20",
    label: `Cheapest ${String(DECK_SIZE)} (by printed cost)`,
    resolve: (catalog) =>
      [...candidates(catalog)]
        // `candidates` is already in index order and `sort` is stable, so equal costs keep it.
        .sort((a, b) => printedCost(a.cost) - printedCost(b.cost))
        .slice(0, DECK_SIZE)
        .map((def) => def.id),
  },
];

export const DEFAULT_DECK_ID = "first20";

export const DECK_IDS: readonly string[] = DEV_DECKS.map((source) => source.id);

export function deckSource(id: string): DeckSource | undefined {
  return DEV_DECKS.find((source) => source.id === id);
}

/** Formats refusal message for invalid deck size (§2.6, R184), duplicate, missing or unreleased card (R1420). */
function refusal(deck: readonly string[], catalog: CardDefs, id: string, size: number, preview: readonly SetName[]): string | null {
  if (deck.length !== size) {
    return size === DECK_SIZE
      ? `deck "${id}" holds ${deck.length} cards; a deck is exactly ${DECK_SIZE} (§2.6 L2)`
      : `deck "${id}" holds ${deck.length} cards; its seat's handicap wants exactly ${size} (R184)`;
  }
  if (new Set(deck).size !== deck.length) {
    return `deck "${id}" repeats a card id; at most ${MAX_COPIES} copy of each (§2.6 L3)`;
  }
  for (const defId of deck) {
    const def = catalog[defId];
    if (def === undefined) return `deck "${id}": "${defId}" is not in the catalog (§9.4 L6)`;
    if (!deckable(def, preview)) {
      return setShips(def.set) || preview.includes(def.set)
        ? `deck "${id}": "${defId}" is a Token card and cannot be in a deck (§2.6 L3)`
        : `deck "${id}": "${defId}" is a card of ${def.set}, which has not shipped yet (§2.6 L3, R1420)`;
    }
  }
  return null;
}

/** Resolves deck id against catalog; supports E2E overrides, handicap deck size (R184), and preview sets (R1420). */
export function resolveDeck(
  id: string,
  catalog: CardDefs,
  overrides?: Readonly<Record<string, readonly string[]>>,
  size: number = DECK_SIZE,
  preview: readonly SetName[] = [],
): ResolvedDeck {
  const injected = overrides?.[id];
  if (injected !== undefined) {
    const deck = [...injected];
    const bad = refusal(deck, catalog, id, size, preview);
    return bad === null ? { deck } : { error: bad };
  }

  const source = deckSource(id);
  if (source === undefined) {
    return {
      error: `unknown deck "${id}". Dev decks: ${DECK_IDS.join(", ")}.`,
    };
  }

  const pool = candidates(catalog);
  if (pool.length === 0) {
    return {
      error:
        `deck "${id}" cannot be built: the card catalog is empty. ` +
        `The engine's catalog (crates/cards, in the WebAssembly module) handed the client no ` +
        `definitions, and the client never invents card ids.`,
    };
  }
  if (pool.length < DECK_SIZE) {
    return {
      error: `deck "${id}" needs ${DECK_SIZE} non-Token cards; the catalog offers ${pool.length}.`,
    };
  }

  const deck = source.resolve(catalog);
  const bad = refusal(deck, catalog, id, size, []);
  return bad === null ? { deck } : { error: bad };
}

/** Resolves both seats at once: [p1, p2] or first error; supports custom sizes and preview sets (R1420). */
export function resolveDecks(
  a: string,
  b: string,
  catalog: CardDefs,
  overrides?: Readonly<Record<string, readonly string[]>>,
  sizes: readonly [number, number] = [DECK_SIZE, DECK_SIZE],
  preview: readonly SetName[] = [],
): { decks: [string[], string[]] } | { error: string } {
  const first = resolveDeck(a, catalog, overrides, sizes[0], preview);
  if ("error" in first) return { error: first.error };
  const second = resolveDeck(b, catalog, overrides, sizes[1], preview);
  if ("error" in second) return { error: second.error };
  return { decks: [first.deck, second.deck] };
}
