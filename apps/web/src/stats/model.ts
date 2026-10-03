// What a device remembers about its player's games (SPEC R639), and the pure arithmetic over it.
//
// Nothing here is a rule and nothing reads hidden information (CLAUDE.md rule 7): a game is logged
// from the `PlayerView`s the client already holds, whose events and cards are redacted to what this
// viewer may read (SPEC §10.8, R97), so a statistic can only name a card the player was shown.
// `track.ts` turns views into a `GameLog`; `store.ts` keeps the totals on the device.

import type { PlayerId } from "@jackioh/shared";

/** The five counts kept for each card, all from the viewer's own seat (R639). */
export const CARD_COUNTERS = ["seen", "played", "playedAgainst", "destroyed", "defeated"] as const;
export type CardCounter = (typeof CARD_COUNTERS)[number];

/**
 * - `seen`: games in which the card was in front of the player (their hand, either board, a graveyard).
 * - `played`: times the player played it.
 * - `playedAgainst`: times the opponent played it where the player could read it.
 * - `destroyed`: times a copy of the player's own was destroyed.
 * - `defeated`: times a copy of the opponent's was destroyed.
 */
export type CardCounters = Readonly<Record<CardCounter, number>>;

export const NO_COUNTS: CardCounters = Object.freeze({ seen: 0, played: 0, playedAgainst: 0, destroyed: 0, defeated: 0 });

export type PlayerStats = {
  readonly games: number;
  readonly wins: number;
  readonly losses: number;
  readonly draws: number;
  /** By catalog id (a definition, whichever face it was on). */
  readonly cards: Readonly<Record<string, CardCounters>>;
};

export const EMPTY_STATS: PlayerStats = Object.freeze({
  games: 0,
  wins: 0,
  losses: 0,
  draws: 0,
  cards: Object.freeze({}),
});

/** One game's counts so far: what `track.ts` collects until the game is over. */
export type GameLog = {
  /** Each card once, however often it was in view. */
  readonly seen: readonly string[];
  readonly played: Readonly<Record<string, number>>;
  readonly playedAgainst: Readonly<Record<string, number>>;
  readonly destroyed: Readonly<Record<string, number>>;
  readonly defeated: Readonly<Record<string, number>>;
};

export const EMPTY_LOG: GameLog = Object.freeze({
  seen: Object.freeze([]) as readonly string[],
  played: Object.freeze({}),
  playedAgainst: Object.freeze({}),
  destroyed: Object.freeze({}),
  defeated: Object.freeze({}),
});

export type GameOutcome = "win" | "loss" | "draw";

/** How a finished game reads from `viewer`'s seat. */
export function outcomeFor(winner: PlayerId | "draw", viewer: PlayerId): GameOutcome {
  if (winner === "draw") return "draw";
  return winner === viewer ? "win" : "loss";
}

function bump(counts: Readonly<Record<string, number>>, id: string, by = 1): Record<string, number> {
  return { ...counts, [id]: (counts[id] ?? 0) + by };
}

/** The log with `id` added to each of its counts the event named. */
export function logWith(log: GameLog, counter: Exclude<CardCounter, "seen">, id: string): GameLog {
  return { ...log, [counter]: bump(log[counter], id) };
}

/** The log with these cards seen, each once however many views showed it. */
export function logSeen(log: GameLog, ids: Iterable<string>): GameLog {
  const seen = new Set(log.seen);
  let changed = false;
  for (const id of ids) {
    if (seen.has(id)) continue;
    seen.add(id);
    changed = true;
  }
  return changed ? { ...log, seen: [...seen] } : log;
}

/** True when the log has recorded nothing, so a game never viewed adds no card counts. */
export function logIsEmpty(log: GameLog): boolean {
  return (
    log.seen.length === 0 &&
    Object.keys(log.played).length === 0 &&
    Object.keys(log.playedAgainst).length === 0 &&
    Object.keys(log.destroyed).length === 0 &&
    Object.keys(log.defeated).length === 0
  );
}

/** The totals with one finished game folded in. */
export function addGame(stats: PlayerStats, log: GameLog, outcome: GameOutcome): PlayerStats {
  const cards: Record<string, CardCounters> = { ...stats.cards };
  const add = (id: string, counter: CardCounter, by: number): void => {
    const current = cards[id] ?? NO_COUNTS;
    cards[id] = { ...current, [counter]: current[counter] + by };
  };
  for (const id of log.seen) add(id, "seen", 1);
  for (const counter of ["played", "playedAgainst", "destroyed", "defeated"] as const) {
    for (const [id, n] of Object.entries(log[counter])) add(id, counter, n);
  }
  return {
    games: stats.games + 1,
    wins: stats.wins + (outcome === "win" ? 1 : 0),
    losses: stats.losses + (outcome === "loss" ? 1 : 0),
    draws: stats.draws + (outcome === "draw" ? 1 : 0),
    cards,
  };
}

/** Wins as a whole percentage of games, or null before the first game. */
export function winPercent(stats: PlayerStats): number | null {
  return stats.games === 0 ? null : Math.round((stats.wins / stats.games) * 100);
}

/** One card and its count, for a "favourites" list. */
export type CardTally = { readonly id: string; readonly count: number };

/**
 * The `limit` cards with the highest `counter`, highest first and by id among equals, so the list is
 * the same on every render. A card with a count of 0 is never listed.
 */
export function topCards(stats: PlayerStats, counter: CardCounter, limit: number): CardTally[] {
  return Object.entries(stats.cards)
    .map(([id, counts]) => ({ id, count: counts[counter] }))
    .filter((entry) => entry.count > 0)
    .sort((a, b) => b.count - a.count || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .slice(0, Math.max(0, limit));
}
