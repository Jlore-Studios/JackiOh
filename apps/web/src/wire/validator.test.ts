// The validator boundary (`./validator.ts`, SPEC §9.4): loadout, draft and import rules
// the Rust validator shares with the server. The deckbuilder suites prove the refusals;
// here the boundary's own plumbing is pinned: the passthrough shapes a mutant would change.

import { describe, expect, it } from "vitest";

import {
  checkDeckDraft,
  checkImportRoom,
  checkTrioDraft,
  normalizeName,
  trioConflicts,
} from "./validator.ts";

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
