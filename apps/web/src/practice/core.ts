// The practice game itself: the one `GameState`, both seats' actions and the AI (SPEC §9.9, R187).
//
// This is the ONLY practice file that imports `@jackioh/engine`, `@jackioh/cards` or `@jackioh/ai`
// — the Rust engine and AI, compiled to WebAssembly and reached through `src/wire/` (docs/v0.3.0/
// SURFACE.md §10.4) — and only two things load it: the worker entry (`practice.worker.ts`) and the
// in-thread host that jsdom tests use (`host.ts`), each of which loads the module (`loadWasm`)
// before its first request. The page never does. That is the whole of rule 7 here: the state,
// both hands and both libraries live on this side of `handle`, and what crosses it is a
// `PracticeSnapshot` — `viewFor(state, human)`, the human's `legalActions`, whether the AI owes an
// action, and the engine's refusal of the human's last action. The worker stands where §9.1 puts
// the server.
//
// Nonces: `h<n>` for the human's accepted actions and `a<n>` for the AI's, each counting only
// actions the engine accepted, so a refused action spends nothing and the log folds exactly
// (`fold({ seed, decks, handicaps, lastBoards, dealt, log })`, R187, R508, R433). The AI draws from its own stream,
// `createRng(`${seed}:ai`)`, kept for the whole game; the match rng in state is never touched by it.
// The Rust AI draws from that stream as `(seed, cursor)` and hands the cursor back, and `decide`
// draws this object forward to it (`wire/ai.ts`), so `rng.cursor` is where the AI left it.
//
// Resume (R668). The worker dies with the page, so after every answer a free game in progress is
// written to the env's save store (`saveStore.ts`, IndexedDB in a worker), as the match actor's log
// outlives a server restart: the config, the log, the AI stream's cursor, the catalog version and
// the state's hash. The save never crosses to the page, whose log would name the AI's hidden cards.
// After a reload the page sends `resume` with the setup it remembers, and the log is folded again
// through `apply`, action by action, under the same nonces, to the same hash; the AI picks its
// stream up at the saved cursor.
//
// Replays (R768). A free game that has just ended (not a tutorial lesson, and not one Glitch voided,
// R679) is kept in the same store, the newest PRACTICE_REPLAYS_KEPT of them: its config, its log,
// the catalog version, the final state's hash, the seat the human ended in and a summary for a list.
// `replays` answers the summaries, newest first, each marked unavailable with part 1's reason when it
// was played on another catalog or no longer folds to its hash; `replay` answers a page of the human's
// steps, `viewFor` of the seat the human played at each step (R677), and the total. Both are refused
// while a game is in progress. Like the save, a kept game never crosses to the page: a response holds
// summaries and views, never the log, the seed, the decks or a state.

import { CATALOG_VERSION } from "@jackioh/cards";
import {
  beginGame,
  createGame,
  createRng,
  hashState,
  lastBoardFor,
  legalActions,
  reduce,
  registeredCatalog,
  replayOpen,
  replayPage,
  seatPlayedBy,
  viewFor,
  type FoldInput,
  type GameState,
  type ReplayOpen,
  type ReplayRefusal,
  type Rng,
} from "@jackioh/engine";
import { AI_DIFFICULTY, AI_TUTORIAL, DECK_SIZE, type Handicap } from "@jackioh/engine/config";
import { AI_BUDGET, aiToAct, buildAiDeck, decide, type SearchBudget } from "@jackioh/ai";
import { DEFAULT_PORTRAIT, newestShippedSet, opponentOf, type Action, type ActionBody, type PlayerId } from "@jackioh/shared";

import { PRACTICE_AI_CLOCK_MS, PRACTICE_REPLAYS_KEPT } from "./config.ts";
import { lessonById } from "../tutorial/lessons.ts";
import { presetById } from "./decks.ts";
import type { PracticeReplay, PracticeSave, PracticeSaveStore } from "./saveStore.ts";
import type {
  LastBoardCard,
  PracticeDebug,
  PracticeDeckChoice,
  PracticeReplayListing,
  PracticeRequest,
  PracticeResponse,
  PracticeSnapshot,
  PracticeStartConfig,
} from "./protocol.ts";

