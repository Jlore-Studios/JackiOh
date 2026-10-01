// Exiling at random out of a hand (§6.3 Exile, R60): Classic #15 Nose Hunter's Radiant face, "Exile …
// and a random card from their hand". A card-specific verb of the Classic #1–#45 workstream, kept in
// its own file beside `library.ts`'s two library exiles, which name a library card positionally for
// the same reason this names a hand card by lot: nobody chose it and nothing on the board points at it.
//
// The hand is hidden from the other seat until a card leaves it; exile is public (§3.2), so the
// `exiled` event names the card once it is there, as every other exile does.

import type { Effect, EffectContext } from "../script";
import type { CardInstance } from "../state";
import { moveToZone } from "../zones";
import { playerOf, type PlayerSpec } from "./targets";

/**
 * One card to the exile pile, exactly as `library.ts` and `move.ts` exile one: the game exile counter
 * counts only a card that gets there (R55); a unit-token card ceases to exist instead (R11); the
 * `exiled` event reports the card leaving either way.
 */
function exileCard(ctx: EffectContext, card: CardInstance): void {
  const moved = moveToZone(ctx.state, card, "exile");
  if (moved === "moved") ctx.state.counters.exiled += 1;
  ctx.events.push({
    type: "exiled",
    instanceId: card.id,
    defId: card.defId,
    owner: card.owner,
  });
}

/**
 * §6.3 Exile, `count` random cards out of a player's hand (default one, the opponent's when
 * `player` is "enemy"). The picks are distinct (R60: a random pick of N picks N different cards, or
 * all of them if fewer exist), drawn from the match rng as `exileRandomFromLibrary` draws them, so
 * they depend on nothing but (seed, cursor). An empty hand fizzles: nothing moves and the rng is not
 * drawn from.
 */
export function exileRandomFromHand(args: { count?: number; player?: PlayerSpec } = {}): Effect {
  return {
    kind: "exileRandomFromHand",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      const hand = ctx.state.players[player].hand;
      const count = Math.max(0, Math.trunc(args.count ?? 1));
      if (count === 0 || hand.length === 0) return;

      for (const card of ctx.rng.shuffle(hand).slice(0, count)) exileCard(ctx, card);
    },
  };
}
