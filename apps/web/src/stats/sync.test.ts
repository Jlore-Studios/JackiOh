// Player statistics sync tests (SPEC §9.11, R654, R1142000, R2000000).

import catalogJson from "@jackioh/cards/catalog.json";
import { setShips, type SetName } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { putPlayerStats } from "../net/api.ts";
import { clearSession, writeSession } from "../net/session.ts";
import { SETTINGS_STORAGE_KEY, __resetSettingsForTests, resetSettings } from "../settings/store.ts";
import { PLAYER_STATS_BYTES_MAX } from "../wire/serverConfig.ts";
import { PLAYER_STATS_KEY, PLAYER_STATS_VERSION } from "./config.ts";
import { EMPTY_STATS, NO_COUNTS, type CardCounters, type PlayerStats } from "./model.ts";
import { dropPlayerStatsCache } from "./store.ts";
import { syncPayload, syncPlayerStats } from "./sync.ts";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, putPlayerStats: vi.fn() };
});

function computeSummary(cards: Record<string, { played?: number; playedAgainst?: number; destroyed?: number; defeated?: number } | undefined>) {
  const playedCounts: { id: string; count: number }[] = [];
  const nemesisCounts: { id: string; count: number }[] = [];
  let totalDestroyed = 0;
  let totalDefeated = 0;

  for (const [id, counters] of Object.entries(cards)) {
    if (!counters) continue;
    const played = counters.played ?? 0;
    const playedAgainst = counters.playedAgainst ?? 0;
    const destroyed = counters.destroyed ?? 0;
    const defeated = counters.defeated ?? 0;

    if (played > 0) playedCounts.push({ id, count: played });
    if (playedAgainst > 0) nemesisCounts.push({ id, count: playedAgainst });
    totalDestroyed += destroyed;
    totalDefeated += defeated;
  }

  playedCounts.sort((a, b) => b.count - a.count || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  nemesisCounts.sort((a, b) => b.count - a.count || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));

  return {
    topPlayed: playedCounts.slice(0, 3),
    nemesis: nemesisCounts[0]?.id ?? null,
    totalDestroyed,
    totalDefeated,
  };
}

function setStoredStats(stats: PlayerStats): void {
  window.localStorage.setItem(PLAYER_STATS_KEY, JSON.stringify({ v: PLAYER_STATS_VERSION, ...stats }));
  dropPlayerStatsCache();
}

