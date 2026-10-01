// C #17 Counterspell (SPEC §8.6 row 17). Trap, cost 2, Common.
//   Base:    "Activates when your opponent plays a Spell: Counter it."
//   Radiant: "Activates when your opponent plays a Spell: Counter it. Add a copy of it to your hand.
//             The copy costs ({setCost})."
//
// Counter (§6.3, B5 E1, R448) answers the opponent's `cardAnnounced` of a Spell — the Spell type, so a
// Field Spell, a Trap or a Unit leaves it set — in §10.5's announce window, before the card moves: a
// play or a cast (a cast is a play and is announced too, R70). The countered Spell never resolves and
// goes to its owner's graveyard; it is treated as never played (no spell script, no `cardPlayed` or
// `cardResolved`, not counted by the turn's or the game's plays, Combo, Quickstriker or Ceaseless
// Void, no Echo repeats), and the mana and Tributes paid for it stay spent. With two Counters set, the
// first to resolve cancels the play and the other finds no live announce — the window never offers it
// the cancelled card, so it stays set (R448). A Trap is consumed when it fires (§5.1).
//
// The condition lives in `when`, never in an early return from `run` (R99, R61): a Unit play must
// leave the trap armed and face-down.
//
// Radiant: a fresh copy of the countered Spell (its Radiant flag kept, R57) goes to your hand as your
// card, costing ({setCost}) (`costOverride`, which persists in every zone, R78); a full hand burns it
// (R317), and in your hand the opponent's view names it no more (R97).

import type { GameEvent } from "@jackioh/shared";
import type { Effect, EffectContext, Script, TrapTrigger } from "@jackioh/engine";
import { findInstance, param } from "@jackioh/engine";
import { addToHand, counterPlay } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-017");

type Announced = Extract<GameEvent, { type: "cardAnnounced" }>;

/** "When your opponent plays a Spell": the announce of an opponent's Spell-type play or cast. */
function opponentsSpell(ctx: EffectContext & { event: GameEvent }): Announced | null {
  const event = ctx.event;
  if (event.type !== "cardAnnounced") return null;
  if (event.player === ctx.controller) return null;
  return event.cardType === "Spell" ? event : null;
}

function counterIt(ctx: EffectContext, announced: Announced, copy: boolean): Effect[] {
  const countered = counterPlay({ target: { of: "instance", instanceId: announced.instanceId } });
  if (!copy) return [countered];
  // Read before the counter moves it: the copy keeps the countered card's face (R57).
  const radiant = findInstance(ctx.state, announced.instanceId)?.radiant === true;
  return [
    countered,
    addToHand({ defId: announced.defId, radiant, costOverride: param(ctx, "setCost") }),
  ];
}

function counterspell(copy: boolean): TrapTrigger {
  return {
    id: "counterspell",
    on: ["cardAnnounced"],
    when: (ctx) => opponentsSpell(ctx) !== null,
    run: (ctx) => {
      const announced = opponentsSpell(ctx);
      return announced === null ? [] : counterIt(ctx, announced, copy);
    },
  };
}

export const base: Script = { triggers: [counterspell(false)] };

export const radiant: Script = { triggers: [counterspell(true)] };
