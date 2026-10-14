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
// THE STRINGS. The module is an emscripten `MODULARIZE` factory (`crates/wasm`): its boundary is C,
// so a string crosses as a pointer. `put` copies one into the module (`stringToNewUTF8`, on the one
// emscripten heap Rust's allocator shares, so `jackioh_free` frees argument buffers and answers
// alike); `take` reads an answer back (`UTF8ToString`) and frees it. Every call runs inside
// `withArgs`, which frees every argument pointer after, thrown or returned. A binding that would
// have thrown answers NULL instead, and `take` throws the message the binding stowed for
// `jackioh_last_error` — which already names the binding, as the old thrown `Error` did.
//
// LOADING. Nothing here works until the module is instantiated:
//   - `loadWasm()` fetches its bytes (as an asset beside the glue, in the page's bundle and in the
//     practice worker's alike) and instantiates it (idempotent; one fetch however many callers).
//     The page starts it at boot, and `main.tsx`'s `EngineGate` holds every screen that calls the
//     engine until it resolves (#778), because the deck builder calls the validator synchronously;
//     the practice worker awaits it before it handles a message.
//   - `loadWasmSync(bytes)` instantiates it from bytes already in hand: jsdom has no fetch of a
//     file URL, so `test/setup.ts` reads `pkg/jackioh_wasm.wasm` from disk and hands it over.
// Both call the module's own `init()` once, which registers the catalog and the card scripts with the
// engine (what TypeScript's `registerAll()` did; nothing is left for the web to register).
//
// ERRORS. Where the TypeScript engine threw (a deck `createGame` refuses, a malformed argument), the
// binding answers NULL and the wrapper throws an `Error` with the engine's message. `reduce` and
// `beginGame` never throw on an illegal action: the refusal is `error` on the result. A panic is an
// engine invariant broken; the panic hook prints it on the console, and the call throws "the engine
// panicked".

import createModule from "./pkg/jackioh_wasm.js";
import type { JackiohWasmModule } from "./pkg/jackioh_wasm.js";

import type { Action, ActionBody, CardDefs, EmoteId, GameEvent, PlayerId, PlayerView, SetName } from "../wire/index.ts";
import type { CraftPreview } from "../wire/generated/CraftPreview.ts";
import type { CraftRecipe } from "../wire/generated/CraftRecipe.ts";
import type { CardInstance } from "../wire/generated/CardInstance.ts";
import type { DuosAction } from "../wire/generated/DuosAction.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosEvent } from "../wire/generated/DuosEvent.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosTeamRules } from "../wire/generated/DuosTeamRules.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
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

let mod: JackiohWasmModule | null = null;
let loaded = false;
let loading: Promise<void> | null = null;
const onLoad: (() => void)[] = [];

function ready(): void {
  if (loaded) return;
  if (mod === null) throw new Error("the engine is not instantiated");
  mod._init();
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
    loading = (async (): Promise<void> => {
      const response = await fetch(new URL("./pkg/jackioh_wasm.wasm", import.meta.url));
      if (!response.ok) throw new Error(`the engine could not be fetched: ${response.status}`);
      mod = await createModule({ wasmBinary: await response.arrayBuffer() });
      ready();
    })().catch((cause: unknown) => {
      loading = null;
      throw cause;
    });
  }
  return loading;
}

/**
 * Instantiate the module from its bytes (jsdom tests, `test/setup.ts`). Idempotent: a second call
 * resolves at once. Async: emscripten's factory always instantiates asynchronously.
 */
export function loadWasmSync(bytes: BufferSource): Promise<void> {
  if (loaded) return Promise.resolve();
  if (loading === null) {
    loading = (async (): Promise<void> => {
      mod = await createModule({ wasmBinary: bytes });
      ready();
    })().catch((cause: unknown) => {
      loading = null;
      throw cause;
    });
  }
  return loading;
}

/** Whether `loadWasm` or `loadWasmSync` has finished. */
export function wasmLoaded(): boolean {
  return loaded;
}