describe("R1142000 syncPlayerStats and syncPayload", () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
    resetSettings();
    dropPlayerStatsCache();
    clearSession();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("R1142000 builds stats holding every catalog id with all five counters at 9999 and fits within PLAYER_STATS_BYTES_MAX", () => {
    // Every card a player can hold counters on: the sets that ship (R1420). A held set's entries
    // reach no game, so no device counts them (as the Training Grounds set's did until #2204).
    const catalogIds = Object.entries(catalogJson)
      .filter(([, def]) => setShips((def as { set: SetName }).set))
      .map(([id]) => id);
    expect(catalogIds.length).toBeGreaterThanOrEqual(770);

    const cards: Record<string, CardCounters> = {};
    for (const id of catalogIds) {
      cards[id] = {
        ...NO_COUNTS,
        seen: 9999,
        played: 9999,
        playedAgainst: 9999,
        destroyed: 9999,
        defeated: 9999,
      };
    }

    const fullStats: PlayerStats = {
      ...EMPTY_STATS,
      games: 1000,
      wins: 700,
      losses: 250,
      draws: 50,
      cards,
    };

    const payload = syncPayload(fullStats);
    const putBody = { stats: payload };
    const stringifiedLength = JSON.stringify(putBody).length;

    // Must fit under PLAYER_STATS_BYTES_MAX
    expect(stringifiedLength).toBeLessThanOrEqual(PLAYER_STATS_BYTES_MAX);
    // Every catalog card is included because all of them fit under the cap
    expect(Object.keys(payload.cards).length).toBe(catalogIds.length);

    // Seen is omitted
    const firstId = catalogIds[0] ?? "core-001";
    expect(payload.cards[firstId]).not.toHaveProperty("seen");
    expect(payload.cards[firstId]).toEqual({
      played: 9999,
      playedAgainst: 9999,
      destroyed: 9999,
      defeated: 9999,
    });
  });

  it("R1142000 250-card stats yields a payload under the cap with identical summary figures", () => {
    const catalogIds = Object.keys(catalogJson).slice(0, 250);
    expect(catalogIds.length).toBe(250);

    const cards: Record<string, CardCounters> = {};
    catalogIds.forEach((id, i) => {
      cards[id] = {
        ...NO_COUNTS,
        seen: 50,
        played: (i * 7) % 100,
        playedAgainst: (i * 13) % 80,
        destroyed: (i * 3) % 40,
        defeated: (i * 5) % 50,
      };
    });

    const stats: PlayerStats = {
      ...EMPTY_STATS,
      games: 250,
      wins: 150,
      losses: 90,
      draws: 10,
      cards,
    };

    // Full device stats with 250 cards and all 5 counters exceeds the old 16384 cap
    const deviceBodyLength = JSON.stringify({ stats }).length;
    expect(deviceBodyLength).toBeGreaterThan(16384);

    const payload = syncPayload(stats);
    const putBody = { stats: payload };
    expect(JSON.stringify(putBody).length).toBeLessThanOrEqual(PLAYER_STATS_BYTES_MAX);

    // Summary computed over payload is identical to summary computed over full stats
    const originalSummary = computeSummary(stats.cards);
    const payloadSummary = computeSummary(payload.cards);

    expect(payloadSummary.topPlayed).toEqual(originalSummary.topPlayed);
    expect(payloadSummary.nemesis).toBe(originalSummary.nemesis);
    expect(payloadSummary.totalDestroyed).toBe(originalSummary.totalDestroyed);
    expect(payloadSummary.totalDefeated).toBe(originalSummary.totalDefeated);
  });

  it("R1142000 drops lowest-count cards first when cap is too small for payload", () => {
    const stats: PlayerStats = {
      ...EMPTY_STATS,
      games: 10,
      wins: 8,
      losses: 2,
      draws: 0,
      cards: {
        "card-a": { ...NO_COUNTS, seen: 10, played: 100 },
        "card-b": { ...NO_COUNTS, seen: 5, played: 50 },
        "card-c": { ...NO_COUNTS, seen: 2, played: 10 },
        "card-d": { ...NO_COUNTS, seen: 1, played: 1 },
      },
    };

    // Measure length with card-a and card-b only
    const twoCardsBody = {
      stats: {
        games: 10,
        wins: 8,
        losses: 2,
        draws: 0,
        cards: {
          "card-a": { played: 100 },
          "card-b": { played: 50 },
        },
      },
    };
    const tightCap = JSON.stringify(twoCardsBody).length;

    const payload = syncPayload(stats, tightCap);
    const putBody = { stats: payload };
    expect(JSON.stringify(putBody).length).toBeLessThanOrEqual(tightCap);

    // card-d (count 1) and card-c (count 10) were dropped first; card-a (100) and card-b (50) remain
    expect(payload.cards).toHaveProperty("card-a");
    expect(payload.cards).toHaveProperty("card-b");
    expect(payload.cards).not.toHaveProperty("card-c");
    expect(payload.cards).not.toHaveProperty("card-d");
  });

  it("R1142000 omits seen, zero counters, and all-zero cards from sync payload", () => {
    const stats: PlayerStats = {
      ...EMPTY_STATS,
      games: 5,
      wins: 3,
      losses: 2,
      draws: 0,
      cards: {
        "card-only-seen": { ...NO_COUNTS, seen: 10 },
        "card-zeros": { ...NO_COUNTS },
        "card-mixed": { ...NO_COUNTS, seen: 20, played: 5, destroyed: 2 },
      },
    };

    const payload = syncPayload(stats);

    // All-zero cards or cards with only seen are excluded completely
    expect(payload.cards).not.toHaveProperty("card-only-seen");
    expect(payload.cards).not.toHaveProperty("card-zeros");

    // Mixed card keeps only non-zero summary counters and drops seen
    expect(payload.cards["card-mixed"]).toEqual({
      played: 5,
      destroyed: 2,
    });
  });

  it("R1142000 R2000000 syncPlayerStats calls putPlayerStats with the pruned payload and no isPrivate", async () => {
    writeSession({
      accessToken: "test-token-123",
      refreshToken: "refresh-123",
      expiresAt: Date.now() + 60000,
    });

    setStoredStats({
      ...EMPTY_STATS,
      games: 1,
      wins: 1,
      losses: 0,
      draws: 0,
      cards: {
        "core-001": { ...NO_COUNTS, seen: 5, played: 2, destroyed: 1 },
      },
    });

    await syncPlayerStats();

    expect(putPlayerStats).toHaveBeenCalledTimes(1);
    expect(putPlayerStats).toHaveBeenCalledWith("test-token-123", {
      stats: {
        games: 1,
        wins: 1,
        losses: 0,
        draws: 0,
        cards: {
          "core-001": { played: 2, destroyed: 1 },
        },
      },
    });
    expect(vi.mocked(putPlayerStats).mock.calls[0]?.[1]).not.toHaveProperty("isPrivate");
  });

  it("R2000000 a publicStats false an older device stored sends no isPrivate, and a failed request is swallowed", async () => {
    writeSession({
      accessToken: "token-private",
      refreshToken: "refresh-123",
      expiresAt: Date.now() + 60000,
    });

    window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify({ publicStats: false }));
    __resetSettingsForTests();

    setStoredStats({
      ...EMPTY_STATS,
      games: 2,
      wins: 1,
      losses: 1,
      draws: 0,
      cards: {},
    });

    vi.mocked(putPlayerStats).mockRejectedValueOnce(new Error("Network error"));

    // Should not throw
    await expect(syncPlayerStats()).resolves.toBeUndefined();

    expect(putPlayerStats).toHaveBeenCalledWith("token-private", {
      stats: {
        games: 2,
        wins: 1,
        losses: 1,
        draws: 0,
        cards: {},
      },
    });
    expect(vi.mocked(putPlayerStats).mock.calls[0]?.[1]).not.toHaveProperty("isPrivate");
  });

  it("R1142000 syncPlayerStats does nothing when not signed in", async () => {
    await syncPlayerStats();
    expect(putPlayerStats).not.toHaveBeenCalled();
  });

  it("R1582000 the new counters stay on the device: the sync payload leaves them out", () => {
    const stats: PlayerStats = {
      ...EMPTY_STATS,
      games: 2,
      wins: 1,
      losses: 1,
      draws: 0,
      cards: {
        "card-a": { ...NO_COUNTS, played: 1, allured: 2, lethal: 3, sprung: 4, survived: 5 },
        "card-b": { ...NO_COUNTS, sprung: 7 },
      },
    };
    const payload = syncPayload(stats);
    expect(payload.cards).toEqual({ "card-a": { played: 1 } });
  });

  it("R1743000 the new counters and totals are not sent: the payload and the request body leave them out", async () => {
    const stats: PlayerStats = {
      ...EMPTY_STATS,
      games: 3,
      wins: 2,
      losses: 1,
      draws: 0,
      cards: {
        "card-a": {
          ...NO_COUNTS,
          played: 4,
          damageDealt: 30,
          biggestHit: 9,
          overkill: 4,
          attacks: 8,
          healing: 12,
          drawn: 6,
          radiant: 2,
          playedInWin: 3,
          playedInLoss: 1,
        },
        "card-b": { ...NO_COUNTS, damageDealt: 5, drawn: 2, playedInWin: 1 },
      },
      streak: 2,
      bestStreak: 5,
      longestGame: 21,
      longestGamePlayed: { "card-a": 3 },
    };
    const payload = syncPayload(stats);
    expect(payload).toEqual({ games: 3, wins: 2, losses: 1, draws: 0, cards: { "card-a": { played: 4 } } });
    expect(Object.keys(payload).sort()).toEqual(["cards", "draws", "games", "losses", "wins"]);

    writeSession({ accessToken: "token-new", refreshToken: "refresh-new", expiresAt: Date.now() + 60000 });
    setStoredStats(stats);
    await syncPlayerStats();
    const sent = JSON.stringify(vi.mocked(putPlayerStats).mock.calls);
    for (const word of ["streak", "bestStreak", "longestGame", "damageDealt", "biggestHit", "overkill", "attacks", "healing", "drawn", "radiant", "playedInWin", "playedInLoss"]) {
      expect(sent).not.toContain(word);
    }
  });

  it("R2649000 the new counters are not sent: the payload and the request body leave them out", async () => {
    const NEW = [
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
    const every = Object.fromEntries(NEW.map((counter, i) => [counter, i + 1])) as Partial<CardCounters>;
    const stats: PlayerStats = {
      ...EMPTY_STATS,
      games: 2,
      wins: 1,
      losses: 1,
      draws: 0,
      cards: { "card-a": { ...NO_COUNTS, ...every, played: 4 }, "card-b": { ...NO_COUNTS, ...every } },
    };
    const payload = syncPayload(stats);
    expect(payload).toEqual({ games: 2, wins: 1, losses: 1, draws: 0, cards: { "card-a": { played: 4 } } });

    writeSession({ accessToken: "token-new", refreshToken: "refresh-new", expiresAt: Date.now() + 60000 });
    setStoredStats(stats);
    await syncPlayerStats();
    expect(putPlayerStats).toHaveBeenCalled();
    const sent = JSON.stringify(vi.mocked(putPlayerStats).mock.calls);
    for (const word of NEW) expect(sent).not.toContain(word);
  });
});
