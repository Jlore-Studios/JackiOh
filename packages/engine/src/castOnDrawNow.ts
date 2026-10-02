// Whether a card is being cast on draw right now (SPEC §2.4, §6.2, R58; Classic+ #26 Tommy Tempo).
//
// "Cast on draw:" names what the card does when its draw casts it, and only then: a Tommy Tempo played
// from a hand is a plain Unit and ends nothing. The draw holds its `drawn` back until that cast has
// resolved (`drawComplete.holdDraw`), under the id the card was drawn as, which a Unit keeps through
// the cast — so while the hold stands, the card is being cast on draw.

import type { CardInstance, GameState } from "./state";

/** R58: whether `card` is being cast by the draw that drew it. */
export function isCastOnDraw(state: GameState, card: Pick<CardInstance, "id">): boolean {
  return state.heldDraws?.includes(card.id) ?? false;
}
