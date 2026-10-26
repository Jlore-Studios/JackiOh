// The Collection screen's cosmetics (#1579, R1579000): what the account owns. The playmat, backdrop
// and card back are equipped from preview tiles (`AccountCosmeticTiles.tsx`), and a card's foil,
// finish, border and full art from the card itself, in the Cards section (R1655000). R1953001: the
// selects of the defaults every card wore until it picked its own are gone, so a card with no pick of
// its own wears its kind's default. The list renders `GET /api/cosmetics` and decides nothing
// (CLAUDE.md rule 7); `collection.ts` holds what it lists and the words it says.

import type { ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { t } from "../i18n/i18n.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { collectionTestid, cosmeticKindWords, ownedCosmetics } from "./collection.ts";

/** R1579000: every cosmetic the account owns but the defaults, a per-card one on its card. */
export default function OwnedCosmetics({ owned }: { owned: OwnedCosmeticsResponse }): ReactElement {
  return (
    <ul className="collection-list">
      {ownedCosmetics(owned).map(({ cosmetic, cardId }) => (
        <li
          key={`${cosmetic.id}-${cardId ?? ""}`}
          data-testid={collectionTestid.ownedCosmetic(cosmetic.id, cardId)}
          data-rarity={cosmetic.rarity}
        >
          {t(cosmetic.name)} · {cosmeticKindWords(cosmetic.kind)} · {t(cosmetic.rarity)}
          {cardId === null ? null : ` · ${t("On {card}", { card: CATALOG[cardId]?.name ?? cardId })}`}
        </li>
      ))}
    </ul>
  );
}
