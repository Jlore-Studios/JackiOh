// The Jlore shop's model (#1309, SPEC §14.1, §14.5, §14.6): its test ids and the pure helpers
// behind the shop, the token picker and the cosmetic packs, all of them plain data in and out.
//
// EVERY NUMBER SHOWN COMES FROM THE SERVER (CLAUDE.md rule 7). The wallet, the pack price, both
// bars and their sizes, the cycle's next step and the shard bar all arrive in the responses
// (`net/api.ts`); nothing here prices, counts or decides. The pickers' filters are browsing UX
// only, mirroring the server's `choice_of`: a filter gap cannot grant anything, because the
// server re-refuses every redemption.
//
// Cosmetic rarity is its own scale (Common to Mythic), so a token buys at its tier or below on
// the COSMETIC's rarity, not the card's: a Rare token reaches Common and Rare, an Epic token adds
// Epic, and Legendary and Mythic are unreachable, since no token exceeds Epic (R1305003). It is
// its own type, `CosmeticRarity`, never a card's `PrintedRarity`, so the two cannot mix (R1333000).

import { CATALOG } from "@jackioh/cards";
import { MAX_COPIES } from "@jackioh/engine/config";
import {
  CARD_TOKEN_BAR_POINTS_PER_DUPLICATE,
  COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
  GLOBAL_PACK_NAME,
} from "@jackioh/server-config";
import { SHIPPED_SETS, setShips, type CardDef } from "@jackioh/shared";

import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";
import { t } from "../i18n/i18n.ts";
import type { CollectionResponse, CosmeticSlot, CosmeticTokenTier, OwnedCosmeticsResponse } from "../net/api.ts";

/** One registry entry, structurally: what the filters read. */
export type RegistryCosmetic = {
  readonly id: string;
  readonly kind: string;
  readonly name: string;
  readonly rarity: CosmeticRarity;
  readonly default: boolean;
  readonly starter: boolean;
  readonly chase: boolean;
  readonly card?: string | undefined;
};

/** The shipped registry as the filters read it. */
export const REGISTRY: readonly RegistryCosmetic[] = COSMETICS as unknown as readonly RegistryCosmetic[];

/** Cosmetic rarities lowest first, mirroring `CosmeticRarity::tier`. */
export const COSMETIC_TIER_ORDER: readonly CosmeticRarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];

/** A cosmetic rarity's place on its own scale, commonest first. */
export function cosmeticTier(rarity: CosmeticRarity): number {
  return COSMETIC_TIER_ORDER.indexOf(rarity);
}

/** The kinds a player wears on a card (`CosmeticKind::is_per_card`); the rest sit on the account. */
const PER_CARD_KINDS: readonly string[] = ["foil", "finish", "border", "fullArt"];

/** Whether a cosmetic of this kind goes on a card rather than the account. */
export function isPerCardKind(kind: string): boolean {
  return PER_CARD_KINDS.includes(kind);
}

export const shopTestid = {
  panel: "shop-panel",
  loading: "shop-loading",
  loadError: "shop-load-error",
  retry: "shop-retry",
  wallet: "shop-wallet",
  error: "shop-error",
  held: (pack: string) => `shop-held-${pack}`,
  buy: (pack: string) => `shop-buy-${pack}`,
  shortfall: (pack: string) => `shop-shortfall-${pack}`,
  tokenPanel: "token-panel",
  tokenLoading: "token-loading",
  tokenLoadError: "token-load-error",
  tokenRetry: "token-retry",
  tokenError: "token-error",
  cardBar: "token-card-bar",
  cosmeticBar: "token-cosmetic-bar",
  nextStep: "token-next-step",
  heldCard: "token-held-card",
  heldRare: "token-held-rare",
  heldEpic: "token-held-epic",
  heldPack: "token-held-pack",
  cardPick: (id: string) => `token-card-pick-${id}`,
  cardNone: "token-card-none",
  tierRare: "token-tier-rare",
  tierEpic: "token-tier-epic",
  cosmeticPick: (id: string) => `token-cosmetic-pick-${id}`,
  cosmeticCards: (id: string) => `token-cosmetic-cards-${id}`,
  cosmeticPreview: (id: string) => `token-cosmetic-preview-${id}`,
  cosmeticNone: "token-cosmetic-none",
  packsPanel: "cosmetic-packs-panel",
  packsLoading: "cosmetic-packs-loading",
  packsLoadError: "cosmetic-packs-load-error",
  packsRetry: "cosmetic-packs-retry",
  packsError: "cosmetic-packs-error",
  unopened: "cosmetic-packs-unopened",
  odds: "cosmetic-packs-odds",
  flooredOdds: "cosmetic-packs-floored-odds",
  open: "cosmetic-packs-open",
  reveal: "cosmetic-reveal",
  equipLink: "cosmetic-packs-equip",
  slot: (index: number) => `cosmetic-slot-${String(index)}`,
  shardBar: "shard-bar",
  shardChoice: "shard-choice",
  shardEmpty: "shard-empty",
  chasePick: (id: string) => `shard-pick-${id}`,
  chaseCards: (id: string) => `shard-cards-${id}`,
} as const;

/** The packs the shop sells: the global pack first, then each set that ships. */
export function shopPacks(): string[] {
  return [GLOBAL_PACK_NAME, ...SHIPPED_SETS];
}

/** What a short wallet still needs: 0 when it covers the price. */
export function shortfallOf(price: number, jlore: number): number {
  return Math.max(0, price - jlore);
}

