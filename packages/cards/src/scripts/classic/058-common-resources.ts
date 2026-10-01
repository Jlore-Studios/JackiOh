// C #58 Common Resources (SPEC §8.6 row 58, BUILD M9 Classic row C 58). (2) Field Spell, Common.
//   Base:    "Start of turn: Draw {cards|card|cards} from the bottom of your opponent's deck." (1)
//   Radiant: "Start of turn and end of turn: Draw {cards|card|cards} from the bottom of your
//            opponent's deck." (1)
//   Engine:  "Cards between players' piles (§6.3 Steal, §3.2): a draw of yours taken from the bottom of
//            the opponent's deck, at the start of your turn (Radiant: and at the end of it); it is your
//            draw for your hand cap, cast on draw, your draw limit (§2.4) and your per-turn draw count
//            (§10.1), and the card becomes yours (its owner changes, R12). An empty enemy deck gives
//            nothing, and fatigue for no one. Tunes: cards 1 ↑."
//
// Each draw is B5 E16's `drawFromOpponent`: one draw of this card's controller's out of the bottom of
// the other player's library (`ownership.drawFromLibraryOf`), the owner changing as it leaves (R12, a
// `stolen` event hidden per zone, R97, B5 E16), then §2.4's draw finishing it as the drawer's own — the
// draw counters and `drawn`, a cast on draw for the drawer (R58), the drawer's hand cap (a burn goes
// to the drawer's graveyard, R317). "Draw N" is N draws (§2.4), so the declared count (`param`, R386)
// is that many effects, each its own draw, and a cast on draw that asks pauses the list between them
// (R113). The hooks are §6.2's start- and end-of-turn triggers: their controller's turn only, while the
// card acts on the field (R153), the start one before the turn's own draw (§2.2, R62).

import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { drawFromOpponent } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-058");

/** "Draw {cards} from the bottom of your opponent's deck": that many separate draws (§2.4). */
function drawBottoms(ctx: EffectContext): Effect[] {
  return Array.from({ length: param(ctx, "cards") }, () => drawFromOpponent({ end: "bottom" }));
}

export const base: Script = {
  startOfTurn: drawBottoms,
};

export const radiant: Script = {
  startOfTurn: drawBottoms,
  endOfTurn: drawBottoms,
};
