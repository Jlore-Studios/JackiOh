// The collection screen's model (#1307, SPEC §14): its test ids and the pure helpers behind the
// owned-cards, wallet and track panels, all of them plain data in and out.
//
// EVERY NUMBER SHOWN COMES FROM A RESPONSE, `serverConfig.ts` OR THE SHIPPED CATALOG (CLAUDE.md
// rule 7). The balance, the unopened packs, each track's level, XP and rewards and the quests
// arrive in the server's answers (`net/api.ts`); a set's total is the count of its collectible
// cards in the catalog (§14: only the non-token cards of a set that ships are collected), and
// whether one is owned is the row the server sent. Nothing here prices, levels or decides.

import { CATALOG } from "@jackioh/cards";
import { SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
import { SHIPPED_SETS, newestShippedSet, setShips, type CardDef, type SetName } from "@jackioh/shared";

import { formatNumber, t } from "../i18n/i18n.ts";
import type { CollectionResponse, TrackRewards } from "../net/api.ts";
import { packName } from "../packs/packs.ts";
import { quantitiesOf } from "../shop/shop.ts";

/** The panels that read for themselves; the quests panel owns its own read. */
export type CollectionPart = "cards" | "wallet" | "tracks";

export const collectionTestid = {
  screen: "collection-screen",
  jlore: "collection-jlore",
  cosmeticPacks: "collection-cosmetic-packs",
  noPacks: "collection-no-packs",
  loading: (part: CollectionPart) => `collection-loading-${part}`,
  error: (part: CollectionPart) => `collection-error-${part}`,
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
} as const;

/** The cards a player can own: not a token, of a set that ships. */
export const COLLECTED_DEFS: readonly CardDef[] = Object.values(CATALOG).filter((def) => !def.token && setShips(def.set));

/** One shipped set's cards and how many of them the account owns. */
export type SetCompletion = {
  set: SetName;
  owned: number;
  total: number;
  cards: { def: CardDef; owned: boolean }[];
};

/**
 * One entry per shipped set, in catalog order, over the non-token cards of `defs`. A card is
 * owned when the server's row for it holds a quantity above 0.
 */
export function setCompletions(collection: CollectionResponse, defs: readonly CardDef[] = COLLECTED_DEFS): SetCompletion[] {
  const quantities = quantitiesOf(collection);
  return SHIPPED_SETS.map((set) => {
    const cards = defs
      .filter((def) => def.set === set && !def.token)
      .map((def) => ({ def, owned: (quantities.get(def.id) ?? 0) > 0 }));
    return { set, owned: cards.filter((card) => card.owned).length, total: cards.length, cards };
  });
}

/** What a level pays, the parts above 0 in a player's words: "25 Jlore, 2 × Core pack, 3 × card token". */
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
