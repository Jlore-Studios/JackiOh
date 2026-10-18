// Player statistics sync with the account (SPEC §9.11, R654, R1142000).
//
// When an active session is present, syncs the device's tracked player statistics
// and privacy preference to PUT /api/stats/player. Pruned to the summary's inputs
// (R1142000) and bounded by PLAYER_STATS_BYTES_MAX.

import { putPlayerStats } from "../net/api.ts";
import { readSession } from "../net/session.ts";
import { readSettings } from "../settings/store.ts";
import { PLAYER_STATS_BYTES_MAX } from "../wire/serverConfig.ts";
import type { PlayerStats } from "./model.ts";
import { readPlayerStats } from "./store.ts";

export type PrunedCardCounters = {
  readonly played?: number;
  readonly playedAgainst?: number;
  readonly destroyed?: number;
  readonly defeated?: number;
};

export type SyncedPlayerStats = {
  readonly games: number;
  readonly wins: number;
  readonly losses: number;
  readonly draws: number;
  readonly cards: Readonly<Record<string, PrunedCardCounters>>;
};

type CandidateCard = {
  readonly id: string;
  readonly counters: PrunedCardCounters;
  readonly total: number;
};

/**
 * Prunes local PlayerStats for syncing to the server account copy (R1142000).
 *
 * Keeps game totals (games, wins, losses, draws) and only the four counters
 * read by the public player summary (played, playedAgainst, destroyed, defeated).
 * Omits seen, zero counters, and all-zero cards.
 * If the resulting PUT body exceeds maxBytes, drops lowest-count cards first until it fits.
 */
export function syncPayload(
  stats: PlayerStats,
  maxBytes: number = PLAYER_STATS_BYTES_MAX,
): SyncedPlayerStats {
  const SUMMARY_KEYS = ["played", "playedAgainst", "destroyed", "defeated"] as const;

  const candidates: CandidateCard[] = [];

  for (const [id, raw] of Object.entries(stats.cards)) {
    if (!raw || typeof raw !== "object") continue;
    const counters: Record<string, number> = {};
    let total = 0;

    for (const key of SUMMARY_KEYS) {
      const val = raw[key];
      if (typeof val === "number" && Number.isSafeInteger(val) && val > 0) {
        counters[key] = val;
        total += val;
      }
    }

    if (total > 0) {
      candidates.push({ id, counters: Object.freeze(counters), total });
    }
  }

  // Highest activity first; break ties by card id ascending
  candidates.sort((a, b) => b.total - a.total || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));

  const buildCards = (list: readonly CandidateCard[]): Record<string, PrunedCardCounters> => {
    const map: Record<string, PrunedCardCounters> = {};
    for (const item of list) {
      map[item.id] = item.counters;
    }
    return map;
  };

  const bodyFits = (map: Record<string, PrunedCardCounters>): boolean => {
    const body = {
      stats: {
        games: stats.games,
        wins: stats.wins,
        losses: stats.losses,
        draws: stats.draws,
        cards: map,
      },
      isPrivate: false,
    };
    return JSON.stringify(body).length <= maxBytes;
  };

  let activeList = candidates;
  let cardsMap = buildCards(activeList);

  while (activeList.length > 0 && !bodyFits(cardsMap)) {
    activeList = activeList.slice(0, -1);
    cardsMap = buildCards(activeList);
  }

  return {
    games: stats.games,
    wins: stats.wins,
    losses: stats.losses,
    draws: stats.draws,
    cards: Object.freeze(cardsMap),
  };
}

/**
 * Sends the device's player stats and privacy preference to the server.
 * Safe to call frequently: if not signed in or network is unreachable, it silently completes.
 */
export async function syncPlayerStats(tokenOverride?: string): Promise<void> {
  const token = tokenOverride ?? readSession()?.accessToken;
  if (!token) return;

  const stats = readPlayerStats();
  const settings = readSettings();
  const isPrivate = !settings.publicStats;

  try {
    await putPlayerStats(token, {
      stats: syncPayload(stats) as unknown as Record<string, unknown>,
      isPrivate,
    });
  } catch {
    // Network or server unreachable: device retains authoritative local stats
  }
}
