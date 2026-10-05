// R670: the bespoke entrances of a handful of marquee Legendary and Mythic Units (#258).
//
// Every Legendary or Mythic Unit gets its rarity's entrance (cues.ts `summon`: rays, a gold or
// prismatic burst and extra trauma). A card listed in `CARD_FX` (cardFx.ts) under one of the keys
// here gets its own instead: its recipe claims the `summoned` that puts it into a unit zone and
// replaces the row's summon recipe. It keys off that event's own `defId`, which a unit zone's summon
// always shows both seats and a hidden summon never has, so nothing the viewer may not read keys one
// (R202), and it decorates the entry the runner already timed, so it paces nothing (R200, R201): a
// delay lands inside the entry, and whatever trails after it is gone within FX_MAX_TAIL_MS.
//
//   #100 Ceaseless Void, "voidCollapse": the board darkens into it, a void ring pulls in from the
//     first frame, the void bursts out at the slam with a crack under it and the heaviest shake.
//   #80 BOOM! Big Max, "bigBoom": a fuse of sparks, then the slam goes off: fire, embers, smoke, a
//     crack and a big shake.
//   #45 Nature Titan, "titanBloom": holy rays, a ring of dust, and leaves of sparkle and gold that
//     bloom out of the slam.
//   #61 Plague Bringer Goliath, "plagueStomp": a poison ring and a poison cloud with smoke at the
//     foot of the slam.
//   #56 Spell Tyrant, "tyrantSigil": an arcane ring from the first frame, sparkles round it, and an
//     arcane burst with mythic rays at the slam.

import type { GameEvent } from "@jackioh/shared";

import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { FX_CRACK_TAIL_MS, FX_LEGENDARY_TRAUMA, FX_SLAM_AT } from "./constants.ts";
import type { FxCue, FxPoint } from "./types.ts";

/** Every particle count the entrances throw, at intensity "normal" (rule 9). */
const ENTRANCE_TUNING = {
  voidPull: { count: 36, power: 0.8 },
  voidBurst: { count: 56, power: 1.5 },
  voidShard: { count: 18, power: 1.2 },
  fuseSpark: { count: 16, power: 0.7 },
  boomFire: { count: 60, power: 1.6 },
  boomEmber: { count: 30, power: 1.2 },
  boomSmoke: { count: 24, power: 0.9 },
  bloomSparkle: { count: 34, power: 1.1 },
  bloomGold: { count: 30, power: 1.2 },
  bloomDust: { count: 24, power: 1 },
  plagueCloud: { count: 44, power: 1 },
  plagueSmoke: { count: 22, power: 0.8 },
  sigilSparkle: { count: 26, power: 0.9 },
  sigilArcane: { count: 48, power: 1.4 },
} as const;

/** Each entrance's shake at the slam, on top of nothing (rule 9). The Void's is the heaviest. */
export const ENTRANCE_TRAUMA = {
  voidCollapse: 0.8,
  bigBoom: 0.75,
  titanBloom: FX_LEGENDARY_TRAUMA,
  plagueStomp: 0.6,
  tyrantSigil: FX_LEGENDARY_TRAUMA,
} as const;

/** Where Big Max's fuse sparks, as a fraction of the entry, before the slam at FX_SLAM_AT. */
export const ENTRANCE_FUSE_AT = 0.25;

/** The foot of the unit's zone, where dust and smoke rise (cues.ts's FOOT). */
const FOOT: FxPoint = { x: 0.5, y: 1 };

export type EntranceKey = keyof typeof ENTRANCE_TRAUMA;

/** What an entrance plans against: the entry's duration, the zone's testid and the intensity. */
export type EntrancePlan = { D: number; tgt: string; intensity: number };

type Entrance = (p: EntrancePlan) => FxCue[];

const voidCollapse: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    ringCue(D, "void", at, 0),
    tunedBurst(i, "void", at, "ring", 0, ENTRANCE_TUNING.voidPull),
    raysCue(D, "mythic", at, 0),
    tunedBurst(i, "void", at, "area", slam, ENTRANCE_TUNING.voidBurst),
    tunedBurst(i, "shard", at, "ring", slam, ENTRANCE_TUNING.voidShard),
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.voidCollapse, slam),
  ];
};

const bigBoom: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const fuse = frac(ENTRANCE_FUSE_AT, D);
  const slam = frac(FX_SLAM_AT, D);
  return [
    tunedBurst(i, "spark", at, "point", fuse, ENTRANCE_TUNING.fuseSpark),
    raysCue(D, "legendary", at, 0),
    ringCue(D, "fire", at, slam),
    tunedBurst(i, "fire", at, "area", slam, ENTRANCE_TUNING.boomFire),
    tunedBurst(i, "ember", at, "ring", slam, ENTRANCE_TUNING.boomEmber),
    tunedBurst(i, "smoke", tid(tgt, FOOT), "area", slam, ENTRANCE_TUNING.boomSmoke),
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.bigBoom, slam),
  ];
};

const titanBloom: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    raysCue(D, "holy", at, 0),
    ringCue(D, "dust", at, slam),
    tunedBurst(i, "dust", tid(tgt, FOOT), "ring", slam, ENTRANCE_TUNING.bloomDust),
    tunedBurst(i, "sparkle", at, "area", slam, ENTRANCE_TUNING.bloomSparkle),
    tunedBurst(i, "gold", at, "ring", slam, ENTRANCE_TUNING.bloomGold),
    ...shakeCues(i, ENTRANCE_TRAUMA.titanBloom, slam),
  ];
};

const plagueStomp: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    raysCue(D, "legendary", at, 0),
    ringCue(D, "poison", at, slam),
    tunedBurst(i, "poison", at, "area", slam, ENTRANCE_TUNING.plagueCloud),
    tunedBurst(i, "smoke", tid(tgt, FOOT), "ring", slam, ENTRANCE_TUNING.plagueSmoke),
    ...shakeCues(i, ENTRANCE_TRAUMA.plagueStomp, slam),
  ];
};

const tyrantSigil: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    ringCue(D, "arcane", at, 0),
    tunedBurst(i, "sparkle", at, "ring", 0, ENTRANCE_TUNING.sigilSparkle),
    raysCue(D, "mythic", at, slam),
    tunedBurst(i, "arcane", at, "area", slam, ENTRANCE_TUNING.sigilArcane),
    ...shakeCues(i, ENTRANCE_TRAUMA.tyrantSigil, slam),
  ];
};

export const ENTRANCES: { readonly [K in EntranceKey]: Entrance } = { voidCollapse, bigBoom, titanBloom, plagueStomp, tyrantSigil };

export function isEntranceKey(key: string): key is EntranceKey {
  return Object.prototype.hasOwnProperty.call(ENTRANCES, key);
}

/** The `summoned` an entrance may claim: into a unit zone, with a definition the viewer can read. */
export function entranceEvent(event: GameEvent, hiddenId: string): Extract<GameEvent, { type: "summoned" }> | null {
  if (event.type !== "summoned" || event.row !== "units" || event.defId === hiddenId) return null;
  return event;
}
