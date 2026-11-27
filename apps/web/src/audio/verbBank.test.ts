// #5190: the spell verbs' effects in card-audio.json5 (test/verbBank.ts holds the table): every verb has a
// wind-up, a release and a tail in the bank, each built from a recipe and named for its sound, never a card.

import { describe, expect, it } from "vitest";

import { SFX_ELEMENTS } from "./sfx.ts";
import { CARD_AUDIO } from "./soundTables.ts";
import { DAMAGE_ELEMENTS, SPELL_VERBS, VERB_BANK_EFFECTS, VERB_ROWS } from "./test/verbBank.ts";

const NEW_RECIPES = [
  "elementWindUp", "elementBolt", "elementTail", "windUp", "sparkleTail", "hush", "shatter", "morph", "revealGlint",
] as const;

describe("#5190 the verb table against the bank", () => {
  it("#5190 has a row per damage element and per other verb", () => {
    expect(DAMAGE_ELEMENTS).toEqual(SFX_ELEMENTS);
    const keys = [...DAMAGE_ELEMENTS.map((e) => `damage.${e}`), ...SPELL_VERBS];
    expect(Object.keys(VERB_ROWS).sort()).toEqual(keys.sort());
    expect(SPELL_VERBS).toHaveLength(16);
  });

  it("#5190 gives every row a wind-up, a release and a tail that are bank effects", () => {
    const bank = CARD_AUDIO.effects;
    const missing = Object.entries(VERB_ROWS).flatMap(([key, row]) =>
      (["windUp", "release", "tail"] as const).flatMap((part) =>
        row[part] !== "" && Object.hasOwn(bank, row[part]) ? [] : [`${key}.${part} = "${row[part]}"`],
      ),
    );
    expect(missing).toEqual([]);
  });

  it("#5190 exempts no bank effect that a row does not use", () => {
    const used = new Set(Object.values(VERB_ROWS).flatMap((row) => [row.windUp, row.release, row.tail]));
    expect([...VERB_BANK_EFFECTS].filter((name) => !used.has(name))).toEqual([]);
    expect(VERB_BANK_EFFECTS.size).toBe(used.size);
  });

  it("#5190 builds every new recipe into at least one bank effect", () => {
    const used = new Set(Object.values(CARD_AUDIO.effects).map((effect) => effect.sfx));
    expect(NEW_RECIPES.filter((id) => !used.has(id))).toEqual([]);
  });

  it("#5190 names the verb effects for what they sound like, never a card", () => {
    const names = Object.keys(CARD_AUDIO.effects);
    const verbNames = [...VERB_BANK_EFFECTS];
    const cardIds = Object.keys(CARD_AUDIO.cards);
    const offenders = verbNames.filter((name) => cardIds.some((id) => name.toLowerCase().includes(id)));
    expect(offenders).toEqual([]);
    expect(names).toEqual(expect.arrayContaining(verbNames));
  });
});
