// Test-only definitions and scripts for `subsystems/twiceForward.ts` (C+ #74 Twice Forward One Step
// Backwards, R425). The engine does not depend on `packages/cards`; the real card's test covers the same
// cases again (packages/cards/test/classic-plus/074-twice-forward-one-step-backwards.test.ts).

import type { CardDef, CardDefs } from "@jackioh/shared";
import { castNew } from "../../src/effects/cast";
import { damage } from "../../src/effects/damage";
import { exile } from "../../src/effects/move";
import type { CardScripts } from "../../src/script";
import { twiceForwardTrigger } from "../../src/subsystems/twiceForward";
import { spellDef, unitDef } from "./catalog";

/** #74's stand-in: a (2) Field Trap printing Brittle 4 (Radiant 10), every 2 plays, +1 Brittle. */
export const forward = spellDef(960, {
  id: "fx-tf-forward",
  name: "Fixture Twice Forward",
  type: "Field Trap",
  cost: 2,
  params: [
    { key: "plays", base: 2, radiant: 2, better: "down", step: 1, min: 2 },
    { key: "brittleGain", base: 1, radiant: 1, better: "up", step: 1, min: 1 },
  ],
  base: { keywords: [{ kind: "Brittle", n: 4 }], text: "Brittle 4" },
  radiant: { keywords: [{ kind: "Brittle", n: 10 }], text: "Brittle 10" },
});

/** The opponent's plays: a Spell that stays in the graveyard, one that exiles itself, a Trap. */
export const spell = spellDef(961, { id: "fx-tf-spell", name: "Fixture Plain Spell" });
export const selfExiler = spellDef(962, { id: "fx-tf-exiler", name: "Fixture Self-Exiling Spell" });
export const trap = spellDef(963, { id: "fx-tf-trap", name: "Fixture Trap", type: "Trap" });

/** A Unit whose end-of-turn line hits the enemy hero for 1, and one whose Cry hits it for 5. */
export const turner = unitDef(964, { id: "fx-tf-turner", name: "Fixture End-Of-Turn Unit", attack: 1, health: 1 });
export const crier = unitDef(965, { id: "fx-tf-crier", name: "Fixture Cry Unit", attack: 1, health: 1 });
/** A Spell whose resolution casts the plain Spell (R70): the cast is the later play, though it resolves first. */
export const caster = spellDef(966, { id: "fx-tf-caster", name: "Fixture Casting Spell" });
export const TURNER_DAMAGE = 1;
export const CRIER_DAMAGE = 5;

export function twiceForwardCatalog(base: CardDefs): CardDefs {
  const defs: Record<string, CardDef> = { ...base };
  for (const def of [forward, spell, selfExiler, trap, turner, crier, caster]) defs[def.id] = def;
  return defs;
}

export const TWICE_FORWARD_SCRIPTS: Record<string, CardScripts> = {
  [forward.id]: {
    base: { triggers: [twiceForwardTrigger({ radiantCopy: false })] },
    radiant: { triggers: [twiceForwardTrigger({ radiantCopy: true })] },
  },
  [selfExiler.id]: {
    base: { cry: () => [exile({ target: { of: "self" } })] },
    radiant: { cry: () => [exile({ target: { of: "self" } })] },
  },
  [caster.id]: {
    base: { cry: () => [castNew({ def: spell.id })] },
    radiant: { cry: () => [castNew({ def: spell.id })] },
  },
  [turner.id]: {
    base: { endOfTurn: () => [damage({ to: { of: "enemyHero" }, amount: TURNER_DAMAGE })] },
    radiant: { endOfTurn: () => [damage({ to: { of: "enemyHero" }, amount: TURNER_DAMAGE })] },
  },
  [crier.id]: {
    base: { cry: () => [damage({ to: { of: "enemyHero" }, amount: CRIER_DAMAGE })] },
    radiant: { cry: () => [damage({ to: { of: "enemyHero" }, amount: CRIER_DAMAGE })] },
  },
};
