// Device statistics (SPEC R639) use redacted PlayerViews (SPEC §10.8, R97; CLAUDE.md rule 7).
// `track.ts` turns views into a `GameLog`; `store.ts` keeps totals on the device.

import type { GameOverReason, PlayerId } from "@jackioh/shared";

import { seatOutcome } from "../game/outcome.ts";

export const CARD_COUNTERS = [
  "seen",
  "played",
  "playedAgainst",
  "destroyed",
  "defeated",
  "allured",
  "lethal",
  "sprung",
  "survived",
  "damageDealt",
  "biggestHit",
  "overkill",
  "attacks",
  "healing",
  "drawn",
  "radiant",
  "playedInWin",
  "playedInLoss",
  "tempExpired",
  "tempPlayed",
  "crumbled",
  "crumbledAgainst",
  "fengShuiGood",
  "fengShuiBad",
  "traded",
  "tradeDrawn",
  "created",
  "kills",
  "biggestKill",
  "reborn",
  "shieldsLost",
  "armorAbsorbed",
  "damageTaken",
  "doubleAttacks",
  "lifesteal",
  "sacrificed",
  "froze",
  "frozen",
  "thawed",
  "shattered",
  "flashback",
  "manaSaved",
  "countered",
  "counterspells",
  "burned",
  "bounced",
  "launched",
] as const;
export type CardCounter = (typeof CARD_COUNTERS)[number];

export const SUMMED_COUNTERS = [
  "played",
  "playedAgainst",
  "destroyed",
  "defeated",
  "allured",
  "lethal",
  "sprung",
  "survived",
  "damageDealt",
  "attacks",
  "healing",
  "drawn",
  "radiant",
  "tempExpired",
  "tempPlayed",
  "crumbled",
  "crumbledAgainst",
  "fengShuiGood",
  "fengShuiBad",
  "traded",
  "tradeDrawn",
  "created",
  "kills",
  "reborn",
  "shieldsLost",
  "armorAbsorbed",
  "damageTaken",
  "doubleAttacks",
  "lifesteal",
  "sacrificed",
  "froze",
  "frozen",
  "thawed",
  "shattered",
  "flashback",
  "manaSaved",
  "countered",
  "counterspells",
  "burned",
  "bounced",
  "launched",
] as const;
export type SummedCounter = (typeof SUMMED_COUNTERS)[number];

/** R1743000: the counters that keep the largest value of any game, not a sum. */
export const MAX_COUNTERS = ["biggestHit", "overkill", "biggestKill"] as const;
export type MaxCounter = (typeof MAX_COUNTERS)[number];

/** Per-card viewer counts, summed or kept at their maximum across games (R1582000, R1743000, R2649000). */
export type CardCounters = Readonly<Record<CardCounter, number>>;

export const NO_COUNTS: CardCounters = Object.freeze({
  seen: 0,
  played: 0,
  playedAgainst: 0,
  destroyed: 0,
  defeated: 0,
  allured: 0,
  lethal: 0,
  sprung: 0,
  survived: 0,
  damageDealt: 0,
  biggestHit: 0,
  overkill: 0,
  attacks: 0,
  healing: 0,
  drawn: 0,
  radiant: 0,
  playedInWin: 0,
  playedInLoss: 0,
  tempExpired: 0,
  tempPlayed: 0,
  crumbled: 0,
  crumbledAgainst: 0,
  fengShuiGood: 0,
  fengShuiBad: 0,
  traded: 0,
  tradeDrawn: 0,
  created: 0,
  kills: 0,
  biggestKill: 0,
  reborn: 0,
  shieldsLost: 0,
  armorAbsorbed: 0,
  damageTaken: 0,
  doubleAttacks: 0,
  lifesteal: 0,
  sacrificed: 0,
  froze: 0,
  frozen: 0,
  thawed: 0,
  shattered: 0,
  flashback: 0,
  manaSaved: 0,
  countered: 0,
  counterspells: 0,
  burned: 0,
  bounced: 0,
  launched: 0,
});

