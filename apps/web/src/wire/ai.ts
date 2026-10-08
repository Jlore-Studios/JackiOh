// `@jackioh/ai` and `@jackioh/ai/config` for the web client (docs/v0.3.0/SURFACE.md §10.4): the
// practice opponent (SPEC §9.9) as the client reaches it, with the TypeScript signatures the
// practice core, the tutorial harness and their tests were written against.
//
// The AI is Rust (`crates/ai`), compiled into the WebAssembly module with the engine; `decide`,
// `aiToAct` and `buildAiDeck` below call it through `../wasm`. Its budgets and its shadow ban are the
// Rust AI's own values, read from the module once it is loaded (so the client never restates them,
// CLAUDE.md rule 9), and so are its deck builder's numbers (`AI_DECK`). The emote personas (R645) are presentation, not play, and stay TypeScript:
// `../practice/personas.ts`, re-exported here so `practice/emotes.ts` reads them as it did.
//
// STREAMS. TypeScript's AI drew from an `Rng` object the caller held. The Rust AI draws from the
// stream rebuilt from that object's `(seed, cursor)` and hands back the cursor it stopped at; each
// wrapper then draws the caller's object forward to it, so `rng.cursor` reads afterwards exactly as
// it did (the practice save keeps it, R668).
//
// THE CLOCK. TypeScript's `shouldStop` was a callback, and a callback cannot cross into WebAssembly.
// `decide` takes `deadlineMs` instead, a `Date.now()` time after which the search answers with its
// best so far; absent, there is no clock, and the decision depends only on the state, the stream and
// the budget.

import {
  aiConstants,
  aiToAct as wasmAiToAct,
  buildAiDeck as wasmBuildAiDeck,
  decide as wasmDecide,
  whenWasmLoaded,
} from "../wasm/index.ts";
import { advanceTo, type Rng } from "./rng.ts";

import type { AiDeckConstants } from "../wasm/index.ts";
import type { GameState } from "./generated/GameState.ts";
import type { ActionBody, PlayerId, SetName } from "./index.ts";

export {
  AI_EMOTE,
  AI_PERSONAS,
  EMOTE_REPLY_KEYS,
  EMOTE_TRIGGERS,
  createEmotePersona,
  pickPersona,
  replyKeyOf,
  rollForReply,
  rollForTrigger,
  type AiEmote,
  type EmotePersona,
  type EmoteReplyKey,
  type EmoteTrigger,
  type PersonaName,
  type PersonaSpec,
} from "../practice/personas.ts";

// ---------------------------------------------------------------------------------------------
// the AI's types (the Rust AI's `SearchBudget`, `Decision` and friends, as JSON)
// ---------------------------------------------------------------------------------------------

/** A decision's budget. "Node" = one engine `reduce` call made by the AI, in any determinization. */
export type SearchBudget = {
  /** reduce() calls the whole decision may make, lethal solver and opponent auto-answers included. */
  nodes: number;
  /** Of `nodes`, the most the lethal solver may spend before the beam starts. */
  lethalNodes: number;
  /** Determinizations sampled per decision (K). */
  determinizations: number;
  /** Open lines kept per depth. */
  beamWidth: number;
  /** Children expanded at the root, after move ordering (endTurn always kept on top of this). */
  rootBranching: number;
  /** Children expanded per open line below the root (endTurn always kept on top of this). */
  branching: number;
  /** Actions in one planned line, endTurn included. */
  maxDepth: number;
  /** Best complete lines on determinization 0 that are re-scored on every other determinization. */
  finalists: number;
};

export type AiOptions = {
  /** The AI's own stream; determinize is its only consumer. Drawn forward to where the AI left it. */
  rng: Rng;
  /** Default AI_BUDGET. */
  budget?: SearchBudget;
  /** A `Date.now()` time: past it, the search stops and answers with the best so far. Absent, no clock. */
  deadlineMs?: number;
};

export type DecisionReason = "forced" | "mulligan" | "draw-offer" | "lethal" | "prompt" | "search" | "fallback";

