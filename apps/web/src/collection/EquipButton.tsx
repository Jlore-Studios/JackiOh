// R1955000: a pack opening's reveal equips the cosmetic a slot just granted, writing what the
// Collection's pickers write with no request: the server settled the grant (CLAUDE.md rule 7). A
// portrait is a deck's pick (R641, R1827000), so it has no button. R2655000: a full art's Equip picks
// that tier on its card (`fullArtPickOf`). R2725000: a version's Equip wears it on its card, an augment's layers it
// on whichever version the card wears, each keeping the other (`writeCardVersion`). The wear path decides what is worn.
// R2949000: Signature has no Equip on a card that does not wear the Full art version and Animated, which it needs.

import type { ReactElement } from "react";

import {
  CARD_SLOT_KINDS,
  DECK_SLOT_KINDS,
  useCardPicks,
  useCosmetics,
  writeCardFullArt,
  writeCardPick,
  writeCardVersion,
  writeCosmetics,
  type CardCosmetics,
  type CardPicks,
  type CardSlotKind,
  type CosmeticId,
  type DeckSlotKind,
} from "../cards/cosmetics.ts";
import { fullArtIdOf, fullArtPickOf } from "../cards/fullArt.ts";
import { AUGMENT_SIGNATURE_ID, augmentPicksOf, augmentsOf, lookOf, signatureFits, versionPickOf } from "../cards/versions.ts";
import { t } from "../i18n/i18n.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";

export type EquipTarget =
  | { store: "card"; cardId: string; kind: CardSlotKind; id: CosmeticId }
  | { store: "fullArt"; cardId: string; id: CosmeticId }
  | { store: "version"; cardId: string; id: CosmeticId }
  | { store: "augment"; cardId: string; id: CosmeticId }
  | { store: "account"; kind: DeckSlotKind; id: CosmeticId };

/** What equipping `cosmeticId`, granted on `cardId` (null for an account one), writes: none for a portrait, an unknown id, or a per-card one with no card. */
export function equipTarget(cosmeticId: string, cardId: string | null): EquipTarget | null {
  const found = COSMETICS.find((cosmetic) => cosmetic.id === cosmeticId);
  if (found === undefined) return null;
  const slot = CARD_SLOT_KINDS.find((kind) => kind === found.kind);
  if (slot !== undefined) return cardId === null ? null : { store: "card", cardId, kind: slot, id: found.id };
  if (found.kind === "fullArt") return cardId === null ? null : { store: "fullArt", cardId, id: found.id };
  if (found.kind === "version" || found.kind === "augment") return cardId === null ? null : { store: found.kind, cardId, id: found.id };
  const account = DECK_SLOT_KINDS.find((kind) => kind === found.kind);
  return account === undefined ? null : { store: "account", kind: account, id: found.id };
}

function holds(target: EquipTarget, account: CardCosmetics, card: CardPicks): boolean {
  switch (target.store) {
    case "card":
      return card[target.kind] === target.id;
    case "fullArt":
      return fullArtIdOf(target.cardId, card.fullArt) === target.id;
    case "version":
      return card.fullArt !== undefined && lookOf(card.fullArt).version === target.id;
    case "augment":
      return augmentsOf(lookOf(card.fullArt, card.augments)).includes(target.id);
    case "account":
      return account[target.kind] === target.id;
  }
}

function equip(target: EquipTarget, card: CardPicks): void {
  switch (target.store) {
    case "card":
      writeCardPick(target.cardId, target.kind, target.id);
      return;
    case "fullArt":
      writeCardFullArt(target.cardId, fullArtPickOf(target.cardId, target.id));
      return;
    case "version":
      writeCardVersion(target.cardId, target.id, augmentPicksOf(card.fullArt, card.augments));
      return;
    case "augment": {
      const augments = augmentPicksOf(card.fullArt, card.augments);
      writeCardVersion(target.cardId, versionPickOf(card.fullArt), augments.includes(target.id) ? augments : [...augments, target.id]);
      return;
    }
    case "account":
      writeCosmetics({ [target.kind]: target.id });
      return;
  }
}

/** A revealed slot's Equip for the cosmetic it granted on `cardId`; nothing for one it cannot equip. */
export default function EquipButton({
  cosmeticId,
  cardId,
  testid,
}: {
  cosmeticId: string;
  cardId: string | null;
  testid: string;
}): ReactElement | null {
  const target = equipTarget(cosmeticId, cardId);
  const account = useCosmetics();
  // An account cosmetic sits on no card: "" is no card's id, so it reads no picks.
  const card = useCardPicks(cardId ?? "");
  if (target === null) return null;
  if (target.store === "augment" && target.id === AUGMENT_SIGNATURE_ID && !signatureFits(card.fullArt, card.augments)) return null;
  const equipped = holds(target, account, card);
  return (
    <button
      type="button"
      className="pack-slot__equip"
      data-testid={testid}
      disabled={equipped}
      onClick={() => {
        equip(target, card);
      }}
    >
      {equipped ? t("Equipped") : t("Equip")}
    </button>
  );
}