/** Every call goes through here: a clear error before loading, and a panic named as one. */
function call<T>(name: string, run: (m: JackiohWasmModule) => T): T {
  if (!loaded || mod === null) {
    throw new Error(`the engine is not loaded: await loadWasm() before calling ${name}`);
  }
  try {
    return run(mod);
  } catch (cause) {
    if (typeof WebAssembly !== "undefined" && cause instanceof WebAssembly.RuntimeError) {
      throw new Error(`the engine panicked in ${name} (the console has its message): ${cause.message}`, { cause });
    }
    throw cause;
  }
}

/**
 * Run `run` with an allocator for argument strings: every pointer it takes is freed after, thrown
 * or returned, through the module's one free function (see the header: arguments and answers share
 * the emscripten heap).
 */
function withArgs<T>(name: string, run: (m: JackiohWasmModule, put: (value: string) => number) => T): T {
  return call(name, (m) => {
    const owned: number[] = [];
    try {
      return run(m, (value) => {
        const ptr = m.stringToNewUTF8(value);
        owned.push(ptr);
        return ptr;
      });
    } finally {
      for (const ptr of owned) m._jackioh_free(ptr);
    }
  });
}

/**
 * Read the answer at `ptr` and free it. NULL is the binding's failure: throw the message it stowed
 * for `jackioh_last_error`, which names the binding already.
 */
function take(m: JackiohWasmModule, ptr: number): string {
  if (ptr === 0) {
    throw new Error(m.UTF8ToString(m._jackioh_last_error()));
  }
  const text = m.UTF8ToString(ptr);
  m._jackioh_free(ptr);
  return text;
}

/** A status answer: 0 is success, anything else throws the stowed message. */
function check(m: JackiohWasmModule, status: number): void {
  if (status !== 0) {
    throw new Error(m.UTF8ToString(m._jackioh_last_error()));
  }
}

function json(value: unknown): string {
  return JSON.stringify(value);
}

/** The no-break space `stateJson` puts ahead of a state. */
const STATE_MARK = "\u00A0";

/**
 * A state as it crosses: `["\u00a0",<the state's JSON>]`, which the bindings unwrap (`crates/wasm`'s
 * `state_of`). The mark dates to the `wasm-bindgen` glue and buys nothing on emscripten's
 * `stringToNewUTF8`, which walks the whole string either way; it stays so both sides keep one
 * shape, and no caller above this file ever sees it.
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
  return withArgs("createGame", (m, put) => parsed<GameState>(take(m, m._create_game(put(json(args))))));
}

export function beginGame(state: GameState): ReduceResult {
  return withArgs("beginGame", (m, put) => reduceResult(take(m, m._begin_game(put(stateJson(state)))), state));
}

export function reduce(state: GameState, action: Action): ReduceResult {
  return withArgs("reduce", (m, put) =>
    reduceResult(take(m, m._reduce(put(stateJson(state)), put(json(action)))), state),
  );
}

/** Everything `player` may legally do now. The client greys out the rest (BUILD M5-T2). */
export function legalActions(state: GameState, player: PlayerId): ActionBody[] {
  return withArgs("legalActions", (m, put) =>
    parsed<ActionBody[]>(take(m, m._legal_actions(put(stateJson(state)), put(player)))),
  );
}

/** The only window the client has onto the game (SPEC §10.8). */
export function viewFor(state: GameState, player: PlayerId): PlayerView {
  return withArgs("viewFor", (m, put) =>
    parsed<PlayerView>(take(m, m._view_for(put(stateJson(state)), put(player)))),
  );
}

/** The seat that owes the next action, or null when none does. */
export function seatToAct(state: GameState): PlayerId | null {
  return withArgs("seatToAct", (m, put) => {
    const seat = take(m, m._seat_to_act(put(stateJson(state))));
    return seat === "" ? null : (seat as PlayerId);
  });
}

