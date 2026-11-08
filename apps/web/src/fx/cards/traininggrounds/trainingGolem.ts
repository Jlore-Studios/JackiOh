// R3400001: Training Grounds #39.1 Training-Grounds Golem (`traininggrounds-039-1`, a token printed
// Legendary, entrance "golemLashing", timber dust on its tile). Training posts, dummies and shields fly in
// from the yard's four edges, trailing dust, and lash themselves together in its zone at the slam, where
// it stands up in a ring of dust; then its five keywords lock on one by one: plank armor bolts over its
// chest (Armor), a shield bubble closes (Divine Shield), a wooden palisade rises (Taunt), its Rush flares
// and a splinter drives through (Pierce, the hit it pierces with shown as it lands, since no later hit
// keys a card's frame). A token's entrance keys on its `summoned`'s own readable `defId`, and no cue names
// a card (R202).

import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** The look its timber wears: Physical's steel sub-look. */
const GOLEM_PALETTE = "Physical/steel";

/** The yard's four edges the parts fly in from, as viewport fractions: left, right, top, bottom. */
const YARD_EDGES: readonly FxPoint[] = [
  { x: 0, y: 0.5 },
  { x: 1, y: 0.5 },
  { x: 0.5, y: 0 },
  { x: 0.5, y: 1 },
];

/** Every particle count the lashing throws, at intensity "normal" (rule 9). */
const TUNING = { dust: { count: 26, power: 1.1 } } as const;

/**
 * As fractions of the entry: the first part leaves at `leaveFirst`, the last at `leaveLast`, all landing
 * at the slam; each keyword locks on at its `locks` time and holds until `lockTailMs` past the entry.
 */
const TIMING = { leaveFirst: 0.1, leaveLast: 0.25, locks: [0.62, 0.7, 0.78, 0.86, 0.94], lockTailMs: 300 } as const;

export const golemLashing: Entrance = ({ D, tgt, intensity: i }) => {
  const look = paletteLook(GOLEM_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const tint = figureTint(look);
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const parts = YARD_EDGES.map((edge, n): FxShotCue => {
    const delayMs = frac(TIMING.leaveFirst + ((TIMING.leaveLast - TIMING.leaveFirst) * n) / (YARD_EDGES.length - 1), D);
    return {
      kind: "shot",
      shot: { motion: "bolt", trail: "dust" },
      colors: look.colors,
      preset: "dust",
      from: { kind: "viewport", at: edge },
      to: at,
      delayMs,
      flightMs: Math.max(0, slam - delayMs),
      density: i,
    };
  });
  const [plank = 0, shield = 0, palisade = 0, rush = 0, splinter = 0] = TIMING.locks.map((lock) => frac(lock, D));
  const until = (from: number): number => D - from + TIMING.lockTailMs;
  const cues: FxCue[] = [
    ...parts,
    ringCue(D, "dust", at, slam),
    tunedBurst(i, "dust", tid(tgt, FOOT), "area", slam, TUNING.dust),
    ...shakeCues(i, ENTRANCE_TRAUMA.golemLashing, slam),
    figureCue("plank", at, tint, plank, until(plank)),
    { kind: "shield", size: "full", at, delayMs: shield, durationMs: until(shield) },
    figureCue("palisade", at, tint, palisade, until(palisade)),
    { kind: "callout", keyword: "Rush", at, delayMs: rush, durationMs: until(rush) },
    figureCue("splinter", at, tint, splinter, until(splinter)),
  ];
  return cues;
};
