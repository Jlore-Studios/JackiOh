// The TypeScript wrapper over the WebAssembly module (docs/v0.3.0/SURFACE.md §10.3): the one file
// in apps/web that touches `./pkg`, which `scripts/build-wasm.sh` builds from `crates/wasm` (and git
// ignores). Everything that runs a rule on the client runs it through here: the hotseat port
// (`game/engine.ts`), the practice worker (`practice/core.ts`), the deck builder's validator and the
// `@jackioh/*` aliases (`wire/engine.ts`, `wire/ai.ts`, `wire/validator.ts`).
//
// JSON strings in and out, nothing else crosses (SURFACE §10.1). Each function below parses and
// stringifies, and is named as the TypeScript engine named it, so a caller reads exactly as it did:
// `createGame`, `beginGame`, `reduce`, `legalActions`, `viewFor`, `hashState`, `fold`, … The state is
// the engine's `GameState` as plain JSON; the client keeps it behind its opaque `EngineState` brand
// and never reads it (CLAUDE.md rule 7, SPEC §10.8).
//
// LOADING. Nothing here works until the module is instantiated:
//   - `loadWasm()` fetches and instantiates it (idempotent; one fetch however many callers). The page
//     awaits it in `main.tsx` before the first render, because the deck builder calls the validator
//     synchronously; the practice worker awaits it before it handles a message.
//   - `loadWasmSync(bytes)` instantiates it from bytes already in hand: jsdom has no fetch of a file
//     URL, so `test/setup.ts` reads `pkg/jackioh_wasm_bg.wasm` from disk and hands it over.
// Both call the module's own `init()` once, which registers the catalog and the card scripts with the
// engine (what TypeScript's `registerAll()` did; nothing is left for the web to register).
//
// ERRORS. Where the TypeScript engine threw (a deck `createGame` refuses, a malformed argument), the
// binding throws an `Error` with the engine's message. `reduce` and `beginGame` never throw on an
// illegal action: the refusal is `error` on the result. A panic is an engine invariant broken; the
// panic hook prints it on the console, and the call throws "the engine panicked".

import initWasm, {
  ai_decide,
  ai_to_act,
  begin_game,
  build_ai_deck,
  catalog,
  catalog_version,
  choose_action,
  constants,
  create_game,
  engine_tables,
  find_instance,
  fold as fold_log,
  hash_state,
  init as init_engine,
  initSync,
  last_board_for,
  legal_actions,
  reduce as reduce_action,
  replay_open,
  replay_page,
  seat_played_by,
  seat_to_act,
  validator as validator_call,
  view_for,
} from "./pkg/jackioh_wasm.js";

import type { Action, ActionBody, CardDefs, GameEvent, PlayerId, PlayerView } from "../wire/index.ts";
import type { CardInstance } from "../wire/generated/CardInstance.ts";
import type { GameState } from "../wire/generated/GameState.ts";
import type { ReplayCheckpoints } from "../wire/generated/ReplayCheckpoints.ts";
import type { ReplayOpen } from "../wire/generated/ReplayOpen.ts";
import type { ReplayPage } from "../wire/generated/ReplayPage.ts";
import type { ReplayRecord } from "../wire/generated/ReplayRecord.ts";
import type { Handicap } from "../wire/engineConfig.ts";
import type { Decision, SearchBudget } from "../wire/ai.ts";

// ---------------------------------------------------------------------------------------------
// loading
// ---------------------------------------------------------------------------------------------

let loaded = false;
let loading: Promise<void> | null = null;
const onLoad: (() => void)[] = [];

function ready(): void {
  if (loaded) return;
  init_engine();
  loaded = true;
  for (const run of onLoad.splice(0)) run();
}

/**
 * Run `run` once the module is loaded: now, if it is. For a module that must not touch the engine
 * while it is being imported (the practice worker imports everything before its first message
 * awaits `loadWasm`), but exports values the engine states (`wire/ai.ts`'s budgets).
 */
export function whenWasmLoaded(run: () => void): void {
  if (loaded) run();
  else onLoad.push(run);
}

/**
 * Fetch and instantiate the module, then register the catalog and the card scripts. Idempotent: a
 * second call, or a call after `loadWasmSync`, resolves at once. A failed fetch is not cached; the
 * next call tries again.
 */
