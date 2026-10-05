// SPEC §10.5 "Playing a card": the eight steps, in order, as named steps that other code can run
// and that a prompt can pause in the middle of (BUILD M3-T2/T3, R81, R90).
//
// The pipeline is one table of named steps (`PLAY_STEPS`) driven by `drive`, and every step reads
// and writes the same plain record — a `PlayRun`, which is JSON. That is what makes the sequence
// resumable: when a step leaves a prompt open, the rest of the pipeline is owed to `state.work` as
// a `Resume` naming the step to pick up at plus that record (§9.3 "mid-action choices are state,
// not callbacks"), and the resolution loop drains it after the answer. Nothing is ever held across
// a prompt in a local variable or a closure, which is the bug `work.ts` exists to prevent.
//
// So a Cry that opens a prompt resumes into the *remaining* steps rather than restarting: the
// answer re-enters the card's own step, `settle` then runs what the pipeline owed, and steps 6, 7
// and 8 finish the play. Echo (R30) is the same machinery pointed at itself — step 6 is re-entrant,
// takes one repeat at a time off `state.echoQueue` and re-runs step 5 with *fresh* prompts, so a
// prompt inside one repeat leaves the rest waiting in state (§6.1 "Echo X").
//
// Three steps keep their own cursor inside the record, so a pause inside one continues where it
// stopped instead of at the top: step 3 over the `onPlayHook` holders, step 5 over its named parts,
// step 6 over the echo queue and the repeat's fresh picks.
//
// Patch v0.2.0 adds a step between 3 and 4, the announce (B5 E1, R448): the paid-for card moves into
// its player's resolving zone, `cardAnnounced` goes out, and a window runs in which Counters answer
// it — traps first, then the other triggers the announce woke (§10.3). A countered play stops there:
// step 4 never places it, so it is never counted, and step 8 settles. Step 1 is the targeting point
// of the play's declared targets (B5 E5, E9, R450): an interception (Classic #33) moves a pick, and
// step 2 pays the discards a costly target carries (Classic #89). Step 3 may replace the card being
// played (Classic #23 Devil's Pact, R449), and step 4 counts what the play leaves for E4 (R451).
//
// What is *not* here: which choices are legal (that is `playChoices.ts`, R90), what a cost is
// (`mana.ts`, R65), what a card's text does (the card's script, which is a pure builder the engine
// applies — CLAUDE.md rule 5) and R43's power mechanics (`subsystems/heroPower.ts`).

import type { ActionBody, PlayerId, Selection, TargetDecl } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { animateOnEntry } from "./animated";
import { defOf } from "./catalog";
import { MIN_CHOSEN_X, QUICKSTRIKER_COMBO_MULTIPLE } from "./config";
import { whyPlayBanned } from "./costRules";
import { dealDamage } from "./damage";
import { addToHand, draw } from "./draw";
import {
  EXILE_ON_LANDING,
  dropEchoRepeats,
  echoRepeatsOwed,
  exileOnLanding,
  landAfterResolution,
  queueEchoRepeats,
  takeEchoRepeat,
} from "./echo";
import { beginAnnounce, endAnnounce, isAnnounceLive } from "./announce";
import { CAST_ON_DRAW_KEY, releaseDraw } from "./drawComplete";
import { hasEnchantment } from "./enchantments";
import { cardTypeOf } from "./faces";
import { inOwnGraveyard, manaDue, spendPlagueTokens, whyGraveyardPlayRefused } from "./graveyardPlay";
import { costRulesSpentBy, isXCost, manaEvent, modifierIsLive, playCost, spendMana } from "./mana";
import { removeModifier } from "./modifiers";
import {
  DECLARATION_SLICES_KEY,
  activeTargetDecls,
  choosesX,
  declarationSlices,
  declaredModes,
  declaredTargets,
  defaultZoneFor,
  inDeclaredOrder,
  interceptorFitsDecl,
  legalSelectionsFor,
  playMadeRadiant,
  playsOnStack,
  resolvingFace,
  targetingDeclsOf,
  targetsFollowModes,
  whyChoicesRefused,
} from "./playChoices";
import { recordPlay } from "./playCounts";
import {
  closePrompt,
  inOfferedOrder,
  openPrompt,
  registerPromptAnswerer,
  resumeOf,
  runHookResumable,
  whyAnswerRefused,
  type AnswerInput,
} from "./prompts";
import { countChainCast, preferEnemies, preferFriends, randomCastOf, randomPicks, withCastMode } from "./randomCast";
import { targetAim } from "./targeting";
import {
  MANA_BEFORE_PLAY_KEY,
  flagReturnToHandAtEndOfTurn,
  registerCastDriver,
  type CastOptions,
  type EngineSink,
} from "./resolve";
import {
  findInstance,
  newInstance,
  type CardInstance,
  type CastMode,
  type GameState,
  type PlayRecord,
  type Resume,
  type WorkItem,
} from "./state";
import { playedEarlier } from "./query";
import { flagsOf } from "./scripts";
import { countPlay } from "./timesPlayed";
import { sacrificeTogether, stateCheck } from "./stateCheck";
import { OWED_TO_TRAPS, SETTLE_PASS_CAP, dispatchPending, eventOfQueued, runQueuedTrigger, settle } from "./triggers";
import { triggerHolderFor, triggerHoldersWithHook, type TriggerHolder } from "./triggers";
import {
  interceptTargeting,
  payTargetingDiscards,
  registerTargetedContinuation,
  targetAnswer,
  whyTargetAnswerRefused,
} from "./targetingPoint";
import { exitMark, leftFieldAfter } from "./stays";
import { copiedTextOf, copiesText, fixCopiedText, textFaceOf } from "./subsystems/copiedText";
import { beginWorkCascade, drainWork, dropWork, paused, pausedOf, pushWork, registerWorkHandler } from "./work";
import {
  cardAt,
  ceaseToExist,
  firstFreeZone,
  freshFaceDownId,
  isOpen,
  landsFaceDown,
  placeOnField,
  releaseZone,
  removeFromAnyZone,
  reserveZone,
  slotsOf,
  type ZoneSlot,
} from "./zones";

export type PlayAction = Extract<ActionBody, { type: "play" }>;

/** The eight steps of §10.5, in the order that section lists them, and E1's announce between 3 and 4. */
export const PLAY_STEPS = [
  "validate",
  "pay",
  "giftedHook",
  // R70, R452: a cast's choices, made once step 3 has settled the face they answer (R214) and before
  // the card moves — so an announce that follows (E1, between steps 3 and 4) names them. A play made
  // its choices at step 1, and this step does nothing for it.
  "castChoices",
  "announce",
  "place",
  "resolve",
  "echo",
  "finish",
  "settle",
] as const;

export type PlayStepName = (typeof PLAY_STEPS)[number];

/** The named parts of step 5, so a pause inside one continues at the next, never at the top. */
export const RESOLVE_PARTS = ["quickstriker", "comboDraw", "script"] as const;

export type ResolvePart = (typeof RESOLVE_PARTS)[number];

/** `resume.hook` for an owed play pipeline: the sequence name `work.ts` documents (`"play"`). */
export const PLAY_WORK_KIND = "play";

/** Where the run record sits inside `resume.data`, so the rest of `data` stays the card's own. */
const RUN_KEY = "__play";

/**
 * A play part-way through its steps. All JSON: it is stored inside an owed `WorkItem`'s
 * `resume.data` and inside the `resume` of a prompt the pipeline opened itself, so a paused play
 * survives `JSON.parse(JSON.stringify(state))` and a replay resumes it exactly (§9.3, §10.1).
 */
