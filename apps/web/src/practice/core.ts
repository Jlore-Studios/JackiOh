// The practice game itself: the one `GameState`, both seats' actions and the AI (SPEC §9.9, R187).
//
// This is the ONLY practice file that imports `@jackioh/engine`, `@jackioh/cards` or `@jackioh/ai`
// (the WebAssembly engine and AI, docs/v0.3.0/SURFACE.md §10.4), and only the worker entry
// (`practice.worker.ts`) and jsdom's in-thread host (`host.ts`) load it; the page never does. That is
// rule 7 here: the state and both hands stay on this side of `handle`, and what crosses it is a
// `PracticeSnapshot`, the worker standing where §9.1 puts the server. The sure-win check (R2102000,
// R2291000) is asked of the human's own redaction, so the AI's hidden cards never steer it.
//
// Nonces (`h<n>` human, `a<n>` AI) count only accepted actions, so a refused action spends nothing and
// the log folds exactly (R187, R508, R433). The AI draws from its own stream, `${seed}:ai`, never the
// match rng.
//
// Resume (R668): the worker dies with the page, so a free game in progress is saved after every answer
// (`saveStore.ts`) and, after a reload, its log is folded again through `apply` under the same nonces to
// the same hash. The save never crosses to the page: its log names the AI's hidden cards.
//
// Replays (R768): the newest PRACTICE_REPLAYS_KEPT free games that ended (not a lesson, nor one Glitch
// voided, R679) are kept in the same store. `replays` folds none of them, which would stall the worker;
// `replay` answers `viewFor` of the seat the human played at each step (R677). Like the save, a kept
// game never crosses to the page.
//
// Claims (R1303000): a free game on a server seed (`earning: true`) answers `claim` once it is over
// with its seed, result and log, which the server folds (R2560000) before it pays. The log crosses only
// after the end, so it cannot help the human play (rule 7). A resumed game claims on the seed it began on
// (R1303001) and decides under PRACTICE_AI_CLOCK_MS like any other, since the server does not re-run the
// AI's moves.

import { CATALOG_VERSION, versionNameFrom } from "@jackioh/cards";
import {
  beginGame,
  carriedVersions,
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
import {
  AI_DIFFICULTY,
  DECK_SIZE,
  type Handicap,
  VERSIONS_SCHEME,
} from "@jackioh/engine/config";
import { AI_BUDGET, aiToAct, buildAiDeck, decide, lethalHint, type SearchBudget } from "@jackioh/ai";
import { DEFAULT_PORTRAIT, leanSetOf, opponentOf, type Action, type ActionBody, type CustomCard, type PlayerId } from "@jackioh/shared";

import { PRACTICE_AI_CLOCK_MS, PRACTICE_LETHAL_HINT_CLOCK_MS, PRACTICE_REPLAYS_CACHED, PRACTICE_REPLAYS_KEPT } from "./config.ts";
import { seatOutcome } from "../game/outcome.ts";
import { lessonById, lessonHandicaps } from "../tutorial/lessons.ts";
import { presetById } from "./decks.ts";
import type { PracticeReplay, PracticeReplaySetup, PracticeSave, PracticeSaveStore } from "./saveStore.ts";
import { randomDeckLean } from "./protocol.ts";
import type {
  LastBoardCard,
  PracticeDebug,
  PracticeClaim,
  PracticeDeckChoice,
  PracticeReplayListing,
  PracticeReplaySummary,
  PracticeRequest,
  PracticeResponse,
  PracticeSnapshot,
  PracticeStartConfig,
} from "./protocol.ts";
import { messageOf } from "../net/errors.ts";

export type PracticeCoreEnv = {
  /**
   * The AI's and the lethal hint's (R2102000) wall clock, in `Date.now()` terms: the clock the WebAssembly
   * side reads against the deadline; 0 is no clock. Also the time a finished game is kept at (R768).
   */
  now: () => number;
  /** answer `debug` only when true */
  dev: boolean;
  /** default AI_BUDGET */
  budget?: SearchBudget;
  /** R668, R768: where the game in progress and the finished ones kept for replay live; absent, nothing is kept. */
  saves?: PracticeSaveStore;
};

/**
 * `handle` never throws: anything that goes wrong comes back as `"failed"`. `needsLuau` (R915009): whether
 * `request` runs on the Luau module (custom cards), the normal one, or whichever is in use (`null`), so the
 * worker can switch before the synchronous `handle`.
 */
export type PracticeCore = {
  handle(request: PracticeRequest): PracticeResponse;
  needsLuau(request: PracticeRequest): boolean | null;
};

export function hasCustomCards(config: PracticeStartConfig): boolean {
  return config.customCards !== undefined && config.customCards.length > 0;
}

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
  rng: Rng;
  humanCount: number;
  aiCount: number;
  /** The engine's refusal of the human's last action, until the board moves on. */
  error: string | null;
  /** R768: its end has been kept (or passed over), so it is kept once. */
  kept: boolean;
  /** R768000: the number and step count it was kept under, once kept. */
  keptAs?: Pick<PracticeReplaySummary, "game" | "steps">;
};

