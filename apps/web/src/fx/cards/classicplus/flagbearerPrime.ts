// R3399000: Classic+ Felinor Flagbearer Prime (`classicplus-046-1`, entrance "greatBanner"), its signature
// (#3399): his entrance and, in it, his Cry's copies.
// Entrance: he lands and drives a towering banner into the ground (dust, ring, shake); it unfurls so wide that its
// shadow falls across the caster's whole side, a smoke burst and a sheen in each of the side's unit zones, lane by lane.
// A copy (a `summoned` with a `sourceId`, his Cry's) draws a smaller entrance instead: it leaps out of the banner's
// shadow, a bolt in from the shadow's edge, and plants a smaller banner (one burst) with a ring. cardFx.ts plans an
// entrance on every `summoned` that lands in a unit zone, before any recipe, so his Cry's beat is drawn here (moved:
// from a recipe of its own, which no copy's landing would reach).
// Moved: the `buffed` glow of the other Felinors is not drawn: his aura buffs on read and the engine emits no event
// for it; each copy's landing perks its own zone instead.
// R202: nothing here names a card.

import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifRing, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** Every particle count the banner throws, at intensity "normal" (rule 9). */
const TUNING = {
  plant: { count: 24, power: 1.2 },
  shade: { count: 7, power: 0.6 },
  copy: { count: 10, power: 0.9 },
  glow: { count: 6, power: 0.7 },
} as const;

/** The shadow spreads from the banner's lane, lane by lane, starting here and this far apart (fractions of D). */
const SHADE_AT = 0.66;
const SHADE_STEP = 0.06;
const SHADE_MS_SHARE = 0.3;
/** The copy's leap: from the shadow's edge, a bolt landing at this fraction of the entry. */
const LEAP_AT = 0.1;
const LAND_AT = 0.6;
/** Where a copy leaps from: the board's edge above its zone. */
const SHADOW_EDGE = { x: 0.5, y: 0 } as const;

const look = paletteLook("Beast/felinor") ?? FLAVOR_LOOKS.Beast;

function copyLanding(spec: CueSpec, D: number, at: FxAnchor): FxCue[] {
  const land = frac(LAND_AT, D);
  const leap = frac(LEAP_AT, D);
  return [
    motifShot(spec, "bolt", at.kind === "testid" ? tid(at.testid, SHADOW_EDGE) : at, at, leap, land - leap),
    motifRing(spec, D, at, land),
    motifBurst(spec, at, "area", land, TUNING.copy),
    motifBurst(spec, at, "ring", land, TUNING.glow, "sparkle"),
  ];
}

export const greatBanner: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const spec: CueSpec = { look, scale: i, row: "keep" };
  const at = tid(tgt);
  if (landing?.sourceId !== undefined) return copyLanding(spec, D, at);
  const slam = frac(FX_SLAM_AT, D);
  const cues: FxCue[] = [
    motifRing(spec, D, at, slam),
    motifBurst(spec, tid(tgt, FOOT), "area", slam, TUNING.plant, "dust"),
    ...shakeCues(i, ENTRANCE_TRAUMA.greatBanner, slam),
  ];
  if (view === undefined || landing === undefined) return cues;
  const side = sideOf(view, landing.player);
  LANES.forEach((lane, k) => {
    const zone = tid(testid.zone(side, "units", lane));
    const delay = frac(SHADE_AT + k * SHADE_STEP, D);
    cues.push(motifBurst(spec, zone, "area", delay, TUNING.shade, "smoke"), { kind: "sheen", at: zone, delayMs: delay, durationMs: frac(SHADE_MS_SHARE, D) });
  });
  return cues;
};
