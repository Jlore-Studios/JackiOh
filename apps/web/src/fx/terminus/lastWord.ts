// R3338000, S #4.4 Last Word (`lastWord`): as the lethal hit lands, everything freezes in mid-strike under
// frost, the hit on its controller's hero cracks and shatters halfway through the hold, and the turn ends
// (its own banner follows). The next turn opens with every Unit of theirs lit for Charge and a countdown
// that burns away (`planTerminusFollow`). It draws its run (its firing, its way to the graveyard and the
// turns it rearranges) in place of their own rows.

import { sideOf, testid } from "../../game/contract.ts";
import { frac, tid, tunedBurst } from "../build.ts";
import { FX_CRACK_TAIL_MS } from "../constants.ts";
import type { FxCue } from "../types.ts";
import { BOARD, TERMINUS_TUNING, momentPiece, type TerminusPlanner } from "./build.ts";

export const lastWord: TerminusPlanner = (m) => {
  const hero = tid(testid.hero(sideOf(m.view, m.signature.player)));
  const at = frac(TERMINUS_TUNING.freezeAt, m.S);
  const cues: FxCue[] = [
    momentPiece(m, BOARD, [at]),
    { kind: "crack", at: hero, delayMs: at, durationMs: m.D - at + FX_CRACK_TAIL_MS },
    { ...tunedBurst(m.i, "shard", hero, "area", at, TERMINUS_TUNING.shatter), colors: m.look.colors },
  ];
  return { cues, replaces: new Set(m.entry.events) };
};
