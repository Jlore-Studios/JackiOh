// `/stats`: public card and player statistics, signed out and in (SPEC §9.11, R654), on three tabs: Cards
// (the filters, not columns, cover cost, rarity and keyword, R2446000), Sets (R1966000) and Players (public
// aggregates, fun stats from #125). No provisional banner and no reading notes (R2325000): the page never
// names a data source or the gate (R661). Below the per-row floor a row reads "not enough games". The Adjusted
// win rate leads (R2777000), so the table shows no When drawn, Drawn vs not drawn, Radiant or Normal (R3184000);
// with the ranges shown, a tier says how much of the card's range is in it (R3184001). The patch bar
// (R1973002): a patch is the window the cards share (R1973001), so a micro is never a step of its own; each
// offered patch is a tab, and the newest, short of games, says its data is still being collected (R3324000).
// The header names the current patch (R3324001). The Cards tab also holds the games panel (R1994000) and the
// cards made during a game, the Tokens and the Other created cards, two sortable sections that share no row
// (R2089000, R2370000, R3184002, R3324002); its on/off controls are the Segmented's pressed buttons (R3324003). The
// card drill-down shows the faces (a full-art card's still Epic while the Full-art cards setting is on,
// R3011000), patch history, turn curve and co-played synergy.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement, type ReactNode } from "react";
import catalogJson from "@jackioh/cards/catalog.json";
import { KEYWORD_KINDS, SHIPPED_SETS, type CardDef, type CardDefs, type KeywordKind } from "@jackioh/shared";
import Username from "../auth/Username.tsx";
import { CardDefsProvider } from "../cards/index.ts";
import { chineseName, drawnInChinese, readerName, readerTerms } from "../cards/chinese.ts";
import { CardFace } from "../cards/CardFace.tsx";
import { FullArtShown, OwnCopy, fullArtSource } from "../cards/fullArt.ts";
import { faceModel } from "../cards/model.ts";
import { useCardSettings } from "../cards/settings.ts";
import { EmptyState } from "../empty/EmptyState.tsx";
import { Dialog } from "../game/Dialog.tsx";
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
import type { CardTier } from "../wire/generated/CardTier.ts";
import { navigate, paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import {
  ADJUSTED_RANGE_DIGITS,
  CARD_STATS_MIN_SAMPLE,
  PLAYER_SEARCH_DEBOUNCE_MS,
  SET_STATS_DIGITS,
  SET_STATS_OUTLIER_Z,
  WIN_RATE_RANGE_DIGITS,
} from "../stats/config.ts";
import { CreatedPanel } from "../stats/CreatedPanel.tsx";
import { GamesPanel } from "../stats/GamesPanel.tsx";
import {
  currentVersion,
  isCollecting,
  neighbours,
  offeredPatches,
  patchIndex,
  patchName,
  patchTarget,
  type PatchWindow,
} from "../stats/patches.ts";
import { changes, leaders, outliers, setRows, type RatedCard } from "../stats/sets.ts";
import { TableScroll } from "../stats/TableScroll.tsx";
import { statsTestid } from "../stats/testids.ts";
import { TierChip, adjustedText } from "../stats/TierChip.tsx";
import { cardTierFit } from "../stats/tierFit.ts";
import { BackLink, followInApp } from "./nav.tsx";
import { Segmented, Tabs } from "../tabs/index.ts";

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
type SortColumn = "name" | "games" | "tier" | "adjustedWinRate" | "winRate" | "playRate";
type SortDirection = "asc" | "desc";

const SORT_COLUMNS: readonly SortColumn[] = ["name", "games", "tier", "adjustedWinRate", "winRate", "playRate"] as const;

const SORT_LABELS: Record<SortColumn, string> = {
  name: "Name",
  games: "Games",
  tier: "Tier",
  adjustedWinRate: "Adjusted",
  winRate: "Win rate",
  playRate: "Play rate",
};

function defaultDirection(col: SortColumn): SortDirection {
  return col === "name" ? "asc" : "desc";
}

/**
 * R2895000: a rate's 95% range under its figure, its ends to `digits` decimals of a percent (Adjusted's tenths,
 * Win rate's whole points); nothing when the server sent none.
 */
function RangeText({ low, high, digits, testid }: { low?: number | null; high?: number | null; digits: number; testid: string }): ReactElement | null {
  if (typeof low !== "number" || typeof high !== "number") return null;
  return (
    <span className="stats-sample-hint stats-range" data-testid={testid}>
      {`${(low * 100).toFixed(digits)}–${(high * 100).toFixed(digits)}%`}
    </span>
  );
}

/** R3022000: the tiers from the top, S (above the band, a nerf candidate) to F (below it, a buff candidate). */
const TIER_ORDER: readonly CardTier[] = ["S", "A", "B", "C", "D", "F"];

/** R3022000: a tier's place from the bottom, so a descending sort puts S first; -1 for none. */
function tierRank(tier: CardTier | null | undefined): number {
  return tier ? TIER_ORDER.length - TIER_ORDER.indexOf(tier) : -1;
}

/** R3022000: a band edge or a spread as the tiers' note reads it, a percent to a tenth at most ("45%", "52.5%"). */
function percentText(fraction: number): string {
  return `${String(Number((fraction * 100).toFixed(1)))}%`;
}

/** R3184001: under a tier, the share of the card's Adjusted range inside the tier's band, to a whole percent; nothing without one. */
function TierFitText({ fit }: { fit: number | null }): ReactElement | null {
  if (fit === null) return null;
  return (
    <span className="stats-sample-hint stats-range" data-testid={statsTestid.tierFit}>
      {`${Math.round(fit * 100)}%`}
    </span>
  );
}

/** R2777000: the table's default order, the adjusted win rate descending. */
const DEFAULT_SORT: SortColumn = "adjustedWinRate";

/**
 * R2446000: the sort an address asks for. One the table no longer offers (an old address's `cost`, `rarity` or
 * `keyword`, or R3184000's `drawnWinRate`, `drawnDelta`, `radiantWinRate` or `normalWinRate`) is the default,
 * the adjusted win rate descending (R2777000), its direction too.
 */
function sortOf(params: URLSearchParams): { col: SortColumn; dir: SortDirection } {
  const asked = params.get("sort");
  const col = asked === null ? DEFAULT_SORT : SORT_COLUMNS.find((known) => known === asked);
  if (col === undefined) return { col: DEFAULT_SORT, dir: "desc" };
  return { col, dir: params.get("dir") === "asc" ? "asc" : "desc" };
}

/**
 * R2777004: the match types the page reads. Every mode together comes first and is what the page opens on, then
 * Constructed (Best of 1 and Conquest together, R2777001), All random, and each constructed mode alone.
 */
const MODES = ["all", "constructed", "random", "bo1", "bo3"] as const;
type StatsMode = (typeof MODES)[number];
const DEFAULT_MODE: StatsMode = "all";
const MODE_LABELS: Record<StatsMode, string> = {
  all: "All modes",
  constructed: "Constructed",
  random: "All random",
  bo1: "Best of 1",
  bo3: "Conquest",
};

/** R2777004: the mode an address asks for; none, or one the page does not offer, is every mode. */
function modeOf(params: URLSearchParams): StatsMode {
  const asked = params.get("mode");
  return MODES.find((known) => known === asked) ?? DEFAULT_MODE;
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
 * R3324000: the Cards or Sets tab of a patch still collecting its data: the words in place of its figures, and the
 * way to the patch before. No count and no progress (R661).
 */
function CollectingNote({
  patch,
  previous,
  onPrevious,
}: {
  patch: string;
  previous: PatchWindow | null;
  onPrevious: (window: PatchWindow) => void;
}): ReactElement {
  return (
    <EmptyState
      text={t("Data for patch {patch} is still being collected at this time.", { patch })}
      testId={statsTestid.patchCollecting}
      action={
        previous === null
          ? undefined
          : {
              label: t("See patch {patch}", { patch: patchName(previous) }),
              testId: statsTestid.patchCollectingBack,
              onPress: () => onPrevious(previous),
            }
      }
    />
  );
}

export default function StatsRoute(): ReactElement {
  const locale = useLocale();
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
  // R2895000: the 95% ranges under Win rate and Adjusted, off by default.
  const [showRanges, setShowRanges] = useState(initialParams.get("ranges") === "1");
  const [mode, setMode] = useState<StatsMode>(() => modeOf(initialParams));

  const [visibleCols, setVisibleCols] = useState({
    thumbnail: true,
    games: true,
    winRate: true,
    playRate: true,
  });

  // R2125000: a revisit starts on the table these filters last read and reads it again behind it.
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

  const [playersLoading, setPlayersLoading] = useState(false);
  const [playersError, setPlayersError] = useState(false);
  const [playersList, setPlayersList] = useState<PublicPlayerSummary[]>([]);
  const [playerSearch, setPlayerSearch] = useState("");
  // The search last asked for (the box's text, a pause after its last change) and its page.
  const [playerQuery, setPlayerQuery] = useState("");
  const [playerPage, setPlayerPage] = useState(1);
  const [playersHasMore, setPlayersHasMore] = useState(false);

  const [drillDownLoading, setDrillDownLoading] = useState(false);
  const [drillDownData, setDrillDownData] = useState<CardDrillDownResponse | null>(null);
  const [drillDownError, setDrillDownError] = useState(false);
  // Bumped by the details' Retry, as `cardStatsAttempt` is by the table's.
  const [drillDownAttempt, setDrillDownAttempt] = useState(0);
  // The drill-down's "Show Radiant": the Radiant face beside the normal one, kept from card to card.
  const [drillDownRadiant, setDrillDownRadiant] = useState(false);
  const { fullArt: fullArtSetting } = useCardSettings();

  // R1966000: every set's cards, unfiltered, and the previous patch's for the change.
  const [setsData, setSetsData] = useState<PublicStatsCardsResponse | null>(null);
  const [setsPrevious, setSetsPrevious] = useState<PublicStatsCardsResponse | null>(null);
  const [setsError, setSetsError] = useState(false);
  // Bumped by the Sets tab's Retry, as `cardStatsAttempt` is by the table's.
  const [setsAttempt, setSetsAttempt] = useState(0);

  const updateUrl = useCallback(() => {
    const params = new URLSearchParams();
    if (tab !== "cards") params.set("tab", tab);
    if (patch) params.set("patch", patch);
    if (search) params.set("search", search);
    if (sortCol !== DEFAULT_SORT) params.set("sort", sortCol);
    if (sortDir !== "desc") params.set("dir", sortDir);
    if (setFilter) params.set("set", setFilter);
    if (rarityFilter) params.set("rarity", rarityFilter);
    if (costFilter) params.set("cost", costFilter);
    if (keywordFilter) params.set("keyword", keywordFilter);
    if (view !== "table") params.set("view", view);
    if (cardDrillDownId) params.set("card", cardDrillDownId);
    if (mode !== DEFAULT_MODE) params.set("mode", mode);
    if (showRanges) params.set("ranges", "1");

    const query = params.toString();
    const target = query ? `${paths.stats}?${query}` : paths.stats;
    navigate(target, { replace: true });
  }, [tab, patch, search, sortCol, sortDir, setFilter, rarityFilter, costFilter, keywordFilter, view, cardDrillDownId, mode, showRanges]);

  useEffect(() => {
    updateUrl();
  }, [updateUrl]);

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

  // The response carries no total, so a full page of the server's `limit` rows may have another after it.
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

  // One request per pause in the typing, not one per keystroke.
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

  // R1973002: the newest patch is asked for with no `?patch=`, so the address stays the plain one.
  const patchList = cardStatsData?.patches ?? [];
  const shownPatch = cardStatsData ? patchIndex(patchList, cardStatsData.patch) : -1;
  const pickable = offeredPatches(patchList, shownPatch);
  const { previous: previousWindow, next: nextWindow } = neighbours(patchList, shownPatch);
  const choosePatch = (window: PatchWindow): void => setPatch(patchList.indexOf(window) === 0 ? "" : patchTarget(window));
  // R3324000: the newest patch, short of games, shows no figures: its Cards and Sets tabs say it is collecting.
  const collecting = isCollecting(patchList, shownPatch);
  const shownPatchName = shownPatch >= 0 ? patchName(patchList[shownPatch]!) : (cardStatsData?.patch ?? "");

  // R1966000: the Sets tab reads the patch's every card, whatever the Cards tab filters, then the previous
  // patch's for each set's change; a previous patch that fails to load only leaves the change out.
  useEffect(() => {
    // R3324000: a patch still collecting its data has no sets to read.
    if (tab !== "sets" || collecting) return;
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
  }, [tab, patch, mode, setsAttempt, collecting]);

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

  // The drill-down is a modal like the client's others (docs/polish/6-cards.md, B25), drawn through
  // Dialog.tsx (R3221000), whose keys close it and keep Tab inside, and Back too (R2127000). The control
  // that opened it is kept when it is pressed, not read off the focus, which a click on a button does
  // not move in every browser, and has the focus back once it closes.
  const drillDownOpener = useRef<HTMLElement | null>(null);
  const drillDownOpen = Boolean(cardDrillDownId);

  const openDrillDown = (id: string, opener: HTMLElement): void => {
    drillDownOpener.current = opener;
    setCardDrillDownId(id);
  };

  useEffect(() => {
    if (drillDownOpen) return;
    const opener = drillDownOpener.current;
    if (opener !== null && opener.isConnected) opener.focus({ preventScroll: true });
  }, [drillDownOpen]);

  const minSample = cardStatsData?.minSample ?? CARD_STATS_MIN_SAMPLE;
  // R3022000: the field and band the tiers are read against; absent from a server deployed before them.
  const tiers = cardStatsData?.tiers;

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

  const handleSort = (col: SortColumn) => {
    if (sortCol === col) {
      setSortDir((prev) => (prev === "asc" ? "desc" : "asc"));
    } else {
      setSortCol(col);
      setSortDir(defaultDirection(col));
    }
  };

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
      if (sortCol === "tier") {
        // R3022000: by tier, S first descending; a card without one after those with one, then by its adjusted rate.
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        const aRank = tierRank(a.tier);
        const bRank = tierRank(b.tier);
        if (aRank >= 0 && bRank < 0) return -1;
        if (aRank < 0 && bRank >= 0) return 1;
        if (aRank !== bRank) return (aRank - bRank) * flip;
        // R3184001: within a tier, by how much of the card's range is in it, a card with no range after; then by its adjusted rate.
        const aFit = cardTierFit(a, tiers);
        const bFit = cardTierFit(b, tiers);
        if (aFit !== null && bFit === null) return -1;
        if (aFit === null && bFit !== null) return 1;
        if (aFit !== null && bFit !== null && aFit !== bFit) return (aFit - bFit) * flip;
        const aRate = a.adjustedWinRate ?? null;
        const bRate = b.adjustedWinRate ?? null;
        if (aRate !== null && bRate !== null && aRate !== bRate) return (aRate - bRate) * flip;
      } else if (sortCol === "adjustedWinRate") {
        // R2777000: a card with no adjusted rate (nothing seen, or never unseen) sorts after those with one.
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        const aRate = a.adjustedWinRate ?? null;
        const bRate = b.adjustedWinRate ?? null;
        if (aRate !== null && bRate === null) return -1;
        if (aRate === null && bRate !== null) return 1;
        if (aRate !== null && bRate !== null && aRate !== bRate) {
          return (aRate - bRate) * flip;
        }
      } else if (sortCol === "winRate") {
        if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
        if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
        if (a.winRate !== null && b.winRate !== null && a.winRate !== b.winRate) {
          return (a.winRate - b.winRate) * flip;
        }
      } else if (sortCol === "games") {
        if (a.games !== b.games) return (a.games - b.games) * flip;
      } else if (sortCol === "playRate") {
        if (a.playRate !== b.playRate) return (a.playRate - b.playRate) * flip;
      } else if (sortCol === "name") {
        const diff = compareNames(a, b);
        if (diff !== 0) return diff * flip;
      }
      return a.games - b.games || compareNames(a, b);
    });

    return list;
  }, [cardStatsData, search, keywordFilter, sortCol, sortDir, locale, tiers]);

  // R2089000: the Created cards the search box asks for, as it asks for the deck rows; R3324002: the tokens
  // among them in a section of their own, and every other in its own, so no card is in both.
  const createdCards = (cardStatsData?.created ?? []).filter((card) => matchesSearch(card, search));
  const tokenCards = createdCards.filter((card) => CATALOG[card.id]?.token === true);
  const otherCreatedCards = createdCards.filter((card) => CATALOG[card.id]?.token !== true);

  const summary = cardStatsData?.summary;


  const drillDownDef = cardDrillDownId ? CATALOG[cardDrillDownId] : undefined;
  const drillDownFace = drillDownDef ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: false }) : null;
  const drillDownRadiantFace =
    drillDownDef && drillDownRadiant ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: true }) : null;
  const drillDownTokens = drillDownDef ? tokensOf(drillDownDef) : [];

  return (
    <CardDefsProvider defs={CATALOG}>
      <div className="app-shell app-shell--wide tavern stats-page one-screen" data-testid={statsTestid.screen}>
        <header>
          <BackLink title={t("Statistics")}>
            {/* R3324001: the patch the game is on now, whichever patch the page shows */}
            <span className="stats-current-patch" data-testid={statsTestid.currentPatch}>
              {t("Current patch {patch}", { patch: currentVersion(patchList) })}
            </span>
          </BackLink>
        </header>

        {/* R1973002, R3324000: a tab for each offered patch, between the patch before and the patch after */}
        {patchList.length > 0 && (
          <nav className="stats-patch-bar" data-testid={statsTestid.patchBar} aria-label={t("Patches")}>
            <button
              type="button"
              className="stats-patch-step"
              data-testid={statsTestid.patchPrevious}
              disabled={previousWindow === null}
              aria-label={previousWindow ? t("Previous patch ({patch})", { patch: patchName(previousWindow) }) : t("Previous patch")}
              title={previousWindow ? t("Previous patch ({patch})", { patch: patchName(previousWindow) }) : t("Previous patch")}
              onClick={() => previousWindow && choosePatch(previousWindow)}
            >
              ←
            </button>
            <Segmented
              label={t("Patch")}
              className="stats-patch-tabs"
              value={shownPatch >= 0 ? patchTarget(patchList[shownPatch]!) : ""}
              onChange={(target) => {
                const chosen = patchList.find((window) => patchTarget(window) === target);
                if (chosen) choosePatch(chosen);
              }}
              options={pickable.map((window) => ({
                value: patchTarget(window),
                testId: statsTestid.patchTab,
                className: "stats-patch-tab",
                label: (
                  <>
                    <span className="stats-patch-tab-name">{patchName(window)}</span>
                    <span className="stats-patch-tab-games">
                      {isCollecting(patchList, patchList.indexOf(window))
                        ? t("Collecting data")
                        : t("{games} games", { games: formatNumber(window.games) })}
                    </span>
                  </>
                ),
              }))}
            />
            <button
              type="button"
              className="stats-patch-step"
              data-testid={statsTestid.patchNext}
              disabled={nextWindow === null}
              aria-label={nextWindow ? t("Next patch ({patch})", { patch: patchName(nextWindow) }) : t("Next patch")}
              title={nextWindow ? t("Next patch ({patch})", { patch: patchName(nextWindow) }) : t("Next patch")}
              onClick={() => nextWindow && choosePatch(nextWindow)}
            >
              →
            </button>
          </nav>
        )}

        {/* The summary: the patch's games alone (R661); R1646000 took the top and lowest card tiles off the fixed top;
            a patch still collecting its data shows no count (R3324000) */}
        {cardStatsData && !collecting && (
          <section className="stats-summary-tiles" data-testid={statsTestid.summaryTiles} aria-label={t("Statistics summary")}>
            <div className="stats-summary-tile" data-testid={statsTestid.summaryTotalGames}>
              <span className="stats-summary-label">{t("Games on patch {patch}", { patch: shownPatchName })}</span>
              <span className="stats-summary-value">{summary ? formatNumber(summary.totalGames) : ""}</span>
            </div>
          </section>
        )}

        {/* Tabs */}
        <Tabs
          label={t("Statistics views")}
          idPrefix="stats"
          value={tab}
          onChange={setTab}
          className="one-screen__frame"
          tablistClassName="stats-tabs"
          tabClassName="stats-tab"
          panelAs="section"
          items={[
            {
              id: "cards",
              label: t("Cards"),
              testId: statsTestid.tabCards,
              panelClassName: "stats-panel one-screen__body",
              // R3324000: a patch still collecting its data shows no figures.
              panel: collecting ? (
                <CollectingNote patch={shownPatchName} previous={previousWindow} onPrevious={choosePatch} />
              ) : (
                <>
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
                  onChange={(e) => setMode(modeOf(new URLSearchParams({ mode: e.target.value })))}
                  aria-label={t("Filter by match type")}
                >
                  {MODES.map((known) => (
                    <option key={known} value={known}>
                      {t(MODE_LABELS[known])}
                    </option>
                  ))}
                </select>
              </div>

              {/* R3324003: how the cards show, one row of labelled groups, every on/off control a pressed button */}
              <div className="stats-options">
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
                  className="stats-sort-dir"
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

              <div className="stats-view-toggle">
                <span className="stats-sort-label" aria-hidden="true">
                  {t("View")}
                </span>
                <Segmented
                  label={t("View")}
                  value={view}
                  onChange={setView}
                  options={[
                    { value: "table", label: t("Table"), testId: statsTestid.viewTable, className: "stats-view-btn" },
                    { value: "grid", label: t("Grid"), testId: statsTestid.viewGrid, className: "stats-view-btn" },
                  ]}
                />
              </div>

              {/* R2895000: the 95% ranges, off by default; R3324003: an on/off button in the Segmented's look */}
              <div className="segmented">
                <button
                  type="button"
                  className={`segmented__item stats-view-btn ${showRanges ? "segmented__item--selected" : ""}`}
                  data-testid={statsTestid.rangesToggle}
                  aria-pressed={showRanges}
                  onClick={() => setShowRanges((shown) => !shown)}
                >
                  {t("Show 95% ranges")}
                </button>
              </div>

              {/* Column visibility toggles: each on or off on its own, so buttons in the Segmented's look, not one of a few */}
              {view === "table" && (
                <div className="stats-view-toggle" role="group" aria-labelledby="stats-columns-label">
                  <span className="stats-sort-label" id="stats-columns-label">
                    {t("Columns")}
                  </span>
                  <div className="segmented">
                    <button
                      type="button"
                      className={`segmented__item stats-view-btn ${visibleCols.games ? "segmented__item--selected" : ""}`}
                      data-testid={statsTestid.columnGames}
                      aria-pressed={visibleCols.games}
                      onClick={() => setVisibleCols((c) => ({ ...c, games: !c.games }))}
                    >
                      {t("Games")}
                    </button>
                    <button
                      type="button"
                      className={`segmented__item stats-view-btn ${visibleCols.playRate ? "segmented__item--selected" : ""}`}
                      data-testid={statsTestid.columnPlayRate}
                      aria-pressed={visibleCols.playRate}
                      onClick={() => setVisibleCols((c) => ({ ...c, playRate: !c.playRate }))}
                    >
                      {t("Play rate")}
                    </button>
                  </div>
                </div>
              )}
              </div>
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
                        <dt>{t("Tier")}</dt>
                        <dd>
                          <TierChip tier={card.hasEnoughGames ? card.tier : null} testid={statsTestid.tierCell} />
                          {showRanges && <TierFitText fit={cardTierFit(card, tiers)} />}
                        </dd>
                        <dt>{t("Adjusted")}</dt>
                        <dd>
                          {card.hasEnoughGames ? adjustedText(card.adjustedWinRate) : "—"}
                          {showRanges && card.hasEnoughGames && (
                            <RangeText low={card.adjustedLow} high={card.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                          )}
                        </dd>
                        <dt>{t("Win rate")}</dt>
                        <dd>
                          {card.hasEnoughGames && card.winRate !== null
                            ? `${Math.round(card.winRate * 100)}%`
                            : t("not enough games")}
                          {showRanges && card.hasEnoughGames && card.winRate !== null && (
                            <RangeText low={card.winRateLow} high={card.winRateHigh} digits={WIN_RATE_RANGE_DIGITS} testid={statsTestid.winRateRange} />
                          )}
                        </dd>
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
                        title={t("S needs a nerf and F a buff: S is above the band and sure of it, F below it and sure of it, and A to D are the band's quarters")}
                        aria-sort={sortCol === "tier" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("tier")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("tier");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Tier")} {sortCol === "tier" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        title={t("50% plus how far drawing the card moves its deck's win rate beyond the average card's, counted only as far as its games bear out")}
                        aria-sort={sortCol === "adjustedWinRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("adjustedWinRate")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("adjustedWinRate");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Adjusted")} {sortCol === "adjustedWinRate" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
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
                          <td className="stats-td">
                            <TierChip tier={card.hasEnoughGames ? card.tier : null} testid={statsTestid.tierCell} />
                            {showRanges && <TierFitText fit={cardTierFit(card, tiers)} />}
                          </td>
                          <td className="stats-td" data-testid={statsTestid.adjustedCell}>
                            {card.hasEnoughGames ? adjustedText(card.adjustedWinRate) : "—"}
                            {showRanges && card.hasEnoughGames && (
                              <RangeText low={card.adjustedLow} high={card.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                            )}
                          </td>
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
                                {showRanges && (
                                  <RangeText low={card.winRateLow} high={card.winRateHigh} digits={WIN_RATE_RANGE_DIGITS} testid={statsTestid.winRateRange} />
                                )}
                              </div>
                            ) : (
                              <span className="stats-not-enough">{t("not enough games")}</span>
                            )}
                          </td>
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

            {/* R3022000: what the tiers mean, and the field the cards are read against */}
            {tiers && !cardStatsLoading && !cardStatsError && filteredCards.length > 0 && (
              <div className="stats-sample-hint stats-tiers-note" data-testid={statsTestid.tiersNote}>
                <p>
                  {t("Tier S is above {high} with its whole 95% range above 50%: a nerf candidate. Tier F is below {low} with its whole range below 50%: a buff candidate. A, B, C and D split the {low} to {high} band into quarters from the top.", {
                    high: percentText(tiers.bandHigh),
                    low: percentText(tiers.bandLow),
                  })}
                </p>
                <p>
                  {tiers.cards > 0
                    ? t("Read against {cards} cards: the average card reads 50%, and real cards differ by about {spread} points (one standard deviation).", {
                        cards: formatNumber(tiers.cards),
                        spread: (tiers.spread * 100).toFixed(1),
                      })
                    : t("Too few cards have enough games to measure the rest against: the average card is taken to read 50%, and real cards to differ by about {spread} points.", {
                        spread: (tiers.spread * 100).toFixed(1),
                      })}
                </p>
                {typeof tiers.baseline === "number" && (
                  <p>
                    {t("A card that is almost never left undrawn, such as a Quickdraw card, and the Dormant Starship are read against the average deck, which wins {baseline} of its games.", {
                      baseline: percentText(tiers.baseline),
                    })}
                  </p>
                )}
              </div>
            )}

            {/* R2895000: what a range means, while they show */}
            {showRanges && !cardStatsLoading && !cardStatsError && filteredCards.length > 0 && (
              <p className="stats-sample-hint" data-testid={statsTestid.rangesNote}>
                {t("A range is where the card's true rate most likely sits, 95% sure. Two cards whose ranges overlap may be no different.")}
                {tiers && ` ${t("The percentage under a tier is how much of the card's Adjusted range lies inside that tier.")}`}
              </p>
            )}

            {/* R2089000, R3324002: the cards made during a game, apart from the deck cards above: the tokens, then
                every other, no card in both */}
            {!cardStatsLoading && !cardStatsError && (
              <>
                <CreatedPanel kind="tokens" created={tokenCards} minSample={minSample} nameOf={shownName} />
                <CreatedPanel kind="other" created={otherCreatedCards} minSample={minSample} nameOf={shownName} />
              </>
            )}
          </>
        ),
      },
      {
        id: "sets",
        label: t("Sets"),
        testId: statsTestid.tabSets,
        panelClassName: "stats-panel one-screen__body",
        // R3324000: a patch still collecting its data has no sets to show.
        panel: collecting ? (
          <CollectingNote patch={shownPatchName} previous={previousWindow} onPrevious={choosePatch} />
        ) : (
          <>
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
          </>
        ),
      },
      {
        id: "players",
        label: t("Players"),
        testId: statsTestid.tabPlayers,
        panelClassName: "stats-panel one-screen__body",
        panel: (
          <>
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
          </>
        ),
      },
    ]}
  />

        {/* Card Drill-Down Modal */}
        {cardDrillDownId && (
            <Dialog
              role="dialog"
              testid={statsTestid.drillDownModal}
              className="stats-drilldown-modal"
              data={drillDownRadiantFace ? { "data-radiant": "true" } : undefined}
              closeTestid={statsTestid.drillDownClose}
              closeLabel={t("Close card details")}
              title={drillDownDef ? readerName(drillDownDef) : cardDrillDownId}
              onClose={() => setCardDrillDownId(null)}
              actions={[]}
            >
              <div className="stats-drilldown-body">
                {drillDownDef && drillDownFace && (
                  <div className="stats-drilldown-cards">
                    {/* R3011000: the faces print the setting's still Epic while the Full-art cards setting is on, the
                        normal face while it is off. FullArtShown decides only an own copy's face (R1834000), so they
                        are drawn as one; nothing else here reads OwnCopy (no inspect overlay, and CardFace's cosmetics follow
                        `worn`, left false). The page has no FullArtOwned above it, so every tier counts as owned, as in the Almanac
                        (R1313000). Passing the setting down as the pick, rather than leaving the pick out, keeps the
                        device's stored pick out of this public preview. No pick is written. */}
                    <OwnCopy own>
                      <FullArtShown defId={fullArtSource(drillDownDef.id)} pick={fullArtSetting}>
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
                        <h3>{t("Tokens this card makes")}</h3>
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
            </Dialog>
        )}
      </div>
    </CardDefsProvider>
  );
}
