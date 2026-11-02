// R2118000: the command menu's entries. Every entry is a `navigate` move, so R1754000's leave
// guard still holds one. The labels are built per call, from a render, never cached across a
// language switch. Presentation only (CLAUDE.md rule 7): the cards come from the public catalog
// through `almanacShelf`, and nothing here sends an action.

import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { almanacShelf } from "../game/deckbuilder/shelf.ts";
import { t } from "../i18n/i18n.ts";
import {
  duosIdOf,
  matchIdOf,
  newDeckPath,
  paths,
  seriesIdOf,
  watchIdOf,
} from "../net/navigate.ts";

/** JSON widens the unions to strings; the catalog is proved against SPEC §8 in crates/cards. */
const CATALOG = catalogJson as unknown as CardDefs;

/** R2118000: how many card results the menu lists at most, in catalog order, unranked. */
export const COMMAND_MENU_CARDS_SHOWN = 8;

export type CommandEntry = {
  readonly id: string;
  readonly kind: "action" | "page" | "card";
  readonly label: string;
  readonly path: string;
};

/**
 * R2118000: whether the shell mounts the command menu on `path`. Never on a board: a match, a
 * Conquest series (its pick clock runs), a watched game, a Duos game, or the dev boards. A
 * practice game or a replay shares its path with a menu, so those stay mounted and Game.tsx marks
 * itself instead (`game/boardShown.ts`).
 */
export function commandMenuOn(path: string): boolean {
  if (path === paths.hotseat || path === paths.duosDev) return false;
  if (matchIdOf(path) !== null || seriesIdOf(path) !== null) return false;
  if (watchIdOf(path) !== null || duosIdOf(path) !== null) return false;
  return true;
}

/**
 * R2118000: the common actions, in order. Practice and Play open `/practice` and `/play`, not
 * repeated as pages, and start no game by themselves; #2705: the Dungeon Run opens `/dungeon` the same way.
 */
export function commandActions(): CommandEntry[] {
  return [
    { id: "action-new-deck", kind: "action", label: t("New deck"), path: newDeckPath() },
    { id: "action-practice", kind: "action", label: t("Practice"), path: paths.practice },
    { id: "action-play", kind: "action", label: t("Play"), path: paths.play },
    { id: "action-dungeon", kind: "action", label: t("Dungeon Run"), path: paths.dungeon },
  ];
}

/** R2118000: every menu page the player can open. Not listed: the Collection's sections and the sign-in, reset, invite and dev screens. */
export function commandPages(): CommandEntry[] {
  return [
    { id: "page-home", kind: "page", label: t("Home"), path: paths.landing },
    { id: "page-decks", kind: "page", label: t("Decks"), path: paths.decks },
    { id: "page-collection", kind: "page", label: t("Collection"), path: paths.collection },
    { id: "page-creator", kind: "page", label: t("Card builder"), path: paths.creator },
    { id: "page-shop", kind: "page", label: t("Shop"), path: paths.shop },
    { id: "page-purchases", kind: "page", label: t("Purchases"), path: paths.purchases },
    { id: "page-almanac", kind: "page", label: t("Card almanac"), path: paths.almanac },
    { id: "page-patch-notes", kind: "page", label: t("Patch notes"), path: paths.patchNotes },
    { id: "page-stats", kind: "page", label: t("Stats"), path: paths.stats },
    { id: "page-leaderboard", kind: "page", label: t("Leaderboard"), path: paths.leaderboard },
    { id: "page-replays", kind: "page", label: t("Replays"), path: paths.replays },
    { id: "page-together", kind: "page", label: t("Play together"), path: paths.together },
    { id: "page-account", kind: "page", label: t("Account"), path: paths.account },
    { id: "page-privacy", kind: "page", label: t("Privacy"), path: paths.privacy },
    { id: "page-terms", kind: "page", label: t("Terms"), path: paths.terms },
    { id: "page-accessibility", kind: "page", label: t("Accessibility"), path: paths.accessibility },
    { id: "page-copyright", kind: "page", label: t("Copyright"), path: paths.copyright },
  ];
}

/**
 * R2118000: the Almanac shelf's cards by name (`almanacShelf`, R1420: no unshipped card), each
 * opening its inspect detail (`paths.almanacCard`, R630). Named in the reader's language
 * (`readerName`, R711000).
 */
export function commandCards(cards: CardDefs = CATALOG): CommandEntry[] {
  return almanacShelf(cards).flatMap((id): CommandEntry[] => {
    const def = cards[id];
    if (def === undefined) return [];
    return [{ id: `card-${id}`, kind: "card", label: readerName(def), path: paths.almanacCard(id) }];
  });
}

/**
 * R2118000: whether `label` matches `query`: every word of the query is a substring of the label,
 * both in lower case. A blank query matches everything.
 */
export function matchesQuery(label: string, query: string): boolean {
  const words = query.toLocaleLowerCase().split(/\s+/u).filter((word) => word.length > 0);
  if (words.length === 0) return true;
  const lowered = label.toLocaleLowerCase();
  return words.every((word) => lowered.includes(word));
}

/**
 * R2118000: the entries a query lists: the matching actions, then the matching pages, then, once
 * the player types, the matching cards, at most COMMAND_MENU_CARDS_SHOWN of them.
 */
export function commandEntries(query: string, cards: CardDefs = CATALOG): CommandEntry[] {
  const actions = commandActions().filter((entry) => matchesQuery(entry.label, query));
  const pages = commandPages().filter((entry) => matchesQuery(entry.label, query));
  if (query.trim().length === 0) return [...actions, ...pages];
  const matches = commandCards(cards).filter((entry) => matchesQuery(entry.label, query));
  return [...actions, ...pages, ...matches.slice(0, COMMAND_MENU_CARDS_SHOWN)];
}
