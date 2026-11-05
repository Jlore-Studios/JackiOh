// The wrapper over the WebAssembly module (docs/v0.3.0/SURFACE.md §10.3), the one file in apps/web that
// touches `./pkg` (`scripts/build-wasm.sh` builds it from `crates/wasm`). Every rule the client runs goes
// through here, as JSON strings in and out (SURFACE §10.1); the client keeps the state behind its opaque
// `EngineState` brand and never reads it (CLAUDE.md rule 7, SPEC §10.8).
//
// LOADING. Nothing works until `loadWasm()` (which `main.tsx`'s `EngineGate` awaits before any screen
// that calls the engine, #778, as the deck builder calls the validator synchronously) or `loadWasmSync`
// has run the module's `init()`, which registers the catalog and the card scripts.
//
// ERRORS. A refused deck or a malformed argument throws the engine's message; `reduce` and `beginGame`
// answer an illegal action with `error` on the result. A panic (an engine invariant broken) is printed
// on the console by the panic hook, and the call throws "the engine panicked".

import initWasm, { initSync } from "./pkg/jackioh_wasm.js";
import * as normal from "./pkg/jackioh_wasm.js";

import type { Action, ActionBody, CardDefs, EmoteId, GameEvent, PlayerId, PlayerView, SetName } from "../wire/index.ts";
import type { CraftPreview } from "../wire/generated/CraftPreview.ts";
import type { CraftRecipe } from "../wire/generated/CraftRecipe.ts";
import type { PendingPromptView } from "../wire/generated/PendingPromptView.ts";
import type { DeckMark } from "../wire/generated/DeckMark.ts";
import type { WinRateTable } from "../wire/generated/WinRateTable.ts";
import type { CardInstance } from "../wire/generated/CardInstance.ts";
import type {
  RunAction,
  RunConstants,
  RunContent,
  RunFightSetup,
  RunFile,
  RunSetup,
  RunState,
  RunUnlocks,
} from "@jackioh/run";
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

// loading

let loaded = false;
let loading: Promise<void> | null = null;
const onLoad: (() => void)[] = [];