export type PracticeCoreEnv = {
  /**
   * The wall clock the AI's cap is measured on, in `Date.now()` terms (worker: `Date.now`), because
   * that is the clock the WebAssembly side reads against the deadline. A clock that reads 0 is no
   * clock: the tests' frozen `() => 0` never stops a search, so a decision is its budget's alone. It
   * is also the time a finished game is kept at (R768).
   */
  now: () => number;
  /** answer `debug` only when true */
  dev: boolean;
  /** default AI_BUDGET */
  budget?: SearchBudget;
  /** R668, R768: where a free game in progress and the finished ones kept for replay live; absent, nothing is kept, no resume folds and nothing is replayed. */
  saves?: PracticeSaveStore;
};

/** `handle` never throws: anything that goes wrong comes back as `"failed"`. */
export type PracticeCore = { handle(request: PracticeRequest): PracticeResponse };

type PracticeGame = {
  config: PracticeStartConfig;
  /** The seat the AI began in; after a Glitch swap it plays the other one (`aiSeatNow`, R677). */
  aiSeat: PlayerId;
  decks: [string[], string[]];
  handicaps: Partial<Record<PlayerId, Handicap>>;
  /** R508: the human's last board as `createGame` had it, seat ordered; absent when none. */
  lastBoards?: [LastBoardCard[], LastBoardCard[]];
  /** R433: the seats `createGame` was told were dealt (the human's, on the random deck); absent when none. */
  dealt?: PlayerId[];
  state: GameState;
  log: Action[];
  /** The AI's own stream, `${seed}:ai`, for the whole game. */
  rng: Rng;
  humanCount: number;
  aiCount: number;
  /** The engine's refusal of the human's last action, until the board moves on. */
  error: string | null;
  /** R768: its end has been kept (or passed over), so it is kept once. */
  kept: boolean;
};

/** R188: the action types the AI never takes on its own, even as a fallback. */
function isForbiddenAiAction(action: ActionBody): boolean {
  // MD-D29, R1127: the persona's emote is no AI decision — it arrives on its own request, and the
  // random policy never emotes, so the gate answers it, never the search.
  if (action.type === "concede" || action.type === "offerDraw" || action.type === "emote") return true;
  return action.type === "answerDraw" && action.accept;
}

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/**
 * The human's deck: a random draw at the spec's own size, a named preset's list, or a saved
 * loadout deck. `createGame` checks a preset or a saved deck like any other (§2.6): the engine's
 * ruling, printed as given. R1373: a random deck the player asked to lean on the newest set leans on
 * the newest set that ships (R1371), resolved here, where the deck is dealt; the AI's own deck never
 * leans. A player's random deck is dealt from every set: neither the AI's shadow ban nor its soft gate
 * (R1390) applies.
 */
function humanDeckFor(seed: string, choice: PracticeDeckChoice): string[] {
  switch (choice.kind) {
    case "random":
      return buildAiDeck(createRng(`${seed}:human-deck`), DECK_SIZE, {
        banned: [],
        gatedSets: [],
        ...(choice.leanNewest === true ? { leanSet: newestShippedSet() } : {}),
      });
    case "preset": {
      const preset = presetById(choice.id);
      if (preset === undefined) throw new Error(`unknown preset deck "${choice.id}"`);
      return [...preset.cards];
    }
    case "saved":
      return [...choice.cards];
  }
}

/**
 * §9.10, R291: a lesson's two fixed decks and the tutorial handicap (R290), or the free game's: the
 * difficulty's handicap, the chosen deck and a dealt AI deck (§9.9).
 */
function decksAndHandicap(config: PracticeStartConfig): { humanDeck: string[]; aiDeck: string[]; handicap: Handicap } {
  if (config.lesson !== undefined) {
    const lesson = lessonById(config.lesson);
    if (lesson === undefined) throw new Error(`unknown tutorial lesson "${config.lesson}"`);
    return { humanDeck: [...lesson.humanDeck], aiDeck: [...lesson.aiDeck], handicap: AI_TUTORIAL };
  }
  const handicap = AI_DIFFICULTY[config.difficulty];
  return {
    humanDeck: humanDeckFor(config.seed, config.deck),
    aiDeck: buildAiDeck(createRng(`${config.seed}:ai-deck`), handicap.deckSize, { manaCap: handicap.manaCap }),
    handicap,
  };
}

/**
 * R433: the seats whose deck was dealt rather than built. A free game on the random deck deals the
 * human theirs (`humanDeckFor`'s "random"), so the human is shown none of it going in. The AI's
 * deck is never counted dealt: no view shows its seat, and R185's redaction would hide the AI's own
 * library from itself. A lesson plays its fixed decks, whatever `deck` says.
 */
