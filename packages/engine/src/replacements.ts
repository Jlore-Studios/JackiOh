// Replacement windows: effects that change an event before it happens (docs/classic-sets.md B5 E5,
// with E8's heal conversion and E9's redirects; SPEC §3.2, §4.2, §4.4, §4.5; R460–R463).
//
// Before patch v0.2.0 the engine had one replacement, My Pawn's attack cancel (R44), and it lives in
// a trap WINDOW: the declaration waits while the traps answer it, and a trap there may ask a question.
// The five v0.2.0 moments are not windows. Each sits at a fixed point inside a rule that cannot stop
// halfway for a prompt:
//
//   lethalHit    §4.4, after the hero's Armor, divisor and caps and before step 5: a hit that would
//                bring its hero to 0 or less, "this hit alone, as R44 judges it" (Classic #52);
//   healed       inside every heal — Lifesteal, "heal up to" and "heal to full" included, set health
//                not (E7) — on the heal's stated amount (R462; Classic+ #22);
//   wouldDie     §4.5 step 1, once the check has collected its units and before any card moves
//                (Classic #14's Radiant face);
//   toGraveyard  every zone move into a graveyard, asked by `zones.moveToZone` once the card has left
//                the zone it was in (Classic #28, #50, #60);
//   targeted     §4.2 step 2 for an attack, between its target and its trap window, and — the play
//                pipeline's half — §10.5 step 1 and every target prompt (Classic #33 Joro).
//
// So a replacement is DECIDED SYNCHRONOUSLY, out of data the card declares (`Script.replacements`):
// the moment, where the card must stand, a pure `when`, and a declarative `instead` the engine
// applies — never an effect list, which could ask. What a card does beyond the replacement itself
// (Final Gambit's "heal 10 and draw 3", Shadowstep's copies) is its `then`: a step of the card's own
// `resume` table, owed on `state.work` as the card's continuation (R113) and resolved after the
// replaced event, as a trap answering that event would be — so it survives a pause and a replay like
// any continuation, and the default work handler (`prompts.ts`) runs it.
//
// R460 orders several replacements of one event: they apply one at a time in R68's order (the active
// player's side first; within a side units by lane, backrow by lane, then hand; the card the event
// is about, when it stands in none of those, after its owner's hand), each card replaces a given
// event at most once, and once one has changed the event every later one re-checks the changed event
// and applies only if it still does — so a second Final Gambit finds no lethal hit and stays set.
//
// A Trap or Field Trap that replaces an event FIRES: `trapFired`, face-up, and a Trap is spent to its
// owner's graveyard as it would be after any firing (§5.1, R33, R61). A Field Spell, a Unit or a card
// in a hand replaces without firing. A face-down card's text is in no one's use but its own firing,
// so a face-down Trap is never a static source, and a replacement into a graveyard, which
// `moveToZone` asks with no event list to fire into, is only ever a static one.

import type { CardType, PlayerId } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { DAMAGE_REDIRECT_CAP, LIBRARY_CAP } from "./config";
import { dealDamage, type DamageSink, type DamageTarget } from "./damage";
import { flickerCard } from "./effects/flicker";
import { cardTypeOf } from "./faces";
import { modifierIsLive } from "./mana";
import { addModifier } from "./modifiers";
import { flagsOf, scriptOf } from "./scripts";
import { findInstance, type CardInstance, type GameState, type Resume } from "./state";
import { pushWork } from "./work";
import {
  activeUnitsOf,
  cardAt,
  firstEntryZone,
  isUnitToken,
  moveToZone,
  placeOnField,
  registerGraveyardRedirect,
  removeFromAnyZone,
  reportGraveyardLanding,
  slotOf,
  slotsOf,
  type GraveyardRedirect,
} from "./zones";

// ---------------------------------------------------------------------------
// What a card declares
// ---------------------------------------------------------------------------

