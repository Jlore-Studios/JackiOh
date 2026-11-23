// R2094000, as R3730006 amends it: the device remembers its last co-op deck pick, a random deal
// included, so a co-op lobby or a classic co-op room's join card starts on it. The key is the one the
// old co-op panel wrote, so a device keeps what it remembered.

import type { SavedDeck } from "../net/api.ts";
import { getStored, setStored } from "../net/storage.ts";
import type { CoopPick } from "./api.ts";

export const COOP_DECK_KEY = "jackioh.coop.deck";

/** What the key holds for a random deal. */
const RANDOM = "random";

/** The remembered pick: a deck id, `random`, or null when none or storage refuses. */
function readCoopDeck(): string | null {
  const raw = getStored(COOP_DECK_KEY);
  return raw !== null && raw.length > 0 ? raw : null;
}

/** Remember a co-op pick; a refusal costs nothing, since remembering is a convenience. */
export function rememberCoopPick(pick: CoopPick): void {
  setStored(COOP_DECK_KEY, "random" in pick ? RANDOM : pick.deckId);
}

/**
 * The deck a co-op pick starts on: the remembered deck while it is still saved, else the first saved
 * deck, else null, a random deal. A remembered random deal stays one. Before the decks are read there
 * are none, so a remembered deck the account no longer has is never sent.
 */
export function coopStartingDeck(decks: readonly SavedDeck[]): SavedDeck | null {
  const remembered = readCoopDeck();
  if (remembered === RANDOM) return null;
  return decks.find((deck) => deck.id === remembered) ?? decks[0] ?? null;
}
