// R502, R2784000: every recipe a `{ bespoke }` entry of a set's effects table may name: the card recipes
// that claim events of their play (cardFx.ts). R3332000: a marquee entrance (entrances.ts) is an entry's
// `entrance` since, never a `{ bespoke }`, so a bespoke key is a card recipe's. R2795000: Classic+'s
// Flourishes (Rollback, KY's Constant, Book of Pain) join Crushing Walls.
// R2794000: Classic's three Flourishes, pageMend, recycleLoop and crownSwap, are card recipes. R2797000:
// Stinkyscape's three of part one (fx/cards/stinkyscape/), beside part two's (#2798). R3338000: Olivian
// Expanse's rift is its set piece now (fx/terminus/). R3402001: Core's signatures add Silly Silas's
// mirrorball shuffle, Unlicensed Experimentation's clamp weld, Pocket Chaos's swap, Combo-Index's grade
// rings, Call to Chaos's reels, My Pawn's strings and Heroic Power's doors.

import { ENTRANCE_TRAUMA, type CardRecipe, type EntranceKey } from "./recipe.ts";
import { bloodDrain } from "./core/bloodRidden.ts";
import { chaosReels } from "./core/callToChaos.ts";
import { gradeRings } from "./core/comboIndex.ts";
import { fusionForge } from "./core/craftACard.ts";
import { heroicDoors } from "./core/heroicPower.ts";
import { manaCrack } from "./core/hinder.ts";
import { pawnStrings } from "./core/myPawn.ts";
import { pocketSwap } from "./core/pocketChaos.ts";
import { mirrorballShuffle } from "./core/sillySilas.ts";
import { legendaryTide } from "./core/transmogulate.ts";
import { clampWeld } from "./core/unlicensedExperimentation.ts";
import { perfectWish } from "./core/zephrys.ts";
import { inkTear } from "./classicplus/bookOfPain.ts";
import { crushingWalls } from "./classicplus/crushingWalls.ts";
import { constantStamp } from "./classicplus/kysConstant.ts";
import { rollbackSweep } from "./classicplus/rollback.ts";
import { jadeCrack } from "./meditative/auspiciousRock.ts";
import { mindJudge } from "./meditative/mindJudge.ts";
import { cnSweep } from "./meditative/spirituallyZhongguo.ts";
import { harmonyWave } from "./meditativeplus/harmonySpirit.ts";
import { battleshipWarp } from "./stinkyscape/battleshipWarp.ts";
import { duoBond } from "./stinkyscape/duoBond.ts";
import { flashFreeze } from "./stinkyscape/flashFreeze.ts";
import { magiBoot } from "./stinkyscape/magiBoot.ts";
import { stratosSwarm } from "./stinkyscape/stratosSwarm.ts";
import { celestialCall } from "./traininggrounds/celestialCaller.ts";
import { dinoStomp } from "./traininggrounds/dinoSize.ts";
import { gameDesign } from "./traininggrounds/goetzmannGameDesign.ts";
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
  mirrorballShuffle,
  clampWeld,
  pocketSwap,
  gradeRings,
  chaosReels,
  pawnStrings,
  heroicDoors,
  rollbackSweep,
  constantStamp,
  inkTear,
  jadeCrack,
  mindJudge,
  cnSweep,
  harmonyWave,
  magiBoot,
  battleshipWarp,
  duoBond,
  flashFreeze,
  stratosSwarm,
  celestialCall,
  dinoStomp,
  gameDesign,
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