export function loadWasm(): Promise<void> {
  if (loaded) return Promise.resolve();
  if (loading === null) {
    // The literal `new URL(…, import.meta.url)` is what Vite recognises and emits as an asset, in
    // the page's bundle and in the practice worker's alike.
    loading = initWasm({ module_or_path: new URL("./pkg/jackioh_wasm_bg.wasm", import.meta.url) })
      .then(() => {
        ready();
      })
      .catch((cause: unknown) => {
        loading = null;
        throw cause;
      });
  }
  return loading;
}

/** Instantiate the module from its bytes (jsdom tests, `test/setup.ts`). Idempotent. */
export function loadWasmSync(bytes: BufferSource): void {
  if (loaded) return;
  initSync({ module: bytes });
  ready();
}

/** Whether `loadWasm` or `loadWasmSync` has finished. */
export function wasmLoaded(): boolean {
  return loaded;
}

/** Every call goes through here: a clear error before loading, and a panic named as one. */
function call<T>(name: string, run: () => T): T {
  if (!loaded) {
    throw new Error(`the engine is not loaded: await loadWasm() before calling ${name}`);
  }
  try {
    return run();
  } catch (cause) {
    if (typeof WebAssembly !== "undefined" && cause instanceof WebAssembly.RuntimeError) {
      throw new Error(`the engine panicked in ${name} (the console has its message): ${cause.message}`, { cause });
    }
    throw cause;
  }
}

function json(value: unknown): string {
  return JSON.stringify(value);
}

/** The no-break space `stateJson` puts ahead of a state. */
const STATE_MARK = "\u00A0";

/**
 * A state as it crosses: `["\u00A0",<the state's JSON>]`, which the bindings unwrap (`crates/wasm`'s
 * `state_of`). wasm-bindgen's glue copies a string into the module one character at a time up to its
 * first non-ASCII character and hands the rest to `TextEncoder.encodeInto`, and a state's JSON is all
 * ASCII, so its ~80 KB went through the per-character loop on every call (0.18 ms of a 0.24 ms crossing,
 * measured in Node). The no-break space third in this text sends all but two characters through
 * `encodeInto`. One `JSON.stringify` writes it, so no second string is built, and it stays one byte a
 * character (a mark outside Latin-1, or one added with `+`, cost more than it saved).
 */
function stateJson(state: GameState): string {
  return JSON.stringify([STATE_MARK, state]);
}

function parsed<T>(text: string): T {
  return JSON.parse(text) as T;
}

// ---------------------------------------------------------------------------------------------
// the engine (SURFACE §6.1)
// ---------------------------------------------------------------------------------------------

/** R417, R508: one card of a seat's last board, as `createGame` takes it and `lastBoardFor` gives it. */
export type LastBoardCard = { defId: string; radiant: boolean };

/** `createGame`'s options (the engine's `CreateGameOptions`). */
export type CreateGameArgs = {
  seed: string;
  /** Two decks of card ids, in library order; the engine shuffles them with the match rng. */
  decks: readonly [readonly string[], readonly string[]];
  /** Deal from these definitions instead of the registered catalog (tests). */
  catalog?: CardDefs;
  /** R180: per-seat handicaps; an omitted seat, or one equal to a human's, stores nothing. */
  handicaps?: Partial<Record<PlayerId, Handicap>>;
  /** R433: the seats whose deck was dealt rather than built. */
  dealt?: readonly PlayerId[];
  /** R417: each seat's last board, in seat order. */
  lastBoards?: readonly [readonly LastBoardCard[], readonly LastBoardCard[]];
  /** R678: the boards of two other games a Glitch may put on the field, in seat order. */
  glitchBoards?: readonly [readonly LastBoardCard[], readonly LastBoardCard[]];
};

/** `{ state, events, error? }`: `error` is the engine's refusal, and then `state` is the input. */
export type ReduceResult = { state: GameState; events: GameEvent[]; error?: string };

/** `fold`'s input: `createGame`'s setup and the action log. */
export type FoldInput = CreateGameArgs & { log: readonly Action[] };

