// A card changing owner (docs/classic-sets.md B5 E2, E16): the second exception to "a card always
// goes to its owner's piles" (§3.2), beside R73's library swap. A card taken off the stack as it is
// cast, out of a hand or out of a deck moves to the thief's hand and its owner becomes the thief, so
// every pile it reaches afterwards is the thief's. The hand cap applies to the thief's hand, and a
// card it burns goes to the thief's graveyard, since the thief owns it by then (§2.4).
//
// Two ways in, one change of owner (`changeOwner`):
//   - `takeIntoHand`: the card goes straight to the thief's hand (E2's countered Spell, E16's cards
//     handed over, Classic+ #12.3's card out of the deck).
//   - `drawFromLibraryOf`: a draw of the thief's own, taken from the other player's library (Classic
//     #58: "draw the bottom card of your opponent's deck") — §2.4's draw from the moment the card has
//     left the library, so the thief's hand cap, a cast on draw for the thief and the game draw counter
//     all apply, and an empty library gives nothing and deals no fatigue to anybody.
//
// Hidden information (R466). `stolen` names the card to whoever could read it where it was taken
// from and to whoever can read it where it is now (`viewFor.redactEvent`): a card out of a hand is its
// holder's, a face-down card its controller's, a public one everyone's, and a card out of a library
// nobody's, so the victim of a deck steal never learns which card left. The victim's own library list (R310) would give it away by what it stops listing, so a card
// taken out of a library by the other player turns the rest of that library unknown to its owner
// (R312's `hideFromOwner`): the list may show less than they could piece together, never more.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { PLAYER_IDS } from "@jackioh/shared";
import { addToHand, completeDraw, type DrawOutcome } from "./draw";
import { hideFromOwner } from "./ownLibrary";
import { isFaceDown } from "./preview";
import type { EngineSink } from "./resolve";
import type { CardInstance, GameState } from "./state";

type StolenFrom = Extract<GameEvent, { type: "stolen" }>["zone"];

function stolenFrom(instance: CardInstance): StolenFrom | null {
  const zone = instance.zone.z;
  return zone === "gone" ? null : zone;
}

/**
 * E2, E16: `card` becomes `thief`'s — owner and controller — and `stolen` says so, naming the pile it
 * came from and both players. It does not move the card: the caller puts it in the hand, or draws it.
 * R311's record of what the old owner was shown of the card was theirs, so it goes; R466: a card taken
 * out of the other player's library leaves the rest of that library unknown to them. Returns false,
 * changing nothing, for a card that has ceased to exist.
 */
export function changeOwner(sink: EngineSink, card: CardInstance, thief: PlayerId): boolean {
  const zone = stolenFrom(card);
  if (zone === null) return false;
  // R466: who could read the card where it lies, judged before anything about it changes.
  const readableFrom = readersWhereItLies(sink.state, card);
  const from = card.owner;
  card.owner = thief;
  card.controller = thief;
  hideFromOwner(card);
  if (zone === "library" && from !== thief) {
    for (const left of sink.state.players[from].library) hideFromOwner(left);
  }
  sink.events.push({ type: "stolen", instanceId: card.id, defId: card.defId, from, to: thief, zone, readableFrom });
  return true;
}

/**
 * R466: the players who can read a card where it lies (§9.1, §10.8): a hand is its holder's, a library
 * nobody's, a face-down backrow card its controller's (R33), and a face-up card, a graveyard, an exile
 * pile and the resolving zone everyone's (a play is public, R98).
 */
function readersWhereItLies(state: GameState, card: CardInstance): PlayerId[] {
  const zone = card.zone;
  if (zone.z === "hand") return [zone.player];
  if (zone.z === "library") return [];
  if (zone.z === "field" && isFaceDown(state, card)) return [card.controller];
  return [...PLAYER_IDS];
}

/**
 * E2, E16: move `card` to `thief`'s hand as the thief's own card. `stolen` is emitted first, naming
 * the pile it came from and both players, then the hand's own `addedToHand` or `burned` (§2.4).
 * A card that has ceased to exist is not taken. Returns where it went, or null when it was not taken.
 */
export function takeIntoHand(sink: EngineSink, card: CardInstance, thief: PlayerId): "hand" | "burned" | null {
  if (!changeOwner(sink, card, thief)) return null;
  return addToHand(sink, card);
}

/** Which end of a library a draw from it takes: `library[0]` is the top (`effects/library.ts`). */
export type LibraryEnd = "top" | "bottom";

/**
 * E16, E2: one draw of `drawer`'s, taken from `from`'s library — its bottom card by default (Classic
 * #58 Common Resources). The card leaves that library and becomes the drawer's (`changeOwner`), then
 * §2.4's draw finishes it as the drawer's own (`draw.completeDraw`): the game draw counter, `drawn`,
 * a cast on draw for the drawer (R58) and the drawer's hand cap (R4). An empty library gives nothing
 * and deals no fatigue to anybody: returns null.
 */
export function drawFromLibraryOf(
  sink: EngineSink,
  drawer: PlayerId,
  from: PlayerId,
  end: LibraryEnd = "bottom",
): DrawOutcome | null {
  const library = sink.state.players[from].library;
  if (library.length === 0) return null;
  const card = library[end === "top" ? 0 : library.length - 1];
  if (card === undefined) return null;
  // The owner changes while the card still lies where it was taken from, so `stolen` names that pile;
  // then it leaves the library, as `draw.drawOne` takes its card, before §2.4 finishes the draw.
  changeOwner(sink, card, drawer);
  library.splice(library.indexOf(card), 1);
  return completeDraw(sink, drawer, card);
}
