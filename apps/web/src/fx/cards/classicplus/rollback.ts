// R2795000: Classic+ #35 Rollback (`classicplus-035`, bespoke "rollbackSweep") rewinds the board lane by
// lane: at its `rolledBack` an arcane ring runs back from lane 5 to lane 1 through the unit zones of
// every side it rolled back, in the card's sepia, and each of those heroes glints as the board lands
// where it was. Its row (`rewind`, the board's sheen and shake) still plays.
// R3399000: its signature (#3399) adds the clock. Cast (its own `cardPlayed`): a stone clock face frames the
// screen's edge (twelve stone ticks around it and a ring at twelve), its hand stutters at twelve (a violet
// tick, one forward, back again) and a violet tint washes the board (a violet haze over every unit zone,
// lane 5 back to lane 1). At the `rolledBack` the hand swings back one, two or three ticks, as many as the
// turns it rolled back (a violet tick and stone dust on each), beside the sweep.
// Moved: the hand's swing back plays at the `rolledBack`, since the turns are declared with the play (R81)
// and only the `rolledBack` names them; and the sweep runs from the entry's start (not Travel), its heroes
// glinting a step after lane 1 (not at Land), so every burst starts inside a Legendary Spell's first 300 ms
// at the row's 600 ms (#3399's per-beat ceiling). R202: the clock and the tint are the screen's and the
// zones', and no cue names a card.

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst, type BurstTuning } from "../../build.ts";
import { FX_ROLLBACK_STEP } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { motifBurst, motifScale, otherSide, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPreset, FxSpread } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the rewind throws at scale 1 (rule 9). */
const TUNING = {
  laneArcane: { count: 8, power: 0.7 },
  heroSparkle: { count: 14, power: 0.9 },
  // R3399000: the clock's, at intensity "normal".
  tick: { count: 2, power: 0.3 },
  stutter: { count: 6, power: 0.6 },
  tint: { count: 3, power: 0.3 },
  swing: { count: 6, power: 0.7 },
} as const;

/** R3399000: the clock's stone and the tint's violet, hot first. */
const STONE = ["#f5f5f4", "#a8a29e", "#57534e"] as const;
const VIOLET = ["#f5f3ff", "#a78bfa", "#6d28d9"] as const;

/** R3399000: the clock face framing the screen: its centre, its radii as shares of the screen, its hours. */
const CLOCK_CENTER = { x: 0.5, y: 0.5 } as const;
const CLOCK_RADIUS = { x: 0.47, y: 0.45 } as const;
const CLOCK_HOURS = 12;

/**
 * R3399000: the clock's beats as shares of D, every burst inside a Legendary Spell's first 300 ms at the
 * cast's 400 ms: the ticks light FACE_STEP apart from the start, the hand stutters at STUTTER_AT (twelve,
 * one, twelve), the tint washes from TINT_AT a lane every TINT_STEP; at the `rolledBack` the hand swings
 * back from SWING_AT a tick every SWING_STEP.
 */
const FACE_STEP = 0.015;
const STUTTER_AT = [0.2, 0.3, 0.4] as const;
const STUTTER_HOURS = [0, 1, 0] as const;
const TINT_AT = 0.3;
const TINT_STEP = 0.06;
const SWING_AT = 0;
const SWING_STEP = 0.08;

/** R3399000: where the clock's `hour` (0 is twelve, counting clockwise, any integer) stands on the screen's edge. */
function tickAt(hour: number): FxAnchor {
  const angle = (2 * Math.PI * hour) / CLOCK_HOURS;
  return { kind: "viewport", at: { x: CLOCK_CENTER.x + CLOCK_RADIUS.x * Math.sin(angle), y: CLOCK_CENTER.y - CLOCK_RADIUS.y * Math.cos(angle) } };
}

/** Classic+ #35 Rollback: the clock on its cast; at its play's first `rolledBack` the hand's swing and the sweep, on the sides that event names. */
export const rollbackSweep: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const t = timesOf(p.D);
  const i = p.env.intensity;
  const burst = (preset: FxPreset, at: FxAnchor, spread: FxSpread, delay: number, tuning: BurstTuning, colors: readonly string[]): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors,
  });
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const mine = sideOf(p.view, play.player);
    const cues: FxCue[] = [ringCue(p.D, "dust", tickAt(0), t.G)];
    for (let hour = 0; hour < CLOCK_HOURS; hour += 1) cues.push(burst("dust", tickAt(hour), "point", frac(hour * FACE_STEP, p.D), TUNING.tick, STONE));
    STUTTER_AT.forEach((x, k) => cues.push(burst("arcane", tickAt(STUTTER_HOURS[k] ?? 0), "point", frac(x, p.D), TUNING.stutter, VIOLET)));
    [...LANES].reverse().forEach((lane, k) => {
      for (const side of [mine, otherSide(mine)]) cues.push(burst("smoke", tid(testid.zone(side, "units", lane)), "area", frac(TINT_AT + k * TINT_STEP, p.D), TUNING.tint, VIOLET));
    });
    return { cues, row: "keep" };
  }
  if (event.type !== "rolledBack" || play.seen.rolledBack !== 1) return null;
  const look = flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const sides = [...new Set(event.sides.map((player) => sideOf(p.view, player)))];
  const cues: FxCue[] = [];
  // Lane 5 first, back to lane 1, FX_ROLLBACK_STEP of D apart from the entry's start, every ring inside
  // Gather to Land (R200); R3399000: from Gather, not Travel, and the heroes glint a step after lane 1, so
  // every burst lands inside a Legendary Spell's first 300 ms at the row's 600 ms.
  [...LANES].reverse().forEach((lane, k) => {
    const at = Math.min(t.L, t.G + frac(k * FX_ROLLBACK_STEP, p.D));
    for (const side of sides) {
      const zone = tid(testid.zone(side, "units", lane));
      cues.push(ringCue(p.D, "arcane", zone, at), motifBurst(spec, zone, "point", at, TUNING.laneArcane, "arcane"));
    }
  });
  const glint = Math.min(t.L, t.G + frac(LANES.length * FX_ROLLBACK_STEP, p.D));
  for (const side of sides) cues.push(motifBurst(spec, tid(testid.hero(side)), "area", glint, TUNING.heroSparkle, "sparkle"));
  // R3399000: the hand swings back a tick for each turn rolled back, from twelve.
  const turns = Math.min(Math.max(1, event.turnsAgo), CLOCK_HOURS);
  for (let n = 1; n <= turns; n += 1) {
    const at = frac(SWING_AT + (n - 1) * SWING_STEP, p.D);
    cues.push(burst("arcane", tickAt(-n), "point", at, TUNING.swing, VIOLET), burst("dust", tickAt(-n), "point", at, TUNING.tick, STONE));
  }
  return { cues, row: "keep" };
};
