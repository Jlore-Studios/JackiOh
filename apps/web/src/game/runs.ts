// CLAUDE.md rule 7: the runner derives casts (§6.3, R70), pile impacts, and sweeps from redacted events and PlayerView.
// R97, R202, R242, and R440 forbid hidden-card inference; `HIDDEN_PILE_OF` uses only public card text.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { sideOf, type Side } from "./contract";

/** R97's sentinel (engine `viewFor.ts` `HIDDEN_ID`). */
const HIDDEN = "hidden";

const SIDES: readonly Side[] = ["you", "opponent"];

/** The most plays the tracker keeps open: deeper nesting than this is a stream it has lost track of. */
export const PLAYS_OPEN_MAX = 16;

// Casts

/** A play the stream has opened and not yet resolved, and how many casts it has made so far. */
export type OpenPlay = { player: PlayerId; instanceId: string; defId: string; casts: number };

/** A cast: the play that made it (as it stood after this cast) and which of its casts this is (1 on). */
export type Cast = { by: OpenPlay; ordinal: number };

export type PlayTracker = {
  /** The innermost play still resolving, or undefined. */
  current(): OpenPlay | undefined;
  /** Reads one event, in stream order; a `cardPlayed` that is a cast returns who cast it. */
  see(event: GameEvent): Cast | null;
  clear(): void;
};

/**
 * Tracks open plays. R97 may close the innermost play; B5 E1 counters before §10.5's `cardPlayed`, so it closes only an exact match.
 */
export function createPlayTracker(): PlayTracker {
  let open: OpenPlay[] = [];

  const close = (player: PlayerId, instanceId: string, exact: boolean): void => {
    for (let i = open.length - 1; i >= 0; i -= 1) {
      const play = open[i];
      if (play === undefined || play.player !== player) continue;
      const named = play.instanceId === instanceId;
      if (named || (!exact && (play.instanceId === HIDDEN || instanceId === HIDDEN))) {
        open.splice(i, 1);
        return;
      }
    }
  };

  return {
    current() {
      const top = open[open.length - 1];
      return top === undefined ? undefined : { ...top };
    },
    see(event) {
      switch (event.type) {
        case "cardPlayed": {
          const parent = open[open.length - 1];
          let cast: Cast | null = null;
          if (parent !== undefined && event.costPaid === 0) {
            parent.casts += 1;
            cast = { by: { ...parent }, ordinal: parent.casts };
          }
          open.push({ player: event.player, instanceId: event.instanceId, defId: event.defId, casts: 0 });
          if (open.length > PLAYS_OPEN_MAX) open = open.slice(open.length - PLAYS_OPEN_MAX);
          return cast;
        }
        case "cardResolved":
          close(event.player, event.instanceId, false);
          return null;
        case "countered":
          close(event.player, event.instanceId, true);
          return null;
        case "turnStarted":
        case "gameOver":
          open = [];
          return null;
        default:
          return null;
      }
    },
    clear() {
      open = [];
    },
  };
}

// Whole-pile impacts

export type PileKind = "library" | "graveyard" | "exile";

/** One pile: whose (viewer-relative) and which. */
export type Pile = { side: Side; pile: PileKind };

/**
 * A whole-pile impact as the runner plays it: the pile, how many cards it held as the run began, and
 * how many events the run made (a pile changed twice over makes twice its count).
 */
export type ZoneImpact = Pile & { count: number; events: number };

/**
 * R440 hidden changes map only where public text names one library; multi-pile cards (C+ #73) stay unmapped.
 */
export const HIDDEN_PILE_OF: Readonly<Record<string, "own" | "enemy">> = {
  // C+ #8 Withering Storm changes the opponent's deck.
  "classicplus-008": "enemy",
  // Core #42: "Exile 7 random cards from your deck".
  "core-042": "own",
};

/** Events that change a card where it lies, and so land in whatever pile holds it. */
const IN_PILE = new Set<GameEvent["type"]>([
  "degraded",
  "upgraded",
  "numberChanged",
  "costChanged",
  "counterChanged",
  "buffed",
  "keywordGranted",
  "transformed",
]);

