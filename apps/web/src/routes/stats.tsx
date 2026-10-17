// `/stats`: Public card and player statistics page (SPEC §9.11, R654).
//
// Accessible signed out and signed in.
// Provides two tabs:
//  - Cards: sortable, filterable table of card win rates, play rates, and personal numbers.
//  - Players: public player aggregates (games, win rate, favourite cards, fun stats from #125).
// Features:
//  - Provisional banner while the current patch is below the publication gate (R654). It, like the
//    rest of the page, names no data source and none of the gate's workings (R661); its one games
//    tile counts the patch's games, never the gate's live count.
//  - Per-row sample floor (20 games): displays "not enough games" below threshold.
//  - URL search params for shareable views.
//  - Card drill-down modal showing CardFace, patch history, turn curve, and co-played synergy.
//  - Players tab: the search asks the server a pause after the typing, one page of the server's limit at a time.

import { Fragment, useCallback, useEffect, useMemo, useState, type ReactElement, type ReactNode } from "react";
import catalogJson from "@jackioh/cards/catalog.json";
import { SHIPPED_SETS, type CardDefs } from "@jackioh/shared";
import Username from "../auth/Username.tsx";
import { CardDefsProvider } from "../cards/index.ts";
import { chineseName, drawnInChinese, readerName } from "../cards/chinese.ts";
import { CardFace } from "../cards/CardFace.tsx";
import { faceModel } from "../cards/model.ts";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
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
import { CARD_STATS_MIN_SAMPLE, PLAYER_SEARCH_DEBOUNCE_MS } from "../stats/config.ts";
import { formatPlayedDelta, winPercent } from "../stats/model.ts";
import { usePlayerStats } from "../stats/store.ts";
import { statsTestid } from "../stats/testids.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./stats.css";

const CATALOG = catalogJson as unknown as CardDefs;

/** A card's name as the reader reads it: the catalog's Chinese for a reader of Chinese, else the server's own. */
function shownName(id: string, name: string): string {
  return drawnInChinese(false) ? chineseName(id, name, CATALOG[id]) : name;
}

/**
 * A translated sentence with an element inside it: the sentence is one dictionary entry, written with
 * `{token}`s, and each token is replaced by the node given for it, so a language may order them as it
 * needs and English reads as it always did.
 */
function withNodes(sentence: string, nodes: Readonly<Record<string, ReactNode>>): ReactNode[] {
  return sentence.split(/(\{\w+\})/).map((part, index) => {
    const token = /^\{(\w+)\}$/.exec(part)?.[1];
    return <Fragment key={index}>{token !== undefined && token in nodes ? nodes[token] : part}</Fragment>;
  });
}

type TabKey = "cards" | "players";
type SortColumn = "name" | "cost" | "rarity" | "games" | "winRate" | "drawnWinRate" | "playRate" | "playedDelta";
type SortDirection = "asc" | "desc";

