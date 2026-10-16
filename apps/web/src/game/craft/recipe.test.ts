// R883: which hats and keywords a crafted card's type takes is the engine's table (CRAFT_HAT_PRICES,
// CRAFT_KEYWORD_PRICES), read here and never written out again.

import { describe, expect, it } from "vitest";

import type { CardType, CraftRecipe } from "../../wire/index.ts";
import { CRAFT_HAT_PRICES, CRAFT_KEYWORD_PRICES } from "../../wire/engineConfig.ts";
import { setType, takesHat, takesKeyword } from "./recipe.ts";

const CARD_TYPES: readonly CardType[] = ["Unit", "Spell", "Field Spell", "Trap", "Field Trap"];

/** A Unit recipe holding every keyword and every hat the tables list. */
function everything(): CraftRecipe {
  return {
    cost: 2,
    type: "Unit",
    adjective: "Pure",
    noun: "Closure",
    attack: 1,
    health: 1,
    keywords: CRAFT_KEYWORD_PRICES.map((row) => (row.kind === "Armor" ? { kind: row.kind, n: 1 } : { kind: row.kind })),
    echo: 0,
    hats: CRAFT_HAT_PRICES.map((row) => ({ hat: row.hat, effects: [] })),
  };
}

/** What the table allows: a Unit any keyword, a Spell the `spell` rows, any other type none. */
function keywordAllowed(type: CardType, spell: boolean): boolean {
  return type === "Unit" || (type === "Spell" && spell);
}

describe("R883 the crafter's type rules are the engine's tables", () => {
  it("R883 setType keeps a hat exactly when its row's types hold the type", () => {
    for (const type of CARD_TYPES) {
      const kept = setType(everything(), type).hats.map((hat) => hat.hat);
      for (const row of CRAFT_HAT_PRICES) {
        expect(kept.includes(row.hat), `${type} ${row.hat}`).toBe(row.types.includes(type));
      }
    }
  });

  it("R883 setType keeps every keyword on a Unit, the spell rows on a Spell and none on the rest", () => {
    for (const type of CARD_TYPES) {
      const kept = setType(everything(), type).keywords.map((block) => block.kind);
      for (const row of CRAFT_KEYWORD_PRICES) {
        expect(kept.includes(row.kind), `${type} ${row.kind}`).toBe(keywordAllowed(type, row.spell));
      }
    }
  });

  it("R883 takesHat and takesKeyword answer row by row", () => {
    for (const type of CARD_TYPES) {
      for (const row of CRAFT_HAT_PRICES) {
        expect(takesHat(type, row.hat), `${type} ${row.hat}`).toBe(row.types.includes(type));
      }
      for (const row of CRAFT_KEYWORD_PRICES) {
        expect(takesKeyword(type, row.kind), `${type} ${row.kind}`).toBe(keywordAllowed(type, row.spell));
      }
    }
  });
});
