// What the animation runner reads off a burst beyond one event at a time (issue #124): which plays are
// casts, which runs of events hit a whole pile, and which runs of hits sweep a whole side.
//
// - A cast (§6.3 Cast, R70) is a play begun while another play is still resolving: Jogg's Box's ten
//   random Spells, Solarius Prime's five, a Cry that casts a card. Each one is a `cardPlayed` with cost
//   paid 0 that the stream opens inside another play's `cardPlayed` … `cardResolved`, so the tracker
//   below keeps the plays still open, across batches (a cast can wait behind a prompt the other seat
//   answers), and says which play cast it and which of its casts it is. The runner holds each cast up
//   long enough to read (`CAST_ENTRY_MS`), and the showcase holds the cast card up on both seats.
// - A whole-pile impact: a run of events of one type that all land in one Deck, Graveyard or Exile and
//   reach every card in it plays one "affecting this zone" entry instead of one per card; a run that
//   reaches fewer cards than the pile holds (a card naming a number of them) still plays one per card.
// - A sweep: a run of non-combat hits from one source (or of heals) that reaches every unit one side
//   shows, its hero included or not, plays as one entry the effects layer rolls a fog over.
//
// No rule lives here (CLAUDE.md rule 7). Everything is read off the redacted events and the view the
// entry is planned against: the plays the stream opens and closes, the ids an event names, the piles'
// public counts and the units each side shows. A card the view hides is never looked up (R97, R202).
// Which pile a hidden card's change lands in is never in the view (R242, R440 hide it on purpose), so
// it is read only off a card whose public text names that pile (`HIDDEN_PILE_OF`).
//
// Pure, apart from the tracker's own memory.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { sideOf, type Side } from "./contract";

/** R97's sentinel (engine `viewFor.ts` `HIDDEN_ID`). */
const HIDDEN = "hidden";

const SIDES: readonly Side[] = ["you", "opponent"];

/** The most plays the tracker keeps open: deeper nesting than this is a stream it has lost track of. */
export const PLAYS_OPEN_MAX = 16;

/* ------------------------------------------------------------------------------------------- *
 * Casts
 * ------------------------------------------------------------------------------------------- */

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
 * Tracks the plays still resolving. A `cardPlayed` opens one, and is a cast when another is open and
 * it paid nothing (a free play of the player's own never happens inside another play). Its
 * `cardResolved` closes it; R97's sentinel closes the innermost play of that player, as it may name
 * any. A `countered` play never opened (B5 E1 cancels it before §10.5 step 4's `cardPlayed`), so it
 * closes only a play it names exactly. A turn starting or the game ending clears whatever a lost
 * event left open.
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

/* ------------------------------------------------------------------------------------------- *
 * Whole-pile impacts
 * ------------------------------------------------------------------------------------------- */

export type PileKind = "library" | "graveyard" | "exile";

/** One pile: whose (viewer-relative) and which. */
export type Pile = { side: Side; pile: PileKind };

/**
 * A whole-pile impact as the runner plays it: the pile, how many cards it held as the run began, and
 * how many events the run made (a pile changed twice over makes twice its count).
 */
export type ZoneImpact = Pile & { count: number; events: number };

/**
 * The cards whose changes to cards nobody may read land in one library their public text names
 * (R440: each such change is reported, its card hidden, its pile not said), relative to the player of
 * the card resolving: `enemy` for the other player's library, `own` for its own. A card whose hidden
 * changes reach more than one pile (a hand and a deck, C+ #73's Upgrade) is not listed: its changes
 * stay where the view puts them, which is nowhere. Add a definition here when its text names the one
 * deck its hidden changes are in.
 */
export const HIDDEN_PILE_OF: Readonly<Record<string, "own" | "enemy">> = {
  // C+ #8 Withering Storm: base "Degrade 4 random cards in your opponent's deck", radiant "Degrade
  // every card in your opponent's deck".
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
 * Where an event lands, when it lands in a pile: the pile the card was in as it changed, or the pile
 * a card was taken from. `inferred` marks a pile read off a card the view shows nowhere (an exile out
 * of a library); such a pile counts as reached whole only when the run is exactly its size. `play` is
 * the innermost play still resolving, whose public text may name the pile of a hidden change.
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
      // A card shown nowhere that left for exile: a library card, a hand card, or a face-down
      // backrow card. The viewer's own hand and resolving cards are shown, so for the viewer's own
      // cards this is their library (or, rarely, their face-down backrow); an opponent's hidden hand
      // and backrow make the same guess unsound for opponent cards (e.g. an exiled hand card), so
      // those stay unmapped and play singly.
      const side = sideOf(view, event.owner);
      return side === "you" ? { side, pile: "library", inferred: true } : null;
    }
    case "shuffledIn":
      // Every producer lands the card in `player`'s library (draw, setup, zones' graveyard landing),
      // so the pile is certain even when the card is R97's sentinel — no inference.
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
 * The run of events from `at` that share its type and its pile, and whether it reaches the whole pile:
 * at least two events, and at least as many as the pile holds in `view` (exactly as many, for a pile
 * read off cards the view shows nowhere). Null when the event at `at` lands in no pile.
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

/* ------------------------------------------------------------------------------------------- *
 * Sweeps
 * ------------------------------------------------------------------------------------------- */

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
 * The sweep that starts at `at`, if one does: a run of at least two non-combat hits from one source
 * (a Divine Shield that takes one of them, or Armor that takes one whole, counts as hit) or of heals,
 * each on a different target, that reaches at least one unit and every unit on each side it touches.
 * A target hit again starts a new round, and so a new sweep (Blade Storm's repeats).
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
