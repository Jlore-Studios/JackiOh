// The device's copy of the player's statistics (SPEC R639), kept the way the tutorial's progress is
// (tutorial/progress.ts) and the settings are (settings/store.ts):
//
//  - `localStorage[PLAYER_STATS_KEY]` holds `{ v: 1, games, wins, losses, draws, cards }`. Nothing
//    here talks to a server: a statistic is read off views the player was already shown (track.ts),
//    and no record of it leaves the device, so there is nothing for the server to hold or to leak.
//  - `localStorage` is untrusted and may be missing. Private windows, blocked site data and sandboxed
//    frames make it throw on access, and a hand-edited value can hold anything. Every access sits in
//    try/catch: a failed read means "no statistics", a failed write keeps the in-memory value.
//  - The parse is tolerant: a value of the wrong shape or version is no statistics, and a count that
//    is not a whole, non-negative number reads as 0.
//  - A game is added to the totals as stored, not as this tab last saw them, so two tabs finishing a
//    game each add theirs. A `storage` event (another tab wrote) re-reads the key and re-renders.

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

/** The cached snapshot; `null` until the first read, and again after the test seam. */
let snapshot: PlayerStats | null = null;

const listeners = new Set<() => void>();
let storageListening = false;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

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

/**
 * Tolerant: anything but `{ v: PLAYER_STATS_VERSION, ... }` is no statistics. A JSON string is
 * parsed first. Never throws.
 */
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

function storageOrNull(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

function loadStored(): PlayerStats {
  try {
    const raw = storageOrNull()?.getItem(PLAYER_STATS_KEY) ?? null;
    return raw === null ? EMPTY_STATS : parsePlayerStats(raw);
  } catch {
    return EMPTY_STATS;
  }
}

function persist(stats: PlayerStats): void {
  try {
    storageOrNull()?.setItem(PLAYER_STATS_KEY, JSON.stringify({ v: PLAYER_STATS_VERSION, ...stats }));
  } catch {
    // Quota, private mode or blocked storage: the in-memory value stays in force.
  }
}

function notify(): void {
  for (const listener of [...listeners]) listener();
}

/** The current snapshot: the same object until something changes. Reads storage on first call. */
export function readPlayerStats(): PlayerStats {
  if (snapshot === null) snapshot = loadStored();
  return snapshot;
}

/** R639: fold one finished game into the totals, as stored, and tell the subscribers. */
export function recordGame(log: GameLog, outcome: GameOutcome): PlayerStats {
  const next = addGame(loadStored(), log, outcome);
  snapshot = next;
  persist(next);
  notify();
  return next;
}

/** Forget every statistic on this device. */
export function resetPlayerStats(): PlayerStats {
  snapshot = EMPTY_STATS;
  persist(EMPTY_STATS);
  notify();
  return EMPTY_STATS;
}

/** Another tab wrote the key, or cleared storage (`key === null`). */
function onStorage(event: StorageEvent): void {
  if (typeof event.key === "string" && event.key !== PLAYER_STATS_KEY) return;
  snapshot = typeof event.newValue === "string" ? parsePlayerStats(event.newValue) : loadStored();
  notify();
}

/** The `storage` listener is on `window` only while someone is subscribed. */
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

/** The player's statistics, live: a finished game re-renders whoever reads them. */
export function usePlayerStats(): PlayerStats {
  return useSyncExternalStore(subscribePlayerStats, readPlayerStats, () => EMPTY_STATS);
}

/** Test seam: drop the cached snapshot so the next read goes back to storage. */
export function dropPlayerStatsCache(): void {
  snapshot = null;
}
