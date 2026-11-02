// `/stats`: Public card and player statistics page (SPEC §9.11, R654).
//
// Accessible signed out and signed in.
// Provides three tabs:
//  - Cards: sortable, filterable table of card win rates and play rates; the filters, not columns, cover cost, rarity and keyword (R2446000).
//  - Sets: each set's cards on the patch (below).
//  - Players: public player aggregates (games, win rate, favorite cards, fun stats from #125).
// Features:
//  - No provisional banner and no reading notes (R2325000): the page never names a data source or the gate (R661); the patch bar below is the way back to the previous patch.
//  - Per-row sample floor (20 games): displays "not enough games" below threshold, and "not enough data"
//    for a Drawn vs not drawn the server withholds until both its sides reach the floor (R654, R2427000).
//  - URL search params for shareable views.
//  - Card drill-down modal showing CardFace (the Radiant face beside it on request, a full-art card's art as the viewer picks it (R2756000), a link to the card in
//    the Almanac and the tokens its text names), patch history, turn curve, and co-played synergy.
//  - Players tab: the search asks the server a pause after the typing, one page of the server's limit at a time.
//  - Patch bar (R1973002): a picker of the newest patches with enough games, and the patch before and after
//    the one shown; a patch is the window the cards share (R1973001), so a micro is never a step of its own.
//  - Sets tab (R1966000): each set's cards on the patch, pooled, by mean and by median, the change since the
//    previous patch, each set's best and worst card, and the cards furthest from 50% (stats/sets.ts).
//  - Games panel (R1994000): at the head of the Cards tab's scrolling panel, how the patch's games went
//    (stats/GamesPanel.tsx).
//  - Created cards (R2089000): below the Cards tab's table, the cards made during games, apart from the deck
//    cards and answering the same search box (stats/CreatedPanel.tsx).
//  - Tokens (R2370000): behind the Cards tab's Show tokens toggle, off by default, the tokens alone with each
//    one's win rate, above the Created cards (stats/TokensPanel.tsx).