/** SURFACE §5.2's hash: eight hex digits, TypeScript's `hashState` bit for bit. */
export function hashState(state: GameState): string {
  return withArgs("hashState", (m, put) => take(m, m._hash_state(put(stateJson(state)))));
}

/** Fold a recorded log from scratch (SPEC §9.2, §9.3). Throws on a setup `createGame` refuses. */
export function fold(input: FoldInput): FoldResult {
  return withArgs("fold", (m, put) => parsed<FoldResult>(take(m, m._fold(put(json(input))))));
}

/** R768: a finished game's replay, its log checked against its catalog version and final hash. */
export function replayOpen(input: FoldInput, record: ReplayRecord): ReplayOpen {
  return withArgs("replayOpen", (m, put) =>
    parsed<ReplayOpen>(take(m, m._replay_open(put(json(input)), put(json(record))))),
  );
}

/** R768: steps [from, from + count) as `seat` saw them; continue at `from + steps.length`. */
export function replayPage(checkpoints: ReplayCheckpoints, seat: PlayerId, from: number, count: number): ReplayPage {
  return withArgs("replayPage", (m, put) =>
    parsed<ReplayPage>(take(m, m._replay_page(put(json(checkpoints)), put(seat), from, count))),
  );
}

/** R417, R508: the board `seat` takes away. */
export function lastBoardFor(state: GameState, seat: PlayerId): LastBoardCard[] {
  return withArgs("lastBoardFor", (m, put) =>
    parsed<LastBoardCard[]>(take(m, m._last_board_for(put(stateJson(state)), put(seat)))),
  );
}

/** R677: the seat the player who began in `home` plays now. */
export function seatPlayedBy(state: GameState, home: PlayerId): PlayerId {
  return withArgs("seatPlayedBy", (m, put) =>
    take(m, m._seat_played_by(put(stateJson(state)), put(home))) as PlayerId,
  );
}

/** The card with this instance id, wherever it is, or undefined (TypeScript's `findInstance`). */
export function findInstance(state: GameState, instanceId: string): CardInstance | undefined {
  return withArgs("findInstance", (m, put) =>
    parsed<CardInstance | null>(take(m, m._find_instance(put(stateJson(state)), put(instanceId)))) ?? undefined,
  );
}

/** R1341: the emote hand `seat` is dealt in the game seeded `seed`, as the server's actor deals it. */
export function dealEmoteHand(seed: string, seat: PlayerId): EmoteId[] {
  return withArgs("dealEmoteHand", (m, put) =>
    parsed<EmoteId[]>(take(m, m._deal_emote_hand(put(seed), put(seat)))),
  );
}

/**
 * R1420: from now on the module treats `sets` as sets that ship (a deck may hold their cards, a pool
 * that names no set reaches them), as the testkit previews a set on a test's thread. Only a module
 * built with crates/wasm's `preview` feature can (scripts/build-wasm.sh; the production bundle's is
 * built without it and throws). The hotseat route calls it for its E2E injection's `preview` alone.
 */
export function previewSets(sets: readonly SetName[]): void {
  withArgs("previewSets", (m, put) => {
    check(m, m._preview_sets(put(json(sets))));
  });
}

let catalogCache: CardDefs | null = null;

/** The registered catalog (TypeScript's `registeredCatalog()`): the same object on every call. */
export function registeredCatalog(): CardDefs {
  return withArgs("registeredCatalog", (m) => {
    catalogCache ??= parsed<CardDefs>(take(m, m._catalog()));
    return catalogCache;
  });
}

/** The catalog version compiled into the module (the newest entry of patches.json). */
export function catalogVersion(): string {
  return withArgs("catalogVersion", (m) => take(m, m._catalog_version()));
}

let carriedCache: readonly string[] | null = null;

/**
 * R767000: the older shipped versions the module carries, oldest first, so a practice game played on
 * one of them replays under its own cards: those from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM`
 * on. The same array on every call.
 */
export function carriedVersions(): readonly string[] {
  return withArgs("carriedVersions", (m) => {
    carriedCache ??= parsed<string[]>(take(m, m._carried_versions()));
    return carriedCache;
  });
}

