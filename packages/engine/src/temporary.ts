// Temporary (R637): a card that is discarded from its owner's hand at the end of their turn. The
// keyword is a card keyword and not temporary mana (§2.3, `mana.ts`), which is a different thing that
// only shares the word, so nothing here or in the card data is called "temporary" without "card".
//
// A card is Temporary when its running face prints it or an effect granted it (`cardKeywords`, the
// set a card in a hand is made of), so a Vanilla card has lost the printed one and keeps a given one,
// as §10.4 keeps every granted keyword. Cleanup (`turn.ts`) discards them after every end-of-turn
// step, so an end-of-turn effect, a trap and a delayed effect may still play or use them first; the
// discard is a real one (§6.3), so "whenever you discard" sees each card, and `endOfTurnCleanupSettle`
// answers its events like cleanup's others.

import type { PlayerId } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { discardFromHand } from "./effects/move";
import { cardKeywords } from "./layers";
import type { EngineSink } from "./resolve";
import type { CardInstance, GameState } from "./state";

/** R637: whether this card is discarded from a hand at the end of its owner's turn. */
export function isTemporaryCard(state: GameState, card: CardInstance): boolean {
  return hasKeyword(cardKeywords(state, card), "Temporary");
}

/**
 * R637: every Temporary card in `player`'s hand goes to their graveyard, in hand order. Deterministic
 * and drawing nothing from the rng, like a whole-hand discard (`move.discardHand`): no choice arises.
 */
export function discardTemporaryCards(sink: EngineSink, player: PlayerId): void {
  // A snapshot: a discard splices the hand.
  for (const card of [...sink.state.players[player].hand]) {
    if (isTemporaryCard(sink.state, card)) discardFromHand(sink, card);
  }
}
