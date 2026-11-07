// R3338000, S #4 Olivian Expanse (`terminusRift`): a rift tears open over the board's centre, the
// Terminus cards circle in it as stars (the pool, every one: never which it gave), and one falls out of
// it into its caster's hand as a card back (R202) when the hold ends. It draws the Discover's
// `addedToHand` in place of that event's own row.

import { animTestid } from "../../game/animations.ts";
import { sideOf } from "../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../build.ts";
import { FX_SLAM_AT, FX_TERMINUS_TRAUMA } from "../constants.ts";
import type { FxCue } from "../types.ts";
import { BOARD, TERMINUS_TUNING, beats, eventsOf, momentPiece, type TerminusPlanner } from "./build.ts";
import { TERMINUS_IDS } from "./ids.ts";

export const rift: TerminusPlanner = (m) => {
  const added = eventsOf(m.entry, ["addedToHand"]);
  const cues: FxCue[] = [
    momentPiece(m, BOARD, beats(TERMINUS_IDS.length, m.S)),
    ringCue(m.D, "void", BOARD, 0),
    { ...tunedBurst(m.i, "void", BOARD, "area", 0, TERMINUS_TUNING.rift), colors: m.look.colors },
    ringCue(m.D, "void", BOARD, m.S),
  ];
  const first = added[0];
  if (first !== undefined) {
    // The star falls over the motion after the hold (unheld, after the swallow), landing inside the entry.
    const fall = frac(FX_SLAM_AT, m.M > 0 && m.H > 0 ? m.M : m.D - m.S);
    const hand = tid(animTestid.hand(sideOf(m.view, first.player)));
    cues.push(
      { kind: "ghost", from: BOARD, to: hand, delayMs: m.S, durationMs: Math.max(1, fall) },
      { ...tunedBurst(m.i, "sparkle", hand, "point", m.S + fall, TERMINUS_TUNING.arrive), colors: m.look.colors },
      ...shakeCues(m.i, FX_TERMINUS_TRAUMA, m.S + fall),
    );
  }
  return { cues, replaces: new Set(added) };
};
