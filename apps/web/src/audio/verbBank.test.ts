// #5190: the spell verbs' effects in card-audio.json5 (test/verbBank.ts holds the table): every verb has a
// wind-up, a release and a tail in the bank, each built from a recipe and named for its sound, never a card.

import { describe, expect, it } from "vitest";

import { CATALOG } from "../wire/cards.ts";
import { SFX_ELEMENTS } from "./sfx.ts";
import { CARD_AUDIO } from "./soundTables.ts";
import { DAMAGE_ELEMENTS, SPELL_VERBS, VERB_BANK_EFFECTS, VERB_ROWS } from "./test/verbBank.ts";

const NEW_RECIPES = [
  "elementWindUp", "elementBolt", "elementTail", "windUp", "sparkleTail", "hush", "shatter", "morph", "revealGlint",
] as const;

/** A name as lowercase letters and digits only, so `Fire Wind`, `fire-wind` and `fireWind` compare equal. */
function squash(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]/g, "");
}

/** The `effects` whose squashed name is a card's. Equality, not containment: "Arc" and "Rewind" are cards. */
function namedForACard(effects: Iterable<string>, cardNames: ReadonlySet<string>): string[] {
  return [...effects].filter((effect) => cardNames.has(squash(effect)));
}

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

  it("#5190 builds every new recipe into at least one bank effect", () => {
    const used = new Set(Object.values(CARD_AUDIO.effects).map((effect) => effect.sfx));
    expect(NEW_RECIPES.filter((id) => !used.has(id))).toEqual([]);
  });

  it("#5190 names the verb effects for what they sound like, never a card", () => {
    const cardNames = new Set(Object.values(CATALOG).map((def) => squash(def.name)));
    expect(cardNames.size, "the catalog's card names were read").toBeGreaterThan(0);
    // The comparison can fail: a name that squashes to a card's is found.
    expect(namedForACard(["Fire-Wind"], new Set(["firewind"]))).toEqual(["Fire-Wind"]);
    expect(namedForACard(VERB_BANK_EFFECTS, cardNames), "verb effects named like a catalog card").toEqual([]);
  });
});
