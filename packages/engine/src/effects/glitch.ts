// The Glitch token's Cry (SPEC §7, R664): "When played, does one of the following at random".
//
// One draw of the match rng (`rollGlitchOutcome`) picks one of the four outcomes, and each outcome is
// its own effect. None of them is built yet: resetting the match, swapping the seats, putting other
// games' last boards on the field and voiding the match each wait for a decision on issue #170 (its
// plan's open questions), and each outcome does nothing until then. The draw is made all the same,
// so the outcome a seed picks, and every later draw of the match, stays where it is when they land.

import type { Rng } from "../rng";
import type { Effect } from "../script";

/** R664: Glitch's four outcomes, in the issue's order; the roll indexes this list. */
export const GLITCH_OUTCOMES = ["reset", "swapSeats", "foreignBoards", "void"] as const;

export type GlitchOutcome = (typeof GLITCH_OUTCOMES)[number];

/** R664: the outcome one draw of the match rng picks, each of the four as likely. */
export function rollGlitchOutcome(rng: Rng): GlitchOutcome {
  return GLITCH_OUTCOMES[rng.int(GLITCH_OUTCOMES.length)] ?? GLITCH_OUTCOMES[0];
}

/** An outcome that is not built yet: it changes nothing (R664). */
function notYetBuilt(outcome: GlitchOutcome): Effect {
  return {
    kind: `glitch:${outcome}`,
    apply(): void {
      // R664: waiting on issue #170's decisions; nothing happens.
    },
  };
}

/** R664: what each outcome does. */
const OUTCOME_EFFECTS: Readonly<Record<GlitchOutcome, Effect>> = {
  // Reset the match: start playing again from the beginning.
  reset: notYetBuilt("reset"),
  // The players swap seats, and the match's result counts for the other account from here on.
  swapSeats: notYetBuilt("swapSeats"),
  // Another match's endgame boards replace every zone, the life totals kept.
  foreignBoards: notYetBuilt("foreignBoards"),
  // Both players are disconnected and the match is treated as if it never existed.
  void: notYetBuilt("void"),
};

/** R664: Glitch's Cry — roll an outcome on the match rng and run it. */
export function glitchOutcome(): Effect {
  return {
    kind: "glitch",
    apply(ctx): void {
      OUTCOME_EFFECTS[rollGlitchOutcome(ctx.rng)].apply(ctx);
    },
  };
}
