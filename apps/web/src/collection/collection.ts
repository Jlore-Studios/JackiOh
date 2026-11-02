// The collection screen's model (#1307, SPEC §14): its test ids and the pure helpers behind its
// panels, a card's strip (R1655000) and the account's tiles. EVERY NUMBER SHOWN COMES FROM A RESPONSE,
// `serverConfig.ts` OR THE SHIPPED CATALOG (CLAUDE.md rule 7): a set's total counts its collectible
// cards (§14: the non-token cards of a set that ships), and ownership is the server's row.

import { CATALOG } from "@jackioh/cards";
import { SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
import { SHIPPED_SETS, newestShippedSet, setShips, type CardDef, type SetName } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { CARD_SLOT_KINDS, cosmeticsOf, type Cosmetic, type CosmeticId, type DeckSlotKind } from "../cards/cosmetics.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { CollectionResponse, OwnedCosmeticsResponse, TrackRewards } from "../net/api.ts";
import { packName } from "../packs/packs.ts";
import { quantitiesOf } from "../shop/shop.ts";
import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";

/** The panels that read for themselves; the quests panel owns its own read. */
export type CollectionPart = "cards" | "wallet" | "tracks" | "cosmetics";

/** R1579000: the hub's sections that open in place, by `?section=`. */
export const COLLECTION_SECTIONS = ["cards", "cosmetics"] as const;
export type CollectionSection = (typeof COLLECTION_SECTIONS)[number];
export type CollectionDoor = "decks" | "cards" | "cosmetics" | "builder";

/** The section `?section=` names, or null for the hub: an unknown name opens the hub. */
export function collectionSectionOf(search: string): CollectionSection | null {
  const named = new URLSearchParams(search).get("section");
  return COLLECTION_SECTIONS.find((section) => section === named) ?? null;
}

export const collectionTestid = {
  screen: "collection-screen",
  jlore: "collection-jlore",
  cosmeticPacks: "collection-cosmetic-packs",
  noPacks: "collection-no-packs",
  loading: (part: CollectionPart) => `collection-loading-${part}`,
  error: (part: CollectionPart) => `collection-error-${part}`,
  /** #1691: Try again beside a panel's failed read. */
  retry: (part: CollectionPart) => `collection-retry-${part}`,
  set: (set: string) => `collection-set-${set}`,
  setToggle: (set: string) => `collection-set-toggle-${set}`,
  completion: (set: string) => `collection-completion-${set}`,
  card: (id: string) => `collection-card-${id}`,
  packs: (pack: string) => `collection-packs-${pack}`,
  track: (set: string) => `collection-track-${set}`,
  trackLevel: (set: string) => `collection-track-level-${set}`,
  trackXp: (set: string) => `collection-track-xp-${set}`,
  trackRewards: (set: string) => `collection-track-rewards-${set}`,
  trackLock: (set: string) => `collection-track-lock-${set}`,
  body: "collection-body",
  door: (id: CollectionDoor) => `collection-door-${id}`,
  // R1655000: a card in context (CardCosmeticsView) and the account's tiles (AccountCosmeticTiles).
  cardView: "collection-card-view",
  cardViewFace: "collection-card-view-face",
  cardViewPreview: "collection-card-view-preview",
  cardViewText: "collection-card-view-text",
  cardViewClose: "collection-card-view-close",
  strip: (kind: string) => `collection-strip-${kind}`,
  stripItem: (id: string) => `collection-strip-item-${id}`,
  stripDefault: (kind: string) => `collection-strip-default-${kind}`,
  tiles: (kind: string) => `collection-tiles-${kind}`,
  tile: (id: string) => `collection-tile-${id}`,
  // R1953001: a held cosmetic token offered on a locked cosmetic (TokenOffer), and its refusal.
  tokenOffer: (id: string) => `collection-token-offer-${id}`,
  tokenOfferError: (id: string) => `collection-token-offer-error-${id}`,
  // R2480000: the question the offer asks before it spends, and its two answers.
  tokenOfferDialog: (id: string) => `collection-token-offer-dialog-${id}`,
  tokenOfferConfirm: (id: string) => `collection-token-offer-confirm-${id}`,
  tokenOfferCancel: (id: string) => `collection-token-offer-cancel-${id}`,
  ownedCosmetic: (id: string, cardId: string | null) =>
    cardId === null ? `collection-owned-${id}` : `collection-owned-${id}-${cardId}`,
} as const;

export const COLLECTED_DEFS: readonly CardDef[] = Object.values(CATALOG).filter((def) => !def.token && setShips(def.set));

export type SetCompletion = {
  set: SetName;
  owned: number;
  total: number;
  cards: { def: CardDef; owned: boolean }[];
};

/** One entry per shipped set, in catalog order; a card is owned when its row's quantity is above 0. */
export function setCompletions(collection: CollectionResponse, defs: readonly CardDef[] = COLLECTED_DEFS): SetCompletion[] {
  const quantities = quantitiesOf(collection);
  return SHIPPED_SETS.map((set) => {
    const cards = defs
      .filter((def) => def.set === set && !def.token)
      .map((def) => ({ def, owned: (quantities.get(def.id) ?? 0) > 0 }));
    return { set, owned: cards.filter((card) => card.owned).length, total: cards.length, cards };
  });
}

/** What a level pays, the parts above 0 in a player's words. */
export function rewardWords(rewards: TrackRewards, setId: SetName): string {
  const parts: string[] = [];
  if (rewards.jlore > 0) parts.push(t("{jlore} Jlore", { jlore: formatNumber(rewards.jlore) }));
  if (rewards.setPacks > 0) {
    parts.push(t("{count} × {pack}", { count: formatNumber(rewards.setPacks), pack: packName(setId) }));
  }
  if (rewards.cardTokens > 0) parts.push(t("{count} × card token", { count: formatNumber(rewards.cardTokens) }));
  return parts.join(t(", "));
}

/** How a locked track unlocks (R1302003): the newest set's track reaching its unlock level, then one older set a game. */
export function lockWords(): string {
  return t(
    "Locked: unlocks once the {newest} track reaches level {level}. Older sets then unlock one per game, the most recent first.",
    { newest: t(newestShippedSet()), level: formatNumber(SET_TRACK_UNLOCK_LEVEL) },
  );
}

/**
 * R1579000: the cosmetics the account owns, in the registry's order: each starter (every account's,
 * §14.2) once, no default, and the server's rest (R1313000), a per-card one once per card it sits on,
 * an account one (a portrait, R1827000) on no card. An id this client does not know is not shown.
 */
export function ownedCosmetics(owned: OwnedCosmeticsResponse): { cosmetic: Cosmetic; cardId: string | null }[] {
  const out: { cosmetic: Cosmetic; cardId: string | null }[] = [];
  for (const cosmetic of COSMETICS) {
    if (cosmetic.default) continue;
    if (cosmetic.starter || owned.account.includes(cosmetic.id)) {
      out.push({ cosmetic, cardId: null });
      continue;
    }
    for (const row of owned.cards) {
      if (row.cosmeticId === cosmetic.id) out.push({ cosmetic, cardId: row.cardId });
    }
  }
  return out;
}

/** R711000: a catalog card's name as the reader reads it (`readerName`), else its id. */
export function cardNameOf(cardId: string): string {
  const def = CATALOG[cardId];
  return def === undefined ? cardId : readerName(def);
}

export function cosmeticKindWords(kind: Cosmetic["kind"]): string {
  switch (kind) {
    case "foil":
      return t("Foil");
    case "finish":
      return t("Finish");
    case "border":
      return t("Border");
    case "table":
      return t("Playmat");
    case "field":
      return t("Backdrop");
    case "cardBack":
      return t("Card back");
    case "fullArt":
      return t("Full art");
    case "portrait":
      return t("Portrait");
  }
}

/** R1655000: the kinds a card's strip offers, in the registry's order, its full art last. */
export const STRIP_KINDS = [...CARD_SLOT_KINDS, "fullArt"] as const;
export type StripKind = (typeof STRIP_KINDS)[number];

/** R1655000: the face without a full art, the default of that kind and a pick of its own, which no registry entry names. */
export const NO_FULL_ART_ID = "no-full-art";

export type StripItem = {
  kind: StripKind;
  /** The registry's id; `NO_FULL_ART_ID` for the face with no full art. */
  id: string;
  /** In the reader's language. */
  name: string;
  rarity: CosmeticRarity;
  /** Whether the viewer may equip it on this card (`useCanEquip`): a locked one shows and equips nothing. */
  owned: boolean;
};

/** One kind's items, its default first. */
export type StripGroup = { kind: StripKind; items: StripItem[] };

/**
 * R1655000: what a card may wear, by kind: the registry's foils, finishes and borders (the default
 * first), and the face with no full art then the card's own (R2655000: its Epic, Legendary and
 * Mythic), the kind absent where it has none.
 */
export function cardStrip(defId: string, canEquip: (defId: string, cosmeticId: CosmeticId) => boolean): StripGroup[] {
  const groups: StripGroup[] = CARD_SLOT_KINDS.map((kind) => ({
    kind,
    items: cosmeticsOf(kind).map((cosmetic) => ({
      kind,
      id: cosmetic.id,
      name: t(cosmetic.name),
      rarity: cosmetic.rarity,
      owned: canEquip(defId, cosmetic.id),
    })),
  }));
  const fullArts: StripItem[] = COSMETICS.flatMap((cosmetic): StripItem[] =>
    cosmetic.kind === "fullArt" && cosmetic.card === defId
      ? [{ kind: "fullArt", id: cosmetic.id, name: t(cosmetic.name), rarity: cosmetic.rarity, owned: canEquip(defId, cosmetic.id) }]
      : [],
  );
  if (fullArts.length > 0) {
    groups.push({
      kind: "fullArt",
      items: [{ kind: "fullArt", id: NO_FULL_ART_ID, name: t("No full art"), rarity: "Common", owned: true }, ...fullArts],
    });
  }
  return groups;
}

export type AccountTile = { cosmetic: Cosmetic; owned: boolean };

/**
 * R1579000, R1655000: `kind`'s entries in the registry's order, owned as `owns` says (R1833000:
 * `useOwnsAccountCosmetic`'s; a default and a starter are every account's, §14.2).
 */
export function accountTiles(kind: DeckSlotKind, owns: (id: string) => boolean): AccountTile[] {
  return cosmeticsOf(kind).map((cosmetic) => ({ cosmetic, owned: owns(cosmetic.id) }));
}