export type PlayerStats = {
  readonly games: number;
  readonly wins: number;
  readonly losses: number;
  readonly draws: number;
  /** By catalog definition id. */
  readonly cards: Readonly<Record<string, CardCounters>>;
  /** R1743000: the games won in a row now; a loss or a draw resets it. */
  readonly streak: number;
  readonly bestStreak: number;
  /** the most turns any finished game ran. */
  readonly longestGame: number;
  /** the player's plays, by card, in that longest game. */
  readonly longestGamePlayed: Readonly<Record<string, number>>;
};

export const EMPTY_STATS: PlayerStats = Object.freeze({
  games: 0,
  wins: 0,
  losses: 0,
  draws: 0,
  cards: Object.freeze({}),
  streak: 0,
  bestStreak: 0,
  longestGame: 0,
  longestGamePlayed: Object.freeze({}),
});

export type GameLog = {
  /** Each card id once. */
  readonly seen: readonly string[];
  readonly played: Readonly<Record<string, number>>;
  readonly playedAgainst: Readonly<Record<string, number>>;
  readonly destroyed: Readonly<Record<string, number>>;
  readonly defeated: Readonly<Record<string, number>>;
  /** The player's cards the opponent's Allure took. */
  readonly allured: Readonly<Record<string, number>>;
  /** The cards that dealt the lethal blow of a game the player won. */
  readonly lethal: Readonly<Record<string, number>>;
  /** The player's Traps that fired. */
  readonly sprung: Readonly<Record<string, number>>;
  /** The player's Units standing when a game they won ended. */
  readonly survived: Readonly<Record<string, number>>;
  readonly damageDealt: Readonly<Record<string, number>>;
  readonly biggestHit: Readonly<Record<string, number>>;
  /** The most by which one hit of each of the player's cards went past its target's health. */
  readonly overkill: Readonly<Record<string, number>>;
  readonly attacks: Readonly<Record<string, number>>;
  readonly healing: Readonly<Record<string, number>>;
  readonly drawn: Readonly<Record<string, number>>;
  /** The player's cards made Radiant. */
  readonly radiant: Readonly<Record<string, number>>;
  /** The highest turn a `turnStarted` named in this game. */
  readonly turns: number;
  /** R2649000: the player's Temporary cards discarded unplayed at the end of their turn. */
  readonly tempExpired: Readonly<Record<string, number>>;
  /** the player's plays of a card Temporary in their hand. */
  readonly tempPlayed: Readonly<Record<string, number>>;
  /** the player's cards that crumbled (Brittle, R638). */
  readonly crumbled: Readonly<Record<string, number>>;
  /** the opponent's cards that crumbled. */
  readonly crumbledAgainst: Readonly<Record<string, number>>;
  /** the player's plays a Feng Shui judged positive (R983). */
  readonly fengShuiGood: Readonly<Record<string, number>>;
  /** the player's plays a Feng Shui judged negative. */
  readonly fengShuiBad: Readonly<Record<string, number>>;
  /** the player's Tradable cards traded away (R1838001). */
  readonly traded: Readonly<Record<string, number>>;
  /** the player's cards drawn by a trade. */
  readonly tradeDrawn: Readonly<Record<string, number>>;
  /** the player's Created cards (R943), each instance once. */
  readonly created: Readonly<Record<string, number>>;
  /** the opponent's cards the player's cards killed. */
  readonly kills: Readonly<Record<string, number>>;
  /** the largest `maxHealth` of a card each of the player's cards killed. */
  readonly biggestKill: Readonly<Record<string, number>>;
  /** the player's Reborn Units that came back. */
  readonly reborn: Readonly<Record<string, number>>;
  /** the Divine Shields the player's Units lost. */
  readonly shieldsLost: Readonly<Record<string, number>>;
  /** the damage Armor took off the player's Units (R1360, R1361). */
  readonly armorAbsorbed: Readonly<Record<string, number>>;
  /** the damage the player's Units took. */
  readonly damageTaken: Readonly<Record<string, number>>;
  /** the player's Units' second attack in one turn. */
  readonly doubleAttacks: Readonly<Record<string, number>>;
  /** the healing the player's Lifesteal cards drained. */
  readonly lifesteal: Readonly<Record<string, number>>;
  /** the player's own deaths at their own hand. */
  readonly sacrificed: Readonly<Record<string, number>>;
  /** the opponent's Units the player's cards froze. */
  readonly froze: Readonly<Record<string, number>>;
  /** the player's Units frozen. */
  readonly frozen: Readonly<Record<string, number>>;
  /** the player's Units that thawed on the field. */
  readonly thawed: Readonly<Record<string, number>>;
  /** the frozen opponent Units the player's cards killed. */
  readonly shattered: Readonly<Record<string, number>>;
  /** the player's plays from the graveyard (R783012). */
  readonly flashback: Readonly<Record<string, number>>;
  /** the mana the player's plays cost under their printed cost. */
  readonly manaSaved: Readonly<Record<string, number>>;
  /** the player's plays countered. */
  readonly countered: Readonly<Record<string, number>>;
  /** the opponent's plays the player's cards countered. */
  readonly counterspells: Readonly<Record<string, number>>;
  /** the player's cards burned from a full hand (R4). */
  readonly burned: Readonly<Record<string, number>>;
  /** the player's cards bounced back to the hand. */
  readonly bounced: Readonly<Record<string, number>>;
  /** the player's Starship pieces launched in a Starship (R1852003). */
  readonly launched: Readonly<Record<string, number>>;
  /** the Created instances already counted this game; never stored. */
  readonly createdIds: readonly string[];
  /** this turn's attacks, by the attacker's instance id; never stored. */
  readonly attackers: Readonly<Record<string, number>>;
};

