// C #22 Mid Runner (SPEC §8.6 row 22). (1) Unit, Human, Common, 2/1 → 4/2.
//   Base:    "Cry: If this is in lane 3, Tribute it. If you had {threshold} or more mana when you
//            played this, bounce {bounces|random enemy permanent|random enemy permanents}." — 4, 2
//   Radiant: the same text; its Radiant face is its doubled stats, the designer's word, recorded
//            against R275 in `docs/radiant-audit.md`.
//   Engine:  "Two independent checks. Midlane is lane 3 (`MID_LANE`). "When you played this" is the
//            mana before paying for it, recorded as the play begins (§10.5 step 1). Two different
//            random enemy permanents (R60) go to their owners' hands (Bounce, §6.3: the hand cap
//            applies and tokens vanish). Tunes: mana threshold 4 ↓; bounces 2 ↑."
//
// The two checks are read as the Cry begins and act in the text's order. In `MID_LANE` the card
// Tributes itself: §6.3's Sacrifice, a death (Death, Reborn and the destroyed count), which bypasses
// Indestructible. Anywhere else it stays.
//
// "If you had N or more mana when you played this" is the mana its controller held as the play began
// (§10.5 step 1), before paying. `manaWhenPlayed` below is the one place that reads it. The engine
// keeps no record of that number yet (requested: "mana before paying, recorded at §10.5 step 1"), so
// until it does the read is the mana its controller holds now plus what this play paid for it
// (`costPaidThisTurn`, the turn log's price, a cast's 0 included, R70): the price is the only change
// to its controller's mana between §10.5 step 1 and the Cry. Swapping in the record is one line.
//
// The bounce: that many DIFFERENT enemy permanents (R60), fewer if fewer exist — the tops of their
// unit piles (R13) and their backrow cards, face-down ones included — drawn from the match rng as the
// Cry reaches it, each to its owner's hand: the hand cap burns one that does not fit (R4, R317) and
// a unit token ceases to exist (R11). A bounced face-down card lands in a hand its bouncer may not
// read, so no view of theirs names it (R97).
//
// R195: in hand the card glows when playing it now would bounce — its controller's mana now is the
// mana it would have "when it was played" — read by the same threshold test the Cry uses.
//
// Both numbers are the declared `threshold` and `bounces` (R386), read through `param`.

import {
  MID_LANE,
  activeUnitsOf,
  cardAt,
  costPaidThisTurn,
  param,
  slotOf,
  slotsOf,
  unspentManaOf,
  type CardInstance,
  type ConditionContext,
  type EffectContext,
  type Effect,
  type Script,
} from "@jackioh/engine";
import { bounce, forEachCard, sacrifice } from "@jackioh/engine/effects";
import { opponentOf } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-022");

/** "{threshold} or more mana": the one test both the Cry and the glow make. */
function enoughMana(ctx: EffectContext | ConditionContext, mana: number): boolean {
  return mana >= param(ctx, "threshold");
}

/**
 * The mana its controller had as the play of `self` began (§10.5 step 1). Requested engine record:
 * "mana before paying, recorded at §10.5 step 1". Until then: the mana held now plus this play's price.
 */
function manaWhenPlayed(ctx: EffectContext, self: CardInstance): number {
  return unspentManaOf(ctx.state, ctx.controller) + (costPaidThisTurn(ctx.state, ctx.controller, self) ?? 0);
}

/** Every permanent the opponent controls: the tops of their unit piles, then their backrow, lane order. */
function enemyPermanents(ctx: EffectContext): CardInstance[] {
  const enemy = opponentOf(ctx.controller);
  const backrow = slotsOf(enemy, "backrow").flatMap((ref) => {
    const card = cardAt(ctx.state, ref);
    return card === null ? [] : [card];
  });
  return [...activeUnitsOf(ctx.state, enemy), ...backrow];
}

const cry = (ctx: EffectContext): Effect[] => {
  const self = ctx.self;
  if (self === null) return [];
  const inMidlane = slotOf(ctx.state, self)?.lane === MID_LANE && self.zone.z === "field" && self.zone.row === "units";
  const bounces = param(ctx, "bounces");
  return [
    ...(inMidlane ? [sacrifice({ target: { of: "self" } })] : []),
    ...(enoughMana(ctx, manaWhenPlayed(ctx, self))
      ? [
          forEachCard({
            // R60: different cards, drawn as the Cry reaches this clause.
            cards: (c) => c.rng.shuffle(enemyPermanents(c)).slice(0, bounces),
            each: (instanceId) => bounce({ target: { of: "instance", instanceId } }),
          }),
        ]
      : []),
  ];
};

export const base: Script = {
  cry,
  // R195: in hand, whether playing it now would bounce.
  conditionMet: (ctx) => ctx.zone === "hand" && enoughMana(ctx, unspentManaOf(ctx.state, ctx.controller)),
};

// The same script: the Radiant face is the base text on doubled stats (docs/radiant-audit.md).
export const radiant: Script = base;
