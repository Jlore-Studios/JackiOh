// C #72 Grand Counterspell (SPEC §8.6 row 72, BUILD M9 Classic row C 72). (2) Trap, Rare.
//   Base:    "When your opponent plays a Spell, Field Spell, Trap or Field Trap: Counter it."
//   Radiant: "When your opponent plays a Spell, Field Spell, Trap or Field Trap: Steal it."
//   Engine:  "Counter (§6.3), in §10.5's announce window, on the opponent's announce of any non-Unit
//            card, so a Field Spell or a Trap is countered before it reaches the backrow: it never
//            resolves and goes to its owner's graveyard (a face-down set is announced to you by its zone
//            only, but the engine knows what it is). Radiant: Steal off the field (§6.3 Steal; the owner
//            changes, §3.2, R12): the card is countered and moves to your hand as yours, where your hand
//            cap applies (a burned card goes to your graveyard). The designer named it Counterspell, as
//            C #17; it is renamed so the two never collide (R381), and no card names either."
//
// Readings:
//   - The trigger is B5 E1's announce (`cardAnnounced`), answered in the window between §10.5
//     steps 3 and 4 by the trap engine, which fires a face-down Trap the moment an event it watches is
//     dispatched (§10.3) and spends it afterwards (§5.1: a Trap goes to the graveyard once it fires).
//   - "Your opponent plays" is the announce's `player`; a cast is a play and is announced too (R70).
//   - "A Spell, Field Spell, Trap or Field Trap" is every type but Unit, read off the announce's
//     `cardType` — the type the card is played as (a face with its own type included, B2.7).
//   - The counter names the announced card (`counterPlay`'s target), so a second counter answering the
//     same announce finds no live play and stays set (B5 E1). A countered play is treated as never
//     played: no `cardPlayed`, nothing counted, mana and Tributes spent (§6.3 Counter).
//   - Radiant: `counterPlay({ to: "thief" })` is E2's steal off the field — the owner becomes this
//     trap's controller (R12), the hand cap applies (R317), and the card is hidden in the opponent's
//     view once in the thief's hand (R97).
// No tuned numbers.

import type { Effect, Script, TriggerDef } from "@jackioh/engine";
import { counterPlay, type CounterDestination } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-072");

/** The opponent's announce of anything but a Unit (§6.3 Counter, B5 E1). */
function answers(to: CounterDestination): TriggerDef {
  return {
    id: "grand-counter",
    on: ["cardAnnounced"],
    when: (ctx) =>
      ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller && ctx.event.cardType !== "Unit",
    run: (ctx): Effect[] =>
      ctx.event.type === "cardAnnounced"
        ? [counterPlay({ to, target: { of: "instance", instanceId: ctx.event.instanceId } })]
        : [],
  };
}

export const base: Script = {
  triggers: [answers("graveyard")],
};

/** E2, R12: countered and taken into this trap's controller's hand, as theirs. */
export const radiant: Script = {
  triggers: [answers("thief")],
};
