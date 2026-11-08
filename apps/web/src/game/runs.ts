// CLAUDE.md rule 7: the runner derives casts (§6.3, R70), pile impacts, sweeps, wipes, the Starship's fuse runs and
// launches (R2801000) and an Anomaly's rift (R3337003) from redacted events and PlayerView.
// R97, R202, R242, and R440 forbid hidden-card inference; `HIDDEN_PILE_OF` uses only public card text.

import type { GameEvent, PlayerId, PlayerView, Row } from "@jackioh/shared";

import { isAnomalyCode, type AnomalyCode } from "../cards/anomalies.ts";
import { flavorSourceOf } from "../fx/flavorSource.ts";
import { sideOf, testid, type Side } from "./contract";

/** R97's sentinel (the engine's `HIDDEN_ID`). */
const HIDDEN = "hidden";

const SIDES: readonly Side[] = ["you", "opponent"];

/** The most plays the tracker keeps open: deeper nesting than this is a stream it has lost track of. */
export const PLAYS_OPEN_MAX = 16;

// Casts

/** A play the stream has opened and not yet resolved. */
export type OpenPlay = { player: PlayerId; instanceId: string; defId: string; casts: number };

/** A cast: the play that made it (as it stood after this cast) and which of its casts this is (1 on). */
export type Cast = { by: OpenPlay; ordinal: number };

/**
 * R2800000: the hook resolving now, as the seat's own stream marks it: a `hookResolving` (R2783000) or a
 * Trap's `trapFired`, whose `trap` is the zone it fired from (R154). `defId` is R97's sentinel when the
 * seat cannot read the card.
 */
export type OpenHook = { defId: string; sourceId: string; controller: PlayerId; trap: { row: Row; lane: number } | null };

export type PlayTracker = {
  current(): OpenPlay | undefined;
  /** R2800000: the hook the stream marked last, until its play resolves, the turn turns or `closeHook`. */
  hook(): OpenHook | undefined;
  /** R2800000: forgets the open hook, so it makes one wipe. */
  closeHook(): void;
  /** Reads one event, in stream order; a `cardPlayed` that is a cast returns who cast it. */
  see(event: GameEvent): Cast | null;
  clear(): void;
};

/** R97 may close the innermost play; B5 E1 counters before §10.5's `cardPlayed`, so it closes only an exact match. */
export function createPlayTracker(): PlayTracker {
  let open: OpenPlay[] = [];
  let hook: OpenHook | undefined;

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
    hook() {
      return hook === undefined ? undefined : { ...hook, trap: hook.trap === null ? null : { ...hook.trap } };
    },
    closeHook() {
      hook = undefined;
    },
    see(event) {
      switch (event.type) {
        case "hookResolving":
          hook = { defId: event.defId, sourceId: event.sourceId, controller: event.controller, trap: null };
          return null;
        case "trapFired":
          hook = { defId: event.defId, sourceId: event.instanceId, controller: event.controller, trap: { row: event.row, lane: event.lane } };
          return null;
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
          hook = undefined;
          return null;
        case "countered":
          close(event.player, event.instanceId, true);
          hook = undefined;
          return null;
        case "turnStarted":
        case "gameOver":
          open = [];
          hook = undefined;
          return null;
        case "attackDeclared":
          // R2800000: an attack is an action of its own, never a hook's: a hook left open (a Death's) ends here.
          hook = undefined;
          return null;
        default:
          return null;
      }
    },
    clear() {
      open = [];
      hook = undefined;
    },
  };
}

// Whole-pile impacts

export type PileKind = "library" | "graveyard" | "exile";

export type Pile = { side: Side; pile: PileKind };

/**
 * A whole-pile impact: how many cards the pile held as the run began, and how many events the run
 * made (a pile changed twice over makes twice its count).
 */
export type ZoneImpact = Pile & { count: number; events: number };

/** R440 hidden changes map only where public text names one library; multi-pile cards (C+ #73) stay unmapped. */
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

export function pileCount(view: PlayerView, pile: Pile): number {
  const sv = seat(view, pile.side);
  if (pile.pile === "library") return sv.libraryCount;
  return pile.pile === "graveyard" ? sv.graveyard.length : sv.exile.length;
}

function publicPileOf(view: PlayerView, instanceId: string): Pile | null {
  for (const side of SIDES) {
    const sv = seat(view, side);
    if (sv.graveyard.some((card) => card.instanceId === instanceId)) return { side, pile: "graveyard" };
    if (sv.exile.some((card) => card.instanceId === instanceId)) return { side, pile: "exile" };
  }
  return null;
}

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

function otherOf(player: PlayerId): PlayerId {
  return player === "p1" ? "p2" : "p1";
}

