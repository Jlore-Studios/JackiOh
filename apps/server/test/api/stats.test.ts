/**
 * Public card and player statistics page tests (SPEC §9.11, R641).
 */

import { beforeEach, describe, expect, it } from "vitest";

import type { CardDef, GameRecord } from "@jackioh/shared";
import { createRouter, type Router } from "../../src/api/http";
import type { PublicPlayerSummary } from "../../src/api/ports";
import { createStatsRoutes, type CardDrillDownResponse, type PublicStatsCardsResponse } from "../../src/api/stats";
import {
  CARD_STATS_CACHE_TTL_SECONDS,
  CARD_STATS_MIN_SAMPLE,
  PLAYER_STATS_BYTES_MAX,
  PLAYER_STATS_CACHE_TTL_SECONDS,
  PUBLIC_STATS_MIN_LIVE_GAMES,
} from "../../src/config";
import { createTestCatalog, createTestDeps, jsonRequest, readJson, type TestDeps } from "../fakes/deps";

const PATCH = "v0.2.0";

let deps: TestDeps;
let router: Router;
let userToken: string;
let pendingToken: string;

function makeGameRecord(
  id: string,
  options: {
    source?: "live" | "dev";
    mode?: "bo1" | "bo3" | "random" | "tutorial";
    patch?: string;
    p1Deck?: string[];
    p2Deck?: string[];
    winner?: "p1" | "p2" | "draw";
    p1Played?: string[];
    p2Played?: string[];
    p1PlayedTurns?: number[];
    p2PlayedTurns?: number[];
    turns?: number;
  } = {},
): GameRecord {
  const p1Deck = options.p1Deck ?? ["core-001", "core-002"];
  const p2Deck = options.p2Deck ?? ["core-003", "core-004"];
  const source = options.source ?? "live";
  const patch = options.patch ?? PATCH;
  const recordId = source === "dev" ? (id.startsWith("dev:") ? id : `dev:${patch}:${id}`) : id;

  const p1Played = options.p1Played ?? p1Deck.slice(0, 2);
  const p2Played = options.p2Played ?? p2Deck.slice(0, 2);

  return {
    id: recordId,
    source,
    mode: (options.mode ?? "bo1") as GameRecord["mode"],
    patch,
    pilots: { p1: source === "dev" ? "ai" : "human", p2: source === "dev" ? "ai" : "human" },
    game: {
      first: "p1",
      winner: options.winner ?? "p1",
      reason: "hero-death",
      turns: options.turns ?? 5,
      seats: {
        p1: {
          deck: p1Deck,
          opening: p1Deck.slice(0, 3),
          drawn: p1Deck.slice(3),
          played: p1Played,
          playedTurns: options.p1PlayedTurns ?? p1Played.map((_, idx) => idx + 1),
        },
        p2: {
          deck: p2Deck,
          opening: p2Deck.slice(0, 3),
          drawn: p2Deck.slice(3),
          played: p2Played,
          playedTurns: options.p2PlayedTurns ?? p2Played.map((_, idx) => idx + 1),
        },
      },
    },
  };
}

