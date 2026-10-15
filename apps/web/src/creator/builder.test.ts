// R915010, #920: the creator's builder writes card-kit Luau, a pure function of its model, and the
// card's numbers and text beside it; every script it writes passes the Luau module's check.

import { afterEach, describe, expect, it } from "vitest";

import { CUSTOM_TEXT_MAX_LENGTH } from "@jackioh/engine/config";
import type { CustomCard } from "@jackioh/shared";

import { luauBytes } from "../test/luau.ts";
import { checkCustomCard, useLuauModule } from "../wasm/index.ts";
import {
  AURA_WORDS,
  COUNT_WORDS,
  ONE_WORDS,
  OTHER_WORDS,
  TOKEN_WORDS,
  builderShows,
  emptyModel,
  isBuilderModel,
  numbersOf,
  writeScript,
  writeText,
  type BuilderModel,
  type BuilderStep,
} from "./builder.ts";

/** The e2e spec's card (#922): "Cry: deal 2 damage to an enemy unit". */
const STRIKE: BuilderModel = {
  ...emptyModel(),
  target: "an enemy unit",
  hooks: [{ trigger: "cry", steps: [{ verb: "damage", target: { kind: "declared" }, amount: 2 }] }],
};

afterEach(async () => {
  await useLuauModule(false);
});

describe("R915010 the builder writes card-kit Luau", () => {
  it("R915010 a Cry that deals damage to the card's target is the kit's `target` and `c:damage(c:target(), n.dmg)`", () => {
    expect(writeScript(STRIKE)).toBe(
      [
        "-- Written by the card creator's builder.",
        "return Card {",
        '  target = "an enemy unit",',
        "  cry = function(c, n)",
        "    c:damage(c:target(), n.dmg)",
        "  end,",
        "}",
        "",
      ].join("\n"),
    );
    expect(numbersOf(STRIKE)).toEqual([{ key: "dmg", base: 2, radiant: 4 }]);
    expect(writeText(STRIKE)).toBe("Cry: Deal {dmg} damage to an enemy unit.");
  });

  it("R915010 each amount is one of the card's numbers, keyed by its verb, a digit from the second on", () => {
    const model: BuilderModel = {
      ...emptyModel(),
      hooks: [
        {
          trigger: "cry",
          steps: [
            { verb: "damage", target: { kind: "words", words: "the enemy hero" }, amount: 1 },
            { verb: "damage", target: { kind: "words", words: "each enemy unit" }, amount: 3 },
            { verb: "buff", target: { kind: "words", words: "each friendly unit" }, attack: 1, health: 2 },
          ],
        },
      ],
      radiant: { dmg2: 5 },
    };
    expect(numbersOf(model)).toEqual([
      { key: "dmg", base: 1, radiant: 2 },
      { key: "dmg2", base: 3, radiant: 5 },
      { key: "buff", base: 1, radiant: 2 },
      { key: "buff2", base: 2, radiant: 4 },
    ]);
    expect(writeScript(model)).toContain('c:damage("each enemy unit", n.dmg2)');
    expect(writeScript(model)).toContain('c:buff("each friendly unit", n.buff, n.buff2)');
    expect(writeText(model)).toBe(
      "Cry: Deal {dmg} damage to the enemy hero, then deal {dmg2} damage to each enemy unit, then give each friendly unit +{buff}/+{buff2}.",
    );
  });

  it("R915010 a condition, an aura and an Activate are written as the kit reads them", () => {
    const model: BuilderModel = {
      ...emptyModel(),
      hooks: [
        {
          trigger: "endOfTurn",
          condition: { count: "friendly units", op: ">=", value: 3 },
          steps: [{ verb: "draw", amount: 1 }],
        },
      ],
      aura: { attack: 1, health: 0, give: ["Taunt"], to: "your units" },
      activate: { label: "Zap", cost: 2, steps: [{ verb: "armor", amount: 3 }] },
    };
    const script = writeScript(model);
    expect(script).toContain('    if c:count("friendly units") >= 3 then\n      c:draw(n.cards)\n    end');
    expect(script).toContain('  aura = { attack = 1, give = { "Taunt" }, to = "your units" },');
    expect(script).toContain('    label = "Zap",\n    cost = 2,\n    run = function(c, n)\n      c:armor(n.armor)');
    expect(writeText(model)).toBe(
      ["End of turn: If you have at least 3 friendly units: Draw {cards|card|cards}.", "Aura: Your units have +1/+0, Taunt.", "Activate (2): Gain {armor} Armor."].join("\n"),
    );
  });

  it("R915010 a model the builder saved is read back, and shows only while the script is the one it writes", () => {
    const saved: unknown = JSON.parse(JSON.stringify(STRIKE));
    expect(isBuilderModel(saved)).toBe(true);
    expect(isBuilderModel({ version: 2, hooks: [], target: null, radiant: {} })).toBe(false);
    expect(builderShows(STRIKE, writeScript(STRIKE))).toBe(true);
    expect(builderShows(STRIKE, `${writeScript(STRIKE)}-- mine\n`)).toBe(false);
    expect(builderShows(null, writeScript(STRIKE))).toBe(false);
  });
});

