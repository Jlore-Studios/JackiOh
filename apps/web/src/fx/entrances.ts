// R670: the bespoke entrances of a handful of marquee Legendary and Mythic Units (#100 Ceaseless Void
// "voidCollapse", #80 BOOM! Big Max "bigBoom", #45 Nature Titan "titanBloom", #56 Spell Tyrant "tyrantSigil"),
// in place of its rarity's (cues.ts `summon`) for a card whose entry in its set's effects table names it
// as its `entrance` (R2784000, R3332000, fx/cards/tables.ts); each recipe is its card's own file in
// fx/cards/<set>/. R3332001: #92 Felinor Fiender "felinorAssembly", the first signature's. R3334004: the
// Dormant Starship's "shipyardRollout" and #137 Starfleet Commander's "fleetCommand". R3332000: a
// Field Spell's own entrance plays as it lands face up in the backrow. R3400001: Meditative+ #24 Bob's
// "snowplowHeave", Training Grounds #38 Tutorial Master's "chalkboardLesson" and #39.1 the Training-Grounds
// Golem's "golemLashing". R3389001: Classic's signatures' six (#4 Palantir "seeingOrb", #7 InfiniScepter
// "sceptrePlant", #28 Second Wind "galeBurst", #33 Joro "silkDrop", #85 King Wagtoggle "royalCloak", #90 In
// Too Deep "abyssSurface"). R3407000: Stinkyscape part two's (fx/cards/stinkyscape/). R3405001: Meditative's
// twenty-five, from #7 Introspection's "poolBreath" to #97.9 Jlockeed's Headquarters' "fortressRise", each
// starting after its entry's lead (`leadMs`, R3405000). R3399000: Classic+'s signatures' seventeen, from #12
// The Mother Pancake's "pancakeStack" to #78 Claude's Datacenter's "obeliskRise". R3395000: Stinkyscape 001 to
// 090's signature Units and Field Spells, from #6 Tylannt's "vaultStride" to #88 Siva 2's "venomStep"
// (fx/cards/stinkyscape/), each starting after its entry's lead too. It keys off the `summoned` event's
// own `defId`, which a hidden summon never has (R202), and decorates the entry the runner already timed,
// so it paces nothing (R200, R201).
//
// #1445: the per-flavor entrances `summon` appends for every other face-up Unit, additive over the motion
// underneath, their delays 0 or the landing beat and their tails existing tail constants.

import type { Flavor, GameEvent, PlayerView } from "@jackioh/shared";

import { raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { bigBoom } from "./cards/classic/bigMax.ts";
import { sceptrePlant } from "./cards/classic/infiniScepter.ts";
import { abyssSurface } from "./cards/classic/inTooDeep.ts";
import { silkDrop } from "./cards/classic/joro.ts";
import { royalCloak } from "./cards/classic/kingWagtoggle.ts";
import { titanBloom } from "./cards/classic/natureTitan.ts";
import { seeingOrb } from "./cards/classic/palantir.ts";
import { galeBurst } from "./cards/classic/secondWind.ts";
import { tyrantSigil } from "./cards/classic/spellTyrant.ts";
import { voidCollapse } from "./cards/core/ceaselessVoid.ts";
import { felinorAssembly } from "./cards/core/felinorFiender.ts";
import { flexBurst } from "./cards/classicplus/botLoser.ts";
import { templeStack } from "./cards/classicplus/classicGolem.ts";
import { obeliskRise } from "./cards/classicplus/datacenter.ts";
import { bannerPlant } from "./cards/classicplus/flagbearer.ts";
import { greatBanner } from "./cards/classicplus/flagbearerPrime.ts";
import { griddleFreeze } from "./cards/classicplus/frostspatula.ts";
import { vineSwing } from "./cards/classicplus/jlease.ts";
import { vineNap } from "./cards/classicplus/jungleLoser.ts";
import { giftDesk } from "./cards/classicplus/kysGift.ts";
import { briefcaseStride } from "./cards/classicplus/lobbyist.ts";
import { spotlightPose } from "./cards/classicplus/midLoser.ts";
import { kitchenDoor } from "./cards/classicplus/mommyBarker.ts";
import { pancakeStack } from "./cards/classicplus/motherPancake.ts";
import { staffShuffle } from "./cards/classicplus/supportLoser.ts";
import { shieldPlant } from "./cards/classicplus/topLoser.ts";
import { waffleIron } from "./cards/classicplus/waffleShell.ts";
import { drumLeap } from "./cards/classicplus/wardrum.ts";
import { dojoBoot } from "./cards/meditative/aiGirlfriend.ts";
import { crackedEarth } from "./cards/meditative/alternateFate.ts";
import { lanternLight } from "./cards/meditative/aluneth.ts";
import { golemLock } from "./cards/meditative/cnGolem.ts";
import { cushionDrop } from "./cards/meditative/devinBot.ts";
import { floodlights } from "./cards/meditative/doubleHeader.ts";
import { treeSprout } from "./cards/meditative/everGrowingTree.ts";
import { fiveCompass } from "./cards/meditative/fengShui.ts";
import { poolBreath } from "./cards/meditative/introspection.ts";
import { jadeBoulder } from "./cards/meditative/jadeBeauty.ts";
import { fortressRise } from "./cards/meditative/jlockeedHq.ts";
import { keyRing } from "./cards/meditative/keymasterKeenus.ts";
import { chalkMask } from "./cards/meditative/knowledgeBreaker.ts";
import { maskSplit } from "./cards/meditative/knowledgeBreakerPrime.ts";
import { coinFlip } from "./cards/meditative/mayorMedinamogger.ts";
import { studyCurve } from "./cards/meditative/paretoOptimality.ts";
import { prisonBars } from "./cards/meditative/prison.ts";
import { skullThud } from "./cards/meditative/skullOfJnari.ts";
import { bronzeCauldron } from "./cards/meditative/taotie.ts";
import { totemStack } from "./cards/meditative/tatchesTheTotem.ts";
import { caneRise } from "./cards/meditative/theCane.ts";
import { chalkHand } from "./cards/meditative/touchedByKy.ts";
import { scrollScholar } from "./cards/meditative/yileGpt.ts";
import { pavilionUnfurl } from "./cards/meditative/yileGptPrime.ts";
import { virusBoot } from "./cards/meditative/yilesVirus.ts";
import { snowplowHeave } from "./cards/meditativeplus/bobSoldier.ts";
import { golemLashing } from "./cards/traininggrounds/trainingGolem.ts";
import { chalkboardLesson } from "./cards/traininggrounds/tutorialMaster.ts";
import { vaultStride } from "./cards/stinkyscape/tylannt.ts";
import { heistDrop } from "./cards/stinkyscape/greatestHeist.ts";
import { rodRise } from "./cards/stinkyscape/rodOfAges.ts";
import { lecternRise } from "./cards/stinkyscape/zolarKeth.ts";
import { spotlightKiss } from "./cards/stinkyscape/kwzka.ts";
import { labVault } from "./cards/stinkyscape/narcorProject.ts";
import { emitterBloom } from "./cards/stinkyscape/beamV5.ts";
import { barrelSpin } from "./cards/stinkyscape/repeaterV5.ts";
import { breechLock } from "./cards/stinkyscape/cannonV5.ts";
import { turretScan } from "./cards/stinkyscape/flakV5.ts";
import { intakeMaw } from "./cards/stinkyscape/minerV5.ts";
import { wraithGather } from "./cards/stinkyscape/zZoth.ts";
import { heraldHorn } from "./cards/stinkyscape/aZoth.ts";
import { paragonStreak } from "./cards/stinkyscape/paragonMk2.ts";
import { paragonDive } from "./cards/stinkyscape/paragonX.ts";
import { flagshipDescent } from "./cards/stinkyscape/galaxy.ts";
import { testLurch } from "./cards/stinkyscape/prototypeX1.ts";
import { forgeBurst } from "./cards/stinkyscape/prototypeX2.ts";
import { redCarpet } from "./cards/stinkyscape/siva1.ts";
import { venomStep } from "./cards/stinkyscape/siva2.ts";
import { alteranDive } from "./cards/stinkyscape/alteranReach.ts";
import { golemCrash } from "./cards/stinkyscape/chaosCalls.ts";
import { freshJackVault } from "./cards/stinkyscape/freshJack.ts";
import { ikwzkiDawn } from "./cards/stinkyscape/ikwzki.ts";
import { blobPlop, in33dh3Drop, primeCrown } from "./cards/stinkyscape/in33dh3.ts";
import { resumeTube } from "./cards/stinkyscape/jobApplication.ts";
import { balthasarLamp, casperMirror, melchiorBoot } from "./cards/stinkyscape/magi.ts";
import { megabaseDock } from "./cards/stinkyscape/megabase.ts";
import { sanctumRise } from "./cards/stinkyscape/sanctum.ts";
import { scornGate } from "./cards/stinkyscape/scornboi.ts";
import { shockwaveRing } from "./cards/stinkyscape/shockwave447.ts";
import { FOOT, type Entrance, type EntranceKey } from "./cards/recipe.ts";
import { fleetCommand, shipyardRollout } from "./cards/stinkyscape/starshipSignatures.ts";
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
  shipyardRollout,
  fleetCommand,
  snowplowHeave,
  chalkboardLesson,
  golemLashing,
  vaultStride,
  heistDrop,
  rodRise,
  lecternRise,
  spotlightKiss,
  labVault,
  emitterBloom,
  barrelSpin,
  breechLock,
  turretScan,
  intakeMaw,
  wraithGather,
  heraldHorn,
  paragonStreak,
  paragonDive,
  flagshipDescent,
  testLurch,
  forgeBurst,
  redCarpet,
  venomStep,
  seeingOrb,
  sceptrePlant,
  galeBurst,
  silkDrop,
  royalCloak,
  abyssSurface,
  // R3399000: Classic+'s signatures.
  pancakeStack,
  waffleIron,
  griddleFreeze,
  kitchenDoor,
  shieldPlant,
  vineNap,
  spotlightPose,
  staffShuffle,
  flexBurst,
  drumLeap,
  giftDesk,
  bannerPlant,
  greatBanner,
  briefcaseStride,
  templeStack,
  vineSwing,
  obeliskRise,
  // R3407000: Stinkyscape part two's signatures (fx/cards/stinkyscape/).
  melchiorBoot,
  balthasarLamp,
  casperMirror,
  freshJackVault,
  ikwzkiDawn,
  shockwaveRing,
  resumeTube,
  scornGate,
  megabaseDock,
  in33dh3Drop,
  primeCrown,
  blobPlop,
  alteranDive,
  golemCrash,
  sanctumRise,
  // R3405001: Meditative's signature Units' and Field Spells'.
  poolBreath,
  floodlights,
  crackedEarth,
  jadeBoulder,
  fiveCompass,
  chalkMask,
  maskSplit,
  bronzeCauldron,
  scrollScholar,
  pavilionUnfurl,
  virusBoot,
  dojoBoot,
  cushionDrop,
  skullThud,
  keyRing,
  studyCurve,
  caneRise,
  treeSprout,
  chalkHand,
  lanternLight,
  coinFlip,
  totemStack,
  golemLock,
  prisonBars,
  fortressRise,
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
