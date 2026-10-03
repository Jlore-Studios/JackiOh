// C #23 Devil's Pact (SPEC §8.6 row 23). Field Spell, cost 2, Rare.
//   Base:    "Cry: Discard {discards|card|cards}.
//             Activate: This turn, each card you play is replaced by a Book of Flame."
//   Radiant: "Cry: Discard {discards|card|cards}.
//             Activate: This turn, each card you play is replaced by a Radiant Book of Flame."
//   (discards: 666 on the base face — the whole hand, the joke — and 6 on the Radiant face.)
//
// The Cry (a Field Spell's Cry) discards {discards} random cards (R640): when the hand holds that
// many or fewer there is nothing to choose, and the whole hand goes (a discard all the same, so
// C #64 sees each card). An empty hand discards nothing.
//
// Activate (R384), once per turn, installs a this-turn player modifier (`replacePlays`, R449): at
// §10.5 step 3 each card you play — a cast included (R70) — is replaced by a new C #16 Book of Flame
// (its Radiant face on this card's Radiant face), which resolves as that play: it is announced as a
// Book of Flame, counts as one, and asks its target then, since the old card's choices were the old
// card's. The old card ceases to exist (R35), unread by the opponent when it left a hand (R177), and
// the price paid was the old card's; a Unit or a trap replaced this way takes no zone. A card played
// before the activation is not replaced, and the modifier expires at cleanup. Activating is not a play.

import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { param, zoneCount } from "@jackioh/engine";
import { addPlayerModifier, discardHand, discardRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-023");

/** C #16, the Book of Flame this card names (its `refs`). */
const BOOK_OF_FLAME = "classic-016";

/** "Discard {discards} cards": that many at random, or the whole hand when it holds no more. */
function discardCards(ctx: EffectContext): Effect[] {
  const count = param(ctx, "discards");
  const held = zoneCount(ctx.state, ctx.controller, "hand");
  if (held === 0) return [];
  if (held <= count) return [discardHand({ player: "self" })];
  return [discardRandom({ count })];
}

function pact(radiant: boolean): Script {
  return {
    cry: discardCards,
    activations: [
      {
        id: "pact",
        label: radiant
          ? "This turn, each card you play is replaced by a Radiant Book of Flame"
          : "This turn, each card you play is replaced by a Book of Flame",
        uses: 1,
        run: (ctx) => [
          addPlayerModifier({
            mod: { kind: "replacePlays", defId: BOOK_OF_FLAME, radiant, expiry: { until: "thisTurn", turn: ctx.state.turn } },
          }),
        ],
      },
    ],
  };
}

export const base: Script = pact(false);

export const radiant: Script = pact(true);