export type PlayRun = {
  instanceId: string;
  defId: string;
  player: PlayerId;
  /** The mana actually paid, which `cardPlayed` reports and Gifted Program reads (§8 #64). */
  costPaid: number;
  /** Where a permanent goes; null for a Spell, which resolves instead (§10.5 step 4). */
  zone: ZoneSlot | null;
  targets: Selection[];
  modes: string[];
  tributes: string[];
  /** Index into `PLAY_STEPS` of the step to run next. */
  at: number;
  /** Step 3's cursor: how many `onPlayHook` holders have run. */
  hookAt: number;
  /**
   * Step 3's holders, by id, in R68's order as the step began. The cursor indexes this list, never a
   * fresh read of the board, which a hook's own answer can reshape (a hook that bounced its own card
   * dropped the next one, R113).
   */
  hookIds?: string[];
  /** Step 5's cursor into `RESOLVE_PARTS`. */
  resolveAt: number;
  /** Step 6: whether this play has worked out how many repeats it owes yet. */
  echoQueued: boolean;
  /**
   * Step 6: the repeat being re-resolved, with the fresh answers collected so far, and `partAt`,
   * its cursor into `RESOLVE_PARTS` once the answers are in — a repeat is step 5 again, granted
   * Combo parts included, and #78's Combo draw can pause it before its script runs. `exitsFrom` is
   * the field's departures once the answers were in (R174): the repeat's script is aimed at the
   * stays its fresh picks were made on, so a target the repeat's own Combo draw killed is gone for
   * it, Reborn body or not, as a play's declared target is gone for its Cry (R81, R83).
   */
  repeat: null | {
    targets: Selection[];
    modes: string[];
    declAt: number;
    modeAt: number;
    partAt?: number;
    exitsFrom?: number;
  };
  /** Set while a prompt this pipeline opened is waiting; says which bucket the answer fills. */
  awaiting: null | "echoTarget" | "echoMode" | "castX";
  /**
   * R70, R81: a cast's own choices are made. A play carries its targets and modes in the action, and
   * a cast has none, so step 4 asks the caster for them before it places the card (R90: a play's
   * choices are made with the card still in hand), as prompts, into `repeat` (the same record an
   * Echo repeat's fresh picks fill), and sets this once they are in.
   */
  castChosen?: boolean;
  /**
   * R70: a cast rather than a play from hand — the same steps, entered at step 3 for 0, placed by
   * R64 at step 4 wherever the card is, and leaving the resolution loop to the effect that cast it.
   */
  cast?: boolean;
  /** The face the card was played with, set at step 4 for `cardResolved` (R34, R57). */
  radiant?: boolean;
  /**
   * R214: whether Gifted Program makes this play Radiant, as step 1 read it to know the face the
   * play's choices answer. Step 3 applies this answer rather than asking the board again, which a
   * Tribute's Death at step 2 can have changed (#22's copies of the Gifted Program it ate). A cast has
   * no step 1 (R70), so its step 3 reads the board.
   */
  gifted?: boolean;
  /**
   * Step 4 has placed the card and announced the play; what is left of it is its resolution loop,
   * which a trap's question can pause (§10.3, R17), so the step is re-entered at that loop.
   */
  placed?: boolean;
  /**
   * R90, R102: how many of `targets` each declaration took when step 1 read the play, which a fused
   * card's Cry splits its ingredients' choices by (`playChoices.DECLARATION_SLICES_KEY`).
   */
  targetSlices?: number[];
  /**
   * R174: the field's departures when the play's choices were checked — at step 1 for a play, and
   * for a cast once its caster has made them (R70) — so a declared target a Tribute (step 2) or a
   * trap answering the play (step 4) took off the field is gone for step 5, even back through Reborn.
   */
  exitsFrom?: number;
  /** R174: the field's departures when step 3 read its holders (`hookIds`), whose stays it runs. */
  hooksFrom?: number;
  /**
   * R174: the field's departures once step 4 had put the card on the field. The play follows that
   * stay and no other: a card that has left it since — a trap at step 4 killed it, its own Cry did —
   * is no longer the card being played even when Reborn has put a new body in its zone (R83), so
   * that body has no Cry to resolve (R1, R118) and is not in play for step 7 (R61).
   */
  placedFrom?: number;
  /**
   * R119: every card acting on the field as the play began — at step 1 for a play, as the cast began
   * for a cast (R70) — by id, with the field's departures then (`standingFrom`). A permanent that
   * arrives on the field after that, whatever puts it there — a tributed unit's Death at step 2
   * (#22's copies, R210), a Cry recruiting it, summoning it (#95) or bringing a body back
   * through Reborn, a trap answering the play summoning it — does not answer the play, as the played
   * card itself does not: not its `cardPlayed` and `summoned` at step 4, not step 5's granted Combo
   * (#38) on the first resolution or an Echo repeat, and not its `cardResolved` at step 7
   * (`arrivedDuring`). Nor does a card that lay dormant under a Stack pile as the play began and
   * resumed as its top while the play resolved: it registered nothing then (§3.2, R153).
   */
  standing?: string[];
  standingFrom?: number;
  /**
   * R119: the playing player's modifiers as the play began, by id. A granted Combo that is a
   * modifier (#78's `comboDraw`, a `quickstrikerDamage`) answers the play only when it was already
   * in place then, so one the play itself installed — /fullsend's own rider, met again by the Echo
   * repeat of the same play (§10.5 step 6) — does not.
   */
  modsBefore?: string[];
  /**
   * R360: the Tribute step 2 paid took at least one of the opponent's units (#55's "if opposing
   * Units are used"), read as it was paid, before those units died. Step 4 reads it against the
   * face the card is placed with, since only #55's base face summons itself for the opponent.
   */
  enemyTributed?: boolean;
  /**
   * R226, §10.5 step 4, §10.1: the card left its owner's hand before step 4 could move it — a
   * Tribute's Death at step 2, or an `onPlayHook` at step 3, had it discarded — or, for a cast, left
   * the resolving zone it waits in (R70) — so it is not played: no placement, no `cardPlayed`, no
   * resolution. What steps 2 and 3 did stands, and step 8 settles it.
   */
  lost?: boolean;
  // ---- play pipeline B (E11, E12; R452–R454) ----
  /**
   * E11, R454: the card is played from its player's graveyard under a permission (`graveyardPlay.ts`)
   * rather than from the hand. Absent for a play from hand and for a cast.
   */
  source?: "graveyard";
  /** E11, R454: the Plague Tokens paying part of the price (`costPaid` is the whole price). */
  plague?: { from: string; tokens: number };
  /** E12, R452: a random cast — every choice its caster would make is made at random. */
  random?: boolean;
  /** E12, E39, R452: its target picks narrow to enemies when one is legal. */
  targetEnemies?: boolean;
  /**
   * E12, R453: a cast Field Spell, Trap or Field Trap that found no zone. It still counts as played
   * (R70, R138), nothing of its text resolves, and step 7 puts it in its owner's graveyard.
   */
  fizzled?: boolean;
  /**
   * The player's current mana as the play began — at step 1, before step 2 pays — or as a cast began,
   * which the card's Cry reads as `ctx.manaBeforePlay` (Classic #22 Mid Runner).
   */
  manaBefore?: number;
  /** R453: the X its caster chose for a cast X card (the `number` prompt's answer), until it is set. */
  castX?: number;
  // ---- v0.2.0 play pipeline A (E1, E5, Devil's Pact) ----
  /**
   * B5 E5, R450: the cards the play's declared targets cost to target (Classic #89), carried in the
   * action and discarded at step 2 with the rest of the price.
   */
  discards?: string[];
  /**
   * B5 E1, R448: the announce has gone out and the card waits in the resolving zone; what is left of
   * the step is its window, which a response's question can pause, so the step is re-entered there.
   */
  announced?: boolean;
  /** R448: a Counter cancelled the play in its announce window: nothing after it happens but step 8. */
  countered?: boolean;
  /**
   * R449: step 3 replaced the card played by a new card (Classic #23 Devil's Pact). The new card makes
   * its choices as a cast does (R70), before the announce, and takes its zone at step 4 by R64.
   */
  replaced?: boolean;
  /**
   * R58: the id the card was drawn under, for a cast-on-draw cast. The draw is complete once this
   * cast has resolved, and its held `drawn` is released then (`drawComplete.ts`).
   */
  drawnAs?: string;
  // ---- B5 E14 (Classic #57 Echo; R399, R546) ----
  /**
   * The Spell text a copier's play resolves, fixed at step 1 as its choices are checked against it
   * (null: nothing had been played), and written on the card as the announce moves it into the
   * resolving zone (`subsystems/copiedText.fixCopiedText`). Absent for every other card.
   */
  copied?: PlayRecord | null;
};

// ---------------------------------------------------------------------------
// The owed record
// ---------------------------------------------------------------------------

function resumeFor(run: PlayRun, at: number): Resume {
  return {
    defId: run.defId,
    hook: PLAY_WORK_KIND,
    step: PLAY_STEPS[at] ?? "settle",
    radiant: false,
    instanceId: run.instanceId,
    data: { [RUN_KEY]: { ...run, at } },
  };
}

/** True when this continuation is an owed play pipeline rather than a card's own step. */
export function isPlayResume(resume: { hook: string }): boolean {
  return resume.hook === PLAY_WORK_KIND;
}

/** The run record a continuation carries, or null when it is not one of ours. */
export function runOf(resume: { data: Record<string, unknown> }): PlayRun | null {
  const raw = resume.data[RUN_KEY];
  if (raw === null || typeof raw !== "object") return null;
  const run = raw as Partial<PlayRun>;
  if (typeof run.instanceId !== "string" || typeof run.at !== "number") return null;
  return raw as PlayRun;
}

// ---------------------------------------------------------------------------
// Step 1 — validate (§10.5 step 1)
// ---------------------------------------------------------------------------

function handCard(state: GameState, player: PlayerId, instanceId: string): CardInstance | undefined {
  return state.players[player].hand.find((card) => card.id === instanceId);
}

/**
 * E11, R454: where a play takes its card from — the player's hand, or their own graveyard, where a
 * permission may let them play it (`graveyardPlay.ts`). Undefined when the card is in neither.
 */
function playSourceOf(
  state: GameState,
  player: PlayerId,
  instanceId: string,
): { card: CardInstance; from: "hand" | "graveyard" } | undefined {
  const inHand = handCard(state, player, instanceId);
  if (inHand !== undefined) return { card: inHand, from: "hand" };
  const inGraveyard = state.players[player].graveyard.find((card) => card.id === instanceId);
  return inGraveyard !== undefined && inOwnGraveyard(state, player, inGraveyard)
    ? { card: inGraveyard, from: "graveyard" }
    : undefined;
}

export function isPermanent(state: GameState, card: CardInstance): boolean {
  return cardTypeOf(state, card) !== "Spell";
}

/**
 * §10.5 step 1: "legal zone, cost ≤ current mana after all modifiers, Tribute available, X or
 * embiggen chosen, targets legal — every choice the play carried is checked against what the card
 * declared and what the board allows (R90)". Every one of those checks is `playChoices.ts`'s, which
 * is why this asks once and restates nothing; what is left here is the cost, which `mana.ts` owns,
 * and the default zone a play that named none takes (R64's leftmost free zone).
 *
 * X and embiggen are stamped on the instance before the cost is read, because that is where R65's
 * calculation looks for them. `reduce` clones the state and throws the clone away on a refusal, so
 * a refused play leaves nothing behind (§9.3).
 */
export function validatePlay(
  sink: EngineSink,
  player: PlayerId,
  action: PlayAction,
): { error: string } | { run: PlayRun } {
  const state = sink.state;
  const source = playSourceOf(state, player, action.instanceId);
  if (source === undefined) return { error: `no card ${action.instanceId} in ${player}'s hand` };
  const card = source.card;
  // R454: Plague Tokens pay only for a play from the graveyard a permission lets them pay for.
  if (source.from === "hand" && action.plague !== undefined) {
    return { error: "only a play from your graveyard can spend Plague Tokens" };
  }

  const refused = whyChoicesRefused(state, player, card, action);
  if (refused !== null) return { error: refused };

  if (choosesX(state, card)) card.x = action.x ?? 0;
  if (typeof defOf(state, card.defId).cost === "object") card.embiggened = action.embiggen === true;

  // R65, R454, R455: the price of this play, the player's prices included wherever the card lies.
  const cost = playCost(state, card);
  const banned = whyPlayBanned(state, player, card, cost);
  if (banned !== null) return { error: banned };
  if (source.from === "graveyard") {
    const why = whyGraveyardPlayRefused(state, player, card, cost, action.plague);
    if (why !== null) return { error: why };
  } else if (cost > state.players[player].mana.current) {
    return { error: `${defOf(state, card.defId).name} costs ${cost}, more than your mana` };
  }

  // Step 1 accepted the zone; a play that named none takes the leftmost free one — or, on a full
  // row, the leftmost one its own Tribute empties (R391).
  let zone: ZoneSlot | null = null;
  if (isPermanent(state, card)) {
    const named = action.zone;
    zone =
      named === undefined
        ? defaultZoneFor(state, player, card, action.tributes ?? [])
        : { player, row: named.row, lane: named.lane };
    if (zone === null) return { error: "no free zone" };
  }

  const modes = [...(action.modes ?? [])];
  // R221, R90: each declaration's picks are a set, taken in the order it offers them, so a listing
  // `legalActions` never offers resolves as the offered one does. The face is step 5's (R214).
  const targets = inDeclaredOrder(state, player, resolvingFace(state, player, card, cost), action.targets ?? [], modes);
  return {
    run: {
      instanceId: card.id,
      defId: card.defId,
      player,
      costPaid: cost,
      zone,
      targets,
      modes,
      tributes: [...(action.tributes ?? [])],
      at: 1,
      hookAt: 0,
      resolveAt: 0,
      echoQueued: false,
      repeat: null,
      awaiting: null,
      gifted: playMadeRadiant(state, player, card, cost),
      ...slicesFor(state, player, card, cost, targets, modes),
      ...((action.discards ?? []).length === 0 ? {} : { discards: [...(action.discards ?? [])] }),
      exitsFrom: exitMark(state),
      ...playBegins(state, player),
      manaBefore: state.players[player].mana.current,
      // B5 E14, R546: a copier's copy is fixed as its play is checked, the choices with it.
      ...(copiesText(card) ? { copied: copiedTextOf(state, card) } : {}),
      ...(source.from === "graveyard" ? { source: "graveyard" as const } : {}),
      ...(action.plague === undefined ? {} : { plague: { from: action.plague.from, tokens: action.plague.tokens } }),
    },
  };
}

/** R119: the board and the player's modifiers as a play or a cast begins, which it answers against. */
function playBegins(state: GameState, player: PlayerId): Pick<PlayRun, "standing" | "standingFrom" | "modsBefore"> {
  return {
    standing: fieldCardIds(state),
    standingFrom: exitMark(state),
    modsBefore: state.players[player].mods.map((mod) => mod.id),
  };
}

/**
 * R90, R102: the split step 1 just checked, kept for a fused card, whose Cry hands each ingredient
 * its own slice (`subsystems/fuse.ts`) and must hand it the slice the play was checked with. The
 * face is the one step 5 will resolve (R214). Nothing is kept for any other card, which reads the
 * flat list itself.
 */
function slicesFor(
  state: GameState,
  player: PlayerId,
  card: CardInstance,
  cost: number,
  targets: readonly Selection[],
  modes: readonly string[],
): { targetSlices?: number[] } {
  if (state.transientDefs[card.defId] === undefined) return {};
  const face = resolvingFace(state, player, card, cost);
  return { targetSlices: declarationSlices(state, player, face, targets, modes) };
}

// ---------------------------------------------------------------------------
// Step 2 — pay (§10.5 step 2)
// ---------------------------------------------------------------------------

/**
 * §6.3 Tribute: sacrifice the units this play named. How many they must be worth, whether an enemy
 * unit counts (#55) and that a Sheep Token counts 2 were all settled in step 1, so this is the
 * payment and nothing else — which is why it sacrifices what it was given rather than judging it
 * again. The set is one payment and dies together, its Death hooks in R68's order whatever order the
 * play listed it in (`stateCheck.sacrificeTogether`), so `legalActions`, which offers each set once,
 * and `reduce`, which accepts any listing of it, mean the same play.
 */
