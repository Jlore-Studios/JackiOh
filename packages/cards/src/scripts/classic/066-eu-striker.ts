// C #66 EU Striker (SPEC §8.6 row 66, BUILD M9 Classic row C 66). (2) Unit, Human, 5/4 → 10/8, Common.
//   Base:    "While this is in your hand: After you play a Unit, summon this. / After you play a card,
//            return this to your hand."
//   Radiant: "Rush / (the same)."
//   Engine:  "Hand and deck triggers (§6.2): a hand trigger on your `cardPlayed` of a Unit, after it
//            resolves, summoning this (no Cry, R1; summoning sick, §4.1) into your leftmost open,
//            unlocked, unreserved unit zone (R64; none open: it stays in hand); a field trigger on your
//            `cardPlayed` of any card, returning this to your hand (R78's reset; the hand cap applies).
//            Neither trigger answers the play that moved the card (R401, R119): the Unit that summons it
//            doesn't bounce it, and the card that bounces it doesn't summon it back. Tunes: none."
//
// Readings:
//   - R548: both triggers answer a play of yours once it has resolved — §10.5 step 7's `cardResolved`,
//     which a cast's play emits too (R70) and a countered play never does (B5 E1) — the moment the
//     card's "After you play" names, as Hearthstone's "after you play" waits for the card to resolve.
//     R401 then holds by the dispatch itself (§10.3, R212): an event is offered to the cards where they
//     stand as it is dispatched, so the `cardResolved` of the Unit that summons the Striker reaches it
//     in hand and never on the field, and the `cardResolved` of the card that bounces it reaches it on
//     the field and never in hand.
//   - Its own play does not bounce it (R119): the field trigger passes over the play naming this card,
//     the check R119 leaves to a permanent that is not a trap.
//   - "Summon this" is B5 E26's `summonThis`: from the hand only, no Cry (R1), summoning sick, R64's
//     leftmost open, unlocked, unreserved unit zone, nothing when the row is full. A summon is no play,
//     so it answers nothing that answers plays.
//   - "A Unit" is the played card's definition's type; "you" is the play's player, the Striker's
//     controller (its owner, in hand). The opponent's plays do nothing.
//   - The return is §6.3 Bounce: R78's reset, the hand cap (a burned card goes to the graveyard, §2.4).
// Rush on the Radiant face is printed (§10.4 layer 1). No tuned numbers.

import type { EffectContext, Script, TriggerDef } from "@jackioh/engine";
import { defOf } from "@jackioh/engine";
import { bounce, summonThis } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-066");

type Resolved = Extract<GameEvent, { type: "cardResolved" }>;

/** R548: a play of this card's controller's that has resolved, other than this card's own (R119). */
function yourOtherPlay(ctx: EffectContext & { event: GameEvent }): Resolved | null {
  const event = ctx.event;
  if (event.type !== "cardResolved" || event.player !== ctx.controller) return null;
  if (event.instanceId === ctx.self?.id) return null;
  return event;
}

/** While this is in your hand: after you play a Unit, summon this (B5 E26). */
const arrive: TriggerDef = {
  id: "eu-striker-arrive",
  on: ["cardResolved"],
  run: (ctx) => {
    const played = yourOtherPlay(ctx);
    return played !== null && defOf(ctx.state, played.defId).type === "Unit" ? [summonThis()] : [];
  },
};

/** On the field: after you play a card, return this to your hand (§6.3 Bounce, R78). */
const leave: TriggerDef = {
  id: "eu-striker-leave",
  on: ["cardResolved"],
  run: (ctx) => (yourOtherPlay(ctx) === null ? [] : [bounce({ target: { of: "self" } })]),
};

export const base: Script = {
  handTriggers: [arrive],
  triggers: [leave],
};

// The same script: the Radiant face differs only in what the engine reads off the catalog (its doubled
// stats and Rush).
export const radiant: Script = base;