export const EMPTY_LOG: GameLog = Object.freeze({
  seen: Object.freeze([]) as readonly string[],
  played: Object.freeze({}),
  playedAgainst: Object.freeze({}),
  destroyed: Object.freeze({}),
  defeated: Object.freeze({}),
  allured: Object.freeze({}),
  lethal: Object.freeze({}),
  sprung: Object.freeze({}),
  survived: Object.freeze({}),
  damageDealt: Object.freeze({}),
  biggestHit: Object.freeze({}),
  overkill: Object.freeze({}),
  attacks: Object.freeze({}),
  healing: Object.freeze({}),
  drawn: Object.freeze({}),
  radiant: Object.freeze({}),
  turns: 0,
  tempExpired: Object.freeze({}),
  tempPlayed: Object.freeze({}),
  crumbled: Object.freeze({}),
  crumbledAgainst: Object.freeze({}),
  fengShuiGood: Object.freeze({}),
  fengShuiBad: Object.freeze({}),
  traded: Object.freeze({}),
  tradeDrawn: Object.freeze({}),
  created: Object.freeze({}),
  kills: Object.freeze({}),
  biggestKill: Object.freeze({}),
  reborn: Object.freeze({}),
  shieldsLost: Object.freeze({}),
  armorAbsorbed: Object.freeze({}),
  damageTaken: Object.freeze({}),
  doubleAttacks: Object.freeze({}),
  lifesteal: Object.freeze({}),
  sacrificed: Object.freeze({}),
  froze: Object.freeze({}),
  frozen: Object.freeze({}),
  thawed: Object.freeze({}),
  shattered: Object.freeze({}),
  flashback: Object.freeze({}),
  manaSaved: Object.freeze({}),
  countered: Object.freeze({}),
  counterspells: Object.freeze({}),
  burned: Object.freeze({}),
  bounced: Object.freeze({}),
  launched: Object.freeze({}),
  createdIds: Object.freeze([]) as readonly string[],
  attackers: Object.freeze({}),
});

export type GameOutcome = "win" | "loss" | "draw";

/** R1843004: given the result's reason, the other player's `fled` is a draw (`game/outcome.ts`). */
export function outcomeFor(winner: PlayerId | "draw", viewer: PlayerId, reason?: GameOverReason): GameOutcome {
  if (reason !== undefined) return seatOutcome(winner, reason, viewer);
  if (winner === "draw") return "draw";
  return winner === viewer ? "win" : "loss";
}

