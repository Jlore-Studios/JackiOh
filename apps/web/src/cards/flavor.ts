// R1437000: which flavor a card draws, the one lookup every flavor effect reads (docs/polish/8-flavor.md,
// #1436). A transient or custom definition (fused, crafted, custom) carries none, so it draws Physical,
// and a fused one the first ingredient's.

import type { CardDef, CardDefs, Flavor } from "@jackioh/shared";

import { currentCatalog } from "../game/currentCatalog.ts";

export const FLAVOR_FALLBACK = "Physical" satisfies Flavor;

/** `undefined` is a card no catalog knows, such as a hidden one. */
export function flavorOf(
  def: Pick<CardDef, "flavor" | "ingredients"> | undefined,
  // #4180: the module's catalog (`game/currentCatalog.ts`), read in the call: flavor draws on the
  // board and in the inspect overlays, once the module is in.
  catalog: Readonly<CardDefs> = currentCatalog().cards,
): Flavor {
  if (def === undefined) return FLAVOR_FALLBACK;
  if (def.flavor !== undefined) return def.flavor;
  const first = def.ingredients?.[0];
  return (first === undefined ? undefined : catalog[first.defId]?.flavor) ?? FLAVOR_FALLBACK;
}

/**
 * The flavor of the card `defId` names: the module catalog's entry, else the match's own
 * definition (`PlayerView.defs`, R243). It reads `currentCatalog()` and not `CatalogContext`,
 * because a replay's older catalog has no flavors (R1437000).
 */
export function flavorOfId(defId: string, matchDefs?: Readonly<CardDefs>): Flavor {
  return flavorOf(currentCatalog().cards[defId] ?? matchDefs?.[defId]);
}