/** The five points an event can be replaced at (see the header). */
export type ReplacementMoment = "lethalHit" | "healed" | "wouldDie" | "toGraveyard" | "targeted";

/** A heal's target, by id, so a replaced event is plain JSON (§10.1). */
export type HealedRef = { kind: "hero"; player: PlayerId } | { kind: "unit"; instanceId: string; controller: PlayerId };

/** The event a replacement is offered, as plain JSON: the follow-up step reads it back (R113). */
export type ReplacedEvent =
  /** A hit of `amount` (after Armor, divisor and caps) would bring `player`'s hero to 0 or less. */
  | { moment: "lethalHit"; player: PlayerId; amount: number; sourceId: string | null }
  /** A heal of `amount` (its stated amount, R462) would land on `target`. */
  | { moment: "healed"; target: HealedRef; amount: number }
  /** §4.5 step 1 collected these units, which would now die. */
  | { moment: "wouldDie"; units: { instanceId: string; controller: PlayerId }[] }
  /** This card would go to its owner's graveyard, from `from`. */
  | { moment: "toGraveyard"; instanceId: string; defId: string; owner: PlayerId; from: string }
  /**
   * `by` chose this unit of `controller`'s — as an attack's target, or as a play's or prompt's pick.
   * `source` is the targeting card's type (a Spell, a Unit, a Trap …); an attack has none (R651).
   */
  | {
    moment: "targeted";
    instanceId: string;
    controller: PlayerId;
    by: PlayerId;
    what: "attack" | "target";
    source?: CardType;
  };

/**
 * Where the card must stand to replace: acting on the field (a unit on top of its pile, a backrow
 * card — a Trap face-down, a Field Trap either way); in its controller's hand (Classic #33 Joro); or
 * wherever it is, for an event about the card itself (Classic #60 Pile On's "if this would go").
 */
export type ReplacementWhere = "field" | "hand" | "self";

/** What a replacement's `when` reads: a PURE READ, like `conditionMet` — no writes, no rng. */
export type ReplacementContext<E extends ReplacedEvent = ReplacedEvent> = {
  state: GameState;
  self: CardInstance;
  /** The card's controller: "your" in its text (§8 Conventions). */
  controller: PlayerId;
  radiant: boolean;
  event: E;
};

type ReplacementOf<M extends ReplacementMoment, Instead> = {
  id: string;
  on: M;
  /** Default "field". */
  where?: ReplacementWhere;
  /** Whether it replaces this event; absent, the moment alone arms it. Declining leaves a Trap set (R99). */
  when?: (ctx: ReplacementContext<Extract<ReplacedEvent, { moment: M }>>) => boolean;
  instead: Instead;
  /** A step of the card's `resume` table resolved after the replaced event, owed on `state.work`. */
  then?: string;
};

/**
 * One replacement a card makes. The moment fixes what `instead` may say:
 *  - `lethalHit`: `redirect: "enemyHero"` — the hit moves to the enemy hero as a new instance from
 *    the same source, through that hero's Armor and caps (E9). Answers only for its own hero.
 *  - `healed`: `damage: "pierce"` — the heal becomes that much Pierce damage from this card (E8);
 *    `lasting: "thisTurn"` also converts every later heal on its enemies this turn (a player
 *    modifier). Answers only a heal on its controller's enemies.
 *  - `wouldDie`: `flicker: "yours"` — its controller's units among the dying flicker instead (E22):
 *    back in their zones at once, reset, summoning sick, no Death, no Reborn. One firing for them all.
 *  - `toGraveyard`: `to` — exile, or the bottom of the owner's library.
 *  - `targeted`: `interpose: true` — summon this from its controller's hand (leftmost open unit
 *    zone; with none it does nothing), no Cry, summoning sick, and the attack or pick moves to it.
 *    Answers only its controller's unit targeted by the other player (`TargetedReplacement`); `by:
 *    "spell"` answers only a targeting whose `source` is a Spell (Classic #33 Joro, R651).
 */