beforeEach(() => {
  const testCatalog = createTestCatalog();
  testCatalog.defs = {
    "core-001": {
      id: "core-001",
      name: "Big D-fender",
      cost: 2,
      rarity: "Common",
      set: "Core",
      type: "Unit",
      attack: 1,
      health: 4,
      text: "Taunt",
    } as unknown as CardDef,
    "core-002": {
      id: "core-002",
      name: "Tempo Timmy",
      cost: 3,
      rarity: "Rare",
      set: "Core",
      type: "Unit",
      attack: 3,
      health: 3,
      text: "Battlecry",
    } as unknown as CardDef,
    "core-003": {
      id: "core-003",
      name: "Flood",
      cost: 4,
      rarity: "Epic",
      set: "Core",
      type: "Spell",
      text: "Deal 2 damage to all units",
    } as unknown as CardDef,
    "core-004": {
      id: "core-004",
      name: " Gary the Gambler",
      cost: 5,
      rarity: "Legendary",
      set: "Core",
      type: "Unit",
      attack: 5,
      health: 5,
      text: "Chaos",
    } as unknown as CardDef,
    "core-005": {
      id: "core-005",
      name: "High Roller",
      cost: 6,
      rarity: "Epic",
      set: "Core",
      type: "Unit",
      attack: 6,
      health: 6,
      text: "Big",
    } as unknown as CardDef,
    "core-006": {
      id: "core-006",
      name: "Colossus",
      cost: 10,
      rarity: "Legendary",
      set: "Core",
      type: "Unit",
      attack: 10,
      health: 10,
      text: "Colossal",
    } as unknown as CardDef,
  };

  deps = createTestDeps({ catalog: testCatalog, games: { patch: PATCH, summarize: () => null } });
  router = createRouter(createStatsRoutes(), deps);

  deps.store.seedProfile({ id: "p-active", userId: "u-active", status: "active", displayName: "Alice" });
  userToken = deps.auth.addUser({ userId: "u-active", email: "alice@example.test" });

  deps.store.seedProfile({ id: "p-pending", userId: "u-pending", status: "pending", displayName: "Bob" });
  pendingToken = deps.auth.addUser({ userId: "u-pending", email: "bob@example.test" });
});

