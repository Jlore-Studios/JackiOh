// Test-only definitions and scripts for `effects/datacenter.ts` (T-AI-4 Chain of Thought's chained draw,
// T-AI-6 Datacenter Fire's sweep). The engine does not depend on `packages/cards`; the real cards' tests
// cover the same cases again (packages/cards/test/classic-plus/t-ai-04-*.test.ts, t-ai-06-*.test.ts).

import type { CardDef, CardDefs } from "@jackioh/shared";
import { damage } from "../../src/effects/damage";
import type { CardScripts } from "../../src/script";
import { spellDef, unitDef } from "./catalog";

/** The running Spell (either verb is a Spell's). */
export const runner = spellDef(940, { id: "fx-dc-runner", name: "Fixture Datacenter Spell", tags: ["AI", "Token"], token: true, rarity: "Token" });

/** Field Spells: a plain one, an Indestructible one (#98's keyword), and one whose Death hits the enemy hero. */
export const field = spellDef(941, { id: "fx-dc-field", name: "Fixture Field Spell", type: "Field Spell" });
export const hardField = spellDef(942, {
  id: "fx-dc-hard",
  name: "Fixture Indestructible Field Spell",
  type: "Field Spell",
  base: { keywords: [{ kind: "Indestructible" }], text: "Indestructible" },
  radiant: { keywords: [{ kind: "Indestructible" }], text: "Indestructible" },
});
export const dyingField = spellDef(943, { id: "fx-dc-dying", name: "Fixture Dying Field Spell", type: "Field Spell" });
/** DYING_FIELD's Death: 3 damage to the enemy hero of its controller. */
export const DYING_FIELD_DAMAGE = 3;

/** An Animated Field Spell, which prints the attack and health of the Unit it becomes (B3.1 rule 1). */
export const animatedField = spellDef(951, {
  id: "fx-dc-animated",
  name: "Fixture Animated Field Spell",
  type: "Field Spell",
  base: { attack: 2, health: 3, keywords: [{ kind: "Animated" }], text: "Animated" },
  radiant: { attack: 4, health: 6, keywords: [{ kind: "Animated" }], text: "Animated" },
});

/** The backrow cards that are no Field Spells. */
export const trap = spellDef(944, { id: "fx-dc-trap", name: "Fixture Trap", type: "Trap" });
export const fieldTrap = spellDef(945, { id: "fx-dc-ftrap", name: "Fixture Field Trap", type: "Field Trap" });

/** Library cards for the chained draw: (0), (1), (2) and (X) Spells, a (2) Unit and a cast-on-draw Spell. */
export const free = spellDef(946, { id: "fx-dc-free", name: "Fixture (0) Spell", cost: 0 });
export const one = spellDef(947, { id: "fx-dc-one", name: "Fixture (1) Spell", cost: 1 });
export const two = unitDef(948, { id: "fx-dc-two", name: "Fixture (2) Unit", cost: 2 });
export const xCost = spellDef(949, { id: "fx-dc-x", name: "Fixture (X) Spell", cost: "X" });
export const castOnDraw = spellDef(950, { id: "fx-dc-cod", name: "Fixture Cast On Draw", cost: 0 });

export function datacenterCatalog(base: CardDefs): CardDefs {
  const defs: Record<string, CardDef> = { ...base };
  for (const def of [runner, field, animatedField, hardField, dyingField, trap, fieldTrap, free, one, two, xCost, castOnDraw]) defs[def.id] = def;
  return defs;
}

export const DATACENTER_SCRIPTS: Record<string, CardScripts> = {
  [dyingField.id]: {
    base: { death: () => [damage({ to: { of: "enemyHero" }, amount: DYING_FIELD_DAMAGE })] },
    radiant: { death: () => [damage({ to: { of: "enemyHero" }, amount: DYING_FIELD_DAMAGE })] },
  },
  [castOnDraw.id]: { base: { staticFlags: { castOnDraw: true } }, radiant: { staticFlags: { castOnDraw: true } } },
};
