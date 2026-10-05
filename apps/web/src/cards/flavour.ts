// A card's flavour line and artist credit (R660), read from the sidecar beside the catalog
// (`@jackioh/cards/flavour.json`, packages/cards/src/flavour.ts), the way the catalog itself is read
// from its JSON: the bundle carries the words, and no server is asked. Presentation only (CLAUDE.md
// rule 7). The sidecar is keyed by catalog id, so the hidden sentinel and a match-made definition
// (a Fuse's, R77) have no entry and nothing is shown for them.

import flavourJson from "@jackioh/cards/flavour.json";
import type { CardFlavour } from "@jackioh/cards";

import { corruptedText, isGlitch } from "./glitch.ts";

export const CARD_FLAVOUR: Readonly<Record<string, CardFlavour>> = flavourJson;

/** The card's entry when it has a flavour line or an artist, else null; Glitch's corrupted (R662). */
export function flavourFor(
  defId: string,
  sidecar: Readonly<Record<string, CardFlavour>> = CARD_FLAVOUR,
): CardFlavour | null {
  if (!Object.hasOwn(sidecar, defId)) return null;
  const entry = sidecar[defId];
  if (entry === undefined || (entry.flavour === undefined && entry.artist === undefined)) return null;
  if (!isGlitch(defId)) return entry;
  return {
    ...(entry.flavour === undefined ? {} : { flavour: corruptedText(entry.flavour) }),
    ...(entry.artist === undefined ? {} : { artist: corruptedText(entry.artist) }),
  };
}