/** `fold`'s answer: the state the log folds to, and every action the engine refused on the way. */
export type FoldResult = { state: GameState; errors: { nonce: string; error: string }[] };

/** Rust skips an absent `error`; the client tests `error !== undefined`, so a `null` never survives. */
function reduceResult(text: string, input: GameState): ReduceResult {
  const result = parsed<{ state: GameState; events: GameEvent[]; error?: string | null }>(text);
  if (result.error === undefined || result.error === null) return { state: result.state, events: result.events };
  // A refusal leaves the state as it was: hand back the caller's own object, as TypeScript did.
  return { state: input, events: [], error: result.error };
}

export function createGame(args: CreateGameArgs): GameState {
  return call("createGame", () => parsed<GameState>(create_game(json(args))));
}

export function beginGame(state: GameState): ReduceResult {
  return call("beginGame", () => reduceResult(begin_game(stateJson(state)), state));
}

export function reduce(state: GameState, action: Action): ReduceResult {
  return call("reduce", () => reduceResult(reduce_action(stateJson(state), json(action)), state));
}

/** Everything `player` may legally do now. The client greys out the rest (BUILD M5-T2). */
export function legalActions(state: GameState, player: PlayerId): ActionBody[] {
  return call("legalActions", () => parsed<ActionBody[]>(legal_actions(stateJson(state), player)));
}

/** The only window the client has onto the game (SPEC §10.8). */
export function viewFor(state: GameState, player: PlayerId): PlayerView {
  return call("viewFor", () => parsed<PlayerView>(view_for(stateJson(state), player)));
}

/** The seat that owes the next action, or null when none does. */
export function seatToAct(state: GameState): PlayerId | null {
  return call("seatToAct", () => {
    const seat = seat_to_act(stateJson(state));
    return seat === "" ? null : (seat as PlayerId);
  });
}

/** SURFACE §5.2's hash: eight hex digits, TypeScript's `hashState` bit for bit. */
export function hashState(state: GameState): string {
  return call("hashState", () => hash_state(stateJson(state)));
}

/** Fold a recorded log from scratch (SPEC §9.2, §9.3). Throws on a setup `createGame` refuses. */
export function fold(input: FoldInput): FoldResult {
  return call("fold", () => parsed<FoldResult>(fold_log(json(input))));
}

/** R768: a finished game's replay, its log checked against its catalog version and final hash. */
export function replayOpen(input: FoldInput, record: ReplayRecord): ReplayOpen {
  return call("replayOpen", () => parsed<ReplayOpen>(replay_open(json(input), json(record))));
}

/** R768: steps [from, from + count) as `seat` saw them; continue at `from + steps.length`. */
export function replayPage(checkpoints: ReplayCheckpoints, seat: PlayerId, from: number, count: number): ReplayPage {
  return call("replayPage", () => parsed<ReplayPage>(replay_page(json(checkpoints), seat, from, count)));
}

/** R417, R508: the board `seat` takes away. */
export function lastBoardFor(state: GameState, seat: PlayerId): LastBoardCard[] {
  return call("lastBoardFor", () => parsed<LastBoardCard[]>(last_board_for(stateJson(state), seat)));
}

/** R677: the seat the player who began in `home` plays now. */
export function seatPlayedBy(state: GameState, home: PlayerId): PlayerId {
  return call("seatPlayedBy", () => seat_played_by(stateJson(state), home) as PlayerId);
}

/** The card with this instance id, wherever it is, or undefined (TypeScript's `findInstance`). */
export function findInstance(state: GameState, instanceId: string): CardInstance | undefined {
  return call("findInstance", () => parsed<CardInstance | null>(find_instance(stateJson(state), instanceId)) ?? undefined);
}

let catalogCache: CardDefs | null = null;

/** The registered catalog (TypeScript's `registeredCatalog()`): the same object on every call. */
export function registeredCatalog(): CardDefs {
  return call("registeredCatalog", () => {
    catalogCache ??= parsed<CardDefs>(catalog());
    return catalogCache;
  });
}

/** The catalog version compiled into the module (the newest entry of patches.json). */
export function catalogVersion(): string {
  return call("catalogVersion", () => catalog_version());
}