function payTributes(sink: EngineSink, run: PlayRun): void {
  if (run.tributes.length === 0) return;
  const units = run.tributes.flatMap((id) => {
    const unit = findInstance(sink.state, id);
    return unit === undefined || unit.zone.z !== "field" ? [] : [unit];
  });
  sacrificeTogether(sink, units);
}

/** R360: whether the Tribute this play pays takes a unit the opponent controls, read before it dies. */
function tributesAnEnemy(state: GameState, run: PlayRun): boolean {
  return run.tributes.some((id) => {
    const unit = findInstance(state, id);
    return unit !== undefined && unit.zone.z === "field" && unit.controller !== run.player;
  });
}

/**
 * R360: where #55's base face lands when its Tribute took an opposing unit — "summon for your
 * opponent". The opponent's zone in the lane the player named when it is open, else the leftmost
 * open zone of their unit row, which is R15's placement for a card changing sides. With no open zone
 * there it is placed where the player named, on their own side, as a steal that finds no zone leaves
 * the card where it is (R15).
 */
function handedOverZone(state: GameState, run: PlayRun, card: CardInstance): ZoneSlot | null {
  if (run.enemyTributed !== true || run.zone === null || run.zone.row !== "units") return null;
  if (flagsOf(card).enemyTributeHandsOver !== true) return null;
  const opponent = opponentOf(run.player);
  const sameLane: ZoneSlot = { player: opponent, row: "units", lane: run.zone.lane };
  return isOpen(state, sameLane) ? sameLane : firstFreeZone(state, opponent, "units");
}

/**
 * §10.5 step 2: "consume the next-spell discount if used". A one-shot discount (Lunar Eclipse) is
 * `{ until: "used" }`, so the play it applied to spends it; an X-cost card ignores discounts and so
 * spends none (R65), and Professor Curvature's is not a one-shot but a next-turn modifier (R48).
 */
function consumeUsedDiscounts(sink: EngineSink, run: PlayRun, card: CardInstance): void {
  const state = sink.state;
  if (isXCost(state, card)) return;
  const type = cardTypeOf(state, card);
  for (const mod of [...state.players[run.player].mods]) {
    if (mod.kind !== "costDiscount") continue;
    // §2.2 names the Lunar Eclipse discount as both "consumed on use" AND expired at cleanup, so
    // it cannot be expressed by the expiry alone: `{ until: "used" }` survives cleanup (that is
    // what keeps #79 Twinspell's `echoNextSpell` alive, R30), while `{ until: "thisTurn" }` is
    // never consumed here. `oncePerTurn` is the flag for exactly that pair — the card keeps the
    // "this turn" expiry and this consumes it on the first matching play (§8 row 35).
    if (mod.expiry.until !== "used" && mod.oncePerTurn !== true) continue;
    if (!modifierIsLive(state, mod)) continue;
    if (mod.onlyType !== undefined && mod.onlyType !== type) continue;
    removeModifier(sink, run.player, mod.id);
  }
}

function payStep(sink: EngineSink, run: PlayRun): void {
  const card = findInstance(sink.state, run.instanceId);
  if (card === undefined) return;
  const side = sink.state.players[run.player];
  // R455: the "until used" price rules this play's price met, read before a Tribute can change the
  // board they are read against.
  const spentRules = costRulesSpentBy(sink.state, card);

  // R454: Plague Tokens pay their part of the price, and the mana the rest.
  spendMana(side, manaDue(run.costPaid, run.plague));
  sink.events.push(manaEvent(run.player, side));
  if (run.plague !== undefined) {
    const holder = findInstance(sink.state, run.plague.from);
    if (holder !== undefined) spendPlagueTokens(sink, holder, run.plague.tokens);
  }
  // B5 E5, R450: a targeting cost is part of the price, paid with it (Classic #89).
  if (run.discards !== undefined) payTargetingDiscards(sink, run.player, run.discards);
  // R210: the zone step 1 accepted is the play's until step 4 puts the card in it. A Tribute is
  // paid here, and a tributed unit's Death — #3 radiant's summon, #22's copies, #86's steals — lands
  // cards by R64 and R15 in the very row the play is going to; held like a Reborn zone (R64), the
  // named zone is closed to them, so they take the next one and the played card is never left with
  // no zone at all.
  if (run.zone !== null && run.tributes.length > 0) reserveZone(sink.state, run.zone);
  if (tributesAnEnemy(sink.state, run)) run.enemyTributed = true;
  payTributes(sink, run);
  consumeUsedDiscounts(sink, run, card);
  for (const id of spentRules) removeModifier(sink, run.player, id);
}

// ---------------------------------------------------------------------------
// Step 3 — the Gifted Program hook (§10.5 step 3)
// ---------------------------------------------------------------------------

/**
 * §10.5 step 3: "the Gifted Program hook may set `radiant` now". #64 itself is a static flag the
 * engine reads first (`giftedProgramStep`, R213), because step 1 has to know the face the play will
 * resolve with before it reads the play's choices (R214). Then every `onPlayHook` on the board runs
 * in R68's order, before the card is moved and before anything resolves, with the played card as its
 * selection and the cost paid in `data` — no Core card has one now, and the engine's fixtures keep
 * the hook honest. The cursor is advanced before each hook runs, so a hook that opens a prompt
 * continues with the hooks after it instead of running any of them twice.
 */
function giftedHookStep(sink: EngineSink, run: PlayRun): void {
  // R213: #64 Gifted Program's own rule, which step 1 has already read to know the face the play's
  // choices answer (R214), so the two cannot disagree. Once, before the hooks: the cursor is 0 only
  // on the first entry.
  if (run.hookIds === undefined) {
    // R449: a replacement comes first, so what step 3 makes Radiant is the card that is played.
    replacePlayedCard(sink, run);
    giftedProgramStep(sink, run);
    run.hookIds = triggerHoldersWithHook(sink.state, "onPlayHook").map((holder) => holder.card.id);
    run.hooksFrom = exitMark(sink.state);
  }
  const ids = run.hookIds;
  for (let at = run.hookAt; at < ids.length; at += 1) {
    run.hookAt = at + 1;
    // Each holder is read again as its turn comes: one an earlier hook took off the field, or out of
    // the zone that registers the hook, has nothing to run (R153, R174) — and one that has left the
    // field since the step began is not the holder it was, even back through Reborn: a new arrival
    // that did not stand there as the play reached step 3 (R83), as a hook queued before a death
    // does not fire for the body that came back.
    const holder = holderWithOnPlayHook(sink.state, ids[at], run.hooksFrom);
    if (holder === null) continue;
    runHookResumable(sink, holder.card, "onPlayHook", {
      controller: holder.controller,
      targets: [{ pick: "instance", instanceId: run.instanceId }],
      data: { playedId: run.instanceId, playedBy: run.player, costPaid: run.costPaid },
    });
    if (paused(sink)) return;
  }
}

/**
 * The `onPlayHook` holder a card is right now, or null when its zone no longer registers one, or
 * when it has left the field since `from` — the stay step 3 began with has ended (R174).
 */
function holderWithOnPlayHook(state: GameState, id: string | undefined, from: number | undefined): TriggerHolder | null {
  const card = id === undefined ? undefined : findInstance(state, id);
  if (card === undefined || card.zone.z !== "field") return null;
  if (from !== undefined && leftFieldAfter(state, from, card.id)) return null;
  const holder = triggerHolderFor(state, card);
  return holder === null || holder.script.onPlayHook === undefined ? null : holder;
}

/**
 * §8 #64 Gifted Program: "the first card costing 1 or less you play each turn becomes Radiant as it
 * is played" (2 or less on its radiant face). The card is a static flag the engine reads here —
 * `playChoices.giftedMakesRadiant` counts the player's plays this turn (R213) — so step 1 can know
 * the face this play will resolve with before it checks the play's choices (R214), as a hook that
 * only ran now could not tell it. Setting the flag is §6.3's Make Radiant: in hand the card's stats
 * and text swap on the next read (§5.2, R74), and the event says so.
 */
function giftedProgramStep(sink: EngineSink, run: PlayRun): void {
  const card = findInstance(sink.state, run.instanceId);
  if (card === undefined) return;
  const gifted = run.gifted ?? playMadeRadiant(sink.state, run.player, card, run.costPaid);
  if (!gifted) return;
  // R177: reported whether or not the card was Radiant already. A face-down trap stays hidden from
  // the other seat, whose stream keeps its events redacted but present (R97, R33), so a cue only for
  // a card that changed would tell that seat the trap's face in hand. Whether Gifted Program applies
  // is public — the cost paid and the Field Spell are — so the cue says nothing more than that.
  card.radiant = true;
  sink.events.push({ type: "radiantSet", instanceId: card.id, defId: card.defId, zone: card.zone });
}

/**
 * Classic #23 Devil's Pact, R449: "this turn, each card you play is replaced by a Book of Flame". A live
 * `replacePlays` modifier of the playing player replaces the card played, here at §10.5 step 3, by a
 * new instance of its definition (Radiant per the modifier) that is played in its place: it waits in
 * the resolving zone, makes its own choices as a cast does (R70) at the announce, and takes its zone
 * by R64 at step 4. The old card ceases to exist (R35), in its hand or in the resolving zone a cast
 * waited in, and the price paid was the old card's (its mana, Tribute and targeting cost stay paid).
 * Several modifiers replace in turn, in creation order, each re-reading the card; a card that already
 * is the named card on the named face is not replaced by itself.
 */
function replacePlayedCard(sink: EngineSink, run: PlayRun): void {
  const state = sink.state;
  for (const mod of [...state.players[run.player].mods]) {
    if (mod.kind !== "replacePlays" || !modifierIsLive(state, mod)) continue;
    const old = findInstance(state, run.instanceId);
    if (old === undefined) return;
    if (old.defId === mod.defId && old.radiant === mod.radiant) continue;
    // R226: a card already taken out of the hand (or the resolving zone) is not played at all.
    const zone = old.zone;
    if (zone.z !== "hand" && zone.z !== "resolving") return;
    if (zone.z === "hand" && zone.player !== run.player) return;
    // R177: a card replaced in a hand was never the other player's to read.
    const hiddenFrom = zone.z === "hand" ? [opponentOf(zone.player)] : [];

    const fresh = newInstance(state, mod.defId, run.player, { z: "resolving", player: run.player });
    fresh.radiant = mod.radiant;
    ceaseToExist(state, old);
    state.players[run.player].resolving.push(fresh);
    sink.events.push({
      type: "transformed",
      instanceId: old.id,
      fromDefId: old.defId,
      toDefId: fresh.defId,
      newInstanceId: fresh.id,
      ...(hiddenFrom.length === 0 ? {} : { hiddenFrom }),
    });

    run.instanceId = fresh.id;
    run.defId = fresh.defId;
    run.targets = [];
    run.modes = [];
    run.replaced = true;
    delete run.targetSlices;
    delete run.castChosen;
    // R214: whether step 3 makes it Radiant is read for the card that is played, off the board now.
    delete run.gifted;
  }
}

// ---------------------------------------------------------------------------
// The announce (B5 E1, R448): between §10.5 steps 3 and 4
// ---------------------------------------------------------------------------

/** B5 E1: how an announce names what the play declared — a card by its id, a hero as `hero-<player>`. */
function announcedTargets(targets: readonly Selection[]): string[] {
  return targets.flatMap((selection) => {
    if (selection.pick === "instance") return [selection.instanceId];
    if (selection.pick === "hero") return [`hero-${selection.player}`];
    return [];
  });
}

/**
 * The zone the announce names: the one step 1 accepted for a play, and for a cast or a replacement
 * the one R64 would give it now (step 4 reads it again, as the window may have changed the row).
 */
