// R2722002: when a hidden card makes its entrance (its intro, its sting, its animation and a Mythic's
// takeover): as it reveals, never as it is set face-down.
//
// A Trap or Field Trap reveals at its first firing (`trapFired` without `again`, R2657001), and so
// does a Cloaked card (§6.1, R783004–R783008), whose Activate: Reveal is its firing (R783006). The
// firing names the card to each seat that may read it where it is now, both while it stands face-up
// or lies in a graveyard (R97, R2734000), so every seat that reads it makes the entrance there, and
// none twice. Its face-down set is a `cardPlayed` with the fresh id R227 gives it (`formerId`) on its
// controller's seat and the sentinel on the other, and makes no entrance on either. Every predicate
// reads the seat's own redacted event and nothing else (CLAUDE.md rule 7).

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_DEF_ID as HIDDEN_CARD } from "../audio/constants.ts";

type Fired = Extract<GameEvent, { type: "trapFired" }>;

/**
 * A play that sets its card face-down: the controller's own set, which took a fresh id and still
 * names the hand card it was (R227), or any play the seat reads as the sentinel.
 */
export function isFaceDownSet(event: GameEvent): boolean {
  return event.type === "cardPlayed" && (event.formerId !== undefined || event.defId === HIDDEN_CARD);
}

/** A hidden card's first firing, on a seat that can read it: where a Trap or a Cloaked card reveals. */
export function firstReveal(event: GameEvent): event is Fired {
  return event.type === "trapFired" && event.again !== true && event.defId !== HIDDEN_CARD;
}