export type ReplacementDef =
  | ReplacementOf<"lethalHit", { redirect: "enemyHero" }>
  | ReplacementOf<"healed", { damage: "pierce"; lasting?: "thisTurn" }>
  | ReplacementOf<"wouldDie", { flicker: "yours" }>
  | ReplacementOf<"toGraveyard", { to: "exile" | "bottomOfLibrary" }>
  | (ReplacementOf<"targeted", { interpose: true }> & {
      /** R651: when present, the replacement answers only a targeting by this kind of card. */
      by?: "spell";
    });

/**
 * B5 E5, E9: THE declaration of "a friendly unit is targeted" (Classic #33 Joro), for every targeting
 * there is — an attack declared at the unit (§4.2 step 2, `combat.declareAttack`), and a play's, a
 * cast's or an activation's declared target or a prompt answer's pick (§10.5 step 1, the play
 * pipeline's half). Both halves read this one entry, `{ on: "targeted", where: "hand", instead: {
 * interpose: true } }`, through `answerTargeting`, so a card declares it once.
 */
export type TargetedReplacement = Extract<ReplacementDef, { on: "targeted" }>;

/** One unit a `wouldDie` replacement flickered, as its follow-up reads it. */
export type FlickeredCard = { instanceId: string; defId: string; radiant: boolean };

/**
 * What a follow-up (`then`) is handed, in its context's data under `REPLACED_KEY`: the event as the
 * replacement met it, where a lethal hit went, and the units a `wouldDie` replacement flickered.
 */
export type ReplacementRecord = {
  event: ReplacedEvent;
  redirectedTo?: PlayerId;
  flickered?: FlickeredCard[];
};

/** Where a follow-up's context data carries its `ReplacementRecord`. */
export const REPLACED_KEY = "replaced";

/** The record a follow-up step was handed (Classic #14's copies read `flickered`), or null. */
export function replacementOf(ctx: { data: Record<string, unknown> }): ReplacementRecord | null {
  const raw = ctx.data[REPLACED_KEY];
  if (raw === null || typeof raw !== "object") return null;
  const record = raw as Partial<ReplacementRecord>;
  return record.event === undefined ? null : (raw as ReplacementRecord);
}

// ---------------------------------------------------------------------------
// The walk (R460, R68)
// ---------------------------------------------------------------------------

type CandidateZone = "field" | "backrow" | "hand" | "elsewhere";

type Candidate = { card: CardInstance; controller: PlayerId; zone: CandidateZone };

/**
 * Every card that could replace, in R68's order: the active player's side first; within a side the
 * units on top of their piles by lane, the backrow by lane, the hand in order — and `about`, the
 * card the event concerns, after its owner's hand when it stands in none of those (a Spell resolving,
 * a card already lifted off the field on its way to a graveyard). No graveyard card replaces.
 */
function candidates(state: GameState, about: CardInstance | null): Candidate[] {
  const sides: PlayerId[] = state.active === "p1" ? ["p1", "p2"] : ["p2", "p1"];
  const out: Candidate[] = [];
  const seen = new Set<string>();
  const add = (card: CardInstance, controller: PlayerId, zone: CandidateZone): void => {
    if (seen.has(card.id)) return;
    seen.add(card.id);
    out.push({ card, controller, zone });
  };
  for (const player of sides) {
    for (const card of activeUnitsOf(state, player)) add(card, player, "field");
    for (const ref of slotsOf(player, "backrow")) {
      const card = cardAt(state, ref);
      if (card !== null) add(card, player, "backrow");
    }
    for (const card of state.players[player].hand) add(card, player, "hand");
    if (about !== null && about.owner === player) add(about, about.controller, "elsewhere");
  }
  return out;
}

function isTrapCard(state: GameState, card: CardInstance): boolean {
  const type = cardTypeOf(state, card);
  return type === "Trap" || type === "Field Trap";
}