import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement, type ReactNode } from "react";
import catalogJson from "@jackioh/cards/catalog.json";
import { KEYWORD_KINDS, SHIPPED_SETS, type CardDef, type CardDefs, type KeywordKind } from "@jackioh/shared";
import Username from "../auth/Username.tsx";
import { CardDefsProvider } from "../cards/index.ts";
import { chineseName, drawnInChinese, readerName, readerTerms } from "../cards/chinese.ts";
import { CardFace } from "../cards/CardFace.tsx";
import { FULL_ART_TIERS, FULL_ARTS_OF_CARD, FullArtShown, OwnCopy, fullArtPickOf, fullArtSource, isFullArtCard, type FullArtTier } from "../cards/fullArt.ts";
import { faceModel } from "../cards/model.ts";
import { useCardSettings } from "../cards/settings.ts";
import { EmptyState } from "../empty/EmptyState.tsx";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import {
  cardStatsPath,
  getCardDrillDown,
  getCardStats,
  getPublicPlayers,
  type CardDrillDownResponse,
  type CardStatsOptions,
  type PublicPlayerSummary,
  type PublicStatsCardsResponse,
} from "../net/api.ts";
import { useBackLayer } from "../net/backLayers.ts";
import { navigate, paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { CARD_STATS_MIN_SAMPLE, PLAYER_SEARCH_DEBOUNCE_MS, SET_STATS_DIGITS, SET_STATS_OUTLIER_Z } from "../stats/config.ts";
import { CreatedPanel } from "../stats/CreatedPanel.tsx";
import { GamesPanel } from "../stats/GamesPanel.tsx";
import { faceRate, formatPlayedDelta } from "../stats/model.ts";
import { neighbours, offeredPatches, patchIndex, patchName, patchTarget, type PatchWindow } from "../stats/patches.ts";
import { changes, leaders, outliers, setRows, type RatedCard } from "../stats/sets.ts";
import { TableScroll } from "../stats/TableScroll.tsx";
import { statsTestid } from "../stats/testids.ts";
import { TokensPanel } from "../stats/TokensPanel.tsx";
import { BackLink, followInApp } from "./nav.tsx";

import "../auth/tavern.css";
import "./stats.css";

const CATALOG = catalogJson as unknown as CardDefs;

/** Unit conversion, not configuration: a rate as a percentage has two more decimal places than as a fraction. */
const PERCENT_PLACES = 2;

/** A card's name as the reader reads it: the catalog's Chinese for a reader of Chinese, else the server's own. */
function shownName(id: string, name: string): string {
  return drawnInChinese(false) ? chineseName(id, name, CATALOG[id]) : name;
}

/** Whether a card answers the search box: the server's name, the name as the reader reads it, or the id holds the text. */
function matchesSearch(card: { id: string; name: string }, search: string): boolean {
  const q = search.trim().toLowerCase();
  return (
    card.name.toLowerCase().includes(q) ||
    shownName(card.id, card.name).toLowerCase().includes(q) ||
    card.id.toLowerCase().includes(q)
  );
}

function keywordKinds(id: string): KeywordKind[] {
  const def = CATALOG[id];
  if (!def) return [];
  const kinds = new Set<KeywordKind>();
  for (const kw of def.base?.keywords ?? []) {
    kinds.add(kw.kind as KeywordKind);
  }
  for (const kw of def.radiant?.keywords ?? []) {
    kinds.add(kw.kind as KeywordKind);
  }
  return KEYWORD_KINDS.filter((kind) => kinds.has(kind));
}

function keywordName(kind: KeywordKind): string {
  return readerTerms()?.keywords[kind] ?? kind;
}

/** The tokens a card's text names (`CardDef.refs`, R279), the card itself left out: what the drill-down shows it makes. */
function tokensOf(def: CardDef): CardDef[] {
  return (def.refs ?? []).flatMap((id) => {
    const named = CATALOG[id];
    return named !== undefined && named.token && id !== def.id ? [named] : [];
  });
}

/**
 * R2756000: the arts a card's drill-down offers: its normal face, then each tier it has a full art of, commonest
 * first (a listed token's are its parent's, R2655002); none for a card without full art.
 */
function artsOf(defId: string): DrillDownArt[] {
  if (!isFullArtCard(defId)) return [];
  const tiers = FULL_ARTS_OF_CARD.get(fullArtSource(defId));
  return ["normal", ...FULL_ART_TIERS.filter((tier) => tiers?.has(tier) === true)];
}

/** R2756000: `art` as the full-art pick `FullArtShown` takes for `defId`'s faces: `false` for the normal face, else that tier's (R2655000). */
function artPick(defId: string, art: DrillDownArt): boolean | string {
  const source = fullArtSource(defId);
  const id = art === "normal" ? undefined : FULL_ARTS_OF_CARD.get(source)?.get(art);
  return id === undefined ? false : fullArtPickOf(source, id);
}

/**
 * R1973002: the patch before the one a response shows, as its name and the version to ask its figures
 * at, from the response's patch list; a server without one names the version before.
 */
function previousPatchOf(res: PublicStatsCardsResponse): { name: string; target: string } | null {
  const list = res.patches ?? [];
  if (list.length > 0) {
    const before = neighbours(list, patchIndex(list, res.patch)).previous;
    return before ? { name: patchName(before), target: patchTarget(before) } : null;
  }
  return res.previousPatch ? { name: res.previousPatch, target: res.previousPatch } : null;
}

/** R1966000: a set's win rate, or its change, to `SET_STATS_DIGITS` places: sets differ by tenths of a point. */
function setPercent(rate: number, signed = false): string {
  return formatNumber(rate, {
    style: "percent",
    minimumFractionDigits: SET_STATS_DIGITS,
    maximumFractionDigits: SET_STATS_DIGITS,
    signDisplay: signed ? "exceptZero" : "auto",
  });
}

type TabKey = "cards" | "sets" | "players";
type ViewKey = "table" | "grid";
type SortColumn =
  | "name"
  | "games"
  | "winRate"
  | "drawnWinRate"
  | "drawnDelta"
  | "radiantWinRate"
  | "normalWinRate"
  | "playRate";
type SortDirection = "asc" | "desc";

/** R2756000: the art the drill-down's faces print: the normal face, or one of the card's full arts by tier. */
type DrillDownArt = "normal" | FullArtTier;

/** R2756000: each art choice's button, in English, put through `t()` where it is drawn as `SORT_LABELS` are. */
const ART_LABELS: Record<DrillDownArt, string> = { normal: "No full art", Epic: "Epic", Legendary: "Legendary", Mythic: "Mythic" };

const SORT_COLUMNS: readonly SortColumn[] = [
  "name",
  "games",
  "winRate",
  "drawnWinRate",
  "drawnDelta",
  "radiantWinRate",
  "normalWinRate",
  "playRate",
] as const;

const SORT_LABELS: Record<SortColumn, string> = {
  name: "Name",
  games: "Games",
  winRate: "Win rate",
  drawnWinRate: "When drawn",
  drawnDelta: "Drawn vs not drawn",
  radiantWinRate: "Radiant",
  normalWinRate: "Normal",
  playRate: "Play rate",
};

function defaultDirection(col: SortColumn): SortDirection {
  return col === "name" ? "asc" : "desc";
}

/**
 * R2446000: the sort an address asks for. One the table no longer offers (an old address's `cost`, `rarity` or
 * `keyword`) is the default, win rate descending, its direction too.
 */
function sortOf(params: URLSearchParams): { col: SortColumn; dir: SortDirection } {
  const asked = params.get("sort");
  const col = asked === null ? "winRate" : SORT_COLUMNS.find((known) => known === asked);
  if (col === undefined) return { col: "winRate", dir: "desc" };
  return { col, dir: params.get("dir") === "asc" ? "asc" : "desc" };
}

/** The Cards tab's read for these filters; an empty filter asks for nothing, as `fetchCardStats` always did. */
function cardStatsOptionsOf(patch: string, set: string, rarity: string, cost: string, mode: string): CardStatsOptions {
  return {
    patch: patch ? patch : undefined,
    set: set ? set : undefined,
    rarity: rarity ? rarity : undefined,
    cost: cost ? Number(cost) : undefined,
    mode: mode ? mode : undefined,
  };
}

/**
 * R2127000: the card drill-down's Back layer. It is mounted only while the drill-down is open, so Back
 * closes the drill-down as its Close does, and with it shut Back leaves the page as before.
 */
function DrillDownBackLayer({ onClose }: { onClose: () => void }): null {
  useBackLayer(onClose);
  return null;
}

export default function StatsRoute(): ReactElement {
  const locale = useLocale();
  // Read initial state from URL search params
  const initialParams = useMemo(() => new URLSearchParams(window.location.search), []);

  const [tab, setTab] = useState<TabKey>(() => {
    const asked = initialParams.get("tab");
    return asked === "sets" || asked === "players" ? asked : "cards";
  });
  const [patch, setPatch] = useState<string>(initialParams.get("patch") ?? "");
  const [search, setSearch] = useState<string>(initialParams.get("search") ?? "");
  const [sortCol, setSortCol] = useState<SortColumn>(() => sortOf(initialParams).col);
  const [sortDir, setSortDir] = useState<SortDirection>(() => sortOf(initialParams).dir);
  const [setFilter, setSetFilter] = useState<string>(initialParams.get("set") ?? "");
  const [rarityFilter, setRarityFilter] = useState<string>(initialParams.get("rarity") ?? "");
  const [costFilter, setCostFilter] = useState<string>(initialParams.get("cost") ?? "");
  const [keywordFilter, setKeywordFilter] = useState<string>(initialParams.get("keyword") ?? "");
  const [view, setView] = useState<ViewKey>(initialParams.get("view") === "grid" ? "grid" : "table");
  const [cardDrillDownId, setCardDrillDownId] = useState<string | null>(initialParams.get("card"));
  // R2370000: the Tokens section, off unless the address asks for it.
  const [showTokens, setShowTokens] = useState(initialParams.get("tokens") === "1");
  const [mode, setMode] = useState<string>(
    ["bo1", "bo3", "random"].includes(initialParams.get("mode") ?? "")
      ? (initialParams.get("mode") as string)
      : "",
  );

  // Column visibility
  const [visibleCols, setVisibleCols] = useState({
    thumbnail: true,
    games: true,
    winRate: true,
    drawnWinRate: true,
    drawnDelta: true,
    radiantWinRate: true,
    normalWinRate: true,
    playRate: true,
  });

  // Card stats response. R2125000: a revisit starts on the table these filters last read and reads it again behind it.
  const [cardStatsKept] = useState(() =>
    cachedRead<PublicStatsCardsResponse>(
      null,
      cardStatsPath(cardStatsOptionsOf(patch, setFilter, rarityFilter, costFilter, mode)),
    ),
  );
  const [cardStatsLoading, setCardStatsLoading] = useState(cardStatsKept === undefined);
  const [cardStatsError, setCardStatsError] = useState(false);
  const [cardStatsData, setCardStatsData] = useState<PublicStatsCardsResponse | null>(cardStatsKept ?? null);
  // Bumped by Retry: the effect asks again, so its cleanup cancels a retry like any other request.
  const [cardStatsAttempt, setCardStatsAttempt] = useState(0);

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
  const [drillDownError, setDrillDownError] = useState(false);
  // Bumped by the details' Retry, as `cardStatsAttempt` is by the table's.
  const [drillDownAttempt, setDrillDownAttempt] = useState(0);
  // The drill-down's "Show Radiant": the Radiant face beside the normal one, kept from card to card.
  const [drillDownRadiant, setDrillDownRadiant] = useState(false);
  // R2756000: the drill-down's art, kept from card to card as Show Radiant is; null until the viewer picks one,
  // when the faces print what the Full-art cards setting says.
  const [drillDownArt, setDrillDownArt] = useState<DrillDownArt | null>(null);
  const { fullArt: fullArtSetting } = useCardSettings();

  // R1966000: the Sets tab's patch, every set's cards unfiltered, and its previous patch's for the change.
  const [setsData, setSetsData] = useState<PublicStatsCardsResponse | null>(null);
  const [setsPrevious, setSetsPrevious] = useState<PublicStatsCardsResponse | null>(null);
  const [setsError, setSetsError] = useState(false);
  // Bumped by the Sets tab's Retry, as `cardStatsAttempt` is by the table's.
  const [setsAttempt, setSetsAttempt] = useState(0);

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
    if (keywordFilter) params.set("keyword", keywordFilter);
    if (view !== "table") params.set("view", view);
    if (cardDrillDownId) params.set("card", cardDrillDownId);
    if (mode) params.set("mode", mode);
    if (showTokens) params.set("tokens", "1");

    const query = params.toString();
    const target = query ? `${paths.stats}?${query}` : paths.stats;
    navigate(target, { replace: true });
  }, [tab, patch, search, sortCol, sortDir, setFilter, rarityFilter, costFilter, keywordFilter, view, cardDrillDownId, mode, showTokens]);

  useEffect(() => {
    updateUrl();
  }, [updateUrl]);

  // Fetch card stats
  const fetchCardStats = useCallback(
    () => {
      const options = cardStatsOptionsOf(patch, setFilter, rarityFilter, costFilter, mode);
      // R2125000: a table kept for these filters draws at once; only a first visit to them waits behind a skeleton.
      const kept = cachedRead<PublicStatsCardsResponse>(null, cardStatsPath(options));
      if (kept === undefined) {
        setCardStatsLoading(true);
      } else {
        setCardStatsData(kept);
        setCardStatsLoading(false);
      }
      setCardStatsError(false);
      const controller = new AbortController();

      getCardStats({ ...options, signal: controller.signal })
        .then((res) => {
          if (controller.signal.aborted) return;
          setCardStatsData(res);
          setCardStatsLoading(false);
        })
        .catch(() => {
          if (controller.signal.aborted) return;
          // A read that fails behind a kept table leaves it and says nothing.
          if (kept !== undefined) return;
          setCardStatsError(true);
          setCardStatsLoading(false);
        });

      return () => controller.abort();
    },
    [patch, setFilter, rarityFilter, costFilter, mode],
  );

  useEffect(() => {
    return fetchCardStats();
  }, [fetchCardStats, cardStatsAttempt]);

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

  // R1966000: the Sets tab reads the patch's every card, whatever the Cards tab filters, then the previous
  // patch's for each set's change; a previous patch that fails to load only leaves the change out.
  useEffect(() => {
    if (tab !== "sets") return;
    setSetsError(false);
    const controller = new AbortController();
    getCardStats({ patch: patch ? patch : undefined, mode: mode ? mode : undefined, signal: controller.signal })
      .then((res) => {
        if (controller.signal.aborted) return;
        setSetsData(res);
        setSetsPrevious(null);
        // R1973002: the patch before, not the version before, which may be a micro of the same patch.
        const before = previousPatchOf(res);
        if (!before) return;
        getCardStats({ patch: before.target, mode: mode ? mode : undefined, signal: controller.signal })
          .then((previous) => {
            if (!controller.signal.aborted) setSetsPrevious(previous);
          })
          .catch(() => undefined);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setSetsError(true);
      });
    return () => controller.abort();
  }, [tab, patch, mode, setsAttempt]);

  // Fetch card drill-down
  useEffect(() => {
    if (!cardDrillDownId) {
      setDrillDownData(null);
      return;
    }
    setDrillDownLoading(true);
    setDrillDownError(false);
    const controller = new AbortController();
    getCardDrillDown(cardDrillDownId, mode ? mode : undefined, controller.signal)
      .then((data) => {
        if (controller.signal.aborted) return;
        setDrillDownData(data);
        setDrillDownLoading(false);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setDrillDownError(true);
        setDrillDownLoading(false);
      });

    return () => controller.abort();
  }, [cardDrillDownId, mode, drillDownAttempt]);

  // The drill-down is a modal like the client's others (ConfirmConcede.tsx; docs/polish/6-cards.md,
  // B25): the focus moves to its Close when it opens, Tab and Shift+Tab stay inside, Escape and Back
  // (R2127000, `DrillDownBackLayer`) close it, and the control that opened it has the focus back once
  // it closes. That control is kept when it is pressed, not read off the focus, which a click on a
  // button does not move in every browser.
  const drillDownOpener = useRef<HTMLElement | null>(null);
  const drillDownDialog = useRef<HTMLDivElement>(null);
  const drillDownCloseButton = useRef<HTMLButtonElement>(null);
  const drillDownOpen = Boolean(cardDrillDownId);

  const openDrillDown = (id: string, opener: HTMLElement): void => {
    drillDownOpener.current = opener;
    setCardDrillDownId(id);
  };

  useEffect(() => {
    if (!drillDownOpen) return;
    drillDownCloseButton.current?.focus({ preventScroll: true });
    // Both keys are read at the window, in the capture phase, so they hold wherever the focus is.
    function onKey(event: globalThis.KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        setCardDrillDownId(null);
        return;
      }
      if (event.key !== "Tab") return;
      const dialog = drillDownDialog.current;
      if (dialog === null) return;
      const order = [...dialog.querySelectorAll<HTMLElement>("button:not([disabled]), a[href]")];
      if (order.length === 0) return;
      const at = order.indexOf(document.activeElement as HTMLElement);
      const next = event.shiftKey ? (at <= 0 ? order.length - 1 : at - 1) : at === order.length - 1 ? 0 : at + 1;
      event.preventDefault();
      event.stopPropagation();
      order[next]?.focus();
    }
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      const opener = drillDownOpener.current;
      if (opener !== null && opener.isConnected) opener.focus({ preventScroll: true });
    };
  }, [drillDownOpen]);

  const minSample = cardStatsData?.minSample ?? CARD_STATS_MIN_SAMPLE;

  // R1966000: the Sets tab's figures, off the cards it read; R1973002: compared with the patch before.
  const setsSince = setsData ? previousPatchOf(setsData)?.name : undefined;
  const setsMinSample = setsData?.minSample ?? CARD_STATS_MIN_SAMPLE;
  const sets = useMemo(() => (setsData ? setRows(setsData.cards, setsMinSample) : []), [setsData, setsMinSample]);
  const setLeaders = useMemo(() => leaders(sets), [sets]);
  const setChanges = useMemo(
    () => changes(sets, setsPrevious ? setRows(setsPrevious.cards, setsPrevious.minSample) : []),
    [sets, setsPrevious],
  );
  const setOutliers = useMemo(
    () => (setsData ? outliers(setsData.cards, setsMinSample) : { strongest: [], weakest: [] }),
    [setsData, setsMinSample],
  );

  const keywordOptions = useMemo(() => {
    const present = new Set<KeywordKind>();
    for (const [id, def] of Object.entries(CATALOG)) {
      if (def.token) continue;
      for (const kind of keywordKinds(id)) {
        present.add(kind);
      }
    }
    return KEYWORD_KINDS.filter((kind) => present.has(kind));
  }, []);

  // Toggle sort
  const handleSort = (col: SortColumn) => {
    if (sortCol === col) {
      setSortDir((prev) => (prev === "asc" ? "desc" : "asc"));
    } else {
      setSortCol(col);
      setSortDir(defaultDirection(col));
    }
  };

  // Filter and sort cards
  const filteredCards = useMemo(() => {
    if (!cardStatsData) return [];
    let list = [...cardStatsData.cards];

    if (search.trim()) {
      list = list.filter((c) => matchesSearch(c, search));
    }

    if (keywordFilter) {
      list = list.filter((c) => keywordKinds(c.id).includes(keywordFilter as KeywordKind));
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
      } else if (sortCol === "radiantWinRate") {
        const aRate = faceRate(a.playedRadiantGames, a.playedRadiantWinRate, minSample);
        const bRate = faceRate(b.playedRadiantGames, b.playedRadiantWinRate, minSample);
        if (aRate !== null && bRate === null) return -1;
        if (aRate === null && bRate !== null) return 1;
        if (aRate !== null && bRate !== null && aRate !== bRate) {
          return (aRate - bRate) * flip;
        }
      } else if (sortCol === "normalWinRate") {
        const aRate = faceRate(a.playedNormalGames, a.playedNormalWinRate, minSample);
        const bRate = faceRate(b.playedNormalGames, b.playedNormalWinRate, minSample);
        if (aRate !== null && bRate === null) return -1;
        if (aRate === null && bRate !== null) return 1;
        if (aRate !== null && bRate !== null && aRate !== bRate) {
          return (aRate - bRate) * flip;
        }
      } else if (sortCol === "games") {
        if (a.games !== b.games) return (a.games - b.games) * flip;
      } else if (sortCol === "playRate") {
        if (a.playRate !== b.playRate) return (a.playRate - b.playRate) * flip;
      } else if (sortCol === "drawnDelta") {
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        const aDelta = a.drawnDelta ?? null;
        const bDelta = b.drawnDelta ?? null;
        if (aDelta !== null && bDelta === null) return -1;
        if (aDelta === null && bDelta !== null) return 1;
        if (aDelta !== null && bDelta !== null && aDelta !== bDelta) {
          return (aDelta - bDelta) * flip;
        }
      } else if (sortCol === "name") {
        const diff = compareNames(a, b);
        if (diff !== 0) return diff * flip;
      }
      return a.games - b.games || compareNames(a, b);
    });

    return list;
  }, [cardStatsData, search, keywordFilter, sortCol, sortDir, locale, minSample]);

  // R2089000: the Created cards in the server's order, those the search box asks for as the deck rows are.
  const createdCards = (cardStatsData?.created ?? []).filter((card) => matchesSearch(card, search));
  // R2370000: of those, the catalog's tokens.
  const tokenCards = createdCards.filter((card) => CATALOG[card.id]?.token === true);

  const summary = cardStatsData?.summary;

  // R1973002: the patch shown, the picker's choices and the patches either side of it. The newest is
  // asked for with no `?patch=`, so the address stays the plain one.
  const patchList = cardStatsData?.patches ?? [];
  const shownPatch = cardStatsData ? patchIndex(patchList, cardStatsData.patch) : -1;
  const pickable = offeredPatches(patchList, shownPatch);
  const { previous: previousWindow, next: nextWindow } = neighbours(patchList, shownPatch);
  const choosePatch = (window: PatchWindow): void => setPatch(patchList.indexOf(window) === 0 ? "" : patchTarget(window));

  const drillDownDef = cardDrillDownId ? CATALOG[cardDrillDownId] : undefined;
  const drillDownFace = drillDownDef ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: false }) : null;
  const drillDownRadiantFace =
    drillDownDef && drillDownRadiant ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: true }) : null;
  const drillDownTokens = drillDownDef ? tokensOf(drillDownDef) : [];
  // R2756000: the arts offered and the one the faces print: the one picked, else the setting's Epic or the normal face.
  const drillDownArts = drillDownDef ? artsOf(drillDownDef.id) : [];
  const shownArt: DrillDownArt = drillDownArt ?? (fullArtSetting ? "Epic" : "normal");

  return (
    <CardDefsProvider defs={CATALOG}>
      <div className="app-shell app-shell--wide tavern stats-page one-screen" data-testid={statsTestid.screen}>
        <header className="stats-header">
          <BackLink />
          <h1 className="stats-title">{t("Statistics")}</h1>
        </header>

        {/* The summary: the patch's games alone (R661); R1646000 took the top and lowest card tiles off the fixed top */}
        {cardStatsData && (
          <section className="stats-summary-tiles" data-testid={statsTestid.summaryTiles} aria-label={t("Statistics summary")}>
            <div className="stats-summary-tile" data-testid={statsTestid.summaryTotalGames}>
              <span className="stats-summary-label">
                {t("Games on patch {patch}", {
                  patch: shownPatch >= 0 ? patchName(patchList[shownPatch]!) : cardStatsData.patch,
                })}
              </span>
              <span className="stats-summary-value">{summary ? formatNumber(summary.totalGames) : ""}</span>
            </div>
          </section>
        )}

        {/* R1973002: the patch picker, and the patch before and after the one shown */}
        {patchList.length > 0 && (
          <nav className="stats-patch-bar" data-testid={statsTestid.patchBar} aria-label={t("Patches")}>
            <button
              type="button"
              className="stats-pager-btn"
              data-testid={statsTestid.patchPrevious}
              disabled={previousWindow === null}
              onClick={() => previousWindow && choosePatch(previousWindow)}
            >
              {previousWindow ? t("← Previous patch ({patch})", { patch: patchName(previousWindow) }) : t("← Previous patch")}
            </button>
            <select
              className="stats-select"
              data-testid={statsTestid.patchSelect}
              aria-label={t("Patch")}
              value={shownPatch >= 0 ? patchTarget(patchList[shownPatch]!) : ""}
              onChange={(e) => {
                const chosen = patchList.find((window) => patchTarget(window) === e.target.value);
                if (chosen) choosePatch(chosen);
              }}
            >
              {shownPatch < 0 && <option value="">{cardStatsData?.patch}</option>}
              {pickable.map((window) => (
                <option key={patchTarget(window)} value={patchTarget(window)}>
                  {t("{patch} · {games} games", { patch: patchName(window), games: formatNumber(window.games) })}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="stats-pager-btn"
              data-testid={statsTestid.patchNext}
              disabled={nextWindow === null}
              onClick={() => nextWindow && choosePatch(nextWindow)}
            >
              {nextWindow ? t("Next patch ({patch}) →", { patch: patchName(nextWindow) }) : t("Next patch →")}
            </button>
          </nav>
        )}

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
            id="stats-tab-sets"
            aria-selected={tab === "sets"}
            aria-controls="stats-panel-sets"
            className={`stats-tab ${tab === "sets" ? "stats-tab--active" : ""}`}
            data-testid={statsTestid.tabSets}
            onClick={() => setTab("sets")}
          >
            {t("Sets")}
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
          <section id="stats-panel-cards" role="tabpanel" aria-labelledby="stats-tab-cards" className="stats-panel one-screen__body">
            {/* R1994000: how the patch's games went, in the scrolling panel, not the fixed top (R1646000) */}
            <GamesPanel patch={patch} mode={mode} />

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

                <select
                  className="stats-select"
                  value={keywordFilter}
                  onChange={(e) => setKeywordFilter(e.target.value)}
                  aria-label={t("Filter by keyword")}
                >
                  <option value="">{t("All keywords")}</option>
                  {keywordOptions.map((kind) => (
                    <option key={kind} value={kind}>
                      {keywordName(kind)}
                    </option>
                  ))}
                </select>

                <select
                  className="stats-select"
                  data-testid={statsTestid.modeSelect}
                  value={mode}
                  onChange={(e) => setMode(e.target.value)}
                  aria-label={t("Filter by match type")}
                >
                  <option value="">{t("All modes")}</option>
                  <option value="bo1">{t("Best of 1")}</option>
                  <option value="bo3">{t("Conquest")}</option>
                  <option value="random">{t("All Random")}</option>
                </select>
              </div>

              <div className="stats-sort">
                <span className="stats-sort-label" id="stats-sort-label">
                  {t("Sort")}
                </span>
                <select
                  className="stats-select"
                  aria-labelledby="stats-sort-label"
                  data-testid={statsTestid.sortSelect}
                  value={sortCol}
                  onChange={(e) => {
                    const next = e.target.value as SortColumn;
                    setSortCol(next);
                    setSortDir(defaultDirection(next));
                  }}
                >
                  {SORT_COLUMNS.map((col) => (
                    <option key={col} value={col}>
                      {t(SORT_LABELS[col])}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="stats-sort-dir link-button"
                  data-testid={statsTestid.sortDir}
                  aria-label={
                    sortDir === "asc"
                      ? t("Ascending; switch to descending")
                      : t("Descending; switch to ascending")
                  }
                  title={sortDir === "asc" ? t("Ascending") : t("Descending")}
                  onClick={() => setSortDir((d) => (d === "asc" ? "desc" : "asc"))}
                >
                  {sortDir === "asc" ? "↑" : "↓"}
                </button>
              </div>

              <div className="stats-view-toggle" role="group" aria-label={t("View")}>
                <button
                  type="button"
                  className={`stats-view-btn ${view === "table" ? "stats-view-btn--active" : ""}`}
                  data-testid={statsTestid.viewTable}
                  aria-pressed={view === "table"}
                  onClick={() => setView("table")}
                >
                  {t("Table")}
                </button>
                <button
                  type="button"
                  className={`stats-view-btn ${view === "grid" ? "stats-view-btn--active" : ""}`}
                  data-testid={statsTestid.viewGrid}
                  aria-pressed={view === "grid"}
                  onClick={() => setView("grid")}
                >
                  {t("Grid")}
                </button>
              </div>

              {/* R2370000: the Tokens section, off by default */}
              <div className="stats-col-toggles">
                <label className="stats-col-toggle">
                  <input
                    type="checkbox"
                    data-testid={statsTestid.tokensToggle}
                    checked={showTokens}
                    onChange={(e) => setShowTokens(e.target.checked)}
                  />
                  {t("Show tokens")}
                </label>
              </div>

              {/* Column visibility toggles */}
              {view === "table" && (
                <div className="stats-col-toggles">
                  <span className="stats-col-toggles-label">{t("Columns:")}</span>
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
                      checked={visibleCols.drawnDelta}
                      onChange={(e) => setVisibleCols((c) => ({ ...c, drawnDelta: e.target.checked }))}
                    />
                    {t("Drawn vs not drawn")}
                  </label>
                  <label className="stats-col-toggle">
                    <input
                      type="checkbox"
                      checked={visibleCols.radiantWinRate}
                      onChange={(e) => setVisibleCols((c) => ({ ...c, radiantWinRate: e.target.checked }))}
                    />
                    {t("Radiant")}
                  </label>
                  <label className="stats-col-toggle">
                    <input
                      type="checkbox"
                      checked={visibleCols.normalWinRate}
                      onChange={(e) => setVisibleCols((c) => ({ ...c, normalWinRate: e.target.checked }))}
                    />
                    {t("Normal")}
                  </label>
                  <label className="stats-col-toggle">
                    <input
                      type="checkbox"
                      checked={visibleCols.playRate}
                      onChange={(e) => setVisibleCols((c) => ({ ...c, playRate: e.target.checked }))}
                    />
                    {t("Play rate")}
                  </label>
                </div>
              )}
            </div>

            {/* Table or Grid */}
            {cardStatsLoading ? (
              <Skeleton variant={view === "grid" ? "grid" : "table"} label={t("Loading card statistics…")} />
            ) : cardStatsError ? (
              <div className="stats-error">
                <p>{t("The card statistics didn't load.")}</p>
                <button type="button" className="stats-retry-btn" onClick={() => setCardStatsAttempt((n) => n + 1)}>
                  {t("Retry")}
                </button>
              </div>
            ) : filteredCards.length === 0 ? (
              <EmptyState text={t("No cards match the selected filters.")} />
            ) : view === "grid" ? (
              <ul className="stats-grid" data-testid={statsTestid.cardsGrid}>
                {filteredCards.map((card) => {
                  const def = CATALOG[card.id];
                  const face = def ? faceModel({ defId: card.id, def, radiant: false }) : null;
                  const radiant = faceRate(card.playedRadiantGames, card.playedRadiantWinRate, minSample);
                  const normal = faceRate(card.playedNormalGames, card.playedNormalWinRate, minSample);

                  return (
                    <li key={card.id} className="stats-grid-item" data-card-id={card.id}>
                      <button
                        type="button"
                        className="stats-grid-card"
                        onClick={(e) => openDrillDown(card.id, e.currentTarget)}
                        aria-label={shownName(card.id, card.name)}
                      >
                        {face && (
                          <div className="stats-grid-face">
                            <CardFace face={face} layout="full" lazyArt />
                          </div>
                        )}
                      </button>
                      <dl className="stats-grid-figures">
                        <dt>{t("Win rate")}</dt>
                        <dd>
                          {card.hasEnoughGames && card.winRate !== null
                            ? `${Math.round(card.winRate * 100)}%`
                            : t("not enough games")}
                        </dd>
                        <dt>{t("Radiant")}</dt>
                        <dd>{radiant !== null ? `${Math.round(radiant * 100)}%` : "—"}</dd>
                        <dt>{t("Normal")}</dt>
                        <dd>{normal !== null ? `${Math.round(normal * 100)}%` : "—"}</dd>
                        <dt>{t("Games")}</dt>
                        <dd>{card.games}</dd>
                      </dl>
                    </li>
                  );
                })}
              </ul>
            ) : (
              <TableScroll labeledBy="stats-tab-cards">
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
                      {visibleCols.drawnDelta && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "drawnDelta" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("drawnDelta")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("drawnDelta");
                            }
                          }}
                          tabIndex={0}
                        >
                          {t("Drawn vs not drawn")} {sortCol === "drawnDelta" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {visibleCols.radiantWinRate && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "radiantWinRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("radiantWinRate")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("radiantWinRate");
                            }
                          }}
                          tabIndex={0}
                        >
                          {t("Radiant")} {sortCol === "radiantWinRate" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      {visibleCols.normalWinRate && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={sortCol === "normalWinRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("normalWinRate")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("normalWinRate");
                            }
                          }}
                          tabIndex={0}
                        >
                          {t("Normal")} {sortCol === "normalWinRate" && (sortDir === "asc" ? "↑" : "↓")}
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
                    </tr>
                  </thead>
                  <tbody>
                    {filteredCards.map((card) => {
                      const radiantRate = faceRate(card.playedRadiantGames, card.playedRadiantWinRate, minSample);
                      const normalRate = faceRate(card.playedNormalGames, card.playedNormalWinRate, minSample);

                      return (
                        <tr
                          key={card.id}
                          className="stats-tr"
                          data-card-id={card.id}
                          onClick={(e) => openDrillDown(card.id, e.currentTarget)}
                          role="button"
                          tabIndex={0}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              openDrillDown(card.id, e.currentTarget);
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
                          {visibleCols.drawnDelta && (
                            <td className="stats-td">
                              {card.hasEnoughGames && typeof card.drawnDelta === "number" ? (
                                <span>
                                  {formatPlayedDelta(card.drawnDelta)}{" "}
                                  <span className="stats-sample-hint">
                                    ({card.drawnGames} / {card.notDrawnGames ?? 0})
                                  </span>
                                </span>
                              ) : (
                                <span className="stats-not-enough">{t("not enough data")}</span>
                              )}
                            </td>
                          )}
                          {visibleCols.radiantWinRate && (
                            <td className="stats-td">
                              {radiantRate !== null ? `${Math.round(radiantRate * 100)}%` : "—"}
                            </td>
                          )}
                          {visibleCols.normalWinRate && (
                            <td className="stats-td">
                              {normalRate !== null ? `${Math.round(normalRate * 100)}%` : "—"}
                            </td>
                          )}
                          {visibleCols.playRate && (
                            <td className="stats-td">{`${Math.round(card.playRate * 100)}%`}</td>
                          )}
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </TableScroll>
            )}

            {/* R2370000: the tokens alone, each with its win rate */}
            {showTokens && !cardStatsLoading && !cardStatsError && (
              <TokensPanel tokens={tokenCards} minSample={minSample} nameOf={shownName} />
            )}

            {/* R2089000: the Created cards, apart from the deck cards above */}
            {!cardStatsLoading && !cardStatsError && (
              <CreatedPanel created={createdCards} minSample={minSample} nameOf={shownName} />
            )}
          </section>
        )}

        {/* SETS TAB (R1966000) */}
        {tab === "sets" && (
          <section id="stats-panel-sets" role="tabpanel" aria-labelledby="stats-tab-sets" className="stats-panel one-screen__body">
            {setsError ? (
              <div className="stats-error">
                <p>{t("The set statistics didn't load.")}</p>
                <button
                  type="button"
                  className="stats-retry-btn"
                  data-testid={statsTestid.setsRetry}
                  onClick={() => setSetsAttempt((attempt) => attempt + 1)}
                >
                  {t("Retry")}
                </button>
              </div>
            ) : setsData === null ? (
              <Skeleton variant="table" label={t("Loading set statistics…")} />
            ) : sets.length === 0 ? (
              <EmptyState text={t("No games on patch {patch} yet.", { patch: setsData.patch })} />
            ) : (
              <>
                <p className="stats-player-note">
                  {t(
                    "How each set's cards do on patch {patch}, in decks that mix every set. A set's win rate pools its cards' games; its mean and median are over its cards with {minSample} games or more.",
                    { patch: setsData.patch, minSample: setsMinSample },
                  )}
                </p>
                <section className="stats-summary-tiles stats-sets-leaders" data-testid={statsTestid.setsLeaders} aria-label={t("Best sets")}>
                  <div className="stats-summary-tile">
                    <span className="stats-summary-label">{t("Highest win rate")}</span>
                    <span className="stats-summary-value">{setLeaders.pooled ? t(setLeaders.pooled) : "—"}</span>
                  </div>
                  <div className="stats-summary-tile">
                    <span className="stats-summary-label">{t("Highest mean card")}</span>
                    <span className="stats-summary-value">{setLeaders.mean ? t(setLeaders.mean) : "—"}</span>
                  </div>
                  <div className="stats-summary-tile">
                    <span className="stats-summary-label">{t("Highest median card")}</span>
                    <span className="stats-summary-value">{setLeaders.median ? t(setLeaders.median) : "—"}</span>
                  </div>
                </section>
                <TableScroll labeledBy="stats-tab-sets">
                  <table className="stats-table" data-testid={statsTestid.setsTable}>
                    <thead>
                      <tr>
                        <th scope="col" className="stats-th">{t("Set")}</th>
                        <th scope="col" className="stats-th">{t("Cards")}</th>
                        <th scope="col" className="stats-th">{t("Card-games")}</th>
                        <th scope="col" className="stats-th">{t("Win rate")}</th>
                        <th scope="col" className="stats-th">{t("Mean")}</th>
                        <th scope="col" className="stats-th">{t("Median")}</th>
                        <th scope="col" className="stats-th">
                          {setsSince ? t("Since {patch}", { patch: setsSince }) : t("Change")}
                        </th>
                        <th scope="col" className="stats-th">{t("Best card")}</th>
                        <th scope="col" className="stats-th">{t("Worst card")}</th>
                        <th scope="col" className="stats-th">{t("Strong")}</th>
                        <th scope="col" className="stats-th">{t("Weak")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {sets.map((row) => {
                        // Rounded first, so a change too small to show reads 0.0% and takes no color.
                        const changed = setChanges.get(row.set);
                        const change = changed === undefined ? undefined : Number(changed.toFixed(SET_STATS_DIGITS + PERCENT_PLACES));
                        const cardCell = (card: RatedCard | null): ReactNode =>
                          card === null ? (
                            <span className="stats-not-enough">{t("not enough games")}</span>
                          ) : (
                            <button
                              type="button"
                              className="link-button stats-card-name-link"
                              onClick={(e) => openDrillDown(card.id, e.currentTarget)}
                            >
                              {shownName(card.id, card.name)} <span className="stats-sample-hint">{setPercent(card.winRate)}</span>
                            </button>
                          );
                        return (
                          <tr key={row.set} className="stats-tr" data-set={row.set}>
                            <th scope="row" className="stats-td stats-td--name">
                              {t(row.set)}
                              {row.set === setLeaders.pooled && (
                                <span className="stats-sets-leader" aria-hidden="true">
                                  {" ★"}
                                </span>
                              )}
                            </th>
                            <td className="stats-td">
                              {formatNumber(row.rated)}
                              {row.rated !== row.cards && (
                                <span className="stats-sample-hint"> {t("of {cards}", { cards: row.cards })}</span>
                              )}
                            </td>
                            <td className="stats-td">{formatNumber(row.games)}</td>
                            <td className="stats-td stats-td--winrate">
                              {row.pooled === null ? (
                                "—"
                              ) : (
                                <>
                                  <span className="stats-winrate-text">{setPercent(row.pooled)}</span>
                                  {row.error !== null && (
                                    <span className="stats-sample-hint"> ±{setPercent(row.error)}</span>
                                  )}
                                </>
                              )}
                            </td>
                            <td className="stats-td">{row.mean === null ? "—" : setPercent(row.mean)}</td>
                            <td className="stats-td">{row.median === null ? "—" : setPercent(row.median)}</td>
                            <td className="stats-td">
                              {change === undefined ? (
                                "—"
                              ) : (
                                <span className={`stats-sets-change stats-sets-change--${change > 0 ? "up" : change < 0 ? "down" : "flat"}`}>
                                  {setPercent(change, true)}
                                </span>
                              )}
                            </td>
                            <td className="stats-td">{cardCell(row.best)}</td>
                            <td className="stats-td">{cardCell(row.worst)}</td>
                            <td className="stats-td">{formatNumber(row.strong)}</td>
                            <td className="stats-td">{formatNumber(row.weak)}</td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </TableScroll>
                <div className="stats-sets-outliers" data-testid={statsTestid.setsOutliers}>
                  {(
                    [
                      ["strongest", t("Strongest cards"), setOutliers.strongest],
                      ["weakest", t("Weakest cards"), setOutliers.weakest],
                    ] as const
                  ).map(([key, title, list]) => (
                    <section key={key} className="stats-sets-outlier-list" aria-label={title}>
                      <h2 className="stats-sets-outlier-title">{title}</h2>
                      {list.length === 0 ? (
                        <p className="stats-empty-sub">
                          {t("No card is {z} margins or more from 50% yet.", { z: SET_STATS_OUTLIER_Z })}
                        </p>
                      ) : (
                        <ol className="stats-sets-outlier-cards">
                          {list.map((card) => (
                            <li key={card.id} data-card-id={card.id}>
                              <button
                                type="button"
                                className="link-button stats-card-name-link"
                                onClick={(e) => openDrillDown(card.id, e.currentTarget)}
                              >
                                {shownName(card.id, card.name)}
                              </button>{" "}
                              <span className="stats-sample-hint">
                                {t("{set} · {rate} over {games} games · {z} margins", {
                                  set: t(card.set),
                                  rate: setPercent(card.winRate),
                                  games: formatNumber(card.games),
                                  z: formatNumber(card.z, {
                                    minimumFractionDigits: SET_STATS_DIGITS,
                                    maximumFractionDigits: SET_STATS_DIGITS,
                                    signDisplay: "exceptZero",
                                  }),
                                })}
                              </span>
                            </li>
                          ))}
                        </ol>
                      )}
                    </section>
                  ))}
                </div>
                <p className="stats-sample-hint stats-sets-note">
                  {t(
                    "★ marks the highest win rate. ± is the margin: about how far luck alone can move a win rate. The true margin is wider, since one game counts every card in both decks, so sets less than two or three margins apart are too close to call. A card is strong or weak when it is {z} of its own margins or more over or under 50%.",
                    { z: SET_STATS_OUTLIER_Z },
                  )}
                </p>
              </>
            )}
          </section>
        )}

        {/* PLAYERS TAB */}
        {tab === "players" && (
          <section id="stats-panel-players" role="tabpanel" aria-labelledby="stats-tab-players" className="stats-panel one-screen__body">
            <div className="stats-player-head">
              <input
                type="search"
                className="stats-search-input"
                placeholder={t("Search player by username…")}
                value={playerSearch}
                onChange={(e) => setPlayerSearch(e.target.value)}
              />
              <p className="stats-player-note">
                {t("Player stats are public by default. Ratings and ranks are not shown here.")}
              </p>
            </div>

            {playersLoading ? (
              <Skeleton variant="table" label={t("Loading players…")} />
            ) : playersError ? (
              <div className="stats-error">
                <p>{t("The player stats didn't load.")}</p>
                <button type="button" className="stats-retry-btn" onClick={() => fetchPlayers(playerQuery, playerPage)}>
                  {t("Retry")}
                </button>
              </div>
            ) : playersList.length === 0 ? (
              <EmptyState text={t("No players found.")} />
            ) : (
              <TableScroll labeledBy="stats-tab-players">
                <table className="stats-table" data-testid={statsTestid.playersTable}>
                  <thead>
                    <tr>
                      <th scope="col" className="stats-th">{t("Player")}</th>
                      <th scope="col" className="stats-th">{t("Games")}</th>
                      <th scope="col" className="stats-th">{t("Win rate")}</th>
                      <th scope="col" className="stats-th">{t("Favorite card")}</th>
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
              </TableScroll>
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
          <div className="stats-modal-scrim" onClick={() => setCardDrillDownId(null)}>
            <DrillDownBackLayer onClose={() => setCardDrillDownId(null)} />
            <div
              ref={drillDownDialog}
              className="stats-drilldown-modal"
              data-testid={statsTestid.drillDownModal}
              data-radiant={drillDownRadiantFace ? "true" : undefined}
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
                  ref={drillDownCloseButton}
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
                {drillDownDef && drillDownFace && (
                  <div className="stats-drilldown-cards">
                    {/* R2756000: the faces print the art picked. FullArtShown decides only an own copy's face (R1834000), so they
                        are drawn as one; nothing else here reads OwnCopy (no inspect overlay, and CardFace's cosmetics follow
                        `worn`, left false). The page has no FullArtOwned above it, so every tier counts as owned, as in the Almanac
                        (R1313000). No pick is written. */}
                    <OwnCopy own>
                      <FullArtShown defId={fullArtSource(drillDownDef.id)} pick={artPick(drillDownDef.id, shownArt)}>
                        <div className="stats-drilldown-faces">
                          <figure className="stats-drilldown-face">
                            <div className="stats-drilldown-card-preview">
                              <CardFace face={drillDownFace} layout="full" />
                            </div>
                            {drillDownRadiantFace && <figcaption className="stats-drilldown-caption">{t("Normal")}</figcaption>}
                          </figure>
                          {drillDownRadiantFace && (
                            <figure className="stats-drilldown-face" data-testid={statsTestid.drillDownRadiantFace}>
                              <div className="stats-drilldown-card-preview">
                                <CardFace face={drillDownRadiantFace} layout="full" />
                              </div>
                              <figcaption className="stats-drilldown-caption">{t("Radiant")}</figcaption>
                            </figure>
                          )}
                        </div>
                      </FullArtShown>
                    </OwnCopy>
                    {drillDownArts.length > 0 && (
                      <div className="stats-drilldown-actions" role="group" aria-label={t("Full art")} data-testid={statsTestid.drillDownArt}>
                        {drillDownArts.map((art) => (
                          <button
                            key={art}
                            type="button"
                            className="stats-drilldown-action"
                            aria-pressed={shownArt === art}
                            onClick={() => setDrillDownArt(art)}
                          >
                            {t(ART_LABELS[art])}
                          </button>
                        ))}
                      </div>
                    )}
                    <div className="stats-drilldown-actions">
                      <button
                        type="button"
                        className="stats-drilldown-action"
                        data-testid={statsTestid.drillDownRadiantToggle}
                        aria-pressed={drillDownRadiant}
                        onClick={() => setDrillDownRadiant((shown) => !shown)}
                      >
                        {t("Show Radiant")}
                      </button>
                      <a
                        className="stats-drilldown-action button-secondary"
                        data-testid={statsTestid.drillDownAlmanac}
                        href={paths.almanacCard(drillDownDef.id)}
                        onClick={followInApp(paths.almanacCard(drillDownDef.id))}
                      >
                        {t("View in Almanac")}
                      </a>
                    </div>
                    {drillDownTokens.length > 0 && (
                      <section className="stats-drilldown-section stats-drilldown-tokens" data-testid={statsTestid.drillDownTokens}>
                        <h3>{t("Tokens")}</h3>
                        <ul className="stats-drilldown-token-list">
                          {drillDownTokens.map((token) => (
                            <li key={token.id} className="stats-drilldown-token">
                              <a
                                className="stats-drilldown-token-link"
                                href={paths.almanacCard(token.id)}
                                onClick={followInApp(paths.almanacCard(token.id))}
                                aria-label={t("View {name} in the Almanac", { name: readerName(token) })}
                              >
                                <span className="stats-drilldown-token-face">
                                  <CardFace face={faceModel({ defId: token.id, def: token, radiant: false })} layout="full" lazyArt />
                                </span>
                                <span className="stats-drilldown-token-name">{readerName(token)}</span>
                              </a>
                            </li>
                          ))}
                        </ul>
                      </section>
                    )}
                  </div>
                )}

                <div className="stats-drilldown-details">
                  {drillDownLoading ? (
                    <Skeleton variant="rows" label={t("Loading this card's details…")} />
                  ) : drillDownError ? (
                    <div className="stats-error">
                      <p>{t("Details unavailable.")}</p>
                      <button type="button" className="stats-retry-btn" onClick={() => setDrillDownAttempt((n) => n + 1)}>
                        {t("Retry")}
                      </button>
                    </div>
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
