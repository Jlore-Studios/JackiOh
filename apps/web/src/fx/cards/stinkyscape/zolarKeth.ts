// R3395000: Stinkyscape #26 ZolarKeth (`stinkyscape-026`, entrance "lecternRise", bespoke "campusLaunch",
// starlight drifting on his tile), a Legendary Unit's signature. A lectern of light rises in his zone and
// he steps up to it, a mortarboard spinning down, stars bursting at his feet. His Cry plays the mode it
// chose: the University's domed campus rises beside him in chalk-blue lines as it is summoned, in a frost
// ring and arcane sparks; or, from then on, each End of turn a Starship Piece drops in on a rocket trail
// from above its zone and lands in a ring. It keys on his play or his hook, never on a card's definition
// (R202). Moved: the launch gantry locking onto his tile is the rocket trail itself, and the satellite and
// medals on his tile are a slow drift of starlight (an aura draws motes and an edge).

import { ringCue, tid, tunedBurst } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifShot } from "../../motifs/build.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { entranceFigure, lookOf, replace, slamAt, slamShake, summonZone } from "./signatureKit.ts";

/** His lecture's starlight, his campus's chalk and his rockets. */
const STARLIGHT = "Tech/starfield";
const CHALK = "Nerd/chalk";
const ROCKET = "Tech/rocket";

/** Where a Starship Piece drops from: high above its zone. */
const DROP_FROM = { x: 0.5, y: -2 } as const;

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  stars: { count: 22, power: 1 },
  campus: { count: 16, power: 0.9 },
} as const;

/** As a share of the entry, how long a Starship Piece falls. */
const TIMING = { dropFlight: 0.6 } as const;

export const lecternRise: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("lectern", STARLIGHT, plan),
    { ...tunedBurst(plan.intensity, "star", tid(plan.tgt, FOOT), "area", slam, TUNING.stars), colors: lookOf(STARLIGHT).colors },
    ringCue(plan.D, "sparkle", at, slam),
    ...slamShake(plan, "lecternRise"),
  ];
};

export const campusLaunch: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned" || event.player !== play.player || event.instanceId === play.instanceId) return null;
  const zone = summonZone(p.view, event);
  const at = tid(zone);
  if (play.hook === undefined) {
    // The Cry's University: its campus rises in chalk lines.
    return replace([
      figureCue("campus", at, figureTint(lookOf(CHALK)), 0, p.D),
      ringCue(p.D, "frost", at, 0),
      { ...tunedBurst(p.env.intensity, "arcane", at, "area", 0, TUNING.campus), colors: lookOf(CHALK).colors },
    ]);
  }
  // His End of turn's Starship Piece, dropped on a rocket trail.
  const flight = Math.round(TIMING.dropFlight * p.D);
  return replace([motifShot({ look: lookOf(ROCKET), scale: p.env.intensity, row: "replace" }, "bolt", tid(zone, DROP_FROM), at, 0, flight), ringCue(p.D, "exhaust", at, flight)]);
};
