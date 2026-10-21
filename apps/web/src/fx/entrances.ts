// R670: the bespoke entrances of a handful of marquee Legendary and Mythic Units.
//
// A card listed in `CARD_FX` (cardFx.ts) under one of these keys gets its own entrance instead of its
// rarity's (cues.ts `summon`): its recipe claims the `summoned` into a unit zone. It keys off that
// event's own `defId`, which a hidden summon never has (R202), and decorates the entry the runner
// already timed, so it paces nothing (R200, R201): a delay lands inside the entry, and a tail is gone
// within FX_MAX_TAIL_MS.
//
//   #100 Ceaseless Void "voidCollapse", #80 BOOM! Big Max "bigBoom", #45 Nature Titan "titanBloom",
//   #56 Spell Tyrant "tyrantSigil".
//
// #1445: the per-flavor summon entrances one of those replaces for a marquee card, and the
// `summon` recipe (cues.ts) appends for every other face-up Unit. Each is additive over the
// motion underneath (the dust ring and burst, the rarity rays, the slam shake stay) and built
// only from existing cue kinds. Every delay is 0 or the landing beat and every tail reuses an
// existing tail constant, so an entrance paces nothing (R200); the flavor resolves through
// `flavorLookOf`, so a hidden summon keeps today's dust (R202).

import type { Flavor, GameEvent, PlayerView } from "@jackioh/shared";

import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import {
  FX_CRACK_TAIL_MS,
  FX_FLAVOR_LIVE,
  FX_LEGENDARY_TRAUMA,
  FX_SLAM_AT,
  FX_SUMMON_BRUTE_TRAUMA,
  FX_SUMMON_FLAVOR_TUNING,
} from "./constants.ts";
import { chaosFlavor, FLAVOR_LOOKS } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
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
  sigilSparkle: { count: 26, power: 0.9 },
  sigilArcane: { count: 48, power: 1.4 },
} as const;

/** Each entrance's shake at the slam, on top of nothing (rule 9). The Void's is the heaviest. */
export const ENTRANCE_TRAUMA = {
  voidCollapse: 0.8,
  bigBoom: 0.75,
  titanBloom: FX_LEGENDARY_TRAUMA,
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

export const ENTRANCES: { readonly [K in EntranceKey]: Entrance } = { voidCollapse, bigBoom, titanBloom, tyrantSigil };

/* ------------------------------------------------------------------------------------------- *
 * #1445: one summon entrance per flavor
 * ------------------------------------------------------------------------------------------- */

/** What a flavor entrance plans against: the entry's duration, the zone's testid, the intensity, the landing beat and the summoned card's instance id. */
export type FlavorEntrancePlan = { D: number; tgt: string; intensity: number; landAt: number; instanceId: string };

export type FlavorEntrance = (p: FlavorEntrancePlan) => FxCue[];

/** Physical: a short dust ring at its feet; the base dust ring underneath is its dust ring. */
const physical: FlavorEntrance = ({ tgt, intensity: i, landAt }) => [
  tunedBurst(i, "spark", tid(tgt, FOOT), "point", landAt, FX_SUMMON_FLAVOR_TUNING.physicalSpark),
];

/** Brute: a heavy landing, in the huge tier's board language (UNIT_SLAM huge: cracks plus shake). */
const brute: FlavorEntrance = ({ D, tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    ringCue(D, "dust", at, landAt),
    tunedBurst(i, "dust", tid(tgt, FOOT), "ring", landAt, FX_SUMMON_FLAVOR_TUNING.bruteCloud),
    tunedBurst(i, "dust", tid(tgt, FOOT), "area", landAt, FX_SUMMON_FLAVOR_TUNING.bruteHeavy),
    { kind: "crack", at, delayMs: landAt, durationMs: D - landAt + FX_CRACK_TAIL_MS },
    ...shakeCues(i, FX_SUMMON_BRUTE_TRAUMA, landAt),
  ];
};

/** Beast: claw marks scratched in around it. */
const beast: FlavorEntrance = ({ D, tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "shard", at, "ring", landAt, FX_SUMMON_FLAVOR_TUNING.beastClaw),
    tunedBurst(i, "shard", tid(tgt, FOOT), "point", landAt, FX_SUMMON_FLAVOR_TUNING.beastScratch),
    { kind: "crack", at, delayMs: landAt, durationMs: D - landAt + FX_CRACK_TAIL_MS },
  ];
};

/** Fire: embers rising. */
const fire: FlavorEntrance = ({ tgt, intensity: i, landAt }) => [
  tunedBurst(i, "fire", tid(tgt, FOOT), "area", landAt, FX_SUMMON_FLAVOR_TUNING.fireFlame),
  tunedBurst(i, "ember", tid(tgt, FOOT), "area", landAt, FX_SUMMON_FLAVOR_TUNING.fireEmber),
];

/** Dark: smoke rising from below. */
const dark: FlavorEntrance = ({ tgt, intensity: i, landAt }) => [
  tunedBurst(i, "void", tid(tgt, FOOT), "area", landAt, FX_SUMMON_FLAVOR_TUNING.darkRise),
  tunedBurst(i, "smoke", tid(tgt, FOOT), "area", landAt, FX_SUMMON_FLAVOR_TUNING.darkSmoke),
];

/** Holy: a column of light. */
const holy: FlavorEntrance = ({ D, tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [raysCue(D, "holy", at, 0), tunedBurst(i, "holy", at, "area", landAt, FX_SUMMON_FLAVOR_TUNING.holyLight)];
};

/** Nerd: pages fluttering. */
const nerd: FlavorEntrance = ({ tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "arcane", at, "area", landAt, FX_SUMMON_FLAVOR_TUNING.nerdPages),
    tunedBurst(i, "sparkle", at, "ring", landAt, FX_SUMMON_FLAVOR_TUNING.nerdMotes),
  ];
};

