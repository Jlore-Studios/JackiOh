// Plague Tokens (SPEC §6.3 Plague Token; docs/classic-sets.md B5 E19; R471): the counter on a
// permanent, how many a placement puts there, and how they come off again.
//
// A Plague Token is `instance.counters.plague` (§10.1), a count on a permanent that R78 clears when
// the card leaves the field. Three things touch it and this module owns all three, so a card, the
// play pipeline and a verb in `effects/` can never disagree about them:
//   - a placement (`placePlagueOn`): one effect putting N tokens on one card, multiplied by what the
//     card receiving them says (Classic #27 Pestilent Slime's "doubled", `Script.plagueMultiplier`),
//     and reported once as `counterChanged` with `placed`, which "whenever Plague Tokens are placed
//     on this" answers once per placement however many tokens it put there (R471). Every gain of
//     tokens is a placement, Core #91 Fed Fauci's "+1 Plague Token" included;
//   - a removal (`removePlague`): Classic #78 Mutate Spell's "remove a Plague Token", and Classic #74
//     Corpse Plantation's tokens spent as mana, which the play pipeline pays through this, never by
//     writing the counter itself. A removal is no placement and carries no `placed`;
//   - the reads a card asks (`plagueOn`, `plagueOnField`, `permanentsOnField`), which `query.ts`
//     re-exports as board facts.
//
// Only a permanent on the field carries tokens: the top of a unit pile or a backrow card, face-down
// ones included (R471), never a card dormant under a Stack (R13) or one in a hand, deck or pile.

import type { PlayerId } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { PLAGUE_MULTIPLIER_NONE } from "./config";
import type { EngineSink } from "./resolve";
import { scriptOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import { cardAt, isBuried, slotsOf } from "./zones";

/** The tokens a card carries now; an untouched card carries none (§6.3 Plague Token). */
export function plagueOn(card: Pick<CardInstance, "counters">): number {
  return Math.max(0, card.counters.plague ?? 0);
}

/**
 * Every permanent on the field in R68's order — the given side first (the active player's by
 * default), units lane 1 upward then the backrow lane 1 upward, then the other side: the top of each
 * unit pile only (R13), and every backrow card, face-down or not.
 */
export function permanentsOnField(state: GameState, first: PlayerId = state.active): CardInstance[] {
  const sides = PLAYER_IDS.includes(first) ? [first, opponentOf(first)] : [...PLAYER_IDS];
  return sides.flatMap((player) =>
    (["units", "backrow"] as const).flatMap((row) =>
      slotsOf(player, row).flatMap((ref) => {
        const card = cardAt(state, ref);
        return card === null ? [] : [card];
      }),
    ),
  );
}

/**
 * Classic #59 Plague Doctor: "the number of Plague Tokens on the field" — every token on every
 * permanent, both sides, face-down cards included; or one side's only, with `player`.
 */
export function plagueOnField(state: GameState, player?: PlayerId): number {
  return permanentsOnField(state)
    .filter((card) => player === undefined || card.controller === player)
    .reduce((sum, card) => sum + plagueOn(card), 0);
}

/** Whether a card can carry Plague Tokens now: a permanent on the field, not dormant (R13). */
export function carriesPlague(state: GameState, card: CardInstance): boolean {
  return card.zone.z === "field" && !isBuried(state, card);
}

/**
 * R471: what one placement onto this card is multiplied by — its text's `plagueMultiplier` (Classic
 * #27), read on the face it wears; a card with no such text, or a Vanilla one (§6.3), multiplies by
 * 1. Never below 1, since a multiplier shrinks nothing.
 */
export function plagueMultiplierOf(state: GameState, card: CardInstance): number {
  const hook = scriptOf(card).plagueMultiplier;
  if (hook === undefined) return PLAGUE_MULTIPLIER_NONE;
  const value = Math.trunc(hook({ state, self: card, radiant: card.radiant }));
  return Number.isFinite(value) ? Math.max(PLAGUE_MULTIPLIER_NONE, value) : PLAGUE_MULTIPLIER_NONE;
}

/**
 * R471: one placement of `amount` Plague Tokens on `card`, multiplied by the card's multiplier.
 * Returns how many went on — 0 when the card is not a permanent on the field or the amount is not
 * positive, in which case nothing changes and nothing is reported.
 */
export function placePlagueOn(sink: EngineSink, card: CardInstance, amount: number): number {
  const base = Math.trunc(amount);
  if (base <= 0 || !carriesPlague(sink.state, card)) return 0;
  const placed = base * plagueMultiplierOf(sink.state, card);
  const value = plagueOn(card) + placed;
  card.counters.plague = value;
  sink.events.push({ type: "counterChanged", instanceId: card.id, counter: "plague", value, placed });
  return placed;
}

/**
 * Take up to `amount` Plague Tokens off a card (Classic #78's "remove a Plague Token", Classic #74's
 * tokens spent as mana). Returns how many came off; the count floors at 0, and a removal that takes
 * nothing reports nothing.
 */
export function removePlague(sink: EngineSink, card: CardInstance, amount: number): number {
  const had = plagueOn(card);
  const removed = Math.min(had, Math.max(0, Math.trunc(amount)));
  if (removed === 0) return 0;
  const value = had - removed;
  if (value === 0) delete card.counters.plague;
  else card.counters.plague = value;
  sink.events.push({ type: "counterChanged", instanceId: card.id, counter: "plague", value });
  return removed;
}
