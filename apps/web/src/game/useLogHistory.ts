// The game log's history (SPEC §10.10, R744).
//
// `view.events` is a window sized for animation (§10.8, R168): after a busy turn the player's own
// last play has left it. So the log joins each view's window to the last window the same viewer
// was shown, with the animation runner's own diff (`newEventsSince`), and keeps every line that
// leaves the window.
//
// A line is kept as it read when its event arrived: its words and the face of the definition it
// names, never an instance id, so the history holds only what a view once said and follows no card
// into a hidden zone (R97, R223). There is one history per viewer, because the seats read
// differently redacted windows and hotseat's two seats must never share one. A view that shares no
// event with the last one (a reconnect after a long drop, §9.5, or a board that caught up past a
// whole window) cannot be joined: the history keeps what it had and adds one line saying so. A
// view with no result after one with a result starts a new game, and its history empty.
//
// Presentation only: nothing here reaches the engine, the view or the wire (CLAUDE.md rule 7).

import { useState } from "react";

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { FaceModel } from "../cards/index.ts";
import { newEventsSince } from "./animations.ts";
import { LOG_HISTORY_LIMIT } from "./config.ts";

/** The `type` of the line that stands where two windows could not be joined. */
export const LOG_GAP = "gap";
/** What that line says. */
export const LOG_GAP_TEXT = "Some events may be missing here";

/** A line as the log keeps it: its words and the face of the definition it names, never an instance id. */
export type LoggedLine = {
  readonly type: GameEvent["type"] | typeof LOG_GAP;
  readonly text: string;
  readonly face: FaceModel | null;
};
/** A kept line and the key it is drawn under, which it keeps as the window slides. */
export type KeptLine = LoggedLine & { readonly key: string };
/** One viewer's history. */
export type SeatLog = {
  /** The window this viewer's last view carried. */
  readonly events: readonly GameEvent[];
  /** One entry per event of `events`: its key, and the line kept for it when it arrived (null: none). */
  readonly window: readonly { readonly key: string; readonly line: KeptLine | null }[];
  /** The lines whose events have left the window, oldest first. */
  readonly kept: readonly KeptLine[];
  /** The number the next key takes. */
  readonly next: number;
};
export type LogHistory = {
  readonly view: PlayerView | null;
  readonly seats: Readonly<Partial<Record<PlayerId, SeatLog>>>;
};
export const EMPTY_LOG_HISTORY: LogHistory = { view: null, seats: {} };
/** What the log draws for its viewer: the kept lines, then one key per event of the view's window. */
export type ViewerLog = { readonly kept: readonly KeptLine[]; readonly keys: readonly string[] };

export function advanceLogHistory(
  history: LogHistory,
  view: PlayerView,
  keep: (event: GameEvent) => LoggedLine | null,
): LogHistory {
  if (history.view === view) return history;
  // A view with no result after one with a result is a new game (as stats/useGameStats.ts reads it).
  const seats: LogHistory["seats"] =
    history.view !== null && history.view.result !== null && view.result === null ? {} : history.seats;
  const seat = view.viewer;
  const before = seats[seat];
  let next = before?.next ?? 0;
  const key = (): string => {
    const made = `${seat}-${String(next)}`;
    next += 1;
    return made;
  };
  const arrive = (event: GameEvent): SeatLog["window"][number] => {
    const made = key();
    const line = keep(event);
    return { key: made, line: line === null ? null : { ...line, key: made } };
  };
  if (before === undefined) {
    const window = view.events.map(arrive);
    return { view, seats: { ...seats, [seat]: { events: view.events, window, kept: [], next } } };
  }
  const fresh = newEventsSince(before.events, view.events);
  const overlap = view.events.length - fresh.length;
  const left = before.events.length - overlap;
  const joined = overlap > 0 || before.events.length === 0 || view.events.length === 0;
  const gap: KeptLine[] = joined ? [] : [{ key: key(), type: LOG_GAP, text: LOG_GAP_TEXT, face: null }];
  const kept = [
    ...before.kept,
    ...before.window.slice(0, left).flatMap((entry) => (entry.line === null ? [] : [entry.line])),
    ...gap,
  ].slice(-LOG_HISTORY_LIMIT);
  const window = [...before.window.slice(left), ...fresh.map(arrive)];
  return { view, seats: { ...seats, [seat]: { events: view.events, window, kept, next } } };
}

/** The log's history for `view`'s viewer; `keep` is called only for events new to that viewer. */
export function useLogHistory(view: PlayerView, keep: (event: GameEvent) => LoggedLine | null): ViewerLog {
  const [history, setHistory] = useState<LogHistory>(EMPTY_LOG_HISTORY);
  const current = advanceLogHistory(history, view, keep);
  // Set while rendering, as Game.tsx resets its concede question: React re-renders before committing.
  if (current !== history) setHistory(current);
  const seat = current.seats[view.viewer];
  return { kept: seat?.kept ?? [], keys: seat?.window.map((entry) => entry.key) ?? [] };
}
