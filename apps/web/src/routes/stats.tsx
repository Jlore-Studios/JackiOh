// `/stats`: Public card and player statistics page (SPEC §9.11, R654).
//
// Accessible signed out and signed in.
// Provides two tabs:
//  - Cards: sortable, filterable table of card win rates, play rates, and personal numbers.
//  - Players: public player aggregates (games, win rate, favourite cards, fun stats from #125).
// Features:
//  - Provisional banner while the current patch is below the publication gate (R654). It, like the
//    rest of the page, names no data source and none of the gate's workings (R661); the patch tile
//    shows the patch's counted games, never the gate's live count.
//  - Per-row sample floor (20 games): displays "not enough games" below threshold.
//  - URL search params for shareable views.
//  - Card drill-down modal showing CardFace, patch history, turn curve, and co-played synergy.

import { useCallback, useEffect, useMemo, useState, type ReactElement } from "react";
import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";
import { CardDefsProvider } from "../cards/index.ts";
import { CardFace } from "../cards/CardFace.tsx";
import { faceModel } from "../cards/model.ts";
import {
  getCardDrillDown,
  getCardStats,
  getPublicPlayers,
  type CardDrillDownResponse,
  type PublicPlayerSummary,
  type PublicStatsCardsResponse,
} from "../net/api.ts";
import { navigate, paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { CARD_STATS_MIN_SAMPLE } from "../stats/config.ts";
import { winPercent } from "../stats/model.ts";
import { usePlayerStats } from "../stats/store.ts";
import { statsTestid } from "../stats/testids.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./stats.css";

const CATALOG = catalogJson as unknown as CardDefs;

type TabKey = "cards" | "players";
type SortColumn = "name" | "cost" | "rarity" | "games" | "winRate" | "drawnWinRate" | "playRate";
type SortDirection = "asc" | "desc";

export default function StatsRoute(): ReactElement {
  // Read initial state from URL search params
  const initialParams = useMemo(() => new URLSearchParams(window.location.search), []);

  const [tab, setTab] = useState<TabKey>(initialParams.get("tab") === "players" ? "players" : "cards");
  const [patch, setPatch] = useState<string>(initialParams.get("patch") ?? "");
  const [search, setSearch] = useState<string>(initialParams.get("search") ?? "");
  const [sortCol, setSortCol] = useState<SortColumn>(
    (initialParams.get("sort") as SortColumn) ?? "winRate",
  );
  const [sortDir, setSortDir] = useState<SortDirection>(
    initialParams.get("dir") === "asc" ? "asc" : "desc",
  );
  const [setFilter, setSetFilter] = useState<string>(initialParams.get("set") ?? "");
  const [rarityFilter, setRarityFilter] = useState<string>(initialParams.get("rarity") ?? "");
  const [costFilter, setCostFilter] = useState<string>(initialParams.get("cost") ?? "");
  const [cardDrillDownId, setCardDrillDownId] = useState<string | null>(initialParams.get("card"));

  // Column visibility
  const [visibleCols, setVisibleCols] = useState({
    thumbnail: true,
    cost: true,
    rarity: true,
    games: true,
    winRate: true,
    drawnWinRate: true,
    playRate: true,
    personal: true,
  });

  // Card stats response
  const [cardStatsLoading, setCardStatsLoading] = useState(true);
  const [cardStatsError, setCardStatsError] = useState(false);
  const [cardStatsData, setCardStatsData] = useState<PublicStatsCardsResponse | null>(null);

  // Players response
  const [playersLoading, setPlayersLoading] = useState(false);
  const [playersError, setPlayersError] = useState(false);
  const [playersList, setPlayersList] = useState<PublicPlayerSummary[]>([]);
  const [playerSearch, setPlayerSearch] = useState("");

  // Drill-down data
  const [drillDownLoading, setDrillDownLoading] = useState(false);
  const [drillDownData, setDrillDownData] = useState<CardDrillDownResponse | null>(null);

  const playerStats = usePlayerStats();
  const signedIn = readSession() !== null;

  // Sync state to URL
  const updateUrl = useCallback(() => {
    const params = new URLSearchParams();
    if (tab !== "cards") params.set("tab", tab);
    if (patch) params.set("patch", patch);
    if (search) params.set("search", search);
    if (sortCol !== "winRate") params.set("sort", sortCol);
    if (sortDir !== "desc") params.set("dir", sortDir);
    if (setFilter) params.set("set", setFilter);
    if (rarityFilter) params.set("rarity", rarityFilter);
    if (costFilter) params.set("cost", costFilter);
    if (cardDrillDownId) params.set("card", cardDrillDownId);

    const query = params.toString();
    const target = query ? `${paths.stats}?${query}` : paths.stats;
    navigate(target, { replace: true });
  }, [tab, patch, search, sortCol, sortDir, setFilter, rarityFilter, costFilter, cardDrillDownId]);

  useEffect(() => {
    updateUrl();
  }, [updateUrl]);

  // Fetch card stats
  const fetchCardStats = useCallback(
    (targetPatch?: string) => {
      setCardStatsLoading(true);
      setCardStatsError(false);
      const controller = new AbortController();

      getCardStats({
        patch: targetPatch || (patch ? patch : undefined),
        set: setFilter ? setFilter : undefined,
        rarity: rarityFilter ? rarityFilter : undefined,
        cost: costFilter ? Number(costFilter) : undefined,
        signal: controller.signal,
      })
        .then((res) => {
          setCardStatsData(res);
          setCardStatsLoading(false);
        })
        .catch(() => {
          setCardStatsError(true);
          setCardStatsLoading(false);
        });

      return () => controller.abort();
    },
    [patch, setFilter, rarityFilter, costFilter],
  );

  useEffect(() => {
    return fetchCardStats();
  }, [fetchCardStats]);

  // Fetch players
  const fetchPlayers = useCallback((query?: string) => {
    setPlayersLoading(true);
    setPlayersError(false);
    const controller = new AbortController();

    getPublicPlayers({ search: query, signal: controller.signal })
      .then((res) => {
        setPlayersList(res.players);
        setPlayersLoading(false);
      })
      .catch(() => {
        setPlayersError(true);
        setPlayersLoading(false);
      });

    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (tab === "players") {
      return fetchPlayers(playerSearch);
    }
  }, [tab, playerSearch, fetchPlayers]);

  // Fetch card drill-down
  useEffect(() => {
    if (!cardDrillDownId) {
      setDrillDownData(null);
      return;
    }
    setDrillDownLoading(true);
    const controller = new AbortController();
    getCardDrillDown(cardDrillDownId, controller.signal)
      .then((data) => {
        setDrillDownData(data);
        setDrillDownLoading(false);
      })
      .catch(() => {
        setDrillDownLoading(false);
      });

    return () => controller.abort();
  }, [cardDrillDownId]);

  // Toggle sort
  const handleSort = (col: SortColumn) => {
    if (sortCol === col) {
      setSortDir((prev) => (prev === "asc" ? "desc" : "asc"));
    } else {
      setSortCol(col);
      setSortDir(col === "name" || col === "cost" || col === "rarity" ? "asc" : "desc");
    }
  };

  // Filter and sort cards
  const filteredCards = useMemo(() => {
    if (!cardStatsData) return [];
    let list = [...cardStatsData.cards];

    if (search.trim()) {
      const q = search.trim().toLowerCase();
      list = list.filter((c) => c.name.toLowerCase().includes(q) || c.id.toLowerCase().includes(q));
    }

    const flip = sortDir === "desc" ? -1 : 1;
    list.sort((a, b) => {
      if (sortCol === "winRate") {
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        if (a.winRate !== null && b.winRate !== null && a.winRate !== b.winRate) {
          return (a.winRate - b.winRate) * flip;
        }
      } else if (sortCol === "drawnWinRate") {
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        if (a.drawnWinRate !== null && b.drawnWinRate !== null && a.drawnWinRate !== b.drawnWinRate) {
          return (a.drawnWinRate - b.drawnWinRate) * flip;
        }
      } else if (sortCol === "games") {
        if (a.games !== b.games) return (a.games - b.games) * flip;
      } else if (sortCol === "cost") {
        if (a.cost !== b.cost) return (a.cost - b.cost) * flip;
      } else if (sortCol === "playRate") {
        if (a.playRate !== b.playRate) return (a.playRate - b.playRate) * flip;
      } else if (sortCol === "rarity") {
        const rarities = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
        const diff = rarities.indexOf(a.rarity) - rarities.indexOf(b.rarity);
        if (diff !== 0) return diff * flip;
      } else if (sortCol === "name") {
        const diff = a.name.localeCompare(b.name);
        if (diff !== 0) return diff * flip;
      }
      return a.games - b.games || a.name.localeCompare(b.name);
    });

    return list;
  }, [cardStatsData, search, sortCol, sortDir]);

  const summary = cardStatsData?.summary;
  const isProvisional = cardStatsData?.source === "provisional";
  const minSample = cardStatsData?.minSample ?? CARD_STATS_MIN_SAMPLE;

  const drillDownDef = cardDrillDownId ? CATALOG[cardDrillDownId] : undefined;
  const drillDownFace = drillDownDef ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: false }) : null;

  return (
    <CardDefsProvider defs={CATALOG}>
      <div className="app-shell app-shell--wide tavern stats-page" data-testid={statsTestid.screen}>
        <header className="stats-header">
          <BackLink />
          <h1 className="stats-title">Statistics</h1>
        </header>

        {/* Summary tiles */}
        {cardStatsData && (
          <section className="stats-summary-tiles" data-testid={statsTestid.summaryTiles} aria-label="Statistics summary">
            <div className="stats-summary-tile" data-testid={statsTestid.summaryTotalGames}>
              <span className="stats-summary-label">Total games</span>
              <span className="stats-summary-value">{summary?.totalGames.toLocaleString()}</span>
            </div>
            <div className="stats-summary-tile" data-testid={statsTestid.summaryPatchGames}>
              <span className="stats-summary-label">Patch {cardStatsData.patch}</span>
              <span className="stats-summary-value">{summary?.totalGames.toLocaleString()}</span>
            </div>
            {summary?.bestCard && (
              <div className="stats-summary-tile" data-testid={statsTestid.summaryBestCard}>
                <span className="stats-summary-label">Top card (win rate)</span>
                <span className="stats-summary-highlight">
                  {summary.bestCard.name} ({Math.round(summary.bestCard.winRate * 100)}%)
                </span>
              </div>
            )}
            {summary?.worstCard && (
              <div className="stats-summary-tile" data-testid={statsTestid.summaryWorstCard}>
                <span className="stats-summary-label">Lowest card (win rate)</span>
                <span className="stats-summary-highlight">
                  {summary.worstCard.name} ({Math.round(summary.worstCard.winRate * 100)}%)
                </span>
              </div>
            )}
          </section>
        )}

        {/* Provisional banner */}
        {isProvisional && (
          <div className="stats-provisional-banner" data-testid={statsTestid.provisionalBanner} role="alert">
            <div className="stats-provisional-info">
              <span className="stats-provisional-title">Provisional statistics</span>
            </div>
            {cardStatsData?.previousPatch && (
              <button
                type="button"
                className="stats-fallback-btn link-button"
                data-testid={statsTestid.fallbackToggle}
                onClick={() => {
                  const target = cardStatsData.previousPatch!;
                  setPatch(target);
                  fetchCardStats(target);
                }}
              >
                View previous patch ({cardStatsData.previousPatch}) →
              </button>
            )}
          </div>
        )}

        {/* How to read this */}
        <details className="stats-info-accordion">
          <summary className="stats-info-summary">How to read these statistics</summary>
          <div className="stats-info-content">
            <p>
              <strong>Sample size threshold:</strong> Cards with fewer than {minSample} games display{" "}
              <em>"not enough games"</em> instead of a percentage to prevent misleading sample artifacts.
            </p>
            <p>
              <strong>When drawn vs Play rate:</strong> <em>When drawn</em> shows win rate when the card was in
              hand or played; <em>Play rate</em> shows the percentage of all recorded decks containing the card.
            </p>
          </div>
        </details>

        {/* Tabs */}
        <div className="stats-tabs" role="tablist" aria-label="Statistics views">
          <button
            type="button"
            role="tab"
            id="stats-tab-cards"
            aria-selected={tab === "cards"}
            aria-controls="stats-panel-cards"
            className={`stats-tab ${tab === "cards" ? "stats-tab--active" : ""}`}
            data-testid={statsTestid.tabCards}
            onClick={() => setTab("cards")}
          >
            Cards
          </button>
          <button
            type="button"
            role="tab"
            id="stats-tab-players"
            aria-selected={tab === "players"}
            aria-controls="stats-panel-players"
            className={`stats-tab ${tab === "players" ? "stats-tab--active" : ""}`}
            data-testid={statsTestid.tabPlayers}
            onClick={() => setTab("players")}
          >
            Players
          </button>
        </div>

        {/* CARDS TAB */}
        {tab === "cards" && (
          <section id="stats-panel-cards" role="tabpanel" aria-labelledby="stats-tab-cards" className="stats-panel">
            {/* Filter controls */}
            <div className="stats-filter-bar">
              <input
                type="search"
                className="stats-search-input"
                data-testid={statsTestid.searchInput}
                placeholder="Search card by name or id…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />

              <div className="stats-filter-selects">
                <select
                  className="stats-select"
                  value={setFilter}
                  onChange={(e) => setSetFilter(e.target.value)}
                  aria-label="Filter by set"
                >
                  <option value="">All sets</option>
                  <option value="core">Core</option>
                  <option value="classic">Classic+</option>
                </select>

                <select
                  className="stats-select"
                  value={rarityFilter}
                  onChange={(e) => setRarityFilter(e.target.value)}
                  aria-label="Filter by rarity"
                >
                  <option value="">All rarities</option>
                  <option value="Common">Common</option>
                  <option value="Rare">Rare</option>
                  <option value="Epic">Epic</option>
                  <option value="Legendary">Legendary</option>
                  <option value="Mythic">Mythic</option>
                </select>

                <select
                  className="stats-select"
                  value={costFilter}
                  onChange={(e) => setCostFilter(e.target.value)}
                  aria-label="Filter by cost"
                >
                  <option value="">All costs</option>
                  <option value="0">0</option>
                  <option value="1">1</option>
                  <option value="2">2</option>
                  <option value="3">3</option>
                  <option value="4">4</option>
                  <option value="5">5</option>
                  <option value="6">6+</option>
                </select>
              </div>

              {/* Column visibility toggles */}
              <div className="stats-col-toggles">
                <span className="stats-col-toggles-label">Columns:</span>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.cost}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, cost: e.target.checked }))}
                  />
                  Cost
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.rarity}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, rarity: e.target.checked }))}
                  />
                  Rarity
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.games}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, games: e.target.checked }))}
                  />
                  Games
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.drawnWinRate}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, drawnWinRate: e.target.checked }))}
                  />
                  When drawn
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.playRate}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, playRate: e.target.checked }))}
                  />
                  Play rate
                </label>
                {signedIn && (
                  <label className="stats-col-toggle">
                    <input
                      type="checkbox"
                      checked={visibleCols.personal}
                      onChange={(e) => setVisibleCols((c) => ({ ...c, personal: e.target.checked }))}
                    />
                    Your stats
                  </label>
                )}
              </div>
            </div>

            {/* Table */}
            {cardStatsLoading ? (
              <p className="stats-loading">Loading card statistics…</p>
            ) : cardStatsError ? (
              <div className="stats-error">
                <p>Failed to load card statistics.</p>
                <button type="button" className="stats-retry-btn" onClick={() => fetchCardStats()}>
                  Retry
                </button>
              </div>
            ) : filteredCards.length === 0 ? (
              <p className="stats-empty">No cards match the selected filters.</p>
            ) : (
              <div className="stats-table-wrapper">
                <table className="stats-table" data-testid={statsTestid.cardsTable}>
                  <thead>
                    <tr>
                      {visibleCols.thumbnail && <th className="stats-th--thumb" scope="col">Card</th>}
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        aria-sort={sortCol === "name" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("name")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("name");
                          }
                        }}
                        tabIndex={0}
                      >
                        Name {sortCol === "name" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      {visibleCols.cost && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "cost" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("cost")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("cost");
                            }
                          }}
                          tabIndex={0}
                        >
                          Cost{sortCol === "cost" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}
                        </th>
                      )}
                      {visibleCols.rarity && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "rarity" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("rarity")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("rarity");
                            }
                          }}
                          tabIndex={0}
                        >
                          Rarity {sortCol === "rarity" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {visibleCols.games && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "games" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("games")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("games");
                            }
                          }}
                          tabIndex={0}
                        >
                          Games {sortCol === "games" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        aria-sort={sortCol === "winRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("winRate")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("winRate");
                          }
                        }}
                        tabIndex={0}
                      >
                        Win rate {sortCol === "winRate" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      {visibleCols.drawnWinRate && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "drawnWinRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("drawnWinRate")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("drawnWinRate");
                            }
                          }}
                          tabIndex={0}
                        >
                          When drawn {sortCol === "drawnWinRate" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {visibleCols.playRate && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "playRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("playRate")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("playRate");
                            }
                          }}
                          tabIndex={0}
                        >
                          Play rate {sortCol === "playRate" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {signedIn && visibleCols.personal && (
                        <th scope="col" className="stats-th">
                          Your stats
                        </th>
                      )}
                    </tr>
                  </thead>
                  <tbody>
                    {filteredCards.map((card) => {
                      const personalPlayed = playerStats.cards[card.id]?.played ?? 0;
                      const personalRate = winPercent(playerStats);

                      return (
                        <tr
                          key={card.id}
                          className="stats-tr"
                          data-card-id={card.id}
                          onClick={() => setCardDrillDownId(card.id)}
                          role="button"
                          tabIndex={0}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              setCardDrillDownId(card.id);
                            }
                          }}
                        >
                          {visibleCols.thumbnail && (
                            <td className="stats-td stats-td--thumb">
                              <span className="stats-card-thumb-badge" data-rarity={card.rarity}>
                                {card.cost}
                              </span>
                            </td>
                          )}
                          <td className="stats-td stats-td--name">
                            <span className="stats-card-name-link">{card.name}</span>
                          </td>
                          {visibleCols.cost && <td className="stats-td">{card.cost}</td>}
                          {visibleCols.rarity && (
                            <td className="stats-td">
                              <span className="stats-rarity-tag" data-rarity={card.rarity}>
                                {card.rarity}
                              </span>
                            </td>
                          )}
                          {visibleCols.games && <td className="stats-td">{card.games}</td>}
                          <td className="stats-td stats-td--winrate">
                            {card.hasEnoughGames && card.winRate !== null ? (
                              <div className="stats-winrate-cell">
                                <span className="stats-winrate-text">{Math.round(card.winRate * 100)}%</span>
                                <div className="stats-confidence-bar">
                                  <div
                                    className="stats-confidence-fill"
                                    style={{ width: `${Math.round(card.winRate * 100)}%` }}
                                  />
                                </div>
                              </div>
                            ) : (
                              <span className="stats-not-enough">not enough games</span>
                            )}
                          </td>
                          {visibleCols.drawnWinRate && (
                            <td className="stats-td">
                              {card.hasEnoughGames && card.drawnWinRate !== null
                                ? `${Math.round(card.drawnWinRate * 100)}%`
                                : "—"}
                            </td>
                          )}
                          {visibleCols.playRate && (
                            <td className="stats-td">{`${Math.round(card.playRate * 100)}%`}</td>
                          )}
                          {signedIn && visibleCols.personal && (
                            <td className="stats-td stats-td--personal">
                              {personalPlayed > 0 ? (
                                <span>
                                  {personalPlayed}× {personalRate !== null ? `(${personalRate}%)` : ""}
                                </span>
                              ) : (
                                <span className="stats-not-played">0</span>
                              )}
                            </td>
                          )}
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        )}

        {/* PLAYERS TAB */}
        {tab === "players" && (
          <section id="stats-panel-players" role="tabpanel" aria-labelledby="stats-tab-players" className="stats-panel">
            <div className="stats-player-head">
              <input
                type="search"
                className="stats-search-input"
                placeholder="Search player by display name…"
                value={playerSearch}
                onChange={(e) => setPlayerSearch(e.target.value)}
              />
              <p className="stats-player-note">
                Player stats are public by default (opt-out in Settings). Elo ratings and rankings are kept separate.
              </p>
            </div>

            {playersLoading ? (
              <p className="stats-loading">Loading players…</p>
            ) : playersError ? (
              <div className="stats-error">
                <p>Failed to load player stats.</p>
                <button type="button" className="stats-retry-btn" onClick={() => fetchPlayers(playerSearch)}>
                  Retry
                </button>
              </div>
            ) : playersList.length === 0 ? (
              <p className="stats-empty">No players found.</p>
            ) : (
              <div className="stats-table-wrapper">
                <table className="stats-table" data-testid={statsTestid.playersTable}>
                  <thead>
                    <tr>
                      <th scope="col" className="stats-th">Player</th>
                      <th scope="col" className="stats-th">Games</th>
                      <th scope="col" className="stats-th">Win rate</th>
                      <th scope="col" className="stats-th">Favourite card</th>
                      <th scope="col" className="stats-th">Cards destroyed</th>
                      <th scope="col" className="stats-th">Cards defeated</th>
                    </tr>
                  </thead>
                  <tbody>
                    {playersList.map((p) => {
                      const name = p.displayName ?? `Player ${p.profileId.slice(0, 8)}`;
                      const rate = p.winRate !== null ? `${Math.round(p.winRate * 100)}%` : "—";
                      const topCard = p.favouriteCards?.[0];
                      const favCardName = topCard ? (CATALOG[topCard.id]?.name ?? topCard.id) : "—";
                      return (
                        <tr key={p.profileId} className="stats-tr">
                          <td className="stats-td stats-td--player-name">{name}</td>
                          <td className="stats-td">{p.games}</td>
                          <td className="stats-td">{rate}</td>
                          <td className="stats-td">{favCardName}</td>
                          <td className="stats-td">{p.funStats?.totalDestroyed ?? 0}</td>
                          <td className="stats-td">{p.funStats?.totalDefeated ?? 0}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        )}

        {/* Card Drill-Down Modal */}
        {cardDrillDownId && (
          <div className="stats-modal-scrim" onClick={() => setCardDrillDownId(null)} aria-hidden="true">
            <div
              className="stats-drilldown-modal"
              data-testid={statsTestid.drillDownModal}
              role="dialog"
              aria-modal="true"
              aria-labelledby="drilldown-title"
              onClick={(e) => e.stopPropagation()}
            >
              <header className="stats-drilldown-header">
                <h2 id="drilldown-title" className="stats-drilldown-title">
                  {drillDownDef?.name ?? cardDrillDownId}
                </h2>
                <button
                  type="button"
                  className="stats-drilldown-close"
                  data-testid={statsTestid.drillDownClose}
                  aria-label="Close card details"
                  onClick={() => setCardDrillDownId(null)}
                >
                  ✕
                </button>
              </header>

              <div className="stats-drilldown-body">
                {drillDownFace && (
                  <div className="stats-drilldown-card-preview">
                    <CardFace face={drillDownFace} layout="full" />
                  </div>
                )}

                <div className="stats-drilldown-details">
                  {drillDownLoading ? (
                    <p className="stats-loading">Loading drill-down data…</p>
                  ) : drillDownData ? (
                    <>
                      {/* Patches historical win rate */}
                      <section className="stats-drilldown-section">
                        <h3>Win rate by patch</h3>
                        {drillDownData.patches.length === 0 ? (
                          <p className="stats-empty-sub">No earlier patches to show yet.</p>
                        ) : (
                          <ul className="stats-drilldown-list">
                            {drillDownData.patches.map((p) => (
                              <li key={p.patch} className="stats-drilldown-item">
                                <span className="stats-drilldown-label">Patch {p.patch}:</span>{" "}
                                <span>
                                  {p.winRate !== null ? `${Math.round(p.winRate * 100)}%` : "not enough games"}{" "}
                                  ({p.games} games)
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </section>

                      {/* Turn played distribution */}
                      <section className="stats-drilldown-section">
                        <h3>Win rate by turn played</h3>
                        {drillDownData.byTurn.length === 0 ? (
                          <p className="stats-empty-sub">No games recorded with this card played.</p>
                        ) : (
                          <ul className="stats-drilldown-list">
                            {drillDownData.byTurn.map((t) => (
                              <li key={t.turn} className="stats-drilldown-item">
                                <span className="stats-drilldown-label">Turn {t.turn}:</span>{" "}
                                <span>
                                  {t.winRate !== null ? `${Math.round(t.winRate * 100)}%` : "—"} ({t.games} games)
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </section>

                      {/* Co-played synergy cards */}
                      <section className="stats-drilldown-section">
                        <h3>Co-played cards (synergy)</h3>
                        {drillDownData.coPlayed.length === 0 ? (
                          <p className="stats-empty-sub">No co-played cards recorded.</p>
                        ) : (
                          <ul className="stats-drilldown-list">
                            {drillDownData.coPlayed.map((c) => (
                              <li key={c.id} className="stats-drilldown-item">
                                <span className="stats-drilldown-label">{c.name}:</span>{" "}
                                <span>
                                  {c.winRate !== null ? `${Math.round(c.winRate * 100)}%` : "—"} ({c.games} games)
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </section>
                    </>
                  ) : (
                    <p className="stats-empty-sub">Details unavailable.</p>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </CardDefsProvider>
  );
}
