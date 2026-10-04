// The effects library (BUILD M3-T1): the one place a card script gets its verbs.
//
// A card script never touches state (CLAUDE.md rule 5); it composes `Effect[]` out of the factories
// below, and the resolver applies them. So this barrel is the whole vocabulary a card file may
// import — `import { summon, damage } from "../effects"` inside the engine, or from
// `@jackioh/engine/effects` in `packages/cards`. A verb that is not re-exported here does not exist
// as far as a card script is concerned, which is why every module in this directory is listed.
//
// Order follows the SPEC §6.3 verb table (Summon, Destroy, Sacrifice, Exile, … Swap), with the
// target vocabulary first because every other verb's arguments are written in it, and the two
// non-§6.3 modules (buff, memory) last. Each module is re-exported by name rather than with
// `export *`: the surface is then readable as the verb list it is, and a name that two modules
// come to export — or one that a module renames away — fails `pnpm typecheck` instead of silently
// vanishing from the barrel (an ESM ambiguous star export resolves to `undefined` at runtime).
//
// Collisions: as of this writing there are none. All 65 names below are distinct, so no module
// "wins" over another and nothing had to be dropped. Four names do shadow same-named helpers
// elsewhere in the engine, which is deliberate and not a conflict here, because the root
// `@jackioh/engine` index exposes this directory as a namespace (`export * as effects`):
//   - `addToHand`, `draw`   — the effect factories; `../draw` has the pipeline functions of the
//                             same names that these call into.
//   - `loseHealth`          — the effect factory; `../damage` has the hero-health helper it calls.
//   - `refreshMana`         — §6.3 Refresh's effect factory (R364); `../mana` has the start-of-turn
//                             refresh of the same name, which is a different rule.
//   - `counter`             — the §6.3 Counter verb, from `move.ts`. It is unrelated to
//                             `counters.ts`, whose verbs are `plague`, `clearPlague` and `lock`.
// Verbs that §6.3 lists but this directory does not implement live outside it and are not part of
// the card-script surface: Play/Cast (`../resolve`), Switch position as a PLAYER ACTION
// (`../combat`, which spends exertion; the effect form is `position.ts` per R20), and Tribute,
// Embiggen and Replace, which are play-validator or resolver concerns rather than effects.
// Forced attack, Cancel an attack, Rotate and Fuse were on that list and are not any more: §6.3
// lists them as verbs, and #9, #52, #60, #96 and #99 need them from a card hook, which has no
// `EngineSink` of its own. Each is a thin wrapper that never reimplements what it wraps.

// The target, player and board-scope vocabulary every verb below is written in (§6.3, §10.6).
// `BoardScope` is the second half of that vocabulary: `TargetSpec` names ONE card, because
// `resolveTarget` answers with one `DamageTarget | null`, so a verb that sweeps a whole board — and
// every card-facing `*All` verb below — is written in a scope instead (§3.1, §3.2, R13, R68).
export {
  adjacentTo,
  cardsInScope,
  instanceOf,
  matchesScope,
  playerOf,
  resolveTarget,
  sidesOf,
} from "./targets";
export type { BoardScope, PlayerSpec, TargetSpec } from "./targets";

// Summon, Recruit, and the copy and random-pool forms (§6.3, §5.1, §6.2, R21, R57, R64).
export { fillBoard, recruit, summon, summonCopy, summonRandom } from "./summon";
export type {
  RecruitFilter,
  StatsOverride,
  SummonArgs,
  SummonCopyArgs,
  SummonPlacement,
} from "./summon";

// Destroy, Sacrifice, and the board-wide and adjacent forms (§6.3, §4.5, §3.1, R46, R59).
export { destroy, destroyAdjacentTo, destroyAll, sacrifice } from "./destroy";

// Exile, Bounce, Discard, Counter, and the board-wide, whole-hand and by-cost forms
// (§6.3, §2.4, §3.1, R11, R12, R16, R26, R31, R66, R135).
export {
  EXILE_ZONE_ORDER,
  bounce,
  bounceAll,
  counter,
  discard,
  discardHand,
  discardRandom,
  exile,
  exileAdjacentTo,
  exileAll,
  exileHand,
  exileMatching,
} from "./move";
export type { CostFilter, ExileZone } from "./move";

// Steal (§6.3).
export { steal, stealAll } from "./steal";
export type { StealTarget } from "./steal";

// Transform, Vanilla (§6.3).
export { transform, vanilla } from "./transform";
export type { TransformTarget } from "./transform";

// Heal (§6.3, R19).
export { heal } from "./heal";
export type { HealArgs } from "./heal";

// Discover, Choose one and the target/hand prompts (§6.3, §10.6, R50, R81).
export {
  chooseFromHand,
  chooseMode,
  chooseTarget,
  chosenOptions,
  discoverFromCatalog,
  discoverFromGraveyard,
  discoverFromLibrary,
  targetsInScope,
} from "./choose";
export type { DiscoverOffer, LibraryFilter, TargetScope } from "./choose";

