// R635's hero portraits on the client: which catalog card's art each portrait draws, in `CardArt`'s
// `oval` shape (issue §1 — the card's own art, manifest image first, procedural SVG otherwise).
//
// The roster in `packages/shared` names each portrait by its card's NAME, not its id — the issue's
// roster table says to look cards up by name, never by number — so this module resolves the name
// once, against the catalog the bundle ships (`@jackioh/cards/catalog.json`, the same import
// `routes/almanac.tsx` makes), and keeps the def for `CardArt`.

import catalogJson from "@jackioh/cards/catalog.json";

import type { CardDef, PortraitId } from "@jackioh/shared";
import { PORTRAIT_IDS, PORTRAITS } from "@jackioh/shared";

const catalog = catalogJson as unknown as Record<string, CardDef>;

function defFor(portrait: PortraitId): { defId: string; def: CardDef } {
  const cardName = PORTRAITS[portrait].cardName;
  for (const [defId, def] of Object.entries(catalog)) {
    if (def.name === cardName) return { defId, def };
  }
  throw new Error(`portrait ${portrait}: no catalog card named ${JSON.stringify(cardName)}`);
}

/** Every portrait's card, resolved once at module load. A missing card fails loudly, not blank. */
export const PORTRAIT_DEFS: Readonly<Record<PortraitId, { defId: string; def: CardDef }>> =
  Object.fromEntries(PORTRAIT_IDS.map((id) => [id, defFor(id)])) as Record<
    PortraitId,
    { defId: string; def: CardDef }
  >;
