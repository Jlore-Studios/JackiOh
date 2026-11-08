// R502, R2784000: every recipe a `{ bespoke }` entry of a set's effects table may name: the card recipes
// that claim events of their play (cardFx.ts). R3332000: a marquee entrance (entrances.ts) is an entry's
// `entrance` since, never a `{ bespoke }`, so a bespoke key is a card recipe's. R2795000: Classic+'s
// Flourishes (Rollback, KY's Constant, Book of Pain) join Crushing Walls.
// R2794000: Classic's three Flourishes, pageMend, recycleLoop and crownSwap, are card recipes. R2797000:
// Stinkyscape's three of part one (fx/cards/stinkyscape/), beside part two's (#2798). R3338000: Olivian
// Expanse's rift is its set piece now (fx/terminus/). R3337002's and R3337003's Scanning Probes' scan and
// Anomaly Found!'s call (#3337). R3400001: the signatures of Meditative+'s and Training Grounds' Legendaries
// and Mythics (Bob's Plan, Bob, Scratch Pad, Seance, Tutorial Master, Call to Chaos (Training-Grounds
// Edition); Goetzmann Game Design's grows from its flourish). R3407000: Stinkyscape part two's signatures.

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
import { sanctumEcho } from "./stinkyscape/sanctum.ts";
import { scornSweep } from "./stinkyscape/scornboi.ts";
import { shockwaveLecture } from "./stinkyscape/shockwave447.ts";
import { stimmyDrop } from "./stinkyscape/stimmycheck.ts";
import { chaosOrb } from "./stinkyscape/wildar.ts";
import { probeScan } from "./stinkyscape/probeScan.ts";
import { stratosSwarm } from "./stinkyscape/stratosSwarm.ts";
import { celestialCall } from "./traininggrounds/celestialCaller.ts";
import { chaosDice } from "./traininggrounds/chaosDice.ts";
import { dinoStomp } from "./traininggrounds/dinoSize.ts";
import { gameDesign } from "./traininggrounds/goetzmannGameDesign.ts";
import { seance } from "./traininggrounds/seance.ts";
import { tutorialMaster } from "./traininggrounds/tutorialMaster.ts";
import { pageMend } from "./classic/bookOfHeal.ts";
import { crownSwap } from "./classic/kingWagtoggle.ts";
import { recycleLoop } from "./classic/recycle.ts";

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
  battleshipWarp,
  duoBond,
  flashFreeze,
  stratosSwarm,
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
} as const satisfies Readonly<Record<string, CardRecipe>>;

export type CardRecipeKey = keyof typeof CARD_RECIPES;

export function isCardRecipeKey(key: string): key is CardRecipeKey {
  return Object.prototype.hasOwnProperty.call(CARD_RECIPES, key);
}

export function isEntranceKey(key: string): key is EntranceKey {
  return Object.prototype.hasOwnProperty.call(ENTRANCE_TRAUMA, key);
}