/** R188: the action types the AI never takes on its own, even as a fallback. */
function isForbiddenAiAction(action: ActionBody): boolean {
  // MD-D29, R1127: the persona's emote is no AI decision; it arrives on its own request. R923000: the
  // sandbox's tools are the human's alone (`legalActions` never lists one; this gate holds it twice).
  if (action.type === "concede" || action.type === "offerDraw" || action.type === "emote" || action.type === "sandbox") return true;
  return action.type === "answerDraw" && action.accept;
}

/**
 * `createGame` checks a preset or a saved deck like any other (§2.6). R1373, R924000: a random deck leans as
 * asked, "newest" (R1371) resolved here; the AI's never leans. A player's random deck is dealt from every
 * set: neither the AI's shadow ban nor its soft gate (R1390) applies.
 */
function humanDeckFor(seed: string, choice: PracticeDeckChoice): string[] {
  switch (choice.kind) {
    case "random": {
      const lean = randomDeckLean(choice);
      return buildAiDeck(createRng(`${seed}:human-deck`), DECK_SIZE, {
        banned: [],
        gatedSets: [],
        ...(lean === null ? {} : { leanSet: leanSetOf(lean) }),
      });
    }
    case "preset": {
      const preset = presetById(choice.id);
      if (preset === undefined) throw new Error(`unknown preset deck "${choice.id}"`);
      return [...preset.cards];
    }
    case "saved":
      return [...choice.cards];
    case "try":
      // #920: the creator's "Try it": the custom card and a random deck around it.
      return [
        choice.card,
        ...buildAiDeck(createRng(`${seed}:human-deck`), DECK_SIZE - 1, { banned: [], gatedSets: [] }),
      ];
  }
}

/**
 * §9.10, R291: a lesson's fixed decks, the tutorial handicap on the AI (R290) and `TUTORIAL_HUMAN` on the
 * human, whose fixed deck is smaller than `DECK_SIZE` (R1705005); or a free game's (§9.9), none on the human.
 */
function decksAndHandicap(config: PracticeStartConfig): {
  humanDeck: string[];
  aiDeck: string[];
  handicap: Handicap;
  humanHandicap?: Handicap;
} {
  if (config.lesson !== undefined) {
    const lesson = lessonById(config.lesson);
    if (lesson === undefined) throw new Error(`unknown tutorial lesson "${config.lesson}"`);
    // R2201001: the lesson's pool hold rides on the human's handicap (lessons 1–4).
    const handicaps = lessonHandicaps(lesson);
    return {
      humanDeck: [...lesson.humanDeck],
      aiDeck: [...lesson.aiDeck],
      handicap: handicaps.ai,
      humanHandicap: handicaps.human,
    };
  }
  const handicap = AI_DIFFICULTY[config.difficulty];
  return {
    humanDeck: humanDeckFor(config.seed, config.deck),
    aiDeck: buildAiDeck(createRng(`${config.seed}:ai-deck`), handicap.deckSize, {
      manaCap: handicap.manaCap,
      ...(config.aiBanned === undefined ? {} : { banned: [...config.aiBanned] }),
    }),
    handicap,
  };
}

