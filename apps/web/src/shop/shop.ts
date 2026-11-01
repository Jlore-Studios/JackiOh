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
// Epic, a Legendary token adds Legendary, a Legendary chase included, and Mythic is unreachable,
// since no token exceeds Legendary (R1305003, R2728000). It is its own type, `CosmeticRarity`,
// never a card's `PrintedRarity`, so the two cannot mix (R1333000).

import { CATALOG } from "@jackioh/cards";
import { MAX_COPIES } from "@jackioh/engine/config";
import { GLOBAL_PACK_NAME } from "@jackioh/server-config";
import { SHIPPED_SETS, setShips, type CardDef } from "@jackioh/shared";

import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";
import type { CatalogSnapshot } from "@jackioh/validator";
import { t } from "../i18n/i18n.ts";
import type { CollectionResponse, CosmeticSlot, CosmeticTokenStep, OwnedCosmeticsResponse } from "../net/api.ts";
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
  heldLegendary: "token-held-legendary",
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
/**
 * The pool the card-token picker browses: the ids of `unowned` that the catalog knows, filtered
 * and sorted as the deck builder sorts (SPEC §14.5 names exactly `unownedCards`; the browse is UX
 * only, and `matchesFilter` never reads `ownedOnly`, so the hidden owned toggle cannot affect it).
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
export function nextStepWords(step: CosmeticTokenStep): string {
  switch (step) {
    case "rare":
      return t("Rare token");
    case "epic":
      return t("Epic token");
    case "legendary":
      return t("Legendary token");
    case "cosmeticPack":
      return t("Cosmetic pack");
  }
}

/**
 * What a cosmetic pack slot became, as the server decided it: a new cosmetic by name, else the
 * same duplicate sentence the card packs use (R1306001 spends a slot into both token bars), saying
 * `points`, the opening's `slotPoints` for the slot, which it added to each bar (R1953000).
 */
export function cosmeticSlotOutcome(slot: CosmeticSlot, points: number): string {
  switch (slot.grant.kind) {
    case "cosmetic":
      return t("New cosmetic: {name}", { name: cosmeticName(slot.grant.cosmeticId) });
    case "bars":
      return t("Duplicate: +{card} card token bar, +{cosmetic} cosmetic token bar", { card: points, cosmetic: points });
  }
}

/** A cosmetic's name from the registry, else its id. */
export function cosmeticName(id: string): string {
  return REGISTRY.find((cosmetic) => cosmetic.id === id)?.name ?? id;
}
