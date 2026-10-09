// The practice worker's wire protocol (SPEC §9.9, R187).
//
// The worker stands where §9.1 puts the server: it holds the one `GameState`, runs `reduce` for
// both seats and the AI for its own, and answers the page with what the human may see. So the
// shapes below are everything the main thread ever learns about a practice game (CLAUDE.md rule 7):
// `viewFor(state, human)`, the human's `legalActions`, whether the AI owes an action, and the
// engine's refusal of the human's last action. `PracticeDebug` is the one exception, and the core
// answers it only in a non-production build, for the e2e replay check. Besides the snapshot, the
// page may read the summaries of the finished games the worker kept and, for one of them, the
// human's `viewFor` at each step (R768): never its log, seed or decks.

import type { Action, ActionBody, CardDefs, DistributiveOmit, EmoteId, PlayerId, PlayerView, PortraitId } from "@jackioh/shared";
import type { ReplayRefusal, ReplayStep } from "@jackioh/engine";
import type { Difficulty, Handicap } from "@jackioh/engine/config";

export type PracticeDeckChoice =
  /**
   * R1373: `leanNewest`, "More cards from the newest set": the worker deals the random deck leaning
   * on the newest set that ships (R1370, R1371). Absent is off.
   */
  | { kind: "random"; leanNewest?: boolean }
  | { kind: "preset"; id: string }
  /** `index` is the 1-based deck number the setup's `saved:<n>` value names. */
  | { kind: "saved"; index: number; cards: string[]; portrait?: string | null };

export type PracticeStartConfig = {
  seed: string;
  difficulty: Difficulty;
  humanSeat: PlayerId;
  deck: PracticeDeckChoice;
  /**
   * SPEC §9.10, R291: a tutorial lesson, by id (`tutorial/lessons.ts`). When set, the lesson's own
   * two decks and the tutorial handicap (`AI_TUTORIAL`, R290) replace `deck`, the AI's dealt deck
   * and `difficulty`'s handicap. The seed and the seat are still this config's: the tutorial passes
   * the lesson's own, and a test may pass others.
   */
  lesson?: string;
  /**
   * R417, R508: the human's last board, from the device (`lastBoard.ts`): the seat's `lastBoards`
   * input to `createGame`. A lesson ignores it; omitted, the human has none.
   */
  lastBoard?: LastBoardCard[];
  /** R768: both seats' portraits as the page dealt them (R642), for a finished game's replay summary. Omitted: the default portrait on both. */
  portraits?: { p1: PortraitId; p2: PortraitId };
};

/** R417: one card of a last board, its card and face only (the engine's `LastBoardEntry`). */
export type LastBoardCard = { defId: string; radiant: boolean };

/** Rule 7: everything the main thread ever gets about the game. */
export type PracticeSnapshot = {
  /** viewFor(state, humanSeat) */
  view: PlayerView;
  /** legalActions(state, humanSeat) */
  legal: ActionBody[];
  /** aiToAct(state, aiSeat) */
  aiToAct: boolean;
  /** The engine's refusal of the last human action. */
  error: string | null;
  /**
   * R508: once a free game is over, the board the human takes from it (`lastBoardFor(state, human)`:
   * the field as the human saw it, the AI's face-down cards left out). Absent otherwise.
   */
  lastBoard?: LastBoardCard[];
};

/** R768: one kept practice game as the replay list shows it: never its log, seed or decks. */
export type PracticeReplaySummary = {
  /** The number a `replay` request names: 1, 2, 3… on this device. */
  game: number;
  /** When it ended, as the worker's clock read it (`PracticeCoreEnv.now`). */
  endedAt: number;
  /** The result for the human, read from the seat it ended in (R677). */
  result: "win" | "loss" | "draw";
  /** The state's turn when it ended. */
  turns: number;
  /** The replay's steps: step 0 and one per accepted action. */
  steps: number;
  portraits: { p1: PortraitId; p2: PortraitId };
};

/** R768: a summary as `replays` answers it; `unavailable` is part 1's reason it cannot be replayed. */
export type PracticeReplayListing = PracticeReplaySummary & { unavailable?: ReplayRefusal };

/** Dev builds only (MODE !== "production"); the one message that carries the raw state. */
export type PracticeDebug = {
  seed: string;
  decks: [string[], string[]];
  handicaps: Partial<Record<PlayerId, Handicap>>;
  log: Action[];
  state: unknown;
  hash: string;
  difficulty: Difficulty;
  humanSeat: PlayerId;
  /** The tutorial lesson this game is, when it is one (§9.10). */
  lesson?: string;
  /** R417: the last boards `createGame` had, seat ordered, when the human brought one. */
  lastBoards?: [LastBoardCard[], LastBoardCard[]];
  /** R433: the seats `createGame` was told were dealt (the human's, on the random deck); absent when none was. */
  dealt?: PlayerId[];
};

export type PracticeRequest =
  | { id: number; type: "start"; config: PracticeStartConfig }
  /**
   * R668: pick up the free game the worker saved on this device. `config` is the setup the page
   * remembers (`resume.ts`); the save itself never leaves the worker. Answered as `started`, or
   * `failed` when there is no such save or it does not fold.
   */
  | { id: number; type: "resume"; config: PracticeStartConfig }
  | { id: number; type: "act"; action: ActionBody }
  | { id: number; type: "aiStep" }
  /**
   * MD-D29, R1127: the AI persona's emote, sent the same way the match actor mints one — applied
   * only while the AI seat's `legalActions` hold it, dropped without an error otherwise.
   */
  | { id: number; type: "aiEmote"; emote: EmoteId }
  /** The public card data, for the setup screen's deck preview; needs no game. */
  | { id: number; type: "catalog" }
  /**
   * R768: the finished free games the worker kept, newest first. Refused while a game is in progress.
   */
  | { id: number; type: "replays" }
  /**
   * R768: steps `from`, … of a kept game, as the human saw them (`viewFor` of the seat it played at
   * each step, R677). Refused while a game is in progress, and for a game that is unavailable. A page
   * may hold fewer steps than asked (part 1's reduce budget, or a Glitch swap that moved the human):
   * the next page starts at `from + steps.length`.
   */
  | { id: number; type: "replay"; game: number; from: number; count: number }
  | { id: number; type: "debug" };

export type PracticeResponse =
  | { id: number; type: "started"; snapshot: PracticeSnapshot; defs: CardDefs; aiSeat: PlayerId }
  | { id: number; type: "snapshot"; snapshot: PracticeSnapshot }
  | { id: number; type: "catalog"; defs: CardDefs }
  | { id: number; type: "replays"; replays: PracticeReplayListing[] }
  | { id: number; type: "replay"; game: number; from: number; steps: ReplayStep[]; total: number }
  | { id: number; type: "debug"; debug: PracticeDebug }
  | { id: number; type: "failed"; message: string };

export type PracticeRequestBody = DistributiveOmit<PracticeRequest, "id">;
