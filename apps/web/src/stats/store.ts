// Device statistics (SPEC R639) are optional, untrusted localStorage: malformed data is empty and
// failed persistence stays in memory. Re-read before writes so concurrent tabs retain both games.

import { useSyncExternalStore } from "react";

import { PLAYER_STATS_KEY, PLAYER_STATS_VERSION } from "./config.ts";
import {
  CARD_COUNTERS,
  EMPTY_STATS,
  addGame,
  type CardCounters,
  type GameLog,
  type GameOutcome,
  type PlayerStats,
} from "./model.ts";
import { isRecord } from "../wire/json.ts";
import { localStorageOrNull } from "../net/storage.ts";

let snapshot: PlayerStats | null = null;

const listeners = new Set<() => void>();
let storageListening = false;

function count(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0 ? value : 0;
}

function parseCounters(raw: unknown): CardCounters | null {
  if (!isRecord(raw)) return null;
  const counters = Object.fromEntries(CARD_COUNTERS.map((counter) => [counter, count(raw[counter])])) as Record<
    (typeof CARD_COUNTERS)[number],
    number
  >;
  return CARD_COUNTERS.every((counter) => counters[counter] === 0) ? null : Object.freeze(counters);
}

/** Tolerant parsing: malformed input becomes empty statistics. */
export function parsePlayerStats(raw: unknown): PlayerStats {
  try {
    let value: unknown = raw;
    if (typeof value === "string") {
      try {
        value = JSON.parse(value) as unknown;
      } catch {
        return EMPTY_STATS;
      }
    }
    if (!isRecord(value) || value.v !== PLAYER_STATS_VERSION) return EMPTY_STATS;
    const cards: Record<string, CardCounters> = {};
    if (isRecord(value.cards)) {
      for (const [id, counters] of Object.entries(value.cards)) {
        const parsed = parseCounters(counters);
        if (parsed !== null) cards[id] = parsed;
      }
    }
    return Object.freeze({
      games: count(value.games),
      wins: count(value.wins),
      losses: count(value.losses),
      draws: count(value.draws),
      cards: Object.freeze(cards),
    });
  } catch {
    return EMPTY_STATS;
  }
}

function loadStored(): PlayerStats {
  try {
    const raw = localStorageOrNull()?.getItem(PLAYER_STATS_KEY) ?? null;
    return raw === null ? EMPTY_STATS : parsePlayerStats(raw);
  } catch {
    return EMPTY_STATS;
  }
}

function persist(stats: PlayerStats): void {
  try {
    localStorageOrNull()?.setItem(PLAYER_STATS_KEY, JSON.stringify({ v: PLAYER_STATS_VERSION, ...stats }));
  } catch {
    // Storage unavailable: retain the in-memory value.
  }
}

function notify(): void {
  for (const listener of [...listeners]) listener();
}

export function readPlayerStats(): PlayerStats {
  if (snapshot === null) snapshot = loadStored();
  return snapshot;
}

/** R639: fold a finished game into stored totals. */
export function recordGame(log: GameLog, outcome: GameOutcome): PlayerStats {
  const next = addGame(loadStored(), log, outcome);
  snapshot = next;
  persist(next);
  notify();
  return next;
}

export function resetPlayerStats(): PlayerStats {
  snapshot = EMPTY_STATS;
  persist(EMPTY_STATS);
  notify();
  return EMPTY_STATS;
}

function onStorage(event: StorageEvent): void {
  if (typeof event.key === "string" && event.key !== PLAYER_STATS_KEY) return;
  snapshot = typeof event.newValue === "string" ? parsePlayerStats(event.newValue) : loadStored();
  notify();
}

export function subscribePlayerStats(listener: () => void): () => void {
  listeners.add(listener);
  if (!storageListening) {
    window.addEventListener("storage", onStorage);
    storageListening = true;
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && storageListening) {
      window.removeEventListener("storage", onStorage);
      storageListening = false;
    }
  };
}

export function usePlayerStats(): PlayerStats {
  return useSyncExternalStore(subscribePlayerStats, readPlayerStats, () => EMPTY_STATS);
}

export function dropPlayerStatsCache(): void {
  snapshot = null;
}