// ---------------------------------------------------------------------------------------------
// Duos co-op (#936 S4, S8): the wrapper over two duels, for `/dev/duos`'s four-seat hotseat
// ---------------------------------------------------------------------------------------------

declare const duosStateBrand: unique symbol;

/**
 * The engine's `DuosState` (two `GameState`s and the wrapper's own fields), opaque: it holds all four
 * hands and libraries, so the client passes it back and never reads it (CLAUDE.md rule 7). It crosses
 * in the same envelope as a `GameState` (`stateJson`).
 */
export type DuosState = { readonly [duosStateBrand]: never };

/** `duosCreate`'s options (the engine's `DuosCreateOptions`): four decks, one per seat. */
export type DuosCreateArgs = {
  seed: string;
  decks: { a1: readonly string[]; a2: readonly string[]; b1: readonly string[]; b2: readonly string[] };
  /** Deal from these definitions instead of the registered catalog (tests). */
  catalog?: CardDefs;
  /** Each team's rules; absent, DUOS_HUMAN_TEAM for both. */
  teams?: { a: DuosTeamRules; b: DuosTeamRules };
  /** §9.9's per-seat handicaps; an absent seat is a human's. */
  handicaps?: Partial<Record<DuosSeat, Handicap>>;
  /** R433: the seats whose deck was dealt rather than built. */
  dealt?: readonly DuosSeat[];
};

/** `{ state, events, error? }`: `error` is the engine's refusal, and then `state` is the input. */
export type DuosReduceResult = { state: DuosState; events: DuosEvent[]; error?: string };

function duosStateJson(state: DuosState): string {
  return JSON.stringify([STATE_MARK, state]);
}

/** As `reduceResult`: a refusal hands back the caller's own state object and no events. */
function duosReduceResult(text: string, input: DuosState): DuosReduceResult {
  const result = parsed<{ state: DuosState; events: DuosEvent[]; error?: string | null }>(text);
  if (result.error === undefined || result.error === null) return { state: result.state, events: result.events };
  return { state: input, events: [], error: result.error };
}

/** A begun-able Duos game: two duels, seeded `<seed>:duel0` and `<seed>:duel1`. Throws on a deck it refuses. */
export function duosCreate(args: DuosCreateArgs): DuosState {
  return withArgs("duosCreate", (m, put) => parsed<DuosState>(take(m, m._duos_create(put(json(args))))));
}

/** Both duels' setups, all four mulligans open at once (B1, B2). */
export function duosBegin(state: DuosState): DuosReduceResult {
  return withArgs("duosBegin", (m, put) => duosReduceResult(take(m, m._duos_begin(put(duosStateJson(state)))), state));
}

/** One seat's action; an illegal one is refused in `error`, never thrown. */
export function duosReduce(state: DuosState, action: DuosAction): DuosReduceResult {
  return withArgs("duosReduce", (m, put) =>
    duosReduceResult(take(m, m._duos_reduce(put(duosStateJson(state)), put(json(action)))), state),
  );
}

/** Everything `seat` may do now: its duel's moves wrapped as `duel`, its passes, its concede. */
export function duosLegalActions(state: DuosState, seat: DuosSeat): DuosActionBody[] {
  return withArgs("duosLegalActions", (m, put) =>
    parsed<DuosActionBody[]>(take(m, m._duos_legal_actions(put(duosStateJson(state)), put(seat)))),
  );
}

/** The seat's `DuosView`: its own duel, its teammate's from the teammate's seat, both teams' life (B23). */
export function duosViewFor(state: DuosState, seat: DuosSeat): DuosView {
  return withArgs("duosViewFor", (m, put) =>
    parsed<DuosView>(take(m, m._duos_view_for(put(duosStateJson(state)), put(seat)))),
  );
}

// ---------------------------------------------------------------------------------------------
// the AI (SURFACE §9)
// ---------------------------------------------------------------------------------------------

