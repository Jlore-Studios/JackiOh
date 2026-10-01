// C #32 Felinor Feelings (SPEC §8.6 row 32). Spell, Felinor, cost 0, Rare.
//   Base:    "Steal an enemy permanent in a lane where you control a (1) Cost Unit."
//   Radiant: "Summon a Felinor Token. Then steal an enemy permanent in a lane where you control a (1)
//             Cost Unit."
//
// "In a lane where you control a (1) Cost Unit": a lane (§3.1) holding a Unit its caster controls —
// the top of a unit pile, since a card dormant under a Stack is not on the field (R13) — whose own
// cost is exactly (1): R396's reading (`costNow`), R65's cost where it stands, its cost changes
// counting, an X Unit at the X it was played for (0 with none chosen). The enemy permanent may be the
// top of their unit pile in that lane or their backrow card in it, face-down included (whose identity
// the thief reads from then on, and its owner no longer does, R33).
//
// Base: a declared target (R81), filtered by `targetChecks` (§10.6) so `legalActions` and the client
// offer only the permanents in such lanes. A board that offers none does not refuse the play: it is
// legal with no target and the steal fizzles (§8's conventions, R90). The steal is §6.3's (R15: its
// own lane on your side if free, else the first free zone of its row; none, and it stays), an entry
// that leaves a stolen Unit summoning sick (R171).
//
// Radiant: first a Felinor Token (a (1) Cost Unit) in your leftmost open unit zone (R64); then the
// steal is a prompt, so the token's own lane counts. A full board summons no token and the prompt
// reads the lanes as they are; with no such permanent the prompt is never opened.

import { opponentOf } from "@jackioh/shared";
import type { CardInstance, EffectContext, GameState, Script, TargetCheck } from "@jackioh/engine";
import { activeUnitsOf, costNow } from "@jackioh/engine";
import { chooseTargetWhere, steal, summon } from "@jackioh/engine/effects";
import type { PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-032");

/** §7's shared Felinor Token, a (1) Cost Unit. */
const FELINOR_TOKEN = "core-t-felinor";
/** The name of the lane rule in `targetChecks`, as the declaration's filter names it. */
const LANE_RULE = "felinorLane";
/** The continuation the Radiant face's prompt re-enters. */
const STEAL = "feelings-steal";

/** The lanes where `player` controls a Unit that costs exactly (1) (R396). */
function feelingLanes(state: GameState, player: PlayerId): Set<number> {
  const lanes = new Set<number>();
  for (const unit of activeUnitsOf(state, player)) {
    if (unit.zone.z === "field" && costNow(state, unit) === 1) lanes.add(unit.zone.lane);
  }
  return lanes;
}

/** An enemy permanent standing in one of those lanes. */
function inFeelingLane(state: GameState, player: PlayerId, card: CardInstance | null): boolean {
  if (card === null || card.zone.z !== "field") return false;
  if (card.controller !== opponentOf(player)) return false;
  return feelingLanes(state, player).has(card.zone.lane);
}

const laneRule: TargetCheck = ({ state, player, candidate }) => inFeelingLane(state, player, candidate);

export const base: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit", "backrow"], check: LANE_RULE } }],
  targetChecks: { [LANE_RULE]: laneRule },
  cry: () => [steal({ target: { of: "chosen" } })],
};

export const radiant: Script = {
  cry: () => [
    summon({ defId: FELINOR_TOKEN }),
    chooseTargetWhere({
      step: STEAL,
      scope: { side: "enemy", of: ["unit", "backrow"] },
      where: (ctx: EffectContext, card) => inFeelingLane(ctx.state, ctx.controller, card),
      prompt: "Steal an enemy permanent in a lane where you control a (1) Cost Unit",
    }),
  ],
  resume: { [STEAL]: () => [steal({ target: { of: "chosen" } })] },
};