/** Tech: scanlines and a boot-up flicker. */
const tech: FlavorEntrance = ({ D, tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "frost", at, "ring", 0, FX_SUMMON_FLAVOR_TUNING.techScan),
    tunedBurst(i, "frost", at, "point", landAt, FX_SUMMON_FLAVOR_TUNING.techBoot),
    { kind: "sheen", at, delayMs: 0, durationMs: D },
  ];
};

/** Nature: leaves sprouting. */
const nature: FlavorEntrance = ({ tgt, intensity: i, landAt }) => [
  tunedBurst(i, "poison", tid(tgt, FOOT), "ring", landAt, FX_SUMMON_FLAVOR_TUNING.natureSprout),
  tunedBurst(i, "poison", tid(tgt, FOOT), "area", landAt, FX_SUMMON_FLAVOR_TUNING.natureBloom),
];

/** Toxic: bubbles popping. */
const toxic: FlavorEntrance = ({ tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "poison", at, "area", landAt, FX_SUMMON_FLAVOR_TUNING.toxicBubbles),
    tunedBurst(i, "poison", tid(tgt, FOOT), "point", landAt, FX_SUMMON_FLAVOR_TUNING.toxicPop),
  ];
};

/** Snack: crumbs falling. */
const snack: FlavorEntrance = ({ tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "ember", at, "area", landAt, FX_SUMMON_FLAVOR_TUNING.snackCrumbs),
    tunedBurst(i, "ember", tid(tgt, FOOT), "ring", landAt, FX_SUMMON_FLAVOR_TUNING.snackFall),
  ];
};

/** Hustle: a coin shower. */
const hustle: FlavorEntrance = ({ tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "gold", at, "area", landAt, FX_SUMMON_FLAVOR_TUNING.hustleShower),
    tunedBurst(i, "gold", tid(tgt, FOOT), "ring", landAt, FX_SUMMON_FLAVOR_TUNING.hustleCoins),
  ];
};

/** Clout: camera flashes. */
const clout: FlavorEntrance = ({ tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "confetti", at, "area", landAt, FX_SUMMON_FLAVOR_TUNING.cloutConfetti),
    tunedBurst(i, "spark", at, "point", landAt, FX_SUMMON_FLAVOR_TUNING.cloutFlash),
  ];
};

/** Spirit: a fade-in with wisps. */
const spirit: FlavorEntrance = ({ D, tgt, intensity: i, landAt }) => {
  const at = tid(tgt);
  return [
    { kind: "sheen", at, delayMs: 0, durationMs: D },
    tunedBurst(i, "sparkle", at, "area", 0, FX_SUMMON_FLAVOR_TUNING.spiritFade),
    tunedBurst(i, "sparkle", tid(tgt, FOOT), "ring", landAt, FX_SUMMON_FLAVOR_TUNING.spiritWisps),
  ];
};

/** Chaos: another flavor's entrance, picked from the instance id (never Chaos itself). */
const chaos: FlavorEntrance = (p) => FLAVOR_ENTRANCES[chaosFlavor(p.instanceId)](p);

/** One entrance per flavor; the `Record<Flavor, …>` type makes a missing one fail `typecheck`. */
export const FLAVOR_ENTRANCES: Readonly<Record<Flavor, FlavorEntrance>> = {
  Physical: physical,
  Brute: brute,
  Beast: beast,
  Fire: fire,
  Dark: dark,
  Holy: holy,
  Nerd: nerd,
  Tech: tech,
  Nature: nature,
  Toxic: toxic,
  Snack: snack,
  Hustle: hustle,
  Clout: clout,
  Spirit: spirit,
  Chaos: chaos,
};

/**
 * #1445: the flavor entrance a face-up `summoned` (or `animated`) appends to the motion
 * underneath, or nothing when the view hides the card. A summon the view does not show face-up
 * (a face-down set never reaches here; a hidden or unknown `defId` resolves to null through
 * `flavorLookOf`, CLAUDE.md rule 7) and reduced motion (every duration is 0) keep today's cue.
 */
export function flavorEntranceCues(opts: {
  defId: string;
  instanceId: string;
  view: PlayerView;
  D: number;
  tgt: string;
  intensity: number;
  landAt: number;
}): FxCue[] {
  if (!FX_FLAVOR_LIVE || !(opts.D > 0)) return [];
  const look = flavorLookOf(opts.defId, opts.instanceId, opts.view);
  if (look === null) return [];
  const flavor = (Object.keys(FLAVOR_LOOKS) as Flavor[]).find((key) => FLAVOR_LOOKS[key] === look);
  if (flavor === undefined) return [];
  return FLAVOR_ENTRANCES[flavor]({ D: opts.D, tgt: opts.tgt, intensity: opts.intensity, landAt: opts.landAt, instanceId: opts.instanceId });
}

export function isEntranceKey(key: string): key is EntranceKey {
  return Object.prototype.hasOwnProperty.call(ENTRANCES, key);
}

/** The `summoned` an entrance may claim: into a unit zone, with a definition the viewer can read. */
export function entranceEvent(event: GameEvent, hiddenId: string): Extract<GameEvent, { type: "summoned" }> | null {
  if (event.type !== "summoned" || event.row !== "units" || event.defId === hiddenId) return null;
  return event;
}