function announcedZone(state: GameState, run: PlayRun, card: CardInstance): ZoneSlot | null {
  if (run.cast !== true && run.replaced !== true) return run.zone;
  const type = defOf(state, card.defId).type;
  if (type === "Spell") return null;
  return firstFreeZone(state, run.player, type === "Unit" ? "units" : "backrow");
}

/**
 * R448: the card leaves its player's hand (or graveyard, R454) for the resolving zone (a cast's, or a replacement's,
 * waits there already), its announce opens, and `cardAnnounced` goes out — what `cardPlayed` would
 * show: a card to be set face-down shows the other player only its zone (`viewFor`). False when the
 * card is no longer where the play takes it from (R226): the play is lost.
 */
function announcePlay(sink: EngineSink, run: PlayRun): boolean {
  const state = sink.state;
  const card = findInstance(state, run.instanceId);
  if (card === undefined) return false;
  const side = state.players[run.player];
  if (run.cast === true || run.replaced === true) {
    if (card.zone.z !== "resolving") return false;
  } else {
    // E11, R454: from the hand, or from the graveyard for a play under a permission.
    if (!takeFromPlaySource(state, run, card)) return false;
    card.zone = { z: "resolving", player: run.player };
    side.resolving.push(card);
    // B5 E14, R546: the copy step 1 checked the choices against is the one the card resolves.
    if (run.copied !== undefined) fixCopiedText(state, card, run.copied);
  }

  const type = cardTypeOf(state, card);
  const faceDown = type === "Trap" || type === "Field Trap";
  beginAnnounce(state, { instanceId: card.id, player: run.player, ...(faceDown ? { faceDown: true as const } : {}) });
  const zone = announcedZone(state, run, card);
  sink.events.push({
    type: "cardAnnounced",
    player: run.player,
    instanceId: card.id,
    defId: card.defId,
    cardType: type,
    costPaid: run.costPaid,
    targets: announcedTargets(run.targets),
    ...(zone === null ? {} : { row: zone.row, lane: zone.lane }),
    ...(faceDown ? { faceDown: true as const } : {}),
  });
  return true;
}

/** Whether a queue entry is a trigger answering this card's announce (not an owed trap dispatch). */
function answersAnnounce(entry: GameState["triggerQueue"][number], instanceId: string): boolean {
  if (entry.hook === OWED_TO_TRAPS) return false;
  const event = eventOfQueued(entry);
  return event !== null && event.type === "cardAnnounced" && event.instanceId === instanceId;
}

/**
 * B5 E1, R448: the announce window. Traps see every event so far first, the announce among them, and
 * fire at once as responses (`triggers.dispatchPending`); then every other trigger the announce woke
 * resolves, one at a time in queue order (R68), each followed by the state check (R59). Triggers that
 * answer anything else wait for the step-4 loop, or for a cast the loop of the effect that cast it
 * (R70, R117), exactly as they did before the announce existed. A Counter among them cancels the play:
 * the triggers still to come on it find no card (`isAnnounceLive`) and are dropped, and a trap on it is
 * not offered it (`traps.standingEvent`). Stops at a prompt, leaving the rest owed in state.
 */
function runAnnounceWindow(sink: EngineSink, run: PlayRun): void {
  for (let pass = 0; pass < SETTLE_PASS_CAP; pass += 1) {
    dispatchPending(sink);
    if (paused(sink)) return;
    const queue = sink.state.triggerQueue;
    const at = queue.findIndex((entry) => answersAnnounce(entry, run.instanceId));
    if (at < 0) return;
    const [entry] = queue.splice(at, 1);
    if (entry === undefined || !isAnnounceLive(sink.state, run.instanceId)) continue;
    runQueuedTrigger(sink, entry);
    if (paused(sink)) return;
    stateCheck(sink);
    if (paused(sink)) return;
  }
  throw new Error(`the announce window did not close in ${SETTLE_PASS_CAP} passes (§10.3)`);
}

/**
 * B5 E1, R448: the step between §10.5 steps 3 and 4. A cast's choices, and a replacement's (R449), are
 * made first, so the announce names them (R70, R90). Then the card is announced and its window runs;
 * a question in it pauses the step, which is re-entered at its window (`PlayRun.announced`) after the
 * check the answer is owed (R59). Once the window is over the announce closes: a countered play stops
 * here (`PlayRun.countered`), and one whose card something else took out of the resolving zone is lost
 * (R226).
 */
function announceStep(sink: EngineSink, run: PlayRun): void {
  if (run.announced !== true) {
    if ((run.cast === true || run.replaced === true) && !castChoicesMade(sink, run, "announce")) return;
    if (!announcePlay(sink, run)) {
      run.lost = true;
      return;
    }
    run.announced = true;
  } else {
    stateCheck(sink);
    if (paused(sink)) return;
  }
  runAnnounceWindow(sink, run);
  if (paused(sink)) return;
  const record = endAnnounce(sink.state, run.instanceId);
  if (record?.countered === true) {
    run.countered = true;
    return;
  }
  const card = findInstance(sink.state, run.instanceId);
  if (card === undefined || card.zone.z !== "resolving") run.lost = true;
}

// ---------------------------------------------------------------------------
// Step 4 — move the card and announce the play (§10.5 step 4)
// ---------------------------------------------------------------------------

function playedEvents(sink: EngineSink, run: PlayRun, card: CardInstance, formerId: string | undefined): void {
  const former = formerId === undefined ? {} : { formerId };
  // R119: what has already arrived on the field during the play — a tributed unit's Death at step 2
  // (#22's copies of a Sheepish) — does not answer it, which the step-4 pair names, as step 7's does.
  const arrived = arrivedDuring(sink.state, run);
  const arrivals = arrived.length === 0 ? {} : { arrivedDuring: arrived };
  // R174, R212: the stays the play was announced on, so a response the loop hands the pair later —
  // behind the traps that answer what step 2 did — judges the played card from here.
  const exitsFrom = exitMark(sink.state);
  sink.events.push({
    type: "cardPlayed",
    player: run.player,
    instanceId: card.id,
    defId: card.defId,
    costPaid: run.costPaid,
    ...(card.x === undefined ? {} : { x: card.x }),
    ...(card.embiggened === undefined ? {} : { embiggened: card.embiggened }),
    ...former,
    ...arrivals,
    // R454: a play from the graveyard says so, for a client to animate it from there. Public: the
    // graveyard is.
    ...(run.source === "graveyard" ? { from: "graveyard" as const } : {}),
    exitsFrom,
  });
  if (run.zone !== null) {
    sink.events.push({
      type: "summoned",
      // The side it lands on, which is the player's own unless R360 summoned it for the opponent.
      player: run.zone.player,
      instanceId: card.id,
      defId: card.defId,
      row: run.zone.row,
      lane: run.zone.lane,
      ...former,
      ...(arrived.length === 0 ? {} : { arrivedDuring: [...arrived] }),
      exitsFrom,
    });
  }
}

/**
 * §10.5 step 4: the card leaves the hand for the field (Units, Field Spells, Traps) or for the
 * resolving state (Spells); `cardPlayed` goes out; `turnLog.cardsPlayed` and the game's `played`
 * counter go up (R55); "Sheepish fires here for Units".
 *
 * That last clause is why this step ends in the resolution loop: the trap answers the play event
 * and resolves before step 5 runs the Cry, which is R17's "fires before the Cry (Cry lost)". It
 * also means the play's own events are never left owed while a prompt is open. The loop holds the
 * state check until something in it has resolved (§4.5, `SettleOptions.holdCheck`), so a card that
 * arrives at 0 or less health is not collected before its own Cry (R118).
 */
function placeStep(sink: EngineSink, run: PlayRun): void {
  // Re-entered after a pause: the answer has resolved what asked — a trap, or a trigger whose
  // answered step and tail ran in the answer's own drain — so §4.5's check is due before anything
  // else moves (R59), as the loop runs it after a trigger it resolves itself. The unit that
  // trigger killed has died before step 5's Cry counts the board (R118, R113).
  const resumed = run.placed === true;
  if (!resumed) {
    // R70, R90: a cast has made its choices by now (`castChoicesStep`), before step 4 puts it on the
    // field, as a play makes them at step 1 with the card still in hand — so a cast Unit is never one
    // of its own Cry's options. A run owed from before that step existed makes them here.
    if (run.cast === true && !castChoicesMade(sink, run, "castChoices")) return;
    run.placed = true;
    if (!placeCard(sink, run)) run.lost = true;
  }
  // R17's step-4 window. The play has not resolved yet, so the loop holds §4.5's check until
  // something in it has (R118).
  //
  // A trap that asks here pauses the loop with its events still owed — the other traps that answer
  // the play (`triggers.OWED_TO_TRAPS`), the events after the one it answered, the triggers they
  // queue — and a trap is a response that resolves to completion before the play goes on (§10.3,
  // R118). So the step owes itself, and the answer brings it back to this loop rather than on to
  // step 5: a Sheepish owed the play's `cardPlayed` behind a trap that asked still turns the unit
  // into a Sheep before its Cry (R17).
  if (run.cast !== true) {
    settle(sink, { holdCheck: !resumed });
    return;
  }
  // R70: a cast is a play, so its step 4 is a window too, and a Sheepish answering a cast Unit turns
  // it into a Sheep before its Cry (R17). A cast runs inside another effect (§2.4's draw, #95), whose
  // own loop is running around it (`castThroughPipeline`), so the window is the traps' part of that
  // loop and no more (`triggers.dispatchPending`): every event so far reaches the traps, which fire
  // at once as responses — an earlier cast of the same chain included — while the other triggers
  // they wake, and the work owed around the cast, wait for the effect's own loop (R117). A cast
  // re-entered here after a trap's question meets the check the answer is owed first (R59).
  if (resumed) {
    stateCheck(sink);
    if (paused(sink)) return;
  }
  dispatchPending(sink);
}

/**
 * Every card acting on the field, both sides: each Stack pile's top and the backrow. A card dormant
 * under a pile is not on the field for effects and registers nothing (§3.2, R13, R153), so one that
 * resumes while a play resolves arrives for R119 as a Reborn body does.
 */
function fieldCardIds(state: GameState): string[] {
  const out: string[] = [];
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    for (const pile of side.units) {
      const top = pile?.[0];
      if (top !== undefined) out.push(top.id);
    }
    for (const card of side.backrow) if (card !== null && card !== undefined) out.push(card.id);
    // R446: a Unit a carrier holds acts on the field too.
    for (const card of side.carried ?? []) if (card !== null) out.push(card.id);
  }
  return out;
}

/**
 * R119: the permanents on the field now that were not there when step 4 announced the play, or have
 * left the field since and stand there again (a Reborn body, R83) — what arrived on the field while
 * the play resolved, on either side and whatever put it there. The played card is its own case
 * (`traps.isOwnArrival`, #33's own check).
 */
function arrivedDuring(state: GameState, run: PlayRun): string[] {
  if (run.standing === undefined) return [];
  const standing = new Set(run.standing);
  const from = run.standingFrom;
  return fieldCardIds(state).filter(
    (id) =>
      id !== run.instanceId && (!standing.has(id) || (from !== undefined && leftFieldAfter(state, from, id))),
  );
}

/**
 * Step 4's placement and announcement, once. False when the card is no longer the hand's to move,
 * or for a cast the resolving zone's (`PlayRun.lost`), which ends the play.
 */