// Plague Token, Lock (§6.3, §3.2).
export { clearPlague, lock, plague } from "./counters";
export type { ZoneSpec } from "./counters";

// Mana, Refresh, next-turn mana (§6.3, §2.3, R364).
export { gainMana, nextTurnMana, refreshMana } from "./mana";

// Damage, for one target or a whole scope (§6.3, §4.4).
export { damage, damageAll } from "./damage";
export type { DamageAllArgs, DamageEffectArgs } from "./damage";

// Lose health (§6.3, R18).
export { loseHealth } from "./loseHealth";

// Draw, of the top card or of a card the script named out of a library (§6.3, §2.4, R4, R58, R135).
export { draw, drawFromLibrary } from "./draw";

// Add to hand: creates OR moves the card (§6.3, §2.4, R4, R57, R60, R65).
export { addRandomFromCatalog, addRandomFromGraveyard, addToHand } from "./addToHand";

// Exile out of a library, by random draw or from the bottom (§6.3, §3.2, R11, R60).
export { exileBottomOfLibrary, exileRandomFromLibrary } from "./library";

// Shuffle into a library (§6.3, R80).
export { shuffleCopiesOfSelf, shuffleInto } from "./shuffleInto";

// Make Radiant, by name, by random pick, or by a roll per card (§6.3, §5.2, §6.1, R60, R74).
export { radiantChance, setRadiant, setRadiantRandom } from "./radiant";
export type { RadiantTarget, RadiantZone } from "./radiant";

// Switch position as an effect, which spends no exertion (§6.3, R20).
export { switchAllPositions, switchPositionOf } from "./position";

// Cost (§6.3, R65, R77).
export { setCostMod, setCostOverride } from "./cost";

// Swap (§6.3, R73).
export { SWAP_ROWS, swap, swapBoard, swapHealth, swapLibrary } from "./swap";
export type { SwapWhat } from "./swap";

// Stat buffs and keyword grants: layer 4 of §10.4 rather than a §6.3 row of their own (R21, R78).
export { buff, buffAllUnits, grantKeyword, grantRandomKeywords } from "./buff";
export type { BuffAmount } from "./buff";

// Forced attack, Cancel an attack, and My Pawn's AI playout: thin Effect wrappers over `../combat`
// and `../subsystems/aiPolicy`, which a card hook cannot call itself for want of an `EngineSink`
// (§6.3, §4.2, §10.7, R44, R53, R84).
export {
  aiPlaysOutTurn,
  cancelAttack,
  forcedAttackOwnHero,
  forcedAttackRandom,
  forcedAttacks,
  forcedAttacksOn,
} from "./combat";
export type { ForcedAttackerFilter, ForcedSide, ForcedTarget } from "./combat";

// A delayed effect, resolved at its R62 point in creation order (§2.2, §10.6, R62, R68).
export { DELAYED_HOOK, THIS_TURN, delay } from "./delay";
export type { DelayAt } from "./delay";

// Player-scoped modifiers with their expiry (§2.2, §2.3, §6.3 Cost, R30, R48, R65).
export { addPlayerModifier } from "./playerMods";

// Coin flips (§6.3, §10.7): every flip goes through `ctx.rng`, never `Math.random`.
export { flipCoins } from "./coins";

// Fuse (§6.3, R77, R102) and Rotate (§6.3, §3.1, R14, R88): both wrap their subsystem whole.
export { fuseCards } from "./fuse";
export { rotate } from "./rotate";

// What a card remembers on its own instance (§10.1, R43).
export { remember, rememberRandom } from "./memory";

// One effect per card of a set read once off the board, which a pause resumes over whole (R113, R66).
export { forEachCard } from "./each";

// ---------------------------------------------------------------------------------------------
// Patch v0.2.0's verbs (docs/classic-sets.md B3, B5), by workstream.
// ---------------------------------------------------------------------------------------------

// ---- v0.2.0 verbs: instance data (Brittle, Degrade, Upgrade, KY's Constant's numbers, E38, E39) ----

// Cards anywhere a player keeps them — the field, a hand, a deck — for the verbs below (B3.3, B3.4,
// E38, E39), walked in R242's order, public cards first.
export { cardsInCardScope, matchesCardScope, readersOf, unreadableBy } from "./cardScope";
export type { CardScope, CardZone, Readers, ScopedCard } from "./cardScope";

// Degrade and Upgrade (B3.4, R386, R440, R442), an Upgrade of a named number of the card's own (Core #98's
// Steady Shot, R656), and KY's Constant's number set outright (Classic+ #41).
export {
  NUMBER_CARD_KEY,
  applicableChanges,
  chosenTuningNumber,
  degrade,
  discoverNumber,
  reachedCards,
  setNumber,
  tuneOnce,
  upgrade,
  upgradeOwnNumber,
} from "./tune";
export type { TuneArgs, TuneDirection, TuneRow } from "./tune";

