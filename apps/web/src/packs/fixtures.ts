// Fixture responses for the pack screen's tests and its component spec (#1308): what `GET /api/packs`
// and `POST /api/packs/open` answer, as the server's JSON. Neither uses the shipped numbers, so a test
// that finds one on screen has found the response's value and not a copy of the server's config.

import { GLOBAL_PACK_NAME, PACK_PRICE_JLORE, PACK_SLOTS } from "@jackioh/server-config";

import type { OpenedPackResponse, PacksResponse } from "../net/api.ts";

/** `GET /api/packs` for an account holding one global pack and two Core packs. */
export function packsInfo(): PacksResponse {
  return {
    price: PACK_PRICE_JLORE,
    slots: PACK_SLOTS,
    pity: { epic: 7, legendary: 13, mythic: 21 },
    thresholds: { epic: 10, legendary: 20, mythic: 40 },
    odds: {
      slot: { common: 0.5, rare: 0.32, epic: 0.123, legendary: 0.045, mythic: 0.012 },
      flooredSlot: { common: 0, rare: 0.82, epic: 0.123, legendary: 0.045, mythic: 0.012 },
      duplicateCosmetic: { common: 0.5, rare: 0.3, epic: 0.15, legendary: 0.04, mythic: 0.01 },
    },
    unopened: [
      { pack: GLOBAL_PACK_NAME, count: 1 },
      { pack: "Core", count: 2 },
    ],
  };
}

/** `POST /api/packs/open` for a Core pack with a Mythic, an Epic, a cosmetic, a bar and a card. */
export function openedPack(): OpenedPackResponse {
  return {
    pack: "Core",
    slots: [
      { rarity: "Mythic", cardId: "core-096", grant: { kind: "card" } },
      { rarity: "Epic", cardId: "core-018", grant: { kind: "card" } },
      {
        rarity: "Common",
        cardId: "core-004",
        grant: { kind: "cosmetic", cosmeticId: "full-art-gary-the-gambler", cosmeticRarity: "Epic" },
      },
      { rarity: "Rare", cardId: "core-007", grant: { kind: "bars", cosmeticRarity: "Rare" } },
      { rarity: "Common", cardId: "core-002", grant: { kind: "card" } },
    ],
    pity: { epic: 0, legendary: 14, mythic: 0 },
    unopened: 1,
  };
}
