// R641's hero portraits on the client: which catalog card's art each portrait draws, in `CardArt`'s
// `oval` shape (issue §1). The wire layer's roster (`src/wire/emotes.ts`) names each portrait by its
// card's name, never its id, so this module resolves the name once against the module's catalog
// (`game/currentCatalog.ts`) and keeps the def for `CardArt`.

import type { CardDef, PortraitId } from "@jackioh/shared";
import { PORTRAIT_IDS, PORTRAITS } from "@jackioh/shared";

import { currentCatalog } from "../game/currentCatalog.ts";

function defFor(portrait: PortraitId, catalog: Readonly<Record<string, CardDef>>): { defId: string; def: CardDef } {
  const cardName = PORTRAITS[portrait].cardName;
  for (const [defId, def] of Object.entries(catalog)) {
    if (def.name === cardName) return { defId, def };
  }
  throw new Error(`portrait ${portrait}: no catalog card named ${JSON.stringify(cardName)}`);
}

let defs: Readonly<Record<PortraitId, { defId: string; def: CardDef }>> | null = null;

/**
 * #4180: every portrait's card, resolved once against the module's catalog. A missing card fails
 * loudly, not blank. Call it from a render or an effect, never at import: before the module loads
 * it throws (`game/currentCatalog.ts`).
 */
export function portraitDefs(): Readonly<Record<PortraitId, { defId: string; def: CardDef }>> {
  defs ??= Object.fromEntries(PORTRAIT_IDS.map((id) => [id, defFor(id, currentCatalog().cards)])) as Record<
    PortraitId,
    { defId: string; def: CardDef }
  >;
  return defs;
}