export type SearchStats = {
  nodes: number;
  determinizations: number;
  /** Complete lines scored on determinization 0. */
  lines: number;
  /** Simulated reduce calls that threw or were refused; never escape `decide`. */
  simErrors: number;
  stoppedBy: "exhausted" | "budget" | "clock";
  /** Mean score of the chosen line across determinizations (0 for forced/mulligan/draw-offer). */
  score: number;
};

export type Decision = {
  action: ActionBody;
  reason: DecisionReason;
  /** The planned line this action starts. Only line[0] is ever played; the AI re-plans after it. */
  line: ActionBody[];
  stats: SearchStats;
};

export type AiDeckOptions = {
  /** Default SHADOW_BAN_IDS; pass [] for a human's random deck. */
  banned?: readonly string[];
  /** Ids forced in (the sweep); must be non-token and not banned. */
  include?: readonly string[];
  /** A tag to lean on; undefined = roll one (AI_DECK.themeChance), null = none. */
  theme?: string | null;
  /** The seat's handicap manaCap; default MAX_MANA. Shifts the curve and the uncastable test. */
  manaCap?: number;
  /** R390: these ids' weights are multiplied by `by` (the sweep's pass 2, as `themeBoost` leans a theme). */
  boost?: { ids: readonly string[]; by: number };
  /**
   * R1370: a set at least AI_DECK.leanMinShare of the deck comes from (a hard floor); absent, none,
   * and the deck is the one the seed always dealt. "More cards from the newest set" passes
   * `newestShippedSet()` (R1371, R1373).
   */
  leanSet?: SetName;
};

export type { AiDeckConstants };

// ---------------------------------------------------------------------------------------------
// the Rust AI's own numbers, once the module is loaded
// ---------------------------------------------------------------------------------------------

/**
 * The practice AI's budget (SPEC §9.9): the same at every difficulty (R180). Like the two below, a
 * live binding the module fills as soon as it is loaded: the practice worker imports this file
 * before its first message awaits `loadWasm`, and reads the value only after.
 */
export let AI_BUDGET: SearchBudget;

/** The quality gates' budget, and the shadow-ban sweep's: the browser's own. */
export let AI_GATE_BUDGET: SearchBudget;

/** R186: the ids the AI never deals itself, sorted. */
export let SHADOW_BAN_IDS: readonly string[];

/** SPEC §9.9, R1370: the deck builder's numbers, `leanMinShare` among them (CLAUDE.md rule 9). */
export let AI_DECK: AiDeckConstants;

whenWasmLoaded(() => {
  const constants = aiConstants();
  AI_BUDGET = constants.AI_BUDGET;
  AI_GATE_BUDGET = constants.AI_GATE_BUDGET;
  SHADOW_BAN_IDS = constants.SHADOW_BAN_IDS;
  AI_DECK = constants.AI_DECK;
});

// ---------------------------------------------------------------------------------------------
// the AI's entry points
// ---------------------------------------------------------------------------------------------

/** R187, R188: whether `seat` owes an action the AI should take now. */
export function aiToAct(state: GameState, seat: PlayerId): boolean {
  return wasmAiToAct(state, seat);
}

/**
 * One decision for `seat` (SPEC §9.9). Reads the true state only through `aiToAct` and `redact`
 * (R185), on the Rust side. Null when the AI owes nothing.
 */
export function decide(state: GameState, seat: PlayerId, options: AiOptions): Decision | null {
  const { rng } = options;
  const { decision, rngCursor } = wasmDecide(
    state,
    seat,
    {
      rngSeed: rng.seed,
      rngCursor: rng.cursor,
      ...(options.budget === undefined ? {} : { budget: options.budget }),
    },
    options.deadlineMs ?? 0,
  );
  advanceTo(rng, rngCursor);
  return decision;
}

/** A deck of `size` ids for a seat (R186's shadow ban unless `banned` says otherwise), drawn from `rng`. */
export function buildAiDeck(rng: Rng, size: number, options: AiDeckOptions = {}): string[] {
  const { deck, rngCursor } = wasmBuildAiDeck({ rngSeed: rng.seed, rngCursor: rng.cursor, size, ...options });
  advanceTo(rng, rngCursor);
  return deck;
}
