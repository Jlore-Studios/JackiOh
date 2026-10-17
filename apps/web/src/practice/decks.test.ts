// §9.9: which saved decks the practice setup offers (#1095).

import { describe, expect, it } from "vitest";

import { DECK_SIZE } from "@jackioh/engine/config";

import { practiceDeckIssues } from "../wire/validator.ts";
import { deckChoiceFromValue, deckOptions, isPlayableSavedDeck, savedDeckRefusal, type PracticeSavedDeck } from "./decks.ts";

const CORE = Array.from({ length: DECK_SIZE }, (_, i) => `core-${String(i + 1).padStart(3, "0")}`);

const VALID: PracticeSavedDeck = { name: "Straight Core", cards: CORE };
const REPEATED: PracticeSavedDeck = { name: "Twice Over", cards: [...CORE.slice(0, -1), CORE[0] ?? ""] };
const UNKNOWN: PracticeSavedDeck = { name: "Lost Card", cards: [...CORE.slice(0, -1), "core-999"] };
const CUSTOM: PracticeSavedDeck = { name: "Imp Deck", cards: [...CORE.slice(0, -1), "custom-ember-imp"] };
const SHORT: PracticeSavedDeck = { name: "Half Built", cards: CORE.slice(0, 12) };

describe("the saved decks practice offers", () => {
  it("a complete legal deck is offered, though the player owns none of it", () => {
    expect(practiceDeckIssues(VALID)).toEqual([]);
    expect(savedDeckRefusal(VALID)).toBeNull();
    expect(isPlayableSavedDeck(VALID)).toBe(true);
    expect(deckOptions([VALID]).find((option) => option.value === "saved:1")).toMatchObject({
      disabled: false,
      label: "Straight Core",
    });
  });

  it("a 20-card deck that repeats a card is refused before Start with the validator's reason", () => {
    const reason = savedDeckRefusal(REPEATED);
    expect(reason).toBe(practiceDeckIssues(REPEATED)[0]?.message);
    expect(reason).toContain("Twice Over");
    expect(reason).toContain("core-001");
    expect(isPlayableSavedDeck(REPEATED)).toBe(false);
    expect(deckChoiceFromValue("saved:1", [REPEATED])).toBeNull();
    expect(deckOptions([REPEATED]).find((option) => option.value === "saved:1")).toMatchObject({
      disabled: true,
      label: "Twice Over (needs a fix)",
    });
  });

  it("a 20-card deck naming a card the catalog does not have is refused before Start with the validator's reason", () => {
    const reason = savedDeckRefusal(UNKNOWN);
    expect(reason).toBe(practiceDeckIssues(UNKNOWN)[0]?.message);
    expect(reason).toContain("Lost Card");
    expect(reason).toContain("core-999");
    expect(isPlayableSavedDeck(UNKNOWN)).toBe(false);
    expect(deckChoiceFromValue("saved:1", [UNKNOWN])).toBeNull();
  });

  it("a deck naming a custom card stays selectable: custom cards play in practice", () => {
    expect(practiceDeckIssues(CUSTOM)).toEqual([]);
    expect(isPlayableSavedDeck(CUSTOM)).toBe(true);
    expect(deckChoiceFromValue("saved:1", [CUSTOM])).toMatchObject({ kind: "saved", cards: CUSTOM.cards });
  });

  it("a draft short of DECK_SIZE has no refusal, only its count", () => {
    expect(savedDeckRefusal(SHORT)).toBeNull();
    expect(isPlayableSavedDeck(SHORT)).toBe(false);
    expect(deckOptions([SHORT]).find((option) => option.value === "saved:1")?.label).toBe(
      `Half Built (12 of ${String(DECK_SIZE)} cards, not complete)`,
    );
  });
});
