// R3395000: Stinkyscape #58 Beam v5 (`stinkyscape-058`, entrance "emitterBloom", bespoke "beamCharge", heat
// shimmer on its tile), a Legendary Unit's signature. Its emitter unfolds like a flower of white mirrors in
// its zone, a pilot light igniting at its core in sparks. At its Activate a long charge glows on it, then a
// violet-white beam spears the target, and its mirrors glow red as they cool; each further hit of the same
// Activate (Cleave's neighbours, Pierce's) flares where it lands. It keys on its hook and its own public id,
// never on a card's definition (R202). Moved: a Beam v5 equipped into another Unit activates as that Unit,
// whose definition keys nothing here; Cleave and Pierce flare in place of a split beam, inside the
// Legendary's ceiling; the heat shimmer on its tile is rising embers.

import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxCue, FxShotCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** The emitter's white-hot laser. */
const LASER = "Tech/laser";

/** Its mirrors' red as they cool. */
const COOLING = { rim: "#ef4444", core: "#fee2e2", glow: "#b91c1c" } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  bloom: { count: 22, power: 1 },
  pilot: { count: 10, power: 0.6 },
} as const;

/** As shares of the entry: the charge glows until `fire`, the beam flies for `flight`, the mirrors cool from `cool`. */
const TIMING = { fire: 0.4, flight: 0.2, cool: 0.6 } as const;

export const emitterBloom: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("mirrorBloom", LASER, plan),
    { ...tunedBurst(plan.intensity, "sparkle", at, "area", slam, TUNING.bloom), colors: lookOf(LASER).colors },
    tunedBurst(plan.intensity, "ember", tid(plan.tgt, FOOT), "point", 0, TUNING.pilot),
    ringCue(plan.D, "holy", at, slam),
    ...slamShake(plan, "emitterBloom"),
  ];
};

export const beamCharge: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate" || event.type !== "damage") return null;
  const target = placeOf(p.view, event.targetId);
  if (target === null) return null;
  const at = tid(target);
  if ((play.seen.damage ?? 0) > 1) return replace([ringCue(p.D, "spark", at, 0)]);
  const emitter = cardAt(p.view, play);
  const fire = Math.round(TIMING.fire * p.D);
  const flight = Math.round(TIMING.flight * p.D);
  const cool = Math.round(TIMING.cool * p.D);
  const beam: FxShotCue = { kind: "shot", shot: { motion: "beam" }, colors: lookOf(LASER).colors, preset: "spark", from: emitter, to: at, delayMs: fire, flightMs: flight, density: p.env.intensity };
  const cues: FxCue[] = [{ kind: "sheen", at: emitter, delayMs: 0, durationMs: fire }, beam, { kind: "brand", at: emitter, tint: COOLING, delayMs: cool, durationMs: p.D - cool }];
  return replace(cues);
};