export default function StatsRoute(): ReactElement {
  const locale = useLocale();
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
    playedDelta: false,
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
  // The search the players tab last asked for (the box's text, a pause after its last change) and its page.
  const [playerQuery, setPlayerQuery] = useState("");
  const [playerPage, setPlayerPage] = useState(1);
  const [playersHasMore, setPlayersHasMore] = useState(false);

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

  // Fetch players: one page of the server's `limit` rows. The response carries no total, so a full page may have another after it.
  const fetchPlayers = useCallback((query: string, page: number) => {
    setPlayersLoading(true);
    setPlayersError(false);
    const controller = new AbortController();

    getPublicPlayers({ search: query, page: page > 1 ? page : undefined, signal: controller.signal })
      .then((res) => {
        if (controller.signal.aborted) return;
        setPlayersList(res.players);
        setPlayersHasMore(res.players.length >= res.limit);
        setPlayersLoading(false);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setPlayersError(true);
        setPlayersLoading(false);
      });

    return () => controller.abort();
  }, []);

  // One request per pause in the typing, not one per keystroke: the box's text becomes the query
  // `PLAYER_SEARCH_DEBOUNCE_MS` after its last change, from page 1.
  useEffect(() => {
    if (playerSearch === playerQuery) return;
    const timer = window.setTimeout(() => {
      setPlayerQuery(playerSearch);
      setPlayerPage(1);
    }, PLAYER_SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [playerSearch, playerQuery]);

  useEffect(() => {
    if (tab === "players") {
      return fetchPlayers(playerQuery, playerPage);
    }
  }, [tab, playerQuery, playerPage, fetchPlayers]);

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
      list = list.filter(
        (c) =>
          c.name.toLowerCase().includes(q) ||
          shownName(c.id, c.name).toLowerCase().includes(q) ||
          c.id.toLowerCase().includes(q),
      );
    }

    const flip = sortDir === "desc" ? -1 : 1;
    const language = locale === "en" ? undefined : locale;
    const compareNames = (a: { id: string; name: string }, b: { id: string; name: string }): number =>
      shownName(a.id, a.name).localeCompare(shownName(b.id, b.name), language);
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
      } else if (sortCol === "playedDelta") {
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        if (a.playedDelta !== null && b.playedDelta === null) return -1;
        if (a.playedDelta === null && b.playedDelta !== null) return 1;
        if (a.playedDelta !== null && b.playedDelta !== null && a.playedDelta !== b.playedDelta) {
          return (a.playedDelta - b.playedDelta) * flip;
        }
      } else if (sortCol === "rarity") {
        const rarities = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
        const diff = rarities.indexOf(a.rarity) - rarities.indexOf(b.rarity);
        if (diff !== 0) return diff * flip;
      } else if (sortCol === "name") {
        const diff = compareNames(a, b);
        if (diff !== 0) return diff * flip;
      }
      return a.games - b.games || compareNames(a, b);
    });

    return list;
  }, [cardStatsData, search, sortCol, sortDir, locale]);

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
          <h1 className="stats-title">{t("Statistics")}</h1>
        </header>

        {/* Summary tiles */}
        {cardStatsData && (
          <section className="stats-summary-tiles" data-testid={statsTestid.summaryTiles} aria-label={t("Statistics summary")}>
            <div className="stats-summary-tile" data-testid={statsTestid.summaryTotalGames}>
              <span className="stats-summary-label">{t("Games on patch {patch}", { patch: cardStatsData.patch })}</span>
              <span className="stats-summary-value">{summary ? formatNumber(summary.totalGames) : ""}</span>
            </div>
            {summary?.bestCard && (
              <div className="stats-summary-tile" data-testid={statsTestid.summaryBestCard}>
                <span className="stats-summary-label">{t("Top card (win rate)")}</span>
                <span className="stats-summary-highlight">
                  {shownName(summary.bestCard.id, summary.bestCard.name)} ({Math.round(summary.bestCard.winRate * 100)}%)
                </span>
              </div>
            )}
            {summary?.worstCard && (
              <div className="stats-summary-tile" data-testid={statsTestid.summaryWorstCard}>
                <span className="stats-summary-label">{t("Lowest card (win rate)")}</span>
                <span className="stats-summary-highlight">
                  {shownName(summary.worstCard.id, summary.worstCard.name)} ({Math.round(summary.worstCard.winRate * 100)}%)
                </span>
              </div>
            )}
          </section>
        )}

        {/* Provisional banner */}
        {isProvisional && (
          <div className="stats-provisional-banner" data-testid={statsTestid.provisionalBanner} role="alert">
            <div className="stats-provisional-info">
              <span className="stats-provisional-title">{t("Provisional statistics")}</span>
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
                {t("View previous patch ({patch}) →", { patch: cardStatsData.previousPatch })}
              </button>
            )}
          </div>
        )}

        {/* How to read this */}
        <details className="stats-info-accordion">
          <summary className="stats-info-summary">{t("How to read these statistics")}</summary>
          <div className="stats-info-content">
            <p>
              <strong>{t("Sample size threshold:")}</strong>{" "}
              {withNodes(
                t("Cards with fewer than {minSample} games display {phrase} instead of a percentage to prevent misleading sample artifacts.", {
                  minSample,
                }),
                { phrase: <em>{t('"not enough games"')}</em> },
              )}
            </p>
            <p>
              <strong>{t("When drawn vs Play rate:")}</strong>{" "}
              {withNodes(
                t(
                  "{whenDrawn} shows win rate when the card was in hand or played; {playRate} shows the percentage of all recorded decks containing the card.",
                ),
                { whenDrawn: <em>{t("When drawn")}</em>, playRate: <em>{t("Play rate")}</em> },
              )}
            </p>
          </div>
        </details>

        {/* Tabs */}
        <div className="stats-tabs" role="tablist" aria-label={t("Statistics views")}>
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
            {t("Cards")}
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
            {t("Players")}
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
                placeholder={t("Search card by name or id…")}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />

              <div className="stats-filter-selects">
                <select
                  className="stats-select"
                  value={setFilter}
                  onChange={(e) => setSetFilter(e.target.value)}
                  aria-label={t("Filter by set")}
                >
                  <option value="">{t("All sets")}</option>
                  {/* R1420: every set that ships, and only those. The server compares the value with each card's set, case folded. */}
                  {SHIPPED_SETS.map((set) => (
                    <option key={set} value={set.toLowerCase()}>
                      {t(set)}
                    </option>
                  ))}
                </select>

                <select
                  className="stats-select"
                  value={rarityFilter}
                  onChange={(e) => setRarityFilter(e.target.value)}
                  aria-label={t("Filter by rarity")}
                >
                  <option value="">{t("All rarities")}</option>
                  <option value="Common">{t("Common")}</option>
                  <option value="Rare">{t("Rare")}</option>
                  <option value="Epic">{t("Epic")}</option>
                  <option value="Legendary">{t("Legendary")}</option>
                  <option value="Mythic">{t("Mythic")}</option>
                </select>

                <select
                  className="stats-select"
                  value={costFilter}
                  onChange={(e) => setCostFilter(e.target.value)}
                  aria-label={t("Filter by cost")}
                >
                  <option value="">{t("All costs")}</option>
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
                <span className="stats-col-toggles-label">{t("Columns:")}</span>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.cost}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, cost: e.target.checked }))}
                  />
                  {t("Cost")}
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.rarity}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, rarity: e.target.checked }))}
                  />
                  {t("Rarity")}
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.games}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, games: e.target.checked }))}
                  />
                  {t("Games")}
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.drawnWinRate}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, drawnWinRate: e.target.checked }))}
                  />
                  {t("When drawn")}
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.playRate}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, playRate: e.target.checked }))}
                  />
                  {t("Play rate")}
                </label>
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    checked={visibleCols.playedDelta}
                    onChange={(e) => setVisibleCols((c) => ({ ...c, playedDelta: e.target.checked }))}
                  />
                  {t("Played Δ")}
                </label>
                {signedIn && (
                  <label className="stats-col-toggle">
                    <input
                      type="checkbox"
                      checked={visibleCols.personal}
                      onChange={(e) => setVisibleCols((c) => ({ ...c, personal: e.target.checked }))}
                    />
                    {t("Your stats")}
                  </label>
                )}
              </div>
            </div>

            {/* Table */}
            {cardStatsLoading ? (
              <p className="stats-loading">{t("Loading card statistics…")}</p>
            ) : cardStatsError ? (
              <div className="stats-error">
                <p>{t("Failed to load card statistics.")}</p>
                <button type="button" className="stats-retry-btn" onClick={() => fetchCardStats()}>
                  {t("Retry")}
                </button>
              </div>
            ) : filteredCards.length === 0 ? (
              <p className="stats-empty">{t("No cards match the selected filters.")}</p>
            ) : (
              <div className="stats-table-wrapper">
                <table className="stats-table" data-testid={statsTestid.cardsTable}>
                  <thead>
                    <tr>
                      {visibleCols.thumbnail && <th className="stats-th--thumb" scope="col">{t("Card")}</th>}
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
                        {t("Name")} {sortCol === "name" && (sortDir === "asc" ? "↑" : "↓")}
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
                          {t("Cost")}{sortCol === "cost" ? (sortDir === "asc" ? " ↑" : " ↓") : ""}
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
                          {t("Rarity")} {sortCol === "rarity" && (sortDir === "asc" ? "↑" : "↓")}
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
                          {t("Games")} {sortCol === "games" && (sortDir === "asc" ? "↑" : "↓")}
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
                        {t("Win rate")} {sortCol === "winRate" && (sortDir === "asc" ? "↑" : "↓")}
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
                          {t("When drawn")} {sortCol === "drawnWinRate" && (sortDir === "asc" ? "↑" : "↓")}
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
                          {t("Play rate")} {sortCol === "playRate" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {visibleCols.playedDelta && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "playedDelta" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("playedDelta")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("playedDelta");
                            }
                          }}
                          tabIndex={0}
                        >
                          {t("Played Δ")} {sortCol === "playedDelta" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {signedIn && visibleCols.personal && (
                        <th scope="col" className="stats-th">
                          {t("Your stats")}
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
                            <span className="stats-card-name-link">{shownName(card.id, card.name)}</span>
                          </td>
                          {visibleCols.cost && <td className="stats-td">{card.cost}</td>}
                          {visibleCols.rarity && (
                            <td className="stats-td">
                              <span className="stats-rarity-tag" data-rarity={card.rarity}>
                                {t(card.rarity)}
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
                              <span className="stats-not-enough">{t("not enough games")}</span>
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
                          {visibleCols.playedDelta && (
                            <td className="stats-td">
                              {card.hasEnoughGames && card.playedDelta !== null ? (
                                <span>
                                  {formatPlayedDelta(card.playedDelta)}{" "}
                                  <span className="stats-sample-hint">
                                    ({card.playedGames} / {card.drawnNotPlayedGames})
                                  </span>
                                </span>
                              ) : (
                                "—"
                              )}
                            </td>
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
                placeholder={t("Search player by username…")}
                value={playerSearch}
                onChange={(e) => setPlayerSearch(e.target.value)}
              />
              <p className="stats-player-note">
                {t("Player stats are public by default (opt-out in Settings). Elo ratings and rankings are kept separate.")}
              </p>
            </div>

            {playersLoading ? (
              <p className="stats-loading">{t("Loading players…")}</p>
            ) : playersError ? (
              <div className="stats-error">
                <p>{t("Failed to load player stats.")}</p>
                <button type="button" className="stats-retry-btn" onClick={() => fetchPlayers(playerQuery, playerPage)}>
                  {t("Retry")}
                </button>
              </div>
            ) : playersList.length === 0 ? (
              <p className="stats-empty">{t("No players found.")}</p>
            ) : (
              <div className="stats-table-wrapper">
                <table className="stats-table" data-testid={statsTestid.playersTable}>
                  <thead>
                    <tr>
                      <th scope="col" className="stats-th">{t("Player")}</th>
                      <th scope="col" className="stats-th">{t("Games")}</th>
                      <th scope="col" className="stats-th">{t("Win rate")}</th>
                      <th scope="col" className="stats-th">{t("Favourite card")}</th>
                      <th scope="col" className="stats-th">{t("Cards destroyed")}</th>
                      <th scope="col" className="stats-th">{t("Cards defeated")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {playersList.map((p) => {
                      const rate = p.winRate !== null ? `${Math.round(p.winRate * 100)}%` : "—";
                      const topCard = p.favouriteCards?.[0];
                      const favCardDef = topCard ? CATALOG[topCard.id] : undefined;
                      const favCardName = topCard ? (favCardDef ? readerName(favCardDef) : topCard.id) : "—";
                      return (
                        <tr key={p.profileId} className="stats-tr">
                          {/* R1436: the username, never the profile id the row is keyed by. */}
                          <td className="stats-td stats-td--player-name">
                            <Username name={p.username} />
                          </td>
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

            {!playersLoading && !playersError && (playerPage > 1 || playersHasMore) && (
              <div className="stats-pager" data-testid={statsTestid.playersPager}>
                <button
                  type="button"
                  className="stats-pager-btn"
                  data-testid={statsTestid.playersPrev}
                  disabled={playerPage <= 1}
                  onClick={() => setPlayerPage((page) => page - 1)}
                >
                  {t("Previous page")}
                </button>
                <span className="stats-pager-page">{t("Page {page}", { page: playerPage })}</span>
                <button
                  type="button"
                  className="stats-pager-btn"
                  data-testid={statsTestid.playersNext}
                  disabled={!playersHasMore}
                  onClick={() => setPlayerPage((page) => page + 1)}
                >
                  {t("Next page")}
                </button>
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
                  {drillDownDef ? readerName(drillDownDef) : cardDrillDownId}
                </h2>
                <button
                  type="button"
                  className="stats-drilldown-close"
                  data-testid={statsTestid.drillDownClose}
                  aria-label={t("Close card details")}
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
                    <p className="stats-loading">{t("Loading drill-down data…")}</p>
                  ) : drillDownData ? (
                    <>
                      {/* Patches historical win rate */}
                      <section className="stats-drilldown-section">
                        <h3>{t("Win rate by patch")}</h3>
                        {drillDownData.patches.length === 0 ? (
                          <p className="stats-empty-sub">{t("No earlier patches to show yet.")}</p>
                        ) : (
                          <ul className="stats-drilldown-list">
                            {drillDownData.patches.map((p) => (
                              <li key={p.patch} className="stats-drilldown-item">
                                <span className="stats-drilldown-label">{t("Patch {patch}:", { patch: p.patch })}</span>{" "}
                                <span>
                                  {p.winRate !== null ? `${Math.round(p.winRate * 100)}%` : t("not enough games")}{" "}
                                  {t("({games} games)", { games: p.games })}
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </section>

                      {/* Turn played distribution */}
                      <section className="stats-drilldown-section">
                        <h3>{t("Win rate by turn played")}</h3>
                        {drillDownData.byTurn.length === 0 ? (
                          <p className="stats-empty-sub">{t("No games recorded with this card played.")}</p>
                        ) : (
                          <ul className="stats-drilldown-list">
                            {drillDownData.byTurn.map((row) => (
                              <li key={row.turn} className="stats-drilldown-item">
                                <span className="stats-drilldown-label">{t("Turn {turn}:", { turn: row.turn })}</span>{" "}
                                <span>
                                  {row.winRate !== null ? `${Math.round(row.winRate * 100)}%` : "—"}{" "}
                                  {t("({games} games)", { games: row.games })}
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </section>

                      {/* Co-played synergy cards */}
                      <section className="stats-drilldown-section">
                        <h3>{t("Co-played cards (synergy)")}</h3>
                        {drillDownData.coPlayed.length === 0 ? (
                          <p className="stats-empty-sub">{t("No co-played cards recorded.")}</p>
                        ) : (
                          <ul className="stats-drilldown-list">
                            {drillDownData.coPlayed.map((c) => (
                              <li key={c.id} className="stats-drilldown-item">
                                <span className="stats-drilldown-label">
                                  {shownName(c.id, c.name)}
                                  {t(":")}
                                </span>{" "}
                                <span>
                                  {c.winRate !== null ? `${Math.round(c.winRate * 100)}%` : "—"}{" "}
                                  {t("({games} games)", { games: c.games })}
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </section>
                    </>
                  ) : (
                    <p className="stats-empty-sub">{t("Details unavailable.")}</p>
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