function dealtSeats(config: PracticeStartConfig): PlayerId[] {
  return config.lesson === undefined && config.deck.kind === "random" ? [config.humanSeat] : [];
}

function startGame(config: PracticeStartConfig): PracticeGame {
  const aiSeat = opponentOf(config.humanSeat);
  const { humanDeck, aiDeck, handicap } = decksAndHandicap(config);
  const decks: [string[], string[]] = config.humanSeat === "p1" ? [humanDeck, aiDeck] : [aiDeck, humanDeck];
  const handicaps: Partial<Record<PlayerId, Handicap>> = { [aiSeat]: { ...handicap } };
  // R508: the human's last board, on a free game only; the AI's seat never has one.
  const board = config.lesson === undefined ? (config.lastBoard ?? []) : [];
  const lastBoards: [LastBoardCard[], LastBoardCard[]] | undefined =
    board.length === 0 ? undefined : config.humanSeat === "p1" ? [[...board], []] : [[], [...board]];
  const dealt = dealtSeats(config);

  // `createGame` throws on an illegal deck and `beginGame` reports a refusal in `error`; either way
  // there is no game, and `handle` turns the throw into "failed".
  const created = createGame({
    seed: config.seed,
    decks,
    handicaps,
    ...(lastBoards === undefined ? {} : { lastBoards }),
    ...(dealt.length === 0 ? {} : { dealt }),
  });
  const begun = beginGame(created);
  if (begun.error !== undefined) throw new Error(`the engine refused to begin the game: ${begun.error}`);

  return {
    config: { ...config },
    aiSeat,
    decks: [[...decks[0]], [...decks[1]]],
    handicaps,
    ...(lastBoards === undefined ? {} : { lastBoards }),
    ...(dealt.length === 0 ? {} : { dealt }),
    state: begun.state,
    log: [],
    rng: createRng(`${config.seed}:ai`),
    humanCount: 0,
    aiCount: 0,
    error: null,
    kept: false,
  };
}

/** R677: the seat the human plays now: the one it began in, or the AI's after an odd number of Glitch swaps. */
function humanSeatNow(game: PracticeGame): PlayerId {
  return seatPlayedBy(game.state, game.config.humanSeat);
}

/** R677: the seat the AI plays now. */
function aiSeatNow(game: PracticeGame): PlayerId {
  return seatPlayedBy(game.state, game.aiSeat);
}

/**
 * R677, R768: the seat the human played at step `from` of a kept game, and how many steps from
 * there, up to `count`, it went on playing it. Every logged action was sent as its player's seat
 * at the step it was taken from (`h…` the human's, `a…` the AI's), and the last step's seat is the
 * one the game ended in, so a page stops where a Glitch swap moved the human.
 */
export function replaySeatRun(log: readonly Action[], endSeat: PlayerId, from: number, count: number): { seat: PlayerId; count: number } {
  const seatAt = (step: number): PlayerId => {
    const action = log[step];
    if (action === undefined) return endSeat;
    return action.nonce.startsWith("h") ? action.playerId : opponentOf(action.playerId);
  };
  const seat = seatAt(from);
  let run = 1;
  while (run < count && from + run <= log.length && seatAt(from + run) === seat) run += 1;
  return { seat, count: run };
}

/**
 * One action for the human or the AI, in the seat it plays now (R677); null when the engine
 * accepted it, else the engine's reason. The nonce counts by player, not by seat, so a swap leaves
 * both streams unbroken.
 */
function apply(game: PracticeGame, body: ActionBody, human: boolean): string | null {
  const seat = human ? humanSeatNow(game) : aiSeatNow(game);
  const nonce = human ? `h${String(game.humanCount)}` : `a${String(game.aiCount)}`;
  const action = { ...body, playerId: seat, nonce } as Action;

  let result: ReturnType<typeof reduce>;
  try {
    result = reduce(game.state, action);
  } catch (cause) {
    return `the engine failed on ${body.type}: ${messageOf(cause)}`;
  }
  if (result.error !== undefined) return result.error;

  game.state = result.state;
  game.log.push(action);
  if (human) game.humanCount += 1;
  else game.aiCount += 1;
  return null;
}

