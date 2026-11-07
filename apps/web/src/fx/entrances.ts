// R670: the bespoke entrances of a handful of marquee Legendary and Mythic Units (#100 Ceaseless Void
// "voidCollapse", #80 BOOM! Big Max "bigBoom", #45 Nature Titan "titanBloom", #56 Spell Tyrant "tyrantSigil"),
// in place of its rarity's (cues.ts `summon`) for a card whose entry in its set's effects table names it
// as its `entrance` (R2784000, R3332000, fx/cards/tables.ts); each recipe is its card's own file in
// fx/cards/<set>/. R3332001: #92 Felinor Fiender "felinorAssembly", the first signature's; R3402001: #52
// Silly Silas "silasSpotlight", #93 Combo-Index "comboStadium" and #98 Heroic Power "heroicPedestal". R3332000: a
// Field Spell's own entrance plays as it lands face up in the backrow. It keys off the `summoned` event's
// own `defId`, which a hidden summon never has (R202), and decorates the entry the runner already timed,
// so it paces nothing (R200, R201).
//
// #1445: the per-flavor entrances `summon` appends for every other face-up Unit, additive over the motion
// underneath, their delays 0 or the landing beat and their tails existing tail constants.

import type { Flavor, GameEvent, PlayerView } from "@jackioh/shared";

import { raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { bigBoom } from "./cards/classic/bigMax.ts";
import { titanBloom } from "./cards/classic/natureTitan.ts";
import { tyrantSigil } from "./cards/classic/spellTyrant.ts";
import { voidCollapse } from "./cards/core/ceaselessVoid.ts";
import { comboStadium } from "./cards/core/comboIndex.ts";
import { felinorAssembly } from "./cards/core/felinorFiender.ts";
import { heroicPedestal } from "./cards/core/heroicPower.ts";
import { silasSpotlight } from "./cards/core/sillySilas.ts";
import { FOOT, type Entrance, type EntranceKey } from "./cards/recipe.ts";
import { FX_CRACK_TAIL_MS, FX_FLAVOR_LIVE, FX_SUMMON_BRUTE_TRAUMA, FX_SUMMON_FLAVOR_TUNING } from "./constants.ts";
import { chaosFlavor, FLAVOR_LOOKS } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
import type { FxCue } from "./types.ts";

export { ENTRANCE_FUSE_AT, ENTRANCE_TRAUMA, type EntranceKey, type EntrancePlan } from "./cards/recipe.ts";
export { isEntranceKey } from "./cards/bespoke.ts";

export const ENTRANCES: { readonly [K in EntranceKey]: Entrance } = {
  voidCollapse,
  bigBoom,
  titanBloom,
  tyrantSigil,
  felinorAssembly,
  silasSpotlight,
  comboStadium,
  heroicPedestal,
};

/* #1445: one summon entrance per flavor */

/** An `EntrancePlan` plus the landing beat and the summoned card's instance id. */
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
 * #1445: the flavor entrance a face-up `summoned` (or `animated`) appends to the motion underneath.
 * A hidden or unknown `defId` resolves to null through `flavorLookOf` (CLAUDE.md rule 7), so it and
 * reduced motion (every duration is 0) get nothing.
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

/**
 * The `summoned` an entrance may claim, with a definition the viewer can read: into a unit zone, or
 * R3332000 for a Field Spell (`type`, its catalog type) into a backrow zone face up, never as it is set
 * face down (its fresh id's `formerId`, R227, R2722002).
 */
export function entranceEvent(event: GameEvent, hiddenId: string, type: string | undefined): Extract<GameEvent, { type: "summoned" }> | null {
  if (event.type !== "summoned" || event.defId === hiddenId) return null;
  if (type === "Field Spell") return event.row === "backrow" && event.formerId === undefined ? event : null;
  return event.row === "units" ? event : null;
}
