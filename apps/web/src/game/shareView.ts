// #3609: a view arrives as fresh JSON (the wasm module's `viewFor`, a socket's view frame), so every card
// object in it is new even when the card is what it was a view ago. `Card` (Card.tsx, `sameCardProps`) and
// `useLiveFace` (faces.ts) key on a card's object, so a fresh one drew and crossed into the engine for every
// card on every view. `shareView` makes each subtree equal to one the previous view held that very object;
// only what changed is new. It reuses objects the same viewer already received, so it brings the client no
// fact it had not got (CLAUDE.md rule 7), and it never shares across a change of viewer (a hotseat hand-over).
//
// Only the two seats and the viewer's Trades are shared. `events` stay as they came: `newEventsSince`
// (animations.ts) hands events back by identity and the sound director matches the runner's entries by it.
// The root is always a new object when anything was shared, as `Game` compares views by identity.

import { useRef } from "react";

import type { PlayerView } from "@jackioh/shared";

import { isRecord } from "../wire/json.ts";

type Plain = Record<string, unknown>;

/** The instance id an item names itself by, when it is an object that has one. */
function instanceIdOf(item: unknown): string | undefined {
  if (!isRecord(item)) return undefined;
  const id = item["instanceId"];
  return typeof id === "string" ? id : undefined;
}

function shareArray(prev: unknown, next: readonly unknown[]): readonly unknown[] {
  if (!Array.isArray(prev)) return next;
  const before: readonly unknown[] = prev;
  // A card is paired with its own earlier self by instance id, so one that left (the first of a hand,
  // played) does not shift the pairing of the rest. An item with no id pairs by place.
  const byId = new Map<string, unknown>();
  for (const item of before) {
    const id = instanceIdOf(item);
    if (id !== undefined) byId.set(id, item);
  }
  const shared = next.map((item, index) => {
    const id = instanceIdOf(item);
    return shareEqual(id === undefined ? before[index] : byId.get(id), item);
  });
  return shared.length === before.length && shared.every((item, index) => item === before[index]) ? before : shared;
}

function shareObject(prev: unknown, next: Plain): Plain {
  if (!isRecord(prev)) return next;
  const keys = Object.keys(next);
  let same = keys.length === Object.keys(prev).length;
  const entries = keys.map((key) => {
    const known = Object.hasOwn(prev, key);
    const value = shareEqual(known ? prev[key] : undefined, next[key]);
    if (!known || value !== prev[key]) same = false;
    return [key, value] as const;
  });
  return same ? prev : Object.fromEntries(entries);
}

/**
 * `next`, but with every subtree deep-equal to the one at the same place in `prev` replaced by that
 * very object. The result always deep-equals `next`; `prev` itself comes back when the two are equal.
 */
export function shareEqual<T>(prev: unknown, next: T): T {
  if (Object.is(prev, next)) return next;
  if (Array.isArray(next)) return shareArray(prev, next as readonly unknown[]) as T;
  if (isRecord(next)) return shareObject(prev, next) as T;
  return next;
}

/** `next` with its seats and Trades shared with `prev`'s; `next` itself when there is nothing to share. */
export function shareView(prev: PlayerView | null, next: PlayerView): PlayerView {
  if (prev === null || prev.viewer !== next.viewer) return next;
  const you = shareEqual(prev.you, next.you);
  const opponent = shareEqual(prev.opponent, next.opponent);
  const trades = next.trades === undefined ? undefined : shareEqual(prev.trades, next.trades);
  if (you === next.you && opponent === next.opponent && trades === next.trades) return next;
  return { ...next, you, opponent, ...(trades === undefined ? {} : { trades }) };
}

/**
 * The view `Game` draws: the incoming one with what is unchanged since the last one shared. One object
 * per incoming view, however often `Game` renders, so a render that brings no new view changes nothing.
 */
export function useSharedView(incoming: PlayerView): PlayerView {
  const kept = useRef<{ incoming: PlayerView; shared: PlayerView } | null>(null);
  if (kept.current === null || kept.current.incoming !== incoming) {
    kept.current = { incoming, shared: shareView(kept.current?.shared ?? null, incoming) };
  }
  return kept.current.shared;
}