function snapshotOf(game: PracticeGame): PracticeSnapshot {
  // R677: the human sees, and acts from, the seat it plays now; the result reads from it too.
  const human = humanSeatNow(game);
  const result = game.state.result;
  // R508: the board the human takes away, once a free game is over; R679: a voided game leaves none.
  const keepsBoard = result !== null && result.reason !== "voided" && game.config.lesson === undefined;
  return {
    view: viewFor(game.state, human),
    legal: legalActions(game.state, human),
    aiToAct: aiToAct(game.state, aiSeatNow(game)),
    error: game.error,
    ...(keepsBoard ? { lastBoard: lastBoardFor(game.state, human) } : {}),
  };
}

/**
 * R668: the save a free game in progress leaves after each answer; none for a lesson (its coach
 * reads the game from its first snapshot on, so a lesson restarts instead) or a finished game.
 */
function saveOf(game: PracticeGame): PracticeSave | null {
  if (game.config.lesson !== undefined || game.state.result !== null) return null;
  return {
    catalog: CATALOG_VERSION,
    config: JSON.parse(JSON.stringify(game.config)) as PracticeStartConfig,
    log: JSON.parse(JSON.stringify(game.log)) as Action[],
    aiCursor: game.rng.cursor,
    hash: hashState(game.state),
  };
}

/**
 * R668: a save folded back into its game. The log is replayed through `apply`, so every action must
 * be accepted under the nonce it was accepted under, and the folded state must hash as it did when
 * saved; a save from another catalog, or one that does not fold to the same game, is refused.
 */
function resumeGame(save: PracticeSave | null, asked: PracticeStartConfig): PracticeGame {
  if (save === null) throw new Error("no practice game is saved on this device");
  // The page names the game it remembers; a save of any other game is not the one it asked for.
  const { seed, humanSeat, difficulty } = save.config;
  if (seed !== asked.seed || humanSeat !== asked.humanSeat || difficulty !== asked.difficulty) {
    throw new Error("the saved practice game is not the one the page remembers");
  }
  if (save.catalog !== CATALOG_VERSION) {
    throw new Error(`the saved game was played on catalog ${save.catalog}, not ${CATALOG_VERSION}`);
  }
  const game = startGame(save.config);
  for (const action of save.log) {
    const { playerId: _seat, nonce, ...body } = action;
    // R677: the nonce names the player (`h…` the human, `a…` the AI) whatever seat a swap left it in.
    const refusal = apply(game, body as ActionBody, nonce.startsWith("h"));
    if (refusal !== null) throw new Error(`the saved game does not fold: ${refusal}`);
    if (game.log[game.log.length - 1]?.nonce !== nonce) throw new Error("the saved game does not fold: a nonce differs");
  }
  if (hashState(game.state) !== save.hash) throw new Error("the saved game does not fold to the state it was saved at");
  game.rng = createRng(`${save.config.seed}:ai`, save.aiCursor);
  return game;
}

/** R768: the setup `startGame` deals from the kept config, as a resume does, and the log. */
function foldInputOf(replay: PracticeReplay): FoldInput {
  const dealt = startGame(replay.config);
  return {
    seed: replay.config.seed,
    decks: dealt.decks,
    handicaps: dealt.handicaps,
    ...(dealt.lastBoards === undefined ? {} : { lastBoards: dealt.lastBoards }),
    ...(dealt.dealt === undefined ? {} : { dealt: dealt.dealt }),
    log: replay.log,
  };
}

/**
 * The AI's fallback when `decide` gave nothing usable: `endTurn` when legal, else the first legal
 * action the engine accepts, never one R188 forbids.
 */
function fallbackActions(game: PracticeGame): ActionBody[] {
  const legal = legalActions(game.state, aiSeatNow(game)).filter((action) => !isForbiddenAiAction(action));
  const endTurn = legal.filter((action) => action.type === "endTurn");
  const rest = legal.filter((action) => action.type !== "endTurn");
  return [...endTurn, ...rest];
}

