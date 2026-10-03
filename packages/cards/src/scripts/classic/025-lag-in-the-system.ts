// C #25 Lag in the System (SPEC §8.6 row 25, §6.3 Exile; R13, R65, R66, R113, R135, R396). Spell,
// cost 0, Common.
//   Base:    "Exile every card on the field, in hands and in decks that costs ({threshold}) or less."
//   Radiant: "Exile every enemy card on the field, in their hand and in their deck that costs
//            ({threshold}) or less."
//   Engine:  "C #18 with the numbers fixed at 0 and 1: the same zones, the same cost reading (R65 at
//            resolution; an X card in a hand or deck costs 0, so it goes, and on the field it costs its
//            X, R396), the Spell itself spared, graveyards and exile untouched. Tunes: threshold 1 ↑."
//
// THE SET is read once, as the Spell resolves (`forEachCard`, R66, R113), and each card is then its
// own exile (R135), so a card that a sweep uncovers — one dormant under a Stack pile, which is not on
// the field while it lies there (§3.2, R13) — is not in it. The zones are the field (the tops of the
// unit piles and every backrow card, face-down ones included, both sides in R68's walk), then each
// side's hand and deck, the Spell's controller first. Graveyards and exile are never read. The Spell
// itself is resolving (§10.5), in none of those zones, so it is spared.
//
// THE COST is `costNow` (R396), R65's cost as it stands at resolution: a hand card at its hand cost
// (its player's discounts included), a deck or field card at its own; an X card counts the X it was
// played for on the field, and 0 anywhere else or when it arrived with none chosen. An exiled card is
// public, so each `exiled` event names it; none carries a deck position.
//
// THE RADIANT FACE reads the opponent's side only: their field, hand and deck.

import type { CardInstance, EffectContext, Script } from "@jackioh/engine";
import { costNow, param, zoneCards } from "@jackioh/engine";
import { cardsInScope, exile, forEachCard, sidesOf } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-025");

type Whose = "any" | "enemy";

/** Every card on the field and in the hands and decks of `whose` sides, in the order the header gives. */
function reachable(ctx: EffectContext, whose: Whose): CardInstance[] {
  const field = cardsInScope(ctx, { side: whose, rows: ["units", "backrow"] });
  const players = whose === "enemy" ? sidesOf(ctx, "enemy") : [ctx.controller, ...sidesOf(ctx, "enemy")];
  const piles = players.flatMap((player) => [...zoneCards(ctx.state, player, "hand"), ...zoneCards(ctx.state, player, "library")]);
  return [...field, ...piles];
}

function lag(whose: Whose): Script {
  return {
    cry: (ctx) => {
      const threshold = param(ctx, "threshold");
      return [
        forEachCard({
          cards: (read) => reachable(read, whose).filter((card) => costNow(read.state, card) <= threshold),
          each: (instanceId) => exile({ target: { of: "instance", instanceId } }),
        }),
      ];
    },
  };
}

export const base: Script = lag("any");

export const radiant: Script = lag("enemy");