/** Each card's owned copies, by catalog id. */
export function quantitiesOf(collection: CollectionResponse): Map<string, number> {
  return new Map(collection.entries.map((entry) => [entry.cardId, entry.quantity]));
}

/** Whether the account owns `MAX_COPIES` of a card: no token unlocks it again. */
export function ownsMax(cardId: string, quantities: Map<string, number>): boolean {
  return (quantities.get(cardId) ?? 0) >= MAX_COPIES;
}

/**
 * The cards a card token unlocks: not a token, of a set that ships, and owned below
 * `MAX_COPIES` (the server's `redeem_card_token`). `defs` defaults to the catalog; a test passes
 * its own.
 */
export function unownedCards(collection: CollectionResponse, defs: readonly CardDef[] = Object.values(CATALOG)): CardDef[] {
  const quantities = quantitiesOf(collection);
  return defs.filter((def) => !def.token && setShips(def.set) && !ownsMax(def.id, quantities));
}

/**
 * The owned cards a per-card cosmetic fits on (the server's `choice_of`): a full art names its
 * own card alone (R1313000); any other per-card kind fits every card owned at `MAX_COPIES`.
 * Null for an account cosmetic, which names no card.
 */
export function fittingCards(cosmetic: RegistryCosmetic, quantities: Map<string, number>): string[] | null {
  if (!isPerCardKind(cosmetic.kind)) return null;
  if (cosmetic.card !== undefined) return ownsMax(cosmetic.card, quantities) ? [cosmetic.card] : [];
  const fits: string[] = [];
  for (const [cardId, quantity] of quantities) {
    if (quantity >= MAX_COPIES && CATALOG[cardId] !== undefined) fits.push(cardId);
  }
  return fits;
}

/** Whether the account already owns a cosmetic there: on the card, or on the account. */
export function ownedThere(cosmeticId: string, cardId: string | null, owned: OwnedCosmeticsResponse): boolean {
  return cardId === null
    ? owned.account.includes(cosmeticId)
    : owned.cards.some((held) => held.cardId === cardId && held.cosmeticId === cosmeticId);
}

/**
 * The cosmetics a cosmetic token buys (R1305003): at its tier or below on the cosmetic's own
 * scale, never a chase, never a default or a starter (which §14.2 gives every account), never
 * already owned where it would go, and a per-card kind only where an owned card fits it.
 * Legendary and Mythic never appear: no token exceeds Epic. `registry` defaults to the shipped
 * one; a test passes its own.
 */
export function redeemableCosmetics(
  token: CosmeticTokenTier,
  owned: OwnedCosmeticsResponse,
  quantities: Map<string, number>,
  registry: readonly RegistryCosmetic[] = REGISTRY,
): RegistryCosmetic[] {
  const cap = cosmeticTier(token);
  return registry.filter((cosmetic) => {
    if (cosmetic.chase || cosmetic.default || cosmetic.starter) return false;
    if (cosmeticTier(cosmetic.rarity) > cap) return false;
    const fits = fittingCards(cosmetic, quantities);
    if (fits === null) return !owned.account.includes(cosmetic.id);
    return fits.some((cardId) => !ownedThere(cosmetic.id, cardId, owned));
  });
}

/**
 * The cosmetics a full Chase Shard bar buys: a chase, Legendary or Mythic (R1333000), the account
 * does not own, a per-card one for a card it owns (the server's `buy_chase`). `registry`
 * defaults to the shipped one; a test passes its own.
 */
export function chaseCandidates(
  owned: OwnedCosmeticsResponse,
  quantities: Map<string, number>,
  registry: readonly RegistryCosmetic[] = REGISTRY,
): RegistryCosmetic[] {
  return registry.filter((cosmetic) => {
    if (cosmetic.default || cosmetic.starter) return false;
    if (cosmeticTier(cosmetic.rarity) < cosmeticTier("Legendary")) return false;
    const fits = fittingCards(cosmetic, quantities);
    if (fits === null) return !owned.account.includes(cosmetic.id);
    return fits.some((cardId) => !ownedThere(cosmetic.id, cardId, owned));
  });
}

/** The cycle's next step in a player's words. */
export function nextStepWords(step: "rare" | "epic" | "cosmeticPack"): string {
  switch (step) {
    case "rare":
      return t("Rare token");
    case "epic":
      return t("Epic token");
    case "cosmeticPack":
      return t("Cosmetic pack");
  }
}

/** A cosmetic token's tier in a player's words. */
export function tokenTierWords(token: CosmeticTokenTier): string {
  return token === "Rare" ? t("Rare token") : t("Epic token");
}

/**
 * What a cosmetic pack slot became, as the server decided it: a new cosmetic by name, else the
 * same duplicate sentence the card packs use (R1306001 spends a slot into both token bars).
 */
export function cosmeticSlotOutcome(slot: CosmeticSlot): string {
  switch (slot.grant.kind) {
    case "cosmetic":
      return t("New cosmetic: {name}", { name: cosmeticName(slot.grant.cosmeticId) });
    case "bars":
      return t("Duplicate: +{card} card token bar, +{cosmetic} cosmetic token bar", {
        card: CARD_TOKEN_BAR_POINTS_PER_DUPLICATE,
        cosmetic: COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
      });
  }
}

/** A cosmetic's name from the registry, else its id. */
export function cosmeticName(id: string): string {
  return REGISTRY.find((cosmetic) => cosmetic.id === id)?.name ?? id;
}
