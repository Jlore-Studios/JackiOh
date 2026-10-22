// R1563010: Scratch Pad's code is a spelling of a recipe — what it reads, what it refuses to read, and
// that a recipe written out reads back as itself. Whether the card may be crafted is the engine's.

import { describe, expect, it } from "vitest";

import type { CraftRecipe } from "../../wire/index.ts";
import { blankRecipe } from "./recipe.ts";
import { parseRecipeText, recipeText } from "./recipeText.ts";

/** A recipe of every statement the code has. */
function everything(): CraftRecipe {
  return {
    cost: 3,
    type: "Unit",
    adjective: "Async",
    noun: "Macro",
    attack: 2,
    health: 3,
    keywords: [{ kind: "Taunt" }, { kind: "Armor", n: 2 }],
    echo: 0,
    hats: [
      { hat: "cry", effects: [{ verb: "damageTarget", n: 2 }, { verb: "grantKeyword", keyword: "Rush" }] },
      { hat: "death", effects: [{ verb: "addRandom", cardType: "Field Spell" }, { verb: "summonRush" }] },
    ],
  };
}

function read(text: string, cost = 3): CraftRecipe {
  const parsed = parseRecipeText(text, cost);
  if (!parsed.ok) throw new Error(parsed.errors.join("; "));
  return parsed.recipe;
}

function errorOf(text: string): string {
  const parsed = parseRecipeText(text, 2);
  if (parsed.ok) throw new Error("it read");
  return parsed.errors.join("; ");
}

describe("R1563010 Scratch Pad's code", () => {
  it("R1563010 a recipe written out reads back as itself", () => {
    expect(read(recipeText(everything()))).toEqual(everything());
    const spell: CraftRecipe = {
      ...blankRecipe(2),
      type: "Spell",
      echo: 1,
      keywords: [{ kind: "Lifesteal" }],
      hats: [{ hat: "whenCast", effects: [{ verb: "draw", n: 1 }] }],
    };
    expect(read(recipeText(spell), 2)).toEqual(spell);
    expect(read(recipeText(blankRecipe(0)), 0)).toEqual(blankRecipe(0));
  });

  it("R1563010 a name of two words is one code word", () => {
    const shielded: CraftRecipe = {
      ...blankRecipe(2),
      keywords: [{ kind: "Divine Shield" }, { kind: "First Strike" }],
      hats: [{ hat: "cry", effects: [{ verb: "grantKeyword", keyword: "Divine Shield" }, { verb: "discover", cardType: "Field Spell" }] }],
    };
    const text = recipeText(shielded);
    expect(text).toContain("keyword(DivineShield)");
    expect(text).toContain("keyword(FirstStrike)");
    expect(text).toContain("grantKeyword(DivineShield)");
    expect(text).toContain("discover(FieldSpell)");
    expect(read(text, 2)).toEqual(shielded);
  });

  it("R1563010 the empty canvas is written as a card with its stats", () => {
    expect(recipeText(blankRecipe(2))).toBe('card Unit "Pure Closure" {\n  stats(0, 1)\n}\n');
  });

  it("R1563010 comments, semicolons, spacing and line breaks are free; the cost is the prompt's", () => {
    const text = [
      "// my first card",
      'card FieldSpell "Static Trait" { startOfTurn { draw(1); healYourHero(2) } }  // done',
    ].join("\n");
    expect(read(text, 4)).toEqual({
      cost: 4,
      type: "Field Spell",
      adjective: "Static",
      noun: "Trait",
      attack: 0,
      health: 1,
      keywords: [],
      echo: 0,
      hats: [{ hat: "startOfTurn", effects: [{ verb: "draw", n: 1 }, { verb: "healYourHero", n: 2 }] }],
    });
  });

  it("R1563010 a word the code does not know is a syntax error that names its line", () => {
    expect(errorOf('card Unit "Pure Closure" {\n  stats(1, 1)\n  sneeze {\n  }\n}')).toContain("line 3");
    expect(errorOf('card Unit "Pure Closure" {\n  cry {\n    fireball(3)\n  }\n}')).toContain('no effect is called "fireball"');
    expect(errorOf('card Unit "Pure Closure" {\n  keyword(Flying)\n}')).toContain('no keyword is called "Flying"');
    expect(errorOf('card Creature "Pure Closure" {\n}')).toContain('no card type is called "Creature"');
  });

  it("R1563010 a malformed card is a syntax error", () => {
    expect(errorOf("")).toContain('expected "card"');
    expect(errorOf('card Unit "Pure" {\n}')).toContain("two words");
    expect(errorOf('card Unit "Pure Closure" {\n  stats(1, 1)\n')).toContain("found the end");
    expect(errorOf('card Unit "Pure Closure" {\n}\nextra')).toContain("nothing may follow");
    expect(errorOf('card Unit "Pure Closure" {\n  stats(1 1)\n}')).toContain("line 2");
  });

  it("R1563010 a retired block or a wrong hat still reads: refusing it is the engine's", () => {
    const text = 'card Trap "Pure Closure" {\n  cry {\n    gainArmor(2)\n  }\n}';
    expect(read(text).hats).toEqual([{ hat: "cry", effects: [{ verb: "gainArmor", n: 2 }] }]);
  });
});
