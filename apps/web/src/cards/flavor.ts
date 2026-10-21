// R1437000: which flavor a card draws. Every catalog entry carries one; a transient or custom
// definition (fused, crafted, custom) carries none, so it draws Physical, and a fused one the
// first ingredient's. The one lookup every flavor effect reads (docs/polish/8-flavor.md, #1436).

import type { CardDef, CardDefs, Flavor } from "@jackioh/shared";

import { CATALOG } from "../wire/cards.ts";

/** What a definition with no flavor draws (R1437000). */
export const FLAVOR_FALLBACK = "Physical" satisfies Flavor;

/**
 * `def`'s flavor; else, for a fused one, its first ingredient's in `catalog`; else Physical.
 * `undefined` is a card no catalog knows, such as a hidden one.
 */
export function flavorOf(
  def: Pick<CardDef, "flavor" | "ingredients"> | undefined,
  catalog: Readonly<CardDefs> = CATALOG,
): Flavor {
  if (def === undefined) return FLAVOR_FALLBACK;
  if (def.flavor !== undefined) return def.flavor;
  const first = def.ingredients?.[0];
  return (first === undefined ? undefined : catalog[first.defId]?.flavor) ?? FLAVOR_FALLBACK;
}

/**
 * The flavor of the card `defId` names: the bundled catalog's entry, else the match's own
 * definition (`PlayerView.defs`, R243). It reads the bundled `CATALOG` and not `CatalogContext`,
 * because a replay's older catalog has no flavors (R1437000).
 */
export function flavorOfId(defId: string, matchDefs?: Readonly<CardDefs>): Flavor {
  return flavorOf(CATALOG[defId] ?? matchDefs?.[defId]);
}