/** `inferred` piles count whole only on an exact run, because the view did not show the card's origin. */
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
  if (id === null) return null;
  if (id !== HIDDEN) return publicPileOf(view, id);
  if (play === undefined || play.defId === HIDDEN) return null;
  if (!Object.prototype.hasOwnProperty.call(HIDDEN_PILE_OF, play.defId)) return null;
  const owner = HIDDEN_PILE_OF[play.defId] === "enemy" ? otherOf(play.player) : play.player;
  return { side: sideOf(view, owner), pile: "library" };
}

function samePile(a: Pile, b: Pile): boolean {
  return a.side === b.side && a.pile === b.pile;
}

/** A same-type, same-pile run is whole at two or more events and the pile count (exactly for inferred piles). */
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

/** A sweep as the runner plays it: `sides` are the sides every unit of which it reached. */
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

function sweepStep(event: GameEvent): { target: string; tone: Sweep["tone"]; source?: string | null } | null {
  if (event.type === "damage") return event.combat ? null : { target: event.targetId, tone: "damage", source: event.sourceId };
  if (event.type === "divineShieldLost") return { target: event.instanceId, tone: "damage" };
  // R1361: a hit the Armor took whole still reached its target, as a Divine Shield's does.
  if (event.type === "damageAbsorbed") return event.combat ? null : { target: event.targetId, tone: "damage", source: event.sourceId };
  if (event.type === "healed") return { target: event.targetId, tone: "heal" };
  return null;
}

/** A sweep reaches every unit on each touched side with distinct non-combat hits or heals. */
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

/**
 * #1443: one volley of non-combat damage from one source, planned as one entry so its shots start
 * together. Only when the view shows the source face-up (`flavorSourceOf` non-null):
 * hidden/unknown/hero-power sources keep today's entries.
 */
export function volleyAt(events: readonly GameEvent[], at: number, view: PlayerView): { length: number } | null {
  const first = events[at];
  if (first === undefined || first.type !== "damage" || first.combat) return null;
  const source = first.sourceId;
  if (source === null || source === HIDDEN) return null;
  let length = 0;
  for (let i = at; i < events.length; i += 1) {
    const event = events[i];
    if (event === undefined || event.type !== "damage" || event.combat || event.sourceId !== source) break;
    length += 1;
  }
  if (length < 2) return null;
  if (flavorSourceOf(source, view) === null) return null;
  return { length };
}

// Wipes

/** R2800000: the removals a wipe is made of. */
const WIPE_STEPS = new Set<GameEvent["type"]>(["destroyed", "exiled", "bounced"]);

/**
 * R2800000: a run of removals a set-piece card's hook made, as the runner plays it. `by` is the hook's
 * readable definition, `sides` the sides the run took cards from, and `anchor` the zone it centres on:
 * the zone a Trap fired from (R154), or the Unit's own; null for a Spell's.
 */
export type Wipe = { by: string; sourceId: string; sides: readonly Side[]; anchor: string | null };

type WipeStep = Extract<GameEvent, { type: "destroyed" | "exiled" | "bounced" }>;

function isWipeStep(event: GameEvent | undefined): event is WipeStep {
  return event !== undefined && WIPE_STEPS.has(event.type);
}

/** A bounced card's arrival in its hand (or its burning there, R317), which the engine emits right after the bounce. */
function landsFrom(event: GameEvent | undefined, step: WipeStep): boolean {
  if (event === undefined || step.type !== "bounced") return false;
  return (event.type === "addedToHand" || event.type === "burned") && event.instanceId === step.instanceId;
}

/** The side of the field that shows `instanceId` in `view`: a unit, or a face-up backrow card. */
function fieldSide(view: PlayerView, instanceId: string): Side | null {
  if (instanceId === HIDDEN) return null;
  for (const side of SIDES) {
    const sv = seat(view, side);
    if (sv.units.some((unit) => unit !== null && unit.instanceId === instanceId)) return side;
    if (sv.backrow.some((slot) => slot !== null && !slot.faceDown && slot.instanceId === instanceId)) return side;
  }
  return null;
}

/** The units zone the board shows `instanceId` in, or null. */
function unitZone(view: PlayerView, instanceId: string): string | null {
  for (const side of SIDES) {
    const lane = seat(view, side).units.findIndex((unit) => unit !== null && unit.instanceId === instanceId);
    if (lane >= 0) return testid.zone(side, "units", lane + 1);
  }
  return null;
}

