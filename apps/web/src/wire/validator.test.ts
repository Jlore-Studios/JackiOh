// The validator boundary (`./validator.ts`, SPEC §9.4): loadout, draft and import rules
// the Rust validator shares with the server. The deckbuilder suites prove the refusals;
// here the boundary's own plumbing is pinned: the passthrough shapes a mutant would change.

import { describe, expect, it, vi } from "vitest";

import { catalogVersion, registeredCatalog } from "../wasm/index.ts";
import { DECK_SIZE } from "./engineConfig.ts";
import {
  checkDeckDraft,
  checkImportRoom,
  checkTrioDraft,
  normalizeName,
  practiceDeckIssues,
  trioConflicts,
  validateDeck,
  validateLoadout,
} from "./validator.ts";

const CORE = Array.from({ length: DECK_SIZE }, (_, i) => `core-${String(i + 1).padStart(3, "0")}`);

/** The module's own catalog, as a page that judges against it sends it. */
const catalog = () => ({ version: catalogVersion(), cards: registeredCatalog() });

/** One copy of every id, owned. */
const owning = (ids: readonly string[]) => Object.fromEntries(ids.map((id) => [id, 1]));

const rules = (issues: readonly { rule: string }[]) => issues.map((issue) => issue.rule);

describe("validateDeck and validateLoadout (§9.4, R253)", () => {
  it("R253 passes a full owned Best-of-1 deck the catalog holds, sending each card's definition", () => {
    expect(validateDeck({ deck: { cards: CORE }, catalog: catalog(), collection: owning(CORE) })).toEqual({ ok: true });
  });

  it("R253 names a card the catalog lacks under L6 and still counts it toward L2", () => {
    const verdict = validateDeck({ deck: { cards: ["core-999"] }, catalog: catalog(), collection: owning(["core-999"]) });
    expect(verdict.ok ? [] : rules(verdict.errors)).toEqual(["L2", "L6"]);
  });

  it("holds one deck as a trio to L1 alone when it is legal and owned", () => {
    const verdict = validateLoadout({ decks: [{ cards: CORE }], catalog: catalog(), collection: owning(CORE) });
    expect(verdict.ok ? [] : rules(verdict.errors)).toEqual(["L1"]);
  });
});

describe("§9.9 practiceDeckIssues: the rules a practice game holds a deck to", () => {
  it("keeps L2, L3 and L6 and never ownership (L5)", () => {
    const issues = practiceDeckIssues({ name: "Short", cards: ["core-001", "core-001", "core-999"] });
    expect(rules(issues)).toEqual(["L2", "L3", "L6"]);
  });

  it("finds nothing to stop a full legal deck that the player owns none of", () => {
    expect(practiceDeckIssues({ cards: CORE })).toEqual([]);
  });
});

describe("normalizeName", () => {
  it("trims the surrounding whitespace", () => {
    expect(normalizeName("  hello  ")).toBe("hello");
  });

  it("leaves an empty name empty", () => {
    expect(normalizeName("")).toBe("");
  });
});

describe("trioConflicts", () => {
  it("names a card two decks share, in first-appearance order", () => {
    expect(trioConflicts([{ cards: ["core-001", "core-002"] }, { cards: ["core-002", "core-003"] }])).toEqual([
      { cardId: "core-002", decks: [0, 1] },
    ]);
  });

  it("finds no conflict where decks share nothing", () => {
    expect(trioConflicts([{ cards: ["core-001"] }, { cards: ["core-002"] }])).toEqual([]);
  });
});

describe("checkDeckDraft", () => {
  it("accepts a named draft whose cards are all deckable", () => {
    expect(
      checkDeckDraft({ name: "My Deck", cards: [], isDeckable: () => true, nameMaxLength: 30 }),
    ).toEqual([]);
  });

  it("refuses a blank name", () => {
    const issues = checkDeckDraft({ name: "   ", cards: [], isDeckable: () => true, nameMaxLength: 30 });
    expect(issues.length).toBeGreaterThan(0);
  });

  it("R250 D3 names each card the caller says no deck can hold, and only those", () => {
    const issues = checkDeckDraft({
      name: "My Deck",
      cards: ["core-001", "nope", "core-002"],
      isDeckable: (cardId) => cardId !== "nope",
      nameMaxLength: 30,
    });
    expect(issues.map(({ rule, cardId }) => ({ rule, cardId }))).toEqual([{ rule: "D3", cardId: "nope" }]);
  });

  it("R641 D5 refuses a portrait the caller does not know and passes one it does", () => {
    const draft = { name: "My Deck", cards: [], isDeckable: () => true, nameMaxLength: 30 };
    expect(rules(checkDeckDraft({ ...draft, portrait: "nobody", isPortrait: () => false }))).toEqual(["D5"]);
    expect(checkDeckDraft({ ...draft, portrait: "somebody", isPortrait: () => true })).toEqual([]);
  });

  it("R641 D5 takes a portrait as given when the caller cannot answer for it", () => {
    expect(checkDeckDraft({ name: "My Deck", cards: [], isDeckable: () => true, portrait: "anyone", nameMaxLength: 30 })).toEqual([]);
  });

  it("R641 D5 asks about no portrait when the draft has none (null, the default, or absent)", () => {
    const isPortrait = vi.fn(() => false);
    const draft = { name: "My Deck", cards: [], isDeckable: () => true, isPortrait, nameMaxLength: 30 };
    expect(checkDeckDraft({ ...draft, portrait: null })).toEqual([]);
    expect(checkDeckDraft(draft)).toEqual([]);
    expect(isPortrait).not.toHaveBeenCalled();
  });
});

describe("checkTrioDraft", () => {
  it("accepts three empty slots under a name", () => {
    expect(checkTrioDraft({ name: "Trio", deckIds: [null, null, null], nameMaxLength: 30 })).toEqual([]);
  });
});

describe("checkImportRoom", () => {
  it("fits an import under both caps and refuses one over them", () => {
    expect(checkImportRoom({ saved: { decks: 1, trios: 0 }, limits: { decks: 10, trios: 5 }, adding: { decks: 2, trios: 1 } })).toEqual({
      ok: true,
    });
    const over = checkImportRoom({ saved: { decks: 9, trios: 0 }, limits: { decks: 10, trios: 5 }, adding: { decks: 2, trios: 0 } });
    expect(over.ok).toBe(false);
  });
});