/** A backrow Trap whose identity nobody but its controller reads yet (R33). */
function faceDownTrap(state: GameState, cand: Candidate): boolean {
  return cand.zone === "backrow" && isTrapCard(state, cand.card) && cand.card.faceUp !== true;
}

/** Whether the event is about this card, for a "self" replacement. */
function eventNames(event: ReplacedEvent, id: string): boolean {
  switch (event.moment) {
    case "lethalHit":
      return false;
    case "healed":
      return event.target.kind === "unit" && event.target.instanceId === id;
    case "wouldDie":
      return event.units.some((unit) => unit.instanceId === id);
    case "toGraveyard":
    case "targeted":
      return event.instanceId === id;
  }
}

/**
 * Whether the card stands where its replacement says. On the field a Trap answers face-down and fires
 * (§5.1), a Field Trap face-down or up, anything else as it stands; a replacement into a graveyard is
 * static and never fires, so a Trap offers one only once it is face-up.
 */
function standsWhere(state: GameState, cand: Candidate, where: ReplacementWhere, event: ReplacedEvent): boolean {
  if (where === "self") return eventNames(event, cand.card.id);
  if (where === "hand") return cand.zone === "hand";
  if (cand.zone === "field") return true;
  if (cand.zone !== "backrow") return false;
  if (!isTrapCard(state, cand.card)) return true;
  if (event.moment === "toGraveyard") return cand.card.faceUp === true;
  return cardTypeOf(state, cand.card) === "Field Trap" || cand.card.faceUp !== true;
}

/**
 * R651: whether a "targeted" replacement answers a targeting from this source — a `by: "spell"`
 * replacement (Classic #33 Joro) answers only a Spell's targeting, and an attack carries no source.
 */
function targetedSourceMatches(
  def: TargetedReplacement,
  event: Extract<ReplacedEvent, { moment: "targeted" }>,
): boolean {
  if (def.by === undefined) return true;
  return event.source === "Spell";
}

/** The first of the card's replacements for this moment that stands where it must and whose `when` holds. */
function answering<M extends ReplacementMoment>(
  state: GameState,
  cand: Candidate,
  moment: M,
  event: Extract<ReplacedEvent, { moment: M }>,
): Extract<ReplacementDef, { on: M }> | undefined {
  const defs = (scriptOf(cand.card).replacements ?? []).filter(
    (def): def is Extract<ReplacementDef, { on: M }> => def.on === moment,
  );
  return defs.find((def) => {
    if (!standsWhere(state, cand, def.where ?? "field", event)) return false;
    if (
      moment === "targeted" &&
      !targetedSourceMatches(
        def as TargetedReplacement,
        event as Extract<ReplacedEvent, { moment: "targeted" }>,
      )
    )
      return false;
    const when = def.when as ((ctx: ReplacementContext) => boolean) | undefined;
    if (when === undefined) return true;
    return when({ state, self: cand.card, controller: cand.controller, radiant: cand.card.radiant, event }) === true;
  });
}

/**
 * A Trap or Field Trap in the backrow fires as it replaces (§5.1, R33): `trapFired`, and face-up from
 * this moment. Returns whether it fired, so `finish` knows to spend it.
 */
function fire(sink: DamageSink, cand: Candidate): boolean {
  if (cand.zone !== "backrow" || !isTrapCard(sink.state, cand.card)) return false;
  const at = slotOf(sink.state, cand.card);
  sink.events.push({
    type: "trapFired",
    instanceId: cand.card.id,
    defId: cand.card.defId,
    controller: cand.controller,
    row: at?.row ?? "backrow",
    lane: at?.lane ?? 0,
  });
  cand.card.faceUp = true;
  return true;
}

/**
 * After the replacement: a fired Trap is spent to its owner's graveyard (a Field Trap stays face-up,
 * §5.1), and the card's follow-up is owed (R113) with what it needs to know.
 */
