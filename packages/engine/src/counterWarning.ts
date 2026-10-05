// R659: the warning a hand card carries when playing it now would only get it countered (Classic #87
// Plague Chalice). `viewFor` sets `counteredOnPlay` from this on the viewer's own hand cards, and the
// client draws it and decides nothing (CLAUDE.md rule 7).
//
// It asks the cards that would answer the play's announce — the holders `triggers.dispatchEvent`
// offers a `cardAnnounced` to, acting on the field with a trigger registered for it — through their
// `wouldCounter` hook, the predicate their own counter trigger asks, so the warning and the counter
// are one rule. Only a counter the viewer may read counts: a face-down card's would say what it is
// (R33, R97). A card is flagged only when every price it could be played at now (each X and embiggen
// choice, R65) would be countered, since a price that escapes is a play worth making; whether the
// mana is there to pay it does not matter, as the glow it rides beside does not wait for mana either.

import type { PlayerId } from "@jackioh/shared";
import { offeredPlayCosts } from "./playChoices";
import { backrowIsPublic } from "./preview";
import type { CardInstance, GameState } from "./state";
import { cardsInTriggerOrder, triggersOnEvent, type TriggerHolder } from "./triggers";

/** The cards on the field whose counter trigger would answer a play's announce, as `player` may read them. */
function readableCounters(state: GameState, player: PlayerId): TriggerHolder[] {
  return cardsInTriggerOrder(state).filter(
    (holder) =>
      holder.script.wouldCounter !== undefined &&
      !holder.isTrap &&
      (holder.zone === "field" || (holder.zone === "backrow" && backrowIsPublic(state, holder.card, player))) &&
      triggersOnEvent(holder, "cardAnnounced").length > 0,
  );
}

/**
 * R659: the ids of `player`'s hand cards that every price they could be played at now would see
 * countered, by a card on the field `player` may read. Empty when no such card is on the field.
 */
export function counteredHandCards(state: GameState, player: PlayerId): ReadonlySet<string> {
  const counters = readableCounters(state, player);
  if (counters.length === 0) return new Set();
  const countered = (card: CardInstance): boolean => {
    const costs = offeredPlayCosts(state, player, card);
    return (
      costs.length > 0 &&
      costs.every((costPaid) =>
        counters.some(
          (holder) =>
            holder.script.wouldCounter?.({ state, self: holder.card, controller: holder.controller, player, costPaid }) === true,
        ),
      )
    );
  };
  return new Set(state.players[player].hand.filter(countered).map((card) => card.id));
}