export function createPracticeCore(env: PracticeCoreEnv): PracticeCore {
  const budget = env.budget ?? AI_BUDGET;
  let game: PracticeGame | null = null;
  /** R768: each kept game's verdict by number: null when it folds, else part 1's refusal. */
  const verdicts = new Map<number, ReplayRefusal | null>();
  /** The kept game opened last, so paging through it folds it once. */
  let opened: { game: number; open: ReplayOpen } | null = null;

  function current(): PracticeGame {
    if (game === null) throw new Error("no practice game is running: send start first");
    return game;
  }

  /** One AI action, or nothing when the AI owes none. Throws only when no action is possible. */
  function aiStep(active: PracticeGame): void {
    const seat = aiSeatNow(active);
    if (!aiToAct(active.state, seat)) return;

    let chosen: ActionBody | null;
    try {
      const t0 = env.now();
      const decision = decide(active.state, seat, {
        rng: active.rng,
        budget,
        ...(t0 > 0 ? { deadlineMs: t0 + PRACTICE_AI_CLOCK_MS } : {}),
      });
      chosen = decision?.action ?? null;
    } catch {
      // `decide` promises never to throw; if it does anyway, the fallback below still moves the game.
      chosen = null;
    }

    if (chosen !== null && !isForbiddenAiAction(chosen) && apply(active, chosen, false) === null) {
      active.error = null;
      return;
    }

    let lastRefusal = "no legal action";
    for (const candidate of fallbackActions(active)) {
      const refusal = apply(active, candidate, false);
      if (refusal === null) {
        active.error = null;
        return;
      }
      lastRefusal = refusal;
    }
    throw new Error(`the AI owes an action and the engine accepts none of its legal ones (${lastRefusal})`);
  }

  /**
   * R768: a free game that has just ended is kept, the newest of the last PRACTICE_REPLAYS_KEPT; a
   * lesson, or a game Glitch voided (R679: no record is kept of it), is not.
   */
  function keepFinished(active: PracticeGame): void {
    const result = active.state.result;
    if (active.kept || result === null) return;
    active.kept = true;
    const saves = env.saves;
    if (saves === undefined || active.config.lesson !== undefined || result.reason === "voided") return;
    const kept = saves.readReplays();
    const endSeat = humanSeatNow(active);
    const replay: PracticeReplay = {
      catalog: CATALOG_VERSION,
      config: JSON.parse(JSON.stringify(active.config)) as PracticeStartConfig,
      log: JSON.parse(JSON.stringify(active.log)) as Action[],
      hash: hashState(active.state),
      endSeat,
      summary: {
        game: Math.max(0, ...kept.map((old) => old.summary.game)) + 1,
        endedAt: env.now(),
        result: result.winner === "draw" ? "draw" : result.winner === endSeat ? "win" : "loss",
        turns: active.state.turn,
        steps: active.log.length + 1,
        portraits: { ...(active.config.portraits ?? { p1: DEFAULT_PORTRAIT, p2: DEFAULT_PORTRAIT }) },
      },
    };
    saves.writeReplays([...kept, replay].slice(-PRACTICE_REPLAYS_KEPT));
  }

  /** R768: a kept game folded once and checked against its catalog and hash; cached by its number. */
  function openReplay(replay: PracticeReplay): ReplayOpen {
    const gameId = replay.summary.game;
    if (opened?.game === gameId) return opened.open;
    let open: ReplayOpen;
    if (replay.catalog !== CATALOG_VERSION) {
      // Refused before any deck is dealt: another catalog may lack a preset the config names.
      open = { kind: "refused", reason: "earlier_patch" };
    } else {
      try {
        open = replayOpen(foldInputOf(replay), { catalogVersion: replay.catalog, finalHash: replay.hash });
      } catch {
        // A setup that no longer deals no longer folds.
        open = { kind: "refused", reason: "rules_changed" };
      }
    }
    verdicts.set(gameId, open.kind === "ready" ? null : open.reason);
    opened = { game: gameId, open };
    return open;
  }

  function refuseWhilePlaying(): void {
    if (game !== null && game.state.result === null) throw new Error("replays are not available while a practice game is in progress");
  }

  function listReplays(): PracticeReplayListing[] {
    return [...(env.saves?.readReplays() ?? [])].reverse().map((replay) => {
      let verdict = verdicts.get(replay.summary.game);
      if (verdict === undefined) {
        const open = openReplay(replay);
        verdict = open.kind === "ready" ? null : open.reason;
      }
      return {
        ...replay.summary,
        portraits: { ...replay.summary.portraits },
        ...(verdict === null ? {} : { unavailable: verdict }),
      };
    });
  }

  function replayAnswer(id: number, gameId: number, from: number, count: number): PracticeResponse {
    const replay = env.saves?.readReplays().find((kept) => kept.summary.game === gameId);
    if (replay === undefined) throw new Error(`no finished practice game ${String(gameId)} is kept on this device`);
    const open = openReplay(replay);
    if (open.kind === "refused") throw new Error(`practice game ${String(gameId)} cannot be replayed: ${open.reason}`);
    if (!Number.isInteger(from) || from < 0 || from >= open.steps || !Number.isInteger(count) || count < 1) {
      throw new Error(`no step ${String(from)} (${String(count)} asked) in a replay of ${String(open.steps)} steps`);
    }
    const run = replaySeatRun(replay.log, replay.endSeat, from, count);
    const page = replayPage(open.checkpoints, run.seat, from, run.count);
    return { id, type: "replay", game: gameId, from: page.from, steps: page.steps, total: open.steps };
  }

  function debugOf(active: PracticeGame): PracticeDebug {
    // JSON copies: the in-thread host hands these to the caller by reference, and the live state
    // and log must not be reachable from the page.
    return {
      seed: active.config.seed,
      decks: [[...active.decks[0]], [...active.decks[1]]],
      handicaps: JSON.parse(JSON.stringify(active.handicaps)) as Partial<Record<PlayerId, Handicap>>,
      log: JSON.parse(JSON.stringify(active.log)) as Action[],
      state: JSON.parse(JSON.stringify(active.state)) as unknown,
      hash: hashState(active.state),
      difficulty: active.config.difficulty,
      humanSeat: active.config.humanSeat,
      ...(active.config.lesson === undefined ? {} : { lesson: active.config.lesson }),
      ...(active.lastBoards === undefined ? {} : { lastBoards: JSON.parse(JSON.stringify(active.lastBoards)) as [LastBoardCard[], LastBoardCard[]] }),
      ...(active.dealt === undefined ? {} : { dealt: [...active.dealt] }),
    };
  }

  function started(id: number, active: PracticeGame): PracticeResponse {
    env.saves?.write(saveOf(active));
    return { id, type: "started", snapshot: snapshotOf(active), defs: registeredCatalog(), aiSeat: active.aiSeat };
  }

  function snapshotResponse(id: number, active: PracticeGame): PracticeResponse {
    env.saves?.write(saveOf(active));
    keepFinished(active);
    return { id, type: "snapshot", snapshot: snapshotOf(active) };
  }

  function handleUnsafe(request: PracticeRequest): PracticeResponse {
    switch (request.type) {
      case "start": {
        // A start replaces whatever was running, even when it fails: no request may land on the
        // previous game after the page asked for a new one.
        game = null;
        const next = startGame(request.config);
        game = next;
        return started(request.id, next);
      }
      case "resume": {
        // Like a start, a resume replaces whatever was running, even when it fails.
        game = null;
        const next = resumeGame(env.saves?.read() ?? null, request.config);
        game = next;
        return started(request.id, next);
      }
      case "act": {
        const active = current();
        const refusal = apply(active, request.action, true);
        active.error = refusal;
        return snapshotResponse(request.id, active);
      }
      case "aiStep": {
        const active = current();
        aiStep(active);
        return snapshotResponse(request.id, active);
      }
      case "aiEmote": {
        // MD-D29, R1127: the persona's emote, applied only while the AI seat's `legalActions` hold
        // it — an unheard emote is dropped without an error, and the snapshot still answers.
        const active = current();
        const body = { type: "emote", emote: request.emote } as ActionBody;
        const legal = legalActions(active.state, aiSeatNow(active));
        if (legal.some((action) => action.type === "emote" && action.emote === request.emote)) {
          apply(active, body, false);
        }
        return snapshotResponse(request.id, active);
      }
      case "catalog":
        // The card data the setup screen previews decks with, before any game exists (§5.1: the
        // catalog is public). The same map `started` carries.
        return { id: request.id, type: "catalog", defs: registeredCatalog() };
      case "replays":
        refuseWhilePlaying();
        return { id: request.id, type: "replays", replays: listReplays() };
      case "replay":
        refuseWhilePlaying();
        return replayAnswer(request.id, request.game, request.from, request.count);
      case "debug": {
        if (!env.dev) return { id: request.id, type: "failed", message: "debug is available only in a development build" };
        return { id: request.id, type: "debug", debug: debugOf(current()) };
      }
    }
  }

  return {
    handle(request: PracticeRequest): PracticeResponse {
      try {
        return handleUnsafe(request);
      } catch (cause) {
        return { id: request.id, type: "failed", message: messageOf(cause) };
      }
    },
  };
}
