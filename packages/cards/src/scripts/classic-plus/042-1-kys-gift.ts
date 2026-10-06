// C+ #42.1 KY's Gift (SPEC §8.7 row 42.1, §7, R16, R62, R380). (4) Field Spell, KY, Token (printed
// Legendary); C+ #42 KY's Test's Hard reward.
//   Start of turn: Gain {mana} mana. Your opponent discards {discards|card|cards}. Heal your hero {heal}. Add a
//   random Book, a random KY card, a random Legendary card and a random (4) Cost card to your hand.
//   They cost (0). Radiant: 2 mana, 2 discards, heal 10, and the four cards are Radiant.
//
// Its controller's start of turn (R62), in the order written. The mana is temporary (§2.3). The discard
// is random from the opponent's hand (R682): fewer cards than asked taking what they have and none
// taking nothing. The four cards come from non-token pools of every set (R380), each at `costOverride`
// 0; the hand cap burns. The numbers are the declared `mana`, `discards` and `heal` (R386); one script
// runs both faces.

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog, discardRandom, gainMana, heal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-042-1");

/** A Book, a KY card, a Legendary card and a (4) Cost card, in the order the text names them. */
const POOLS = [{ tags: ["Book" as const] }, { tags: ["KY" as const] }, { rarity: "Legendary" as const }, { cost: 4 }];

export const base: Script = {
  startOfTurn: (ctx) => [
    gainMana({ amount: param(ctx, "mana") }),
    discardRandom({ count: param(ctx, "discards"), player: "enemy" }),
    heal({ target: { of: "selfHero" }, amount: param(ctx, "heal") }),
    ...POOLS.map((query) => addRandomFromCatalog({ query, costOverride: 0, radiant: ctx.radiant })),
  ],
};

// The same script: the Radiant face's 2, 2 and 10 are its declared numbers, and `ctx.radiant` makes the cards Radiant.
export const radiant: Script = base;
