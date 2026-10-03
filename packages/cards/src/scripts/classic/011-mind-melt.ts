// C #11 Mind Melt (SPEC §8.6 row 11, §6.3 Look at a hand, §10.6, §10.8; R65, R81, R177). Spell, cost 1,
// Common.
//   Base:    "Look at your opponent's hand. Exile {cards|card|cards} from it."
//   Radiant: "Look at your opponent's hand. Choose a cost. Exile every card of that cost from it."
//   Engine:  "Look at a hand (§6.3, §10.8): their hand cards are the options of a `pick` prompt of one
//            card (§10.6), seen by you alone. Radiant: a `mode` prompt whose options are their hand
//            grouped by cost (the cost each would be played for now, R65); every card of the chosen
//            cost is exiled. The opponent sees that a prompt is open, then which cards left their
//            hand (exile is public). An empty hand opens no prompt. Tunes: cards exiled 1 ↑."
//
// LOOKING AT THE HAND is the prompt itself (B5 E17): `chooseFromHand({ of: "enemy" })` offers the
// opponent's hand cards to you, and `viewFor` shows an open prompt's options to the player it is for
// alone — the hand's owner reads only that a prompt is open for you (R177). The answer exiles what it
// picked; exile is public, so from then on both players read those cards. The count is the card's
// declared number (`param(ctx, "cards")`), so an Upgrade makes it two; a hand shorter than that
// offers what it holds, and an empty hand asks nothing (`openPrompt` opens no prompt without options).
//
// THE RADIANT COST is the one each card would be played for now (R65, `effectiveCost`): the engine's
// `chooseCostInHand` groups the hand by it into the options of a `number` prompt, and `exileMatching`
// reads the same cost when it sweeps the hand, so the group chosen is the group exiled. The option
// captions name the cards of each cost, which only you are shown.

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { chooseCostInHand, chooseFromHand, chosenNumber, exile, exileMatching } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-011");

export const base: Script = {
  cry: (ctx) => [
    chooseFromHand({
      of: "enemy",
      count: param(ctx, "cards"),
      step: "exile",
      prompt: "Look at your opponent's hand: exile a card from it",
    }),
  ],
  resume: {
    exile: (ctx) => ctx.targets.map((_, index) => exile({ target: { of: "chosen", index } })),
  },
};

export const radiant: Script = {
  cry: () => [chooseCostInHand({ of: "enemy", step: "cost", prompt: "Look at your opponent's hand: choose a cost" })],
  resume: {
    cost: (ctx) => {
      const cost = chosenNumber(ctx);
      return cost === null ? [] : [exileMatching({ zones: ["hand"], player: "enemy", cost })];
    },
  },
};
