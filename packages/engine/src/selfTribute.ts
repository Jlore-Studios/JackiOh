// "When …, Tribute this" (Classic #88 Siphon Squad, R403): a condition on a permanent's own text
// (`Script.tributeWhen`) that every state check reads (§4.5), the one right after the card arrives
// included. The cards it holds for are sacrificed together (§6.3 Tribute: a death, Indestructible
// bypassed) before the check collects anything. Only a card acting on the field counts — the top of a
// unit pile or a backrow card, face-down ones included (R403: a Trap with no activation condition is
// live while face-down) — and a Vanilla card has no text (`scriptOf`, R115).

import { PLAYER_IDS } from "@jackioh/shared";
import { scriptOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import { cardAt, slotsOf } from "./zones";

/** The permanents whose `tributeWhen` holds now, in R68's order (the active side first). */
export function tributesDue(state: GameState): CardInstance[] {
  const order = state.active === "p2" ? ["p2", "p1"] as const : PLAYER_IDS;
  return order.flatMap((player) =>
    (["units", "backrow"] as const).flatMap((row) =>
      slotsOf(player, row).flatMap((ref) => {
        const card = cardAt(state, ref);
        const hook = card === null ? undefined : scriptOf(card).tributeWhen;
        return card !== null && hook?.({ state, self: card, radiant: card.radiant }) === true ? [card] : [];
      }),
    ),
  );
}
