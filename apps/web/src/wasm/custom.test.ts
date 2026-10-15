// R915009: the normal browser module holds no Luau, so it refuses a game with custom cards, which
// play in the Luau module alone (#915, #919). A normal game carries none and is dealt as before.

import { describe, expect, it } from "vitest";

import type { CustomCard } from "@jackioh/shared";

import { resolveDecks } from "../game/decks.ts";
import { registeredCatalog } from "@jackioh/engine";
import { createGame } from "./index.ts";

const CARD: CustomCard = {
  id: "custom-fixture",
  version: 1,
  frame: { name: "Fixture", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 1, text: "" },
  source: "return Card {}",
};

describe("R915009 the normal module and custom cards", () => {
  it("R915009 refuses a game with custom cards, naming the module that runs them", () => {
    const resolved = resolveDecks("cheap20", "cheap20", registeredCatalog());
    if ("error" in resolved) throw new Error(resolved.error);
    const [mine, theirs] = resolved.decks;
    const deck = [CARD.id, ...mine.slice(1)];
    expect(() => createGame({ seed: "r915009", decks: [deck, theirs], customCards: [CARD] })).toThrow(/Luau module/);
  });

  it("R915009 deals a game with none, whose state carries no custom card", () => {
    const resolved = resolveDecks("cheap20", "cheap20", registeredCatalog());
    if ("error" in resolved) throw new Error(resolved.error);
    const state = createGame({ seed: "r915009", decks: resolved.decks });
    expect(JSON.stringify(state)).not.toContain("customCards");
  });
});
