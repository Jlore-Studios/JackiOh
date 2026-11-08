// R502, R2784000: every recipe a `{ bespoke }` entry of a set's effects table may name: the card recipes
// that claim events of their play (cardFx.ts). R3332000: a marquee entrance (entrances.ts) is an entry's
// `entrance` since, never a `{ bespoke }`, so a bespoke key is a card recipe's. R2795000: Classic+'s
// Flourishes (Rollback, KY's Constant, Book of Pain) join Crushing Walls.
// R2794000: Classic's three Flourishes, pageMend, recycleLoop and crownSwap, are card recipes. R2797000:
// Stinkyscape's three of part one (fx/cards/stinkyscape/), beside part two's (#2798). R3338000: Olivian
// Expanse's rift is its set piece now (fx/terminus/). R3337002's and R3337003's Scanning Probes' scan and
// Anomaly Found!'s call (#3337). R3334004: #176 Preemptive Strike's signature. R3400001: the signatures of
// Meditative+'s and Training Grounds' Legendaries and Mythics (Bob's Plan, Bob, Scratch Pad, Seance, Tutorial
// Master, Call to Chaos (Training-Grounds Edition); Goetzmann Game Design's grows from its flourish).
// R3389001: Classic's eleven signatures' recipes (#3389), King Wagtoggle's crownSwap grown among them.
// R3407000: Stinkyscape part two's signatures.
// R3395000: the signatures of Stinkyscape 001 to 090's Legendaries and Mythics (fx/cards/stinkyscape/;
// Imitated Instrumentality's magiBoot grows from its flourish).

import { ENTRANCE_TRAUMA, type CardRecipe, type EntranceKey } from "./recipe.ts";
import { bloodDrain } from "./core/bloodRidden.ts";
import { fusionForge } from "./core/craftACard.ts";
import { manaCrack } from "./core/hinder.ts";
import { legendaryTide } from "./core/transmogulate.ts";
import { perfectWish } from "./core/zephrys.ts";
import { inkTear } from "./classicplus/bookOfPain.ts";
import { crushingWalls } from "./classicplus/crushingWalls.ts";
import { constantStamp } from "./classicplus/kysConstant.ts";
import { rollbackSweep } from "./classicplus/rollback.ts";
import { jadeCrack } from "./meditative/auspiciousRock.ts";
import { mindJudge } from "./meditative/mindJudge.ts";
import { cnSweep } from "./meditative/spirituallyZhongguo.ts";
import { bobSoldier } from "./meditativeplus/bobSoldier.ts";
import { bobsPlan } from "./meditativeplus/bobsPlan.ts";
import { harmonyWave } from "./meditativeplus/harmonySpirit.ts";
import { scratchPad } from "./meditativeplus/scratchPad.ts";
import { anomalyCall } from "./stinkyscape/anomalyCall.ts";
import { battleshipWarp } from "./stinkyscape/battleshipWarp.ts";
import { duoBond } from "./stinkyscape/duoBond.ts";
import { flashFreeze } from "./stinkyscape/flashFreeze.ts";
import { magiBoot } from "./stinkyscape/magiBoot.ts";
import { edgeFlee, starscapeJump } from "./stinkyscape/alteranReach.ts";
import { stinkCloud, tabletopTilt } from "./stinkyscape/chaosCalls.ts";
import { freshJackComet } from "./stinkyscape/freshJack.ts";
import { blobDrum, dataPacket, primeConduct } from "./stinkyscape/in33dh3.ts";
import { jobInterview, payday } from "./stinkyscape/jobApplication.ts";
import { balthasarCradle, casperKiss, melchiorTally } from "./stinkyscape/magi.ts";
import { giftShop } from "./stinkyscape/megabase.ts";
import { narcorDust, redNarcorHum } from "./stinkyscape/narcor.ts";
import { preemptiveStrike } from "./stinkyscape/preemptiveStrike.ts";
import { sanctumEcho } from "./stinkyscape/sanctum.ts";
import { scornSweep } from "./stinkyscape/scornboi.ts";
import { shockwaveLecture } from "./stinkyscape/shockwave447.ts";
import { stimmyDrop } from "./stinkyscape/stimmycheck.ts";
import { chaosOrb } from "./stinkyscape/wildar.ts";
import { probeScan } from "./stinkyscape/probeScan.ts";
import { stratosSwarm } from "./stinkyscape/stratosSwarm.ts";
import { echoRings } from "./stinkyscape/aZoth.ts";
import { beamCharge } from "./stinkyscape/beamV5.ts";
import { cannonShell } from "./stinkyscape/cannonV5.ts";
import { droneConveyor } from "./stinkyscape/droneCoreEight.ts";
import { seedPod } from "./stinkyscape/droneCoreNine.ts";
import { flakWaves } from "./stinkyscape/flakV5.ts";
import { bellyBay } from "./stinkyscape/galaxy.ts";
import { heistLocks } from "./stinkyscape/greatestHeist.ts";
import { allureRibbon } from "./stinkyscape/kwzka.ts";
import { asteroidCrunch } from "./stinkyscape/minerV5.ts";
import { narcorCases } from "./stinkyscape/narcorProject.ts";
import { paperclipLink } from "./stinkyscape/paperclipTheory.ts";
import { wingMount } from "./stinkyscape/prototypeX1.ts";
import { spineWeld } from "./stinkyscape/prototypeX2.ts";
import { repeaterBurst } from "./stinkyscape/repeaterV5.ts";
import { rodBands } from "./stinkyscape/rodOfAges.ts";
import { glowLeash } from "./stinkyscape/siva1.ts";
import { venomHand } from "./stinkyscape/siva2.ts";
import { vaultHeist } from "./stinkyscape/tylannt.ts";
import { ghostHulls } from "./stinkyscape/zZoth.ts";
import { campusLaunch } from "./stinkyscape/zolarKeth.ts";
import { celestialCall } from "./traininggrounds/celestialCaller.ts";
import { chaosDice } from "./traininggrounds/chaosDice.ts";
import { dinoStomp } from "./traininggrounds/dinoSize.ts";
import { gameDesign } from "./traininggrounds/goetzmannGameDesign.ts";
import { seance } from "./traininggrounds/seance.ts";
import { tutorialMaster } from "./traininggrounds/tutorialMaster.ts";
import { graveRise } from "./classic/backFromTheGy.ts";
import { craterLock } from "./classic/bigMax.ts";
import { pageMend } from "./classic/bookOfHeal.ts";
import { taxSeizure } from "./classic/incomeTax.ts";
import { sceptreLocket } from "./classic/infiniScepter.ts";
import { shrineDescent } from "./classic/inTooDeep.ts";
import { webRedirect } from "./classic/joro.ts";
import { crownSwap } from "./classic/kingWagtoggle.ts";
import { titanLeaf } from "./classic/natureTitan.ts";
import { palantirGaze } from "./classic/palantir.ts";
import { recycleLoop } from "./classic/recycle.ts";
import { secondGust } from "./classic/secondWind.ts";
import { tyrantCommand } from "./classic/spellTyrant.ts";

