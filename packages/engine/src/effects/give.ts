// Cards between the players' piles (docs/classic-sets.md B5 E16, with E2's hand and deck half):
// cards handed from one hand to the other (Classic #9 Income Tax), a card taken out of the other
// player's deck (Classic+ #12.3 Fluffy Grip) and a draw from the other player's deck (Classic #58
// Common Resources). Every one of them is `ownership.ts`'s change of owner: the card becomes the
// taker's for good, its later piles are the taker's, and the taker's hand cap burns what does not fit
// into the taker's graveyard (§2.4). Swapping whole decks is R73's library swap (`swap.ts`), not this.

import type { PlayerId } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { drawFromLibraryOf, takeIntoHand, type LibraryEnd } from "../ownership";
import type { Effect, EffectContext } from "../script";
import type { CardInstance } from "../state";
import { type LibraryFilter, matchesLibraryFilter } from "./choose";
import { playerOf, type PlayerSpec } from "./targets";

/**
 * What a taken card is given as it lands in the taker's hand: a cost (`costOverride`, Classic+ #12.3's
 * "it costs (0)"), a discount (`costMod`, Classic #9 Radiant's "cost (1) less") and the Radiant face
 * (#12.3 Radiant). As `addToHand`'s riders are: the face goes on first, since the card is Radiant
 * wherever it ends up (R74), and a price goes on only once the card is in the hand — one the hand cap
 * burned is an ordinary graveyard card (R4).
 */
export type TakenRiders = { costOverride?: number; costMod?: number; radiant?: boolean };

function takeWith(ctx: EffectContext, card: CardInstance, taker: PlayerId, riders: TakenRiders): void {
  if (riders.radiant === true) card.radiant = true;
  if (takeIntoHand(ctx, card, taker) !== "hand") return;
  if (riders.costOverride !== undefined) card.costOverride = Math.max(0, Math.trunc(riders.costOverride));
  if (riders.costMod !== undefined) card.costMod += Math.trunc(riders.costMod);
}

/** The instance ids a step's answered prompt picked (`ctx.targets`), in offered order. */
function chosenIds(ctx: EffectContext): string[] {
  return ctx.targets.flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []));
}

/**
 * B5 E16: cards out of `from`'s hand into the other player's hand, as theirs — the default `from` is
 * the enemy, so the taker is this card's controller; `from: "self"` gives your own cards away. `cards` names
 * which: `"chosen"` the ones the step's prompt picked, `"unchosen"` every other card of that hand
 * (Classic #9: "they keep one card of their choice and give you the rest" — their own hand pick, then
 * this), `"all"` the whole hand, `"random"` `count` different cards at random (R60). Hand order,
 * snapshotted first, so the cap burns the last of them.
 */
export function giveFromHand(
  args: {
    from?: PlayerSpec;
    cards?: "chosen" | "unchosen" | "all" | "random";
    count?: number;
  } & TakenRiders = {},
): Effect {
  return {
    kind: "giveFromHand",
    apply(ctx): void {
      const from = playerOf(ctx, args.from ?? "enemy");
      const hand = [...ctx.state.players[from].hand];
      const chosen = new Set(chosenIds(ctx));
      const which = args.cards ?? "chosen";
      const taken =
        which === "all"
          ? hand
          : which === "chosen"
            ? hand.filter((card) => chosen.has(card.id))
            : which === "unchosen"
              ? hand.filter((card) => !chosen.has(card.id))
              : ctx.rng.shuffle(hand).slice(0, Math.max(0, Math.trunc(args.count ?? 1)));
      for (const card of taken) takeWith(ctx, card, opponentOf(from), args);
    },
  };
}

/**
 * B5 E2, E16: cards out of `from`'s library into the other player's hand, as theirs (Classic+
 * #12.3 Fluffy Grip: "steal a random Unit from your opponent's deck"). `pick` is `"random"` — `count`
 * different matching cards (R60) — `"top"` or `"bottom"`, or `"chosen"`, the cards the step's
 * prompt picked out of it. `filter` narrows the pool as a library reveal does (a unit-token card
 * never leaves a library for a hand this way, R218). No match takes nothing. The library's owner
 * learns only that cards left it (R466).
 */
export function takeFromLibrary(
  args: {
    from?: PlayerSpec;
    pick?: "random" | "top" | "bottom" | "chosen";
    count?: number;
    filter?: LibraryFilter;
  } & TakenRiders = {},
): Effect {
  return {
    kind: "takeFromLibrary",
    apply(ctx): void {
      const from = playerOf(ctx, args.from ?? "enemy");
      const pool = ctx.state.players[from].library.filter((card) =>
        matchesLibraryFilter(ctx.state, card, args.filter ?? {}),
      );
      const count = Math.max(0, Math.trunc(args.count ?? 1));
      const pick = args.pick ?? "random";
      const chosen = new Set(chosenIds(ctx));
      const taken =
        pick === "chosen"
          ? pool.filter((card) => chosen.has(card.id))
          : pick === "top"
            ? pool.slice(0, count)
            : pick === "bottom"
              ? pool.slice(Math.max(0, pool.length - count)).reverse()
              : ctx.rng.shuffle(pool).slice(0, count);
      for (const card of taken) takeWith(ctx, card, opponentOf(from), args);
    },
  };
}

/**
 * B5 E16: one draw of this card's controller's, taken from the other player's library — its bottom
 * card by default (Classic #58 Common Resources). It is a draw of the controller's
 * (`ownership.drawFromLibraryOf`): their hand cap, a cast on draw for them, the draw counters. An
 * empty library gives nothing and deals no fatigue. One draw per effect, so a card that draws two
 * returns two, and a cast on draw that asks something pauses the list between them (R113).
 */
export function drawFromOpponent(args: { end?: LibraryEnd } = {}): Effect {
  return {
    kind: "drawFromOpponent",
    apply(ctx): void {
      drawFromLibraryOf(ctx, ctx.controller, playerOf(ctx, "enemy"), args.end ?? "bottom");
    },
  };
}