function seat(view: PlayerView, side: Side): PlayerView["you"] {
  return side === "you" ? view.you : view.opponent;
}

/** How many cards a pile holds in `view`. */
export function pileCount(view: PlayerView, pile: Pile): number {
  const sv = seat(view, pile.side);
  if (pile.pile === "library") return sv.libraryCount;
  return pile.pile === "graveyard" ? sv.graveyard.length : sv.exile.length;
}

/** The public pile (a graveyard or an exile) `view` lists the card in, or null. */
function publicPileOf(view: PlayerView, instanceId: string): Pile | null {
  for (const side of SIDES) {
    const sv = seat(view, side);
    if (sv.graveyard.some((card) => card.instanceId === instanceId)) return { side, pile: "graveyard" };
    if (sv.exile.some((card) => card.instanceId === instanceId)) return { side, pile: "exile" };
  }
  return null;
}

/** Whether `view` shows the card anywhere: on the field, in a hand the viewer reads, resolving, in a public pile. */
function shown(view: PlayerView, instanceId: string): boolean {
  if (publicPileOf(view, instanceId) !== null) return true;
  for (const side of SIDES) {
    const sv = seat(view, side);
    if (sv.units.some((unit) => unit !== null && unit.instanceId === instanceId)) return true;
    if (sv.carried?.some((unit) => unit !== null && unit.instanceId === instanceId) === true) return true;
    if (sv.backrow.some((slot) => slot !== null && !slot.faceDown && slot.instanceId === instanceId)) return true;
    if (sv.resolving.some((card) => card.instanceId === instanceId)) return true;
    if (Array.isArray(sv.hand) && sv.hand.some((card) => card.instanceId === instanceId)) return true;
  }
  return false;
}

/** The other player. */
function otherOf(player: PlayerId): PlayerId {
  return player === "p1" ? "p2" : "p1";
}

/**
 * `inferred` piles count whole only on an exact run, because the view did not show the card's origin.
 */
export function pileOf(
  event: GameEvent,
  view: PlayerView,
  play: OpenPlay | undefined,
): (Pile & { inferred?: true }) | null {
  switch (event.type) {
    case "exiled": {
      if (event.instanceId === HIDDEN) return null;
      const held = publicPileOf(view, event.instanceId);
      if (held !== null) return held.pile === "graveyard" ? held : null;
      if (shown(view, event.instanceId)) return null;
      // Only an own hidden card can be inferred as library: an opponent's could be from hand or backrow.
      const side = sideOf(view, event.owner);
      return side === "you" ? { side, pile: "library", inferred: true } : null;
    }
    case "shuffledIn":
      // `shuffledIn` is certainly in the player's library, even for R97's sentinel.
      return { side: sideOf(view, event.player), pile: "library" };
    case "stolen":
      return event.zone === "library" || event.zone === "graveyard" || event.zone === "exile"
        ? { side: sideOf(view, event.from), pile: event.zone }
        : null;
    case "radiantSet": {
      const z = event.zone.z;
      return z === "library" || z === "graveyard" || z === "exile" ? { side: sideOf(view, event.zone.player), pile: z } : null;
    }
    default:
      break;
  }
  if (!IN_PILE.has(event.type) || !("instanceId" in event)) return null;
  const id = event.instanceId;
  if (id !== HIDDEN) return publicPileOf(view, id);
  if (play === undefined || play.defId === HIDDEN) return null;
  if (!Object.prototype.hasOwnProperty.call(HIDDEN_PILE_OF, play.defId)) return null;
  const owner = HIDDEN_PILE_OF[play.defId] === "enemy" ? otherOf(play.player) : play.player;
  return { side: sideOf(view, owner), pile: "library" };
}

function samePile(a: Pile, b: Pile): boolean {
  return a.side === b.side && a.pile === b.pile;
}

/**
 * A same-type, same-pile run is whole at two or more events and the pile count (exactly for inferred piles).
 */
