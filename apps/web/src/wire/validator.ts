// `@jackioh/validator` for the web client (docs/v0.3.0/SURFACE.md §10.4): the loadout rules L1–L6,
// the draft rules D1–D5 and T1–T3, and the import room check (SPEC §9.4, R250, R252, R253, R340,
// R641), with the TypeScript signatures the deck builder was written against.
//
// §9.4 asks for "one validator module shared by client and server, at save and again at queue". It
// is `crates/engine/src/validator.rs`, which the server calls directly and the client calls through
// WebAssembly (`../wasm`'s `validator`), so the builder's verdict and the server's are one
// computation and every message is the same sentence on both sides. Nothing here decides a rule: each
// function below turns its arguments into JSON, calls the Rust function of the same name, and returns
// its answer. The page loads the module before its first render (`main.tsx`), so these calls are
// synchronous, as they were.
//
// Two things the boundary changes, neither of them a rule:
//   - `checkDeckDraft` takes two predicates, and a function cannot cross into WebAssembly. They are
//     answered here first, for exactly the arguments the rules ask about (each distinct card id, the
//     one portrait), and the answers cross instead.
//   - A snapshot's `cards` is the whole catalog, and the rules only ever look up the ids the decks
//     hold. Only those definitions cross, so a check costs the decks' size and not the catalog's.

import { validator } from "../wasm/index.ts";

import type { CardDefs } from "./index.ts";

/** §9.4: exactly 3 decks per loadout (Rust's `validator::LOADOUT_DECKS`). */
export const LOADOUT_DECKS = 3;

/** R252: a trio is §9.4's loadout of three decks, so it holds `LOADOUT_DECKS` of them. */
export const TRIO_DECKS = LOADOUT_DECKS;

export type CardId = string;

export type LoadoutDeck = {
  /** Optional builder label; messages fall back to `Deck <n>`. */
  name?: string;
  cards: readonly CardId[];
};

/** The static, versioned catalog snapshot (§9.4) a loadout is checked against. */
export type CatalogSnapshot = {
  version: string;
  cards: CardDefs;
  banned?: readonly CardId[];
};

/** The profile's entitlements projected to quantities; an absent id means none owned. */
export type Collection = Readonly<Record<CardId, number>>;

export type LoadoutInput = {
  decks: readonly LoadoutDeck[];
  catalog: CatalogSnapshot;
  collection: Collection;
};

export type LoadoutRule = "L1" | "L2" | "L3" | "L4" | "L5" | "L6";

export type LoadoutError = {
  rule: LoadoutRule;
  message: string;
  /** 1-based deck index; absent on loadout-wide failures (L1, L4, L5). */
  deck?: number;
  cardId?: CardId;
};

export type LoadoutResult = { ok: true } | { ok: false; errors: readonly LoadoutError[] };

export type DeckInput = {
  deck: LoadoutDeck;
  catalog: CatalogSnapshot;
  collection: Collection;
};

/** One card that two or more decks of a trio hold, and which decks (0-based, in trio order). */
export type TrioConflict = { cardId: CardId; decks: readonly number[] };

/** R250, R641: a saved deck's structural rules. */
export type DraftRule = "D1" | "D2" | "D3" | "D4" | "D5";

/** R252: a saved trio's rules. T1 a name as D1; T2 exactly `TRIO_DECKS` slots; T3 no deck twice. */
export type TrioDraftRule = "T1" | "T2" | "T3";

export type DraftIssue<Rule extends string = DraftRule> = {
  rule: Rule;
  message: string;
  cardId?: CardId;
};

export type NameLimits = {
  /** The longest name, in characters, after trimming. The caller's config states the number. */
  nameMaxLength: number;
};

export type DeckDraftInput = {
  name: string;
  cards: readonly CardId[];
  /** Whether an id is a deckable card: in the current catalog and not a Token. */
  isDeckable: (cardId: CardId) => boolean;
  /** R641's D5: the deck's hero portrait, `null` (the default, `vanilla`) or a known id. */
  portrait?: string | null;
  /** Whether an id is a known portrait; absent, D5 has nothing to check against. */
  isPortrait?: (portrait: string) => boolean;
} & NameLimits;

