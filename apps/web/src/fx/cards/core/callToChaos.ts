// R3402001: #95 Call to Chaos (Core Edition) (`core-095`, bespoke "chaosReels"), a Legendary Spell's
// signature (R3332000). At its cast a crystal slot-wheel drops over the board in Legendary rays and a
// prismatic ring as the lever is yanked, and its reels blur in magenta sparks, left, middle and right. On
// its `chaosRolled` the reels slam to the rolled effect, shown to both seats in the roll's own reveal
// (chaos.ts, R436), and a gold ring rings out with the jackpot's shake. A Spell's budget ends a beat into
// its roll, so its own reveal stands FX_REELS_MS in place of the shared one, whose sparks and tail would
// run past it; the effect's own events then play their rows. It anchors on the board and the viewport and
// reads only the roll's public effect keys (R202).

import { testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { chaosCues, chaosLandAt, chaosRollOf } from "../../chaos.ts";
import { FX_CENTER, FX_REELS_MS, FX_REELS_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxChaosCue, FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { isFirstBeat } from "./flourish.ts";

/** Every particle count the reels throw, at intensity "normal" (rule 9). */
const TUNING = {
  reel: { count: 16, power: 1 },
} as const;

/** Where on the board each of the three reels blurs, left to right, and when, as shares of the cast's travel. */
const REELS: readonly { x: number; share: number }[] = [
  { x: 0.4, share: 0 },
  { x: 0.5, share: 0.5 },
  { x: 0.6, share: 1 },
];

/** The reels' sparks: Chaos's magenta first. */
const REEL_COLORS = FLAVOR_LOOKS.Chaos.colors;

const PANEL: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

export const chaosReels: CardRecipe = (event, p, play) => {
  // R2722002: a Cloaked copy plays at its reveal, the `cast` its `trapFired` opens.
  if (play.hook !== undefined && play.hook !== "cast") return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const centre = tid(testid.board, FX_CENTER);
  if (isFirstBeat(event, play)) {
    return {
      cues: [
        raysCue(p.D, "legendary", centre, t.G),
        ringCue(p.D, "prismatic", centre, t.G),
        ...REELS.map((reel) => ({ ...tunedBurst(i, "spark", tid(testid.board, { x: reel.x, y: FX_CENTER.y }), "point", frac(reel.share, t.L - t.T) + t.T, TUNING.reel), colors: REEL_COLORS })),
      ],
      row: "keep",
    };
  }
  if (event.type !== "chaosRolled" || chaosRollOf(event) === null) return null;
  // The roll's own reveal, its lines landing within FX_REELS_MS, standing that long and no longer.
  const panel = chaosCues(event, FX_REELS_MS, i).find((cue): cue is FxChaosCue => cue.kind === "chaos");
  if (panel === undefined) return null;
  const firstLand = frac(chaosLandAt(0, panel.lines.length), FX_REELS_MS);
  const cues: FxCue[] = [{ ...panel, durationMs: FX_REELS_MS }, ringCue(p.D, "gold", PANEL, t.T), ...shakeCues(i, FX_REELS_TRAUMA, firstLand)];
  return { cues, row: "replace" };
};
