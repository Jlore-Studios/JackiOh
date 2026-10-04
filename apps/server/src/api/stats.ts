/**
 * Public card and player statistics page API (SPEC §9.11, R641).
 *
 * Exposes:
 *  - GET /api/stats/cards: public card win rates, provisional gate at 1000 live games (R641).
 *  - GET /api/stats/cards/:id: card drill-down (patches, turn played, co-played cards).
 *  - GET /api/stats/player: caller's full stats (auth: "active").
 *  - PUT /api/stats/player: caller's stats upsert (auth: "active").
 *  - GET /api/stats/players: public player summaries (auth: "none").
 */

import { cardStats, winRate, type GameRecord } from "@jackioh/shared";
import {
  CARD_STATS_CACHE_TTL_SECONDS,
  CARD_STATS_CURVE_TOP,
  CARD_STATS_MIN_SAMPLE,
  PLAYER_STATS_BYTES_MAX,
  PLAYER_STATS_CACHE_TTL_SECONDS,
  PLAYER_STATS_PAGE_LIMIT,
  PUBLIC_STATS_MIN_LIVE_GAMES,
} from "../config";
import { loadPatchVersions } from "./catalog";
import { callerProfile } from "./collection";
import { ApiError, badRequest, ok, route, type Route } from "./http";

export type PublicCardStat = {
  id: string;
  name: string;
  cost: number;
  rarity: string;
  set: string;
  games: number;
  winRate: number | null;
  drawnGames: number;
  drawnWinRate: number | null;
  playedGames: number;
  playedWinRate: number | null;
  playRate: number;
  hasEnoughGames: boolean;
};

export type PublicStatsCardsResponse = {
  patch: string;
  previousPatch: string | null;
  gate: {
    cleared: boolean;
    liveGames: number;
    minLiveGames: number;
  };
  source: "provisional" | "live" | "dev";
  sourceLabel: string;
  minSample: number;
  totalGames: number;
  cards: PublicCardStat[];
  summary: {
    totalGames: number;
    liveGames: number;
    activePatch: string;
    source: "provisional" | "live" | "dev";
    bestCard: { id: string; name: string; winRate: number; games: number } | null;
    worstCard: { id: string; name: string; winRate: number; games: number } | null;
  };
};

export type CardDrillDownResponse = {
  card: {
    id: string;
    name: string;
    cost: number;
    rarity: string;
  };
  patches: { patch: string; games: number; winRate: number | null }[];
  byTurn: { turn: number; games: number; winRate: number | null }[];
  coPlayed: { id: string; name: string; games: number; winRate: number | null }[];
};

function cachedOk(body: unknown, maxAgeSeconds: number): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": `public, max-age=${String(maxAgeSeconds)}`,
    },
  });
}

