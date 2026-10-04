// Flavour text and artist credits (issue #265, R658): a sidecar beside the catalog, keyed by card
// id, never a catalog field. A card's flavour line and its artist are words about the card, not the
// card, so an edit to `flavour.json` changes no rule, no def and no patch: it needs no pending
// fragment (README §8), and the designer may rewrite any line at will.
//
// `test/flavour.test.ts` holds the file to its contract: every key is a catalog card or token, every
// card and token has a flavour line, an entry carries only `flavour` and `artist`, each a trimmed,
// non-empty, single-line string under its cap below. The client reads the same file
// (apps/web/src/cards/flavour.ts) and also proves the lines speak no rules words, as the voice lines
// must not (issue #115).

import flavourJson from "../flavour.json";

/** One card's sidecar entry. Either field may be absent; an artist arrives with its art. */
export type CardFlavour = { readonly flavour?: string; readonly artist?: string };

/** A flavour line's longest length, in characters: one or two short lines under the rules. */
export const FLAVOUR_MAX_CHARS = 120;

/** An artist credit's longest length, in characters. */
export const ARTIST_MAX_CHARS = 60;

/** Every card's flavour line and artist, by catalog id. */
export const FLAVOUR: Readonly<Record<string, CardFlavour>> = flavourJson;
