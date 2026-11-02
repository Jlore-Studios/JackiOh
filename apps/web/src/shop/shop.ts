// The Jlore shop's model (#1309, SPEC §14.1, §14.5, §14.6): its test ids and the pure helpers behind
// the shop, the token picker and the cosmetic packs.
// EVERY NUMBER SHOWN COMES FROM THE SERVER (CLAUDE.md rule 7): nothing here prices, counts or decides.
// The pickers' filters only mirror the server's `choice_of`, which re-refuses every redemption.
// Cosmetic rarity is its own scale and type (`CosmeticRarity`, never a card's `PrintedRarity`, R1333000):
// a token buys at its tier or below on the COSMETIC's rarity, and no token exceeds Epic (R1305003).

import { CATALOG } from "@jackioh/cards";
import { MAX_COPIES } from "@jackioh/engine/config";
import { GLOBAL_PACK_NAME } from "@jackioh/server-config";
import { SHIPPED_SETS, setShips, type CardDef } from "@jackioh/shared";

import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";
import type { CatalogSnapshot } from "@jackioh/validator";
import { t } from "../i18n/i18n.ts";
import type { CollectionResponse, CosmeticSlot, OwnedCosmeticsResponse } from "../net/api.ts";
import { matchesFilter, sortPool, type PoolFilter, type PoolSort } from "../game/deckbuilder/filters.ts";

/** One registry entry, structurally: what the filters read. */
export type RegistryCosmetic = {
  readonly id: string;
  readonly kind: string;
  readonly name: string;
  readonly rarity: CosmeticRarity;
  readonly default: boolean;
  readonly starter: boolean;
  readonly chase: boolean;
  /** R2755000: owned only by reaching Jlorious; the registry writes it only where it is true. */
  readonly jlorious?: boolean | undefined;
  readonly card?: string | undefined;
};

export const REGISTRY: readonly RegistryCosmetic[] = COSMETICS as unknown as readonly RegistryCosmetic[];

/** Cosmetic rarities lowest first, mirroring `CosmeticRarity::tier`. */
export const COSMETIC_TIER_ORDER: readonly CosmeticRarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];

export function cosmeticTier(rarity: CosmeticRarity): number {
  return COSMETIC_TIER_ORDER.indexOf(rarity);
}

/** The kinds a player wears on a card (`CosmeticKind::is_per_card`); the rest sit on the account. */
const PER_CARD_KINDS: readonly string[] = ["foil", "finish", "border", "fullArt"];

export function isPerCardKind(kind: string): boolean {
  return PER_CARD_KINDS.includes(kind);
}

export const shopTestid = {
  /** R1580000: the Shop screen, and its body, which scrolls while the page does not. */
  screen: "shop-screen",
  body: "shop-body",
  panel: "shop-panel",
  loading: "shop-loading",
  loadError: "shop-load-error",
  retry: "shop-retry",
  wallet: "shop-wallet",
  error: "shop-error",
  held: (pack: string) => `shop-held-${pack}`,
  buy: (pack: string) => `shop-buy-${pack}`,
  packObject: (pack: string) => `shop-pack-${pack}`,
  fly: (pack: string) => `shop-fly-${pack}`,
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
  cosmeticSpend: "token-cosmetic-spend",
  cardPick: (id: string) => `token-card-pick-${id}`,
  cardConfirm: "token-card-confirm",
  cardCancel: "token-card-cancel",
  cardDialog: "token-card-dialog",
  cardHint: "token-card-hint",
  cardNone: "token-card-none",
  packsPanel: "cosmetic-packs-panel",
  packsLoading: "cosmetic-packs-loading",
  packsLoadError: "cosmetic-packs-load-error",
  packsRetry: "cosmetic-packs-retry",
  packsError: "cosmetic-packs-error",
  unopened: "cosmetic-packs-unopened",
  // The panel draws no odds (R1665000 moved them to /purchases); the ids stay so the Shop route's
  // test can assert nothing wears them.
  odds: "cosmetic-packs-odds",
  flooredOdds: "cosmetic-packs-floored-odds",
  open: "cosmetic-packs-open",
  cosmeticPackObject: "cosmetic-packs-object",
  openingBars: "cosmetic-opening-bars",
  reveal: "cosmetic-reveal",
  equipLink: "cosmetic-packs-equip",
  slot: (index: number) => `cosmetic-slot-${String(index)}`,
  equip: (index: number) => `cosmetic-slot-${String(index)}-equip`,
  shardBar: "shard-bar",
  shardBurst: "shard-burst",
  shardReveal: "shard-reveal",
  shardChoice: "shard-choice",
  shardEmpty: "shard-empty",
  chasePick: (id: string) => `shard-pick-${id}`,
  chaseCards: (id: string) => `shard-cards-${id}`,
  chasePreview: (id: string) => `shard-preview-${id}`,
} as const;

