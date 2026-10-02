// C+ #65.4 Golden Grape (SPEC §8.7 row 65.4). (1) Spell, Fruit, Token (printed Legendary).
//   Base:    "Make a card on your side of the field or in your hand Radiant."
//   Radiant: "Make a card on your side of the field or in your hand Radiant, and the cards next to it
//            (beside it in its row, or beside it in your hand)."
//   Engine:  "A declared pick (R81): a card in your hand or one of your permanents. Make Radiant (§6.3):
//            a field card converts in place (§5.2); a Radiant card changes nothing. 'Next to it' is
//            §3.1's adjacency on the field (your side, the same row, N − 1 and N + 1) and the
//            neighbours by index in the hand. Tunes: none."
//
// The pick travels in the play (R81): your Units (tops of piles), your backrow cards (a face-down one
// included — it is yours to read, R33) and your other hand cards. Make Radiant is the engine's
// `setRadiant`: a field card converts in place with no Cry (R22), a card already Radiant is left as it
// is, and a change to a card someone may not read — a hand card, a face-down card — is cued to them
// redacted whether or not the flag moved (R177), so the cue never tells them which were Radiant.
//
// The Radiant face's neighbours are read as the Spell resolves, which is when this hook runs (a Spell's
// `cry` is its resolution): on the field §3.1's adjacency on the pick's own side and row (`adjacentTo`;
// an empty or dormant neighbour is nothing), in the hand the cards at the indices either side of it.

import type { CardInstance, EffectContext, Script } from "@jackioh/engine";
import { zoneCards } from "@jackioh/engine";
import { adjacentTo, instanceOf, setRadiant } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-065-4");

/** §8.7: a card on your side of the field or in your hand. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "ally", of: ["unit", "backrow", "hand"] } }];

/** "The cards next to it": its row neighbours on the field, or its neighbours by index in your hand. */
function neighboursOf(ctx: EffectContext, picked: CardInstance): CardInstance[] {
  if (picked.zone.z === "field") return adjacentTo(ctx, { of: "chosen" });
  if (picked.zone.z !== "hand") return [];
  const hand = zoneCards(ctx.state, picked.owner, "hand");
  const at = hand.findIndex((card) => card.id === picked.id);
  if (at < 0) return [];
  return [hand[at - 1], hand[at + 1]].filter((card): card is CardInstance => card !== undefined);
}

export const base: Script = {
  targets,
  cry: () => [setRadiant({ target: { of: "chosen" } })],
};

export const radiant: Script = {
  targets,
  cry: (ctx) => {
    const picked = instanceOf(ctx, { of: "chosen" });
    if (picked === null) return [];
    return [
      setRadiant({ target: { of: "chosen" } }),
      ...neighboursOf(ctx, picked).map((card) => setRadiant({ instanceId: card.id })),
    ];
  },
};