function placeCard(sink: EngineSink, run: PlayRun): boolean {
  const state = sink.state;
  // R210: step 2 held the named zone for this play; it is released here, whatever happens next.
  if (run.zone !== null) releaseZone(state, run.zone);
  const card = findInstance(state, run.instanceId);
  if (card === undefined) return false;
  const side = state.players[run.player];

  // R448, R70, R226: the card waits in the resolving zone from its announce — a played card moved
  // there out of its owner's hand, a cast's or a replacement's was made there. One that has left it
  // by now is where that move put it, in one zone (§10.1), and is not played: pulling it back out of
  // exile would undo a move §6.3 makes final. Not `moveToZone`: that resets the instance (R78), and a
  // play does not.
  if (card.zone.z !== "resolving") return false;
  removeFromAnyZone(state, card);
  if (run.cast === true || run.replaced === true) {
    // R70, R449: a permanent takes the leftmost empty, unlocked zone of its row (R64), as a play that
    // names none does.
    const type = cardTypeOf(state, card);
    run.zone = type === "Spell" ? null : firstFreeZone(state, run.player, type === "Unit" ? "units" : "backrow");
  } else {
    // R360: #55's base face, paid for with an opposing unit, is summoned for the opponent. It is
    // still this player's play (`cardPlayed`), while the receiving side becomes its current owner
    // and controller on the field (R12, `placeOnField`).
    const theirs = handedOverZone(state, run, card);
    if (theirs !== null) run.zone = theirs;
  }

  // R227: a Trap or Field Trap set face-down takes a fresh id before anything names it on the field,
  // so no player can link the face-down card to an id they saw while it was public (R177). The run
  // follows the card, and the events that place it carry the id it had (`formerId`).
  let formerId: string | undefined;
  if (run.zone !== null && landsFaceDown(state, card, run.zone.row)) {
    formerId = freshFaceDownId(state, card);
    run.instanceId = card.id;
  }

  // §3.2/§6.2 Stack: step 1 already accepted an occupied unit zone for a Stack card, so the
  // placement is the one that builds the pile — the arriving card goes on top and the card beneath
  // stops acting (R13). Every other card needs the zone empty, which is what `stack: false` keeps
  // `placeOnField` insisting on. R210 keeps the zone the play's, so a refusal is a broken
  // invariant rather than a game rule; should one ever happen, the card waits in `resolving` and
  // step 7 lands it in its graveyard, as R138 has a permanent with no zone do, rather than being
  // left in no pile at all (§10.1).
  if (run.zone !== null && placeOnField(state, card, run.zone, { stack: playsOnStack(state, card) })) {
    card.summonedTurn = state.turn;
    run.placedFrom = exitMark(state);
  } else {
    run.zone = null;
    card.zone = { z: "resolving", player: run.player };
    side.resolving.push(card);
  }
  // R453: a cast Field Spell, Trap or Field Trap with no zone fizzles; a cast permanent on the field
  // has no landing for "then exile it" to change (Classic #56's rider is a Spell's).
  if (run.cast === true) settleCastPlacement(state, run, card);

  // R119: a run owed from before the marks were kept reads the board the play was announced on.
  if (run.standing === undefined) {
    run.standing = fieldCardIds(state);
    run.standingFrom = exitMark(state);
  }

  side.turnLog.playedIds.push(card.id);
  side.turnLog.cardsPlayed += 1;
  // R213: what this play paid, which the next play's Gifted Program check counts (R56, R70).
  side.turnLog.costsPaid = [...(side.turnLog.costsPaid ?? []), run.costPaid];
  state.counters.played += 1;
  // R429: a card that counts its own plays counts this one here, with every other count of it.
  countPlay(card);
  run.radiant = card.radiant;
  // B5 E4, R451: the per-turn types, the per-game tags and the "last" records.
  recordPlay(state, run.player, card);

  playedEvents(sink, run, card, formerId);
  // B3.1 rule 4 (R383): an Animated Field Spell, or an "Animated on your turn" card on its controller's
  // turn, animates as it enters the field.
  animateOnEntry(sink, card);
  // §6.2 Echo, R30: the Spell GAINS its Echo as it is played — "the next Spell you play gains Echo
  // +1" — so the grant is taken here, from the player who played it, and not at step 6 after the
  // Spell's own text has run. Taken later, a Spell that moves Twinspell to the other side (#87's
  // board swap) or out of play took nothing, and Twinspell stayed for the other player's next one.
  // The repeats still resolve at step 6, which only takes what is queued.
  run.echoQueued = true;
  queueEchoRepeats(sink, card, run.player);
  // E39 (Classic+ #14): "the next Spell you play gains …" is gained as it is played, as Echo is (R178).
  stampNextSpell(sink, card, run.player);
  return true;
}

/**
 * E11, R454, §10.5 step 4: take the played card out of the pile the play takes it from — its owner's
 * hand, or, for a play from the graveyard, its player's graveyard. False, and nothing moved, when it is
 * no longer there (R226): a Tribute's Death at step 2 or a step-3 hook has moved it, and it is not
 * played. Exported so a step that moves the card earlier (E1's announce) takes it the same way.
 */
export function takeFromPlaySource(state: GameState, run: PlayRun, card: CardInstance): boolean {
  if (run.source === "graveyard") {
    if (!inOwnGraveyard(state, run.player, card)) return false;
    removeFromAnyZone(state, card);
    return true;
  }
  const side = state.players[run.player];
  const at = side.hand.findIndex((held) => held.id === card.id);
  if (at < 0) return false;
  side.hand.splice(at, 1);
  return true;
}

/** R453: a cast permanent's placement settled — fizzled when it found no zone, the Spell rider dropped. */
function settleCastPlacement(state: GameState, run: PlayRun, card: CardInstance): void {
  if (run.zone !== null) {
    delete card.memory[EXILE_ON_LANDING];
    return;
  }
  const type = cardTypeOf(state, card);
  if (type !== "Spell" && type !== "Unit") run.fizzled = true;
}

/**
 * E39, R455 (Classic+ #14 Forever&): every live `enchantNextSpell` modifier of the player gives its
 * enchantment to the Spell being played and is spent — "the next Spell you play gains …". A cast is a
 * play (R70), so a cast Spell takes it; a countered play never reaches step 4 and leaves it waiting.
 */
function stampNextSpell(sink: EngineSink, card: CardInstance, player: PlayerId): void {
  const state = sink.state;
  if (cardTypeOf(state, card) !== "Spell") return;
  for (const mod of [...state.players[player].mods]) {
    if (mod.kind !== "enchantNextSpell" || !modifierIsLive(state, mod)) continue;
    card.enchantments = [...(card.enchantments ?? []), { ...mod.enchantment }];
    removeModifier(sink, player, mod.id);
  }
}

// ---------------------------------------------------------------------------
// Step 5 — resolve the card (§10.5 step 5)
// ---------------------------------------------------------------------------

/**
 * §6.2 Combo X: "cards you played earlier this turn", so the card being played does not count, and
 * nor does a card its own step 5 casts — the count is the one at play time (`query.playedEarlier`).
 */
function playedEarlierThisTurn(state: GameState, run: PlayRun): number {
  return playedEarlier(state, run.player, run.instanceId);
}

/**
 * The card as step 4 left it, or null when it is no longer there to resolve: Sheepish transformed
 * it, a trap countered it, or it has ceased to exist (R11's `{ z: "gone" }`). R17: the Cry is lost.
 */
function stillResolving(state: GameState, run: PlayRun): CardInstance | null {
  const card = findInstance(state, run.instanceId);
  if (card === undefined || card.defId !== run.defId) return null;
  const zone = card.zone.z;
  if (zone === "resolving") return card;
  if (zone !== "field") return null;
  // R174, R118: the stay step 4 put it on. One it has left since — a trap answering the play killed
  // it at step 4 — is gone for the play, and a Reborn body in its zone is a new arrival whose "Cry
  // does not fire" (§4.5 step 4, R1).
  return run.placedFrom !== undefined && leftFieldAfter(state, run.placedFrom, card.id) ? null : card;
}

/**
 * #38 Quickstriker's lasting effect (`staticFlags.quickstriker`) as it is granted from this player's
 * side of the field: one entry per grant, each the multiple of X that grant deals as one hit — its
 * granting card's own face's (`QUICKSTRIKER_COMBO_MULTIPLE`, R281), so a base and a Radiant
 * Quickstriker give `[1, 2]`, and a card fused from two carries both texts at its one face (R102).
 * The played card is never one of them: a permanent does not answer its own arrival (R119), and a
 * Quickstriker being played is on the field by step 5. Nor is one that arrived on the field during
 * the play (`arrivedDuring`): a copy a tributed Cube's Death summoned at step 2, one #95's first
 * resolution summoned, met again by the Echo repeat of that same play (R119).
 */
function quickstrikerGrants(state: GameState, run: PlayRun, played: CardInstance): number[] {
  const arrived = new Set(arrivedDuring(state, run));
  const multiples: number[] = [];
  for (const row of ["units", "backrow"] as const) {
    for (const ref of slotsOf(run.player, row)) {
      const held = cardAt(state, ref);
      if (held === null || held.id === played.id || arrived.has(held.id)) continue;
      const flag = flagsOf(held).quickstriker;
      const grants = flag === true ? 1 : typeof flag === "number" ? Math.max(0, Math.trunc(flag)) : 0;
      const multiple = QUICKSTRIKER_COMBO_MULTIPLE[held.radiant ? "radiant" : "base"];
      for (let grant = 0; grant < grants; grant += 1) multiples.push(multiple);
    }
  }
  return multiples;
}

/**
 * #38 Quickstriker: "your cards gain 'Combo X: deal X damage to the enemy hero', X = cards you
 * played earlier this turn" (radiant 2X) — one of the two granted Combo parts §10.5 step 5 resolves
 * before the card's own script. It is the Field Spell's lasting effect, so it is read off the
 * permanents on the field now (one hit per grant) rather than from a trigger on `cardPlayed`, which
 * popped whenever that event was dispatched: a cast's events wait for the loop of the effect that
 * cast it (R70), so a cast-on-draw chain of two read the count after the chain, 1 and 1, instead of
 * 0 and 1. R281: each grant's hit is X times its face's multiple, dealt as ONE damage instance
 * (§4.4), so Armor and the Anti-oneshot cap apply to a Radiant one's 2X once; the grants go in board
 * order, each its own hit. A `quickstrikerDamage` modifier counts as one more grant of X.
 */
function quickstrikerCombo(sink: EngineSink, run: PlayRun, card: CardInstance): void {
  const state = sink.state;
  const amount = playedEarlierThisTurn(state, run);
  if (amount <= 0) return;
  const riders = state.players[run.player].mods.filter(
    (mod) => mod.kind === "quickstrikerDamage" && modifierIsLive(state, mod) && inPlaceBefore(run, mod.id),
  ).length;
  const multiples = [
    ...quickstrikerGrants(state, run, card),
    ...Array.from({ length: riders }, () => QUICKSTRIKER_COMBO_MULTIPLE.base),
  ];
  for (const multiple of multiples) {
    dealDamage(sink, {
      source: card,
      target: { kind: "hero", player: opponentOf(run.player) },
      amount: amount * multiple,
    });
  }
}

/**
 * #78 /fullsend: "this turn your cards gain 'Combo: draw 1'" — a Combo with no X is Combo 1 (§6.2).
 * Each live rider is its own draw, and "draw N" is N separate draws (§2.4, R58), so the riders are
 * one `draw` of their total: a draw that pauses owes the rest to the answer (R113), and one whose
 * cast ended the game ends the rest with it (R216) — looping over the riders drew on over both.
 */
function comboDrawStep(sink: EngineSink, run: PlayRun): void {
  const state = sink.state;
  if (playedEarlierThisTurn(state, run) < 1) return;
  let draws = 0;
  for (const mod of state.players[run.player].mods) {
    if (mod.kind === "comboDraw" && modifierIsLive(state, mod) && inPlaceBefore(run, mod.id)) {
      draws += Math.max(0, mod.amount);
    }
  }
  if (draws > 0) draw(sink, run.player, draws);
}

/**
 * R119: whether a modifier was in place as the play began (`PlayRun.modsBefore`). One the play
 * installed itself — /fullsend's "Combo: draw 1", which an Echo repeat of the same /fullsend would
 * otherwise meet — does not answer it.
 */
