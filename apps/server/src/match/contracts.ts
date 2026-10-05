/**
 * The seams inside `src/match`: the socket transport, the match clock and the results writer.
 *
 * They are separated so the actor (M6-T4) never imports a WebSocket library, the clock (M7-T1)
 * never imports a timer, and the results writer (M7-T2) never imports the actor. Everything is
 * driven from injected ports, which is what makes the fake-timer tests possible.
 */

import type { GameOverReason, PlayerId } from "@jackioh/shared";
import type {
  LastBoardEntry,
  Logger,
  MatchClocks,
  MatchSeat,
  ResultRow,
  ServerConfig,
  Store,
  Timers,
} from "../api/ports";
import type { EnginePort } from "./engine";

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

export type SocketHandlers = {
  message: (text: string) => void;
  close: () => void;
};

/**
 * One player's connection. Text frames only: every protocol message is JSON. `ws` and the
 * in-memory test pair both satisfy this, so the actor is transport-agnostic.
 */
export type Socket = {
  readonly isOpen: boolean;
  send: (text: string) => void;
  close: (code?: number, reason?: string) => void;
  /** Installed once, by the actor, when the socket is attached to a seat. */
  attach: (handlers: SocketHandlers) => void;
};

// ---------------------------------------------------------------------------
// Clock (SPEC §9.5, R79)
// ---------------------------------------------------------------------------

/** Everything the clock needs to know about the game; none of it is hidden information. */
export type ClockView = {
  turn: number;
  active: PlayerId;
  /** Who owes the open prompt an answer, or null. */
  pendingFor: PlayerId | null;
  /**
   * R265: the seats that still owe their mulligan while both are open, in seat order; empty outside
   * that window. Non-empty with no `pendingFor` is the window the mulligan clock runs over (R268).
   */
  mulliganOwed: readonly PlayerId[];
  over: boolean;
};

export type ClockExpiry =
  /** R79: the active player's turn clock ran out; `timeout` answers their prompts and ends the turn. */
  | { kind: "turn"; player: PlayerId }
  /** R79: a prompt held by the non-active player ran out; `timeout` answers only that prompt. */
  | { kind: "prompt"; player: PlayerId }
  /**
   * R268: the one mulligan clock ran out. It names no player because it belongs to both: the actor
   * times out every seat still owing its mulligan at that moment, each with its own `timeout`.
   */
  | { kind: "mulligan" }
  /** §9.5: grace expired; `disconnectExpired` makes it a loss for that player. */
  | { kind: "grace"; player: PlayerId }
  /** R79: the hard wall-clock ceiling; `ceilingReached` ends the match in a draw. */
  | { kind: "ceiling" };

export type MatchClock = {
  /** Called once after `beginGame` and again after every state change. */
  sync: (view: ClockView) => void;
  /** §9.5: the countdown is stored on the match so both clients show it. */
  startGrace: (player: PlayerId) => void;
  clearGrace: (player: PlayerId) => void;
  /** Deadlines for `matches.clocks` and the protocol's `clock` message. */
  snapshot: () => MatchClocks;
  /** Milliseconds left on the deadline that currently belongs to this player, or null. */
  remainingFor: (player: PlayerId) => number | null;
  /** Cancels every outstanding timer. Idempotent. */
  stop: () => void;
};

export type CreateMatchClockInput = {
  timers: Timers;
  config: ServerConfig;
  /** When the match started; the ceiling is measured from here (R79). */
  startedAt: number;
  onExpire: (expiry: ClockExpiry) => void;
};

export type CreateMatchClock = (input: CreateMatchClockInput) => MatchClock;

// ---------------------------------------------------------------------------
// Results (SPEC §9.5, M7-T2)
// ---------------------------------------------------------------------------

export type TerminalOutcome = { winner: PlayerId | "draw"; reason: GameOverReason };

export type RecordResultInput = {
  matchId: string;
  /** Seat order, index 0 is p1. */
  seats: readonly [MatchSeat, MatchSeat];
  outcome: TerminalOutcome;
  turns: number;
  at: number;
  /**
   * R417, R565: each seat's board as this game ended, read from that seat's own side (seat order).
   * Absent when the writer could not read the game: the reaper (R112).
   */
  lastBoards?: readonly [readonly LastBoardEntry[], readonly LastBoardEntry[]];
};

/**
 * §9.5: "Every ending records a result and clears both players' in-match state." One row per
 * match; calling it twice for the same match is a no-op that returns the row already written.
 */
export type RecordResult = (input: RecordResultInput) => Promise<ResultRow>;

/** R678: what the actor knows of a match a Glitch voided. */
export type VoidMatchInput = {
  matchId: string;
  /** The match's two profiles, seat order as it began (`MatchRow.players`), for the log line. */
  players: readonly [string, string];
  at: number;
};

/**
 * R678: forget a voided match — no result, no rating, no record, no last board; the row and its log
 * go and both players are let go — and log the one line that names it. A voided game of a Conquest
 * series is played again. Bound at the composition root to `createVoidMatch` (`api/results.ts`).
 */
export type VoidMatch = (input: VoidMatchInput) => Promise<void>;

// ---------------------------------------------------------------------------
// The actor's dependencies
// ---------------------------------------------------------------------------

export type ActorDeps = {
  store: Store;
  timers: Timers;
  config: ServerConfig;
  log: Logger;
  engine: EnginePort;
  createClock: CreateMatchClock;
  recordResult: RecordResult;
  /** R678: what a Glitch's void outcome does instead of `recordResult`. */
  voidMatch: VoidMatch;
};
