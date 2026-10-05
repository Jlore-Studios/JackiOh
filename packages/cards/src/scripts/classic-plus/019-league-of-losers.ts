// C+ #19 League of Losers (SPEC §8.7 row 19): summons the five Losers into unit zones 1–5, each aimed at
// its own zone (Radiant: all Radiant); the Mid Loser it summons has its Cry triggered (R411).
// An occupied or Locked zone is skipped: occupancy fizzles in the engine, and the Lock is this
// card's own override of R667, so the script skips a Locked lane itself.

import { cardAt, isLocked, summonedSoFar, type Effect, type EffectContext, type Script } from "@jackioh/engine";
import { forEachCard, summon, triggerCry } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-019");

/** The five-stack, in the order they are summoned; each goes to the unit zone of its lane (index + 1). */
const FIVE_STACK = [
  "classicplus-019-1", // Top Loser
  "classicplus-019-2", // Jungle Loser
  "classicplus-019-3", // Mid Loser
  "classicplus-019-4", // Support Loser
  "classicplus-019-5", // Bot Loser
] as const;

const MID_LOSER = "classicplus-019-3";

/** R411: the Mid Loser this list summoned into `lane`, if it is there, has its Cry triggered. */
function midLoserCry(lane: number): Effect {
  return forEachCard({
    cards: (ctx: EffectContext) => {
      const unit = cardAt(ctx.state, { player: ctx.controller, row: "units", lane });
      return unit !== null && unit.defId === MID_LOSER && summonedSoFar(ctx).includes(unit.id) ? [unit] : [];
    },
    each: (instanceId) => triggerCry({ instanceId }),
  });
}

function league(radiant: boolean): Script {
  return {
    cry: (ctx) =>
      FIVE_STACK.flatMap((defId, at) => {
        const lane = at + 1;
        // R667's card-specific override: a Locked lane is skipped (§8.7 row 19).
        if (isLocked(ctx.state, { player: ctx.controller, row: "units", lane })) return [];
        const summoned = summon({ defId, lane, radiant });
        return defId === MID_LOSER ? [summoned, midLoserCry(lane)] : [summoned];
      }),
  };
}

export const base: Script = league(false);

export const radiant: Script = league(true);