export const CARD_RECIPES = {
  manaCrack,
  bloodDrain,
  crushingWalls,
  legendaryTide,
  perfectWish,
  fusionForge,
  rollbackSweep,
  constantStamp,
  inkTear,
  jadeCrack,
  mindJudge,
  cnSweep,
  harmonyWave,
  bobsPlan,
  bobSoldier,
  scratchPad,
  magiBoot,
  vaultHeist,
  heistLocks,
  rodBands,
  campusLaunch,
  droneConveyor,
  seedPod,
  allureRibbon,
  narcorCases,
  paperclipLink,
  beamCharge,
  repeaterBurst,
  cannonShell,
  flakWaves,
  asteroidCrunch,
  ghostHulls,
  echoRings,
  bellyBay,
  wingMount,
  spineWeld,
  glowLeash,
  venomHand,
  battleshipWarp,
  duoBond,
  flashFreeze,
  stratosSwarm,
  preemptiveStrike,
  probeScan,
  anomalyCall,
  // R3407000: Stinkyscape part two's signatures.
  melchiorTally,
  balthasarCradle,
  casperKiss,
  freshJackComet,
  shockwaveLecture,
  jobInterview,
  payday,
  scornSweep,
  giftShop,
  dataPacket,
  primeConduct,
  blobDrum,
  starscapeJump,
  edgeFlee,
  chaosOrb,
  stinkCloud,
  tabletopTilt,
  sanctumEcho,
  stimmyDrop,
  narcorDust,
  redNarcorHum,
  celestialCall,
  dinoStomp,
  gameDesign,
  seance,
  tutorialMaster,
  chaosDice,
  pageMend,
  recycleLoop,
  crownSwap,
  palantirGaze,
  sceptreLocket,
  taxSeizure,
  secondGust,
  webRedirect,
  graveRise,
  titanLeaf,
  tyrantCommand,
  craterLock,
  shrineDescent,
} as const satisfies Readonly<Record<string, CardRecipe>>;

export type CardRecipeKey = keyof typeof CARD_RECIPES;

export function isCardRecipeKey(key: string): key is CardRecipeKey {
  return Object.prototype.hasOwnProperty.call(CARD_RECIPES, key);
}

export function isEntranceKey(key: string): key is EntranceKey {
  return Object.prototype.hasOwnProperty.call(ENTRANCE_TRAUMA, key);
}
