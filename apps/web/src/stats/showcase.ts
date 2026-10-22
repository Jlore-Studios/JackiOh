// R1584000: the stats the floor below the tavern's showcase shows, one at a time: each with a title,
// the card it is about and the player's number for it. A stat the device holds no data for is left
// out. This module only chooses and words them; StatShowcase.tsx draws them and showcaseStage.ts
// plays the board's animation for each. It takes the stage's types and nothing else of it, so the
// effects layer stays off the first screen until the showcase is drawn.

import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { Side } from "../game/contract.ts";
import { topCards, winPercent, type CardCounter, type PlayerStats } from "./model.ts";
import type { PocketPick } from "./pocketPick.ts";
import type { StageAnimation } from "./showcaseStage.ts";

const CATALOG = catalogJson as unknown as CardDefs;

export type ShowcaseKey =
  | "toppled"
  | "coveted"
  | "pocketPick"
  | "faithful"
  | "nemesis"
  | "fallen"
  | "record"
  | "lastLaugh"
  | "springloaded"
  | "survivor";

export type ShowcaseStat = {
  readonly key: ShowcaseKey;
  /** The card the stat is about, one the catalog holds. */
  readonly cardId: string;
  /** The stat's name and its sentence say themselves when they are called, never at import. */
  readonly title: () => string;
  readonly line: () => string;
  /** The board animation the card plays, and the side whose lane it stands in. */
  readonly animation: StageAnimation;
  readonly side: Side;
};

/** The record line: "12 games: 7 won, 4 lost, 1 drawn (58% won)". */
export function recordLine(stats: PlayerStats): string {
  const percent = winPercent(stats);
  const games = stats.games === 1 ? t("{n} game", { n: stats.games }) : t("{n} games", { n: stats.games });
  const counts = { games, won: stats.wins, lost: stats.losses, drawn: stats.draws, percent: percent ?? 0 };
  if (stats.draws > 0) {
    return percent === null
      ? t("{games}: {won} won, {lost} lost, {drawn} drawn", counts)
      : t("{games}: {won} won, {lost} lost, {drawn} drawn ({percent}% won)", counts);
  }
  return percent === null
    ? t("{games}: {won} won, {lost} lost", counts)
    : t("{games}: {won} won, {lost} lost ({percent}% won)", counts);
}

/** R1583000: "you play it 4× as often as everyone else", the ratio to a tenth; "nobody else plays it" at a base rate of 0. */
export function pocketLine(pick: PocketPick): string {
  if (pick.baseRate === 0) return t("nobody else plays it");
  return t("you play it {n}× as often as everyone else", { n: Math.round((pick.playerRate / pick.baseRate) * 10) / 10 });
}

/** The card with the most of one counter that the catalog still holds; ties go to the lower id (`topCards`'s order). */
function topCard(stats: PlayerStats, counter: CardCounter): { id: string; count: number } | undefined {
  return topCards(stats, counter, Object.keys(stats.cards).length).find((tally) => CATALOG[tally.id] !== undefined);
}

/** A stat about the card the device logged most under one counter, worded with that count. */
function counted(
  stats: PlayerStats,
  counter: CardCounter,
  stat: Omit<ShowcaseStat, "cardId" | "line">,
  line: (n: number) => string,
): ShowcaseStat[] {
  const top = topCard(stats, counter);
  return top === undefined ? [] : [{ ...stat, cardId: top.id, line: () => line(top.count) }];
}

/**
 * The stats to show, in the order the floor tells them: one for each the device has data for, none
 * before the first finished game. The Pocket Pick comes from R1583000's request, so it is absent
 * until that answers, when it names nothing, or when the catalog no longer holds its card.
 */
export function showcaseStats(stats: PlayerStats, pocket: PocketPick | null): ShowcaseStat[] {
  if (stats.games === 0) return [];
  const out: ShowcaseStat[] = [
    ...counted(stats, "defeated", { key: "toppled", title: () => t("Most Toppled"), animation: "death", side: "opponent" }, (n) =>
      t("taken down {n}×", { n }),
    ),
    ...counted(stats, "allured", { key: "coveted", title: () => t("Most Coveted"), animation: "allure", side: "you" }, (n) =>
      t("stolen {n}×", { n }),
    ),
  ];
  if (pocket !== null && CATALOG[pocket.id] !== undefined) {
    out.push({
      key: "pocketPick",
      cardId: pocket.id,
      title: () => t("Pocket Pick"),
      line: () => pocketLine(pocket),
      animation: "trap",
      side: "you",
    });
  }
  out.push(
    ...counted(stats, "played", { key: "faithful", title: () => t("Old Faithful"), animation: "entrance", side: "you" }, (n) =>
      t("played {n}×", { n }),
    ),
    ...counted(stats, "playedAgainst", { key: "nemesis", title: () => t("Arch-Nemesis"), animation: "lunge", side: "opponent" }, (n) =>
      t("played against you {n}×", { n }),
    ),
    ...counted(stats, "destroyed", { key: "fallen", title: () => t("Fallen Hero"), animation: "death", side: "you" }, (n) =>
      t("lost {n}×", { n }),
    ),
  );
  const seen = topCard(stats, "seen");
  if (seen !== undefined) {
    out.push({
      key: "record",
      cardId: seen.id,
      title: () => t("The Record"),
      line: () => recordLine(stats),
      animation: "radiant",
      side: "you",
    });
  }
  out.push(
    ...counted(stats, "lethal", { key: "lastLaugh", title: () => t("Last Laugh"), animation: "lunge", side: "you" }, (n) =>
      t("dealt the winning blow {n}×", { n }),
    ),
    ...counted(stats, "sprung", { key: "springloaded", title: () => t("Springloaded"), animation: "trap", side: "you" }, (n) =>
      t("sprung {n}×", { n }),
    ),
    ...counted(stats, "survived", { key: "survivor", title: () => t("Survivor"), animation: "shield", side: "you" }, (n) =>
      t("survived to the end {n}×", { n }),
    ),
  );
  return out;
}
