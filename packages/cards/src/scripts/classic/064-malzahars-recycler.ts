// C #64 Malzahar's Recycler (SPEC §8.6 row 64, BUILD M9 Classic row C 64). (2) Field Spell, Rare.
//   Base:    "End of turn: Discard 2 cards. / Whenever you discard cards, draw that many."
//   Radiant: "End of turn: Discard 2 cards. / Whenever you discard cards, draw your deck."
//   Engine:  "The end-of-turn discard is 2 random cards (R654; fewer in hand: all of them). The draw
//            answers your `discarded` events one effect at a time: an effect that discards 2 draws 2.
//            Radiant: 'draw your deck' (R58, the deck's size as it starts) once per discarding effect.
//            Every discard of yours counts: your own, C #15 Nose Hunter's random one, C #8 Pickle's,
//            C #37 Last Hurrah's; a card crumbling from Brittle (§6.1, R385) is not a discard.
//            Tunes: none."
//
// Readings:
//   - The end-of-turn discard is §6.2's end-of-turn hook (its controller's turn, while it acts on the
//     field): 2 random cards of its controller's hand (R654), all of a smaller hand, nothing from an
//     empty one.
//   - "Whenever you discard cards" answers the `discarded` events of cards its controller owned in
//     hand as they went — whoever's effect discarded them (an opponent's C #8 Pickle makes you
//     discard), never the opponent's discards. The trigger answers each discarded card, one draw each,
//     so an effect that discards 2 draws 2, after that effect has finished (the draws are queued
//     triggers, §10.3); a Brittle crumble emits `crumbled`, not `discarded`, and is not answered.
//   - Radiant: each answer draws the deck as its size stands then (R58), so the first answer to a
//     discarding effect draws the whole deck and the rest of that effect's answers find it empty and
//     draw nothing — no fatigue (R87: an empty library draws nothing). Most of it burns at the hand cap
//     (§2.4); that is the card.

import type { Effect, EffectContext, Script, TriggerDef } from "@jackioh/engine";
import { zoneCount } from "@jackioh/engine";
import { discardRandom, draw } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-064");

/** "End of turn: Discard 2 cards." — random (R654). */
const END_OF_TURN_DISCARDS = 2;

const endOfTurn = (): Effect[] => [discardRandom({ count: END_OF_TURN_DISCARDS })];

/** Whether this event is a card its controller discarded. */
function yourDiscard(ctx: EffectContext & { event: GameEvent }): boolean {
  return ctx.event.type === "discarded" && ctx.event.owner === ctx.controller;
}

function answer(draws: (ctx: EffectContext) => Effect[]): TriggerDef {
  return {
    id: "recycle",
    on: ["discarded"],
    when: (ctx) => yourDiscard(ctx),
    run: (ctx) => (yourDiscard(ctx) ? draws(ctx) : []),
  };
}

/** "Draw that many": one draw for each card discarded. */
const drawOne = (): Effect[] => [draw({ count: 1 })];

/** "Draw your deck": the deck's size as the draw starts (R58). */
const drawDeck = (ctx: EffectContext): Effect[] => [draw({ count: zoneCount(ctx.state, ctx.controller, "library") })];

export const base: Script = {
  endOfTurn,
  triggers: [answer(drawOne)],
};

export const radiant: Script = {
  endOfTurn,
  triggers: [answer(drawDeck)],
};