// Brittle X: give (set) and gain (add) a count (B3.3, R385, R441).
export { gainBrittle, giveBrittle } from "./brittle";
export type { BrittleTarget } from "./brittle";

// Enchantments that ride a card through every zone (E39).
export { enchant } from "./enchant";

// Buffs and granted keywords that reach hands and decks and ride onto the field (E38).
export { buffCards, grantKeywordCards } from "./buff";

// ---- v0.2.0 verbs: field (Animate, Lock variants, Unlock, Flicker) ----
// Animate (B3.1, R383, R445): an Animated Trap's firing ends with it.
export { animate } from "./animate";
export type { AnimateArgs } from "./animate";
// Flicker (B5 E22, R444), for one card or a board scope; `flickerCard` for a sequence holding a sink.
export { flicker, flickerCard } from "./flicker";
// The Lock variants and Unlock (B5 E20; §3.2 Lock). The single-zone `unlock` sits with `lock`.
export { unlock } from "./counters";
export { lockLane, lockOwnZone, lockPlayedZone, lockRandomZone, unlockAll } from "./locks";
export type { LaneSpec, ZoneScope } from "./locks";

// ---- v0.2.0 verbs: play pipeline (Counter, steal off the stack, casts, cost rules) ----

// play pipeline B: casts from anywhere and random casts (E12; R452, R453), and the price rules and
// next-Spell rider a card puts on a player (E15, E39; R455).
export { addCostRule, cast, castEach, castNew, castRandom, enchantNextSpell } from "./cast";
export type { CastDef, CastHow, CostRuleSpan } from "./cast";

// B5 E1, E2, R448: Counter the play an announce window answers — to its owner's graveyard, to exile,
// or to the countering player's hand as theirs ("thief"). `counter` above is its §6.3 name.
export { counterPlay } from "./move";
export type { CounterDestination } from "./move";

// ---- v0.2.0 verbs: activate and turn (end the turn, delayed kinds, rest of the game) ----

// B5 E10, R456: end the turn from an effect, now or after N more actions.
export { endTurn, endTurnAfterActions } from "./turnEnd";
// B5 E27, E28, R458: a destroy at the start of your next turn (a unit, or a scope read then), your hand
// discarded at the end of this or your next turn, and a start-of-turn effect for the rest of the game.
export { destroyAtNextTurnStart, discardHandAtTurnEnd, forRestOfGame } from "./delay";

// ---- v0.2.0 verbs: damage and combat (set health, redirect, split damage, statuses) ----
// Set health (E7) and heals turned into Pierce damage (E8). The replacements themselves (E5, E9) are
// declared as data (`Script.replacements`), and the forced attacks on a random enemy and on the unit's
// own hero (E35) are `./combat`'s, exported above with the other forced attacks.
export { convertHealing, setHealth } from "./health";
// Random split damage (E37).
export { damageSplit } from "./split";
// Berserk and "may attack again" (E35).
export { goBerserk, mayAttackAgain } from "./statuses";
// R42, R412: kills an effect watches, and a kill credited to another unit (Classic+ #19.2).
export { withKillCredit } from "./killCredit";

// ---- v0.2.0 verbs: prompts and generation (trigger a Cry, piles, prompts, plague, fuse, transform, recruit) ----
// Prompts and movement: the new prompt kinds and the opponent's hand as options (E17, E18, R465),
// cards between the players' piles (E2, E16, R466), trigger a Cry (E13, R467), summon this out of a
// hand or a deck (E26).
export {
  ANSWER_OPTION_IDS,
  answeredCorrectly,
  chooseAnswer,
  chooseCell,
  chooseCostInHand,
  chooseNumber,
  choosePick,
  chooseReward,
  chosenCells,
  chosenNumber,
  matchesLibraryFilter,
} from "./choose";
export type { CellScope, PickFilter, PileSpec } from "./choose";
export { drawFromOpponent, giveFromHand, takeFromLibrary } from "./give";
export type { TakenRiders } from "./give";
export { hasTriggerableCry, triggerCry } from "./cry";
export { summonThis } from "./summonThis";

// generation, below A6a's prompt verbs — Plague placements and removal (B5 E19, R471):
export {
  PLAGUE_PLACEMENT_HOOK,
  consumePlague,
  placePlague,
  placePlagueEach,
  placePlagueRandom,
  placePlagueTokens,
} from "./plague";
// the Fuse variants (B5 E23, R468–R470):
export { FUSE_ONTO_HOOK, fuseGenerated, fuseOntoYourCard, fuseRandomInto } from "./fuse";
export type { FuseInto, FuseOntoPile } from "./fuse";
// the Transform variants (B5 E24):
export { transformBeneath, transformRandom } from "./transform";
// the Recruit extensions (B5 E25; `recruit` above gained `from`, `whose` and `count`):
export { recruitAll } from "./summon";
export type { RecruitSource } from "./summon";

