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
// ARMING (R61, R472). `traps.ts` rules that `run` returning `[]` is "a trap that fired for nothing" and
// "can never mean 'this event was not mine'", so every condition that must leave the trap armed and
// face-down lives in the `when` predicate: the opponent's play, a Unit, and a Unit still standing on
// the field from that play (`cardResolved.permanent`, which `traps.standingEvent` reads again for
// each trap). So a Spell, a Field Spell, a Trap, the controller's own Unit, and a Unit that left the
// field during its own resolution — its Cry killed it, an earlier trap answering the same play took
// it — all leave Sheepish set (R472): there is no played Unit left to transform, and Hearthstone's
// "after your opponent plays a minion" secrets wait for one that is.
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
      if (defOf(ctx.state, event.defId).type !== "Unit") return false;
      // R472: the played Unit still stands on the field from that play, or there is nothing to
      // transform and the trap stays set.
      return event.permanent;
    },
    run: (ctx) => {
      const event = ctx.event;
      if (event.type !== "cardResolved") return [];
      // Named by the event, not by a TargetSpec: nobody chose this unit, the play produced it.
      const effects: Effect[] = [transform({ instanceId: event.instanceId, defId: SHEEP_TOKEN })];
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
