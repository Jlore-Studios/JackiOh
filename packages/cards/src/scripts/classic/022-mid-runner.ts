// C #22 Mid Runner (SPEC §8.6 row 22). (1) Unit, Human, Common, 2/1 → 4/2.
//   Base:    "Cry: If this is in midlane, Tribute it. If you had {threshold} or more mana when you
//            played this, bounce {bounces|random enemy permanent|random enemy permanents}." — 4, 2
//   Radiant: the same text but bounce 3; its Radiant face is its doubled stats, the designer's word,
//            recorded against R275 in `docs/radiant-audit.md`.
//   Engine:  "Two independent checks. Midlane is computed from the lane count (R665: an odd count's
//            center lane, an even count's both center lanes). "When you played this" is the
//            mana before paying for it, recorded as the play begins (§10.5 step 1). Two different
//            random enemy permanents (R60; Radiant: three) go to their owners' hands (Bounce, §6.3:
//            the hand cap applies and tokens vanish). Tunes: mana threshold 4 ↓; bounces 2 ↑."
//
// The two checks are read as the Cry begins and act in the text's order. In midlane the card
// Tributes itself: §6.3's Sacrifice, a death (Death, Reborn and the destroyed count), which bypasses
// Indestructible. Anywhere else it stays.
//
// "If you had N or more mana when you played this" is the mana its controller held as the play began
// (§10.5 step 1), before paying — the engine's record of it, `ctx.manaBeforePlay`, which the played
// card's own Cry carries (a cast's too, R70, read as the cast began). A Cry run any other way (another
// card triggering it, E13) was not played now, so it reads its controller's mana as it runs.
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
  midlaneLanesOf,
  param,
  slotOf,
  unspentManaOf,
  type ConditionContext,
  type EffectContext,
  type Effect,
  type Script,
} from "@jackioh/engine";
import { bounce, cardsInScope, forEachCard, sacrifice } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-022");

/** "{threshold} or more mana": the one test both the Cry and the glow make. */
function enoughMana(ctx: EffectContext | ConditionContext, mana: number): boolean {
  return mana >= param(ctx, "threshold");
}

/** The mana its controller had as the play began (§10.5 step 1), or, for a Cry not played now, now. */
function manaWhenPlayed(ctx: EffectContext): number {
  return ctx.manaBeforePlay ?? unspentManaOf(ctx.state, ctx.controller);
}

const cry = (ctx: EffectContext): Effect[] => {
  const self = ctx.self;
  if (self === null) return [];
  // R665: midlane is computed from the lane count, never hardcoded — read through the engine
  // helper, since card scripts never reach into state themselves (M3-T1).
  const inMidlane =
    midlaneLanesOf(ctx.state, ctx.controller).includes(slotOf(ctx.state, self)?.lane ?? -1) &&
    self.zone.z === "field" &&
    self.zone.row === "units";
  const bounces = param(ctx, "bounces");
  return [
    ...(inMidlane ? [sacrifice({ target: { of: "self" } })] : []),
    ...(enoughMana(ctx, manaWhenPlayed(ctx))
      ? [
          forEachCard({
            // R60: different cards, drawn as the Cry reaches this clause.
            cards: (c) => c.rng.shuffle(cardsInScope(c, { side: "enemy", rows: ["units", "backrow"] })).slice(0, bounces),
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
