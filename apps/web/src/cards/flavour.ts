// A card's flavour line and artist credit (R660), read from the sidecar beside the catalog
// (`@jackioh/cards/flavour.json`, which the aliases resolve to crates/cards/flavour.json), the way
// the catalog itself is read from its JSON: the bundle carries the words, and no server is asked.
// Presentation only (CLAUDE.md rule 7). The sidecar is keyed by catalog id, so the hidden sentinel
// and a match-made definition (a Fuse's, R77) have no entry and nothing is shown for them.
//
// A card's flavour line and its artist are words about the card, not the card, so an edit to
// `flavour.json` changes no rule, no def and no patch. The Rust catalog checks hold the file to its
// contract (each string trimmed, non-empty, single-line and under its cap below);
// `flavour.test.tsx` also proves the lines speak no rules words, as the voice lines must not.

import flavourJson from "@jackioh/cards/flavour.json";

/** One card's sidecar entry. Either field may be absent; an artist arrives with its art. */
export type CardFlavour = { readonly flavour?: string; readonly artist?: string };

/** A flavour line's longest length, in characters: one or two short lines under the rules. */
export const FLAVOUR_MAX_CHARS = 120;

/** An artist credit's longest length, in characters. */
export const ARTIST_MAX_CHARS = 60;

export const CARD_FLAVOUR: Readonly<Record<string, CardFlavour>> = flavourJson;

/** The card's entry when it has a flavour line or an artist, else null. */
export function flavourFor(
  defId: string,
  sidecar: Readonly<Record<string, CardFlavour>> = CARD_FLAVOUR,
): CardFlavour | null {
  if (!Object.hasOwn(sidecar, defId)) return null;
  const entry = sidecar[defId];
  if (entry === undefined || (entry.flavour === undefined && entry.artist === undefined)) return null;
  return entry;
}
