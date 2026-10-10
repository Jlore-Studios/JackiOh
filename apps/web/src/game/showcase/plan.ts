// Derives showcase data only from redacted events (CLAUDE.md rule 7; R97, R202, R227).
// It shows opponent plays, R502 casts on draw on both seats, and casts by another card (C+ #47 Jogg's Box).
// `view.events` is §10.8's re-redacted sliding window, so `sameOccurrence` matches redacted and revealed events.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { castOnDrawAt } from "../../fx/castOnDraw.ts";
import { chaosRollOf, type ChaosRoll } from "../../fx/chaos.ts";
import { sideOf } from "../contract.ts";
import { createPlayTracker, type PlayTracker } from "../runs.ts";
import { HIDDEN_CARD, cardInView } from "../faces.ts";

/** One card the showcase holds up. `defId` is null exactly when the view hides the card. */
export type ShowcasePlay = {
  player: PlayerId;
  defId: string | null;
  /** SPEC §10.10: current face, or paid-cost definition once absent from the view. */
  instanceId?: string;
  costPaid?: number;
  radiant: boolean;
  set: boolean;
  /** R370: face-down backrow cost, when still visible. */
  cost?: number;
  /** R502: set only for cards cast on draw. */
  castOnDraw?: true;
  /** Another card cast it: that card's definition (null when the view hides it) and which of its casts this is, 1 on. */
  castBy?: { defId: string | null; ordinal: number };
};

/** R97 keeps event seats, even when card fields are redacted. */
const SEAT_FIELDS = ["player", "owner", "controller", "row", "lane", "turn"] as const;

function mentionsSentinel(event: GameEvent): boolean {
  for (const value of Object.values(event)) {
    if (value === HIDDEN_CARD) return true;
    if (Array.isArray(value) && value.includes(HIDDEN_CARD)) return true;
  }
  return false;
}

function seatKey(event: GameEvent): string {
  const record = event as Record<string, unknown>;
  return JSON.stringify([event.type, ...SEAT_FIELDS.map((field) => record[field] ?? null)]);
}

/** R97 and R177 may redact a later view differently; matching seats preserves event identity. */
export function sameOccurrence(a: GameEvent | undefined, b: GameEvent | undefined): boolean {
  if (a === undefined || b === undefined || a.type !== b.type) return false;
  if (JSON.stringify(a) === JSON.stringify(b)) return true;
  return (mentionsSentinel(a) || mentionsSentinel(b)) && seatKey(a) === seatKey(b);
}

export function eventsSince(prev: readonly GameEvent[], next: readonly GameEvent[]): GameEvent[] {
  if (prev.length === 0 || next.length === 0) return [...next];
  for (let overlap = Math.min(prev.length, next.length); overlap > 0; overlap -= 1) {
    const from = prev.length - overlap;
    let matches = true;
    for (let i = 0; i < overlap && matches; i += 1) matches = sameOccurrence(prev[from + i], next[i]);
    if (matches) return next.slice(overlap);
  }
  return [...next];
}

type Played = Extract<GameEvent, { type: "cardPlayed" }>;

function radiantOf(play: Played, fresh: readonly GameEvent[], view: PlayerView): boolean {
  const resolved = fresh.find(
    (event): event is Extract<GameEvent, { type: "cardResolved" }> =>
      event.type === "cardResolved" && event.instanceId === play.instanceId,
  );
  if (resolved?.radiant !== undefined) return resolved.radiant;
  return cardInView(view, play.instanceId)?.radiant ?? false;
}

/** Finds a hidden play's backrow summon before another play. */
function setLane(play: Played, at: number, fresh: readonly GameEvent[]): number | null {
  for (let i = at + 1; i < fresh.length; i += 1) {
    const event = fresh[i];
    if (event === undefined) continue;
    if (event.type === "cardPlayed") return null;
    if (event.type === "summoned" && event.player === play.player && event.instanceId === HIDDEN_CARD) {
      return event.row === "backrow" ? event.lane : null;
    }
  }
  return null;
}

/** R370: the cost the view gives the face-down card in that backrow lane, if one still stands there. */
function faceDownCost(view: PlayerView, player: PlayerId, lane: number): number | undefined {
  const seat = sideOf(view, player) === "you" ? view.you : view.opponent;
  const entry = seat.backrow[lane - 1];
  if (entry === null || entry === undefined || !entry.faceDown) return undefined;
  return "cost" in entry && typeof entry.cost === "number" ? entry.cost : undefined;
}

function playOf(event: Played, at: number, fresh: readonly GameEvent[], view: PlayerView): ShowcasePlay {
  const hidden = event.defId === HIDDEN_CARD || event.instanceId === HIDDEN_CARD;
  const lane = hidden ? setLane(event, at, fresh) : null;
  const cost = lane === null ? undefined : faceDownCost(view, event.player, lane);
  return hidden
    ? { player: event.player, defId: null, radiant: false, set: lane !== null, ...(cost === undefined ? {} : { cost }) }
    : {
        player: event.player,
        defId: event.defId,
        instanceId: event.instanceId,
        costPaid: event.costPaid,
        radiant: radiantOf(event, fresh, view),
        set: false,
      };
}

export function opponentPlays(fresh: readonly GameEvent[], view: PlayerView): ShowcasePlay[] {
  const plays: ShowcasePlay[] = [];
  fresh.forEach((event, at) => {
    if (event.type !== "cardPlayed" || sideOf(view, event.player) !== "opponent") return;
    plays.push(playOf(event, at, fresh, view));
  });
  return plays;
}

export type ShowcaseItem = { play: ShowcasePlay; event: GameEvent };

/** The tail offset keeps earlier draw events available to cast-on-draw detection. */
function tailOffset(fresh: readonly GameEvent[], window: readonly GameEvent[]): number | null {
  const offset = window.length - fresh.length;
  if (offset < 0) return null;
  return fresh.every((event, i) => window[offset + i] === event) ? offset : null;
}

/** R502 includes both seats' casts on draw; per-viewer trackers also find chained casts (R97, R202). */
export function showcasePlays(
  fresh: readonly GameEvent[],
  view: PlayerView,
  plays: PlayTracker = createPlayTracker(),
): ShowcaseItem[] {
  const offset = tailOffset(fresh, view.events);
  const window = offset === null ? fresh : view.events;
  const base = offset ?? 0;
  const items: ShowcaseItem[] = [];
  fresh.forEach((event, at) => {
    const cast = plays.see(event);
    if (event.type !== "cardPlayed") return;
    const onDraw = castOnDrawAt(window, base + at);
    if (!onDraw && cast === null && sideOf(view, event.player) !== "opponent") return;
    const play = playOf(event, at, fresh, view);
    if (onDraw) items.push({ play: { ...play, castOnDraw: true }, event });
    else if (cast !== null) {
      const by = cast.by.defId === HIDDEN_CARD ? null : cast.by.defId;
      items.push({ play: { ...play, castBy: { defId: by, ordinal: cast.ordinal } }, event });
    } else items.push({ play, event });
  });
  return items;
}

/** R436: the Call to Chaos rolls among `fresh`, in order, with the event each came from. */
export function chaosRollsIn(fresh: readonly GameEvent[]): { roll: ChaosRoll; event: GameEvent }[] {
  return fresh.flatMap((event) => {
    const roll = chaosRollOf(event);
    return roll === null || roll.effects.length === 0 ? [] : [{ roll, event }];
  });
}

/** Keeps the queue to the newest `max` plays; the ones that fall off are in the log. */
export function capQueue<T>(queue: readonly T[], max: number): T[] {
  return queue.length <= max ? [...queue] : queue.slice(queue.length - max);
}
