// R437: a card that something still to come is aimed at carries a mark in both players' views —
// #50 K-Pop Fanatic's pending steal marks its target, drawn by the client as a corruption sparkle in
// the mark's colour ("purple"), one effect reusable in other colours for other marks (SPEC §10.8).
//
// A mark is made by the effect that waits: `effects/delay.ts`'s `delay({ …, watch, mark })` marks the
// card it watches (R174) as it schedules itself. A delayed destroy of a scope (Classic #20's Radiant
// face, R750) marks every card the scope names instead, kept in step with the board each time events
// are collected (`syncMarks`), so a card that arrives while it waits is marked and one that leaves is
// not. A mark lasts exactly as long as the delayed effect that made it does, so the record here is
// tied to the entry's id and holds nothing the entry does not: when the effect resolves at its R62
// point, fizzles, or is dropped because its card left the field
// (`zones.forgetWatchers`, R76, R174), the entry is gone and so is the mark. `sweepMarks` notices it
// at the next point the resolution loop collects events and says so with a `marked` event
// (`added: false`), after whatever took the entry away; the one that made the mark said
// `added: true`. `viewFor` reads `marksOn`, which reads the live entries, so the view never shows a
// mark whose effect has gone, even before the sweep has run.
//
// Hidden information: the mark was made by a public play aimed at a card its maker chose, so it
// tells neither player anything the play did not (R437). The event names the card and follows R97
// (`viewFor.redactEvent`): a face-down target is the sentinel to the player who may not read it,
// whose view carries the mark on the zone's back instead (R33).

import type { CardMark, GameEvent } from "@jackioh/shared";
import type { DelayedEffect, GameState, MarkRecord } from "./state";

/** What marking needs: the state, and the event list the `marked` event goes on. */
type MarkSink = { state: GameState; events: GameEvent[] };

function markedEvent(record: MarkRecord, added: boolean): GameEvent {
  return { type: "marked", instanceId: record.instanceId, mark: record.mark, color: record.color, added };
}

/**
 * R437: mark the card a delayed effect is aimed at (`DelayedEffect.watch`) for as long as the effect
 * waits. A delayed effect aimed at no card marks nothing.
 */
export function markDelayed(sink: MarkSink, entry: DelayedEffect, mark: CardMark): void {
  if (entry.watch === undefined) return;
  const record: MarkRecord = { instanceId: entry.watch, mark: mark.mark, color: mark.color, delayedId: entry.id };
  sink.state.marks = [...(sink.state.marks ?? []), record];
  sink.events.push(markedEvent(record, true));
}

/**
 * R750: the marks of a delayed effect that names every card of a scope (`delayedId`), made to match
 * `ids`, the cards the scope names now: a card that left it loses its mark and a card that came into
 * it gains one, each with a `marked` event, in that order. Nothing changes, and nothing is said, when
 * they already match.
 */
export function syncMarks(sink: MarkSink, delayedId: string, mark: CardMark, ids: readonly string[]): void {
  const marks = sink.state.marks ?? [];
  const held = marks.filter((record) => record.delayedId === delayedId);
  const gone = held.filter((record) => !ids.includes(record.instanceId));
  const added = ids
    .filter((id) => !held.some((record) => record.instanceId === id))
    .map((instanceId): MarkRecord => ({ instanceId, mark: mark.mark, color: mark.color, delayedId }));
  if (gone.length === 0 && added.length === 0) return;
  const kept = [...marks.filter((record) => !gone.includes(record)), ...added];
  for (const record of gone) sink.events.push(markedEvent(record, false));
  for (const record of added) sink.events.push(markedEvent(record, true));
  if (kept.length === 0) delete sink.state.marks;
  else sink.state.marks = kept;
}

function waiting(state: GameState, record: MarkRecord): boolean {
  return state.delayed.some((entry) => entry.id === record.delayedId);
}

/**
 * R437: drop the marks whose delayed effect is no longer waiting — it resolved, fizzled, or was
 * forgotten as its card left the field — each with a `marked` event (`added: false`), in the order
 * the marks were made. Called where the resolution loop collects events (`triggers.collectEvents`),
 * so the removal follows the event that ended it and is dispatched with the rest.
 */
export function sweepMarks(sink: MarkSink): void {
  const marks = sink.state.marks;
  if (marks === undefined) return;
  const kept = marks.filter((record) => waiting(sink.state, record));
  if (kept.length === marks.length) return;
  for (const record of marks) {
    if (!kept.includes(record)) sink.events.push(markedEvent(record, false));
  }
  if (kept.length === 0) delete sink.state.marks;
  else sink.state.marks = kept;
}

/** R437: the marks a card carries now, in the order they were made — only those still waiting. */
export function marksOn(state: GameState, instanceId: string): CardMark[] {
  return (state.marks ?? [])
    .filter((record) => record.instanceId === instanceId && waiting(state, record))
    .map((record) => ({ mark: record.mark, color: record.color }));
}
