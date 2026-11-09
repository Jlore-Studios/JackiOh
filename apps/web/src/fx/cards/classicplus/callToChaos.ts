// R3399000: Classic+ Call to Chaos (Classic+ Edition) (`classicplus-073`, bespoke "chaosLever"), its signature (#3399):
// Cast: a giant stone lever slams down at the board's right edge (dust, a ring and a shake there) and a wheel
// of ten carved faces spins up behind the board, rattling (a ring and chaos sparks at its centre).
// Text: it lands on its faces (`chaosRolled`, up to three effects in the order they resolve), each flaring in a
// colour by its label: card backs raining to the hand (a ghost and gold sparks), a golem dropping in (dust at
// the middle of the caster's units), a wipe (blood across the foe's units), a fusion glow (prismatic at the
// hero), anything else a chaos flare at the wheel. Only that effect's own events follow, under their rows.
// The shared chaos wheel cue stays (`keep`). R202: the roll is public; no cue names a card.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { chaosRollOf } from "../../chaos.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPreset } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the lever and the faces throw, at intensity "normal" (rule 9). */
const TUNING = {
  slam: { count: 12, power: 1 },
  wheel: { count: 12, power: 0.8 },
  rain: { count: 8, power: 0.7 },
  golem: { count: 14, power: 1 },
  wipe: { count: 6, power: 0.9 },
  fusion: { count: 16, power: 0.9 },
  flare: { count: 10, power: 0.8 },
} as const;

const LEVER: FxAnchor = { kind: "viewport", at: { x: 0.92, y: 0.5 } };
const WHEEL: FxAnchor = { kind: "viewport", at: { x: 0.5, y: 0.4 } };
const SLAM_AT = 0;
const SLAM_SHAKE = 0.6;
const WHEEL_AT = 0.1;
const FLARE_AT = 0.05;
const FLARE_STEP = 0.06;
const WIPE_STEP = 0.02;
const RAIN_LAND_AT = 0.2;
const GOLEM_LANE = 3;

/** What an outcome's label draws, by the words its clause uses (the engine's labels, `chaosNames`'s keys). */
type Outcome = "cards" | "golem" | "wipe" | "fusion" | "flare";
const OUTCOMES: readonly (readonly [RegExp, Outcome])[] = [
  [/^(Add|Replace)\b/, "cards"],
  [/Golem/, "golem"],
  [/^(Destroy|Nerf)\b/, "wipe"],
  [/^Fuse\b/, "fusion"],
];

function outcomeOf(label: string): Outcome {
  return OUTCOMES.find(([pattern]) => pattern.test(label))?.[1] ?? "flare";
}

/** Classic+ #73 Call to Chaos: the lever and wheel on its cast, each rolled face's flare on its roll. */
export const chaosLever: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const look = FLAVOR_LOOKS.Chaos;
  const burst = (preset: FxPreset, at: FxAnchor, spread: "point" | "area" | "ring", delay: number, tuning: { count: number; power: number }): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors: look.colors,
  });
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const cues: FxCue[] = [
      ringCue(p.D, "dust", LEVER, frac(SLAM_AT, p.D)),
      burst("dust", LEVER, "area", frac(SLAM_AT, p.D), TUNING.slam),
      ringCue(p.D, look.particle, WHEEL, frac(WHEEL_AT, p.D)),
      burst(look.particle, WHEEL, "ring", frac(WHEEL_AT, p.D), TUNING.wheel),
      ...shakeCues(i, SLAM_SHAKE, frac(SLAM_AT, p.D)),
    ];
    return { cues, row: "keep" };
  }
  const roll = chaosRollOf(event);
  if (roll === null) return null;
  const mine = sideOf(p.view, roll.player);
  const foe = otherSide(mine);
  const hand = tid(animTestid.hand(mine));
  const cues: FxCue[] = [];
  roll.effects.forEach((label, k) => {
    const at = frac(FLARE_AT + k * FLARE_STEP, p.D);
    switch (outcomeOf(label)) {
      case "cards":
        cues.push({ kind: "ghost", from: WHEEL, to: hand, delayMs: at, durationMs: frac(RAIN_LAND_AT - FLARE_AT, p.D) }, burst("gold", hand, "area", frac(RAIN_LAND_AT + k * FLARE_STEP, p.D), TUNING.rain));
        break;
      case "golem":
        cues.push(burst("dust", tid(testid.zone(mine, "units", GOLEM_LANE)), "area", at, TUNING.golem));
        break;
      case "wipe":
        LANES.forEach((lane, n) => cues.push(burst("blood", tid(testid.zone(foe, "units", lane)), "area", at + frac(n * WIPE_STEP, p.D), TUNING.wipe)));
        break;
      case "fusion":
        cues.push(burst("prismatic", tid(testid.hero(mine)), "area", at, TUNING.fusion));
        break;
      case "flare":
        cues.push(burst(look.particle, WHEEL, "area", at, TUNING.flare));
        break;
    }
  });
  return { cues, row: "keep" };
};

/** The gallery's Call: it rolls a card-adding effect, a golem and a wipe. */
export function chaosLeverFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [{ type: "chaosRolled", player, instanceId, defId: "classicplus-073", effects: ["Add 3 random Books to your hand, which cost (0)", "Summon a Classic Golem", "Destroy all enemy permanents"] }];
}
