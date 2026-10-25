// The Collection screen's cosmetics (#1579, R1579000): what the account equips and what it owns.
//
// The equip rows are the selects that lived in the settings (R1295000), moved with their store: they
// read and write the same `cosmetics` group (R633, `cards/cosmetics.ts`, `settings/groups.ts`) and
// make no request. R1655000: they are the defaults every card wears until it picks its own (the
// Cards section's card view); the playmat, backdrop and card back are preview tiles
// (`AccountCosmeticTiles.tsx`). The list renders `GET /api/cosmetics` and decides nothing (CLAUDE.md
// rule 7); `collection.ts` holds what it lists and the words it says.
//
// R1833000: a per-card cosmetic is owned on one card, so while ownership decides the defaults offer
// only what every account owns on every card, the kind's default and its starters; a card wears an
// earned one by its own pick, in the Cards section.

import { useContext, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import {
  DEFAULT_CARD_COSMETICS,
  OwnershipContext,
  cosmeticsOf,
  resolveCosmetic,
  useCosmetics,
  writeCosmetics,
  type CardSlotKind,
} from "../cards/cosmetics.ts";
import { t } from "../i18n/i18n.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { SelectRow } from "../settings/controls.tsx";
import { collectionTestid, cosmeticKindWords, ownedCosmetics } from "./collection.ts";

import "../settings/settings.css";

/** R1295000: a row per card cosmetic, in the registry's kinds' order, each list in the registry's order. */
const COSMETIC_ROWS: readonly { kind: CardSlotKind; label: string; hint: string }[] = [
  { kind: "foil", label: "Default foil", hint: "A sheen over the art of your own cards. Mythic and Radiant cards keep their own foil." },
  { kind: "finish", label: "Default finish", hint: "How your own cards are printed. Radiant cards keep their gold." },
  { kind: "border", label: "Default border", hint: "The trim around your own cards. Radiant cards keep their gold trim." },
];

/**
 * R1295000, R1655000: the defaults your own cards wear in a game, until a card picks its own in the
 * Cards section. R1833000: while ownership decides, each offers its kind's default and starters, and
 * every entry where it does not. A pick this client does not know (a newer client's), or one it does
 * not offer, shows as its kind's default, which is what it draws.
 */
export function CardCosmeticsControls(): ReactElement {
  const picks = useCosmetics();
  const owned = useContext(OwnershipContext);
  return (
    <>
      {COSMETIC_ROWS.map((row) => {
        const offered = cosmeticsOf(row.kind).filter((cosmetic) => owned === "every" || cosmetic.default || cosmetic.starter);
        const pick = resolveCosmetic(row.kind, picks[row.kind]);
        return (
          <SelectRow
            key={row.kind}
            label={t(row.label)}
            hint={t(row.hint)}
            testId={collectionTestid.equip(row.kind)}
            value={offered.some((cosmetic) => cosmetic.id === pick) ? pick : DEFAULT_CARD_COSMETICS[row.kind]}
            options={offered.map((cosmetic) => ({ value: cosmetic.id, label: cosmetic.name }))}
            onChange={(value) => {
              writeCosmetics({ [row.kind]: value });
            }}
          />
        );
      })}
    </>
  );
}

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
