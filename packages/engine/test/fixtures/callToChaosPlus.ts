// Test-only definitions for `subsystems/callToChaosPlus.ts` (C+ #73 Call to Chaos (Classic+ Edition)'s
// table, R423). The engine does not depend on `packages/cards`, so each pool its entries name — Fruits
// (Grapes included, R382), Books, Classic cards, the "Call to Chaos" pool of both editions, the Classic
// Golem — gets stand-ins under the tags, sets and indices the real cards carry; the real card's test
// covers the same cases against the real catalog
// (packages/cards/test/classic-plus/073-call-to-chaos-classic-edition.test.ts).

import type { CardDef, CardDefs } from "@jackioh/shared";
import { spellDef, unitDef } from "./catalog";

/** C+ #73 itself and Core #95: the two editions the recursion and the deck replacement draw from (R28). */
export const plus = spellDef(970, {
  id: "fx-cp-plus",
  index: "73",
  name: "Fixture Call to Chaos (Classic+ Edition)",
  set: "Classic+",
  tags: ["Call to Chaos"],
  rarity: "Legendary",
  cost: 4,
});
export const core95 = spellDef(971, {
  id: "fx-cp-core",
  index: "95",
  name: "Fixture Call to Chaos",
  tags: ["Call to Chaos"],
  rarity: "Legendary",
  cost: 4,
});

/** §7's Classic Golem stand-in: Classic+ index 73.1. */
export const golem = unitDef(972, {
  id: "fx-cp-golem",
  index: "73.1",
  name: "Fixture Classic Golem",
  set: "Classic+",
  tags: ["Token"],
  rarity: "Token",
  token: true,
  cost: 4,
  attack: 10,
  health: 10,
});

/** The pools: a Fruit and a Grape token (R382), a Book of another set and a Book token, Classic cards. */
export const fruit = spellDef(973, { id: "fx-cp-fruit", name: "Fixture Fruit", set: "Classic+", tags: ["Fruit"] });
export const grape = spellDef(974, {
  id: "fx-cp-grape",
  name: "Fixture Grape",
  set: "Classic+",
  tags: ["Fruit", "Token"],
  rarity: "Token",
  token: true,
});
export const book = spellDef(975, { id: "fx-cp-book", name: "Fixture Book", set: "Classic", tags: ["Book"], cost: 2 });
export const bookToken = spellDef(976, {
  id: "fx-cp-book-token",
  name: "Fixture Book Token",
  tags: ["Book", "Token"],
  rarity: "Token",
  token: true,
});
export const classic = unitDef(977, { id: "fx-cp-classic", name: "Fixture Classic Unit", set: "Classic", cost: 3 });

/** A deck card no Fuse may keep (R23). */
export const immutable = unitDef(978, { id: "fx-cp-immutable", name: "Fixture Immutable Unit", keywords: [{ kind: "Immutable" }] });

/** A backrow Trap and an Indestructible Field Spell for the destroy entry. */
export const trap = spellDef(979, { id: "fx-cp-trap", name: "Fixture Trap", type: "Trap" });
export const hardField = spellDef(980, {
  id: "fx-cp-hard",
  name: "Fixture Indestructible Field Spell",
  type: "Field Spell",
  base: { keywords: [{ kind: "Indestructible" }], text: "Indestructible" },
  radiant: { keywords: [{ kind: "Indestructible" }], text: "Indestructible" },
});

export function chaosPlusCatalog(base: CardDefs): CardDefs {
  const defs: Record<string, CardDef> = { ...base };
  for (const def of [plus, core95, golem, fruit, grape, book, bookToken, classic, immutable, trap, hardField]) defs[def.id] = def;
  return defs;
}
