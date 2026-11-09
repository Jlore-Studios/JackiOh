// The collection screen's model (#1307, SPEC §14): its test ids and the pure helpers behind its
// panels, a card's strip (R1655000) and the account's tiles. EVERY NUMBER SHOWN COMES FROM A RESPONSE,
// `serverConfig.ts` OR THE SHIPPED CATALOG (CLAUDE.md rule 7): a set's total counts its collectible
// cards (§14: the non-token cards of a set that ships), and ownership is the server's row.

import { CATALOG } from "@jackioh/cards";
import { SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
import { SHIPPED_SETS, newestShippedSet, setShips, type CardDef, type SetName } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import {
  CARD_SLOT_KINDS,
  cosmeticsOf,
  type CardPicks,
  type Cosmetic,
  type CosmeticId,
  type DeckSlotKind,
  type StarshipSlotKind,
} from "../cards/cosmetics.ts";
import {
  DEFAULT_VARIANT,
  FIRST_RANK,
  defaultLookOf,
  isNextLevel,
  ladderOf,
  levelOf,
  levelOfPick,
  levelsBelow,
  levelsOn,
  variantsOn,
  wornLevel,
} from "../cards/versions.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { CollectionResponse, TrackRewards } from "../net/api.ts";
import { packName } from "../packs/packs.ts";
import { quantitiesOf } from "../shop/shop.ts";
import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";

/** The panels that read for themselves; the quests panel owns its own read. R3335001: the Fleet track's is the tracks' read. */
export type CollectionPart = "cards" | "wallet" | "tracks" | "fleet";

/** R1579000: the hub's sections that open in place, by `?section=`. */
export const COLLECTION_SECTIONS = ["cards", "cosmetics"] as const;
export type CollectionSection = (typeof COLLECTION_SECTIONS)[number];
export type CollectionDoor = "decks" | "cards" | "cosmetics";

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
  /** R3349000: a card's "new" mark for a newly earned level, until the card is viewed. */
  newMark: (id: string) => `collection-new-${id}`,
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
  // R3335001: the Fleet track (FleetTrackPanel), its level, its bar's words, today's XP and its next rewards.
  fleet: "collection-fleet",
  fleetLevel: "collection-fleet-level",
  fleetXp: "collection-fleet-xp",
  fleetToday: "collection-fleet-today",
  fleetRewards: "collection-fleet-rewards",
  fleetReward: (id: string) => `collection-fleet-reward-${id}`,
  // R3335000: the Starship looks (StarshipLooks), the equipped one's preview, and a locked tile's words.
  starshipLooks: "collection-starship-looks",
  starshipEquipped: "collection-starship-equipped",
  cosmeticsTab: (tab: string) => `collection-cosmetics-tab-${tab}`,
  tileEarn: (id: string) => `collection-tile-earn-${id}`,
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
  // R3015007: each variant's ladder (VariantLadders) and its levels' rungs.
  ladder: (variant: string) => `collection-ladder-${variant}`,
  rung: (id: string) => `collection-rung-${id}`,
  tiles: (kind: string) => `collection-tiles-${kind}`,
  tile: (id: string) => `collection-tile-${id}`,
  // R1953001: a held cosmetic token offered on a locked cosmetic (TokenOffer), and its refusal.
  tokenOffer: (id: string) => `collection-token-offer-${id}`,
  tokenOfferError: (id: string) => `collection-token-offer-error-${id}`,
  // R2480000: the question the offer asks before it spends, and its two answers.
  tokenOfferDialog: (id: string) => `collection-token-offer-dialog-${id}`,
  tokenOfferConfirm: (id: string) => `collection-token-offer-confirm-${id}`,
  tokenOfferCancel: (id: string) => `collection-token-offer-cancel-${id}`,
  // R2730000: the fullscreen preview a held or inspected item opens, the only place a token is spent.
  preview: "collection-preview",
  previewScrim: "collection-preview-scrim",
  previewClose: "collection-preview-close",
  previewStatus: "collection-preview-status",
  // R3016000: a card's look as the card, in hand and on the board (LookViews), its Radiant toggle, and the hover panel that shows them (LookHover).
  look: "collection-look",
  lookView: (view: string) => `collection-look-${view}`,
  lookRadiant: "collection-look-radiant",
  lookHover: "collection-look-hover",
  // R3016001: the equip and buy screen (LooksScreen), its opener, its rungs' actions and its payment question.
  looksOpen: "collection-looks-open",
  looks: "collection-looks",
  looksClose: "collection-looks-close",
  looksRung: (id: string) => `collection-looks-rung-${id}`,
  looksEquip: (id: string) => `collection-looks-equip-${id}`,
  looksBuy: (id: string) => `collection-looks-buy-${id}`,
  looksMissing: (id: string) => `collection-looks-missing-${id}`,
  looksNeeds: (id: string) => `collection-looks-needs-${id}`,
  looksError: (id: string) => `collection-looks-error-${id}`,
  looksDialog: (id: string) => `collection-looks-dialog-${id}`,
  looksPay: (id: string, way: string) => `collection-looks-pay-${id}-${way}`,
  looksConfirm: (id: string) => `collection-looks-confirm-${id}`,
  looksCancel: (id: string) => `collection-looks-cancel-${id}`,
  // R3013001: a card's proficiency in its card view (ProficiencyPanel), its level, its plays and its stamps.
  proficiency: "collection-proficiency",
  proficiencyLevel: "collection-proficiency-level",
  proficiencyPlays: "collection-proficiency-plays",
  stamp: (level: number) => `collection-stamp-${String(level)}`,
  stampNone: "collection-stamp-none",
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
    case "version":
      return t("Version");
    case "augment":
      return t("Augments");
    case "starshipHull":
      return t("Hull design");
    case "starshipLivery":
      return t("Livery");
    case "starshipTrail":
      return t("Engine trail");
    case "starshipPieceSkin":
      return t("Piece skin");
  }
}