function finish(sink: DamageSink, cand: Candidate, def: ReplacementDef, record: ReplacementRecord, fired: boolean): void {
  const card = cand.card;
  if (fired && cardTypeOf(sink.state, card) === "Trap" && card.zone.z === "field") {
    reportGraveyardLanding(sink, card, moveToZone(sink.state, card, "graveyard"));
  }
  if (def.then === undefined) return;
  const resume: Resume = {
    defId: card.defId,
    hook: "resume",
    step: def.then,
    radiant: card.radiant,
    instanceId: card.id,
    data: { [REPLACED_KEY]: JSON.parse(JSON.stringify(record)) as ReplacementRecord },
  };
  // R462: owed as the card's own continuation the moment it fires (R113's cursor), so it resolves once
  // the effect the replaced event happened in has finished or paused, before the resolution loop pops
  // any queued trigger — where a trap answering that event would resolve.
  pushWork(sink, resume, cand.controller);
}

// ---------------------------------------------------------------------------
// §4.4: would take lethal damage (E5, E9)
// ---------------------------------------------------------------------------

/**
 * §4.4 between the hero's caps and step 5: a hit of `amount` would bring `player`'s hero to 0 or
 * less. Returns the hero the hit now goes to — the damage pipeline deals it there as a new instance
 * from the same source, through that hero's Armor, divisor and caps — or null when nothing replaced
 * it. Only the hero's own controller's cards answer: "you would take lethal damage".
 */
