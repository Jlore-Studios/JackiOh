// R3407000: Stinkyscape #110 iKwzki (`stinkyscape-110`, a Mythic Unit): entrance "ikwzkiDawn", bespoke
// "duoBond" (duoBond.ts), tile aura "duohalo" in Spirit, takeover "ikwzki" (takeover.ts draws her own art,
// the glass spire above a ringed planet at rose dawn).
//
// - Entrance (after the takeover's hold): she offers her hand, a rose glow on her zone, and the scene folds
//   into it as motes: a rose mote burst at each corner of the board, each flying in on Spirit's wavering
//   bolt to land at the slam, where dawn's holy rays rise over her zone, a rose and a cyan ring spread, a
//   burst of motes settles and the board shakes a little.
// - Her Cry and its bloom are duoBond.ts's: the ribbon of starlight to the permanent she Duos with and the
//   knot of motes there (`marked`), and the knot blooming as it turns Radiant (`radiantSet`).
//
// Moved from the issue's concept: the hand she offers is a glow on her zone (no figure draws a hand); the
// tile's slow halo is the aura (`duohalo`, its table entry), not a cue here.
//
// R202: it anchors on her zone and the board alone and names no card. Each beat starts inside its entry and
// ends within FX_MAX_TAIL_MS of it (R200).

import { testid } from "../../../game/contract.ts";
import { raysCue, ringCue, shakeCues, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { motifBurst, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type Entrance, type EntrancePlan } from "../recipe.ts";
import { entranceSpec, entranceTimes } from "./signature.ts";

/** Her palettes: Spirit's cyan, and Clout's rose thrown as motes. */
const CYAN = "Spirit";
const ROSE = "Clout";
/** The dawn's ring, warm under the cyan one that follows it. */
const DAWN_RING = "holy";

/** Every particle count her entrance throws at scale 1 (rule 9). */
const TUNING = {
  offer: { count: 10, power: 0.5 },
  corner: { count: 6, power: 0.5 },
  settle: { count: 18, power: 0.8 },
} as const;

/** Her entrance's moments as fractions of its motion after the hold (the cyan ring `ringStep` after the slam). */
const TIMING = { offerAt: 0, cornerAt: 0, foldAt: 0.1, ringStep: 0.08 } as const;

/** The board's corners, where the scene folds in from. */
const CORNERS: readonly FxPoint[] = [
  { x: 0.08, y: 0.12 },
  { x: 0.92, y: 0.12 },
  { x: 0.08, y: 0.88 },
  { x: 0.92, y: 0.88 },
];

/** Rose motes: Clout's colors thrown as Spirit's sparkle. */
function roseSpec(p: EntrancePlan): CueSpec {
  const spec = entranceSpec(p, ROSE);
  return { ...spec, look: { ...spec.look, particle: entranceSpec(p, CYAN).look.particle } };
}

export const ikwzkiDawn: Entrance = (p) => {
  const cyan = entranceSpec(p, CYAN);
  const rose = roseSpec(p);
  const { at, slam } = entranceTimes(p);
  const T = TIMING;
  const zone = tid(p.tgt);
  const fold = at(T.foldAt);
  return [
    motifBurst(rose, zone, "point", at(T.offerAt), TUNING.offer),
    ...CORNERS.map((corner) => motifBurst(rose, tid(testid.board, corner), "point", at(T.cornerAt), TUNING.corner)),
    ...CORNERS.map((corner) => motifShot(cyan, "bolt", tid(testid.board, corner), zone, fold, Math.max(0, slam - fold))),
    raysCue(p.D, "holy", zone, slam),
    ringCue(p.D, DAWN_RING, zone, slam),
    ringCue(p.D, cyan.look.particle, zone, at(FX_SLAM_AT + T.ringStep)),
    motifBurst(rose, zone, "area", slam, TUNING.settle),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.ikwzkiDawn, slam),
  ];
};