/** R1655000: the kinds a card's strip offers, in the registry's order. R3015007: its versions are its ladders (`cardLadders`). */
export const STRIP_KINDS = CARD_SLOT_KINDS;
export type StripKind = (typeof STRIP_KINDS)[number];

/** R1655000, R2725000: the normal face, the Default variant's first level (R3015000) and a pick of its own, which no registry entry names. */
export const NO_FULL_ART_ID = "no-full-art";

export type StripItem = {
  kind: StripKind;
  /** The registry's id. */
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
 * R1655000: what a card may wear of its foils, finishes and borders: the registry's, the default first. R3015007: its
 * versions are its ladders (`cardLadders`), and no retired full art or augment is listed.
 */
export function cardStrip(defId: string, canEquip: (defId: string, cosmeticId: CosmeticId) => boolean): StripGroup[] {
  return CARD_SLOT_KINDS.map((kind) => ({
    kind,
    items: cosmeticsOf(kind).map((cosmetic) => ({
      kind,
      id: cosmetic.id,
      name: t(cosmetic.name),
      rarity: cosmetic.rarity,
      owned: canEquip(defId, cosmetic.id),
    })),
  }));
}

/** R3015007: one level of a card's variant, as its ladder shows it. */
export type Rung = {
  /** The level's registry id; `NO_FULL_ART_ID` for the Default variant's first level, the normal face. */
  id: string;
  rank: number;
  /** In the reader's language. */
  name: string;
  rarity: CosmeticRarity;
  /** Whether the viewer may equip it on this card (`useCanEquip`); the normal face always. */
  owned: boolean;
  /** Not owned, and every level below it is: the one rung a token is offered on (R3015004). */
  next: boolean;
  /** A locked rung that is not next: the name of the highest level below it not owned, which it needs first. */
  needs: string | null;
};

/** R3015007: one variant a card has, its levels lowest first. */
export type Ladder = { variant: string; name: string; rungs: Rung[] };

/** R3015007: a level's name in the reader's language; the normal face's "Normal". */
function levelName(id: string): string {
  if (id === NO_FULL_ART_ID) return t("Normal");
  const name = COSMETICS.find((cosmetic) => cosmetic.id === id)?.name;
  return name === undefined ? id : t(name);
}

/**
 * R3015007: a card's variants as ladders, the Default first (its first level the normal face, always owned), then each
 * variant whose first level the card is offered (R3015000); each level the card is offered a rung, owned as `canEquip`
 * says, the next one marked (every level below it owned, R3015004), a locked one naming what it needs. R3343002: the
 * Default's first rung is named by the card's default look where it has one (R3343001, a Mythic's "Full art"), and a
 * variant whose first level that look is, offered nowhere there, is listed from the level above it, owned by everyone
 * (R3343000), so its first offered rung is next with nothing owned.
 */
export function cardLadders(defId: string, canEquip: (defId: string, cosmeticId: string) => boolean): Ladder[] {
  const shown = defaultLookOf(defId);
  const owns = (card: string, id: string): boolean => id === NO_FULL_ART_ID || id === shown || canEquip(card, id);
  return variantsOn(defId).map((variant) => {
    const levels = levelsOn(defId, variant);
    const rungs: Rung[] = levels.map((id) => {
      const entry = COSMETICS.find((cosmetic) => cosmetic.id === id);
      const owned = owns(defId, id);
      const next = !owned && isNextLevel(id, defId, owns);
      const missing = levelsBelow(id).filter((below) => !owns(defId, below));
      const needs = owned || next ? null : (missing.at(-1) ?? null);
      return {
        id,
        rank: levelOf(id)?.rank ?? FIRST_RANK,
        name: levelName(id),
        rarity: entry?.rarity ?? "Common",
        owned,
        next,
        needs: needs === null ? null : levelName(needs),
      };
    });
    if (variant !== DEFAULT_VARIANT) return { variant, name: levelName(ladderOf(variant)[0] ?? variant), rungs };
    const normal: Rung = { id: NO_FULL_ART_ID, rank: FIRST_RANK, name: levelName(shown ?? NO_FULL_ART_ID), rarity: "Common", owned: true, next: false, needs: null };
    return { variant, name: t("Default"), rungs: [normal, ...rungs] };
  });
}

/**
 * R3015005, R3015007: the rung a card's pick wears: the level it names (an older shape mapped as the server maps it),
 * else the highest one below it that `owns` takes; the normal face where that is none, and none with no pick at all
 * (the Full-art cards setting decides). R3016001: the equip and buy screen marks the same one.
 */
export function equippedRung(defId: string, picks: CardPicks, owns: (defId: string, id: string) => boolean): string | null {
  if (picks.fullArt === undefined && picks.augments === undefined) return null;
  return wornLevel(levelOfPick(defId, picks.fullArt, picks.augments), (id) => owns(defId, id)) ?? NO_FULL_ART_ID;
}

export type AccountTile = { cosmetic: Cosmetic; owned: boolean };

/**
 * R3335002: how an earned-only cosmetic is earned, in a player's words: the Fleet track level that pays it (R3335001),
 * or reaching Jlorious (R3335003); null for any other, which nothing but its own source earns.
 */
export function earnWords(cosmetic: {
  readonly kind: string;
  readonly jlorious?: boolean | undefined;
  readonly track?: string | null | undefined;
  readonly trackLevel?: number | null | undefined;
}): string | null {
  if (cosmetic.jlorious === true) return t("Jlorious on the ladder");
  if (cosmetic.track === "fleet" && typeof cosmetic.trackLevel === "number") {
    return t("Fleet track level {level}", { level: formatNumber(cosmetic.trackLevel) });
  }
  return null;
}

/**
 * R1579000, R1655000: `kind`'s entries in the registry's order, owned as `owns` says (R1833000:
 * `useOwnsAccountCosmetic`'s; a default and a starter are every account's, §14.2). R3335000: a Starship kind's too.
 */
export function accountTiles(kind: DeckSlotKind | StarshipSlotKind, owns: (id: string) => boolean): AccountTile[] {
  return cosmeticsOf(kind).map((cosmetic) => ({ cosmetic, owned: owns(cosmetic.id) }));
}
