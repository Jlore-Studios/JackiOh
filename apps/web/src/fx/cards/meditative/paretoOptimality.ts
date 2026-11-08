// R3405001: Meditative #71 Pareto Optimality (`meditative-071`, entrance "studyCurve", bespoke "pareto",
// takeover "studyHall", a holy glow down its lane), a Mythic Field Spell's signature in Nerd's chalk. After
// its takeover's hold one sweeping white curve is brushed across its backrow zone, chalk dust lifting off
// its foot, and it settles at the slam in a white ring that runs on into the unit zone in front of it. The
// Cane its play summons drops in with The Cane's own entrance (theCane.ts), its `summoned` never this
// recipe's, so the cane lifting is The Cane's. The judgement stays hidden (§10.8): the only tell is its
// trigger, where the Cane crackles white in its backrow zone as it animates (`animated`) and whips down its
// lane in a white streak toward the enemy's side. The strike's `damage` comes after the forced attack's
// `attackDeclared`, which ends the trigger's entry (memory.ts), so the hit's own rows draw it. It keys on
// its play or its trigger and the event's public zones, never on a card's definition, and names no card (R202).

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its curve and streak wear: Nerd's chalk, white on the study hall's board. */
const PARETO_PALETTE = "Nerd/chalk";

/** Every particle count it throws, at intensity "normal" (rule 9): the curve's dust, its settling, the lane's end, the Cane's crackle. */
const TUNING = {
  dust: { count: 14, power: 0.7 },
  settle: { count: 18, power: 1 },
  lane: { count: 10, power: 0.8 },
  crackle: { count: 16, power: 1.2 },
} as const;

/**
 * The curve runs from the lead to `curveTailMs` past its entry; the streak lasts its entry and `streakTailMs`
 * more, its white bolt leaving at `boltAt` of the entry and landing by `boltEnd` of it.
 */
const TIMING = { curveTailMs: 300, streakTailMs: 300, boltAt: 0.2, boltEnd: 0.7 } as const;

/** The gallery's Cane, animating out of the backrow zone beside Pareto's into the unit zone in front of it. */
const FIXTURE_CANE = { instanceId: "fx-cane", defId: "meditative-071-1", lane: 2 } as const;

export const studyCurve: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(PARETO_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landing === undefined ? undefined : sideOf(view, landing.player);
  const cues: FxCue[] = [
    figureCue("paretoCurve", at, figureTint(look), lead, D - lead + TIMING.curveTailMs, side === undefined ? {} : { side }),
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "area", lead, TUNING.dust), colors: look.colors },
    ringCue(D, "sparkle", at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.settle), colors: look.colors },
  ];
  // The curve's far end, down its lane: the unit zone in front of it.
  if (side !== undefined && landing !== undefined) {
    cues.push({ ...tunedBurst(i, "sparkle", tid(testid.zone(side, "units", landing.lane)), "point", slam, TUNING.lane), colors: look.colors });
  }
  cues.push(...shakeCues(i, ENTRANCE_TRAUMA.studyCurve, slam));
  return cues;
};

export const pareto: CardRecipe = (event, p, play) => {
  // Its trigger animates its own Cane alone (cane_strike), out of its backrow zone into a unit zone.
  if (event.type !== "animated" || event.player !== play.player) return null;
  const look = paletteLook(PARETO_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const side = sideOf(p.view, event.player);
  const stand = tid(testid.zone(side, "units", event.unitLane));
  const leave = frac(TIMING.boltAt, p.D);
  return {
    cues: [
      { ...tunedBurst(p.env.intensity, "spark", tid(testid.zone(side, "backrow", event.backrowLane)), "area", 0, TUNING.crackle), colors: look.colors },
      figureCue("caneStreak", stand, figureTint(look), 0, p.D + TIMING.streakTailMs, { side }),
      {
        kind: "shot",
        shot: { motion: "bolt", trail: "spark" },
        colors: look.colors,
        preset: "spark",
        from: stand,
        to: tid(testid.zone(otherSide(side), "units", event.unitLane)),
        delayMs: leave,
        flightMs: frac(TIMING.boltEnd, p.D) - leave,
        density: p.env.intensity,
      },
    ],
    row: "replace",
  };
};

/** The gallery's Pareto Optimality: its trigger's one public beat, its Cane animating to strike. */
export const paretoFixture: RecipeFixture = (_view, _instanceId, player) => [
  { type: "animated", player, instanceId: FIXTURE_CANE.instanceId, defId: FIXTURE_CANE.defId, backrowLane: FIXTURE_CANE.lane, unitLane: FIXTURE_CANE.lane },
];