export function pileRunAt(
  events: readonly GameEvent[],
  at: number,
  view: PlayerView,
  play: OpenPlay | undefined,
): { pile: Pile; length: number; whole: boolean; count: number } | null {
  const first = events[at];
  if (first === undefined) return null;
  const pile = pileOf(first, view, play);
  if (pile === null) return null;
  let length = 1;
  let inferred = pile.inferred === true;
  for (let i = at + 1; i < events.length; i += 1) {
    const event = events[i];
    if (event === undefined || event.type !== first.type) break;
    const next = pileOf(event, view, play);
    if (next === null || !samePile(next, pile)) break;
    if (next.inferred === true) inferred = true;
    length += 1;
  }
  const count = pileCount(view, pile);
  const whole = length >= 2 && count > 0 && (inferred ? length === count : length >= count);
  return { pile: { side: pile.side, pile: pile.pile }, length, whole, count };
}

// Sweeps

/** A sweep as the runner plays it: hits or heals, the sides every unit of which it reached, and its source. */
export type Sweep = { tone: "damage" | "heal"; sides: readonly Side[]; sourceId: string | null };

const HERO_ID = /^hero-(p1|p2)$/;

/** The side a hit or heal target is on in `view`: a hero, or a unit on top of its pile. Null otherwise. */
function targetSide(view: PlayerView, targetId: string): { side: Side; unit: boolean } | null {
  if (targetId === HIDDEN) return null;
  const hero = HERO_ID.exec(targetId)?.[1];
  if (hero !== undefined) return { side: sideOf(view, hero as PlayerId), unit: false };
  for (const side of SIDES) {
    if (seat(view, side).units.some((unit) => unit !== null && unit.instanceId === targetId)) return { side, unit: true };
  }
  return null;
}

/** One step of a sweep: the card or hero it reaches, and whether it is a hit, a heal or a shield taking a hit. */
function sweepStep(event: GameEvent): { target: string; tone: Sweep["tone"]; source?: string | null } | null {
  if (event.type === "damage") return event.combat ? null : { target: event.targetId, tone: "damage", source: event.sourceId };
  if (event.type === "divineShieldLost") return { target: event.instanceId, tone: "damage" };
  // R1361: a hit the Armor took whole still reached its target, as a Divine Shield's does.
  if (event.type === "damageAbsorbed") return event.combat ? null : { target: event.targetId, tone: "damage", source: event.sourceId };
  if (event.type === "healed") return { target: event.targetId, tone: "heal" };
  return null;
}

/**
 * A sweep reaches every unit on each touched side with distinct non-combat hits or heals.
 */
export function sweepAt(
  events: readonly GameEvent[],
  at: number,
  view: PlayerView,
): { sweep: Sweep; length: number } | null {
  const first = events[at];
  const head = first === undefined ? null : sweepStep(first);
  if (first === undefined || head === null) return null;
  const tone = head.tone;
  let source: string | null | undefined = head.source;
  const reached = new Map<string, { side: Side; unit: boolean }>();
  let hits = 0;
  let i = at;
  for (; i < events.length; i += 1) {
    const event = events[i];
    const step = event === undefined ? null : sweepStep(event);
    if (event === undefined || step === null || step.tone !== tone || reached.has(step.target)) break;
    if (step.source !== undefined) {
      if (source === undefined) source = step.source;
      else if (step.source !== source) break;
    }
    const where = targetSide(view, step.target);
    if (where === null) break;
    reached.set(step.target, where);
    if (event.type === "damage" || event.type === "healed") hits += 1;
  }
  const length = i - at;
  if (length < 2 || hits === 0) return null;
  const touched = SIDES.filter((side) => [...reached.values()].some((r) => r.side === side));
  if (![...reached.values()].some((r) => r.unit)) return null;
  for (const side of touched) {
    const every = seat(view, side).units.every((unit) => unit === null || reached.has(unit.instanceId));
    if (!every) return null;
  }
  return { sweep: { tone, sides: touched, sourceId: source ?? null }, length };
}
