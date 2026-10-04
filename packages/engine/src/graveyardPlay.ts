// Playing cards from the graveyard (docs/classic-sets.md B5 E11, R454): the permissions that allow
// it, the plays `legalActions` offers under them, the refusal §10.5 step 1 gives, and the Plague
// Token payment Classic #74 Corpse Plantation adds.
//
// A permission is a card on the field saying so — Classic #28 Second Wind ("You may play cards from
// your graveyard"; its Radiant face only those whose price, as it would be paid, is (1) or more),
// Classic #74 Corpse Plantation (Units, paid partly or wholly with the Plague Counters on it), and
// Classic #90 In Too Deep's reward L (an aura of the same permission while the card stands). Each is
// `Script.graveyardPlay`, a pure read asked of the cards acting on the player's own side.
//
// Everything else is a play's: the card leaves the graveyard at §10.5 step 4 as a hand card leaves
// the hand (`playSteps.takeFromPlaySource`), its choices are checked and offered as a hand card's
// are (R81, R90), R65's player prices reach it (`mana.playCost`), it counts as played and its Cry
// fires (R1: "played", wherever from). A permission only decides whether the play may be made and how
// it may be paid.

import type { ActionBody, PlayerId } from "@jackioh/shared";
import { MIN_PLAGUE_PAYMENT, PLAGUE_TOKEN_MANA } from "./config";
import { cardTypeOf } from "./faces";
import type { EngineSink } from "./resolve";
import { scriptOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import { cardAt, slotsOf } from "./zones";

/**
 * E11, R454: what one card on the field lets its controller play from their own graveyard.
 *
 * - `units`: Units only (Classic #74); absent, every card type (Classic #28, In Too Deep's reward L).
 * - `minPrice`: the least price a play under it may have, as it would be paid — after every discount
 *   (Classic #28 Radiant: (1), which stops a (0) loop).
 * - `plague`: plays under it pay with the Plague Counters on the granting card (Classic #74): each token
 *   pays PLAGUE_TOKEN_MANA, at least MIN_PLAGUE_PAYMENT of them, at most the tokens there and the
 *   price, and the rest in mana. Such a play must spend tokens; a permission without `plague` is paid
 *   in mana alone.
 */
export type GraveyardPlayPermission = {
  units?: boolean;
  minPrice?: number;
  plague?: boolean;
};

/** A permission and the card on the field granting it. */
export type GraveyardGrant = { source: CardInstance; permission: GraveyardPlayPermission };

/** The Plague Counters a play from the graveyard spends: the play action's `plague`. */
export type PlagueSpend = NonNullable<Extract<ActionBody, { type: "play" }>["plague"]>;

/** How one play pays its price besides mana: the Plague Counters a graveyard play spends (R454). */
export type PlayPayment = { plague?: PlagueSpend };

/**
 * E11: every permission the player has now — from each card acting on their side of the field (the
 * top of a pile, a backrow card) whose running face grants one, in lane order, units first. A Vanilla
 * card grants nothing (`scriptOf`, R115); a card dormant under a Stack pile is not on the field for
 * effects (§3.2, R13).
 */
export function graveyardGrantsOf(state: GameState, player: PlayerId): GraveyardGrant[] {
  const out: GraveyardGrant[] = [];
  for (const row of ["units", "backrow"] as const) {
    for (const ref of slotsOf(player, row)) {
      const source = cardAt(state, ref);
      if (source === null || source.controller !== player) continue;
      const hook = scriptOf(source).graveyardPlay;
      if (hook === undefined) continue;
      for (const permission of hook({ state, self: source, radiant: source.radiant })) {
        out.push({ source, permission });
      }
    }
  }
  return out;
}

/** Whether a card lies in this player's own graveyard, where E11's permissions reach ("your graveyard"). */
export function inOwnGraveyard(state: GameState, player: PlayerId, card: CardInstance): boolean {
  // By id: a price is often read off a copy of the card with a probe's X or embiggen (`playChoices`).
  return card.zone.z === "graveyard" && card.zone.player === player && state.players[player].graveyard.some((held) => held.id === card.id);
}

/** The grants that admit this card at all: the right type (Units only, or every type). */
function admitting(state: GameState, player: PlayerId, card: CardInstance): GraveyardGrant[] {
  const unit = cardTypeOf(state, card) === "Unit";
  return graveyardGrantsOf(state, player).filter((grant) => grant.permission.units !== true || unit);
}

/**
 * R454, R65: whether a play may take this card from its player's graveyard now — the card lies there
 * and a permission admits its type. Such a card is priced as a play wherever it is read (R65: the
 * player's prices reach a card where a play takes it from), whatever price a permission then asks.
 */
export function playableFromGraveyard(state: GameState, card: CardInstance): boolean {
  const player = card.zone.player;
  return inOwnGraveyard(state, player, card) && admitting(state, player, card).length > 0;
}

/** The Plague Counters on a card now (§6.3 Plague Counter). */
function plagueOn(card: CardInstance): number {
  return card.counters.plague ?? 0;
}

/**
 * R454: every way a play of this graveyard card at this price may be paid under the permissions — `{}`
 * in mana alone when a permission without `plague` admits the price and the mana covers it; and, per
 * Plague permission that admits it, each number of tokens from MIN_PLAGUE_PAYMENT up to what the card
 * holds and the price allows, with the rest in mana. None when no permission admits the card, or the
 * card is not in its player's own graveyard. `playChoices.graveyardPlayActionsFor` crosses these with
 * the play's choices.
 */
export function graveyardPaymentsFor(state: GameState, player: PlayerId, card: CardInstance, price: number): PlayPayment[] {
  if (!inOwnGraveyard(state, player, card)) return [];
  const grants = admitting(state, player, card);
  const mana = state.players[player].mana.current;
  const out: PlayPayment[] = [];
  const priced = grants.filter((grant) => price >= (grant.permission.minPrice ?? 0));
  if (priced.some((grant) => grant.permission.plague !== true) && price <= mana) out.push({});
  for (const grant of priced) {
    if (grant.permission.plague !== true) continue;
    const most = Math.min(plagueOn(grant.source), Math.floor(price / PLAGUE_TOKEN_MANA));
    for (let tokens = MIN_PLAGUE_PAYMENT; tokens <= most; tokens += 1) {
      if (price - tokens * PLAGUE_TOKEN_MANA <= mana) out.push({ plague: { from: grant.source.id, tokens } });
    }
  }
  return out;
}

/**
 * E11, R454: why a play of this graveyard card at this price, paid as the action says, is refused —
 * the same permissions and payments `graveyardPlayActionsFor` offers, asked from the other side
 * (§10.2), or null when a permission admits it. Mana is part of the answer, since a Plague payment
 * changes what the mana has to cover.
 */
export function whyGraveyardPlayRefused(
  state: GameState,
  player: PlayerId,
  card: CardInstance,
  price: number,
  plague: PlagueSpend | undefined,
): string | null {
  const grants = admitting(state, player, card);
  if (grants.length === 0) return "you may not play that card from your graveyard";
  const priced = grants.filter((grant) => price >= (grant.permission.minPrice ?? 0));
  if (priced.length === 0) return `a card played from your graveyard must cost (${Math.min(...grants.map((grant) => grant.permission.minPrice ?? 0))}) or more`;
  const mana = state.players[player].mana.current;

  if (plague === undefined) {
    if (!priced.some((grant) => grant.permission.plague !== true)) {
      return "that card may only be played from your graveyard by spending Plague Counters";
    }
    return price > mana ? `that card costs ${price}, more than your mana` : null;
  }

  const grant = priced.find((entry) => entry.permission.plague === true && entry.source.id === plague.from);
  if (grant === undefined) return "those Plague Counters cannot pay for that card";
  if (!Number.isInteger(plague.tokens) || plague.tokens < MIN_PLAGUE_PAYMENT) {
    return `spend at least ${MIN_PLAGUE_PAYMENT} Plague Counter`;
  }
  if (plague.tokens > plagueOn(grant.source)) return "there are not that many Plague Counters there";
  if (plague.tokens * PLAGUE_TOKEN_MANA > price) return "that is more Plague Counters than the price";
  if (price - plague.tokens * PLAGUE_TOKEN_MANA > mana) return "the rest of the price is more than your mana";
  return null;
}

/** R454: the mana a play pays once its Plague Counters have paid their part. */
export function manaDue(price: number, plague: PlagueSpend | undefined): number {
  return Math.max(0, price - (plague?.tokens ?? 0) * PLAGUE_TOKEN_MANA);
}

/**
 * R454, §10.5 step 2: take the Plague Counters a play spends off the card that holds them, reported as
 * any change of the count is (`counterChanged`, no `placed`: a removal, E19).
 */
export function spendPlagueTokens(sink: EngineSink, card: CardInstance, tokens: number): void {
  const next = Math.max(0, plagueOn(card) - Math.max(0, Math.trunc(tokens)));
  if (next === plagueOn(card)) return;
  if (next === 0) delete card.counters.plague;
  else card.counters.plague = next;
  sink.events.push({ type: "counterChanged", instanceId: card.id, counter: "plague", value: next });
}