export type TrioDraftInput = {
  name: string;
  /** Deck ids by slot; `null` is an empty slot, which a draft may have (R252). */
  deckIds: readonly (string | null)[];
} & NameLimits;

/** R340: what a trio import would add to a profile, against what it has and the caps it lives under. */
export type ImportRoomInput = {
  /** What the profile has saved now. */
  saved: { decks: number; trios: number };
  /** The most it may keep. */
  limits: { decks: number; trios: number };
  /** What the import would make: the code's decks, and the trio. */
  adding: { decks: number; trios: number };
};

export type ImportRoom = { ok: true } | { ok: false; decksShort: number; triosShort: number; message: string };

/** The snapshot with only the definitions of the ids these decks hold: all the rules look up. */
function snapshotFor(catalog: CatalogSnapshot, decks: readonly LoadoutDeck[]): CatalogSnapshot {
  const cards: CardDefs = {};
  for (const deck of decks) {
    for (const cardId of deck.cards) {
      const def = catalog.cards[cardId];
      if (def !== undefined && !Object.hasOwn(cards, cardId)) cards[cardId] = def;
    }
  }
  return { ...catalog, cards };
}

/** §9.4 L1–L6 over a trio's three decks, every failure at once. */
export function validateLoadout(input: LoadoutInput): LoadoutResult {
  return validator<LoadoutResult>("validateLoadout", { ...input, catalog: snapshotFor(input.catalog, input.decks) });
}

/** R253: a Best-of-3 trio is §9.4's loadout, so its rules are L1–L6 exactly. */
export const validateTrio = validateLoadout;

/** R253: L2, L3, L5 and L6 over one Best-of-1 deck. */
export function validateDeck(input: DeckInput): LoadoutResult {
  return validator<LoadoutResult>("validateDeck", { ...input, catalog: snapshotFor(input.catalog, [input.deck]) });
}

/** Every card more than one of `decks` holds, in first-appearance order (R251, R252). */
export function trioConflicts(decks: readonly { readonly cards: readonly CardId[] }[]): TrioConflict[] {
  return validator<TrioConflict[]>(
    "trioConflicts",
    decks.map((deck) => ({ cards: deck.cards })),
  );
}

/** A name as it is stored: trimmed, and every run of whitespace inside it one space. */
export function normalizeName(raw: string): string {
  return validator<string>("normalizeName", raw);
}

/** R250's D1–D4 and R641's D5, every failure at once. Empty when the draft may be saved. */
export function checkDeckDraft(input: DeckDraftInput): DraftIssue[] {
  const deckable = [...new Set(input.cards)].filter((cardId) => input.isDeckable(cardId));
  const { portrait, isPortrait } = input;
  // D5 asks about a portrait only when there is one and the caller can answer for it.
  const portraitKnown =
    portrait !== undefined && portrait !== null && isPortrait !== undefined ? isPortrait(portrait) : undefined;
  return validator<DraftIssue[]>("checkDeckDraft", {
    name: input.name,
    cards: input.cards,
    deckable,
    ...(portrait === undefined ? {} : { portrait }),
    ...(portraitKnown === undefined ? {} : { portraitKnown }),
    nameMaxLength: input.nameMaxLength,
  });
}

/** R252's T1–T3, every failure at once. Empty when the trio may be saved. */
export function checkTrioDraft(input: TrioDraftInput): DraftIssue<TrioDraftRule>[] {
  return validator<DraftIssue<TrioDraftRule>[]>("checkTrioDraft", {
    name: input.name,
    deckIds: input.deckIds,
    nameMaxLength: input.nameMaxLength,
  });
}

/** R340: whether an import fits under both caps, and the sentence when it does not. */
export function checkImportRoom(input: ImportRoomInput): ImportRoom {
  return validator<ImportRoom>("checkImportRoom", input);
}
