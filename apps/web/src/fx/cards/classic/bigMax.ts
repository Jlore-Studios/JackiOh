// R670: BOOM! Big Max (`classic-080`, entrance "bigBoom"): a fuse, then a blast where it lands.
// R3389001: grown into its signature (R3389000): entrance "bigBoom", bespoke "craterLock" and its aura
// "craterSmoke" on its own tile (R3400000). As its three Tributes sink, a fuse burns along its row lane by lane;
// it drops from above and lands fist-first in a white-orange blast that throws dust down its lane to the
// other side. As it lands its Crater Locks its zone (its own `locked`): the zone cracks into a smoking
// crater and an iron clamp rings shut on it. Standing, smoke curls from the crater's rim and embers tick
// round its fists. Moved: the blows that slide off it as cinders and a Trample hit blasting on to the
// hero, which come in combat, where no hook of its card runs (R2797000); combat's own rows draw them.

import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, otherSide, timesOf } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_FUSE_AT, ENTRANCE_TRAUMA, FOOT, ownWindow, SIGNATURE_TRAUMA, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec } from "./flourish.ts";

/** The look its crater smokes in. */
const PALETTE: FxPalette = "Fire";

/** Every particle count the blast throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  fuseSpark: { count: 8, power: 0.7 },
  boomFire: { count: 54, power: 1.6 },
  boomEmber: { count: 28, power: 1.2 },
  boomSmoke: { count: 22, power: 0.9 },
  laneDust: { count: 14, power: 1 },
  craterSmoke: { count: 18, power: 0.7 },
} as const;

/** Where it drops from: three of its zone's heights above it. */
const ABOVE: FxPoint = { x: 0.5, y: -3 };

export const bigBoom: Entrance = ({ D, tgt, intensity: i, view, landing, landMs }) => {
  const at = tid(tgt);
  const slam = landMs ?? frac(FX_SLAM_AT, D);
  const fuse = Math.min(slam, frac(ENTRANCE_FUSE_AT, D));
  const cues: FxCue[] = [];
  if (view !== undefined && landing !== undefined) {
    // The fuse burns along its row, lane by lane, from the first to its own, before the slam.
    const side = sideOf(view, landing.player);
    const lanes = LANES.filter((lane) => lane <= landing.lane);
    lanes.forEach((lane, k) => {
      cues.push(tunedBurst(i, "spark", tid(testid.zone(side, "units", lane), FOOT), "point", Math.round(fuse + ((slam - fuse) * k) / lanes.length), TUNING.fuseSpark));
    });
    // The blast throws dust down its lane to the other side.
    cues.push(tunedBurst(i, "dust", tid(testid.zone(otherSide(side), "units", landing.lane), FOOT), "area", slam, TUNING.laneDust));
  } else {
    cues.push(tunedBurst(i, "spark", at, "point", fuse, TUNING.fuseSpark));
  }
  cues.push(
    { kind: "shot", shot: { motion: "bolt", trail: "ember" }, colors: (paletteLook(PALETTE) ?? FLAVOR_LOOKS.Fire).colors, preset: "ember", from: tid(tgt, ABOVE), to: at, delayMs: fuse, flightMs: slam - fuse, density: i },
    raysCue(D, "legendary", at, 0),
    ringCue(D, "fire", at, slam),
    tunedBurst(i, "fire", at, "area", slam, TUNING.boomFire),
    tunedBurst(i, "ember", at, "ring", slam, TUNING.boomEmber),
    tunedBurst(i, "smoke", tid(tgt, FOOT), "area", slam, TUNING.boomSmoke),
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.bigBoom, slam),
  );
  return cues;
};

/** #80 BOOM! Big Max's Crater Locks its own zone as it lands: a smoking crater, clamped shut. */
export const craterLock: CardRecipe = (event, p, play) => {
  if (event.type !== "locked" || event.player !== play.player || event.row !== "units" || !ownWindow(p, play)) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const zone = testid.zone(sideOf(p.view, event.player), event.row, event.lane);
  const at = tid(zone);
  return {
    cues: [
      { kind: "crack", at, delayMs: t.G, durationMs: p.D - t.G + FX_CRACK_TAIL_MS },
      motifBurst(spec, tid(zone, FOOT), "area", t.G, TUNING.craterSmoke, "smoke"),
      motifRing(spec, p.D, at, t.L),
      ...shakeCues(p.env.intensity, SIGNATURE_TRAUMA.craterLock, t.G),
    ],
    row: "keep",
  };
};