/**
 * R2800000: the wipe at `at`, or null. Only a hook the seat reads (R202) whose card `wipeOf` names, and
 * only its run of consecutive removals from `at`, each bounce with the hand arrival that follows it: the
 * caller closes the hook after it, so a hook makes one wipe. A removal the board does not show (R97's
 * sentinel, a face-down card) counts on its owner's side (for a death, its controller's).
 */
export function wipeAt(
  events: readonly GameEvent[],
  at: number,
  view: PlayerView,
  hook: OpenHook | undefined,
  wipeOf: (defId: string) => boolean,
): { wipe: Wipe; length: number } | null {
  if (hook === undefined || hook.defId === HIDDEN || !wipeOf(hook.defId) || !isWipeStep(events[at])) return null;
  const taken = new Set<Side>();
  let i = at;
  for (let event = events[i]; isWipeStep(event); event = events[i]) {
    taken.add(fieldSide(view, event.instanceId) ?? sideOf(view, event.type === "destroyed" ? event.controller : event.owner));
    i += landsFrom(events[i + 1], event) ? 2 : 1;
  }
  const length = i - at;
  let anchor: string | null = null;
  if (hook.trap !== null) {
    anchor = testid.zone(sideOf(view, hook.controller), hook.trap.row, hook.trap.lane);
  } else {
    for (let k = at - 1; k >= 0 && anchor === null; k -= 1) {
      const event = events[k];
      if (event?.type === "summoned" && event.instanceId === hook.sourceId) anchor = testid.zone(sideOf(view, event.player), event.row, event.lane);
    }
    anchor ??= unitZone(view, hook.sourceId);
  }
  const sides = SIDES.filter((side) => taken.has(side));
  return { wipe: { by: hook.defId, sourceId: hook.sourceId, sides, anchor }, length };
}

// Signatures

/**
 * R3338000: a Terminus card's signature moment as the runner plays it, one entry its set piece claims:
 * the readable card it is, its instance and its controller.
 */
export type Signature = { by: string; sourceId: string; player: PlayerId };

/**
 * R3338000: the signature at `at`, or null: the run of consecutive events from `at` whose types `runOf`
 * names for the hook resolving (a Trap's own `trapFired` at `at` is its own), when the seat reads that
 * hook's card (R202). A run opens only on the first type `runOf` names, so another hook of the same card
 * (Dice of Doom's end-of-turn return, its `addedToHand` or `burned`, opening on no `rolled`) plays its
 * own rows. The caller closes the hook after it, so a hook makes one signature.
 */
export function signatureRunAt(
  events: readonly GameEvent[],
  at: number,
  hook: OpenHook | undefined,
  runOf: (defId: string) => readonly GameEvent["type"][] | "cast" | null,
): { signature: Signature; length: number } | null {
  const first = events[at];
  if (first === undefined) return null;
  const owner: OpenHook | undefined =
    first.type === "trapFired"
      ? { defId: first.defId, sourceId: first.instanceId, controller: first.controller, trap: { row: first.row, lane: first.lane } }
      : hook;
  if (owner === undefined || owner.defId === HIDDEN) return null;
  const types = runOf(owner.defId);
  if (types === null || types === "cast" || types[0] !== first.type) return null;
  let i = at;
  for (let event = events[i]; event !== undefined && types.includes(event.type); event = events[i]) {
    // A Trap's own run starts at its firing, and another firing starts another run.
    if (i > at && event.type === "trapFired") break;
    i += 1;
  }
  return { signature: { by: owner.defId, sourceId: owner.sourceId, player: owner.controller }, length: i - at };
}

// The Starship

/**
 * R2801000: Starship Pieces fused one after another into one Dormant Starship, as the runner plays them.
 * `anchor` is the zone the Starship stands in (null when the seat cannot place it, R202), each piece's
 * `from` the card the board shows it as (null for one it does not, a piece already in its graveyard), in
 * the order they fused, and `count` how many pieces the Starship holds after the run.
 */
export type StarshipFuse = {
  shipId: string;
  player: PlayerId;
  anchor: string | null;
  pieces: readonly { pieceId: string; from: string | null }[];
  count: number;
};

/** R2801000: a Starship's launch, as the runner plays it: where it stands, how many pieces it holds and its name (R1852003). */
export type StarshipLaunch = { shipId: string; player: PlayerId; anchor: string | null; pieces: number; name: string };

/**
 * R2801000: the zone a Starship stands in: the one its latest `summoned` before `at` names (a Starship
 * the pass summoned in the same batch is not on the planning view yet), else the card the view shows;
 * null for R97's sentinel or a Starship the board does not show.
 */
