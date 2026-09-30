// What a card may not be done to (docs/classic-sets.md B5 E35): the one place a targeting rule, a
// damage step or an attack validator asks whether a card is out of an effect's reach, so the answer a
// client greys a pick out by and the answer the reducer refuses it by are one call (§10.2).

import { cardTypeOf } from "./faces";
import { unitHas } from "./layers";
import type { CardInstance, GameState } from "./state";

/** E35: whether the card has Immune to Spells now (a keyword, so layered: printed, granted, aura). */
export function immuneToSpells(state: GameState, card: CardInstance): boolean {
  return unitHas(state, card, "Immune to Spells");
}

/** E35: whether `source` is a Spell — the type "Immune to Spells" answers (not a Field Spell or Trap). */
export function isSpellSource(state: GameState, source: CardInstance | null): boolean {
  return source !== null && cardTypeOf(state, source) === "Spell";
}

/**
 * E35: "a Spell can't target it and doesn't affect it". True when `source` is a Spell and `card` is
 * immune to Spells, so the effect passes the card by as if it were not there.
 */
export function spellCannotReach(state: GameState, source: CardInstance | null, card: CardInstance): boolean {
  return isSpellSource(state, source) && immuneToSpells(state, card);
}