/** How long a bought pack's copy takes to fly into its kind's unopened count; `shop.css` reads it as `--shop-fly-ms`. */
export const SHOP_FLY_MS = 600;

export function shopPacks(): string[] {
  return [GLOBAL_PACK_NAME, ...SHIPPED_SETS];
}

export function shortfallOf(price: number, jlore: number): number {
  return Math.max(0, price - jlore);
}

export function quantitiesOf(collection: CollectionResponse): Map<string, number> {
  return new Map(collection.entries.map((entry) => [entry.cardId, entry.quantity]));
}

/** Whether the account owns `MAX_COPIES` of a card: no token unlocks it again. */
export function ownsMax(cardId: string, quantities: Map<string, number>): boolean {
  return (quantities.get(cardId) ?? 0) >= MAX_COPIES;
}

/**
 * The cards a card token unlocks: not a token, of a set that ships, and owned below `MAX_COPIES` (the
 * server's `redeem_card_token`).
 */
export function unownedCards(collection: CollectionResponse, defs: readonly CardDef[] = Object.values(CATALOG)): CardDef[] {
  const quantities = quantitiesOf(collection);
  return defs.filter((def) => !def.token && setShips(def.set) && !ownsMax(def.id, quantities));
}

/**
 * The pool the card-token picker browses: the ids of `unowned` that the catalog knows, filtered and
 * sorted as the deck builder sorts. SPEC §14.5 names exactly `unownedCards`: `matchesFilter` never reads
 * `ownedOnly`, so the hidden owned toggle cannot affect it.
 */
export function tokenPoolIds(
  unowned: readonly CardDef[],
  catalog: CatalogSnapshot,
  filter: PoolFilter,
  sort: PoolSort,
): string[] {
  const byId = new Map(unowned.map((def) => [def.id, def]));
  const kept = [...byId.keys()].filter((id) => {
    const def = catalog.cards[id];
    return def !== undefined && matchesFilter(def, filter);
  });
  return sortPool(kept, catalog, sort);
}

/**
 * The owned cards a per-card cosmetic fits on (the server's `choice_of`): a full art its own card alone
 * (R1313000), any other per-card kind every card owned at `MAX_COPIES`; null for an account cosmetic.
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
 * The cosmetics a full Chase Shard bar buys: a chase, Legendary or Mythic (R1333000) but never the
 * Jlorious card back, which only reaching Jlorious grants (R2755000), the account does not own, a
 * per-card one for a card it owns (the server's `buy_chase`). `registry` defaults to the shipped
 * one; a test passes its own.
 */
export function chaseCandidates(
  owned: OwnedCosmeticsResponse,
  quantities: Map<string, number>,
  registry: readonly RegistryCosmetic[] = REGISTRY,
): RegistryCosmetic[] {
  return registry.filter((cosmetic) => {
    if (cosmetic.default || cosmetic.starter) return false;
    if (cosmetic.jlorious === true || cosmeticTier(cosmetic.rarity) < cosmeticTier("Legendary")) return false;
    const fits = fittingCards(cosmetic, quantities);
    if (fits === null) return !owned.account.includes(cosmetic.id);
    return fits.some((cardId) => !ownedThere(cosmetic.id, cardId, owned));
  });
}

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

/**
 * What a cosmetic pack slot became, as the server decided it: a new cosmetic by name, else the card
 * packs' duplicate sentence (R1306001 spends a slot into both token bars), saying `points`, the slot's
 * `slotPoints`, which it added to each bar (R1953000).
 */
export function cosmeticSlotOutcome(slot: CosmeticSlot, points: number): string {
  switch (slot.grant.kind) {
    case "cosmetic":
      return t("New cosmetic: {name}", { name: cosmeticName(slot.grant.cosmeticId) });
    case "bars":
      return t("Duplicate: +{card} card token bar, +{cosmetic} cosmetic token bar", { card: points, cosmetic: points });
  }
}

export function cosmeticName(id: string): string {
  return REGISTRY.find((cosmetic) => cosmetic.id === id)?.name ?? id;
}
