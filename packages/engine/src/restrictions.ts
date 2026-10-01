// What a card may not be done to (docs/classic-sets.md B5 E35): the one place a targeting rule, a
// damage step or an attack validator asks whether a card is out of an effect's reach, so the answer a
// client greys a pick out by and the answer the reducer refuses it by are one call (§10.2).
//
// Two families live here.
//
//  * Immune to Spells: "a Spell can't target it and doesn't affect it". The targeting half is the
//    play pipeline's (`spellCannotReach` in `playChoices`); the "doesn't affect it" half is
//    `effects/targets.ts`, which asks `effectIsFromSpell` for every card a single target or a scope
//    names, so every verb written in that vocabulary passes an immune unit by.
//  * The attack restrictions: can't be attacked (Classic+ #51), attacked only from its own lane
//    (Classic+ #19.1), can't attack or be attacked (Classic+ #33's carried Unit). `combat.ts` asks
//    `attackRestriction` for a declared attack (§4.2 step 2), for the Taunt wall (step 3: a Taunt
//    binds only the attackers that may attack it) and for every forced attack, which skips steps 1 to
//    3 (R53) but not these.
//
// A restriction a card prints is a static flag (`Script.staticFlags`), so a Vanilla takes it with the
// rest of the text (`scripts.scriptOf`). One that comes from where a card stands rather than from its
// text — the Unit an Ivory Tower carries (B5 E21) — is registered here by the module that knows the
// position (`registerAttackBar`), the way `combat.ts` registers its declaration check with `traps.ts`.

import type { DamageTarget } from "./damage";
import { cardTypeOf } from "./faces";
import { unitHas } from "./layers";
import type { EffectContext } from "./script";
import { flagsOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import { cardAt, slotOf } from "./zones";

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

/**
 * E35: whether the effect list running in `ctx` is a Spell's. Its card says so while it is there; a
 * continuation re-entered once the card has gone (R98's resolving Spell returned to a hand, R127's
 * card that ceased to exist) still names the script it runs (`ctx.defId`), on the face it recorded.
 */
export function effectIsFromSpell(ctx: Pick<EffectContext, "state" | "self" | "defId" | "radiant">): boolean {
  if (ctx.self !== null) return isSpellSource(ctx.state, ctx.self);
  if (ctx.defId === undefined || ctx.defId === "") return false;
  return cardTypeOf(ctx.state, { defId: ctx.defId, radiant: ctx.radiant }) === "Spell";
}

/**
 * E35: the "doesn't affect it" half, as `effects/targets.ts` asks it of every card an effect names —
 * a Spell's effects pass an immune unit on the field by. A card in a hand or a deck is no unit, and
 * its printed keyword protects nothing there.
 */
export function unaffectedBy(ctx: Pick<EffectContext, "state" | "self" | "defId" | "radiant">, card: CardInstance): boolean {
  return card.zone.z === "field" && effectIsFromSpell(ctx) && immuneToSpells(ctx.state, card);
}

// ---------------------------------------------------------------------------
// The attack restrictions
// ---------------------------------------------------------------------------

/** What a positional restriction says of a unit: it may not attack, or may not be attacked. */
export type AttackBarAnswer = { cantAttack?: boolean; cantBeAttacked?: boolean };

/** A restriction that comes from where a card stands (B5 E21's carried Unit), read on demand. */
export type AttackBar = (state: GameState, unit: CardInstance) => AttackBarAnswer;

const bars = new Map<string, AttackBar>();

/**
 * Registered at module scope by the module that knows the position, under a name of its own, so a
 * test can take it out again. Returns the bar it replaced.
 */
export function registerAttackBar(name: string, bar: AttackBar | undefined): AttackBar | undefined {
  const previous = bars.get(name);
  if (bar === undefined) bars.delete(name);
  else bars.set(name, bar);
  return previous;
}

function barred(state: GameState, unit: CardInstance, key: keyof AttackBarAnswer): boolean {
  for (const bar of bars.values()) {
    if (bar(state, unit)[key] === true) return true;
  }
  return false;
}

/** E35: this unit may make no attack, declared or forced ("can't attack or be attacked"). */
export function cannotAttack(state: GameState, unit: CardInstance): boolean {
  return flagsOf(unit).cantAttackOrBeAttacked === true || barred(state, unit, "cantAttack");
}

/** E35: no attack may be made on this unit, declared or forced. Effects still target and hit it. */
export function cannotBeAttacked(state: GameState, unit: CardInstance): boolean {
  const flags = flagsOf(unit);
  return flags.cantBeAttacked === true || flags.cantAttackOrBeAttacked === true || barred(state, unit, "cantBeAttacked");
}

/** E35: only a unit standing in this unit's lane may attack it. */
export function attackableOnlyFromLane(unit: CardInstance): boolean {
  return flagsOf(unit).attackedOnlyFromLane === true;
}

/**
 * E35: why the unit restrictions bar `attacker` from attacking `target` — a declared attack or a
 * forced one alike (R53 waives position, sickness and Taunt, never these) — or null when nothing
 * does. A hero carries no restriction of its own.
 */
export function attackRestriction(state: GameState, attacker: CardInstance, target: DamageTarget): string | null {
  if (cannotAttack(state, attacker)) return "that unit cannot attack";
  if (target.kind === "hero") return null;
  const defender = target.instance;
  if (cannotBeAttacked(state, defender)) return "that unit cannot be attacked";
  if (attackableOnlyFromLane(defender)) {
    const from = slotOf(state, attacker);
    const to = slotOf(state, defender);
    if (from === null || to === null || from.lane !== to.lane) return "only a unit in its lane may attack that unit";
  }
  return null;
}

// ---------------------------------------------------------------------------
// Berserk
// ---------------------------------------------------------------------------

/** B5 E35: whether the unit has gone Berserk — a status, kept until it leaves the field (R78). */
export function isBerserk(card: CardInstance): boolean {
  return card.berserk === true;
}

/** B5 E35: whether the unit may go Berserk now: on the field, not Berserk yet, and not immune to it. */
export function canGoBerserk(state: GameState, card: CardInstance): boolean {
  if (card.zone.z !== "field" || card.zone.row !== "units") return false;
  if (!activeOnField(state, card)) return false;
  return card.berserk !== true && flagsOf(card).neverBerserk !== true;
}

/** The card that acts in its zone: on the field and the top of its pile (§3.2, R13). */
function activeOnField(state: GameState, card: CardInstance): boolean {
  const at = slotOf(state, card);
  return at !== null && cardAt(state, at)?.id === card.id;
}
