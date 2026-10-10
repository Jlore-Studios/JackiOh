// Game-log history (SPEC §10.10, R745) joins each viewer's event window (§10.8, R168).
// It retains event-time wording and faces—not instance ids—so no card is followed into a hidden zone (R97, R223).
// Per-viewer histories preserve redaction; reconnects or missed windows add a gap (§9.5).
// Presentation only: nothing reaches the engine, view or wire (CLAUDE.md rule 7).

import { useState } from "react";

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { FaceModel } from "../cards/index.ts";
import { newEventsSince } from "./animations.ts";
import { LOG_HISTORY_LIMIT } from "./config.ts";

export const LOG_GAP = "gap";
export const LOG_GAP_TEXT = "Some events may be missing here";

export type LoggedLine = {
  readonly type: GameEvent["type"] | typeof LOG_GAP;
  readonly text: string;
  readonly face: FaceModel | null;
};
export type KeptLine = LoggedLine & { readonly key: string };
export type SeatLog = {
  readonly events: readonly GameEvent[];
  readonly window: readonly { readonly key: string; readonly line: KeptLine | null }[];
  readonly kept: readonly KeptLine[];
  readonly next: number;
};
export type LogHistory = {
  readonly view: PlayerView | null;
  readonly seats: Readonly<Partial<Record<PlayerId, SeatLog>>>;
};
export const EMPTY_LOG_HISTORY: LogHistory = { view: null, seats: {} };
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

export function useLogHistory(view: PlayerView, keep: (event: GameEvent) => LoggedLine | null): ViewerLog {
  const [history, setHistory] = useState<LogHistory>(EMPTY_LOG_HISTORY);
  const current = advanceLogHistory(history, view, keep);
  // Set while rendering, as Game.tsx resets its concede question: React re-renders before committing.
  if (current !== history) setHistory(current);
  const seat = current.seats[view.viewer];
  return { kept: seat?.kept ?? [], keys: seat?.window.map((entry) => entry.key) ?? [] };
}