export function aiToAct(state: GameState, seat: PlayerId): boolean {
  return withArgs("aiToAct", (m, put) => {
    const answer = m._ai_to_act(put(stateJson(state)), put(seat));
    if (answer < 0) throw new Error(m.UTF8ToString(m._jackioh_last_error()));
    return answer !== 0;
  });
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
  return withArgs("decide", (m, put) =>
    parsed<DecideResult>(take(m, m._ai_decide(put(stateJson(state)), put(seat), put(json(request)), deadlineMs))),
  );
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
  /** R1370: a set at least `AI_DECK.leanMinShare` of the deck comes from; absent, none. */
  leanSet?: SetName;
  /** R1390: the sets the draw never deals from; default the AI's soft gate, [] for a human's random deck. */
  gatedSets?: readonly SetName[];
};

export function buildAiDeck(request: AiDeckRequest): { deck: string[]; rngCursor: number } {
  return withArgs("buildAiDeck", (m, put) =>
    parsed<{ deck: string[]; rngCursor: number }>(take(m, m._build_ai_deck(put(json(request))))),
  );
}

/** §10.7's random policy (`subsystems.chooseAction`), drawing from `(rngSeed, rngCursor)`. */
export function chooseAction(state: GameState, seat: PlayerId, stream: StreamAt): { action: ActionBody | null; rngCursor: number } {
  return withArgs("chooseAction", (m, put) =>
    parsed<{ action: ActionBody | null; rngCursor: number }>(
      take(m, m._choose_action(put(stateJson(state)), put(seat), put(stream.rngSeed), stream.rngCursor)),
    ),
  );
}

/**
 * ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost — a
 * `CraftPreview` — so the block editor shows what the reducer will say and the client decides
 * nothing (CLAUDE.md rule 7).
 */
export function craftPreview(recipe: CraftRecipe, cost: number): CraftPreview {
  return withArgs("craftPreview", (m, put) =>
    parsed<CraftPreview>(take(m, m._craft_preview(put(json(recipe)), cost))),
  );
}

/** The deck builder's numbers (`crates/ai/src/deck.rs`'s `AI_DECK`, SPEC §9.9, R1370), as it states them. */
export type AiDeckConstants = {
  curve: Record<"0-1" | "2" | "3" | "4+", number>;
  curveShiftPerMana: number;
  curveTolerance: number;
  minUnitShare: number;
  themeChance: number;
  minThemeSize: number;
  themeBoost: number;
  themeMinShare: number;
  curveBoost: number;
  curveOverflow: number;
  unitBoost: number;
  uncastable: number;
  costSlack: number;
  minPool: number;
  /** R1370: a deck that leans on a set holds at least ceil(size × leanMinShare) of its cards. */
  leanMinShare: number;
  /** R1370: weight for a card of the leaned set while the deck is short of it. */
  leanBoost: number;
  /** R817000: a random deck holds at most this many Quickdraw cards. */
  maxQuickdraw: number;
};

export type AiConstants = {
  AI_BUDGET: SearchBudget;
  AI_GATE_BUDGET: SearchBudget;
  SHADOW_BAN_IDS: readonly string[];
  AI_DECK: AiDeckConstants;
};

let constantsCache: AiConstants | null = null;

/** The AI's budgets, its shadow-ban ids and its deck builder's numbers, as the Rust AI states them. */
export function aiConstants(): AiConstants {
  return withArgs("constants", (m) => {
    constantsCache ??= parsed<AiConstants>(take(m, m._constants()));
    return constantsCache;
  });
}

/** The engine's tables the web's tests read through `subsystems`. */
export type EngineTables = {
  chaosEffects: readonly { label: string }[];
  chaosPlusEffects: readonly { label: string }[];
  chaosMedEffects: readonly { label: string }[];
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
  return withArgs("engineTables", (m) => {
    tablesCache ??= parsed<EngineTables>(take(m, m._engine_tables()));
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
  return withArgs(name, (m, put) => parsed<T>(take(m, m._validator(put(name), put(json(input))))));
}