export function lethalHitWindow(
  sink: DamageSink,
  hit: { player: PlayerId; amount: number; sourceId: string | null },
): PlayerId | null {
  const event: Extract<ReplacedEvent, { moment: "lethalHit" }> = { moment: "lethalHit", ...hit };
  for (const cand of candidates(sink.state, null)) {
    if (cand.controller !== hit.player) continue;
    const def = answering(sink.state, cand, "lethalHit", event);
    if (def === undefined) continue;
    const to = opponentOf(cand.controller);
    const fired = fire(sink, cand);
    sink.events.push({
      type: "redirected",
      what: "damage",
      fromId: `hero-${hit.player}`,
      toId: `hero-${to}`,
      byInstanceId: cand.card.id,
    });
    finish(sink, cand, def, { event, redirectedTo: to }, fired);
    // R460: the hit is not on this hero any more, so nothing later in the walk finds it lethal here;
    // the new instance meets the other hero's replacements in a walk of its own.
    return to;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Heals: would be healed (E5) and heal into damage (E8)
// ---------------------------------------------------------------------------

/** Heals being converted right now, so two Lifesteal converters cannot convert each other for ever. */
let converting = 0;

function healedRefOf(target: DamageTarget): HealedRef {
  return target.kind === "hero"
    ? { kind: "hero", player: target.player }
    : { kind: "unit", instanceId: target.instance.id, controller: target.instance.controller };
}

/** The unit acting in its zone (§3.2): only a unit on the field is healed. */
function actsOnField(state: GameState, card: CardInstance): boolean {
  const at = slotOf(state, card);
  return at !== null && cardAt(state, at)?.id === card.id;
}

/** E8: whether the card converts heals by its text, as it stands now (face-up, when a Trap). */
function convertsByText(state: GameState, cand: Candidate): boolean {
  if (flagsOf(cand.card).healToDamage !== true) return false;
  if (cand.zone !== "field" && cand.zone !== "backrow") return false;
  return !faceDownTrap(state, cand);
}

/** E8: the heal of `amount` becomes that much Pierce damage from the converting card. */
function convert(sink: DamageSink, converter: CardInstance | null, target: DamageTarget, amount: number): void {
  converting += 1;
  try {
    dealDamage(sink, { source: converter, target, amount, flags: { ignoreArmor: true } });
  } finally {
    converting -= 1;
  }
}

/**
 * E5's "would be healed" and E8's conversion, asked by every heal before it lands (`damage.ts`):
 * true when the heal was replaced — it heals nothing, and its stated amount has been dealt instead.
 * A conversion already in force goes first (the active player's modifiers, then the other's), then
 * the R68 walk over the cards on the healed target's enemies' side: a card converting by its text,
 * or one replacing now — a Trap that fires, and whose `lasting` converts the rest of the turn too.
 */
export function healingReplaced(sink: DamageSink, target: DamageTarget, amount: number): boolean {
  const state = sink.state;
  if (amount <= 0 || converting >= DAMAGE_REDIRECT_CAP) return false;
  if (target.kind === "unit" && !actsOnField(state, target.instance)) return false;
  const healed = target.kind === "hero" ? target.player : target.instance.controller;

  const sides: PlayerId[] = state.active === "p1" ? ["p1", "p2"] : ["p2", "p1"];
  for (const player of sides) {
    if (player === healed) continue;
    const mod = state.players[player].mods.find((each) => each.kind === "healToDamage" && modifierIsLive(state, each));
    if (mod === undefined || mod.kind !== "healToDamage") continue;
    convert(sink, findInstance(state, mod.converterId) ?? null, target, amount);
    return true;
  }

  const event: Extract<ReplacedEvent, { moment: "healed" }> = { moment: "healed", target: healedRefOf(target), amount };
  for (const cand of candidates(state, target.kind === "unit" ? target.instance : null)) {
    if (cand.controller === healed) continue;
    if (convertsByText(state, cand)) {
      convert(sink, cand.card, target, amount);
      return true;
    }
    const def = answering(state, cand, "healed", event);
    if (def === undefined) continue;
    const fired = fire(sink, cand);
    if (def.instead.lasting === "thisTurn") {
      addModifier(sink, cand.controller, {
        kind: "healToDamage",
        converterId: cand.card.id,
        expiry: { until: "thisTurn", turn: state.turn },
      });
    }
    convert(sink, cand.card, target, amount);
    finish(sink, cand, def, { event }, fired);
    return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// §4.5 step 1: would die (E5), and the flicker it sends a unit through (E22)
// ---------------------------------------------------------------------------

function inUnitZone(card: CardInstance): boolean {
  return card.zone.z === "field" && card.zone.row === "units";
}

/**
 * §4.5 step 1, before any card moves: the units the check collected would die, and a `wouldDie`
 * replacement takes its controller's among them — one firing for all of them — and flickers them
 * instead (Classic #14's Radiant face). Returns the cards that still die. R462: the check's own
 * collection only; a Sacrifice is an effect's (§6.3) and is not offered.
 */
export function wouldDieWindow(sink: DamageSink, dying: readonly CardInstance[]): CardInstance[] {
  let left = [...dying];
  if (!left.some(inUnitZone)) return left;
  for (const cand of candidates(sink.state, null)) {
    const theirs = left.filter((card) => inUnitZone(card) && card.controller === cand.controller);
    if (theirs.length === 0) continue;
    const event: Extract<ReplacedEvent, { moment: "wouldDie" }> = {
      moment: "wouldDie",
      units: left.filter(inUnitZone).map((card) => ({ instanceId: card.id, controller: card.controller })),
    };
    const def = answering(sink.state, cand, "wouldDie", event);
    if (def === undefined) continue;
    const fired = fire(sink, cand);
    const flickered: FlickeredCard[] = [];
    for (const unit of theirs) {
      const record: FlickeredCard = { instanceId: unit.id, defId: unit.defId, radiant: unit.radiant };
      // B5 E22, R444: the field workstream's flicker, the same one the `flicker` verb does.
      if (flickerCard(sink, unit)) flickered.push(record);
    }
    // R460: the event has changed — these units no longer die — and the rest of the walk re-checks it.
    left = left.filter((card) => !theirs.includes(card));
    finish(sink, cand, def, { event, flickered }, fired);
  }
  return left;
}

// ---------------------------------------------------------------------------
// Every move into a graveyard (E5)
// ---------------------------------------------------------------------------

/**
 * `zones.moveToZone`'s check, registered below: where a card that would go to its owner's graveyard
 * goes instead, or null. Asked once the card has left the zone it was in, so a Voidwalker dying takes
 * its own aura with it (Classic #50). A unit token never reaches a graveyard (R11), so nothing here
 * changes its death (R462).
 */
function graveyardRedirectFor(state: GameState, card: CardInstance): GraveyardRedirect | null {
  if (isUnitToken(state, card)) return null;
  const event: Extract<ReplacedEvent, { moment: "toGraveyard" }> = {
    moment: "toGraveyard",
    instanceId: card.id,
    defId: card.defId,
    owner: card.owner,
    from: card.zone.z,
  };
  for (const cand of candidates(state, card)) {
    const def = answering(state, cand, "toGraveyard", event);
    if (def === undefined) continue;
    // R80: a full library turns the card away, so that replacement cannot apply and the card goes on
    // toward its graveyard, where a later one may still meet it.
    if (def.instead.to === "bottomOfLibrary" && state.players[card.owner].library.length >= LIBRARY_CAP) continue;
    // R460: the first that applies sends the card elsewhere, and a card no longer on its way to a
    // graveyard is nothing any later one replaces.
    return def.instead.to === "exile" ? { to: "exile" } : { to: "library", position: "bottom" };
  }
  return null;
}

registerGraveyardRedirect(graveyardRedirectFor);

// ---------------------------------------------------------------------------
// "A friendly unit is targeted" (E5, E9)
// ---------------------------------------------------------------------------

/**
 * B5 E5, E9: `by` has chosen `target`, one of the other player's units on the field — as the target
 * of a declared attack (§4.2 step 2, before the trap window; `combat.declareAttack`) or as a play's,
 * a cast's, an activation's or a prompt answer's pick (§10.5 step 1; the play pipeline's half). A
 * card of the targeted unit's controller that replaces it by interposing (Classic #33 Joro, from the
 * hand) is summoned into its controller's leftmost open unit zone — no Cry, summoning sick — and
 * the attack or the pick moves to it: `summoned`, then `redirected`. Returns the new target, or null.
 *
 * One targeting is answered once: the first card that interposes ends it ("one Joro answers one
 * targeting", Classic #33), and a card with no open zone to enter does nothing. A random pick or an "all"
 * effect chooses nobody and never comes here; nor does a forced attack, which the effect declares,
 * not the player (R121).
 */
export function answerTargeting(
  sink: DamageSink,
  args: { target: CardInstance; by: PlayerId; what: "attack" | "target" },
): CardInstance | null {
  const state = sink.state;
  const { target, by, what } = args;
  const defender = target.controller;
  if (by === defender || !actsOnField(state, target)) return null;
  const event: Extract<ReplacedEvent, { moment: "targeted" }> = {
    moment: "targeted",
    instanceId: target.id,
    controller: defender,
    by,
    what,
  };
  for (const cand of candidates(state, target)) {
    if (cand.controller !== defender) continue;
    const def = answering(state, cand, "targeted", event);
    if (def === undefined) continue;
    const card = cand.card;
    // §3.2, R64, R688: the leftmost empty, unreserved unit zone (an unlocked one first), which
    // `placeOnField` accepts: Joro is summoned, not played.
    const ref = firstEntryZone(state, defender, "units");
    if (ref === null) continue;
    removeFromAnyZone(state, card);
    placeOnField(state, card, ref);
    card.summonedTurn = state.turn;
    sink.events.push({ type: "summoned", player: defender, instanceId: card.id, defId: card.defId, row: ref.row, lane: ref.lane });
    sink.events.push({ type: "redirected", what, fromId: target.id, toId: card.id, byInstanceId: card.id });
    finish(sink, cand, def, { event }, false);
    return card;
  }
  return null;
}