function inPlaceBefore(run: PlayRun, id: string): boolean {
  return run.modsBefore === undefined || run.modsBefore.includes(id);
}

/**
 * R174: the targets step 5 hands the card's script. §10.5 step 1 checks a play's targets and its
 * Tribute each on its own (R90), so one play may name a unit both as a target and as a Tribute —
 * #55 Lava Golem's enemy tribute, crafted onto #68 Twisted Sorcerer's "deal 4 damage to a target".
 * Step 2 sacrifices it before anything resolves, and an effect aimed at a card on the field is aimed
 * at that stay: the unit has left the field, so its slot names nothing and the effect fizzles (§8
 * Conventions), even when Reborn has put a new body in its zone. The slot is kept, as `none`, so the
 * declarations after it still read their own (R90).
 */
function standingTargets(run: PlayRun): Selection[] {
  if (run.tributes.length === 0) return run.targets;
  return run.targets.map((selection) =>
    selection.pick === "instance" && run.tributes.includes(selection.instanceId) ? { pick: "none" } : selection,
  );
}

/**
 * What the card's Cry is handed in its data: a fused card's declaration slices (R90, R102), and the
 * player's mana as the play began (`EffectContext.manaBeforePlay`, Classic #22).
 */
function cryData(run: PlayRun): Record<string, unknown> {
  return {
    ...(run.targetSlices === undefined ? {} : { [DECLARATION_SLICES_KEY]: run.targetSlices }),
    ...(run.manaBefore === undefined ? {} : { [MANA_BEFORE_PLAY_KEY]: run.manaBefore }),
  };
}

/**
 * §10.5 step 5: "Resolve Combo checks, Quickstriker, /fullsend's Combo draw, then the card's own
 * Cry or spell script (targets already chosen)". An ordinary card's own Combo check is part of its
 * own script, which reads `query.playedEarlier`; what the engine owes is the two Combo abilities
 * another permanent grants everything you play, and they come first.
 */
function resolveStep(sink: EngineSink, run: PlayRun): void {
  if (run.cast === true && !castChoicesMade(sink, run)) return;
  // R453: a cast that found no zone for its Field Spell or Trap resolves nothing.
  if (run.fizzled === true) return;
  for (let at = run.resolveAt; at < RESOLVE_PARTS.length; at += 1) {
    run.resolveAt = at + 1;
    const card = stillResolving(sink.state, run);
    if (card === null) return;

    switch (RESOLVE_PARTS[at]) {
      case "quickstriker":
        quickstrikerCombo(sink, run, card);
        break;
      case "comboDraw":
        comboDrawStep(sink, run);
        break;
      case "script":
        // B5 E14, R399: a copier resolves the copied Spell's script, itself as "this".
        runHookResumable(sink, textFaceOf(sink.state, card), "cry", {
          controller: run.player,
          targets: standingTargets(run),
          modes: run.modes,
          data: cryData(run),
          // R174: the choices are aimed at the stays step 1 checked them on (a cast's, once made).
          ...(run.exitsFrom === undefined ? {} : { exitsFrom: run.exitsFrom }),
        });
        break;
      default:
        break;
    }

    if (paused(sink)) return;
  }
}

/**
 * R70: "the caster picks its targets and modes", and R81: a choice made during resolution — a cast's
 * among them — opens a `PendingChoice`. So a cast of a card that declares targets or modes asks its
 * caster for them, declaration by declaration, the way an Echo repeat asks for its fresh picks
 * (§10.6) — and for the face step 5 resolves, since step 3 has already made it Radiant if it is
 * going to be (R214). It asks as step 4 begins, before the card is placed (`placeStep`): a play's
 * choices are checked at step 1 with the card still in hand (R90), so a cast Unit asked after its
 * placement was offered as a target of its own Cry, which no play of it ever is. A cast whose caller
 * named its choices (none in Core) keeps them. Returns false while a prompt is waiting.
 */
function castChoicesMade(sink: EngineSink, run: PlayRun, step: PlayStepName = "resolve"): boolean {
  if (run.castChosen === true) return true;
  const card = stillResolving(sink.state, run);
  if (card === null) return true;
  // R453: an X card's X first, so its targets and modes are asked of the card as it will resolve.
  if (!castXChosen(sink, run, card, step)) return false;
  if (run.repeat === null) {
    // B5 E14, R399: a copier's choices are its copied text's.
    const face = textFaceOf(sink.state, card);
    const declares = declaredTargets(face).length > 0 || declaredModes(face).length > 0;
    if (!declares || run.targets.length > 0 || run.modes.length > 0) {
      run.castChosen = true;
      return true;
    }
    run.repeat = { targets: [], modes: [], declAt: 0, modeAt: 0 };
  }
  if (!askRepeatChoices(sink, run, card, step)) return false;
  const chosen = run.repeat;
  run.repeat = null;
  run.castChosen = true;
  if (chosen === null) return true;
  run.targets = [...chosen.targets];
  run.modes = [...chosen.modes];
  // R174: the cast's choices were made against the board as its caster answered.
  run.exitsFrom = exitMark(sink.state);
  // A fused card's Cry splits the choices by its ingredients' declarations (R90, R102).
  Object.assign(run, slicesFor(sink.state, run.player, card, run.costPaid, run.targets, run.modes));
  return true;
}

/**
 * R453: a cast X card's X, which its caster chooses (R70, R81) as the cast's first choice — a `number`
 * prompt over MIN_CHOSEN_X up to their current mana, at least MIN_CHOSEN_X (R348), each option the
 * number as a mode (`{ pick: "mode", option: "3" }`). A cast pays nothing, so the X costs nothing; it
 * is stored on the instance, where the card's script and R396 read it. A random cast's X was set as the
 * cast began: the caster's current mana (`castThroughPipeline`). False while the prompt waits.
 */
function castXChosen(sink: EngineSink, run: PlayRun, card: CardInstance, step: PlayStepName): boolean {
  const state = sink.state;
  if (!choosesX(state, card) || card.x !== undefined) return true;
  const most = Math.max(MIN_CHOSEN_X, state.players[run.player].mana.current);
  if (run.castX !== undefined) {
    card.x = Math.max(MIN_CHOSEN_X, Math.min(most, run.castX));
    return true;
  }
  if (run.random === true || randomCastOf(state, run.player) !== null) {
    card.x = most;
    return true;
  }
  const values = Array.from({ length: most - MIN_CHOSEN_X + 1 }, (_, at) => String(MIN_CHOSEN_X + at));
  run.awaiting = "castX";
  const opened = openPrompt(sink, {
    player: run.player,
    kind: "number",
    prompt: `${askLabel(step, defOf(state, card.defId).name)}: choose X`,
    options: values.map((option) => ({ key: `mode:${option}`, label: option, selection: { pick: "mode", option } })),
    resume: resumeFor(run, PLAY_STEPS.indexOf(step)),
  });
  if (opened !== null) return false;
  run.awaiting = null;
  card.x = most;
  return true;
}

// ---------------------------------------------------------------------------
// Step 6 — Echo (§10.5 step 6, R30)
// ---------------------------------------------------------------------------

function labelOf(selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return selection.instanceId;
    case "hero":
      return `${selection.player}'s hero`;
    case "zone":
      return `${selection.player} ${selection.row} ${selection.lane}`;
    case "mode":
      return selection.option;
    default:
      return "nothing";
  }
}

/**
 * R81: a card's play choices travel in the `play` action *once*. A repeat therefore asks again, as
 * prompts — §10.6's "an Echo repeat of Glowy Jelly Bean reopens its hand pick" — so each
 * declaration the card made is offered in turn and the answers collect in the repeat record. The
 * prompt carries the run record, so answering it re-enters this pipeline (`answerPlayPrompt`, which
 * `prompts.answerPrompt` hands it to).
 *
 * Returns true when every declaration has its answer and the repeat can resolve. A declaration the
 * board cannot satisfy is skipped rather than refused: the effect fizzles (R90, §8's conventions).
 */
function askRepeatChoices(sink: EngineSink, run: PlayRun, played: CardInstance, step: PlayStepName = "echo"): boolean {
  const repeat = run.repeat;
  if (repeat === null) return false;
  // B5 E14, R399: a copier asks for the choices its copied text declares.
  const card = textFaceOf(sink.state, played);
  // A declaration that belongs to some modes only (`forModes`, #24) cannot be asked before the
  // mode it depends on, so such a card's repeat asks its modes first.
  if (targetsFollowModes(declaredTargets(card))) {
    return askRepeatModes(sink, run, card, repeat, step) && askRepeatTargets(sink, run, card, repeat, step);
  }
  return askRepeatTargets(sink, run, card, repeat, step) && askRepeatModes(sink, run, card, repeat, step);
}

/** What a prompt the pipeline opens calls itself: an Echo repeat's picks, or a cast's (R70). */
function askLabel(step: PlayStepName, name: string, run?: PlayRun): string {
  if (step === "echo") return `Echo: ${name}`;
  // R449: a replacement is played, not cast, and asks its choices as a cast does.
  return run?.cast !== true && run?.replaced === true ? `Play: ${name}` : `Cast: ${name}`;
}

/**
 * R70, R452: a cast's own choices, made as a step of their own once step 3 has settled the face they
 * answer (R214) and before the card moves (R90) — so whatever announces the cast next (E1's window,
 * between steps 3 and 4) announces them. A play made its choices at step 1: nothing to do.
 */
function castChoicesStep(sink: EngineSink, run: PlayRun): void {
  if (run.cast !== true) return;
  castChoicesMade(sink, run, "castChoices");
}

type RepeatRecord = NonNullable<PlayRun["repeat"]>;

/**
 * The options one declaration offers a pick the pipeline makes itself — an Echo repeat's or a cast's
 * (R81, R70) — narrowed to enemies when the run targets enemies and one is legal (R452), or to friendly
 * targets when the declaration is beneficial (aim "help", R656).
 */
function castTargetOptions(state: GameState, run: PlayRun, card: CardInstance, decl: TargetDecl): Selection[] {
  const options = legalSelectionsFor(state, run.player, card, decl);
  if (run.targetEnemies !== true) return options;
  const prefer = targetAim(decl) === "help" ? preferFriends : preferEnemies;
  return prefer(state, run.player, options, (selection) => selection, Math.min(decl.min, options.length));
}

/** The repeat's target declarations, each offered in turn; false while one is waiting (R81). */
function askRepeatTargets(
  sink: EngineSink,
  run: PlayRun,
  card: CardInstance,
  repeat: RepeatRecord,
  step: PlayStepName,
): boolean {
  const name = defOf(sink.state, card.defId).name;
  const targets = activeTargetDecls(declaredTargets(card), repeat.modes);
  for (let at = repeat.declAt; at < targets.length; at += 1) {
    repeat.declAt = at + 1;
    const decl = targets[at];
    if (decl === undefined) continue;
    const options = castTargetOptions(sink.state, run, card, decl);
    if (options.length === 0) continue;
    // R452: a random cast makes this pick itself, at random, and asks nobody.
    if (run.random === true || randomCastOf(sink.state, run.player) !== null) {
      repeat.targets.push(...randomPicks(sink.rng, options, decl.min, decl.max));
      continue;
    }
    run.awaiting = "echoTarget";
    const opened = openPrompt(sink, {
      player: run.player,
      kind: decl.kind,
      aim: decl.aim,
      prompt: askLabel(step, name, run),
      options: options.map((selection) => ({
        key: `${selection.pick}:${labelOf(selection)}`,
        label: labelOf(selection),
        selection,
      })),
      min: decl.min,
      max: decl.max,
      resume: resumeFor(run, PLAY_STEPS.indexOf(step)),
    });
    if (opened !== null) return false;
    run.awaiting = null;
  }
  return true;
}