function shipAnchor(events: readonly GameEvent[], at: number, view: PlayerView, shipId: string): string | null {
  if (shipId === HIDDEN) return null;
  for (let k = at - 1; k >= 0; k -= 1) {
    const event = events[k];
    if (event?.type === "summoned" && event.instanceId === shipId) return testid.zone(sideOf(view, event.player), event.row, event.lane);
  }
  return fieldSide(view, shipId) === null ? null : testid.card(shipId);
}

type StarshipFusedEvent = Extract<GameEvent, { type: "starshipFused" }>;

/** One fusion at `at`: a `fused` into the Starship the next event names, with that `starshipFused`, or a lone `starshipFused`. */
function fuseStep(events: readonly GameEvent[], at: number): { fused: StarshipFusedEvent; length: number } | null {
  const first = events[at];
  if (first?.type === "starshipFused") return { fused: first, length: 1 };
  const next = events[at + 1];
  if (first?.type === "fused" && next?.type === "starshipFused" && first.resultInstanceId === next.instanceId) return { fused: next, length: 2 };
  return null;
}

/**
 * R2801000: the fuse run at `at`, or null: every fusion from `at` on into the same Starship of the same
 * player, one after another, each with the `fused` the engine reports right before its `starshipFused`
 * (R1852000). Any other `fused` (an Equip's) is not a Starship's and plays as its own entry.
 */
export function fuseRunAt(events: readonly GameEvent[], at: number, view: PlayerView): { fuse: StarshipFuse; length: number } | null {
  const head = fuseStep(events, at);
  if (head === null) return null;
  const ship = head.fused;
  const pieces: { pieceId: string; from: string | null }[] = [];
  let count = ship.pieces;
  let i = at;
  for (let step: ReturnType<typeof fuseStep> = head; step !== null; step = fuseStep(events, i)) {
    const fused = step.fused;
    if (fused.instanceId !== ship.instanceId || fused.player !== ship.player) break;
    pieces.push({ pieceId: fused.pieceId, from: fieldSide(view, fused.pieceId) === null ? null : testid.card(fused.pieceId) });
    count = fused.pieces;
    i += step.length;
  }
  const anchor = shipAnchor(events, at, view, ship.instanceId);
  return { fuse: { shipId: ship.instanceId, player: ship.player, anchor, pieces, count }, length: i - at };
}

/** R2801000: the launch at `at`, or null. */
export function launchAt(events: readonly GameEvent[], at: number, view: PlayerView): StarshipLaunch | null {
  const event = events[at];
  if (event?.type !== "starshipLaunched") return null;
  return {
    shipId: event.instanceId,
    player: event.player,
    anchor: shipAnchor(events, at, view, event.instanceId),
    pieces: event.pieces.length,
    name: event.name,
  };
}

// An Anomaly

/**
 * R3337003: an Anomaly laid on `player`'s side, as the runner plays it: its code (R3337000), the side it
 * lands on, each card the board's `summoned` lays, in order, as the zone it lands in and its row, and
 * `sourceId`, the play open as it lands (Anomaly Found!'s own, or #151's), or null for none.
 */
export type AnomalyRift = {
  player: PlayerId;
  code: AnomalyCode;
  side: Side;
  landings: readonly { zone: string; row: Row }[];
  sourceId: string | null;
};

/** R3337003: what ends a rift: the next hook (a laid Cry firing, R1855001), a question, a Trap, the play's end, another replacement, the game's or the turn's end. */
const RIFT_ENDS = new Set<GameEvent["type"]>(["hookResolving", "promptOpened", "trapFired", "cardResolved", "boardReplaced", "gameOver", "turnEnded"]);

/**
 * R3337003: the rift at `at`, or null: a `boardReplaced` naming an Anomaly (R3337000), with every event
 * after it up to the first of `RIFT_ENDS`: the old side's `exiled` and the board's `summoned` among
 * them. A replacement naming none plays as its own row. The code and the zones are public (R202): the
 * laid board is, and a face-down card's landing names only its zone.
 */
export function riftAt(events: readonly GameEvent[], at: number, view: PlayerView, play?: OpenPlay): { rift: AnomalyRift; length: number } | null {
  const head = events[at];
  if (head?.type !== "boardReplaced" || !isAnomalyCode(head.anomaly)) return null;
  const side = sideOf(view, head.player);
  const landings: { zone: string; row: Row }[] = [];
  let i = at + 1;
  for (let event = events[i]; event !== undefined && !RIFT_ENDS.has(event.type); event = events[i]) {
    if (event.type === "summoned" && event.player === head.player) landings.push({ zone: testid.zone(side, event.row, event.lane), row: event.row });
    i += 1;
  }
  return { rift: { player: head.player, code: head.anomaly, side, landings, sourceId: play?.instanceId ?? null }, length: i - at };
}