describe("R641: public card and player stats", () => {
  it("R641 GET /api/stats/cards pads with AI development runs below 1000 live games (provisional)", async () => {
    // 999 live games and 50 dev games
    for (let i = 1; i <= 999; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`live-${i}`, {
          source: "live",
          p1Deck: ["core-001", "core-002"],
          winner: i % 2 === 0 ? "p1" : "p2",
        }),
      );
    }
    for (let i = 1; i <= 50; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`dev-${i}`, {
          source: "dev",
          p1Deck: ["core-001", "core-002"],
          winner: "p1",
        }),
      );
    }
    // 5 tutorial games (must be excluded from gate and figures)
    for (let i = 1; i <= 5; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`tut-${i}`, {
          source: "live",
          mode: "tutorial",
          p1Deck: ["core-001"],
        }),
      );
    }

    const res = await router(jsonRequest("GET", "/api/stats/cards"));
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe(`public, max-age=${String(CARD_STATS_CACHE_TTL_SECONDS)}`);

    const data = await readJson<PublicStatsCardsResponse>(res);
    expect(data.gate.cleared).toBe(false);
    expect(data.gate.liveGames).toBe(999);
    expect(data.gate.minLiveGames).toBe(PUBLIC_STATS_MIN_LIVE_GAMES);
    expect(data.source).toBe("provisional");
    expect(data.sourceLabel).toBe("AI games + live games (provisional)");
    expect(data.minSample).toBe(CARD_STATS_MIN_SAMPLE);

    // AI games are included below 1000 live games (999 live + 50 dev = 1049 games; tutorial excluded)
    expect(data.totalGames).toBe(1049);
    const card001 = data.cards.find((c) => c.id === "core-001");
    expect(card001).toBeDefined();
    expect(card001?.games).toBe(1049);
    expect(card001?.hasEnoughGames).toBe(true);
  });

  it("R641 GET /api/stats/cards switches strictly to live games only at exactly 1000 live games", async () => {
    // Exactly 1000 live games and 100 dev games
    for (let i = 1; i <= 1000; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`live-${i}`, {
          source: "live",
          p1Deck: ["core-001", "core-002"],
          winner: "p1",
        }),
      );
    }
    for (let i = 1; i <= 100; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`dev-${i}`, {
          source: "dev",
          p1Deck: ["core-001", "core-002"],
          winner: "p2",
        }),
      );
    }

    const res = await router(jsonRequest("GET", "/api/stats/cards"));
    expect(res.status).toBe(200);

    const data = await readJson<PublicStatsCardsResponse>(res);
    expect(data.gate.cleared).toBe(true);
    expect(data.gate.liveGames).toBe(1000);
    expect(data.source).toBe("live");
    expect(data.sourceLabel).toBe("Live games");

    // At/above 1000 live games, AI dev games are strictly ignored (totalGames is exactly 1000 live)
    expect(data.totalGames).toBe(1000);
    const card001 = data.cards.find((c) => c.id === "core-001");
    expect(card001?.games).toBe(1000);
    expect(card001?.winRate).toBe(1); // all 1000 live games won by p1
  });

  it("R641 respects sample floor CARD_STATS_MIN_SAMPLE (20) for win-rate and best/worst summary", async () => {
    // 19 games with core-001, 25 games with core-002
    for (let i = 1; i <= 19; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`rec-19-${i}`, {
          source: "live",
          p1Deck: ["core-001"],
          winner: "p1",
        }),
      );
    }
    for (let i = 1; i <= 25; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`rec-25-${i}`, {
          source: "live",
          p1Deck: ["core-002"],
          winner: i <= 15 ? "p1" : "p2",
        }),
      );
    }

    const res = await router(jsonRequest("GET", "/api/stats/cards"));
    const data = await readJson<PublicStatsCardsResponse>(res);

    const c1 = data.cards.find((c) => c.id === "core-001");
    const c2 = data.cards.find((c) => c.id === "core-002");

    expect(c1?.games).toBe(19);
    expect(c1?.hasEnoughGames).toBe(false); // below 20

    expect(c2?.games).toBe(25);
    expect(c2?.hasEnoughGames).toBe(true); // >= 20
    expect(c2?.winRate).toBe(15 / 25);

    // Summary best/worst card only considers cards with hasEnoughGames === true
    // core-001 has 100% win rate (19/19) but is excluded because games < 20
    expect(data.summary.bestCard?.id).toBe("core-002");
    expect(data.summary.bestCard?.winRate).toBe(15 / 25);
    expect(data.summary.worstCard?.winRate).toBe(10 / 44);
  });

  it("R641 filters cards by set, rarity, cost, source, and card id", async () => {
    const resAll = await router(jsonRequest("GET", "/api/stats/cards"));
    const all = await readJson<PublicStatsCardsResponse>(resAll);
    expect(all.cards.length).toBe(6);

    const resSet = await router(jsonRequest("GET", "/api/stats/cards?set=Core"));
    const setCards = await readJson<PublicStatsCardsResponse>(resSet);
    expect(setCards.cards.length).toBe(6);

    const resRarity = await router(jsonRequest("GET", "/api/stats/cards?rarity=Legendary"));
    const rarityCards = await readJson<PublicStatsCardsResponse>(resRarity);
    expect(rarityCards.cards.map((c) => c.id).sort()).toEqual(["core-004", "core-006"]);

    const resCost = await router(jsonRequest("GET", "/api/stats/cards?cost=3"));
    const costCards = await readJson<PublicStatsCardsResponse>(resCost);
    expect(costCards.cards.map((c) => c.id)).toEqual(["core-002"]);

    // Cost 6+ bucket includes both 6-cost and 10-cost cards
    const resCost6 = await router(jsonRequest("GET", "/api/stats/cards?cost=6"));
    const cost6Cards = await readJson<PublicStatsCardsResponse>(resCost6);
    expect(cost6Cards.cards.map((c) => c.id).sort()).toEqual(["core-005", "core-006"]);

    const resCost6Plus = await router(jsonRequest("GET", "/api/stats/cards?cost=6%2B"));
    const cost6PlusCards = await readJson<PublicStatsCardsResponse>(resCost6Plus);
    expect(cost6PlusCards.cards.map((c) => c.id).sort()).toEqual(["core-005", "core-006"]);

    const resCard = await router(jsonRequest("GET", "/api/stats/cards?card=core-001"));
    const singleCard = await readJson<PublicStatsCardsResponse>(resCard);
    expect(singleCard.cards.map((c) => c.id)).toEqual(["core-001"]);

    // Source filtering
    const resLive = await router(jsonRequest("GET", "/api/stats/cards?source=live"));
    const liveStats = await readJson<PublicStatsCardsResponse>(resLive);
    expect(liveStats.source).toBe("live");
    expect(liveStats.sourceLabel).toBe("Live games");

    const resDev = await router(jsonRequest("GET", "/api/stats/cards?source=dev"));
    const devStats = await readJson<PublicStatsCardsResponse>(resDev);
    expect(devStats.source).toBe("dev");
    expect(devStats.sourceLabel).toBe("AI development games");

    const resProvisional = await router(jsonRequest("GET", "/api/stats/cards?source=provisional"));
    const provisionalStats = await readJson<PublicStatsCardsResponse>(resProvisional);
    expect(provisionalStats.source).toBe("provisional");
    expect(provisionalStats.sourceLabel).toBe("AI games + live games (provisional)");
  });

  it("R641 GET /api/stats/cards/:id returns drill-down with turn played and co-played cards", async () => {
    for (let i = 1; i <= 30; i++) {
      await deps.store.gameRecords.insert(
        makeGameRecord(`rec-drill-${i}`, {
          source: "live",
          p1Deck: ["core-001", "core-002"],
          p1Played: i % 2 === 0 ? ["core-002", "core-001"] : ["core-001"],
          p1PlayedTurns: i % 2 === 0 ? [2, 3] : [1],
          turns: 4,
          winner: i <= 20 ? "p1" : "p2",
        }),
      );
    }

    const res = await router(jsonRequest("GET", "/api/stats/cards/core-001"));
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe(`public, max-age=${String(CARD_STATS_CACHE_TTL_SECONDS)}`);

    const drill = await readJson<CardDrillDownResponse>(res);
    expect(drill.card.id).toBe("core-001");
    expect(drill.card.name).toBe("Big D-fender");

    // Co-played synergy cards
    const coPlayed = drill.coPlayed.find((c) => c.id === "core-002");
    expect(coPlayed).toBeDefined();
    expect(coPlayed?.games).toBe(30);

    // Turn played distribution: core-001 was played on turn 1 in 15 games, and turn 3 in 15 games
    const turn1 = drill.byTurn.find((t) => t.turn === 1);
    const turn3 = drill.byTurn.find((t) => t.turn === 3);
    expect(turn1).toBeDefined();
    expect(turn1?.games).toBe(15);
    expect(turn3).toBeDefined();
    expect(turn3?.games).toBe(15);

    // 404 on missing card
    const notFoundRes = await router(jsonRequest("GET", "/api/stats/cards/nonexistent-card"));
    expect(notFoundRes.status).toBe(404);
  });

  it("R641 GET and PUT /api/stats/player requires active auth, stores stats and privacy opt-out", async () => {
    // Unauthenticated requests
    const unauthGet = await router(jsonRequest("GET", "/api/stats/player"));
    expect(unauthGet.status).toBe(401);
    const unauthPut = await router(jsonRequest("PUT", "/api/stats/player", { stats: {} }));
    expect(unauthPut.status).toBe(401);

    // Pending account
    const pendingGet = await router(jsonRequest("GET", "/api/stats/player", undefined, { token: pendingToken }));
    expect(pendingGet.status).toBe(403);

    // Active account initially empty
    const initGet = await router(jsonRequest("GET", "/api/stats/player", undefined, { token: userToken }));
    expect(initGet.status).toBe(200);
    const initData = await readJson<{ stats: Record<string, unknown>; isPrivate: boolean }>(initGet);
    expect(initData.stats).toEqual({});
    expect(initData.isPrivate).toBe(false);

    // Save stats with privacy opt-out
    const sampleStats = {
      games: 15,
      wins: 10,
      losses: 5,
      cards: {
        "core-001": { played: 12, defeated: 4, destroyed: 1 },
      },
    };
    const putRes = await router(
      jsonRequest("PUT", "/api/stats/player", { stats: sampleStats, isPrivate: true }, { token: userToken }),
    );
    expect(putRes.status).toBe(200);
    const putData = await readJson<{ stats: Record<string, unknown>; isPrivate: boolean }>(putRes);
    expect(putData.stats).toEqual(sampleStats);
    expect(putData.isPrivate).toBe(true);

    // Verify GET reads it back
    const afterGet = await router(jsonRequest("GET", "/api/stats/player", undefined, { token: userToken }));
    const afterData = await readJson<{ stats: Record<string, unknown>; isPrivate: boolean }>(afterGet);
    expect(afterData.stats).toEqual(sampleStats);
    expect(afterData.isPrivate).toBe(true);

    // Rejects body exceeding PLAYER_STATS_BYTES_MAX
    const hugeStats = { data: "x".repeat(PLAYER_STATS_BYTES_MAX + 10) };
    const hugeRes = await router(
      jsonRequest("PUT", "/api/stats/player", { stats: hugeStats }, { token: userToken }),
    );
    expect(hugeRes.status).toBe(400);
  });

  it("R641 GET /api/stats/players lists public player summaries, respects privacy and search, omits Elo", async () => {
    // Setup profiles
    deps.store.seedProfile({ id: "p-alice", userId: "u-alice", status: "active", displayName: "Alice Wonder" });
    deps.store.seedProfile({ id: "p-bob", userId: "u-bob", status: "active", displayName: "Bob Builder" });
    deps.store.seedProfile({ id: "p-charlie", userId: "u-charlie", status: "active", displayName: "Charlie Secret" });

    // Alice: public, 50 games
    await deps.store.playerStats.put(
      "p-alice",
      {
        games: 50,
        wins: 30,
        losses: 20,
        cards: {
          "core-001": { played: 25, defeated: 3, destroyed: 2 },
          "core-002": { played: 15, defeated: 1, destroyed: 0 },
          "core-nemesis": { playedAgainst: 12 },
        },
      },
      false,
      1000,
    );

    // Bob: public, 100 games
    await deps.store.playerStats.put(
      "p-bob",
      {
        games: 100,
        wins: 70,
        losses: 30,
      },
      false,
      2000,
    );

    // Charlie: opted out (isPrivate: true), 200 games
    await deps.store.playerStats.put(
      "p-charlie",
      {
        games: 200,
        wins: 150,
      },
      true,
      3000,
    );

    const res = await router(jsonRequest("GET", "/api/stats/players"));
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe(`public, max-age=${String(PLAYER_STATS_CACHE_TTL_SECONDS)}`);

    const data = await readJson<{ players: PublicPlayerSummary[] }>(res);
    // Charlie is private, so only Bob and Alice appear
    expect(data.players.map((p) => p.profileId)).toEqual(["p-bob", "p-alice"]);
    const [bob, alice] = data.players;
    expect(bob?.displayName).toBe("Bob Builder");
    expect(alice?.displayName).toBe("Alice Wonder");

    // Elo/rating must NOT be exposed
    expect((bob as unknown as Record<string, unknown> | undefined)?.rating).toBeUndefined();
    expect((alice as unknown as Record<string, unknown> | undefined)?.rating).toBeUndefined();

    // Alice's favourite cards and fun stats
    expect(alice?.favouriteCards).toEqual([
      { id: "core-001", count: 25 },
      { id: "core-002", count: 15 },
    ]);
    expect(alice?.funStats).toEqual({
      nemesisCardId: "core-nemesis",
      totalDestroyed: 2,
      totalDefeated: 4,
    });

    // Search by display name
    const searchRes = await router(jsonRequest("GET", "/api/stats/players?search=Alice"));
    const searchData = await readJson<{ players: PublicPlayerSummary[] }>(searchRes);
    expect(searchData.players.map((p) => p.profileId)).toEqual(["p-alice"]);

    const searchNone = await router(jsonRequest("GET", "/api/stats/players?search=Charlie"));
    const searchNoneData = await readJson<{ players: PublicPlayerSummary[] }>(searchNone);
    expect(searchNoneData.players).toEqual([]); // Charlie opted out
  });
});
