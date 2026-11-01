// The practice worker's wire protocol (SPEC §9.9, R187).
//
// The worker stands where §9.1 puts the server, so the shapes below are everything the main thread ever
// learns about a practice game (CLAUDE.md rule 7). `PracticeDebug` is the one exception, answered only in a
// non-production build, for the e2e replay check. A kept game crosses as its summary and the human's
// `viewFor` at each step (R768), never its log, seed or decks; the one log that crosses is a finished
// game's on a server seed, as its claim (R1303000), once the game it could help with is over.

import { LEAN_NEWEST } from "@jackioh/shared";
import type {
  Action,
  ActionBody,
  CardDefs,
  CustomCard,
  DistributiveOmit,
  LeanPick,
  PlayerId,
  PlayerView,
  PortraitId,
} from "@jackioh/shared";
import type { ReplayRefusal, ReplayStep } from "@jackioh/engine";
import type { Difficulty, Handicap } from "@jackioh/engine/config";

import type { DeckCosmetics } from "../cards/cosmetics.ts";

export type PracticeDeckChoice =
  /**
   * R1373, R924000: `leanSet`, "More cards from" ("newest" when it deals, R1370, R1371). A game kept before
   * the picker (R668, R768) carries R1373's `leanNewest` instead, so it deals the deck it always dealt.
   */
  | { kind: "random"; leanSet?: LeanPick; leanNewest?: boolean }
  | { kind: "preset"; id: string }
  /** `index` is the 1-based deck number the setup's `saved:<n>` value names. */
  | { kind: "saved"; index: number; cards: string[]; portrait?: string | null; cosmetics?: DeckCosmetics }
  /** #920: the creator's "Try it": a random deck of one card fewer, and the custom card `card`. */
  | { kind: "try"; card: string };

/** R924000: the pick a random deck leans on, R1373's `leanNewest` read as "newest"; `null` for none. */
export function randomDeckLean(choice: PracticeDeckChoice & { kind: "random" }): LeanPick | null {
  return choice.leanSet ?? (choice.leanNewest === true ? LEAN_NEWEST : null);
}

export type PracticeStartConfig = {
  seed: string;
  difficulty: Difficulty;
  humanSeat: PlayerId;
  deck: PracticeDeckChoice;
  /**
   * SPEC §9.10, R291: a tutorial lesson (`tutorial/lessons.ts`), whose decks and handicaps (R290, R1705005)
   * replace `deck` and `difficulty`'s; the seed and the seat are still this config's.
   */
  lesson?: string;
  /** R417, R508: the human's last board, from the device (`lastBoard.ts`); a lesson ignores it. */
  lastBoard?: LastBoardCard[];
  /** R768: both seats' portraits as the page dealt them (R642), for the replay summary; omitted, the default. */
  portraits?: { p1: PortraitId; p2: PortraitId };
  /**
   * R923000: a sandbox game, whose dev tools (`sandbox` actions, sent as `act`) only it accepts. It
   * brings no last board and leaves none (R508). Never a lesson.
   */
  sandbox?: true;
  /**
   * R915006: the custom cards the human's deck names (SPEC §12), which the game carries and its replay
   * folds with; a game with any runs in the Luau module (R915009) and records no card statistics (§12.10).
   */
  customCards?: CustomCard[];
  /**
   * Issue #1113, tests only: the ids the AI deck is dealt without, as `banned`; absent, the live shadow
   * ban. The seeded fixtures (the R265/B34/R677 seeds, spec 13's URL games) set it to the frozen pool.
   */
  aiBanned?: readonly string[];
  /**
   * R1303000: the game is on a seed the server issued for this setup, so its end may be claimed and
   * paid; never a lesson's, a sandbox's, one with custom cards, a Try it or a URL seed.
   */
  earning?: true;
};

/** R1303000: a finished game on a server seed, as the server checks it; the human's result is read from the seat it ended in (R677). */
export type PracticeClaim = { seed: string; result: "win" | "loss" | "draw"; log: Action[] };

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
  /** R508: once a free game is over, the field as the human saw it, the AI's face-down cards left out. */
  lastBoard?: LastBoardCard[];
  /** R768000: once a free game is over and kept, its number and step count, which "Watch the replay" opens. */
  replay?: Pick<PracticeReplaySummary, "game" | "steps">;
  /**
   * R2102000: present only while the human of a game against the AI (R2291000) has a sure win this turn
   * by what they can see (`lethalHint`), shown only with the Lethal hint setting on. It never says how.
   */
  lethal?: true;
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

/**
 * R768: `unavailable` is part 1's reason it cannot be replayed: `earlier_patch` from its catalog alone,
 * `rules_changed` once a `replay` of it found so, since the list folds no game.
 */
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
  lesson?: string;
  /** R417: the last boards `createGame` had, seat ordered, when the human brought one. */
  lastBoards?: [LastBoardCard[], LastBoardCard[]];
  /** R433: the seats `createGame` was told were dealt; absent when none was. */
  dealt?: PlayerId[];
  /** R923000: present when the game is a sandbox, which its fold must be told. */
  sandbox?: true;
  /** R915006: the game's custom cards, which its fold must be given; absent when it has none. */
  customCards?: CustomCard[];
};

export type PracticeRequest =
  | { id: number; type: "start"; config: PracticeStartConfig }
  /** R668: pick up the game saved on this device, named by the setup the page remembers (`resume.ts`). */
  | { id: number; type: "resume"; config: PracticeStartConfig }
  | { id: number; type: "act"; action: ActionBody }
  | { id: number; type: "aiStep" }
  /** The public card data, for the setup screen's deck preview; needs no game. */
  | { id: number; type: "catalog" }
  /** R768: the finished free games the worker kept, newest first. Refused while a game is in progress. */
  | { id: number; type: "replays" }
  /**
   * R768: steps `from`, … of a kept game, as the human saw them (R677). A page may hold fewer steps than
   * asked (part 1's reduce budget, or a Glitch swap): the next page starts at `from + steps.length`.
   */
  | { id: number; type: "replay"; game: number; from: number; count: number }
  | { id: number; type: "debug" }
  /**
   * R1303000: the finished game's claim. Refused before the end (CLAUDE.md rule 7: the log names the
   * AI's hidden cards while the game runs) and for a game not on a server seed.
   */
  | { id: number; type: "claim" };

export type PracticeResponse =
  | { id: number; type: "started"; snapshot: PracticeSnapshot; defs: CardDefs; aiSeat: PlayerId }
  | { id: number; type: "snapshot"; snapshot: PracticeSnapshot }
  | { id: number; type: "catalog"; defs: CardDefs }
  | { id: number; type: "replays"; replays: PracticeReplayListing[] }
  | { id: number; type: "replay"; game: number; from: number; steps: ReplayStep[]; total: number }
  | { id: number; type: "debug"; debug: PracticeDebug }
  | { id: number; type: "claim"; claim: PracticeClaim }
  | { id: number; type: "failed"; message: string };

export type PracticeRequestBody = DistributiveOmit<PracticeRequest, "id">;