function bump(counts: Readonly<Record<string, number>>, id: string, by = 1): Record<string, number> {
  return { ...counts, [id]: (counts[id] ?? 0) + by };
}

export function logWith(log: GameLog, counter: SummedCounter, id: string, by = 1): GameLog {
  return { ...log, [counter]: bump(log[counter], id, by) };
}

/** R1743000: raises a card's `counter` to `value` if that is more than it holds; else the log itself. */
export function logMax(log: GameLog, counter: MaxCounter, id: string, value: number): GameLog {
  if (value <= (log[counter][id] ?? 0)) return log;
  return { ...log, [counter]: { ...log[counter], [id]: value } };
}

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

export function logIsEmpty(log: GameLog): boolean {
  return (
    log.seen.length === 0 &&
    log.turns === 0 &&
    [...SUMMED_COUNTERS, ...MAX_COUNTERS].every((counter) => Object.keys(log[counter]).length === 0)
  );
}

export function addGame(stats: PlayerStats, log: GameLog, outcome: GameOutcome): PlayerStats {
  const cards: Record<string, CardCounters> = { ...stats.cards };
  const change = (id: string, counter: CardCounter, to: (held: number) => number): void => {
    const current = cards[id] ?? NO_COUNTS;
    cards[id] = { ...current, [counter]: to(current[counter]) };
  };
  const add = (id: string, counter: CardCounter, by: number): void => {
    change(id, counter, (held) => held + by);
  };
  for (const id of log.seen) add(id, "seen", 1);
  for (const counter of SUMMED_COUNTERS) {
    for (const [id, n] of Object.entries(log[counter])) add(id, counter, n);
  }
  for (const counter of MAX_COUNTERS) {
    for (const [id, n] of Object.entries(log[counter])) change(id, counter, (held) => Math.max(held, n));
  }
  // R1743000: a game won or lost counts once for each card the player played in it; a draw for neither.
  if (outcome !== "draw") {
    for (const id of Object.keys(log.played)) add(id, outcome === "win" ? "playedInWin" : "playedInLoss", 1);
  }
  const streak = outcome === "win" ? stats.streak + 1 : 0;
  const longer = log.turns > stats.longestGame;
  return {
    games: stats.games + 1,
    wins: stats.wins + (outcome === "win" ? 1 : 0),
    losses: stats.losses + (outcome === "loss" ? 1 : 0),
    draws: stats.draws + (outcome === "draw" ? 1 : 0),
    cards,
    streak,
    bestStreak: Math.max(stats.bestStreak, streak),
    longestGame: longer ? log.turns : stats.longestGame,
    longestGamePlayed: longer ? log.played : stats.longestGamePlayed,
  };
}

/** Null before the first game. */
export function winPercent(stats: PlayerStats): number | null {
  return stats.games === 0 ? null : Math.round((stats.wins / stats.games) * 100);
}

export type CardTally = { readonly id: string; readonly count: number };

/** Highest counts first, then id for stable rendering; exclude zeros. */
export function topCards(stats: PlayerStats, counter: CardCounter, limit: number): CardTally[] {
  return Object.entries(stats.cards)
    .map(([id, counts]) => ({ id, count: counts[counter] }))
    .filter((entry) => entry.count > 0)
    .sort((a, b) => b.count - a.count || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .slice(0, Math.max(0, limit));
}

/** SPEC §9.11, R377: a server-calculated played delta as signed percentage points. */
export function formatPlayedDelta(delta: number | null): string | null {
  if (delta === null) return null;
  const points = Math.round(delta * 100);
  const sign = points > 0 ? "+" : "";
  return `${sign}${points}%`;
}

/**
 * R654, R1147000: a server-calculated rate to show, null when none or below the sample floor: the cards
 * table's Radiant and Normal rates, and the Created cards' (R2089000).
 */
export function faceRate(
  games: number | undefined,
  rate: number | null | undefined,
  minSample: number,
): number | null {
  if (games === undefined || rate === undefined || rate === null || games < minSample) {
    return null;
  }
  return rate;
}
