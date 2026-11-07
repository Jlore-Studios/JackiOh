// R502, R2784000: every recipe a `{ bespoke }` entry of a set's effects table may name: the card recipes
// that claim events of their play (cardFx.ts). R3332000: a marquee entrance (entrances.ts) is an entry's
// `entrance` since, never a `{ bespoke }`, so a bespoke key is a card recipe's. R2795000: Classic+'s
// Flourishes (Rollback, KY's Constant, Book of Pain) join Crushing Walls.
// R2794000: Classic's three Flourishes, pageMend, recycleLoop and crownSwap, are card recipes. R2797000:
// Stinkyscape's three of part one (fx/cards/stinkyscape/), beside part two's (#2798). R3338000: Olivian
// Expanse's rift is its set piece now (fx/terminus/).

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
import { holoLoom } from "./classicplus/aiSlop.ts";
import { killNotch } from "./classicplus/botLoser.ts";
import { chaosLever } from "./classicplus/callToChaos.ts";
import { printSlot } from "./classicplus/datacenter.ts";
import { batterBubble } from "./classicplus/deathBoil.ts";
import { pancakeMouth } from "./classicplus/devour.ts";
import { bannerFold } from "./classicplus/flagbearer.ts";
import { bannerShadow } from "./classicplus/flagbearerPrime.ts";
import { doughArm } from "./classicplus/fluffyGrip.ts";
import { spatulaFlip } from "./classicplus/frostspatula.ts";
import { whiteout } from "./classicplus/frozenWastes.ts";
import { grapeBounce } from "./classicplus/goldenGrape.ts";
import { packLob } from "./classicplus/jlease.ts";
import { relicSpill } from "./classicplus/jleasePack.ts";
import { puzzleBox } from "./classicplus/joggsBox.ts";
import { giftBoxes } from "./classicplus/kysGift.ts";
import { slateTest } from "./classicplus/kysTest.ts";
import { lineup } from "./classicplus/league.ts";
import { hungryPot } from "./classicplus/legion.ts";
import { sealedEnvelope } from "./classicplus/lobbyist.ts";
import { coinCall } from "./classicplus/midLoser.ts";
import { pancakeTray } from "./classicplus/mommyBarker.ts";
import { pancakeFlip } from "./classicplus/motherPancake.ts";
import { vineRipen } from "./classicplus/mythicGrape.ts";
import { sepiaPortal } from "./classicplus/portal.ts";
import { flourCloud } from "./classicplus/powderSpray.ts";
import { sighMend } from "./classicplus/supportLoser.ts";
import { stairGlue } from "./classicplus/twiceForward.ts";
import { waffleGrid } from "./classicplus/waffleShell.ts";
import { drumBeat } from "./classicplus/wardrum.ts";
import { zealotCyclone } from "./classicplus/zealotism.ts";
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
  // R3399000: Classic+'s signatures, in catalog order.
  pancakeFlip,
  pancakeMouth,
  batterBubble,
  doughArm,
  flourCloud,
  waffleGrid,
  whiteout,
  hungryPot,
  spatulaFlip,
  pancakeTray,
  lineup,
  coinCall,
  sighMend,
  killNotch,
  zealotCyclone,
  sepiaPortal,
  drumBeat,
  slateTest,
  giftBoxes,
  holoLoom,
  bannerFold,
  bannerShadow,
  puzzleBox,
  sealedEnvelope,
  grapeBounce,
  vineRipen,
  chaosLever,
  stairGlue,
  packLob,
  relicSpill,
  printSlot,
} as const satisfies Readonly<Record<string, CardRecipe>>;

export type CardRecipeKey = keyof typeof CARD_RECIPES;

export function isCardRecipeKey(key: string): key is CardRecipeKey {
  return Object.prototype.hasOwnProperty.call(CARD_RECIPES, key);
}

export function isEntranceKey(key: string): key is EntranceKey {
  return Object.prototype.hasOwnProperty.call(ENTRANCE_TRAUMA, key);
}
