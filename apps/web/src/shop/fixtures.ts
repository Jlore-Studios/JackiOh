// Fixture responses for the shop's tests (#1309): what `GET /api/wallet`, `GET /api/tokens`,
// `GET /api/cosmetic-packs`, `GET /api/collection` and `GET /api/cosmetics` answer, as the
// server's JSON. No number is a shipped one where a test reads it, so a test that finds one on
// screen has found the response's value and not a copy of the server's config.

import type {
  CollectionResponse,
  CosmeticPacksResponse,
  OpenedCosmeticPackResponse,
  OwnedCosmeticsResponse,
  TokenBarsResponse,
  WalletResponse,
} from "../net/api.ts";

/** `GET /api/wallet` for an account holding 37 Jlore: short of every pack's price. */
export function walletInfo(): WalletResponse {
  return { jlore: 37 };
}

/**
 * `GET /api/tokens`: the card bar one point from full, the cosmetic bar at 30 of 150, the
 * cycle's next step Epic, one card token, one Rare token, no Epic token and one cosmetic pack.
 */
export function tokenBars(): TokenBarsResponse {
  return {
    cardBar: { points: 149, size: 150 },
    cosmeticBar: { points: 30, size: 150 },
    nextStep: "epic",
    tokens: { card: 1, cosmeticRare: 1, cosmeticEpic: 0, cosmeticPack: 1 },
  };
}

/** A cosmetic pack's odds, a slot's and the guaranteed slot's, none of them the shipped tables'. */
function cosmeticPackOdds(): CosmeticPacksResponse["odds"] {
  return {
    slot: { common: 0.5, rare: 0.3, epic: 0.15, legendary: 0.04, mythic: 0.01 },
    flooredSlot: { common: 0, rare: 0, epic: 0.9, legendary: 0.08, mythic: 0.02 },
  };
}

/** `GET /api/cosmetic-packs` with the shard bar one pack short of full and two packs unopened. */
export function cosmeticPacksInfo(): CosmeticPacksResponse {
  return { shardBar: { points: 19, size: 20 }, unopened: 2, odds: cosmeticPackOdds() };
}

/** `GET /api/cosmetic-packs` with a full shard bar and one pack unopened. */
export function cosmeticPacksFull(): CosmeticPacksResponse {
  return { shardBar: { points: 20, size: 20 }, unopened: 1, odds: cosmeticPackOdds() };
}

/**
 * `GET /api/collection`: `core-001` and `core-004` owned at `MAX_COPIES` (1), `core-002` absent.
 * `core-051-1` (a token) is listed only so a test can prove the picker skips it.
 */
export function collectionOf(
  over: Partial<CollectionResponse> = {},
): CollectionResponse {
  return {
    catalogVersion: "test",
    entries: [
      { cardId: "core-001", quantity: 1 },
      { cardId: "core-004", quantity: 1 },
      { cardId: "core-051-1", quantity: 1 },
    ],
    ...over,
  };
}

/** `GET /api/cosmetics`: Gary's full art on `core-004`, and the `table-felt` playmat. */
export function ownedCosmeticsOf(
  over: Partial<OwnedCosmeticsResponse> = {},
): OwnedCosmeticsResponse {
  return {
    cards: [{ cardId: "core-004", cosmeticId: "full-art-gary-the-gambler" }],
    account: ["table-felt"],
    ...over,
  };
}

/**
 * `POST /api/cosmetic-packs/open`: an Epic full art for an owned card, an account cosmetic and a
 * spent slot, the shard bar one further on and one pack left.
 */
export function openedCosmeticPack(): OpenedCosmeticPackResponse {
  return {
    slots: [
      {
        rarity: "Epic",
        grant: {
          kind: "cosmetic",
          cosmeticId: "full-art-mr-vanilla",
          cosmeticRarity: "Epic",
          cardId: "core-008",
          cardGranted: true,
        },
      },
      {
        rarity: "Common",
        grant: {
          kind: "cosmetic",
          cosmeticId: "table-wood",
          cosmeticRarity: "Common",
          cardId: null,
          cardGranted: false,
        },
      },
      { rarity: "Rare", grant: { kind: "bars" } },
    ],
    shardBar: { points: 20, size: 20 },
    unopened: 1,
  };
}
