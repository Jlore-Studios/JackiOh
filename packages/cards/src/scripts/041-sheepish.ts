// #41 Sheepish (SPEC §8.2). Trap, cost 1, Epic.
//   Base:    "When your opponent plays a Unit and its Cry resolves: Transform it into a Sheep Token."
//   Radiant: the same, "Add a Lava Golem to your hand. It costs (0)." — the Radiant face adds the
//            Lava Golem and keeps every clause of the base face (§8 Conventions, R277).
//
// TIMING (R427, patch v0.2.0; rewrites R17's Sheepish half). Sheepish no longer costs the Unit its
// Cry: it waits for the play to resolve — the Cry and every Echo repeat — and then turns the Unit into
// a Sheep. That moment is §10.5 step 7's `cardResolved`, the event #60 Bear Honeypot, #33 Unstable
// Clone Machine and #85 Unlicensed Experimentation answer too (R17's other half, R61). A cast Unit
// resolves the same way (R70), so a cast is answered after its Cry as well. A Unit with no Cry is
// answered at the same step: "its Cry resolves" names the moment, not a condition on the text.
//
// `cardResolved` rather than `summoned`: the condition is "your opponent PLAYS a Unit", and the play
// pipeline's step-7 event carries the player who played it, where `summoned` also covers Recruit,
// copies, tokens and Reborn — none of which is a play (R1, R61).
//
// ARMING (R61). `traps.ts` rules that `run` returning `[]` is "a trap that fired for nothing" and "can
// never mean 'this event was not mine'", so every condition that must leave the trap armed and
// face-down lives in the `when` predicate: the opponent's play, and a Unit. A Spell, a Field Spell, a
// Trap, or the controller's own Unit therefore leaves Sheepish set.
//
// A UNIT THAT HAS LEFT (R427, R174). The play of a Unit is what Sheepish answers, so it fires on it
// even when the Unit is no longer on the field by then — its own Cry took it off, or an earlier trap
// answering the same play did: `cardResolved.permanent` says so (`traps.standingEvent` reads it again
// for each trap), the Transform then has no Unit in play to land on and finds nothing — it never
// reaches into a graveyard or a hand, or onto a Reborn body, a new arrival (R83) — and the trap is
// consumed, the Radiant face's Lava Golem still added (R120).
//
// IMMUTABLE (R17, R23). `transform` already refuses an Immutable target, which is exactly R17's
// "Sheepish on an Immutable unit still fires and is consumed with no effect". This card neither
// checks Immutable nor consumes itself: `fireTrap` emits `trapFired`, runs the state check and
// consumes the trap "whatever its effects achieved". R33's face-down identity is the view's.

import type { Effect, Script, TrapTrigger } from "@jackioh/engine";
import { defOf } from "@jackioh/engine";
import { addToHand, transform } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-041");

/** §7: the Sheep Token, whose only generator is this card. */
const SHEEP_TOKEN = "core-t-sheep";
/** #55 Lava Golem, added by the radiant text at cost 0 (`costOverride`, R65). */
const LAVA_GOLEM = "core-055";

/** The two faces differ only in whether the Lava Golem comes with the Sheep. */
function sheepish(lavaGolem: boolean): TrapTrigger {
  return {
    id: lavaGolem ? "sheepish-radiant" : "sheepish",
    on: ["cardResolved"],
    when: (ctx) => {
      const event = ctx.event;
      if (event.type !== "cardResolved") return false;
      // §8: "your opponent". A trap never answers its own controller's play.
      if (event.player === ctx.controller) return false;
      // §5.1: a Unit, so a Spell or a backrow card leaves the trap armed (R61).
      return defOf(ctx.state, event.defId).type === "Unit";
    },
    run: (ctx) => {
      const event = ctx.event;
      if (event.type !== "cardResolved") return [];
      // Named by the event, not by a TargetSpec: nobody chose this unit, the play produced it. R427:
      // only a Unit still in play from that play is transformed; one that has left finds nothing.
      const effects: Effect[] = event.permanent
        ? [transform({ instanceId: event.instanceId, defId: SHEEP_TOKEN })]
        : [];
      // R120: §8's conventions make an "Also" clause independent, so it still lands when an
      // Immutable target refused the Transform (R17, R23) and the trap is still consumed (R61).
      if (lavaGolem) effects.push(addToHand({ defId: LAVA_GOLEM, costOverride: 0 }));
      return effects;
    },
  };
}

const baseTrigger = sheepish(false);
const radiantTrigger = sheepish(true);

export const base: Script = { triggers: [baseTrigger] };

export const radiant: Script = { triggers: [radiantTrigger] };