/** The repeat's mode declarations, each offered in turn; false while one is waiting (R81). */
function askRepeatModes(
  sink: EngineSink,
  run: PlayRun,
  card: CardInstance,
  repeat: RepeatRecord,
  step: PlayStepName,
): boolean {
  const name = defOf(sink.state, card.defId).name;
  const modes = declaredModes(card);
  for (let at = repeat.modeAt; at < modes.length; at += 1) {
    repeat.modeAt = at + 1;
    const decl = modes[at];
    if (decl === undefined || decl.options.length === 0) continue;
    // R452: a random cast makes this choice itself, at random.
    if (run.random === true || randomCastOf(sink.state, run.player) !== null) {
      const option = sink.rng.pick(decl.options);
      if (option !== undefined) repeat.modes.push(option);
      continue;
    }
    run.awaiting = "echoMode";
    const opened = openPrompt(sink, {
      player: run.player,
      kind: decl.kind,
      prompt: askLabel(step, name, run),
      options: decl.options.map((option) => ({
        key: `mode:${option}`,
        label: option,
        selection: { pick: "mode", option },
      })),
      resume: resumeFor(run, PLAY_STEPS.indexOf(step)),
    });
    if (opened !== null) return false;
    run.awaiting = null;
  }
  return true;
}

/**
 * §10.5 step 6: "Echo: repeat step 5 with fresh prompts N times". The repeats outstanding live in
 * `state.echoQueue` and resolve one at a time, so a prompt inside one pauses the rest (§6.1). This
 * is the step an owed pipeline re-enters, and it picks up from whatever the queue and the current
 * repeat record say, which is why it can be entered any number of times.
 *
 * Then §4.5's check, once the last resolution is over and before step 7 (R59): the check runs after
 * "a card's whole Cry, spell, trap or triggered script", and step 7's `cardResolved` is what #60 Bear
 * Honeypot and #85 Unlicensed Experimentation answer, so they must meet the board the card left —
 * without the units its Cry or spell has killed. It sits here rather than in step 7 because this
 * step is re-entered at itself: a Death hook that asks pauses the play here, and the answer brings
 * it back through a check that finds nothing left to do, and on to step 7.
 */
function echoStep(sink: EngineSink, run: PlayRun): void {
  resolveEchoRepeats(sink, run);
  if (paused(sink)) return;
  stateCheck(sink);
}

function resolveEchoRepeats(sink: EngineSink, run: PlayRun): void {
  const opening = stillResolving(sink.state, run);
  if (opening === null || run.fizzled === true) {
    dropEchoRepeats(sink.state, run.instanceId);
    return;
  }
  if (!run.echoQueued) {
    // Once per resolution: `queueEchoRepeats` consumes R30's grant (`echo.ts`).
    run.echoQueued = true;
    queueEchoRepeats(sink, opening, run.player);
  }

  for (;;) {
    const card = stillResolving(sink.state, run);
    if (card === null) {
      dropEchoRepeats(sink.state, run.instanceId);
      return;
    }

    if (run.repeat === null) {
      if (echoRepeatsOwed(sink.state, run.instanceId) <= 0) return;
      // §4.5: the resolution before this repeat was a whole spell script, and the state check runs
      // after every one (R59) — so its deaths, their Death hooks and a hero at 0 are settled before
      // the repeat asks anything. A dead unit is not offered again, and a game the first resolution
      // won ends there. A Death hook that asks pauses the repeats here; they wait in state.
      stateCheck(sink);
      if (paused(sink)) return;
      if (stillResolving(sink.state, run) === null) {
        dropEchoRepeats(sink.state, run.instanceId);
        return;
      }
      if (!takeEchoRepeat(sink.state, run.instanceId)) return;
      run.repeat = { targets: [], modes: [], declAt: 0, modeAt: 0 };
    }

    if (!askRepeatChoices(sink, run, card)) return;
    if (!resolveRepeat(sink, run)) return;
  }
}

/**
 * One Echo repeat, once its fresh answers are in: step 5 again, whole — #38 Quickstriker's and #78
 * /fullsend's granted Combo parts, then the card's own script (§10.5 step 6: "repeat step 5"). The
 * repeat record keeps its place in `RESOLVE_PARTS`, so a Combo draw that pauses resumes at the next
 * part rather than drawing again; the record is let go as the script starts, as before, because the
 * script's own tail is parked by `runHookResumable` and resumes ahead of this step (R113).
 *
 * Returns false when a prompt (or the end of the game) stopped the repeat.
 */
function resolveRepeat(sink: EngineSink, run: PlayRun): boolean {
  const repeat = run.repeat;
  if (repeat === null) return true;
  // R174: the stays the repeat's fresh picks were made on, taken once, as its answers are all in.
  repeat.exitsFrom ??= exitMark(sink.state);
  for (let at = repeat.partAt ?? 0; at < RESOLVE_PARTS.length; at += 1) {
    repeat.partAt = at + 1;
    const card = stillResolving(sink.state, run);
    if (card === null) break;
    switch (RESOLVE_PARTS[at]) {
      case "quickstriker":
        quickstrikerCombo(sink, run, card);
        break;
      case "comboDraw":
        comboDrawStep(sink, run);
        break;
      case "script":
        run.repeat = null;
        // B5 E14, R399: an Echo repeat of a copier runs its copied text again.
        runHookResumable(sink, textFaceOf(sink.state, card), "cry", {
          controller: run.player,
          targets: repeat.targets,
          modes: repeat.modes,
          exitsFrom: repeat.exitsFrom,
          ...(run.manaBefore === undefined ? {} : { data: { [MANA_BEFORE_PLAY_KEY]: run.manaBefore } }),
        });
        break;
      default:
        break;
    }
    if (paused(sink)) return false;
  }
  run.repeat = null;
  return true;
}

// ---------------------------------------------------------------------------
// Step 7 — where a Spell lands (§10.5 step 7)
// ---------------------------------------------------------------------------

/**
 * §10.5 step 7, which `echo.ts` owns because a cast lands the same way (R70): the card reaches the
 * graveyard and `cardResolved` goes out — once per play, here, after step 6 has drained every Echo
 * repeat, which is the moment R17 gives Bear Honeypot, Unstable Clone Machine and Unlicensed
 * Experimentation.
 *
 * R155: this is also where §5.1's `returnToHandAtEndOfTurn` is written, because this is the step
 * §5.1 describes — the Spell has just reached the graveyard, and only a card that got there this
 * way returns from it at the end of the turn. `resolve.flagReturnToHandAtEndOfTurn` holds the three
 * conditions; it runs after the landing because "reached the graveyard" is one of them, and it is a
 * no-op for everything else the step lands — a permanent, and a Spell that exiled itself (#39).
 */
function finishStep(sink: EngineSink, run: PlayRun): void {
  // E39, R410, R455 (Classic+ #14 Forever&): a Spell with the return comes back once it has landed.
  const landing = findInstance(sink.state, run.instanceId)?.zone.z === "resolving";
  landAfterResolution(sink, {
    instanceId: run.instanceId,
    defId: run.defId,
    player: run.player,
    // The same number step 4's `cardPlayed` reported: `run.costPaid` is the mana step 2 actually
    // charged, after every modifier and with R65's X and embiggen prices in it. A cast carries 0
    // (R70), which `castThroughPipeline` puts on the run it drives.
    costPaid: run.costPaid,
    radiant: run.radiant ?? false,
    // R174, R61: whether the card is still in play is asked of the stay step 4 put it on.
    ...(run.placedFrom === undefined ? {} : { placedFrom: run.placedFrom }),
    arrivedDuring: arrivedDuring(sink.state, run),
  });
  if (landing) returnAfterResolving(sink, run);
  flagReturnToHandAtEndOfTurn(sink.state, run.instanceId);
}

/**
 * E39, R410, R455 (Classic+ #14 Forever&: "After this resolves, return it to your hand"): a Spell
 * carrying a `returnAfterResolve` enchantment that has resolved — played or cast — and that step 7 has
 * just landed goes back to its owner's hand from the graveyard or the exile pile it went to (its own
 * "exile this", a cast's "then exile it", a "would go to a graveyard" replacement), the hand cap
 * burning it as always (§2.4). A countered Spell never reaches step 7, a discarded one was never
 * played, and a fizzled cast resolved nothing.
 */
function returnAfterResolving(sink: EngineSink, run: PlayRun): void {
  if (run.fizzled === true) return;
  const card = findInstance(sink.state, run.instanceId);
  if (card === undefined || (card.zone.z !== "graveyard" && card.zone.z !== "exile")) return;
  if (cardTypeOf(sink.state, card) !== "Spell" || !hasEnchantment(card, "returnAfterResolve")) return;
  addToHand(sink, card);
}

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------

type Step = {
  name: PlayStepName;
  run: (sink: EngineSink, run: PlayRun) => void;
  /** A step that is re-entered at itself, because it holds its own place in state. */
  repeats?: boolean;
};

/** Steps 2 to 8 of §10.5; step 1 runs before any of them and cannot pause. */
const STEP_TABLE: readonly Step[] = [
  { name: "validate", run: () => {} },
  { name: "pay", run: payStep },
  { name: "giftedHook", run: giftedHookStep, repeats: true },
  { name: "castChoices", run: castChoicesStep, repeats: true },
  { name: "announce", run: announceStep, repeats: true },
  { name: "place", run: placeStep, repeats: true },
  { name: "resolve", run: resolveStep, repeats: true },
  { name: "echo", run: echoStep, repeats: true },
  { name: "finish", run: finishStep },
  // §10.5 step 8. A cast settles nothing of its own: it runs inside another effect, whose loop
  // takes its events, and §2.4's chain runs the state check a cast on draw is owed (R59).
  { name: "settle", run: (sink, run) => (run.cast === true ? undefined : settle(sink)) },
];

/**
 * Run the steps from `run.at` on, and stop the moment one leaves a prompt open.
 *
 * The rest of the pipeline is owed to `state.work` *only when a step actually pauses*, never
 * before — R117, which this file used to get wrong. Two reasons, and the first is that bug:
 *
 *  - While `drive` is on the stack the steps are the driver's, so nothing else may run them.
 *    Step 4 ends in the resolution loop (`placeStep`), and `settle` drains `state.work` before it
 *    pops a trigger — so a pipeline that had parked itself in advance was drained by its own
 *    nested `settle`, which ran steps 5 to 8 (the Cry included) and then handed control back to a
 *    loop that ran them again. Owing the rest only at a pause keeps one owner per step: the
 *    driver while it is running, `state.work` once it has stopped (R1: a Cry fires exactly once).
 *  - It is also the order R113 wants, which R117 says follows from the rule above rather than being
 *    separate. `work.ts` parks at `state.workCursor`, which the pausing scope has just advanced past
 *    its own item, so the tail of a Cry's effect list — parked by the step that was running — lands
 *    in front of this one and resumes first: steps 6, 7 and 8 come *after* the Cry rather than
 *    inside it. Parking in advance put the pipeline ahead of that tail.
 *
 * Returns true when the pipeline is finished with (a game that ended under it included).
 */
function drive(sink: EngineSink, run: PlayRun): boolean {
  // R452: a random cast, or one that targets enemies, is in force while its steps run, however often
  // they are driven — the first time, and each time an answer brings the owed rest back.
  const finished = withCastMode(sink.state, castModeOf(run), () => driveSteps(sink, run));
  // R58: a cast-on-draw cast has resolved, so the draw that cast it is complete.
  if (finished && run.drawnAs !== undefined) releaseDraw(sink.state, run.drawnAs);
  return finished;
}

/** R452: the mode a run's steps run under, or null for a play and an ordinary cast. */
function castModeOf(run: PlayRun): CastMode | null {
  if (run.cast !== true || (run.random !== true && run.targetEnemies !== true)) return null;
  return {
    instanceId: run.instanceId,
    player: run.player,
    random: run.random === true,
    targetEnemies: run.targetEnemies === true,
    casts: 1,
  };
}

