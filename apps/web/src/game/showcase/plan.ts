// What the showcase holds up, decided from the redacted event stream alone (CLAUDE.md rule 7,
// R97, R202). Pure: it reads its arguments and returns data, so every rule about what may be shown
// is tested here without a DOM.
//
// The trigger is the opponent's `cardPlayed`: Hearthstone shows the card the other player has just
// played, big, for a moment, so it can be read before the game moves on. The viewer's own plays
// are never shown (they chose them), and neither is anything the view does not name: a `cardPlayed`
// the view redacts — a Trap or Field Trap set face down (R227), or any card that has since gone
// somewhere this viewer may not read — carries the sentinel for its definition and becomes a card
// back with a caption, never a face.
//
// R502: a card cast the moment it was drawn is held up on BOTH seats, the drawer's own too: nobody
// chose it, and it never passed through a hand where it could have been read. It is read off the order
// of the redacted events (`castOnDraw.ts`), so a hidden one is a back that says a card was cast as it
// was drawn, never which.
//
// Issue #124: so is every card another card casts (C+ #47 Jogg's Box's ten, Solarius Prime's five, a
// Cry that casts a card), on both seats, with the card that cast it and which of its casts it is: a
// play the stream begins while another is still resolving (`runs.ts`). Nobody chose those either, and
// ten of them in one action cannot be followed unless each is shown.
//
// Which events are new. `view.events` is §10.8's sliding window, and R97 re-judges its redaction on
// every view by where each card sits NOW: the opponent's `drawn` reads as the sentinel while the card
// is in their hand and names the card once it has been played. So two windows that share their
// events do not share their JSON, and a plain comparison finds no overlap and calls the whole window
// new, which would hold up every card the opponent played in the last thirty-odd events again. The
// overlap here is found with `sameOccurrence`, which lets a redacted field match a revealed one.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { castOnDrawAt } from "../../fx/castOnDraw.ts";
import { chaosRollOf, type ChaosRoll } from "../../fx/chaos.ts";
import { sideOf } from "../contract.ts";
import { createPlayTracker, type PlayTracker } from "../runs.ts";
import { HIDDEN_CARD, cardInView } from "../faces.ts";

/** One card the showcase holds up. `defId` is null exactly when the view hides the card. */
export type ShowcasePlay = {
  player: PlayerId;
  /** The card's definition, or null for a card the view redacts: it is drawn as a back. */
  defId: string | null;
  /**
   * The played instance and what its play cost, for a card the view names: the face is the card in
   * play (SPEC §10.10), as it stands where the view still lists it, else at the price it was paid.
   */
  instanceId?: string;
  costPaid?: number;
  /** The face that was played, as far as the view says (its resolution, else where it stands now). */
  radiant: boolean;
  /** A hidden play that put a card face down into a backrow: "set a card" rather than "played a card". */
  set: boolean;
  /**
   * R370: for a card set face down, the cost its back shows, read off the view's backrow where it
   * landed while it still stands there face-down. Absent when the view gives none.
   */
  cost?: number;
  /** R502: the card was cast the moment it was drawn (castOnDraw.ts). Absent for every other play. */
  castOnDraw?: true;
  /**
   * Issue #124: another card cast it: that card's definition (null when the view hides it) and which
   * of its casts this is, 1 on. Absent for every other play.
   */
  castBy?: { defId: string | null; ordinal: number };
};

/** The fields a redacted event keeps as they are (R97): who, and where. Everything else may be the sentinel's. */
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

/**
 * Whether two events are the same occurrence as seen in two views. Identical events are. So are two
 * of the same type about the same seats when either one carries the sentinel: R97 redacts an
 * event's card fields (and the numbers that would name the card, R177) and leaves its seats alone,
 * and a later view may read a card an earlier one could not.
 */
export function sameOccurrence(a: GameEvent | undefined, b: GameEvent | undefined): boolean {
  if (a === undefined || b === undefined || a.type !== b.type) return false;
  if (JSON.stringify(a) === JSON.stringify(b)) return true;
  return (mentionsSentinel(a) || mentionsSentinel(b)) && seatKey(a) === seatKey(b);
}

/**
 * The events of `next` that `prev` did not have: everything after the longest suffix of `prev`
 * that is a prefix of `next`. Two windows with nothing in common (a view that is more than a window
 * of events behind) make all of `next` new, which is the honest answer.
 */
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

/** The face the card resolved with (`cardResolved.radiant`), else the one the view shows it with now. */
function radiantOf(play: Played, fresh: readonly GameEvent[], view: PlayerView): boolean {
  const resolved = fresh.find(
    (event): event is Extract<GameEvent, { type: "cardResolved" }> =>
      event.type === "cardResolved" && event.instanceId === play.instanceId,
  );
  if (resolved?.radiant !== undefined) return resolved.radiant;
  return cardInView(view, play.instanceId)?.radiant ?? false;
}

/**
 * A hidden play whose own `summoned` put it into a backrow: the view says a card was set there, and
 * in which lane, no more. Null when the play set nothing.
 */
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

/** One `cardPlayed` of `fresh` (at index `at`), as the view lets the viewer see it. */
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

/** The opponent's plays among `fresh`, in order, each as the view lets the viewer see it. */
export function opponentPlays(fresh: readonly GameEvent[], view: PlayerView): ShowcasePlay[] {
  const plays: ShowcasePlay[] = [];
  fresh.forEach((event, at) => {
    if (event.type !== "cardPlayed" || sideOf(view, event.player) !== "opponent") return;
    plays.push(playOf(event, at, fresh, view));
  });
  return plays;
}

/** A play the showcase holds up, and the `cardPlayed` it came from (the very object the view holds). */
export type ShowcaseItem = { play: ShowcasePlay; event: GameEvent };

/**
 * Where `fresh` sits in the view's window: its first index there when it is the window's tail (the
 * events before it are what a cast on draw is read off), else null.
 */
function tailOffset(fresh: readonly GameEvent[], window: readonly GameEvent[]): number | null {
  const offset = window.length - fresh.length;
  if (offset < 0) return null;
  return fresh.every((event, i) => window[offset + i] === event) ? offset : null;
}

/**
 * R502: what the showcase holds up among `fresh`, in order: every play of the opponent, as
 * `opponentPlays` finds them, and every cast on draw on either seat, the viewer's own included (the
 * viewer did not choose it, and it never passed through their hand). A cast on draw is read off the
 * view's whole window, so a `drawn` that came in an earlier view still counts; a hidden one is a back
 * (R97, R202).
 *
 * Issue #124: and every card another card cast, on either seat, with `castBy`. `plays` is the tracker
 * of the plays still resolving, which must read every event of the viewer's stream in order (the
 * showcase keeps one per viewer); without one, only the casts inside `fresh` itself are found.
 */
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

/**
 * Keeps the queue to the newest `max` plays. A hotseat hand-over can bring a whole turn's plays at
 * once; the ones that fall off are in the log.
 */
export function capQueue<T>(queue: readonly T[], max: number): T[] {
  return queue.length <= max ? [...queue] : queue.slice(queue.length - max);
}
