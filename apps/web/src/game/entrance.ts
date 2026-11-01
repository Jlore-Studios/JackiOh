// R2722002: when a hidden card makes its entrance (its intro, its sting, its animation and a Mythic's
// takeover): as it reveals, never as it is set face-down.
//
// A Trap or Field Trap reveals at its first firing (`trapFired` without `again`, R2657001), which
// names the card only to its controller (R154, R763). A Cloaked card (§6.1, R783004–R783008) reveals
// on its Activate: Reveal: an `activated` that names it to both seats once it stands face-up or has
// resolved (R97), then the `trapFired` that is its firing (R783006). Its controller's seat makes the
// entrance at that `trapFired`, the other seat, which reads it as the sentinel, at the `activated`,
// so neither seat makes it twice. Its face-down set is a
// `cardPlayed` with the fresh id R227 gives it (`formerId`) on its controller's seat and the
// sentinel on the other, and makes no entrance on either. Every predicate reads the seat's own
// redacted event and nothing else (CLAUDE.md rule 7).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { HIDDEN_DEF_ID as HIDDEN_CARD } from "../audio/constants.ts";

/** R783006: the Reveal ability every Cloaked set card has (engine `alt_play::CLOAKED_REVEAL`). */
export const CLOAKED_REVEAL_ABILITY = "reveal";

type Activated = Extract<GameEvent, { type: "activated" }>;
type Fired = Extract<GameEvent, { type: "trapFired" }>;

/**
 * A play that sets its card face-down: the controller's own set, which took a fresh id and still
 * names the hand card it was (R227), or any play the seat reads as the sentinel.
 */
export function isFaceDownSet(event: GameEvent): boolean {
  return event.type === "cardPlayed" && (event.formerId !== undefined || event.defId === HIDDEN_CARD);
}

/** A hidden card's first firing, on the seat that can read it: where a Trap, or a Cloaked card on its controller's seat, reveals. */
export function firstReveal(event: GameEvent): event is Fired {
  return event.type === "trapFired" && event.again !== true && event.defId !== HIDDEN_CARD;
}

/** A Cloaked card's Activate: Reveal, named, on the seat that is not its controller's: where that seat sees it reveal. */
export function otherSeatReveal(event: GameEvent, viewer: PlayerId): event is Activated {
  return event.type === "activated" && event.ability === CLOAKED_REVEAL_ABILITY && event.defId !== HIDDEN_CARD && event.player !== viewer;
}
