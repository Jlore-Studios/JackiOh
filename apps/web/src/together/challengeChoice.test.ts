// #4550: the pick helpers the lobby, the play screen and the challenge dialog share, which the play
// screen had written again as its own `defaultDeck`, `defaultTrio` and `choiceFor`.

import { describe, expect, it } from "vitest";

import { DECK_SIZE } from "@jackioh/engine/config";

import type { SavedDeck, SavedTrio, TrioSlots } from "../net/api.ts";
import { choiceOf, firstDeck, firstTrio, pickDeck, pickTrio } from "./challengeChoice.ts";

function deck(id: string, size: number): SavedDeck {
  return { id, name: id, cards: Array.from({ length: size }, () => "c001"), catalogVersion: "v", portrait: null, createdAt: 0, updatedAt: 0 };
}

function trio(id: string, deckIds: TrioSlots): SavedTrio {
  return { id, name: id, deckIds, createdAt: 0, updatedAt: 0 };
}

const SHORT = deck("short", DECK_SIZE - 1);
const FULL = deck("full", DECK_SIZE);
const OTHER_FULL = deck("other-full", DECK_SIZE);

const OPEN = trio("open", ["a", null, "c"]);
const WHOLE = trio("whole", ["a", "b", "c"]);
const OTHER_WHOLE = trio("other-whole", ["d", "e", "f"]);

describe("pickDeck", () => {
  it("takes the remembered deck while it is saved, complete or not", () => {
    expect(pickDeck([FULL, SHORT], "short")).toBe(SHORT);
  });

  it("falls back to the first complete deck when the remembered one is gone or none is remembered", () => {
    expect(pickDeck([SHORT, FULL, OTHER_FULL], "deleted")).toBe(FULL);
    expect(pickDeck([SHORT, FULL, OTHER_FULL], null)).toBe(FULL);
    expect(firstDeck([SHORT, FULL, OTHER_FULL])).toBe(FULL);
  });

  it("falls back to the first deck when none is complete, and to null when there is none", () => {
    const shorter = deck("shorter", 1);
    expect(pickDeck([SHORT, shorter], null)).toBe(SHORT);
    expect(pickDeck([], "short")).toBeNull();
    expect(firstDeck([])).toBeNull();
  });
});

describe("pickTrio", () => {
  it("takes the remembered trio while it is saved, filled or not", () => {
    expect(pickTrio([WHOLE, OPEN], "open")).toBe(OPEN);
  });

  it("falls back to the first trio with three decks when the remembered one is gone or none is remembered", () => {
    expect(pickTrio([OPEN, WHOLE, OTHER_WHOLE], "deleted")).toBe(WHOLE);
    expect(pickTrio([OPEN, WHOLE, OTHER_WHOLE], null)).toBe(WHOLE);
    expect(firstTrio([OPEN, WHOLE, OTHER_WHOLE])).toBe(WHOLE);
  });

  it("falls back to the first trio when none has three decks, and to null when there is none", () => {
    const emptier = trio("emptier", [null, null, null]);
    expect(pickTrio([OPEN, emptier], null)).toBe(OPEN);
    expect(pickTrio([], "open")).toBeNull();
    expect(firstTrio([])).toBeNull();
  });
});

describe("R924000 choiceOf", () => {
  it("is null while the mode's own pick is missing", () => {
    expect(choiceOf("bo1", null, "whole")).toBeNull();
    expect(choiceOf("bo3", "full", null)).toBeNull();
  });

  it("names the deck for a best of one and the trio for a best of three, whatever else is picked", () => {
    expect(choiceOf("bo1", "full", "whole", "Classic")).toEqual({ mode: "bo1", deckId: "full" });
    expect(choiceOf("bo3", "full", "whole", "Classic")).toEqual({ mode: "bo3", trioId: "whole" });
  });

  it("R924000 a random deal carries the lean pick, and none without one", () => {
    expect(choiceOf("random", null, null, "Classic")).toEqual({ mode: "random", leanSet: "Classic" });
    expect(choiceOf("random", null, null)).toEqual({ mode: "random" });
    expect(choiceOf("random", null, null, null)).toEqual({ mode: "random" });
  });
});
