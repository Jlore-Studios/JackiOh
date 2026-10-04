// C #9 Income Tax (SPEC §8.6 row 9, §6.3 Steal, §10.6; R12, R33, R58, R61, R78, R97, R99, R317,
// R521). Trap, cost 2, Legendary.
//   Base:    "Reveals when the cards your opponent has drawn in a turn reach {draws}: They keep one
//            card of their choice and give you the rest of their hand."
//   Radiant: the same, then "The cards you get cost ({discount}) less."
//   Engine:  "The per-turn / per-game counts (§10.1): draws per player per turn, counted on both
//            players' turns, the start-of-turn draw included, so on their own turn any extra draw sets
//            it off. The trap fires once that draw is complete (a cast-on-draw card is cast first,
//            R58). The opponent picks the one hand card to keep (their prompt); the rest move to your
//            hand and become yours (cards between players' piles, §6.3 Steal, §3.2; the owner changes,
//            R12); your hand cap burns the overflow into your graveyard. A hand of one card or none
//            gives nothing and asks nothing. Radiant: `costMod` −1 on each card you get. Tunes:
//            trigger draw 2 ↓ (never below 2); Radiant discount 1 ↑."
//
// THE CONDITION (R99: a trap's `when`, so a draw that is not the one leaves it set) is the draw count
// B5 E4 keeps per player per turn, whoever's turn it is, which rides each `drawn` event as
// `turnDraw`: it fires on the opponent's draw that makes their count this turn the card's number
// (`param(ctx, "draws")`, 2). The count is the engine's (B5 E4, R521): the start-of-turn draw counts, a
// burned or cast-on-draw card counts (it left the deck by a draw), a draw a limit stops never happened,
// so under a limit of 1 the second never comes, and a draw from an empty deck draws no card, so it
// makes no `drawn` and fires nothing. A card cast on that draw is cast first (R58).
//
// FIRING, it asks the opponent to keep one card of their hand (`chooseFromHand` over their own hand,
// held by them): the options are theirs to read alone (R177). The answer hands every other card to
// you (`giveFromHand({ cards: "unchosen" })`, B5 E16): each becomes yours (R12), so your hand cap burns
// what does not fit into your graveyard (R317), and once in your hand the opponent reads none of them
// (R97). A hand of one card or none gives nothing, so the firing asks nothing (R61: it fired, and did
// nothing). The Radiant face's discount (`costMod`, which R78 keeps in every zone) is read as the trap
// fires and carried to the answer, since the trap is in its owner's graveyard by then.

import type { GameEvent } from "@jackioh/shared";
import type { EffectContext, Script, TriggerDef } from "@jackioh/engine";
import { param, zoneCount } from "@jackioh/engine";
import { chooseFromHand, giveFromHand } from "@jackioh/engine/effects";
import { opponentOf } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-009");

/** Where the Radiant face's discount travels from the firing to the answer (§10.6). */
const DISCOUNT = "discount";

function taxes(ctx: EffectContext & { event: GameEvent }): boolean {
  const event = ctx.event;
  return event.type === "drawn" && event.player === opponentOf(ctx.controller) && event.turnDraw === param(ctx, "draws");
}

function tax(radiant: boolean): TriggerDef {
  return {
    id: "income-tax",
    on: ["drawn"],
    when: taxes,
    run: (ctx) => {
      if (zoneCount(ctx.state, opponentOf(ctx.controller), "hand") <= 1) return [];
      return [
        chooseFromHand({
          of: "enemy",
          by: "enemy",
          count: 1,
          step: "keep",
          prompt: "Income Tax: keep one card; your opponent takes the rest",
          data: radiant ? { [DISCOUNT]: param(ctx, "discount") } : {},
        }),
      ];
    },
  };
}

function discountOf(ctx: EffectContext): number {
  const value = ctx.data[DISCOUNT];
  return typeof value === "number" && Number.isInteger(value) ? value : 0;
}

export const base: Script = {
  triggers: [tax(false)],
  resume: {
    keep: () => [giveFromHand({ from: "enemy", cards: "unchosen" })],
  },
};

export const radiant: Script = {
  triggers: [tax(true)],
  resume: {
    keep: (ctx) => [giveFromHand({ from: "enemy", cards: "unchosen", costMod: -discountOf(ctx) })],
  },
};