/** Every block the builder offers, each verb and each kind of target once at least. */
function everyStep(): BuilderStep[] {
  const steps: BuilderStep[] = [
    { verb: "damage", target: { kind: "declared" }, amount: 1 },
    { verb: "heal", target: { kind: "words", words: "your hero" }, amount: 2 },
    { verb: "destroy", target: { kind: "words", words: "a random enemy unit" } },
    { verb: "bounce", target: { kind: "words", words: "a random friendly unit" } },
    { verb: "buff", target: { kind: "words", words: "this unit" }, attack: 1, health: 1 },
    { verb: "give", target: { kind: "words", words: "each other friendly unit" }, keyword: "Taunt" },
    { verb: "draw", amount: 1 },
    { verb: "discard", amount: 1, who: "the enemy" },
    { verb: "discard", amount: 1, who: "you" },
    { verb: "armor", amount: 2 },
    { verb: "mana", amount: 1 },
  ];
  for (const words of ONE_WORDS) steps.push({ verb: "damage", target: { kind: "ask", words }, amount: 1 });
  for (const words of OTHER_WORDS) steps.push({ verb: "damage", target: { kind: "words", words }, amount: 1 });
  for (const token of TOKEN_WORDS) steps.push({ verb: "summon", token, amount: 1 });
  return steps;
}

/** A card of the model's: a 1-mana 1/1 Unit whose numbers and text are the builder's (a short text of its own when the builder's is too long to print). */
function cardOf(model: BuilderModel, id: string): CustomCard {
  const text = writeText(model);
  return {
    id,
    version: 1,
    frame: {
      name: "Built",
      type: "Unit",
      cost: 1,
      rarity: "Common",
      attack: 1,
      health: 1,
      text: text.length <= CUSTOM_TEXT_MAX_LENGTH ? text : "Does a great deal.",
      params: numbersOf(model).slice(0, 6),
    },
    source: writeScript(model),
    builder: model,
  };
}

describe("R915010 every script the builder writes passes the Luau module's check", () => {
  it("R915010 each block, target, trigger, condition, aura and Activate the builder offers is a script the kit loads", async () => {
    const steps = everyStep();
    const models: BuilderModel[] = [
      STRIKE,
      ...steps.map((step) => ({
        ...emptyModel(),
        target: "an enemy unit" as const,
        hooks: [{ trigger: "cry" as const, steps: [step] }],
      })),
      {
        ...emptyModel(),
        hooks: [
          { trigger: "death", steps: [{ verb: "summon", token: "Sheep Token", amount: 1 }] },
          { trigger: "startOfTurn", condition: { count: COUNT_WORDS[3], op: "<=", value: 2 }, steps: [{ verb: "draw", amount: 1 }] },
          { trigger: "endOfTurn", steps: [{ verb: "heal", target: { kind: "ask", words: "a friendly unit" }, amount: 1 }] },
        ],
        aura: { attack: 1, health: 1, give: ["Rush", "Taunt"], to: AURA_WORDS[2] },
        activate: { label: "Go", cost: 1, steps: [{ verb: "damage", target: { kind: "words", words: "a random enemy" }, amount: 1 }] },
      },
    ];
    for (const [at, model] of models.entries()) {
      const card = cardOf(model, `custom-built-${String(at)}`);
      const check = await checkCustomCard(card, luauBytes());
      expect(check.script, card.source).toEqual([]);
      expect(check.frame, card.source).toEqual([]);
    }
  });
});
