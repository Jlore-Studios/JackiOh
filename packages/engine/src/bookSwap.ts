// Classic #55 Book of Wildfire's swap (SPEC §8.6 row 55, R671): "Becomes a different Book at the end
// of your turn." One hand trigger, printed by Wildfire's own script and granted to every Book it
// becomes by the `swapsBook` enchantment it leaves on them (B5 E39), so the swap goes on turn after
// turn while the card stays in its owner's hand. The registry reads the grant (`triggers.holderOf`):
// a hand card that carries the enchantment answers this trigger as if its text printed it.

import type { GameEvent } from "@jackioh/shared";
import { bookSwapSourceOf, swapBook } from "./effects/transform";
import type { EffectContext, TriggerDef } from "./script";
import type { CardInstance } from "./state";

export const BOOK_SWAP_TRIGGER_ID = "book-swap";

/**
 * At the end of its controller's turn (that turn's `turnEnded`, which hand triggers answer once the
 * end-of-turn triggers have run), the card in hand becomes a different Book. A swapped Book names the
 * card that started the swap from its enchantment; Wildfire itself is that card.
 */
export const BOOK_SWAP_TRIGGER: TriggerDef = {
  id: BOOK_SWAP_TRIGGER_ID,
  on: ["turnEnded"],
  run: (ctx: EffectContext & { event: GameEvent }) => {
    const self = ctx.self;
    if (self === null || self.zone.z !== "hand") return [];
    if (ctx.event.type !== "turnEnded" || ctx.event.player !== ctx.controller) return [];
    return [swapBook({ instanceId: self.id, from: bookSwapSourceOf(self) ?? self.defId })];
  },
};

/** R671: the hand triggers a card's enchantments grant it — the swap, once, unless its text prints it. */
export function grantedHandTriggers(card: CardInstance, printed: readonly TriggerDef[]): readonly TriggerDef[] {
  if (bookSwapSourceOf(card) === null) return printed;
  if (printed.some((def) => def.id === BOOK_SWAP_TRIGGER_ID)) return printed;
  return [...printed, BOOK_SWAP_TRIGGER];
}
