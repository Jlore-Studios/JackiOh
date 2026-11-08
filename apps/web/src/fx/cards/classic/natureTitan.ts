// R670: Nature Titan (`classic-045`, entrance "titanBloom"): a column of light and a bloom where it lands.
// R3389001: grown into its signature (R3389000): entrance "titanBloom", bespoke "titanLeaf" and its aura
// "mossCreep" on its own zone. Its Tribute sinks into the soil, roots crack through its zone, a moss
// boulder heaves up into the Titan and wildflowers burst open in a ring round its foot. Its Cry draws a
// card on a leaf, a card back from the deck pile to its hand (the face is its owner's alone, R202), and a
// seed of green light flies to its hero, blooming there as it heals. Standing, moss creeps over its frame.
// Moved: the same beats when it attacks, which open no hook of its card (R2797000).

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { cardAnchor, ENTRANCE_TRAUMA, FOOT, ownWindow, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its moss, leaves and seed grow in. */
const PALETTE: FxPalette = "Nature";

/** Every particle count the bloom throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  bloomSparkle: { count: 26, power: 1.1 },
  bloomGold: { count: 20, power: 1.2 },
  bloomDust: { count: 20, power: 1 },
  roots: { count: 10, power: 0.6 },
  flowers: { count: 18, power: 0.9 },
  leaf: { count: 8, power: 0.6 },
} as const;

/** Where the moss boulder heaves up from: below its zone. */
const BELOW: FxPoint = { x: 0.5, y: 1.6 };

export const titanBloom: Entrance = ({ D, tgt, intensity: i, landMs }) => {
  const at = tid(tgt);
  const slam = landMs ?? frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Nature;
  return [
    raysCue(D, "holy", at, 0),
    // Roots crack through the zone as the Tribute sinks.
    { kind: "crack", at, delayMs: 0, durationMs: D + FX_CRACK_TAIL_MS },
    tunedBurst(i, "leaf", tid(tgt, FOOT), "point", 0, TUNING.roots),
    // The moss boulder heaves up into the Titan.
    { kind: "shot", shot: { motion: "arc", sprites: ["boulder"], turn: "spin", count: 1, trail: "leaf" }, colors: look.colors, preset: "dust", from: tid(tgt, BELOW), to: at, delayMs: 0, flightMs: slam, density: i },
    ringCue(D, "dust", at, slam),
    tunedBurst(i, "dust", tid(tgt, FOOT), "ring", slam, TUNING.bloomDust),
    tunedBurst(i, "sparkle", at, "area", slam, TUNING.bloomSparkle),
    tunedBurst(i, "gold", at, "ring", slam, TUNING.bloomGold),
    // Wildflowers burst open in a ring round its foot.
    { ...tunedBurst(i, "confetti", tid(tgt, FOOT), "ring", slam, TUNING.flowers), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.titanBloom, slam),
  ];
};

/** #45 Nature Titan's Cry draws on a leaf and heals its hero with a seed of light. */
export const titanLeaf: CardRecipe = (event, p, play) => {
  if (!ownWindow(p, play)) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  if (event.type === "drawn" && event.player === play.player) {
    const titan = cardAnchor(p, play.instanceId) ?? tid(testid.hero(side));
    const hero = tid(testid.hero(side));
    const cues: FxCue[] = [
      { kind: "ghost", from: tid(animTestid.library(side)), to: tid(animTestid.hand(side)), delayMs: t.G, durationMs: t.L - t.G },
      motifBurst(spec, tid(animTestid.library(side)), "point", t.G, TUNING.leaf, "leaf"),
    ];
    // The seed of light leaves the Titan for its hero as the card is drawn, to bloom as the heal lands.
    if (titan.kind !== "testid" || hero.kind !== "testid" || titan.testid !== hero.testid) {
      cues.push({ kind: "projectile", preset: "leaf", from: titan, to: hero, delayMs: t.T, flightMs: t.L - t.T, density: spec.scale });
    }
    return { cues, row: "keep" };
  }
  if (event.type === "healed" && event.targetId === `hero-${play.player}`) {
    const hero = tid(testid.hero(side));
    return { cues: [motifRing(spec, p.D, hero, t.G), motifRing(spec, p.D, hero, t.T)], row: "keep" };
  }
  return null;
};

/** The heal the gallery's Nature Titan's Cry gives its hero: its base face's (§8.6 row 45). */
const FIXTURE_HEAL = 3;

/** The gallery's Nature Titan: its Cry draws a card and heals its hero. */
export const titanLeafFixture: FlourishFixture = (_view, instanceId, player) => [
  { type: "drawn", player, instanceId: `${instanceId}-leaf`, defId: "core-055" },
  { type: "healed", targetId: `hero-${player}`, amount: FIXTURE_HEAL },
];
