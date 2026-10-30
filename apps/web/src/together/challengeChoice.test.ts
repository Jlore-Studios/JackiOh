// R915005: the picks the challenge dialog, the challenge toast and the co-op panel start on pass over a
// deck that names a custom card, and over a trio holding one: they play only in practice.

import { describe, expect, it } from "vitest";

import type { SavedDeck, SavedTrio } from "../net/api.ts";
import { deckPracticeOnly, firstDeck, firstTrio, pickDeck, pickTrio, practiceOnlyLabel, trioPracticeOnly } from "./challengeChoice.ts";

const IMPS: SavedDeck = {
  id: "d-imps",
  name: "Imps",
  cards: ["custom-ember-imp"],
  catalogVersion: "v1",
  portrait: null,
  createdAt: 0,
  updatedAt: 0,
};
const CORE: SavedDeck = { ...IMPS, id: "d-core", name: "Core", cards: [] };
const IMP_TRIO: SavedTrio = { id: "t-imps", name: "Imp trio", deckIds: ["d-core", "d-imps", "d-core"], createdAt: 0, updatedAt: 0 };
const CORE_TRIO: SavedTrio = { id: "t-core", name: "Core trio", deckIds: ["d-core", "d-core", "d-core"], createdAt: 0, updatedAt: 0 };

describe("R915005 the online picks pass over a deck or trio with a custom card", () => {
  it("R915005 deckPracticeOnly and trioPracticeOnly name a deck with a custom card and a trio holding one", () => {
    expect(deckPracticeOnly(IMPS)).toBe(true);
    expect(deckPracticeOnly(CORE)).toBe(false);
    expect(trioPracticeOnly(IMP_TRIO, [IMPS, CORE])).toBe(true);
    expect(trioPracticeOnly(CORE_TRIO, [IMPS, CORE])).toBe(false);
    // A deck the list does not hold is not known to be one.
    expect(trioPracticeOnly(IMP_TRIO, [])).toBe(false);
  });

  it("R915005 firstDeck and pickDeck never pick a practice-only deck, even the remembered one", () => {
    expect(firstDeck([IMPS, CORE])).toBe(CORE);
    expect(pickDeck([IMPS, CORE], "d-imps")).toBe(CORE);
    expect(firstDeck([IMPS])).toBeNull();
    expect(pickDeck([IMPS], "d-imps")).toBeNull();
  });

  it("R915005 firstTrio and pickTrio never pick a trio holding one", () => {
    expect(firstTrio([IMP_TRIO, CORE_TRIO], [IMPS, CORE])).toBe(CORE_TRIO);
    expect(pickTrio([IMP_TRIO, CORE_TRIO], "t-imps", [IMPS, CORE])).toBe(CORE_TRIO);
    expect(pickTrio([IMP_TRIO], "t-imps", [IMPS, CORE])).toBeNull();
  });

  it("R915005 practiceOnlyLabel marks the name", () => {
    expect(practiceOnlyLabel("Imps")).toBe("Imps (practice only)");
  });
});
