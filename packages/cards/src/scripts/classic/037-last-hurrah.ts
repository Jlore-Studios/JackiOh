// C #37 Last Hurrah (SPEC §8.6 row 37). Spell, cost 0, Epic.
//   Base:    "Draw your deck. At the end of this turn, discard your hand."
//   Radiant: "Draw your deck. At the end of your next turn, discard your hand."
//
// "Draw your deck" is R58's "draw your whole library": as many draws as the library holds when the
// effect starts, each an ordinary §2.4 draw — a cast-on-draw card is cast (R58), a card past the hand
// cap of 10 burns into the graveyard (R4, R317), and a draw a draw limit stops does not happen at all
// (§2.4). An empty library means no draws, so no fatigue.
//
// Then a delayed effect (§10.1, `discardHandAtTurnEnd`) discards the whole hand, with no prompt (there
// is nothing to choose), in the end-of-turn delayed-effect step (R62): at the end of this turn on the
// base face, at the end of its controller's next turn on the Radiant face, taking the hand held then.
// It is a discard (§6.3), so C #64 Malzahar's Recycler sees each card. By design a fatigue clock.

import type { Script } from "@jackioh/engine";
import { zoneCount } from "@jackioh/engine";
import { discardHandAtTurnEnd, draw } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-037");

function lastHurrah(turn: "this" | "next"): Script {
  return {
    cry: (ctx) => [
      draw({ count: zoneCount(ctx.state, ctx.controller, "library") }),
      discardHandAtTurnEnd({ turn }),
    ],
  };
}

export const base: Script = lastHurrah("this");

export const radiant: Script = lastHurrah("next");