export function createStatsRoutes(): Route[] {
  return [
    /**
     * GET /api/stats/cards
     * Public card win-rate data. Applies R641 publication gate:
     * - AI games pad the stats until the current patch has logged 1000 live games.
     * - At and above 1000 live games, live games only, AI games ignored.
     * - Minimum sample threshold: below 20 games, hasEnoughGames is false.
     */
    route("GET", "/api/stats/cards", "none", async (req, deps) => {
      const versions = await loadPatchVersions();
      const currentPatch = deps.games?.patch ?? deps.catalog.version;
      const requestedPatch = req.url.searchParams.get("patch")?.trim() || currentPatch;
      const sourceParam = req.url.searchParams.get("source")?.trim().toLowerCase();

      const patchIdx = versions.indexOf(requestedPatch);
      const previousPatch = patchIdx > 0 ? versions[patchIdx - 1] ?? null : null;

      // Query all records for the requested patch
      const patchRecords = await deps.store.gameRecords.list({
        source: "all",
        mode: null,
        patch: requestedPatch,
      });

      // Filter live games (tutorial games excluded per R641)
      const liveRecords = patchRecords.filter((r) => r.source === "live" && (r.mode as string) !== "tutorial");
      const liveGamesCount = liveRecords.length;

      // R641 publication gate: exactly 1000 live games required to clear the gate
      const cleared = liveGamesCount >= PUBLIC_STATS_MIN_LIVE_GAMES;

      let source: "provisional" | "live" | "dev";
      let sourceLabel: string;
      let recordsToCount: GameRecord[];

      if (cleared) {
        // When cleared, strictly ignore AI development games (no toggle to bring them back)
        source = "live";
        sourceLabel = "Live games";
        recordsToCount = liveRecords;
      } else if (sourceParam === "live") {
        source = "live";
        sourceLabel = "Live games";
        recordsToCount = liveRecords;
      } else if (sourceParam === "dev" || sourceParam === "ai") {
        source = "dev";
        sourceLabel = "AI development games";
        recordsToCount = patchRecords.filter((r) => r.source === "dev" && (r.mode as string) !== "tutorial");
      } else {
        source = "provisional";
        sourceLabel = "AI games + live games (provisional)";
        recordsToCount = patchRecords.filter((r) => (r.mode as string) !== "tutorial");
      }

      const report = cardStats(recordsToCount, {
        source: source === "live" ? "live" : source === "dev" ? "dev" : "all",
        mode: null,
        patch: requestedPatch,
        pilot: "unified",
      });

      const statsMap = new Map(report.cards.map((c) => [c.card, c]));
      const totalDecks = report.decks;

      const setParam = req.url.searchParams.get("set")?.trim().toLowerCase();
      const rarityParam = req.url.searchParams.get("rarity")?.trim().toLowerCase();
      const costParam = req.url.searchParams.get("cost");
      const cardParam = req.url.searchParams.get("card")?.trim();

      const cards: PublicCardStat[] = [];
      for (const [id, def] of Object.entries(deps.catalog.defs)) {
        if (def.token) continue; // Only non-token deckable cards
        if (cardParam && id !== cardParam) continue;
        if (setParam && def.set?.toLowerCase() !== setParam && !id.toLowerCase().startsWith(setParam)) continue;
        if (rarityParam && def.rarity?.toLowerCase() !== rarityParam) continue;
        const numericCost = typeof def.cost === "number" ? def.cost : typeof def.cost === "object" ? def.cost.base : 0;
        if (costParam !== null && costParam !== "") {
          const parsedCost = Number(costParam.replace("+", ""));
          if (!Number.isNaN(parsedCost)) {
            if (parsedCost >= CARD_STATS_CURVE_TOP) {
              if (numericCost < CARD_STATS_CURVE_TOP) continue;
            } else {
              if (numericCost !== parsedCost) continue;
            }
          }
        }

        const stat = statsMap.get(id);
        const inDeckGames = stat?.inDeck.games ?? 0;
        const rate = stat ? winRate(stat.inDeck) : null;
        const drawnGames = (stat?.played.games ?? 0) + (stat?.drawnNotPlayed.games ?? 0);
        const drawnWins = (stat?.played.wins ?? 0) + (stat?.drawnNotPlayed.wins ?? 0);
        const drawnRate = drawnGames > 0 ? drawnWins / drawnGames : null;
        const playedGames = stat?.played.games ?? 0;
        const playedRate = stat ? winRate(stat.played) : null;
        const playRate = totalDecks > 0 ? inDeckGames / totalDecks : 0;
        const hasEnoughGames = inDeckGames >= CARD_STATS_MIN_SAMPLE;

        cards.push({
          id,
          name: def.name,
          cost: numericCost,
          rarity: def.rarity,
          set: def.set ?? "core",
          games: inDeckGames,
          winRate: rate,
          drawnGames,
          drawnWinRate: drawnRate,
          playedGames,
          playedWinRate: playedRate,
          playRate,
          hasEnoughGames,
        });
      }

      // Sort cards by win rate desc, then games desc, then name
      cards.sort((a, b) => {
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        if (a.winRate !== null && b.winRate !== null && a.winRate !== b.winRate) {
          return b.winRate - a.winRate;
        }
        return b.games - a.games || a.name.localeCompare(b.name);
      });

      // Best and worst card above sample threshold
      const qualifyingCards = cards.filter((c) => c.hasEnoughGames && c.winRate !== null);
      const bestCard = qualifyingCards.length > 0 ? {
        id: qualifyingCards[0]!.id,
        name: qualifyingCards[0]!.name,
        winRate: qualifyingCards[0]!.winRate!,
        games: qualifyingCards[0]!.games,
      } : null;
      const worstCard = qualifyingCards.length > 0 ? {
        id: qualifyingCards[qualifyingCards.length - 1]!.id,
        name: qualifyingCards[qualifyingCards.length - 1]!.name,
        winRate: qualifyingCards[qualifyingCards.length - 1]!.winRate!,
        games: qualifyingCards[qualifyingCards.length - 1]!.games,
      } : null;

      const response: PublicStatsCardsResponse = {
        patch: requestedPatch,
        previousPatch,
        gate: {
          cleared,
          liveGames: liveGamesCount,
          minLiveGames: PUBLIC_STATS_MIN_LIVE_GAMES,
        },
        source,
        sourceLabel,
        minSample: CARD_STATS_MIN_SAMPLE,
        totalGames: report.games,
        cards,
        summary: {
          totalGames: report.games,
          liveGames: liveGamesCount,
          activePatch: requestedPatch,
          source,
          bestCard,
          worstCard,
        },
      };

      return cachedOk(response, CARD_STATS_CACHE_TTL_SECONDS);
    }),

    /**
     * GET /api/stats/cards/:id
     * Card drill-down:
     * - Win rate by cleared patch over time.
     * - Win rate by turn played.
     * - Co-played synergy cards.
     */
    route("GET", "/api/stats/cards/:id", "none", async (req, deps) => {
      const cardId = req.params["id"];
      const def = cardId ? deps.catalog.defs[cardId] : undefined;
      if (!def || !cardId) {
        throw new ApiError("not_found", `Card ${cardId ?? ""} not found`);
      }

      const versions = await loadPatchVersions();
      const patchHistory: { patch: string; games: number; winRate: number | null }[] = [];

      for (const p of versions) {
        const pRecords = (await deps.store.gameRecords.list({ source: "live", mode: null, patch: p }))
          .filter((r) => (r.mode as string) !== "tutorial");
        if (pRecords.length >= PUBLIC_STATS_MIN_LIVE_GAMES) {
          const report = cardStats(pRecords, { source: "live", mode: null, patch: p, pilot: "unified" });
          const stat = report.cards.find((c) => c.card === cardId);
          patchHistory.push({
            patch: p,
            games: stat?.inDeck.games ?? 0,
            winRate: stat ? winRate(stat.inDeck) : null,
          });
        }
      }

      // Use active patch or all records for turn and co-play metrics
      const currentPatch = deps.games?.patch ?? deps.catalog.version;
      const allRecords = await deps.store.gameRecords.list({ source: "all", mode: null, patch: currentPatch });
      const liveRecords = allRecords.filter((r) => r.source === "live" && (r.mode as string) !== "tutorial");
      const recordsToCount = liveRecords.length >= PUBLIC_STATS_MIN_LIVE_GAMES
        ? liveRecords
        : allRecords.filter((r) => (r.mode as string) !== "tutorial");

      // By turn played
      const turnTallies = new Map<number, { games: number; wins: number }>();
      // Co-played cards
      const coPlayedTallies = new Map<string, { games: number; wins: number }>();

      for (const record of recordsToCount) {
        for (const seat of ["p1", "p2"] as const) {
          const summary = record.game.seats[seat];
          if (!summary.deck.includes(cardId)) continue;

          const won = record.game.winner === seat;

          // Check turns on which this card was played (from recorded playedTurns)
          const turnsPlayedInGame = new Set<number>();
          if (summary.playedTurns && summary.playedTurns.length === summary.played.length) {
            for (let i = 0; i < summary.played.length; i++) {
              if (summary.played[i] === cardId) {
                const turn = summary.playedTurns[i];
                if (turn !== undefined && turn > 0) {
                  turnsPlayedInGame.add(turn);
                }
              }
            }
          }
          for (const turn of turnsPlayedInGame) {
            let t = turnTallies.get(turn);
            if (!t) {
              t = { games: 0, wins: 0 };
              turnTallies.set(turn, t);
            }
            t.games += 1;
            if (won) t.wins += 1;
          }

          // Co-played cards in the same deck
          for (const otherId of new Set(summary.deck)) {
            if (otherId === cardId) continue;
            let c = coPlayedTallies.get(otherId);
            if (!c) {
              c = { games: 0, wins: 0 };
              coPlayedTallies.set(otherId, c);
            }
            c.games += 1;
            if (won) c.wins += 1;
          }
        }
      }

      const byTurn = Array.from(turnTallies.entries())
        .sort((a, b) => a[0] - b[0])
        .map(([turn, t]) => ({
          turn,
          games: t.games,
          winRate: t.games > 0 ? t.wins / t.games : null,
        }));

      const coPlayed = Array.from(coPlayedTallies.entries())
        .map(([id, t]) => ({
          id,
          name: deps.catalog.defs[id]?.name ?? id,
          games: t.games,
          winRate: t.games > 0 ? t.wins / t.games : null,
        }))
        .sort((a, b) => b.games - a.games)
        .slice(0, 10);

      return cachedOk(
        {
          card: {
            id: def.id,
            name: def.name,
            cost: typeof def.cost === "number" ? def.cost : typeof def.cost === "object" ? def.cost.base : 0,
            rarity: def.rarity,
          },
          patches: patchHistory,
          byTurn,
          coPlayed,
        },
        CARD_STATS_CACHE_TTL_SECONDS,
      );
    }),

    /**
     * GET /api/stats/player
     * Signed-in player reads their own tracked statistics and privacy setting.
     */
    route("GET", "/api/stats/player", "active", async (req, deps) => {
      const profile = callerProfile(req);
      const row = await deps.store.playerStats.get(profile.id);
      return ok({
        stats: row?.stats ?? {},
        isPrivate: row?.isPrivate ?? false,
        updatedAt: row?.updatedAt ?? null,
      });
    }),

    /**
     * PUT /api/stats/player
     * Signed-in player updates their tracked statistics and privacy setting.
     */
    route("PUT", "/api/stats/player", "active", async (req, deps) => {
      const profile = callerProfile(req);
      const raw = JSON.stringify(req.body);
      if (raw.length > PLAYER_STATS_BYTES_MAX) {
        throw badRequest(
          `player stats exceeds maximum payload size of ${String(PLAYER_STATS_BYTES_MAX)} bytes`,
        );
      }

      const existing = await deps.store.playerStats.get(profile.id);
      const isPrivate = typeof req.body["isPrivate"] === "boolean"
        ? (req.body["isPrivate"] as boolean)
        : (existing?.isPrivate ?? false);

      const stats = typeof req.body["stats"] === "object" && req.body["stats"] !== null && !Array.isArray(req.body["stats"])
        ? (req.body["stats"] as Record<string, unknown>)
        : (existing?.stats ?? {});

      const now = deps.timers.now();
      await deps.store.playerStats.put(profile.id, stats, isPrivate, now);
      return ok({ stats, isPrivate, updatedAt: now });
    }),

    /**
     * GET /api/stats/players
     * Public player summaries (games, win rate, favourite cards, fun stats).
     * Excludes private players. Keeps Elo and rankings separate.
     */
    route("GET", "/api/stats/players", "none", async (req, deps) => {
      const search = req.url.searchParams.get("search")?.trim() || undefined;
      const pageStr = req.url.searchParams.get("page");
      const page = pageStr && /^\d+$/.test(pageStr) ? Math.max(1, parseInt(pageStr, 10)) : 1;
      const limit = PLAYER_STATS_PAGE_LIMIT;
      const offset = (page - 1) * limit;

      const players = await deps.store.playerStats.listPublic({ search, limit, offset });
      return cachedOk({ players, page, limit }, PLAYER_STATS_CACHE_TTL_SECONDS);
    }),
  ];
}
