// A card changing owner (docs/classic-sets.md B5 E2, E16): the second exception to "a card always
// goes to its owner's piles" (§3.2), beside R73's library swap. A card taken off the stack as it is
// cast, out of a hand or out of a deck moves to the thief's hand and its owner becomes the thief, so
// every pile it reaches afterwards is the thief's. The hand cap applies to the thief's hand, and a
// card it burns goes to the thief's graveyard, since the thief owns it by then (§2.4).

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { addToHand } from "./draw";
import type { EngineSink } from "./resolve";
import type { CardInstance } from "./state";

type StolenFrom = Extract<GameEvent, { type: "stolen" }>["zone"];

function stolenFrom(instance: CardInstance): StolenFrom | null {
  const zone = instance.zone.z;
  return zone === "gone" ? null : zone;
}

/**
 * E2, E16: move `card` to `thief`'s hand as the thief's own card. `stolen` is emitted first, naming
 * the pile it came from and both players, then the hand's own `addedToHand` or `burned` (§2.4).
 * A card that has ceased to exist is not taken. Returns where it went, or null when it was not taken.
 */
export function takeIntoHand(sink: EngineSink, card: CardInstance, thief: PlayerId): "hand" | "burned" | null {
  const zone = stolenFrom(card);
  if (zone === null) return null;
  const from = card.owner;
  card.owner = thief;
  card.controller = thief;
  sink.events.push({ type: "stolen", instanceId: card.id, defId: card.defId, from, to: thief, zone });
  return addToHand(sink, card);
}
