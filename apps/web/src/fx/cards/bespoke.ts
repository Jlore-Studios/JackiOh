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
// R3402001: Core's signatures add Silly Silas's mirrorball shuffle, Unlicensed Experimentation's clamp weld,
// Pocket Chaos's swap, Combo-Index's grade rings, Call to Chaos's reels, My Pawn's strings and Heroic Power's
// doors.
// R3407000: Stinkyscape part two's signatures.
// R3405001: Meditative's Legendaries' and Mythics' signatures (fx/cards/meditative/), every one but The
// Cane's and Mayor Medinamogger's, whose signatures are their entrances alone. R3399000: Classic+'s signatures'
// thirty recipes (#3399), in catalog order.
// R3395000: the signatures of Stinkyscape 001 to 090's Legendaries and Mythics (fx/cards/stinkyscape/;
// Imitated Instrumentality's magiBoot grows from its flourish).

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
import { holoLoom } from "./classicplus/aiSlop.ts";
import { killNotch } from "./classicplus/botLoser.ts";
import { chaosLever } from "./classicplus/callToChaos.ts";
import { printSlot } from "./classicplus/datacenter.ts";
import { batterBubble } from "./classicplus/deathBoil.ts";
import { pancakeMouth } from "./classicplus/devour.ts";
import { bannerFold } from "./classicplus/flagbearer.ts";
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
import { brushSeal } from "./meditative/aestheticizeTheGame.ts";
import { aiGirlfriend } from "./meditative/aiGirlfriend.ts";
import { alternateFate } from "./meditative/alternateFate.ts";
import { aluneth } from "./meditative/aluneth.ts";
import { jadeDie } from "./meditative/callToChaos.ts";
import { fleaMarket } from "./meditative/cnFleaMarket.ts";
import { cnGolem } from "./meditative/cnGolem.ts";
import { devinBot } from "./meditative/devinBot.ts";
import { doubleHeader } from "./meditative/doubleHeader.ts";
import { everGrowingTree } from "./meditative/everGrowingTree.ts";
import { blueprints } from "./meditative/evilBlueprints.ts";
import { fengShui } from "./meditative/fengShui.ts";
import { introspection } from "./meditative/introspection.ts";
import { jadeBeauty } from "./meditative/jadeBeauty.ts";
import { jlockeedHq } from "./meditative/jlockeedHq.ts";
import { keenusKeys } from "./meditative/keymasterKeenus.ts";
import { knowledgeBreaker } from "./meditative/knowledgeBreaker.ts";
import { breakerPrime } from "./meditative/knowledgeBreakerPrime.ts";
import { pareto } from "./meditative/paretoOptimality.ts";
import { prison } from "./meditative/prison.ts";
import { summitAscent } from "./meditative/reachTheSummit.ts";
import { redJade } from "./meditative/redJade.ts";
import { skullJnari } from "./meditative/skullOfJnari.ts";
import { taotie } from "./meditative/taotie.ts";
import { tatches } from "./meditative/tatchesTheTotem.ts";
import { touchedByKy } from "./meditative/touchedByKy.ts";
import { trueCraft } from "./meditative/trueCraftACard.ts";
import { yileGpt } from "./meditative/yileGpt.ts";
import { yilePrime } from "./meditative/yileGptPrime.ts";
import { yileVirus } from "./meditative/yilesVirus.ts";
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
  // R3405001: Meditative's signatures.
  introspection,
  summitAscent,
  doubleHeader,
  trueCraft,
  brushSeal,
  alternateFate,
  redJade,
  jadeBeauty,
  fengShui,
  fleaMarket,
  knowledgeBreaker,
  breakerPrime,
  taotie,
  yileGpt,
  yilePrime,
  yileVirus,
  aiGirlfriend,
  devinBot,
  skullJnari,
  keenusKeys,
  pareto,
  everGrowingTree,
  touchedByKy,
  aluneth,
  tatches,
  jadeDie,
  cnGolem,
  blueprints,
  prison,
  jlockeedHq,
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
