// The practice worker's wire protocol (SPEC §9.9, R187).
//
// The worker stands where §9.1 puts the server, so the shapes below are everything the main thread ever
// learns about a practice game (CLAUDE.md rule 7). `PracticeDebug` is the one exception, answered only in a
// non-production build, for the e2e replay check. A kept game crosses as its summary and the human's
// `viewFor` at each step (R768), never its log, seed or decks; the one log that crosses is a finished
// game's on a server seed, as its claim (R1303000), once the game it could help with is over. A dungeon
// run's fight answers the run's `FightOver` once it is over (R2706000), never its log.

import { LEAN_NEWEST } from "@jackioh/shared";
import type { ActionBody, CardDefs, DistributiveOmit, LeanPick, PlayerId, PortraitId } from "@jackioh/shared";
import type { ReplayStep } from "@jackioh/engine";
import type { Difficulty } from "@jackioh/engine/config";
import type { RunAction, RunFightSetup } from "@jackioh/run";

import type { DeckCosmetics } from "../cards/cosmetics.ts";
import type { LastBoardEntry } from "../wire/generated/LastBoardEntry.ts";
import type { TimerKey } from "../wire/generated/TimerKey.ts";
import type { PracticeClaim } from "../wire/generated/PracticeClaim.ts";
import type { PracticeDebug } from "../wire/generated/PracticeDebug.ts";
import type { PracticeReplayListing } from "../wire/generated/PracticeReplayListing.ts";
import type { PracticeReplaySummary } from "../wire/generated/PracticeReplaySummary.ts";
import type { PracticeSnapshot } from "../wire/generated/PracticeSnapshot.ts";

export type PracticeDeckChoice =
  /**
   * R1373, R924000: `leanSet`, "More cards from" ("newest" when it deals, R1370, R1371). A game kept before
   * the picker (R668, R768) carries R1373's `leanNewest` instead, so it deals the deck it always dealt.
   */
  | { kind: "random"; leanSet?: LeanPick; leanNewest?: boolean }
  | { kind: "preset"; id: string }
  /** `index` is the 1-based deck number the setup's `saved:<n>` value names. */
  | { kind: "saved"; index: number; cards: string[]; portrait?: string | null; cosmetics?: DeckCosmetics };

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
   * Issue #1113, tests only: the ids the AI deck is dealt without, as `banned`; absent, the live shadow
   * ban. The seeded fixtures (the R265/B34/R677 seeds, spec 13's URL games) set it to the frozen pool.
   */
  aiBanned?: readonly string[];
  /**
   * R1303000: the game is on a seed the server issued for this setup, so its end may be claimed and
   * paid; never a lesson's, a sandbox's or a URL seed.
   */
  earning?: true;
  /**
   * R2882000: today's deal, the UTC day (`YYYY-MM-DD`) whose seed and seat it plays (`todaysDeal.ts`). It brings
   * no last board and asks the server for no seed, so every device deals it alike; it earns nothing.
   */
  dailyDeal?: string;
  /**
   * DR14, R2706000, R3476000: a dungeon run's fight (`docs/dungeon-run.md` DR5). When set, its
   * decks, handicaps, deck marks and seed replace `deck`, `difficulty`'s handicap and the dealt AI
   * deck: the run deck, its handicap and marks at `humanSeat`, the enemy's deck and handicap at the
   * AI's seat. It brings no last board and leaves none, is kept in its own save slot while in
   * progress but never kept for replay, and earns nothing.
   */
  run?: RunFightSetup;
};

/**
 * What the machine tells the page, as `crates/shared`'s practice session writes it (#3099,
 * `wire/generated/`): the snapshot (rule 7, everything the main thread ever gets about the game), a kept
 * game's summary, the claim the server folds (R1303000) and the dev-only debug answer.
 */
export type { PracticeClaim, PracticeDebug, PracticeReplayListing, PracticeReplaySummary, PracticeSnapshot };

/** R417: one card of a last board, its card and face only (the engine's `LastBoardEntry`). */
export type LastBoardCard = LastBoardEntry;

/**
 * R1563006: a timer the machine set (`inMs` from when the answer was made) or took back, as the driver
 * relays it with a response. The page keeps it as a timeout and fires it back as `timerFired`.
 */
export type PracticeTimer =
  | { command: "set"; key: TimerKey; timer: number; inMs: number }
  | { command: "cancel"; key: TimerKey; timer: number };

export type PracticeRequest =
  | { id: number; type: "start"; config: PracticeStartConfig }
  /**
   * R668: pick up the game saved on this device, named by the setup the page remembers
   * (`resume.ts`); R3476000: a run fight resumes its own kept fight the same way, from its own slot.
   */
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
  | { id: number; type: "claim" }
  /**
   * DR14, R2706000: a finished dungeon run's fight as the run reads it: `runFightReport` over the
   * game's setup and log, for the seat the human ended in (R677). Refused before the end and for a
   * game that is no fight. Only the run's action crosses, never the log (CLAUDE.md rule 7).
   */
  | { id: number; type: "runReport" }
  /** R1563006: a timer the machine set came due (`timers` of an earlier answer); `timer` is its id. */
  | { id: number; type: "timerFired"; key: TimerKey; timer: number };

export type PracticeResponse =
  | { id: number; type: "started"; snapshot: PracticeSnapshot; defs: CardDefs; aiSeat: PlayerId; timers?: PracticeTimer[] }
  | { id: number; type: "snapshot"; snapshot: PracticeSnapshot; timers?: PracticeTimer[] }
  | { id: number; type: "catalog"; defs: CardDefs }
  | { id: number; type: "replays"; replays: PracticeReplayListing[] }
  | { id: number; type: "replay"; game: number; from: number; steps: ReplayStep[]; total: number }
  | { id: number; type: "debug"; debug: PracticeDebug }
  | { id: number; type: "claim"; claim: PracticeClaim }
  /** DR14, R2706000: the run's `FightOver`, or the run's refusal to read the fight. */
  | { id: number; type: "runReport"; action: RunAction }
  | { id: number; type: "runReport"; error: string }
  | { id: number; type: "failed"; message: string };

export type PracticeRequestBody = DistributiveOmit<PracticeRequest, "id">;