// ---- v0.2.0 verbs: Core patches (R426–R437) ----

// ---- v0.2.0 verbs: Classic cards #1–#45 (card-specific, the cards-classic-a workstream) ----

// §6.3 Exile at random out of a hand (C #15 Nose Hunter's Radiant face, R60).
export { exileRandomFromHand } from "./handExile";

// §6.3 Shuffle an existing card into its owner's library, R80's cap leaving a graveyard card where it
// is (C #30 Recycle, R316).
export { shuffleCardInto } from "./shuffleCard";

// A target prompt narrowed by a card's own condition (C #32 Felinor Feelings' Radiant face, §10.6).
export { chooseTargetWhere } from "./chooseWhere";

// §4.5's check at this point of a list, then the rest on a stay that begins after it (C #43 Plague
// Nuke, R59, R113, R174).
export { afterStateCheck } from "./afterCheck";

// ---- v0.2.0 verbs: Classic+ cards #40–#78 and the AI cards (card-specific, the cards-plus-d workstream) ----

// Classic+ #40–#45, #77, T-AI-1 (KY's Test's question bank, E31, and its neighbours):
// Random catalog cards shuffled into a library, Radiant and enchanted (C+ #40 Appropriations' Education, E39):
export { shuffleRandomFromCatalog } from "./shuffleRandom";

// Classic+ #62 KY's Papaya's curve (E32), the Degrade and Upgrade cards, T-AI-2, T-AI-3, T-AI-10:
// T-AI-3 Hallucination: copies of random deck cards into the caster's hand, given Brittle (R57, R60, R385).
export { addLibraryCopies } from "./libraryCopies";

// Classic+ #73 Call to Chaos (Classic+ Edition), #73.1, #74, #78, T-AI-4 to T-AI-9:
// a draw repeated while the card it brought is cheap (T-AI-4 Chain of Thought, R596), and a sweep of Field
// Spells that hits the heroes once per Field Spell it dooms (T-AI-6 Datacenter Fire, R408's count).
export { destroyFieldSpellsAndHit, drawWhileCheap, fieldSpellsDoomed } from "./datacenter";
export type { FieldSpellSide } from "./datacenter";

// Classic+ #46–#61:
// Armor a hero keeps for the game (C+ #46) or until its next turn (Core #98's Armor Up, R651), and a
// random hand card made cheaper (C+ #49).
export { discountRandomInHand, gainHeroArmor, gainHeroArmorUntilNextTurn } from "./perks";

// Classic+ #63–#67, #75, #76 and their tokens:
// the Grapes a Grape card rolls (GRAPE_ODDS, R382), a hit on an enemy or a heal on a friend, a draw whose
// card takes a price, and a hand replaced card for card (C+ #65, #65.2, #65.3, #65.5, #66).
export {
  addRolledGrapes,
  cardThisDrawPutInHand,
  damageEnemyOrHealFriend,
  drawPriced,
  replaceHandWithRandom,
  rollGrape,
} from "./fruit";

// ---- v0.2.0 verbs: Classic cards #46–#90 (card-specific, the cards-classic-b workstream) ----

// R60's random picks of existing cards: a random Unit of yours buffed, N random graveyard cards to
// your hand (C #90 In Too Deep's rewards E and C).
export { buffRandomUnit, returnRandomFromGraveyard } from "./randomPicks";

// "Draw until …": one draw at a time while a condition holds, stopping at a draw that adds no card
// (C #46 Divine Favor, §2.4, R58).
export { drawWhile } from "./drawWhile";

// ---- v0.2.0 verbs: cards-plus-c (B5 E30, R417): C+ #29's verbs over a last board ----
export {
  LAST_BOARD_CARD_COST,
  LAST_BOARD_DISCOVER_OPTIONS,
  addFromLastBoard,
  addRandomFromLastBoard,
  discoverFromLastBoard,
} from "./lastBoard";
// ---- v0.2.0 verbs: Classic+ cards #1–#39 (card-specific, the cards-plus-c workstream) ----

// B5 E34, R416: R29's scorer choosing a whole hand (C+ #27 Zephrys Zealotism).
export { replaceHandWithPerfect } from "../subsystems/perfectHand";
// R59: rounds of damage, each with its own state check, until a Unit dies (C+ #32.3 Blade Storm).
export { damageRoundsUntilDeath } from "./rounds";
// B5 E29, R419: return the board, or one side of it, to a snapshot of the last few turns (C+ #35 Rollback).
export { rollBack } from "../subsystems/boardHistory";
