// The Collection screen's cosmetics (#1579, R1579000): what the account equips and what it owns.
//
// The equip rows are the six selects that lived in the settings (R1295000, R1297000), moved with
// their store: they read and write the same `cosmetics` group (R633, `cards/cosmetics.ts`,
// `settings/groups.ts`) and make no request. The list renders `GET /api/cosmetics` and decides
// nothing (CLAUDE.md rule 7); `collection.ts` holds what it lists and the words it says.

import type { ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import {
  cosmeticsOf,
  resolveCosmetic,
  useCosmetics,
  writeCosmetics,
  type CardSlotKind,
  type DeckSlotKind,
} from "../cards/cosmetics.ts";
import { t } from "../i18n/i18n.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { SelectRow } from "../settings/controls.tsx";
import { collectionTestid, cosmeticKindWords, ownedCosmetics } from "./collection.ts";

import "../settings/settings.css";

/** R1295000: a row per card cosmetic, in the registry's kinds' order, each list in the registry's order. */
const COSMETIC_ROWS: readonly { kind: CardSlotKind; label: string; hint: string }[] = [
  { kind: "foil", label: "Foil", hint: "A sheen over the art of your own cards. Mythic and Radiant cards keep their own foil." },
  { kind: "finish", label: "Finish", hint: "How your own cards are printed. Radiant cards keep their gold." },
  { kind: "border", label: "Border", hint: "The trim around your own cards. Radiant cards keep their gold trim." },
];

/**
 * R1295000: the card cosmetics your own cards wear in a game, every one unlocked. A pick this
 * client does not know (a newer client's) shows as its kind's default, which is what it draws.
 */
export function CardCosmeticsControls(): ReactElement {
  const picks = useCosmetics();
  return (
    <>
      {COSMETIC_ROWS.map((row) => (
        <SelectRow
          key={row.kind}
          label={t(row.label)}
          hint={t(row.hint)}
          testId={collectionTestid.equip(row.kind)}
          value={resolveCosmetic(row.kind, picks[row.kind])}
          options={cosmeticsOf(row.kind).map((cosmetic) => ({ value: cosmetic.id, label: cosmetic.name }))}
          onChange={(value) => {
            writeCosmetics({ [row.kind]: value });
          }}
        />
      ))}
    </>
  );
}

/** R1297000: a row per deck slot, in the registry's order. */
const DECK_COSMETIC_ROWS: readonly { kind: DeckSlotKind; label: string; hint: string }[] = [
  { kind: "table", label: "Playmat", hint: "The playmat on your side. A deck can pick its own." },
  { kind: "field", label: "Backdrop", hint: "The backdrop behind your side. Reduce motion holds it still." },
  { kind: "cardBack", label: "Card back", hint: "The back of your own face-down cards and your deck." },
];

/**
 * R1297000: the playmat, backdrop and card back your own side wears in a game, every one unlocked.
 * A deck's own pick, set in the deckbuilder, beats these.
 */
export function DeckCosmeticsControls(): ReactElement {
  const picks = useCosmetics();
  return (
    <>
      {DECK_COSMETIC_ROWS.map((row) => (
        <SelectRow
          key={row.kind}
          label={t(row.label)}
          hint={t(row.hint)}
          testId={collectionTestid.equip(row.kind)}
          value={resolveCosmetic(row.kind, picks[row.kind])}
          options={cosmeticsOf(row.kind).map((cosmetic) => ({ value: cosmetic.id, label: cosmetic.name }))}
          onChange={(value) => {
            writeCosmetics({ [row.kind]: value });
          }}
        />
      ))}
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