// ---------------------------------------------------------------------------------------------
// the AI (SURFACE §9)
// ---------------------------------------------------------------------------------------------

export function aiToAct(state: GameState, seat: PlayerId): boolean {
  return call("aiToAct", () => ai_to_act(stateJson(state), seat));
}

/** The AI's own stream as it crosses: its seed and where it stands. */
export type StreamAt = { rngSeed: string; rngCursor: number };

export type DecideRequest = StreamAt & { budget?: SearchBudget };

/** `decide`'s answer, and the cursor the AI's stream stopped at. */
export type DecideResult = { decision: Decision | null; rngCursor: number };

/**
 * One AI decision. `deadlineMs` is a `Date.now()` time after which the search answers with its best
 * so far (TypeScript's `shouldStop`); 0 or less is no clock, so the decision depends only on the
 * state, the stream and the budget.
 */
export function decide(state: GameState, seat: PlayerId, request: DecideRequest, deadlineMs: number): DecideResult {
  return call("decide", () => parsed<DecideResult>(ai_decide(stateJson(state), seat, json(request), deadlineMs)));
}

/** `buildAiDeck`'s options as they cross (`AiDeckOptions` plus the stream and the size). */
export type AiDeckRequest = StreamAt & {
  size: number;
  /** Default the shadow ban; [] for a human's random deck. */
  banned?: readonly string[];
  /** The seat's handicap manaCap; default MAX_MANA. */
  manaCap?: number;
  /** A tag to lean on; absent = roll one, null = none. */
  theme?: string | null;
  /** Ids forced in (the sweep). */
  include?: readonly string[];
  /** R390: these ids' weights multiplied by `by`. */
  boost?: { ids: readonly string[]; by: number };
};

export function buildAiDeck(request: AiDeckRequest): { deck: string[]; rngCursor: number } {
  return call("buildAiDeck", () => parsed<{ deck: string[]; rngCursor: number }>(build_ai_deck(json(request))));
}

/** §10.7's random policy (`subsystems.chooseAction`), drawing from `(rngSeed, rngCursor)`. */
export function chooseAction(state: GameState, seat: PlayerId, stream: StreamAt): { action: ActionBody | null; rngCursor: number } {
  return call("chooseAction", () =>
    parsed<{ action: ActionBody | null; rngCursor: number }>(choose_action(stateJson(state), seat, stream.rngSeed, stream.rngCursor)),
  );
}

export type AiConstants = {
  AI_BUDGET: SearchBudget;
  AI_GATE_BUDGET: SearchBudget;
  SHADOW_BAN_IDS: readonly string[];
};

let constantsCache: AiConstants | null = null;

/** The AI's budgets and its shadow-ban ids, as the Rust AI states them. */
export function aiConstants(): AiConstants {
  return call("constants", () => {
    constantsCache ??= parsed<AiConstants>(constants());
    return constantsCache;
  });
}

/** The engine's tables the web's tests read through `subsystems`. */
export type EngineTables = {
  chaosEffects: readonly { label: string }[];
  chaosPlusEffects: readonly { label: string }[];
  heroPowerNames: readonly string[];
  heroPowers: readonly {
    name: string;
    x: number;
    title: string;
    radiantTitle: string;
    label: string;
    radiantLabel: string;
  }[];
};

let tablesCache: EngineTables | null = null;

export function engineTables(): EngineTables {
  return call("engineTables", () => {
    tablesCache ??= parsed<EngineTables>(engine_tables());
    return tablesCache;
  });
}

// ---------------------------------------------------------------------------------------------
// the validator (SPEC §9.4: one module, client and server)
// ---------------------------------------------------------------------------------------------

/** The validator's calls, by their TypeScript names. */
export type ValidatorCall =
  | "validateDeck"
  | "validateTrio"
  | "validateLoadout"
  | "checkDeckDraft"
  | "checkTrioDraft"
  | "checkImportRoom"
  | "trioConflicts"
  | "normalizeName";

/** One validator call: the TypeScript function's argument in, its result out (`wire/validator.ts`). */
export function validator<T>(name: ValidatorCall, input: unknown): T {
  return call(name, () => parsed<T>(validator_call(name, json(input))));
}
