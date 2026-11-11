// R3389001: Classic #90 In Too Deep (`classic-090`), a Mythic Field Spell's signature: entrance
// "abyssSurface", bespoke "shrineDescent", its aura "abyssWhirl" and a takeover of its own, "abyss"
// (R3389000, takeover.ts: the board tips into a spiral shaft plunging into violet dark, black-blue water
// rushing down it). As the takeover lands, the water bursts over its zone and the card surfaces in a
// ring. Each step a quest of its makes (its own `questProgressed`) lights a shrine one step lower down a
// spiral on the card; a quest it completes (`questCompleted`) bursts its shrine open in light. The
// rewards it offers are its owner's prompt, never drawn (R202). Standing, a slow whirlpool of black-blue
// water turns on its zone, a faint light at its bottom.

import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxPoint } from "../../types.ts";
import { cardAnchor, ENTRANCE_TRAUMA, FOOT, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its water and its shrines glow in. */
const PALETTE: FxPalette = "Dark/shadow";

/** Every particle count it throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  water: { count: 56, power: 1.4 },
  spray: { count: 40, power: 1.1 },
  depths: { count: 30, power: 0.8 },
  shrine: { count: 10, power: 0.6 },
  burst: { count: 28, power: 1.2 },
} as const;

/** The spiral its shrines descend: the first light's place on the card, and how far down it runs. */
const SPIRAL_TOP = 0.2;
const SPIRAL_DEPTH = 0.65;
/** How far across the card the spiral swings, either side of its middle. */
const SPIRAL_SWING = 0.25;

/** R3389001: where a quest's `progress` of `goal` lights its shrine: a step down the spiral, swinging side to side. */
export function shrinePoint(progress: number, goal: number): FxPoint {
  const share = goal > 0 ? Math.min(1, Math.max(0, progress / goal)) : 1;
  return { x: 0.5 + (progress % 2 === 0 ? SPIRAL_SWING : -SPIRAL_SWING) * (1 - share), y: SPIRAL_TOP + SPIRAL_DEPTH * share };
}

export const abyssSurface: Entrance = ({ D, tgt, intensity: i, landMs }) => {
  const at = tid(tgt);
  const land = landMs ?? frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Dark;
  return [
    { ...tunedBurst(i, "frost", at, "area", land, TUNING.water), colors: look.colors },
    { ...tunedBurst(i, "frost", at, "ring", land, TUNING.spray), colors: look.colors },
    tunedBurst(i, "void", tid(tgt, FOOT), "area", land, TUNING.depths),
    ringCue(D, "void", at, land),
    ringCue(D, "frost", at, land),
    ...shakeCues(i, ENTRANCE_TRAUMA.abyssSurface, land),
  ];
};

/** #90 In Too Deep lights a shrine lower down its spiral for each quest step, and bursts it open as a quest completes. */
export const shrineDescent: CardRecipe = (event, p, play) => {
  if ((event.type !== "questProgressed" && event.type !== "questCompleted") || event.instanceId !== play.instanceId) return null;
  if (event.type === "questProgressed" && !(event.progress > 0)) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const zone = cardAnchor(p, play.instanceId);
  const card = zone ?? heroOf(p.view, play);
  if (event.type === "questProgressed") {
    const light = card.kind === "testid" ? tid(card.testid, shrinePoint(event.progress, event.goal)) : card;
    return { cues: [motifBurst(spec, light, "point", t.G, TUNING.shrine, "sparkle"), motifRing(spec, p.D, light, t.T)], row: "keep" };
  }
  return { cues: [motifBurst(spec, card, "area", t.G, TUNING.burst, "prismatic"), motifRing(spec, p.D, card, t.G), motifRing(spec, p.D, card, t.L)], row: "keep" };
};

/** The gallery's In Too Deep's first quest: one step of its three draws (§8.6 row 90, R3042000). */
const FIXTURE_QUEST = "q1";
const FIXTURE_PROGRESS = 1;
const FIXTURE_GOAL = 3;

/** The gallery's In Too Deep: its first quest takes a step. */
export const shrineDescentFixture: FlourishFixture = (_view, instanceId, player) => [
  { type: "questProgressed", player, instanceId, quest: FIXTURE_QUEST, progress: FIXTURE_PROGRESS, goal: FIXTURE_GOAL },
];
