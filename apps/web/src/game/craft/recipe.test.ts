// R883: which hats and keywords a crafted card's type takes is the engine's table (CRAFT_HAT_PRICES,
// CRAFT_KEYWORD_PRICES), read here and never written out again.

import { describe, expect, it } from "vitest";

import type { CardType, CraftOffer, CraftRecipe } from "../../wire/index.ts";
import {
  CRAFT_HAT_PRICES,
  CRAFT_KEYWORD_PRICES,
  CRAFT_RETIRED_VERBS,
  CRAFT_VERB_PRICES,
} from "../../wire/engineConfig.ts";
import {
  POTION_HAT,
  blankRecipe,
  frameOf,
  offeredHats,
  offeredKeywords,
  offeredVerbs,
  setType,
  takesHat,
  takesKeyword,
} from "./recipe.ts";

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

describe("R1182000 R1182002 the potion frame and the empty canvas", () => {
  it("R1182000 R1563008 no type takes the retired Activate hat, and a Potion's hat is a Spell's When cast", () => {
    for (const type of CARD_TYPES) expect(takesHat(type, "activate"), type).toBe(false);
    expect(POTION_HAT).toBe("whenCast");
    expect(takesHat("Spell", POTION_HAT)).toBe(true);
  });

  it("R1182000 a potion mode fixes its frame and no other mode does", () => {
    expect(frameOf("potion")).toBe("potion");
    expect(frameOf("hugePotion")).toBe("hugePotion");
    expect(frameOf("scratch")).toBeUndefined();
    expect(frameOf(undefined)).toBeUndefined();
  });

  it("R1182002 the empty canvas is a 0/1 Unit with nothing on it at the prompt's cost", () => {
    const blank = blankRecipe(3);
    expect(blank).toMatchObject({ cost: 3, type: "Unit", attack: 0, health: 1, keywords: [], echo: 0, hats: [] });
    expect(blank.frame).toBeUndefined();
  });

  it("R1182000 R1563008 a blank Potion is a 0/1 Spell holding its frame and its one When cast, empty", () => {
    const blank = blankRecipe(1, "potion");
    expect(blank.frame).toBe("potion");
    expect(blank).toMatchObject({ type: "Spell", attack: 0, health: 1 });
    expect(blank.hats).toEqual([{ hat: POTION_HAT, effects: [] }]);
  });
});

describe("R1563007 R1563009 the palette is the prompt's offer", () => {
  const offer: CraftOffer = {
    keywords: ["Armor", "Taunt", "Lifesteal", "Rush"],
    hats: ["death", "cry"],
    verbs: ["healYourHero", "draw", "damageTarget"],
  };

  it("R1563007 an offer's blocks come in the tables' order", () => {
    expect(offeredKeywords(offer)).toEqual(["Taunt", "Rush", "Lifesteal", "Armor"]);
    expect(offeredHats(offer)).toEqual(["cry", "death"]);
    expect(offeredVerbs(offer)).toEqual(["damageTarget", "healYourHero", "draw"]);
  });

  it("R1563009 with no offer every block is there but the retired verbs and the retired Activate", () => {
    expect(offeredKeywords(undefined)).toEqual(CRAFT_KEYWORD_PRICES.map((row) => row.kind));
    expect(offeredHats(undefined)).not.toContain("activate");
    expect(offeredHats(undefined)).toContain("whenCast");
    expect(offeredVerbs(undefined)).not.toContain("gainArmor");
    expect(CRAFT_RETIRED_VERBS).toEqual(["gainArmor"]);
    expect(offeredVerbs(undefined)).toHaveLength(CRAFT_VERB_PRICES.length - CRAFT_RETIRED_VERBS.length);
  });
});