/**
 * R433: the seats whose deck was dealt rather than built, so the human is shown none of it going in. The AI's
 * deck is never counted dealt: no view shows its seat, and R185's redaction would hide its own library.
 */
function dealtSeats(config: PracticeStartConfig): PlayerId[] {
  const dealt = config.deck.kind === "random" || config.deck.kind === "try";
  return config.lesson === undefined && dealt ? [config.humanSeat] : [];
}

function startGame(config: PracticeStartConfig): PracticeGame {
  const aiSeat = opponentOf(config.humanSeat);
  const { humanDeck, aiDeck, handicap, humanHandicap } = decksAndHandicap(config);
  const decks: [string[], string[]] = config.humanSeat === "p1" ? [humanDeck, aiDeck] : [aiDeck, humanDeck];
  const handicaps: Partial<Record<PlayerId, Handicap>> = {
    [aiSeat]: { ...handicap },
    ...(humanHandicap === undefined ? {} : { [config.humanSeat]: { ...humanHandicap } }),
  };
  // R508: the human's last board, on a free game only; the AI's seat never has one. R923000: a sandbox
  // game brings none.
  const board = config.lesson === undefined && config.sandbox !== true ? (config.lastBoard ?? []) : [];
  const lastBoards: [LastBoardCard[], LastBoardCard[]] | undefined =
    board.length === 0 ? undefined : config.humanSeat === "p1" ? [[...board], []] : [[], [...board]];
  const dealt = dealtSeats(config);

  // `createGame` throws on an illegal deck and `beginGame` reports a refusal: no game, and "failed".
  const created = createGame({
    seed: config.seed,
    decks,
    handicaps,
    ...(lastBoards === undefined ? {} : { lastBoards }),
    ...(dealt.length === 0 ? {} : { dealt }),
    // R923000: the one game whose dev tools the engine takes.
    ...(config.sandbox === true ? { sandbox: true } : {}),
    // R915006: the custom cards the human's deck names, which only the Luau module runs (R915009).
    ...(hasCustomCards(config) ? { customCards: config.customCards } : {}),
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

function aiSeatNow(game: PracticeGame): PlayerId {
  return seatPlayedBy(game.state, game.aiSeat);
}

/**
 * R677, R768: the seat the human played at step `from` of a kept game, and for how many steps up to
 * `count`, so a page stops where a Glitch swap moved the human. The last step's is the one it ended in.
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
 * R677: in the seat the player plays now; null when accepted, else the engine's reason. The nonce
 * counts by player, not by seat, so a swap leaves both streams unbroken.
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

/**
 * R2291000: the lethal hint, asked of the human's own redaction under its own wall-clock cap; a check
 * that runs past it, or throws, gives no hint, and the game goes on.
 */
function lethalFor(game: PracticeGame, now: number): boolean {
  try {
    return lethalHint(game.state, humanSeatNow(game), now > 0 ? now + PRACTICE_LETHAL_HINT_CLOCK_MS : undefined);
  } catch {
    return false;
  }
}

/** `now` (the env's clock) caps the lethal hint's check. */
function snapshotOf(game: PracticeGame, now: number): PracticeSnapshot {
  // R677: the human sees, and acts from, the seat it plays now; the result reads from it too.
  const human = humanSeatNow(game);
  const result = game.state.result;
  // R508: the board the human takes away, once a free game is over; R679: a voided game leaves none,
  // and R923000: a sandbox game, whose board its tools built, leaves none either.
  const keepsBoard =
    result !== null && result.reason !== "voided" && game.config.lesson === undefined && game.config.sandbox !== true;
  return {
    view: viewFor(game.state, human),
    legal: legalActions(game.state, human),
    aiToAct: aiToAct(game.state, aiSeatNow(game)),
    error: game.error,
    ...(keepsBoard ? { lastBoard: lastBoardFor(game.state, human) } : {}),
    ...(lethalFor(game, now) ? { lethal: true as const } : {}),
    ...(game.keptAs === undefined ? {} : { replay: { ...game.keptAs } }),
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
    versions: VERSIONS_SCHEME,
    config: JSON.parse(JSON.stringify(game.config)) as PracticeStartConfig,
    log: JSON.parse(JSON.stringify(game.log)) as Action[],
    aiCursor: game.rng.cursor,
    hash: hashState(game.state),
  };
}

/**
 * R668: the log is replayed through `apply`, under the nonces it was accepted under, to the hash it was
 * saved at; a save from another catalog, or one that does not fold to the same game, is refused.
 */
function resumeGame(save: PracticeSave | null, asked: PracticeStartConfig): PracticeGame {
  if (save === null) throw new Error("no practice game is saved on this device");
  // The page names the game it remembers; a save of any other game is not the one it asked for.
  // R923000: a sandbox game is the page's only when it remembers a sandbox, whose panel it then shows.
  const { seed, humanSeat, difficulty, sandbox } = save.config;
  if (seed !== asked.seed || humanSeat !== asked.humanSeat || difficulty !== asked.difficulty || sandbox !== asked.sandbox) {
    throw new Error("the saved practice game is not the one the page remembers");
  }
  if (catalogOf(save) !== CATALOG_VERSION) {
    throw new Error(`the saved game was played on catalog ${catalogOf(save)}, not ${CATALOG_VERSION}`);
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

/**
 * R768, R767000: the setup it kept, else (a game kept before setups were, on this build's catalog only)
 * the setup `startGame` deals from the kept config, as a resume does.
 */
function foldInputOf(replay: PracticeReplay): FoldInput {
  const dealt = replay.setup ?? startGame(replay.config);
  return {
    seed: replay.config.seed,
    decks: dealt.decks,
    handicaps: dealt.handicaps,
    ...(dealt.lastBoards === undefined ? {} : { lastBoards: dealt.lastBoards }),
    ...(dealt.dealt === undefined ? {} : { dealt: dealt.dealt }),
    ...(replay.config.sandbox === true ? { sandbox: true } : {}),
    ...(hasCustomCards(replay.config) ? { customCards: replay.config.customCards } : {}),
    log: replay.log,
  };
}

/**
 * R767000: whether this build can fold a kept game: one played on its own catalog version, or on a
 * version it carries when the game kept its setup (dealing it again would read this build's catalog).
 */
function foldable(replay: PracticeReplay): boolean {
  const catalog = catalogOf(replay);
  return catalog === CATALOG_VERSION || (replay.setup !== undefined && carriedVersions().includes(catalog));
}

/**
 * R1371000: the catalog a kept game or save was played on, as it is named now: one kept before the version
 * retcon carries no `versions` marker, and one of an older marker names a v0.5 version as it was (R2532000).
 */
export function catalogOf(kept: Pick<PracticeReplay, "catalog" | "versions">): string {
  return versionNameFrom(kept.versions ?? 1, kept.catalog);
}

/** The AI's fallback when `decide` gave nothing usable, `endTurn` first, never one R188 forbids. */
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
  /** The last PRACTICE_REPLAYS_CACHED kept games opened, by number, the one opened longest ago first. */
  const opened = new Map<number, ReplayOpen>();

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

  /** R768: a free game that has just ended is kept; a lesson, or a game Glitch voided (R679), is not. */
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
      versions: VERSIONS_SCHEME,
      config: JSON.parse(JSON.stringify(active.config)) as PracticeStartConfig,
      setup: JSON.parse(
        JSON.stringify({
          decks: active.decks,
          handicaps: active.handicaps,
          ...(active.lastBoards === undefined ? {} : { lastBoards: active.lastBoards }),
          ...(active.dealt === undefined ? {} : { dealt: active.dealt }),
        }),
      ) as PracticeReplaySetup,
      log: JSON.parse(JSON.stringify(active.log)) as Action[],
      hash: hashState(active.state),
      endSeat,
      summary: {
        game: Math.max(0, ...kept.map((old) => old.summary.game)) + 1,
        endedAt: env.now(),
        // R1843004: the AI's `fled` is the human's draw.
        result: seatOutcome(result.winner, result.reason, endSeat),
        turns: active.state.turn,
        steps: active.log.length + 1,
        portraits: { ...(active.config.portraits ?? { p1: DEFAULT_PORTRAIT, p2: DEFAULT_PORTRAIT }) },
      },
    };
    saves.writeReplays([...kept, replay].slice(-PRACTICE_REPLAYS_KEPT));
    active.keptAs = { game: replay.summary.game, steps: replay.summary.steps };
  }

  /** R768: a kept game folded once and checked against its catalog and hash; held by its number. */
  function openReplay(replay: PracticeReplay): ReplayOpen {
    const gameId = replay.summary.game;
    const held = opened.get(gameId);
    if (held !== undefined) {
      // Opened again: it moves to the newest end, so it is the last to be dropped.
      opened.delete(gameId);
      opened.set(gameId, held);
      return held;
    }
    let open: ReplayOpen;
    if (!foldable(replay)) {
      // Refused before any deck is dealt: another catalog may lack a preset the config names.
      open = { kind: "refused", reason: "earlier_patch" };
    } else {
      try {
        open = replayOpen(foldInputOf(replay), { catalogVersion: catalogOf(replay), finalHash: replay.hash });
      } catch {
        // A setup that no longer deals no longer folds.
        open = { kind: "refused", reason: "rules_changed" };
      }
    }
    verdicts.set(gameId, open.kind === "ready" ? null : open.reason);
    opened.set(gameId, open);
    if (opened.size > PRACTICE_REPLAYS_CACHED) {
      // A Map keeps insertion order, so its first key is the game opened longest ago.
      const oldest = opened.keys().next().value;
      if (oldest !== undefined) opened.delete(oldest);
    }
    return open;
  }

  function refuseWhilePlaying(): void {
    if (game !== null && game.state.result === null) throw new Error("replays are not available while a practice game is in progress");
  }

  /**
   * R768: no kept game is folded to be listed; one unopened is listed unavailable only when this build
   * cannot fold its catalog version (R767000).
   */
  function listReplays(): PracticeReplayListing[] {
    return [...(env.saves?.readReplays() ?? [])].reverse().map((replay) => {
      const verdict = foldable(replay) ? (verdicts.get(replay.summary.game) ?? null) : "earlier_patch";
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
      ...(active.config.sandbox === true ? { sandbox: true } : {}),
      ...(hasCustomCards(active.config) ? { customCards: JSON.parse(JSON.stringify(active.config.customCards)) as CustomCard[] } : {}),
    };
  }

  /** R1303000: a finished game on a server seed, as the server checks it; JSON copies, as `debugOf`'s. */
  function claimOf(active: PracticeGame): PracticeClaim {
    const result = active.state.result;
    if (active.config.earning !== true) throw new Error("this practice game is not on a server seed, so it earns nothing");
    if (result === null) throw new Error("a practice game is claimed only once it is over");
    const endSeat = humanSeatNow(active);
    return {
      seed: active.config.seed,
      // R677, R1843004: from the seat the human ended in, as `keepFinished` and the server's fold read it.
      result: seatOutcome(result.winner, result.reason, endSeat),
      log: JSON.parse(JSON.stringify(active.log)) as Action[],
    };
  }

  function started(id: number, active: PracticeGame): PracticeResponse {
    env.saves?.write(saveOf(active));
    return { id, type: "started", snapshot: snapshotOf(active, env.now()), defs: registeredCatalog(), aiSeat: active.aiSeat };
  }

  function snapshotResponse(id: number, active: PracticeGame): PracticeResponse {
    env.saves?.write(saveOf(active));
    keepFinished(active);
    return { id, type: "snapshot", snapshot: snapshotOf(active, env.now()) };
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
      case "catalog":
        // §5.1: the catalog is public, so the setup previews decks with it before any game exists.
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
      case "claim":
        return { id: request.id, type: "claim", claim: claimOf(current()) };
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
    needsLuau(request: PracticeRequest): boolean | null {
      switch (request.type) {
        case "start":
          return hasCustomCards(request.config);
        case "resume": {
          // The save, not the page's memory, says what the game carries.
          const save = env.saves?.read() ?? null;
          return save === null ? false : hasCustomCards(save.config);
        }
        case "replay": {
          const kept = env.saves?.readReplays().find((replay) => replay.summary.game === request.game);
          return kept === undefined ? null : hasCustomCards(kept.config);
        }
        default:
          return null;
      }
    },
  };
}
