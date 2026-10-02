// Test-only definitions and scripts for `effects/fruit.ts` (the Classic+ Fruit verbs: C+ #65, #65.2,
// #65.3, #65.5, #66). The engine does not depend on `packages/cards`, so the five Grapes are stand-ins
// under the ids `GRAPE_ODDS` names, with the Fruit tag and the Token rarity the real ones carry; the
// real cards' tests cover the same cases again (packages/cards/test/classic-plus/065-*.test.ts).

import type { CardDef, CardDefs } from "@jackioh/shared";
import { GRAPE_ODDS } from "../../src/config";
import { addRolledGrapes, damageEnemyOrHealFriend, drawPriced, replaceHandWithRandom } from "../../src/effects/fruit";
import type { CardScripts } from "../../src/script";
import { spellDef } from "./catalog";

/** A Grape stand-in: a Fruit Spell token, as §7 prints each of the five. */
function grapeDef(defId: string, at: number): CardDef {
  return spellDef(900 + at, {
    id: defId,
    index: `65.${at + 1}`,
    name: `Fixture Grape ${at + 1}`,
    set: "Classic+",
    tags: ["Fruit", "Token"],
    rarity: "Token",
    token: true,
  });
}

/** Two Grapes' stand-in: "Add 3 Grapes", the Radiant face printing Lucky 1. */
export const grapeRoller = spellDef(910, {
  id: "fx-fruit-roller",
  name: "Fixture Grape Roller",
  radiant: { keywords: [{ kind: "Lucky", n: 1 }], text: "Lucky 1. Add 3 Radiant Grapes." },
});

/** Normal Grape's stand-in: a target, then one draw priced −1. */
export const pricedDraw = spellDef(911, { id: "fx-fruit-priced", name: "Fixture Priced Draw" });

/** A cast-on-draw Spell that does nothing, to end a priced draw with no card in hand (R58). */
export const castOnDraw = spellDef(912, { id: "fx-fruit-cod", name: "Fixture Cast On Draw" });

/** Mythic Grape's stand-in, and the one non-token "Mythic" its pool holds besides. */
export const replacer = spellDef(913, { id: "fx-fruit-replacer", name: "Fixture Replacer", rarity: "Mythic" });
export const mythic = spellDef(914, { id: "fx-fruit-mythic", name: "Fixture Mythic", rarity: "Mythic" });

export function fruitCatalog(base: CardDefs): CardDefs {
  const defs: Record<string, CardDef> = { ...base };
  GRAPE_ODDS.forEach((grape, at) => {
    defs[grape.defId] = grapeDef(grape.defId, at);
  });
  for (const def of [grapeRoller, pricedDraw, castOnDraw, replacer, mythic]) defs[def.id] = def;
  return defs;
}

export const FRUIT_SCRIPTS: Record<string, CardScripts> = {
  [grapeRoller.id]: {
    base: { cry: () => [addRolledGrapes({ count: 3 })] },
    radiant: { cry: () => [addRolledGrapes({ count: 3, radiant: true })] },
  },
  [pricedDraw.id]: {
    base: {
      targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }],
      cry: () => [damageEnemyOrHealFriend({ amount: 2 }), drawPriced({ costMod: -1 })],
    },
    radiant: {
      targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }],
      cry: () => [damageEnemyOrHealFriend({ amount: 4 }), drawPriced({ costOverride: 0 }), drawPriced({ costOverride: 0 })],
    },
  },
  [castOnDraw.id]: { base: { staticFlags: { castOnDraw: true } }, radiant: { staticFlags: { castOnDraw: true } } },
  [replacer.id]: {
    base: { cry: () => [replaceHandWithRandom({ query: { rarity: "Mythic" }, costOverride: 0 })] },
    radiant: { cry: () => [replaceHandWithRandom({ query: { rarity: "Mythic" }, costOverride: 0, radiant: true })] },
  },
};