/** `drive`'s loop over the steps from `run.at`; true when the pipeline is finished with. */
function driveSteps(sink: EngineSink, run: PlayRun): boolean {
  for (let at = Math.max(0, run.at); at < STEP_TABLE.length; at += 1) {
    const step = STEP_TABLE[at];
    if (step === undefined) break;

    // Where a pause would pick up. A `repeats` step is re-entered at itself, because it holds its
    // own place inside the record — and the record is read when the pause is parked, after the step
    // has moved that cursor, never before.
    const next = step.repeats === true ? at : at + 1;
    // A card lost at step 4 is not played (R226): nothing resolves, and step 8 settles what steps 2
    // and 3 did (`PlayRun.lost`).
    // R448: a countered play is the same — nothing after its announce but step 8.
    if ((run.lost === true || run.countered === true) && step.name !== "settle") continue;
    step.run(sink, run);

    if (sink.state.result !== null) return true;
    if (sink.state.pending !== null) {
      // Nothing to owe when the last step is the one that paused, and nothing to owe for a prompt
      // this pipeline opened itself: that prompt's own `resume` already carries the record, so a
      // work item would be a second copy of the same continuation (§10.5 step 6's fresh picks).
      if (run.awaiting === null && next < STEP_TABLE.length) {
        pushWork(sink, resumeFor(run, next), run.player);
      }
      return false;
    }
  }

  return true;
}

/** File an answered selection in the bucket the pause was waiting on (§10.5 step 6). */
function fileSelection(run: PlayRun, selection: readonly Selection[]): void {
  const awaiting = run.awaiting;
  run.awaiting = null;
  if (awaiting === "castX") {
    // R453: the X a cast's caster chose, applied to the card as the step goes on (`castXChosen`).
    const picked = selection.find((pick) => pick.pick === "mode");
    const value = picked?.pick === "mode" ? Number.parseInt(picked.option, 10) : Number.NaN;
    if (Number.isInteger(value)) run.castX = value;
    return;
  }
  const repeat = run.repeat;
  if (repeat === null) return;
  if (awaiting === "echoTarget") repeat.targets.push(...selection);
  if (awaiting === "echoMode") {
    repeat.modes.push(...selection.flatMap((pick) => (pick.pick === "mode" ? [pick.option] : [])));
  }
}

/** The selection an answered prompt brought back, wherever the continuation carries it (§10.6). */
function selectionIn(data: Record<string, unknown>): Selection[] | null {
  const parked = pausedOf(data);
  if (parked !== null && parked.targets.length > 0) return parked.targets;
  for (const key of ["selection", "targets"]) {
    const raw = data[key];
    if (Array.isArray(raw) && raw.length > 0) return raw as Selection[];
  }
  return null;
}

/**
 * `work.ts`'s handler for the `"play"` sequence: the owed pipeline, continued where it stopped. An
 * answer to a prompt the pipeline opened itself does not come back this way: it goes to
 * `answerPlayPrompt`, which `prompts.answerPrompt` hands it to (R122).
 */
function runOwedPlay(sink: EngineSink, item: WorkItem): void {
  const run = runOf(item.resume);
  if (run === null) return;

  // One continuation per play: whichever of them runs first drops the others, so a play that is
  // both owed and named by an answered prompt does not run its tail twice.
  dropWork(
    sink.state,
    (other) =>
      other.id !== item.id &&
      isPlayResume(other.resume) &&
      runOf(other.resume)?.instanceId === run.instanceId,
  );

  const selection = selectionIn(item.resume.data);
  if (selection !== null) fileSelection(run, selection);
  drive(sink, run);
}

registerWorkHandler(PLAY_WORK_KIND, runOwedPlay);

/**
 * R70: a cast is a play, "free … with cost paid 0", so it runs this pipeline from step 3 — step 1
 * has nothing to validate, since the effect chose the card, and step 2 pays nothing. Everything else
 * is a play's: #64 Gifted Program's hook at step 3, the placement, counters, `cardPlayed` and the
 * Echo gained as it is played at step 4 (R178), #38 Quickstriker's and #78 /fullsend's granted Combo
 * parts before the card's own script at step 5, the repeats with fresh prompts at step 6 and the
 * landing and `cardResolved` at step 7. A step that asks parks the rest of the cast as an ordinary
 * `"play"` item at the moment it pauses (R113, R117), exactly as a play does.
 *
 * `resolve.castCard` is the entry point; it reaches this through the driver registered below,
 * because `resolve.ts` sits under `prompts.ts` and cannot import the pipeline itself.
 */
function castThroughPipeline(sink: EngineSink, instance: CardInstance, options: CastOptions): void {
  // The card an effect casts may be in no pile yet — drawn off the library (§2.4) or made from the
  // catalog (#95) — and the pipeline finds its card by id, so it waits in the resolving zone from
  // the start, as a card being played does (§10.5 step 4, R98).
  const state = sink.state;
  const player = instance.controller;
  removeFromAnyZone(state, instance);
  instance.zone = { z: "resolving", player };
  state.players[player].resolving.push(instance);
  // B5 E14, R546: a copier cast resolves the Spell that is last as the cast begins.
  fixCopiedText(state, instance);

  // R452: a cast made while a random cast of its caster's resolves is random too, and counts in that
  // cast's chain; a card that targets enemies (E39's enchantment) is cast so, whoever casts it.
  const random = options.random === true || randomCastOf(state, player) !== null;
  const targetEnemies = options.targetEnemies === true || hasEnchantment(instance, "targetEnemies");
  if (random) countChainCast(state, player);
  // R452, R453: a random cast's X is the caster's current mana, at least MIN_CHOSEN_X (R348); any
  // other cast's caster chooses it as the cast's first choice (`castXChosen`).
  if (random && choosesX(state, instance) && instance.x === undefined) {
    instance.x = Math.max(MIN_CHOSEN_X, state.players[player].mana.current);
  }
  // R453: "then exile it" (Classic #56) — step 7 lands the resolved Spell in exile (R178's mark).
  if (options.afterward === "exile") exileOnLanding(instance);

  const targets = [...(options.targets ?? [])];
  const modes = [...(options.modes ?? [])];
  const drawnAs: unknown = options.data?.[CAST_ON_DRAW_KEY];
  drive(sink, {
    ...(typeof drawnAs === "string" ? { drawnAs } : {}),
    instanceId: instance.id,
    defId: instance.defId,
    player,
    costPaid: 0,
    zone: null,
    targets,
    modes,
    ...slicesFor(sink.state, player, instance, 0, targets, modes),
    tributes: [],
    at: PLAY_STEPS.indexOf("giftedHook"),
    hookAt: 0,
    resolveAt: 0,
    echoQueued: false,
    repeat: null,
    awaiting: null,
    cast: true,
    exitsFrom: exitMark(sink.state),
    ...playBegins(sink.state, player),
    ...(random ? { random: true } : {}),
    ...(targetEnemies ? { targetEnemies: true } : {}),
    manaBefore: state.players[player].mana.current,
  });
}

registerCastDriver(castThroughPipeline);

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/**
 * §10.5, all eight steps: validate the play, then run steps 2 to 8, pausing wherever a prompt
 * opens. Returns the refusal, or null once the play has run (or has owed itself the rest).
 */
export function runPlaySteps(sink: EngineSink, player: PlayerId, action: PlayAction): string | null {
  const checked = validatePlay(sink, player, action);
  if ("error" in checked) return checked.error;
  interceptDeclaredTargets(sink, checked.run);
  drive(sink, checked.run);
  return null;
}

/**
 * B5 E5, E9, R450: the targeting point of a play's declared targets, at §10.5 step 1 once the play is
 * legal. An interceptor in the targeted player's hand (Classic #33) answers the first declared
 * `target` pick naming one of that player's units — a `by: "spell"` one only when the played card
 * is a Spell (R651) — if it would itself be a legal pick of that declaration, and the pick moves to
 * it. The costs the picks carry were checked at step 1 and are paid at step 2, whatever the
 * interception did (the targeting happened).
 */
function interceptDeclaredTargets(sink: EngineSink, run: PlayRun): void {
  if (run.targets.length === 0) return;
  const state = sink.state;
  const card = findInstance(state, run.instanceId);
  if (card === undefined) return;
  // R214: the face whose declarations the picks answer.
  const face = resolvingFace(state, run.player, card, run.costPaid);
  const decls = targetingDeclsOf(state, run.player, face, run.targets, run.modes);
  run.targets = interceptTargeting(sink, {
    chooser: run.player,
    picks: run.targets,
    targeting: (index) => decls[index] !== null && decls[index] !== undefined,
    accepts: (interceptor, index) => {
      const decl = decls[index];
      return decl !== null && decl !== undefined && interceptorFitsDecl(state, run.player, face, decl, interceptor);
    },
    // R651: a `by: "spell"` interceptor (Classic #33 Joro) answers only a Spell's declared target.
    source: cardTypeOf(state, card),
  });
}

/**
 * Answer a prompt this pipeline opened itself (§10.5 step 6's fresh picks, a cast's choices at step
 * 4): validate the answer the same way, close the prompt, file the selection and drive on. Such a
 * prompt names the `"play"` sequence as its hook, which no card script holds, so it is registered
 * with `prompts.answerPrompt` below: every caller answers through that one entry point, the reducer
 * and a caller driving the engine directly alike, and none of them drops the selection and the rest
 * of the play (R122, R113).
 *
 * Returns the refusal, or null.
 */
export function answerPlayPrompt(sink: EngineSink, answer: AnswerInput): string | null {
  const pending = sink.state.pending;
  if (pending === null) return "no prompt is open";

  const refused = whyAnswerRefused(pending, answer);
  if (refused !== null) return refused;

  const run = runOf(resumeOf(pending));
  if (run === null) return "that prompt is not a play's own";
  const picks = inOfferedOrder(pending, answer.selection);
  // B5 E5, R450: picks that cost more discards than the chooser holds are no answer.
  const unpaid = whyTargetAnswerRefused(sink.state, pending, picks);
  if (unpaid !== null) return unpaid;

  closePrompt(sink);

  // The answered prompt is this run's, so any tail owed for it earlier would repeat this step.
  dropWork(sink.state, (item) => isPlayResume(item.resume) && runOf(item.resume)?.instanceId === run.instanceId);
  // B5 E5, E9, R450: the targeting point of an Echo repeat's or a cast's fresh pick — a cost it asks
  // for first (its answer comes back through `continuePlayAnswer`), and an interception.
  const targeted = targetAnswer(sink, pending, picks);
  if (targeted === null) return null;
  continuePlayAnswer(sink, run, targeted);
  return null;
}

/** File an answered pick and drive the play on, finishing what the prompt interrupted (R113, R122). */
function continuePlayAnswer(sink: EngineSink, run: PlayRun, picks: readonly Selection[]): void {
  fileSelection(run, picks);
  // R113: taking the paused step up again resets the cursor, as `prompts.answerPrompt` does.
  beginWorkCascade(sink);
  drive(sink, run);
  // R122: the action that answers finishes what the prompt interrupted — the draw chain a cast's
  // own question stopped (§2.4), the steps a trap's play owes — before the resolution loop moves, as
  // `prompts.answerPrompt` does; otherwise the traps answered the cast's events before the draw
  // repeated into the card beneath it.
  drainWork(sink);
}

registerPromptAnswerer(PLAY_WORK_KIND, answerPlayPrompt);

// R450: a pick whose targeting cost was asked for first goes on here once the cost is paid.
registerTargetedContinuation(PLAY_WORK_KIND, (sink, prompt, picks) => {
  const run = runOf(resumeOf(prompt));
  if (run === null) return;
  dropWork(sink.state, (item) => isPlayResume(item.resume) && runOf(item.resume)?.instanceId === run.instanceId);
  continuePlayAnswer(sink, run, picks);
});
