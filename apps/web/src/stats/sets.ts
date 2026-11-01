// R1966000: the Sets tab of `/stats` (SPEC §9.11), read off the card figures `GET /api/stats/cards` sends.
// A card's win rate is R377's in-deck rate; a set's pools its cards (their wins over their games), and its
// mean and median are over the cards with the sample floor's games (R654). Decks mix sets (R380), so a
// set's rate is how its cards fare in mixed decks, not how a deck of it would.

import { SHIPPED_SETS } from "@jackioh/shared";

import type { PublicCardStat } from "../net/api.ts";
import { SET_STATS_CARDS_LISTED, SET_STATS_OUTLIER_Z } from "./config.ts";

/** A coin flip: the win rate of a card that changes nothing about the games it is in. */
const EVEN = 0.5;

/** A card with the sample floor's games, as the Sets tab ranks it. */
export type RatedCard = {
  id: string;
  name: string;
  set: string;
  winRate: number;
  games: number;
  /** How many standard errors its win rate is from 50%. */
  z: number;
};

export type SetRow = {
  /** As `SHIPPED_SETS` names it. */
  set: string;
  /** Its cards with any games, and those with the sample floor's. */
  cards: number;
  rated: number;
  /** Its cards' games summed (a game counts once per card of it in a deck), and their wins. */
  games: number;
  wins: number;
  pooled: number | null;
  /** The pooled rate's standard error as if every card-game were independent: they are not, so it is a floor. */
  error: number | null;
  mean: number | null;
  median: number | null;
  best: RatedCard | null;
  worst: RatedCard | null;
  /** Its rated cards `SET_STATS_OUTLIER_Z` or more standard errors over 50%, and under it. */
  strong: number;
  weak: number;
};

/** The set a card's row names, as `SHIPPED_SETS` spells it (the server's is the catalog's, in any case). */
export function setOf(card: Pick<PublicCardStat, "set">): string {
  const folded = card.set.toLowerCase();
  return SHIPPED_SETS.find((set) => set.toLowerCase() === folded) ?? card.set;
}

function rated(card: PublicCardStat, minSample: number): RatedCard | null {
  if (card.winRate === null || card.games < minSample) return null;
  const z = (card.winRate - EVEN) / Math.sqrt((EVEN * (1 - EVEN)) / card.games);
  return { id: card.id, name: card.name, set: setOf(card), winRate: card.winRate, games: card.games, z };
}

function median(values: number[]): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}

function byRate(a: RatedCard, b: RatedCard): number {
  return b.winRate - a.winRate || b.games - a.games || a.id.localeCompare(b.id);
}

/** Each set with games on the patch, highest pooled win rate first. */
export function setRows(cards: readonly PublicCardStat[], minSample: number): SetRow[] {
  const sets = new Map<string, PublicCardStat[]>();
  for (const card of cards) {
    if (card.games === 0 || card.winRate === null) continue;
    const set = setOf(card);
    sets.set(set, [...(sets.get(set) ?? []), card]);
  }
  const rows: SetRow[] = [];
  for (const [set, mine] of sets) {
    const games = mine.reduce((sum, card) => sum + card.games, 0);
    const wins = mine.reduce((sum, card) => sum + Math.round((card.winRate ?? 0) * card.games), 0);
    const pooled = games > 0 ? wins / games : null;
    const ranked = mine.flatMap((card) => rated(card, minSample) ?? []).sort(byRate);
    const rates = ranked.map((card) => card.winRate);
    rows.push({
      set,
      cards: mine.length,
      rated: ranked.length,
      games,
      wins,
      pooled,
      error: pooled === null ? null : Math.sqrt((pooled * (1 - pooled)) / games),
      mean: rates.length > 0 ? rates.reduce((sum, rate) => sum + rate, 0) / rates.length : null,
      median: median(rates),
      best: ranked[0] ?? null,
      worst: ranked.at(-1) ?? null,
      strong: ranked.filter((card) => card.z >= SET_STATS_OUTLIER_Z).length,
      weak: ranked.filter((card) => card.z <= -SET_STATS_OUTLIER_Z).length,
    });
  }
  return rows.sort((a, b) => (b.pooled ?? 0) - (a.pooled ?? 0) || a.set.localeCompare(b.set));
}

export type Leaders = { pooled: string | null; mean: string | null; median: string | null };

/** The set with the highest of each measure, or null when no set has it. */
export function leaders(rows: readonly SetRow[]): Leaders {
  const top = (measure: (row: SetRow) => number | null): string | null => {
    let best: SetRow | null = null;
    for (const row of rows) {
      const value = measure(row);
      if (value !== null && (best === null || value > (measure(best) ?? 0))) best = row;
    }
    return best?.set ?? null;
  };
  return { pooled: top((row) => row.pooled), mean: top((row) => row.mean), median: top((row) => row.median) };
}

/** Each set's pooled win rate change from the previous patch, where both have one. */
export function changes(rows: readonly SetRow[], previous: readonly SetRow[]): Map<string, number> {
  const before = new Map(previous.map((row) => [row.set, row.pooled]));
  const out = new Map<string, number>();
  for (const row of rows) {
    const then = before.get(row.set);
    if (row.pooled !== null && then !== null && then !== undefined) out.set(row.set, row.pooled - then);
  }
  return out;
}

/** The cards `SET_STATS_OUTLIER_Z` or more standard errors from 50%, at most `SET_STATS_CARDS_LISTED` a side, furthest first. */
export function outliers(
  cards: readonly PublicCardStat[],
  minSample: number,
): { strongest: RatedCard[]; weakest: RatedCard[] } {
  const ranked = cards.flatMap((card) => rated(card, minSample) ?? []);
  return {
    strongest: ranked
      .filter((card) => card.z >= SET_STATS_OUTLIER_Z)
      .sort((a, b) => b.z - a.z)
      .slice(0, SET_STATS_CARDS_LISTED),
    weakest: ranked
      .filter((card) => card.z <= -SET_STATS_OUTLIER_Z)
      .sort((a, b) => a.z - b.z)
      .slice(0, SET_STATS_CARDS_LISTED),
  };
}
