// Classic+ #24 Crushing Walls (`classicplus-024`, bespoke "crushingWalls"): walls close in from the
// board's edges at its first destroyed card; each card's own death still plays.

import { testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_WALLS_HIT_AT, FX_WALLS_REACH, FX_WALLS_TAIL_MS, FX_WALLS_TRAUMA } from "../../constants.ts";
import type { FxAnchor } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the walls throw, at intensity "normal" (rule 9). */
const TUNING = {
  wallDust: { count: 24, power: 1.2 },
} as const;

/** Classic+ #24 Crushing Walls plays at its first destroyed card; empty or countered plays draw none. */
export const crushingWalls: CardRecipe = (event, p, play) => {
  if (event.type !== "destroyed" || play.seen.destroyed !== 1) return null;
  const i = p.env.intensity;
  const durationMs = p.D + FX_WALLS_TAIL_MS;
  // The walls reach the cards at FX_WALLS_HIT_AT of the cue; the dust and shake go then, inside the entry.
  const hit = Math.min(p.D, frac(FX_WALLS_HIT_AT, durationMs));
  const edge = (x: number): FxAnchor => tid(testid.board, { x, y: FX_CENTER.y });
  return {
    cues: [
      { kind: "walls", at: tid(testid.board), reach: FX_WALLS_REACH, delayMs: 0, durationMs },
      tunedBurst(i, "dust", edge(FX_WALLS_REACH), "point", hit, TUNING.wallDust),
      tunedBurst(i, "dust", edge(1 - FX_WALLS_REACH), "point", hit, TUNING.wallDust),
      ...shakeCues(i, FX_WALLS_TRAUMA, hit),
    ],
    row: "keep",
  };
};
