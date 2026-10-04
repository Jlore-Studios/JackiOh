// Who may be targeted, and what targeting costs (docs/classic-sets.md B5 E5, E35; R450): the pure
// reads the declarations (`playChoices.legalSelectionsFor`), the target prompts
// (`targetingPoint.ts`) and the attack's interception (§4.2 step 2) all ask, so a picker's greyed-out
// option and the reducer's refusal are one rule (§10.2).
//
// "Targeting" is choosing (R450): a play's, a cast's or an activation's declared target of kind
// `target`, and a `target` prompt's answer. Random picks and "all" effects target nothing, and neither
// does a Tribute, a hand pick or a zone pick. What it reaches is a card acting on the field — the
// text that makes a card harder to target (Classic #89) or answers its targeting (Classic #33, from a
// hand) is read from there.

import type { PlayerId, Selection } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { fusedIdParts } from "./catalog";
import { spellCannotReach } from "./restrictions";
import type { Script } from "./script";
import { scriptOf, scriptsFor } from "./scripts";
import { findInstance, type CardInstance, type GameState } from "./state";
import { firstFreeZone, isBuried } from "./zones";

/** A card acting on the field: a unit on top of its pile, or a backrow card (§3.2, R13). */
function actsOnField(state: GameState, card: CardInstance): boolean {
  return card.zone.z === "field" && !isBuried(state, card);
}

/** One face's own targeting cost, read with the card as "this" (a pure read, §10.9). */
function faceCost(script: Script, state: GameState, self: CardInstance): number {
  const hook = script.targetingDiscards;
  if (hook === undefined) return 0;
  const count = hook({ state, self, radiant: self.radiant });
  return typeof count === "number" && Number.isFinite(count) ? Math.max(0, Math.trunc(count)) : 0;
}

/** R102: a fused card carries every ingredient's text, so the stricter of their costs holds. */
function fusedCost(state: GameState, self: CardInstance, defId: string): number {
  const parts = fusedIdParts(defId);
  if (parts === null) {
    const entry = scriptsFor(defId);
    return faceCost(self.radiant ? entry.radiant : entry.base, state, self);
  }
  return Math.max(0, ...parts.map((part) => fusedCost(state, self, part)));
}

/**
 * B5 E5, Classic #89: how many cards a player must also discard to target this card now — its
 * `targetingDiscards`, while it acts on the field. A Vanilla card has no text (§6.3, R115).
 */
export function targetingDiscardsOf(state: GameState, card: CardInstance): number {
  if (card.vanilla || !actsOnField(state, card)) return 0;
  if (fusedIdParts(card.defId) !== null) return fusedCost(state, card, card.defId);
  return faceCost(scriptOf(card), state, card);
}

/**
 * R450: whether `chooser` could pay to target `card`: at least its cost in OTHER cards in their hand —
 * `leaving` is the card a play is taking out of that hand, which cannot pay for its own target. It
 * binds both players, the card's own controller included.
 */
export function canPayToTarget(state: GameState, chooser: PlayerId, card: CardInstance, leaving?: string): boolean {
  const cost = targetingDiscardsOf(state, card);
  if (cost === 0) return true;
  const others = state.players[chooser].hand.filter((held) => held.id !== leaving).length;
  return others >= cost;
}

/**
 * R450, E35: whether `source`'s targeting may pick `candidate` at all — a Spell never picks a card
 * Immune to Spells, and nobody picks a card whose targeting cost they cannot pay.
 */
export function mayTarget(
  state: GameState,
  chooser: PlayerId,
  source: CardInstance | null,
  candidate: CardInstance,
  leaving?: string,
): boolean {
  if (spellCannotReach(state, source, candidate)) return false;
  return canPayToTarget(state, chooser, candidate, leaving);
}

/**
 * Classic #33 Joro, R450: whether a card answers the targeting of a friendly unit from its owner's
 * hand — its script declares the replacement `{ on: "targeted", where: "hand" }` (`Script.replacements`),
 * the one declaration the attack half (§4.2 step 2) reads too. A Vanilla card has no text (`scriptOf`).
 */
export function interposesFromHand(card: CardInstance): boolean {
  return (scriptOf(card).replacements ?? []).some((entry) => entry.on === "targeted" && entry.where === "hand");
}

/**
 * Classic #33 Joro, R450: the card that answers `chooser` targeting `targeted` — the first card in
 * the targeted unit's controller's hand that `interposesFromHand`, when the targeted card is a unit
 * of the chooser's opponent acting on the field and that player has an open unit zone to summon it
 * into (with none, nothing happens). Null when nothing answers.
 */
export function interceptorFor(state: GameState, chooser: PlayerId, targeted: CardInstance): CardInstance | null {
  if (targeted.zone.z !== "field" || targeted.zone.row !== "units" || isBuried(state, targeted)) return null;
  const defender = targeted.controller;
  if (defender !== opponentOf(chooser)) return null;
  if (firstFreeZone(state, defender, "units") === null) return null;
  return state.players[defender].hand.find(interposesFromHand) ?? null;
}

/**
 * R450: how many cards `picks` cost their chooser to target — each targeting pick naming a card with a
 * targeting cost adds that card's count (a card named twice is targeted twice).
 */
export function targetingDiscardsFor(
  state: GameState,
  picks: readonly Selection[],
  targeting?: (index: number) => boolean,
): number {
  let total = 0;
  picks.forEach((pick, index) => {
    if (targeting !== undefined && !targeting(index)) return;
    if (pick.pick !== "instance") return;
    const card = findInstance(state, pick.instanceId);
    if (card !== undefined) total += targetingDiscardsOf(state, card);
  });
  return total;
}

/**
 * R450, R641: whether the player can pay a targeting cost of `required` discards — that many cards
 * held outside `keep` (the card a play is taking out of that hand, §10.5 step 1, and any hand card
 * the same play picks), or null when they can. The discards themselves are random at pay time
 * (R641: a discard is its player's choice only when the card says "of your choice"); nothing lists
 * or chooses them, so there are no paying sets to enumerate.
 */
export function whyTargetingDiscardsUnpayable(
  state: GameState,
  player: PlayerId,
  required: number,
  keep: readonly string[] = [],
): string | null {
  if (required <= 0) return null;
  const held = state.players[player].hand.filter((card) => !keep.includes(card.id)).length;
  return held >= required
    ? null
    : `targeting that costs ${required} discard${required === 1 ? "" : "s"}, and you hold ${held} card${held === 1 ? "" : "s"}`;
}