function ready(): void {
  if (loaded) return;
  normal.init();
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
 * Idempotent: a second call, or a call after `loadWasmSync`, resolves at once. A failed fetch is not
 * cached; the next call tries again.
 */
export function loadWasm(): Promise<void> {
  if (loaded) return Promise.resolve();
  if (loading === null) {
    // The literal `new URL(…, import.meta.url)` is what Vite recognizes and emits as an asset, in
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

const STATE_MARK = "\u00A0";

/**
 * A state as it crosses: `["\u00A0",<the state's JSON>]`, which `crates/wasm`'s `state_of` unwraps.
 * wasm-bindgen's glue copies a string in one character at a time up to its first non-ASCII one, which
 * cost an all-ASCII state 0.18 ms of a 0.24 ms crossing (Node); the mark, third, sends the rest through
 * `TextEncoder.encodeInto`. A mark outside Latin-1, or one added with `+`, cost more than it saved.
 */
function stateJson(state: GameState): string {
  return JSON.stringify([STATE_MARK, state]);
}

function parsed<T>(text: string): T {
  return JSON.parse(text) as T;
}

// the engine (SURFACE §6.1)

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
  /** R923000: a practice sandbox, whose dev tools (`sandbox` actions) only it accepts. */
  sandbox?: boolean;
  /** R2702000: each seat's deck marks, cards of its list dealt Radiant or Buffed at setup. Omitted, nothing is marked or stored. */
  deckMarks?: Partial<Record<PlayerId, readonly DeckMark[]>>;
  /** R3049000: the win-rate snapshot CN Tech and NA Tech read (the public `/api/stats/cards` figures); omitted, the game reads none and the cards fall back. */
  winRates?: WinRateTable;
};

/** `error` is the engine's refusal, and then `state` is the input. */
export type ReduceResult = { state: GameState; events: GameEvent[]; error?: string };

export type FoldInput = CreateGameArgs & { log: readonly Action[] };

/** A finished or unfinished game's record, as `runFightReport` reads it. */
export type ReplayInput = FoldInput;

/** `fold`'s answer: the state the log folds to, and every action the engine refused on the way. */
export type FoldResult = { state: GameState; errors: { nonce: string; error: string }[] };

/** Rust skips an absent `error`; the client tests `error !== undefined`, so a `null` never survives. */
function reduceResult(text: string, input: GameState): ReduceResult {
  const result = parsed<{ state: GameState; events: GameEvent[]; error?: string | null }>(text);
  if (result.error === undefined || result.error === null) return { state: result.state, events: result.events };
  // A refusal leaves the state as it was: hand back the caller's own object.
  return { state: input, events: [], error: result.error };
}

export function createGame(args: CreateGameArgs): GameState {
  return call("createGame", () => parsed<GameState>(normal.create_game(json(args))));
}

export function beginGame(state: GameState): ReduceResult {
  return call("beginGame", () => reduceResult(normal.begin_game(stateJson(state)), state));
}

export function reduce(state: GameState, action: Action): ReduceResult {
  return call("reduce", () => reduceResult(normal.reduce(stateJson(state), json(action)), state));
}

/** Everything `player` may legally do now. The client greys out the rest (BUILD M5-T2). */
export function legalActions(state: GameState, player: PlayerId): ActionBody[] {
  return call("legalActions", () => parsed<ActionBody[]>(normal.legal_actions(stateJson(state), player)));
}

/** The only window the client has onto the game (SPEC §10.8). */
export function viewFor(state: GameState, player: PlayerId): PlayerView {
  return call("viewFor", () => parsed<PlayerView>(normal.view_for(stateJson(state), player)));
}

/** The seat that owes the next action, or null when none does. */
export function seatToAct(state: GameState): PlayerId | null {
  return call("seatToAct", () => {
    const seat = normal.seat_to_act(stateJson(state));
    return seat === "" ? null : (seat as PlayerId);
  });
}

/** SURFACE §5.2's hash: eight hex digits. */
export function hashState(state: GameState): string {
  return call("hashState", () => normal.hash_state(stateJson(state)));
}

/** Fold a recorded log from scratch (SPEC §9.2, §9.3). Throws on a setup `createGame` refuses. */
export function fold(input: FoldInput): FoldResult {
  return call("fold", () => parsed<FoldResult>(normal.fold(json(input))));
}

/** R768: a finished game's replay, its log checked against its catalog version and final hash. */
export function replayOpen(input: FoldInput, record: ReplayRecord): ReplayOpen {
  return call("replayOpen", () => parsed<ReplayOpen>(normal.replay_open(json(input), json(record))));
}

/** R768: steps [from, from + count) as `seat` saw them; continue at `from + steps.length`. */
export function replayPage(checkpoints: ReplayCheckpoints, seat: PlayerId, from: number, count: number): ReplayPage {
  return call("replayPage", () => parsed<ReplayPage>(normal.replay_page(json(checkpoints), seat, from, count)));
}

/** R417, R508: the board `seat` takes away. */
export function lastBoardFor(state: GameState, seat: PlayerId): LastBoardCard[] {
  return call("lastBoardFor", () => parsed<LastBoardCard[]>(normal.last_board_for(stateJson(state), seat)));
}

/** R677: the seat the player who began in `home` plays now. */
export function seatPlayedBy(state: GameState, home: PlayerId): PlayerId {
  return call("seatPlayedBy", () => normal.seat_played_by(stateJson(state), home) as PlayerId);
}

export function findInstance(state: GameState, instanceId: string): CardInstance | undefined {
  return call("findInstance", () => parsed<CardInstance | null>(normal.find_instance(stateJson(state), instanceId)) ?? undefined);
}

/** R1341: the emote hand `seat` is dealt in the game seeded `seed`, as the server's actor deals it. */
export function dealEmoteHand(seed: string, seat: PlayerId): EmoteId[] {
  return call("dealEmoteHand", () => parsed<EmoteId[]>(normal.deal_emote_hand(seed, seat)));
}

/**
 * R1420: from now on the module treats `sets` as sets that ship. Only a module built with crates/wasm's
 * `preview` feature can (scripts/build-wasm.sh; the production bundle's throws). The hotseat route
 * calls it for its E2E injection's `preview` alone.
 */
export function previewSets(sets: readonly SetName[]): void {
  call("previewSets", () => {
    normal.preview_sets(json(sets));
  });
}

let catalogCache: CardDefs | null = null;

/** The registered catalog: the same object on every call. */
export function registeredCatalog(): CardDefs {
  return call("registeredCatalog", () => {
    catalogCache ??= parsed<CardDefs>(normal.catalog());
    return catalogCache;
  });
}

/**
 * R1011000: the learned evaluation's parity fixture, its integer network's correction for each case.
 * Only the tests call it.
 */
export function learnedParity(fixtureJson: string): number[] {
  return call("learnedParity", () => parsed<number[]>(normal.learned_parity(fixtureJson)));
}

/** The catalog version compiled into the module (the newest entry of patches.json). */
export function catalogVersion(): string {
  return call("catalogVersion", () => normal.catalog_version());
}

let carriedCache: readonly string[] | null = null;

/**
 * R767000: the older shipped versions the module carries, oldest first, so a practice game played on
 * one of them replays under its own cards: those from `crates/cards/build.rs`'s `BROWSER_CARRIED_FROM`
 * on. The same array on every call.
 */
export function carriedVersions(): readonly string[] {
  return call("carriedVersions", () => {
    carriedCache ??= parsed<string[]>(normal.carried_versions());
    return carriedCache;
  });
}

// The dungeon run (docs/dungeon-run.md DR14 part C)

/**
 * `runCreate`, `runReduce` and `runFold`'s answer: the run, or the run's refusal, never thrown
 * (only malformed JSON throws, as everywhere else here).
 */
export type RunResult = { state: RunState } | { error: string };

/** `RunContent`: `run_catalog_content` over the compiled-in catalog. */
export function runContent(): RunContent {
  return call("runContent", () => parsed<RunContent>(normal.run_content("[]")));
}

/** Whether the setup may start a run: null, or the reason. */
export function runCheckSetup(setup: RunSetup, unlocks: RunUnlocks, content: RunContent): string | null {
  return call("runCheckSetup", () =>
    parsed<string | null>(normal.run_check_setup(json(setup), json(unlocks), json(content))),
  );
}

/** Start a run: its state, or the setup's refusal. */
export function runCreate(setup: RunSetup, unlocks: RunUnlocks, content: RunContent): RunResult {
  return call("runCreate", () =>
    parsed<RunResult>(normal.run_create(json(setup), json(unlocks), json(content))),
  );
}

/** The run after one action, or the action's refusal. */
export function runReduce(state: RunState, content: RunContent, action: RunAction): RunResult {
  return call("runReduce", () =>
    parsed<RunResult>(normal.run_reduce(json(state), json(content), json(action))),
  );
}

/** A run file folded from its start, or the first refused step. */
export function runFold(file: RunFile, content: RunContent): RunResult {
  return call("runFold", () => parsed<RunResult>(normal.run_fold(json(file), json(content))));
}

/** The fight being played, or null outside one. */
export function runFightSetup(state: RunState, content: RunContent): RunFightSetup | null {
  return call("runFightSetup", () =>
    parsed<RunFightSetup | null>(normal.run_fight_setup(json(state), json(content))),
  );
}

/** The `FightOver` a finished fight sends, or why the game reports none. */
export function runFightReport(game: ReplayInput, human: PlayerId): { action: RunAction } | { error: string } {
  return call("runFightReport", () =>
    parsed<{ action: RunAction } | { error: string }>(normal.run_fight_report(json(game), json(human))),
  );
}

/** The run's score so far. */
export function runScore(state: RunState, content: RunContent): number {
  return call("runScore", () => parsed<number>(normal.run_score(json(state), json(content))));
}

/** The Embers a score pays. */
export function runEmbers(score: number): number {
  return call("runEmbers", () => parsed<number>(normal.run_embers(json(score))));
}

/** The run's max HP, before no fight and after every artifact and blessing. */
export function runMaxHp(state: RunState, content: RunContent): number {
  return call("runMaxHp", () => parsed<number>(normal.run_max_hp(json(state), json(content))));
}

/** The Embers that raise the starting deck from `level` to the next, or null at the ladder's top. */
export function runLevelCost(level: number): number | null {
  return call("runLevelCost", () => parsed<number | null>(normal.run_level_cost(json(level))));
}

/** The run's numbers the screens show, so the client states none (CLAUDE.md rule 9). */
export function runConstants(): RunConstants {
  return call("runConstants", () => parsed<RunConstants>(normal.run_constants()));
}

/** A seed of capital letters drawn from any text. */
export function runSeed(source: string): string {
  return call("runSeed", () => parsed<string>(normal.run_seed(json(source))));
}

/** The daily run's seed: the date's and the day's run number. */
export function runDailySeed(date: string, n: number): string {
  return call("runDailySeed", () => parsed<string>(normal.run_daily_seed(json(date), json(n))));
}

// Duos co-op (#936 S4, S8): the wrapper over two duels, for `/dev/duos`'s four-seat hotseat

declare const duosStateBrand: unique symbol;

/**
 * The engine's `DuosState`, opaque: it holds all four hands and libraries, so the client passes it back
 * and never reads it (CLAUDE.md rule 7). It crosses in the same envelope as a `GameState` (`stateJson`).
 */
export type DuosState = { readonly [duosStateBrand]: never };

/** `duosCreate`'s options (the engine's `DuosCreateOptions`). */
export type DuosCreateArgs = {
  seed: string;
  decks: { a1: readonly string[]; a2: readonly string[]; b1: readonly string[]; b2: readonly string[] };
  catalog?: CardDefs;
  /** Each team's rules; absent, DUOS_HUMAN_TEAM for both. */
  teams?: { a: DuosTeamRules; b: DuosTeamRules };
  /** §9.9's per-seat handicaps; an absent seat is a human's. */
  handicaps?: Partial<Record<DuosSeat, Handicap>>;
  dealt?: readonly DuosSeat[];
};

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

// The Duos bindings are in the module only when `crates/wasm`'s `duos` feature is on, which only the
// dev server's build turns on.

/** A begun-able Duos game: two duels, seeded `<seed>:duel0` and `<seed>:duel1`. Throws on a deck it refuses. */
export function duosCreate(args: DuosCreateArgs): DuosState {
  return call("duosCreate", () => parsed<DuosState>(normal.duos_create(json(args))));
}

/** Both duels' setups, all four mulligans open at once (B1, B2). */
export function duosBegin(state: DuosState): DuosReduceResult {
  return call("duosBegin", () => duosReduceResult(normal.duos_begin(duosStateJson(state)), state));
}

/** One seat's action; an illegal one is refused in `error`, never thrown. */
export function duosReduce(state: DuosState, action: DuosAction): DuosReduceResult {
  return call("duosReduce", () => duosReduceResult(normal.duos_reduce(duosStateJson(state), json(action)), state));
}

/** Everything `seat` may do now: its duel's moves wrapped as `duel`, its passes, its concede. */
export function duosLegalActions(state: DuosState, seat: DuosSeat): DuosActionBody[] {
  return call("duosLegalActions", () => parsed<DuosActionBody[]>(normal.duos_legal_actions(duosStateJson(state), seat)));
}

/** The seat's `DuosView`: its own duel, its teammate's from the teammate's seat, both teams' life (B23). */
export function duosViewFor(state: DuosState, seat: DuosSeat): DuosView {
  return call("duosViewFor", () => parsed<DuosView>(normal.duos_view_for(duosStateJson(state), seat)));
}

// the AI (SURFACE §9)

export function aiToAct(state: GameState, seat: PlayerId): boolean {
  return call("aiToAct", () => normal.ai_to_act(stateJson(state), seat));
}

export type StreamAt = { rngSeed: string; rngCursor: number };

export type DecideRequest = StreamAt & { budget?: SearchBudget };

export type DecideResult = { decision: Decision | null; rngCursor: number };

/**
 * One AI decision. `deadlineMs` is a `Date.now()` time after which the search answers with its best
 * so far; 0 or less is no clock, so the decision depends only on the state, the stream and the budget.
 */
export function decide(state: GameState, seat: PlayerId, request: DecideRequest, deadlineMs: number): DecideResult {
  return call("decide", () => parsed<DecideResult>(normal.ai_decide(stateJson(state), seat, json(request), deadlineMs)));
}

/**
 * R2102000: whether `seat` has a sure win this turn by what it can see (`jackioh_ai::lethal_hint`).
 * `deadlineMs` is a `Date.now()` time, as `decide`'s is: a check still running past it says no; 0, no clock.
 */
export function lethalHint(state: GameState, seat: PlayerId, deadlineMs: number): boolean {
  return call("lethalHint", () => normal.lethal_hint(stateJson(state), seat, deadlineMs));
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
  return call("buildAiDeck", () => parsed<{ deck: string[]; rngCursor: number }>(normal.build_ai_deck(json(request))));
}

/** §10.7's random policy (`subsystems.chooseAction`), drawing from `(rngSeed, rngCursor)`. */
export function chooseAction(state: GameState, seat: PlayerId, stream: StreamAt): { action: ActionBody | null; rngCursor: number } {
  return call("chooseAction", () =>
    parsed<{ action: ActionBody | null; rngCursor: number }>(normal.choose_action(stateJson(state), seat, stream.rngSeed, stream.rngCursor)),
  );
}

/**
 * ME-CRAFT (Meditative #17, R880): the engine's own verdict on a recipe at the chosen cost, so the block
 * editor shows what the reducer will say and the client decides nothing (CLAUDE.md rule 7).
 */
export function craftPreview(
  recipe: CraftRecipe,
  cost: number,
  prompt: Pick<PendingPromptView, "craftMode" | "craftOffer"> = {},
): CraftPreview {
  // R1563007: the open prompt's mode and offer, so the verdict is the one its answer gets.
  const slice = { craftMode: prompt.craftMode, craftOffer: prompt.craftOffer };
  return call("craftPreview", () => parsed<CraftPreview>(normal.craft_preview(json(recipe), cost, json(slice))));
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
  /** Issue #1113: the frozen fixture pool, a copy of SHADOW_BAN_IDS as it stood when the fixtures were dealt. */
  FIXTURE_BANNED_IDS: readonly string[];
  AI_DECK: AiDeckConstants;
};

let constantsCache: AiConstants | null = null;

/** The AI's budgets, its shadow-ban ids and its deck builder's numbers, as the Rust AI states them. */
export function aiConstants(): AiConstants {
  return call("constants", () => {
    constantsCache ??= parsed<AiConstants>(normal.constants());
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
  return call("engineTables", () => {
    tablesCache ??= parsed<EngineTables>(normal.engine_tables());
    return tablesCache;
  });
}

// the validator (SPEC §9.4: one module, client and server)

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
  return call(name, () => parsed<T>(normal.validator(name, json(input))));
}
