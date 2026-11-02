// The Collection screen's cosmetics (#1579, R1579000): what the account owns. The playmat, backdrop
// and card back are equipped from tiles (`AccountCosmeticTiles.tsx`), a card's own from the card in
// the Cards section (R1655000); a card with no pick of its own wears its kind's default (R1953001).
// It renders `GET /api/cosmetics` and decides nothing (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { cardNameOf, collectionTestid, cosmeticKindWords, ownedCosmetics } from "./collection.ts";

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
          {cardId === null ? null : ` · ${t("On {card}", { card: cardNameOf(cardId) })}`}
        </li>
      ))}
    </ul>
  );
}
