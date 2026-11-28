// `/stats`: public card and player statistics, signed out and in (SPEC §9.11, R654), on five tabs, each a raised
// button showing its name alone (R4279001, R4335000, R4678000): Cards (the filters, not columns, cover cost, rarity
// and keyword, R2446000), Tokens (R4279001), Sets (R1966000), Players (public aggregates, fun stats from #125) and
// Experimental (R4678000: figures that span patches; its Running view, R4678001, is each card's live play version
// over every patch since it last changed, under the Cards tab's filters, sort, view and ranges, with no patch bar,
// games tile or Face, a line naming what it shows in their place, and a drill-down on All and the newest patch; R4680000:
// its Segmented's Versions view is every play version of those cards as its own row, Past versions only one button, and
// R4680001 a version's row opens its own drill-down, `?card=<id>&version=<first>`, drawn from its own catalog;
// R5040000: Running is now Historical, the second view, and the first, Major patch running, is the same over the
// newest major's line alone, which the server cuts, `GET /api/stats/versions?scope=major`). No
// provisional banner and no reading notes (R2325000): the page never names a data source or the gate (R661).
// Below the per-row floor a row reads "not enough games". The Adjusted win rate leads (R2777000), so the table
// shows no When drawn, Drawn vs not drawn, Radiant or Normal (R3184000); with the ranges shown, a tier says how
// much of the card's range is in it (R3184001). The patch bar (R1973002): a patch is the window the cards share
// (R1973001), so a micro is never a step of its own; each offered patch is a tab, and the newest, short of games,
// says its data is still being collected (R3324000). The header names the current patch (R3324001), and the games
// tile the patch its games were collected on (R3447000). The Cards tab also holds the games panel (R1994000). The
// Tokens tab holds the cards made during a game, the Tokens and the Other created cards, two sections that share
// no row and show the deck table's columns, opening a row's drill-down as a deck row does, under the Cards tab's
// filters (R2089000, R2370000, R3184002, R3324002, R4155000, R4279001); R4083000: Tier and Adjusted alone, sorted,
// viewed and ranged by the Cards tab's own controls, the keyword filter among them. The page's on/off controls are
// the Segmented's pressed buttons (R3324003). The card drill-down shows the faces (a full-art card's still Epic while
// the Full-art cards setting is on, R3011000), patch history, turn curve and co-played synergy. R3838000: its
// "Show card text" button, the Almanac's, prints every face's words, no full art (cardText.tsx). R3887000: the
// Cards and Tokens tabs' "Plain text" button prints each card they list, and the drill-down's text and its
// Reference cards, as readable text, no face (PlainText.tsx). R4673000, R4673001: their Face control (All, Normal
// or Radiant, kept in the address) reads every figure they and the Sets tab show on one face, against that face's
// own field and band, and the drill-down on the page's face and patch, with each face's rate and the Radiant gap.
// R4676000, R4676001: the drill-down's Extended stats toggle swaps its details for one panel, kept in the address as
// `extended`: a Segmented of the sections the card has, Choices (each choice's share, tier and Adjusted) and Radiant
// vs normal (each face's figures and the gap). R4679000, R4679001: and between them Outputs, each card the card made
// or offered with its share, tier and Adjusted against its siblings and a pick's offers and pick rate, a row opening
// that card's drill-down; while the panel is open, "Tokens this card makes" gives way as the details do.

import { memo, useCallback, useEffect, useId, useMemo, useRef, useState, type ReactElement, type ReactNode } from "react";
import { KEYWORD_KINDS, SHIPPED_SETS, fillParams, type CardDef, type CardDefs, type KeywordKind } from "@jackioh/shared";
import Username from "../auth/Username.tsx";
import { CardDefsProvider } from "../cards/index.ts";
import { chineseName, drawnInChinese, readerName, readerTerms } from "../cards/chinese.ts";
import { CardFace } from "../cards/CardFace.tsx";
import { FullArtShown, OwnCopy, fullArtSource } from "../cards/fullArt.ts";
import { faceModel } from "../cards/model.ts";
import { RefsInteractive } from "../cards/refContext.tsx";
import { useCardSettings } from "../cards/settings.ts";
import { EmptyState } from "../empty/EmptyState.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { currentCatalog } from "../game/currentCatalog.ts";
import { useFaceModel } from "../game/faces.ts";
import { modeText } from "../game/modeText.ts";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import { Icon } from "../icons/Icon.tsx";
import {
  ApiRequestError,
  cardStatsPath,
  getCardDrillDown,
  getCardStats,
  getHallOfFame,
  getPublicPlayers,
  getStatsVersions,
  type CardDrillDownResponse,
  type CardFaces,
  type PublicOutputRow,
  type PublicRolledGroup,
  type PublicVariantGroup,
  type PublicVariantRow,
  type CardStatsOptions,
  type PublicFaceFigures,
  type PublicHallOfFameResponse,
  type PublicPlayVersion,
  type PublicPlayerSummary,
  type PublicStatsCardsResponse,
  type PublicStatsVersionsResponse,
} from "../net/api.ts";
import { navigate, paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { readSession } from "../net/session.ts";
import { PatchFace } from "../patches/PatchFace.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import {
  ADJUSTED_RANGE_DIGITS,
  CARD_STATS_MIN_SAMPLE,
  PLAYER_SEARCH_DEBOUNCE_MS,
  ROLLED_SHARE_DIGITS,
  SET_STATS_DIGITS,
  SET_STATS_OUTLIER_Z,
  WIN_RATE_RANGE_DIGITS,
} from "../stats/config.ts";
import { CREATED_SORT_COLUMNS, CreatedPanel, createdSortOf, type CreatedSort } from "../stats/CreatedPanel.tsx";
import { GamesPanel } from "../stats/GamesPanel.tsx";
import { HallPanel } from "../stats/HallPanel.tsx";
import { FetchLive } from "../stats/FetchLive.tsx";
import { askAgain, StatsLoadProgress } from "../stats/LoadProgress.tsx";
import { FindPatchIcon, PatchSearch, olderPatchesLine } from "../stats/PatchSearch.tsx";
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
import { PlainCard, PlainTextButton, usePlainText } from "../stats/PlainText.tsx";
import { RangeText, TierFitText } from "../stats/RangeText.tsx";
import { RUNNING_SORT_COLUMNS, RunningPanel, runningRows, runningSortOf, versionRows } from "../stats/RunningPanel.tsx";
import { changes, leaders, outliers, setRows, type RatedCard } from "../stats/sets.ts";
import { TableScroll } from "../stats/TableScroll.tsx";
import { statsTestid } from "../stats/testids.ts";
import { TierChip, adjustedText, decisivenessText, tierRank } from "../stats/TierChip.tsx";
import { cardTierFit } from "../stats/tierFit.ts";
import { useVersionSnapshots } from "../stats/versionDefs.ts";
import { VersionDrillDown, type VersionRead } from "../stats/VersionDrillDown.tsx";
import type { StatsFace } from "../wire/generated/StatsFace.ts";
import { CardTextButton, CardTextFaces, cardTextTestid, useCardText, type CardText } from "./cardText.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import { Segmented, Tabs } from "../tabs/index.ts";

import "../auth/tavern.css";
import "./stats.css";

/** Unit conversion, not configuration: a rate as a percentage has two more decimal places than as a fraction. */
const PERCENT_PLACES = 2;

/**
 * #4180: the module's catalog (`game/currentCatalog.ts`), read in a call, never at import: the page
 * sits behind `EngineGate` (`main.tsx`), so the module is in before any of these run.
 */
function catalog(): CardDefs {
  return currentCatalog().cards;
}

/** A card's name as the reader reads it: the catalog's Chinese for a reader of Chinese, else the server's own. */
function shownName(id: string, name: string): string {
  return drawnInChinese(false) ? chineseName(id, name, catalog()[id]) : name;
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
  const def = catalog()[id];
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

/** R279, R3887000: every card a card's text names, token or not, the card itself left out, in the text's order. */
function referencesOf(def: CardDef): CardDef[] {
  return [...new Set(def.refs ?? [])].flatMap((id) => {
    const named = catalog()[id];
    return named !== undefined && id !== def.id ? [named] : [];
  });
}

/** The tokens a card's text names (R279), the card itself left out: what the drill-down shows it makes. */
function tokensOf(def: CardDef): CardDef[] {
  return referencesOf(def).filter((named) => named.token);
}

/** R3887000: a card as plain text, or null for an id the catalog holds no card of. */
function plainCardOf(id: string): ReactElement | null {
  const def = catalog()[id];
  return def === undefined ? null : <PlainCard def={def} />;
}

/**
 * #4511: one grid card's face, built once per card (the deck builder's `PoolCard`, #2266): the memo
 * holds it across the search box's keystrokes, and the hook rebuilds it on a language switch
 * (R711000). Null for an id the catalog holds no card of.
 */
const StatsGridFace = memo(function StatsGridFace({ defId }: { defId: string }): ReactElement | null {
  const def = catalog()[defId];
  if (def === undefined) return null;
  return <StatsGridFaceInner defId={defId} def={def} />;
});

function StatsGridFaceInner({ defId, def }: { defId: string; def: CardDef }): ReactElement {
  const face = useFaceModel({ defId, def, radiant: false });
  return (
    <div className="stats-grid-face">
      <CardFace face={face} layout="full" lazyArt />
    </div>
  );
}

/** R4680000: an old play version's face in the Grid, as its own catalog printed it, built once per definition; R279: an old printing's references are not interactive. */
const StatsVersionGridFace = memo(function StatsVersionGridFace({ def }: { def: CardDef }): ReactElement {
  return (
    <RefsInteractive enabled={false}>
      <StatsGridFaceInner defId={def.id} def={def} />
    </RefsInteractive>
  );
});

/** #4511: a drill-down token's face, built once per token, as the grid card's face is. */
const StatsDrillDownTokenFace = memo(function StatsDrillDownTokenFace({ def }: { def: CardDef }): ReactElement {
  const face = useFaceModel({ defId: def.id, def, radiant: false });
  return (
    <span className="stats-drilldown-token-face">
      <CardFace face={face} layout="full" lazyArt />
    </span>
  );
});

/** R4083000: a Created card's face in the Grid, as the deck grid draws a deck card's; null for an id the catalog holds no card of. */
function gridFaceOf(id: string): ReactElement | null {
  return <StatsGridFace defId={id} />;
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

/** R4677000: the code the server refuses a patch past the caller's with, an older patch for dev accounts alone. */
const PATCH_DEV_ONLY = "patch_dev_only";

/** R4677000: the signed-in token the statistics reads carry, so the server can tell a dev account; none signed out. */
function statsToken(): string | null {
  return readSession()?.accessToken ?? null;
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

type TabKey = "cards" | "tokens" | "sets" | "players" | "experimental";
const TAB_KEYS: readonly TabKey[] = ["cards", "tokens", "sets", "players", "experimental"];
type ViewKey = "table" | "grid";
/**
 * R4678000, R4680000, R4681001, R5040000: the Experimental tab's views, in the Segmented's order; `?experimental=`
 * names any but the first. `running` is Major patch running, so an old link to Running opens it; `historical` is
 * R4678001's view, every patch since a card last changed.
 */
type ExperimentalView = "running" | "historical" | "versions" | "hall";
const EXPERIMENTAL_VIEWS: readonly ExperimentalView[] = ["running", "historical", "versions", "hall"];
type SortColumn = "name" | "games" | "tier" | "adjustedWinRate" | "decisiveness" | "winRate" | "playRate" | "date";
type SortDirection = "asc" | "desc";

const SORT_COLUMNS: readonly SortColumn[] = ["name", "games", "tier", "adjustedWinRate", "decisiveness", "winRate", "playRate"] as const;

/**
 * R5033000: the columns an address may sort by: the Cards tab's and Date, which only Running and Versions offer.
 * The Cards and Tokens tabs read Date as Adjusted, so `?sort=date` survives a switch of tab (R4083000).
 */
const ADDRESS_SORT_COLUMNS: readonly SortColumn[] = [...SORT_COLUMNS, "date"];

const SORT_LABELS: Record<SortColumn, string> = {
  name: "Name",
  games: "Games",
  tier: "Tier",
  adjustedWinRate: "Adjusted",
  decisiveness: "Decisiveness",
  winRate: "Win rate",
  playRate: "Play rate",
  date: "Date",
};

function defaultDirection(col: SortColumn): SortDirection {
  return col === "name" ? "asc" : "desc";
}

/** R2777000: the table's default order, the adjusted win rate descending. */
const DEFAULT_SORT = "adjustedWinRate" satisfies SortColumn;

/**
 * R2446000: the sort an address asks for. One the table no longer offers (an old address's `cost`, `rarity` or
 * `keyword`, or R3184000's `drawnWinRate`, `drawnDelta`, `radiantWinRate` or `normalWinRate`) is the default,
 * the adjusted win rate descending (R2777000), its direction too.
 */
function sortOf(params: URLSearchParams): { col: SortColumn; dir: SortDirection } {
  const asked = params.get("sort");
  const col = asked === null ? DEFAULT_SORT : ADDRESS_SORT_COLUMNS.find((known) => known === asked);
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

/** R4673001: the faces the Cards and Tokens tabs read. All, every seat as before, comes first and is what the page opens on. */
const FACES: readonly StatsFace[] = ["all", "normal", "radiant"];
const DEFAULT_FACE: StatsFace = "all";

/** R4673001: the face an address asks for; none, or one the page does not offer, is All. */
function faceOf(params: URLSearchParams): StatsFace {
  const asked = params.get("face");
  return FACES.find((known) => known === asked) ?? DEFAULT_FACE;
}

/** R4673001: the face a read asks for: none on All, so All's requests and their kept answers stay as they were. */
function askedFace(face: StatsFace): StatsFace | undefined {
  return face === DEFAULT_FACE ? undefined : face;
}

/** The Cards tab's read for these filters; an empty filter asks for nothing, as `fetchCardStats` always did. */
function cardStatsOptionsOf(
  patch: string,
  set: string,
  rarity: string,
  cost: string,
  mode: string,
  face: StatsFace,
): CardStatsOptions {
  return {
    patch: patch ? patch : undefined,
    set: set ? set : undefined,
    rarity: rarity ? rarity : undefined,
    cost: cost ? Number(cost) : undefined,
    mode: mode ? mode : undefined,
    face: askedFace(face),
  };
}

/** R4673001: a face's adjusted rate on the drill-down's face line, as Adjusted reads; not enough games below its floor. */
function faceRateText(figures: PublicFaceFigures): string {
  return figures.hasEnoughGames && typeof figures.adjustedWinRate === "number"
    ? adjustedText(figures.adjustedWinRate)
    : t("not enough games");
}

/** R4673000: one end of the Radiant gap, signed, to a tenth of a point as Adjusted reads. */
function gapPercent(share: number): string {
  return formatNumber(share, {
    style: "percent",
    minimumFractionDigits: ADJUSTED_RANGE_DIGITS,
    maximumFractionDigits: ADJUSTED_RANGE_DIGITS,
    signDisplay: "exceptZero",
  });
}

/** R4676001, R4679001: the Extended stats panel's sections, in its Segmented's order. */
type ExtendedSection = "choices" | "outputs" | "faces";
const EXTENDED_SECTIONS: readonly ExtendedSection[] = ["choices", "outputs", "faces"];

/** R4676001, R4679001: a section's segment, its name and its test id. */
function sectionOption(section: ExtendedSection): { label: string; testId: string } {
  switch (section) {
    case "choices":
      return { label: t("Choices"), testId: statsTestid.extendedChoicesTab };
    case "outputs":
      return { label: t("Outputs"), testId: statsTestid.extendedOutputsTab };
    case "faces":
      return { label: t("Radiant vs normal"), testId: statsTestid.extendedFacesTab };
  }
}

/** R4676001: the panel an address opens beside `card`; none, or a section the page does not know, is closed. */
function extendedOf(params: URLSearchParams): ExtendedSection | null {
  const asked = params.get("extended");
  return EXTENDED_SECTIONS.find((known) => known === asked) ?? null;
}

/** R4676000: a choice's share of the card's plays, a whole percent. */
function shareText(share: number): string {
  return formatNumber(share, { style: "percent", maximumFractionDigits: 0 });
}

/** R4676000: the heading of a kind of choice's table. */
function kindTitle(kind: PublicVariantGroup["kind"]): string {
  switch (kind) {
    case "mode":
      return t("Options chosen");
    case "x":
      return t("X paid");
    case "embiggen":
      return t("Embiggened");
  }
}

/**
 * R4676000: a choice's words: a Choose One row's options each as the card names it in play (`modeText`), its
 * numbers filled from the face the page reads, a "choose two"'s pair joined; the server's label otherwise.
 */
function choiceLabel(row: PublicVariantRow, def: CardDef | undefined, radiant: boolean): string {
  if (row.options === undefined || row.options.length === 0) return row.label;
  const face = radiant ? "radiant" : "base";
  return row.options
    .map((option) => {
      const { label } = modeText(def?.id, option, radiant);
      return def === undefined ? label : fillParams({ params: def.params, base: { ...def.base, text: label }, radiant: { ...def.radiant, text: label } }, face);
    })
    .join(" + ");
}

/**
 * R4676000: the panel's Choices: one table per kind of choice the card has, its rows by share as the server sends
 * them, each with the deck table's tier chip, Adjusted and, with the ranges shown, its 95% range.
 */
function ChoicesTables({
  variants,
  def,
  radiant,
  showRanges,
}: {
  variants: PublicVariantGroup[];
  def: CardDef | undefined;
  radiant: boolean;
  showRanges: boolean;
}): ReactElement {
  return (
    <section className="stats-drilldown-section" data-testid={statsTestid.extendedChoices}>
      {variants.map((group) => (
        <table key={group.kind} className="stats-table stats-extended-table" data-kind={group.kind} data-testid={statsTestid.extendedKind}>
          <caption>{kindTitle(group.kind)}</caption>
          <thead>
            <tr>
              <th scope="col">{t("Choice")}</th>
              <th scope="col">{t("Share")}</th>
              <th scope="col">{t("Tier")}</th>
              <th scope="col">{t("Adjusted")}</th>
              <th scope="col">{t("Games")}</th>
            </tr>
          </thead>
          <tbody>
            {group.rows.map((row) => (
              <tr key={row.key} data-key={row.key} data-testid={statsTestid.extendedRow}>
                <th scope="row">{choiceLabel(row, def, radiant)}</th>
                <td>{shareText(row.share)}</td>
                <td>
                  <TierChip tier={row.hasEnoughGames ? row.tier : null} testid={statsTestid.extendedTier} />
                </td>
                <td>
                  {row.hasEnoughGames ? adjustedText(row.adjustedWinRate) : t("not enough games")}
                  {showRanges && row.hasEnoughGames && (
                    <RangeText low={row.adjustedLow} high={row.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                  )}
                </td>
                <td>{formatNumber(row.games)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ))}
    </section>
  );
}

/**
 * R4679001: the panel's Outputs: one row per card the card made or offered, in the server's order, best adjusted
 * first, each with the deck table's Card cell, its name, share, games, tier chip and Adjusted (its 95% range with the
 * ranges shown), and a Discover-like card's Offered and Pick rate. A row opens that card's own drill-down.
 */
function OutputsTable({
  outputs,
  showRanges,
  onOpen,
}: {
  outputs: PublicOutputRow[];
  showRanges: boolean;
  onOpen: (id: string) => void;
}): ReactElement {
  const titleId = useId();
  const picks = outputs.some((row) => row.offered !== undefined);
  return (
    <section className="stats-drilldown-section" data-testid={statsTestid.extendedOutputs}>
      {outputs.length === 0 ? (
        <p className="stats-empty-sub">{t("No outputs recorded yet.")}</p>
      ) : (
        <TableScroll labeledBy={titleId}>
          <table className="stats-table stats-extended-table">
            <caption id={titleId}>{t("Outputs")}</caption>
            <thead>
              <tr>
                <th className="stats-th--thumb" scope="col">
                  {t("Card")}
                </th>
                <th scope="col">{t("Name")}</th>
                <th scope="col">{t("Share")}</th>
                <th scope="col">{t("Games")}</th>
                <th scope="col">{t("Tier")}</th>
                <th scope="col">{t("Adjusted")}</th>
                {picks && <th scope="col">{t("Offered")}</th>}
                {picks && <th scope="col">{t("Pick rate")}</th>}
              </tr>
            </thead>
            <tbody>
              {outputs.map((row) => (
                <tr
                  key={row.key}
                  className="stats-tr"
                  data-key={row.key}
                  data-testid={statsTestid.extendedOutputRow}
                  onClick={() => onOpen(row.key)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onOpen(row.key);
                    }
                  }}
                >
                  <td className="stats-td stats-td--thumb">
                    <span className="stats-card-thumb-badge" data-rarity={row.rarity}>
                      {row.cost}
                    </span>
                  </td>
                  <th scope="row">
                    <span className="stats-card-name-link">{shownName(row.key, row.label)}</span>
                  </th>
                  <td>{shareText(row.share)}</td>
                  <td>{formatNumber(row.games)}</td>
                  <td>
                    <TierChip tier={row.hasEnoughGames ? row.tier : null} testid={statsTestid.extendedTier} />
                  </td>
                  <td>
                    {row.hasEnoughGames ? adjustedText(row.adjustedWinRate) : t("not enough games")}
                    {showRanges && row.hasEnoughGames && (
                      <RangeText low={row.adjustedLow} high={row.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                    )}
                  </td>
                  {picks && <td>{row.offered === undefined ? "—" : formatNumber(row.offered)}</td>}
                  {picks && <td>{row.pickRate === undefined ? "—" : shareText(row.pickRate)}</td>}
                </tr>
              ))}
            </tbody>
          </table>
        </TableScroll>
      )}
    </section>
  );
}

/** R4707002: the heading of a kind of rolled outcome's table. */
function rolledHeading(kind: PublicRolledGroup["kind"]): string {
  switch (kind) {
    case "effect":
      return t("Rolled effects");
    case "coin":
      return t("Coin flips");
    case "die":
      return t("Dice");
  }
}

/** R4707002: a rolled outcome's Expected or Share, a percent to a tenth of a point. */
function rolledShareText(share: number): string {
  return formatNumber(share, {
    style: "percent",
    minimumFractionDigits: ROLLED_SHARE_DIGITS,
    maximumFractionDigits: ROLLED_SHARE_DIGITS,
  });
}

/**
 * R4707002: one kind of the card's rolled outcomes under its Outputs, in the server's order, best adjusted first: each
 * outcome in the server's words, its expected share beside the share it was rolled, its games, the deck table's tier
 * chip and Adjusted (its 95% range with the ranges shown). A row opens nothing.
 */
function RolledTable({ group, showRanges }: { group: PublicRolledGroup; showRanges: boolean }): ReactElement {
  const titleId = useId();
  return (
    <section className="stats-drilldown-section" data-kind={group.kind} data-testid={statsTestid.extendedRolled}>
      <TableScroll labeledBy={titleId}>
        <table className="stats-table stats-extended-table">
          <caption id={titleId}>{rolledHeading(group.kind)}</caption>
          <thead>
            <tr>
              <th scope="col">{t("Outcome")}</th>
              <th scope="col">{t("Expected")}</th>
              <th scope="col">{t("Share")}</th>
              <th scope="col">{t("Games")}</th>
              <th scope="col">{t("Tier")}</th>
              <th scope="col">{t("Adjusted")}</th>
            </tr>
          </thead>
          <tbody>
            {group.rows.map((row) => (
              <tr key={row.key} data-key={row.key} data-testid={statsTestid.extendedRolledRow}>
                <th scope="row">{row.label}</th>
                <td>{rolledShareText(row.expected)}</td>
                <td>{rolledShareText(row.share)}</td>
                <td>{formatNumber(row.games)}</td>
                <td>
                  <TierChip tier={row.hasEnoughGames ? row.tier : null} testid={statsTestid.extendedTier} />
                </td>
                <td>
                  {row.hasEnoughGames ? adjustedText(row.adjustedWinRate) : t("not enough games")}
                  {showRanges && row.hasEnoughGames && (
                    <RangeText low={row.adjustedLow} high={row.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </TableScroll>
    </section>
  );
}

/**
 * R4673001, R4676001: the panel's Radiant vs normal: the card's Normal and Radiant figures, each read against its own
 * face's field, as two rows of the deck table's cells, and Radiant's adjusted rate minus Normal's with its 95% range.
 */
function FacesTable({ faces, showRanges }: { faces: CardFaces; showRanges: boolean }): ReactElement {
  const { gap } = faces;
  const row = (label: string, figures: PublicFaceFigures, testid: string): ReactElement => (
    <tr data-testid={testid}>
      <th scope="row">{label}</th>
      <td>
        <TierChip tier={figures.hasEnoughGames ? figures.tier : null} testid={statsTestid.extendedTier} />
      </td>
      <td>
        {faceRateText(figures)}
        {showRanges && figures.hasEnoughGames && (
          <RangeText low={figures.adjustedLow} high={figures.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
        )}
      </td>
      <td>{formatNumber(figures.games)}</td>
    </tr>
  );
  return (
    <section className="stats-drilldown-section" data-testid={statsTestid.drillDownFaceLine}>
      <table className="stats-table stats-extended-table">
        <thead>
          <tr>
            <th scope="col">{t("Face")}</th>
            <th scope="col">{t("Tier")}</th>
            <th scope="col">{t("Adjusted")}</th>
            <th scope="col">{t("Games")}</th>
          </tr>
        </thead>
        <tbody>
          {row(t("Normal"), faces.normal, statsTestid.drillDownFaceNormal)}
          {row(t("Radiant"), faces.radiant, statsTestid.drillDownFaceRadiant)}
        </tbody>
      </table>
      <p className="stats-extended-gap" data-testid={statsTestid.drillDownFaceGap}>
        {t("Radiant gap {gap}", {
          gap:
            gap === null
              ? t("not enough games")
              : t("{gap} ({low} to {high})", {
                  gap: gapPercent(gap.gap),
                  low: gapPercent(gap.low),
                  high: gapPercent(gap.high),
                }),
        })}
      </p>
    </section>
  );
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

/** R3838000: the page below its Show card text, which the Almanac shares: every face prints its words while it is pressed. */
export default function StatsRoute(): ReactElement {
  const cardText = useCardText();
  return (
    <CardTextFaces shown={cardText.shown}>
      <StatsPage cardText={cardText} />
    </CardTextFaces>
  );
}

function StatsPage({ cardText }: { cardText: CardText }): ReactElement {
  const locale = useLocale();
  const initialParams = useMemo(() => new URLSearchParams(window.location.search), []);

  const [tab, setTab] = useState<TabKey>(() => TAB_KEYS.find((known) => known === initialParams.get("tab")) ?? "cards");
  // R4678000: the Experimental tab reads no patch and no face: their controls are hidden on it.
  const experimental = tab === "experimental";
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
  // R4680001: with a card, the play version its drill-down opens on, by the version it began at.
  const [drillDownVersion, setDrillDownVersion] = useState<string | null>(() => (initialParams.get("card") ? initialParams.get("version") : null));
  // R4680000: the Experimental tab's view, and Versions' Past versions only.
  const [experimentalView, setExperimentalView] = useState<ExperimentalView>(
    () => EXPERIMENTAL_VIEWS.find((known) => known === initialParams.get("experimental")) ?? "running",
  );
  const [pastOnly, setPastOnly] = useState(initialParams.get("past") === "1");
  // R4676001: the drill-down's Extended stats panel and its section, kept in the address beside `card`.
  const [extended, setExtended] = useState<ExtendedSection | null>(() => (initialParams.get("card") ? extendedOf(initialParams) : null));
  // R2895000: the 95% ranges under Win rate and Adjusted, off by default.
  const [showRanges, setShowRanges] = useState(initialParams.get("ranges") === "1");
  const [mode, setMode] = useState<StatsMode>(() => modeOf(initialParams));
  // R4673001: the face the Cards and Tokens tabs read, one state for both, as the view and the ranges are.
  const [face, setFace] = useState<StatsFace>(() => faceOf(initialParams));

  const [visibleCols, setVisibleCols] = useState({
    thumbnail: true,
    games: true,
    winRate: true,
    playRate: true,
  });

  // R2125000: a revisit starts on the table these filters last read and reads it again behind it, kept under the
  // account that read it (R4677000).
  const [cardStatsKept] = useState(() =>
    cachedRead<PublicStatsCardsResponse>(
      statsToken(),
      cardStatsPath(cardStatsOptionsOf(patch, setFilter, rarityFilter, costFilter, mode, face)),
    ),
  );
  const [cardStatsLoading, setCardStatsLoading] = useState(cardStatsKept === undefined);
  const [cardStatsError, setCardStatsError] = useState(false);
  const [cardStatsData, setCardStatsData] = useState<PublicStatsCardsResponse | null>(cardStatsKept ?? null);
  // Bumped by Retry: the effect asks again, so its cleanup cancels a retry like any other request.
  const [cardStatsAttempt, setCardStatsAttempt] = useState(0);
  // R4677001: the patch search's dialog; R4677000: the address named a patch the server keeps for dev accounts.
  const [patchSearchOpen, setPatchSearchOpen] = useState(false);
  const [patchRefused, setPatchRefused] = useState(false);

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
  // R3887000: the Cards tab's Plain text button, kept on the device.
  const plain = usePlainText();

  // R1966000: every set's cards, unfiltered, and the previous patch's for the change.
  const [setsData, setSetsData] = useState<PublicStatsCardsResponse | null>(null);
  const [setsPrevious, setSetsPrevious] = useState<PublicStatsCardsResponse | null>(null);
  const [setsError, setSetsError] = useState(false);
  // Bumped by the Sets tab's Retry, as `cardStatsAttempt` is by the table's.
  const [setsAttempt, setSetsAttempt] = useState(0);

  // R4678001: every play version's figures on the match type, which Running reads; Retry bumps the attempt.
  const [versionsData, setVersionsData] = useState<PublicStatsVersionsResponse | null>(null);
  const [versionsError, setVersionsError] = useState(false);
  const [versionsAttempt, setVersionsAttempt] = useState(0);
  // R4681001: the Hall of Fame's four lists on the match type, which the Hall of Fame view reads; Retry bumps the attempt.
  // R4681001: the Hall of Fame's answer, kept with the request it answers (the match type and the retry),
  // so a new request reads as loading until its own answer lands, with no reset inside the effect.
  const [hall, setHall] = useState<{ key: string; data: PublicHallOfFameResponse | null; error: boolean } | null>(null);
  const [hallAttempt, setHallAttempt] = useState(0);
  const hallKey = `${mode}:${String(hallAttempt)}`;
  const hallData = hall?.key === hallKey ? hall.data : null;
  const hallError = hall?.key === hallKey && hall.error;
  // R5040000: Major patch running's table, the live play versions cut to the newest major's line, kept with the
  // request it answers (the match type and the retry) as the Hall of Fame's is; Retry bumps `versionsAttempt`.
  const [major, setMajor] = useState<{ key: string; data: PublicStatsVersionsResponse | null; error: boolean } | null>(null);
  const majorKey = `${mode}:${String(versionsAttempt)}`;
  const majorData = major?.key === majorKey ? major.data : null;
  const majorError = major?.key === majorKey && major.error;

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
    if (cardDrillDownId && drillDownVersion) params.set("version", drillDownVersion);
    if (experimentalView !== "running") params.set("experimental", experimentalView);
    if (pastOnly) params.set("past", "1");
    if (cardDrillDownId && drillDownVersion === null && extended) params.set("extended", extended);
    if (mode !== DEFAULT_MODE) params.set("mode", mode);
    if (face !== DEFAULT_FACE) params.set("face", face);
    if (showRanges) params.set("ranges", "1");

    const query = params.toString();
    const target = query ? `${paths.stats}?${query}` : paths.stats;
    navigate(target, { replace: true });
  }, [
    tab,
    patch,
    search,
    sortCol,
    sortDir,
    setFilter,
    rarityFilter,
    costFilter,
    keywordFilter,
    view,
    cardDrillDownId,
    drillDownVersion,
    extended,
    experimentalView,
    pastOnly,
    mode,
    face,
    showRanges,
  ]);

  useEffect(() => {
    updateUrl();
  }, [updateUrl]);

  const fetchCardStats = useCallback(
    () => {
      const options = cardStatsOptionsOf(patch, setFilter, rarityFilter, costFilter, mode, face);
      const token = statsToken();
      // R2125000: a table kept for these filters draws at once; only a first visit to them waits behind a skeleton.
      const kept = cachedRead<PublicStatsCardsResponse>(token, cardStatsPath(options));
      if (kept === undefined) {
        setCardStatsLoading(true);
      } else {
        setCardStatsData(kept);
        setCardStatsLoading(false);
      }
      setCardStatsError(false);
      const controller = new AbortController();

      getCardStats({ ...options, token, signal: controller.signal })
        .then((res) => {
          if (controller.signal.aborted) return;
          setCardStatsData(res);
          setCardStatsLoading(false);
        })
        .catch((cause: unknown) => {
          if (controller.signal.aborted) return;
          // R4677000: a patch the server keeps for dev accounts opens the current patch, with a line saying so.
          if (cause instanceof ApiRequestError && cause.code === PATCH_DEV_ONLY) {
            setPatchRefused(true);
            setPatch("");
            return;
          }
          // A read that fails behind a kept table leaves it and says nothing.
          if (kept !== undefined) return;
          // R5186000: a read that found no answer in time while the server still loads the statistics asks
          // again, behind the progress bar, rather than saying they didn't load.
          void askAgain(cause).then((again) => {
            if (controller.signal.aborted) return;
            if (again) {
              setCardStatsAttempt((n) => n + 1);
              return;
            }
            setCardStatsError(true);
            setCardStatsLoading(false);
          });
        });

      return () => controller.abort();
    },
    [patch, setFilter, rarityFilter, costFilter, mode, face],
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
  const choosePatch = (window: PatchWindow): void => {
    setPatchRefused(false);
    setPatch(patchList.indexOf(window) === 0 ? "" : patchTarget(window));
  };
  // R3324000: the newest patch, short of games, shows no figures: its Cards and Sets tabs say it is collecting.
  const collecting = isCollecting(patchList, shownPatch);
  const shownPatchName = shownPatch >= 0 ? patchName(patchList[shownPatch]!) : (cardStatsData?.patch ?? "");
  // R3447000: the patch the figures were collected on: while they are the previous window's (R1367000), the patch
  // before (the version before, from a server without the list); else the patch shown.
  const collectedPatchName =
    cardStatsData?.source === "previous"
      ? previousWindow
        ? patchName(previousWindow)
        : (cardStatsData.previousPatch ?? shownPatchName)
      : shownPatchName;

  // R1966000: the Sets tab reads the patch's every card, whatever the Cards tab filters, then the previous
  // patch's for each set's change; a previous patch that fails to load only leaves the change out. R4673001:
  // both on the page's face.
  useEffect(() => {
    // R3324000: a patch still collecting its data has no sets to read.
    if (tab !== "sets" || collecting) return;
    setSetsError(false);
    const controller = new AbortController();
    const token = statsToken();
    getCardStats({
      patch: patch ? patch : undefined,
      mode: mode ? mode : undefined,
      face: askedFace(face),
      token,
      signal: controller.signal,
    })
      .then((res) => {
        if (controller.signal.aborted) return;
        setSetsData(res);
        setSetsPrevious(null);
        // R1973002: the patch before, not the version before, which may be a micro of the same patch.
        const before = previousPatchOf(res);
        if (!before) return;
        getCardStats({ patch: before.target, mode: mode ? mode : undefined, face: askedFace(face), token, signal: controller.signal })
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
  }, [tab, patch, mode, face, setsAttempt, collecting]);

  // R4678001: Running reads every play version's figures on the page's match type once the Experimental tab is
  // open; a pooled view is no patch query, so it is open to everyone (R4677000) and sends no token. R4680001: so does
  // a play version's drill-down, from any tab. R5040000: every view but Major patch running, which reads its own.
  const versionOpen = cardDrillDownId !== null && drillDownVersion !== null;
  const majorRunning = experimental && experimentalView === "running";
  const wantVersions = (experimental && !majorRunning) || versionOpen;
  useEffect(() => {
    if (!wantVersions) return;
    setVersionsData(null);
    setVersionsError(false);
    const controller = new AbortController();
    getStatsVersions({ mode, signal: controller.signal })
      .then((res) => {
        if (!controller.signal.aborted) setVersionsData(res);
      })
      .catch(() => {
        if (!controller.signal.aborted) setVersionsError(true);
      });
    return () => controller.abort();
  }, [wantVersions, mode, versionsAttempt]);

  // R5040000: Major patch running asks the route for the newest major's line on the page's match type; the server
  // cuts each span, so the client reads no version.
  useEffect(() => {
    if (!majorRunning) return;
    const key = `${mode}:${String(versionsAttempt)}`;
    const controller = new AbortController();
    getStatsVersions({ mode, scope: "major", signal: controller.signal })
      .then((res) => {
        if (!controller.signal.aborted) setMajor({ key, data: res, error: false });
      })
      .catch(() => {
        if (!controller.signal.aborted) setMajor({ key, data: null, error: true });
      });
    return () => controller.abort();
  }, [majorRunning, mode, versionsAttempt]);

  // R4681001: the Hall of Fame's lists on the page's match type once its view is open; a pooled view is
  // no patch query, so it is open to everyone (R4677000) and sends no token.
  const wantHall = experimental && experimentalView === "hall";
  useEffect(() => {
    if (!wantHall) return;
    const key = `${mode}:${String(hallAttempt)}`;
    const controller = new AbortController();
    getHallOfFame({ mode, signal: controller.signal })
      .then((res) => {
        if (!controller.signal.aborted) setHall({ key, data: res, error: false });
      })
      .catch(() => {
        if (!controller.signal.aborted) setHall({ key, data: null, error: true });
      });
    return () => controller.abort();
  }, [wantHall, mode, hallAttempt]);

  // R4673001: a card's details on the page's match type, face and patch (none asked for the newest patch).
  // R4678001: from the Experimental tab, which shows neither, on All and the newest patch.
  const detailsFace = experimental ? DEFAULT_FACE : face;
  const detailsPatch = experimental ? "" : patch;
  useEffect(() => {
    // R4680001: a play version's drill-down reads no patch's details.
    if (!cardDrillDownId || drillDownVersion !== null) {
      setDrillDownData(null);
      return;
    }
    setDrillDownLoading(true);
    setDrillDownError(false);
    const controller = new AbortController();
    getCardDrillDown(cardDrillDownId, {
      mode: mode ? mode : undefined,
      face: askedFace(detailsFace),
      patch: detailsPatch ? detailsPatch : undefined,
      token: statsToken(),
      signal: controller.signal,
    })
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
  }, [cardDrillDownId, drillDownVersion, mode, detailsFace, detailsPatch, drillDownAttempt]);

  // The drill-down is a modal like the client's others (docs/polish/6-cards.md, B25), drawn through
  // Dialog.tsx (R3221000), whose keys close it and keep Tab inside, and Back too (R2127000). The control
  // that opened it is kept when it is pressed, not read off the focus, which a click on a button does
  // not move in every browser, and has the focus back once it closes.
  const drillDownOpener = useRef<HTMLElement | null>(null);
  const drillDownOpen = Boolean(cardDrillDownId);

  const openDrillDown = (id: string, opener: HTMLElement): void => {
    drillDownOpener.current = opener;
    setCardDrillDownId(id);
    setDrillDownVersion(null);
    setExtended(null);
  };
  // R4679001: an output row opens its card in the same dialog, its details shown. The row is gone with the panel, so
  // the focus moves to the Extended stats toggle, which the new card keeps, and still comes back to whatever opened
  // the dialog once it closes.
  const extendedToggle = useRef<HTMLButtonElement | null>(null);
  const openOutput = (id: string): void => {
    extendedToggle.current?.focus({ preventScroll: true });
    setCardDrillDownId(id);
    setDrillDownVersion(null);
    setExtended(null);
  };
  const closeDrillDown = (): void => {
    setCardDrillDownId(null);
    setDrillDownVersion(null);
    setExtended(null);
  };

  useEffect(() => {
    if (drillDownOpen) return;
    const opener = drillDownOpener.current;
    if (opener !== null && opener.isConnected) opener.focus({ preventScroll: true });
  }, [drillDownOpen]);

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
    for (const [id, def] of Object.entries(catalog())) {
      if (def.token) continue;
      for (const kind of keywordKinds(id)) {
        present.add(kind);
      }
    }
    return KEYWORD_KINDS.filter((kind) => present.has(kind));
  }, []);

  // R5033000: the Cards tab offers no Date, so it reads one as Adjusted in its direction, as Tokens does; the state
  // keeps `date` for the address and the other tabs.
  const cardsCol: Exclude<SortColumn, "date"> = sortCol === "date" ? DEFAULT_SORT : sortCol;
  const handleSort = (col: SortColumn) => {
    if (cardsCol === col) {
      setSortDir((prev) => (prev === "asc" ? "desc" : "asc"));
    } else {
      setSortCol(col);
      setSortDir(defaultDirection(col));
    }
  };

  // R4083000: the Tokens tab sorts by the Cards tab's one sort, kept in the address, a column its tables lack read as
  // Adjusted; a Created header sets it as a deck header does.
  const createdSort = createdSortOf(sortCol, sortDir);
  // R4678001: Running sorts by it too; R5036000: Decisiveness among its columns, and R5033000 Date.
  const runningSort = runningSortOf(sortCol, sortDir);
  const sortCreatedBy = (next: CreatedSort): void => {
    setSortCol(next.col);
    setSortDir(next.dir);
  };

  // The cards the shared filters keep, before any sort.
  const unsortedCards = useMemo(() => {
    if (!cardStatsData) return [];
    let list = [...cardStatsData.cards];

    if (search.trim()) {
      list = list.filter((c) => matchesSearch(c, search));
    }

    if (keywordFilter) {
      list = list.filter((c) => keywordKinds(c.id).includes(keywordFilter as KeywordKind));
    }
    return list;
  }, [cardStatsData, search, keywordFilter]);

  const sortCards = useCallback(
    (cards: readonly (typeof unsortedCards)[number][], cardsCol: Exclude<SortColumn, "date">, sortDir: SortDirection) => {
      const list = [...cards];
      const flip = sortDir === "desc" ? -1 : 1;
      const language = locale === "en" ? undefined : locale;
      const compareNames = (a: { id: string; name: string }, b: { id: string; name: string }): number =>
        shownName(a.id, a.name).localeCompare(shownName(b.id, b.name), language);
      list.sort((a, b) => {
        if (cardsCol === "tier") {
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
        } else if (cardsCol === "adjustedWinRate" || cardsCol === "decisiveness") {
          // R2777000: a card with no adjusted rate (nothing seen, or never unseen) sorts after those with one;
          // R4552001: Decisiveness sorts the same way.
          if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
          if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
          const aRate = (cardsCol === "decisiveness" ? a.decisiveness : a.adjustedWinRate) ?? null;
          const bRate = (cardsCol === "decisiveness" ? b.decisiveness : b.adjustedWinRate) ?? null;
          if (aRate !== null && bRate === null) return -1;
          if (aRate === null && bRate !== null) return 1;
          if (aRate !== null && bRate !== null && aRate !== bRate) {
            return (aRate - bRate) * flip;
          }
        } else if (cardsCol === "winRate") {
          if (a.hasEnoughGames && !b.hasEnoughGames) return -1;
          if (!a.hasEnoughGames && b.hasEnoughGames) return 1;
          if (a.winRate !== null && b.winRate !== null && a.winRate !== b.winRate) {
            return (a.winRate - b.winRate) * flip;
          }
        } else if (cardsCol === "games") {
          if (a.games !== b.games) return (a.games - b.games) * flip;
        } else if (cardsCol === "playRate") {
          if (a.playRate !== b.playRate) return (a.playRate - b.playRate) * flip;
        } else if (cardsCol === "name") {
          const diff = compareNames(a, b);
          if (diff !== 0) return diff * flip;
        }
        return a.games - b.games || compareNames(a, b);
      });

      return list;
    },
    [locale, tiers],
  );
  const filteredCards = useMemo(() => sortCards(unsortedCards, cardsCol, sortDir), [sortCards, unsortedCards, cardsCol, sortDir]);
  // R5033000: Running and Versions take the Cards tab's order as the order their ties keep. Under a Date sort that is
  // Adjusted in its default direction whatever the sort's own, so a row tied on Date keeps one place both ways.
  const rowsInput = useMemo(
    () => (sortCol === "date" ? sortCards(unsortedCards, DEFAULT_SORT, defaultDirection(DEFAULT_SORT)) : filteredCards),
    [sortCol, sortCards, unsortedCards, filteredCards],
  );

  // R2089000: the Created cards the search box asks for, as it asks for the deck rows, and R4083000 the keyword;
  // R3324002: the tokens among them in a section of their own, and every other in its own, so no card is in both.
  const createdCards = (cardStatsData?.created ?? []).filter(
    (card) =>
      matchesSearch(card, search) && (!keywordFilter || keywordKinds(card.id).includes(keywordFilter as KeywordKind)),
  );
  const tokenCards = createdCards.filter((card) => catalog()[card.id]?.token === true);
  const otherCreatedCards = createdCards.filter((card) => catalog()[card.id]?.token !== true);

  // R4678001: the cards the Cards tab lists under the shared filters, each on its live play version. R5040000: Major
  // patch running reads the cut table, Historical and the other views the whole one.
  const experimentalData = majorRunning ? majorData : versionsData;
  const experimentalError = majorRunning ? majorError : versionsError;
  const running = useMemo(
    () => (experimentalData ? runningRows(rowsInput, experimentalData.versions) : []),
    [rowsInput, experimentalData],
  );

  // R4680000: each old version's definition, from its catalog once a row on screen or its drill-down asks for it; a
  // live one's is the catalog's own.
  const { snapshots: versionSnapshots, need: needSnapshot } = useVersionSnapshots();
  const readVersion = useCallback(
    (version: PublicPlayVersion): VersionRead | undefined => {
      const snapshot = version.live ? catalog() : versionSnapshots.get(version.last);
      if (snapshot === undefined) return undefined;
      const def = snapshot?.[version.card];
      return snapshot === null || def === undefined ? null : { def, names: snapshot };
    },
    [versionSnapshots],
  );
  const needVersion = useCallback(
    (version: PublicPlayVersion): void => {
      if (!version.live) needSnapshot(version.last);
    },
    [needSnapshot],
  );
  const versionById = useMemo(() => new Map((versionsData?.versions ?? []).map((version) => [version.id, version])), [versionsData]);
  // R4680000: every play version of the cards the Cards tab lists, or the closed ones alone.
  const versionsShown = useMemo(
    () =>
      versionsData
        ? versionRows(rowsInput, versionsData.versions, pastOnly, (version) => {
            const read = readVersion(version);
            return read === undefined ? undefined : (read?.def ?? null);
          })
        : [],
    [rowsInput, versionsData, pastOnly, readVersion],
  );
  // R4681001: whether the Hall of Fame's four lists are all empty.
  const hallEmpty =
    hallData !== null &&
    (["broken", "nerfed", "buffed", "changed"] as const).every((key) => hallData[key].length === 0);
  const versionFaceOf = (id: string): ReactNode => {
    const version = versionById.get(id);
    if (version === undefined) return null;
    if (version.live) return gridFaceOf(version.card);
    const read = readVersion(version);
    return read ? <StatsVersionGridFace def={read.def} /> : null;
  };
  const versionPlainOf = (id: string): ReactNode => {
    const version = versionById.get(id);
    const read = version === undefined ? null : readVersion(version);
    return read ? <PlainCard def={read.def} /> : null;
  };
  const openVersion = (id: string, opener: HTMLElement): void => {
    const version = versionById.get(id);
    if (version === undefined) return;
    drillDownOpener.current = opener;
    setCardDrillDownId(version.card);
    setDrillDownVersion(version.first);
    setExtended(null);
  };
  // R4681001: a Hall of Fame entry side's face from its own catalog, as the drill-down draws it; the
  // version's name stands in for it until that catalog loads.
  const hallFaceOf = (version: PublicPlayVersion): ReactNode => {
    const read = readVersion(version);
    return read ? (
      <span className="hall-face-card">
        <PatchFace def={read.def} face="base" />
      </span>
    ) : (
      <span className="hall-face-card hall-face-card--empty">{version.name}</span>
    );
  };
  const hallNeedOf = (version: PublicPlayVersion): string | undefined =>
    version.live || readVersion(version) ? undefined : version.last;
  // R4680001: the open play version, and its card's versions either side of it.
  const versionsOfOpen = cardDrillDownId && versionsData ? versionsData.versions.filter((version) => version.card === cardDrillDownId) : [];
  const openAt = versionsOfOpen.findIndex((version) => version.first === drillDownVersion);
  const openPlayVersion = versionOpen && openAt >= 0 ? versionsOfOpen[openAt] : undefined;

  const summary = cardStatsData?.summary;


  const drillDownDef = cardDrillDownId ? catalog()[cardDrillDownId] : undefined;
  // #4511: built once per card, so a keystroke elsewhere on the page never rebuilds the open drill-down's faces.
  const drillDownFace = useMemo(
    () => (drillDownDef ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: false }) : null),
    [drillDownDef, locale],
  );
  const drillDownRadiantFace = useMemo(
    () =>
      drillDownDef && drillDownRadiant ? faceModel({ defId: drillDownDef.id, def: drillDownDef, radiant: true }) : null,
    [drillDownDef, drillDownRadiant, locale],
  );
  // R4673001: the open card's face line, never the card before's while its details load or after they fail;
  // none from a server deployed before it.
  const drillDownFaces = drillDownLoading || drillDownError ? undefined : drillDownData?.faces;
  const drillDownTokens = drillDownDef ? tokensOf(drillDownDef) : [];
  // R4676001: the panel's sections the open card has, Choices only when it offers a choice, R4679001 Outputs only when
  // it made or offered a card, R4707002 rolled an outcome, or its text names a token; the one shown is the one asked
  // for, or the first the card has.
  const drillDownVariants = drillDownLoading || drillDownError ? [] : (drillDownData?.variants ?? []);
  const drillDownOutputs = drillDownLoading || drillDownError ? [] : (drillDownData?.outputs ?? []);
  const drillDownRolled = drillDownLoading || drillDownError ? [] : (drillDownData?.rolled ?? []);
  const extendedSections = EXTENDED_SECTIONS.filter((section) => {
    switch (section) {
      case "choices":
        return drillDownVariants.length > 0;
      case "outputs":
        return drillDownOutputs.length > 0 || drillDownRolled.length > 0 || drillDownTokens.length > 0;
      case "faces":
        return drillDownFaces !== undefined;
    }
  });
  const extendedShown = extended !== null && extendedSections.includes(extended) ? extended : extendedSections[0];
  // R3887000: every card its text names, for the breakdown's Reference cards while Plain text is on.
  const drillDownRefs = drillDownDef ? referencesOf(drillDownDef) : [];

  // R4279001: the Cards and Tokens tabs' filters, one set of state: the search, the set, rarity and cost the
  // read asks for, the match type, and the keyword, which both tabs read (R4083000).
  const cardFilters = (): ReactElement => (
    <>
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
    </>
  );

  // R3324003: how the cards show, one row of labelled groups, every on/off control a pressed button. R4083000: the
  // Tokens tab shows the same row, its sort select offering only the columns its tables have and no Play rate column.
  // R4678001: so does Running, with no Face, which it does not read (R4678000).
  // R4680000: and Versions, as Running, with its Past versions only.
  // R5037000: Running and Versions have the Cards tab's Win rate column and Play rate behind the Columns toggle.
  // R4681001: the Hall of Fame keeps only the ranges, which its entries read; its ranks are the order.
  const cardOptions = (shown: "cards" | "tokens" | "running" | "versions" | "hall"): ReactElement => {
    if (shown === "hall") {
      return (
        <div className="stats-options">
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
        </div>
      );
    }
    const columns: readonly SortColumn[] =
      shown === "cards" ? SORT_COLUMNS : shown === "tokens" ? CREATED_SORT_COLUMNS : RUNNING_SORT_COLUMNS;
    const sortShown = shown === "cards" ? cardsCol : shown === "tokens" ? createdSort.col : runningSort.col;
    return (
      <div className="stats-options">
        <div className="stats-sort">
          <span className="stats-sort-label" id="stats-sort-label">
            {t("Sort")}
          </span>
          <select
            className="stats-select"
            aria-labelledby="stats-sort-label"
            data-testid={statsTestid.sortSelect}
            value={sortShown}
            onChange={(e) => {
              const next = e.target.value as SortColumn;
              setSortCol(next);
              setSortDir(defaultDirection(next));
            }}
          >
            {columns.map((col) => (
              <option key={col} value={col}>
                {t(SORT_LABELS[col])}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="stats-sort-dir"
            data-testid={statsTestid.sortDir}
            aria-label={sortDir === "asc" ? t("Ascending; switch to descending") : t("Descending; switch to ascending")}
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

        {/* R4673001: the face every figure of both tabs reads, kept in the address; its buttons carry their words alone (R3184000) */}
        {(shown === "cards" || shown === "tokens") && (
          <div className="stats-view-toggle">
            <span className="stats-sort-label" aria-hidden="true">
              {t("Face")}
            </span>
            <Segmented
              label={t("Face")}
              value={face}
              onChange={setFace}
              options={[
                { value: "all", label: t("All"), testId: statsTestid.faceAll, className: "stats-view-btn" },
                { value: "normal", label: t("Normal"), testId: statsTestid.faceNormal, className: "stats-view-btn" },
                { value: "radiant", label: t("Radiant"), testId: statsTestid.faceRadiant, className: "stats-view-btn" },
              ]}
            />
          </div>
        )}

        {/* R4680000: the closed versions alone, an on/off button in the Segmented's look (R3324003) */}
        {shown === "versions" && (
          <div className="segmented">
            <button
              type="button"
              className={`segmented__item stats-view-btn ${pastOnly ? "segmented__item--selected" : ""}`}
              data-testid={statsTestid.versionsPastOnly}
              aria-pressed={pastOnly}
              onClick={() => setPastOnly((only) => !only)}
            >
              {t("Past versions only")}
            </button>
          </div>
        )}

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

        {/* R3887000: every card as plain text, no face; the device keeps it, apart from Show card text */}
        <PlainTextButton shown={plain.shown} onToggle={plain.toggle} />

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
              {shown !== "tokens" && (
                <button
                  type="button"
                  className={`segmented__item stats-view-btn ${visibleCols.playRate ? "segmented__item--selected" : ""}`}
                  data-testid={statsTestid.columnPlayRate}
                  aria-pressed={visibleCols.playRate}
                  onClick={() => setVisibleCols((c) => ({ ...c, playRate: !c.playRate }))}
                >
                  {t("Play rate")}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <CardDefsProvider defs={catalog()}>
      <div className="app-shell app-shell--wide tavern stats-page one-screen" data-testid={statsTestid.screen}>
        <header>
          <BackLink title={t("Statistics")}>
            {/* R3324001: the patch the game is on now, whichever patch the page shows */}
            <span className="stats-current-patch" data-testid={statsTestid.currentPatch}>
              {t("Current patch {patch}", { patch: currentVersion(patchList) })}
            </span>
            <CardTextButton shown={cardText.shown} onToggle={cardText.toggle} />
            <FetchLive />
          </BackLink>
        </header>

        {/* R4678000: the Experimental tab reads no patch: one line names what it shows in the patch bar's place */}
        {experimental && (
          <p className="stats-experimental-line" data-testid={statsTestid.experimentalLine}>
            {experimentalView === "versions"
              ? t("Every version of every card, each over the patches it was played on")
              : experimentalView === "hall"
                ? t("The most broken, nerfed, buffed and changed versions of all time")
                : experimentalView === "historical"
                  ? t("Every card's current version, over every patch since it last changed")
                  : t("Every card's current version, over this major's patches since it last changed")}
          </p>
        )}

        {/* R1973002, R3324000, R4674000: a tab for each offered patch; the arrows follow the tabs: left steps newer, right steps older */}
        {patchList.length > 0 && !experimental && (
          <nav className="stats-patch-bar" data-testid={statsTestid.patchBar} aria-label={t("Patches")}>
            <button
              type="button"
              className="stats-patch-step"
              data-testid={statsTestid.patchNext}
              disabled={nextWindow === null}
              aria-label={nextWindow ? t("Newer patch ({patch})", { patch: patchName(nextWindow) }) : t("Newer patch")}
              title={nextWindow ? t("Newer patch ({patch})", { patch: patchName(nextWindow) }) : t("Newer patch")}
              onClick={() => nextWindow && choosePatch(nextWindow)}
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
              data-testid={statsTestid.patchPrevious}
              disabled={previousWindow === null}
              aria-label={previousWindow ? t("Older patch ({patch})", { patch: patchName(previousWindow) }) : t("Older patch")}
              title={previousWindow ? t("Older patch ({patch})", { patch: patchName(previousWindow) }) : t("Older patch")}
              onClick={() => previousWindow && choosePatch(previousWindow)}
            >
              →
            </button>
            {/* R4677001: one button at the bar's end opens the search through every patch the caller may read */}
            <button
              type="button"
              className="stats-patch-step stats-patch-search"
              data-testid={statsTestid.patchSearch}
              aria-label={t("Find a patch")}
              title={t("Find a patch")}
              aria-haspopup="dialog"
              onClick={() => setPatchSearchOpen(true)}
            >
              <FindPatchIcon />
            </button>
          </nav>
        )}
        {/* R4677000: the address named a patch the server keeps for dev accounts; the page shows the current one */}
        {patchRefused && !experimental && (
          <p className="stats-patch-refused" role="status" data-testid={statsTestid.patchRefused}>
            {olderPatchesLine()}
          </p>
        )}
        {patchSearchOpen && (
          <PatchSearch
            patches={patchList}
            shown={shownPatch}
            olderPatchesForDevs={cardStatsData?.olderPatchesForDevs === true}
            onPick={(window) => {
              setPatchSearchOpen(false);
              choosePatch(window);
            }}
            onClose={() => setPatchSearchOpen(false)}
          />
        )}

        {/* The summary: the patch's games alone (R661); R1646000 took the top and lowest card tiles off the fixed top;
            a patch still collecting its data shows no count (R3324000); the tile names the patch its games were
            collected on (R3447000); R4678000: none on the Experimental tab */}
        {cardStatsData && !collecting && !experimental && (
          <section className="stats-summary-tiles" data-testid={statsTestid.summaryTiles} aria-label={t("Statistics summary")}>
            <div className="stats-summary-tile" data-testid={statsTestid.summaryTotalGames}>
              <span className="stats-summary-label">{t("Games on patch {patch}", { patch: collectedPatchName })}</span>
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
              {cardFilters()}

              {cardOptions("cards")}
            </div>

            {/* Table or Grid */}
            {cardStatsLoading ? (
              <>
                <StatsLoadProgress />
                <Skeleton variant={view === "grid" ? "grid" : "table"} label={t("Loading card statistics…")} />
              </>
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
              <ul className={`stats-grid${plain.shown ? " stats-grid--plain" : ""}`} data-testid={statsTestid.cardsGrid}>
                {filteredCards.map((card) => {
                  const plainCard = plain.shown ? plainCardOf(card.id) : null;

                  return (
                    <li key={card.id} className="stats-grid-item" data-card-id={card.id}>
                      <button
                        type="button"
                        className={`stats-grid-card${plainCard === null ? "" : " stats-grid-card--plain"}`}
                        onClick={(e) => openDrillDown(card.id, e.currentTarget)}
                        aria-label={shownName(card.id, card.name)}
                      >
                        {plainCard ?? <StatsGridFace defId={card.id} />}
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
                        <dt>{t("Decisiveness")}</dt>
                        <dd>{card.hasEnoughGames ? decisivenessText(card.decisiveness) : t("not enough games")}</dd>
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
                        aria-sort={cardsCol === "name" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("name")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("name");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Name")} {cardsCol === "name" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      {visibleCols.games && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={cardsCol === "games" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("games")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("games");
                            }
                          }}
                          tabIndex={0}
                        >
                          {t("Games")} {cardsCol === "games" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        title={t("S needs a nerf and F a buff: S is above the band and sure of it, F below it and sure of it, and A to D are the band's quarters")}
                        aria-sort={cardsCol === "tier" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("tier")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("tier");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Tier")} {cardsCol === "tier" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        title={t("50% plus how far drawing the card moves its deck's win rate beyond the average card's, counted only as far as its games bear out")}
                        aria-sort={cardsCol === "adjustedWinRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("adjustedWinRate")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("adjustedWinRate");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Adjusted")} {cardsCol === "adjustedWinRate" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        title={t(
                          "Decisiveness: how far playing a card typically moves your chance of winning, one way or the other; ± is the swing itself, not a margin of error. A typical card: {typical}.",
                          { typical: decisivenessText(tiers?.decisiveness) },
                        )}
                        aria-sort={cardsCol === "decisiveness" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("decisiveness")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("decisiveness");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Decisiveness")} {cardsCol === "decisiveness" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      <th
                        scope="col"
                        className="stats-th stats-th--sortable"
                        aria-sort={cardsCol === "winRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                        onClick={() => handleSort("winRate")}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleSort("winRate");
                          }
                        }}
                        tabIndex={0}
                      >
                        {t("Win rate")} {cardsCol === "winRate" && (sortDir === "asc" ? "↑" : "↓")}
                      </th>
                      {visibleCols.playRate && (
                        <th
                          scope="col"
                          className="stats-th stats-th--sortable"
                          aria-sort={cardsCol === "playRate" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                          onClick={() => handleSort("playRate")}
                          onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === " ") {
                              e.preventDefault();
                              handleSort("playRate");
                            }
                          }}
                          tabIndex={0}
                        >
                          {t("Play rate")} {cardsCol === "playRate" && (sortDir === "asc" ? "↑" : "↓")}
                        </th>
                      )}
                    </tr>
                  </thead>
                  <tbody>
                    {filteredCards.map((card) => {
                      const plainCard = plain.shown ? plainCardOf(card.id) : null;
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
                            {plainCard ?? <span className="stats-card-name-link">{shownName(card.id, card.name)}</span>}
                          </td>
                          {visibleCols.games && <td className="stats-td">{card.games}</td>}
                          <td className="stats-td">
                            <TierChip tier={card.hasEnoughGames ? card.tier : null} testid={statsTestid.tierCell} />
                            {showRanges && <TierFitText fit={cardTierFit(card, tiers)} />}
                          </td>
                          {/* R4084000: the bar lives in the Adjusted cell, sized by the adjusted rate on the same 0–100% scale the Win rate bar used. */}
                          <td className="stats-td" data-testid={statsTestid.adjustedCell}>
                            {card.hasEnoughGames && typeof card.adjustedWinRate === "number" ? (
                              <div className="stats-winrate-cell">
                                <span className="stats-winrate-text">{adjustedText(card.adjustedWinRate)}</span>
                                <div className="stats-confidence-bar">
                                  <div
                                    className="stats-confidence-fill"
                                    style={{ width: `${Math.round(card.adjustedWinRate * 100)}%` }}
                                  />
                                </div>
                                {showRanges && (
                                  <RangeText low={card.adjustedLow} high={card.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
                                )}
                              </div>
                            ) : (
                              "—"
                            )}
                          </td>
                          <td className="stats-td" data-testid={statsTestid.decisivenessCell}>
                            {card.hasEnoughGames ? (
                              decisivenessText(card.decisiveness)
                            ) : (
                              <span className="stats-not-enough">{t("not enough games")}</span>
                            )}
                          </td>
                          <td className="stats-td stats-td--winrate">
                            {card.hasEnoughGames && card.winRate !== null ? (
                              <>
                                <span className="stats-winrate-text">{Math.round(card.winRate * 100)}%</span>
                                {showRanges && (
                                  <RangeText low={card.winRateLow} high={card.winRateHigh} digits={WIN_RATE_RANGE_DIGITS} testid={statsTestid.winRateRange} />
                                )}
                              </>
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
          </>
        ),
      },
      {
        id: "tokens",
        label: t("Tokens"),
        testId: statsTestid.tabTokens,
        panelClassName: "stats-panel one-screen__body",
        // R4279001: the cards made during a game, a tab of their own beside the deck cards' (R2089000, R3324002,
        // R4155000): the tokens, then every other, no card in both, in the deck table's columns, each row
        // opening the drill-down, under the Cards tab's filters. R4083000: the Cards tab's sort, view, ranges and
        // keyword too, one state for both tabs. R3324000: a collecting patch shows no figures.
        panel: collecting ? (
          <CollectingNote patch={shownPatchName} previous={previousWindow} onPrevious={choosePatch} />
        ) : (
          <>
            <div className="stats-filter-bar">
              {cardFilters()}
              {cardOptions("tokens")}
            </div>

            {cardStatsLoading ? (
              <Skeleton variant={view === "grid" ? "grid" : "table"} label={t("Loading card statistics…")} />
            ) : cardStatsError ? (
              <div className="stats-error">
                <p>{t("The card statistics didn't load.")}</p>
                <button type="button" className="stats-retry-btn" onClick={() => setCardStatsAttempt((n) => n + 1)}>
                  {t("Retry")}
                </button>
              </div>
            ) : tokenCards.length === 0 && otherCreatedCards.length === 0 ? (
              <EmptyState text={t("No tokens or created cards match the selected filters.")} />
            ) : (
              <>
                {(
                  [
                    ["tokens", tokenCards],
                    ["other", otherCreatedCards],
                  ] as const
                ).map(([kind, created]) => (
                  <CreatedPanel
                    key={kind}
                    kind={kind}
                    created={created}
                    sort={createdSort}
                    onSort={sortCreatedBy}
                    view={view}
                    showRanges={showRanges}
                    tiers={tiers}
                    nameOf={shownName}
                    faceOf={gridFaceOf}
                    plainCardOf={plain.shown ? plainCardOf : undefined}
                    visibleThumbnail={visibleCols.thumbnail}
                    visibleGames={visibleCols.games}
                    onOpen={(id, opener) => openDrillDown(id, opener)}
                  />
                ))}
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
                                  {" "}<Icon name="star" />
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
                      const favCardDef = topCard ? catalog()[topCard.id] : undefined;
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
      {
        id: "experimental",
        label: t("Experimental"),
        testId: statsTestid.tabExperimental,
        panelClassName: "stats-panel one-screen__body",
        // R4678000: figures that span patches, in a Segmented of its views, now four (R4680000, R4681001, R5040000).
        // R5040000: Major patch running and Historical (R4678001's Running), each card the Cards tab's filters list, on
        // its live play version, over the newest major's line or every patch; R4680000: Versions, every play version
        // of those cards; R4681001: Hall of Fame, the four all-time lists, which read no filter.
        panel: (
          <>
            <Segmented
              label={t("Experimental views")}
              className="stats-experimental-views"
              value={experimentalView}
              onChange={setExperimentalView}
              options={[
                {
                  value: "running",
                  label: t("Major patch running"),
                  title: t(
                    "Each card's current version, over the patches since the newest major or since it last changed, whichever is later. Every row starts again at each major, so it has too few games for a tier until games accrue.",
                  ),
                  testId: statsTestid.experimentalRunning,
                  className: "stats-view-btn",
                },
                {
                  value: "historical",
                  label: t("Historical"),
                  title: t("Each card's current version, over every patch since it last changed, across majors."),
                  testId: statsTestid.experimentalHistorical,
                  className: "stats-view-btn",
                },
                { value: "versions", label: t("Versions"), testId: statsTestid.experimentalVersions, className: "stats-view-btn" },
                { value: "hall", label: t("Hall of fame"), testId: statsTestid.experimentalHall, className: "stats-view-btn" },
              ]}
            />
            <div className="stats-filter-bar">
              {cardFilters()}
              {cardOptions(experimentalView === "historical" ? "running" : experimentalView)}
            </div>

            {/* R4681001: the Hall of Fame's four lists, which read no filter and open the before version. */}
            {experimentalView === "hall" ? (
              hallError || cardStatsError ? (
                <div className="stats-error">
                  <p>{t("The card statistics didn't load.")}</p>
                  <button
                    type="button"
                    className="stats-retry-btn"
                    onClick={() => {
                      if (hallError) setHallAttempt((n) => n + 1);
                      if (cardStatsError) setCardStatsAttempt((n) => n + 1);
                    }}
                  >
                    {t("Retry")}
                  </button>
                </div>
              ) : hallData === null || versionsData === null || cardStatsLoading ? (
                <Skeleton variant="table" label={t("Loading card statistics…")} />
              ) : hallEmpty ? (
                <EmptyState text={t("No cards match the selected filters.")} />
              ) : (
                <HallPanel
                  hall={hallData}
                  showRanges={showRanges}
                  band={hallData}
                  faceOf={hallFaceOf}
                  needOf={hallNeedOf}
                  onNeed={needSnapshot}
                  onOpen={openVersion}
                />
              )
            ) : experimentalError || cardStatsError ? (
              <div className="stats-error">
                <p>{t("The card statistics didn't load.")}</p>
                <button
                  type="button"
                  className="stats-retry-btn"
                  onClick={() => {
                    if (experimentalError) setVersionsAttempt((n) => n + 1);
                    if (cardStatsError) setCardStatsAttempt((n) => n + 1);
                  }}
                >
                  {t("Retry")}
                </button>
              </div>
            ) : experimentalData === null || cardStatsLoading ? (
              <Skeleton variant={view === "grid" ? "grid" : "table"} label={t("Loading card statistics…")} />
            ) : (experimentalView === "versions" ? versionsShown : running).length === 0 ? (
              <EmptyState text={t("No cards match the selected filters.")} />
            ) : experimentalView === "versions" ? (
              <RunningPanel
                kind="versions"
                rows={versionsShown}
                sort={runningSort}
                onSort={sortCreatedBy}
                view={view}
                showRanges={showRanges}
                band={experimentalData}
                typical={experimentalData.decisiveness}
                labeledBy="stats-tab-experimental"
                nameOf={shownName}
                faceOf={versionFaceOf}
                plainCardOf={plain.shown ? versionPlainOf : undefined}
                visibleThumbnail={visibleCols.thumbnail}
                visibleGames={visibleCols.games}
                visiblePlayRate={visibleCols.playRate}
                onOpen={openVersion}
                onNeed={needSnapshot}
              />
            ) : (
              <RunningPanel
                rows={running}
                sort={runningSort}
                onSort={sortCreatedBy}
                view={view}
                showRanges={showRanges}
                band={experimentalData}
                typical={experimentalData.decisiveness}
                labeledBy="stats-tab-experimental"
                nameOf={shownName}
                faceOf={gridFaceOf}
                plainCardOf={plain.shown ? plainCardOf : undefined}
                visibleThumbnail={visibleCols.thumbnail}
                visibleGames={visibleCols.games}
                visiblePlayRate={visibleCols.playRate}
                onOpen={(id, opener) => openDrillDown(id, opener)}
              />
            )}
          </>
        ),
      },
    ]}
  />

        {/* R4680001: a play version's drill-down, once the versions have loaded */}
        {versionOpen && openPlayVersion !== undefined && versionsData && (
          <VersionDrillDown
            version={openPlayVersion}
            before={versionsOfOpen[openAt - 1]}
            after={versionsOfOpen[openAt + 1]}
            band={versionsData}
            title={shownName(openPlayVersion.card, openPlayVersion.name)}
            read={readVersion}
            need={needVersion}
            onOpen={(version) => setDrillDownVersion(version.first)}
            onClose={closeDrillDown}
          />
        )}
        {versionOpen && openPlayVersion === undefined && (
          <Dialog
            role="dialog"
            testid={statsTestid.drillDownModal}
            className="stats-drilldown-modal"
            closeTestid={statsTestid.drillDownClose}
            closeLabel={t("Close card details")}
            title={drillDownDef ? readerName(drillDownDef) : cardDrillDownId}
            onClose={closeDrillDown}
            actions={[]}
          >
            <div className="stats-drilldown-body">
              {versionsData === null && !versionsError ? (
                <Skeleton variant="rows" label={t("Loading this card's details…")} />
              ) : (
                <p className="stats-empty-sub">{t("Details unavailable.")}</p>
              )}
            </div>
          </Dialog>
        )}

        {/* Card Drill-Down Modal */}
        {cardDrillDownId && !versionOpen && (
            <Dialog
              role="dialog"
              testid={statsTestid.drillDownModal}
              className="stats-drilldown-modal"
              data={drillDownRadiantFace ? { "data-radiant": "true" } : undefined}
              closeTestid={statsTestid.drillDownClose}
              closeLabel={t("Close card details")}
              title={drillDownDef ? readerName(drillDownDef) : cardDrillDownId}
              onClose={closeDrillDown}
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
                      {/* R4676001: the details column gives way to the Extended stats panel while pressed */}
                      <button
                        type="button"
                        className="stats-drilldown-action"
                        data-testid={statsTestid.extendedToggle}
                        ref={extendedToggle}
                        aria-pressed={extended !== null}
                        onClick={() => setExtended((open) => (open === null ? (extendedSections[0] ?? "choices") : null))}
                      >
                        {t("Extended stats")}
                      </button>
                      <a
                        className="stats-drilldown-action button-secondary"
                        data-testid={statsTestid.drillDownAlmanac}
                        href={paths.almanacCard(drillDownDef.id)}
                        onClick={followInApp(paths.almanacCard(drillDownDef.id))}
                      >
                        {t("View in Almanac")}
                      </a>
                      {/* R3939000: the dialog covers the header's Show card text, so it holds one of its own below View in Almanac, the same choice */}
                      <CardTextButton
                        shown={cardText.shown}
                        onToggle={cardText.toggle}
                        testid={cardTextTestid.dialog}
                        className="stats-drilldown-action"
                      />
                    </div>
                    {/* R4679001: it gives way while the Extended stats panel is open, as the details do */}
                    {!plain.shown && extended === null && drillDownTokens.length > 0 && (
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
                                <StatsDrillDownTokenFace def={token} />
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
                  {/* R4673001: on Normal or Radiant, the details read that face's games alone */}
                  {detailsFace !== DEFAULT_FACE && (
                    <p className="stats-sample-hint" data-testid={statsTestid.drillDownFaceShown}>
                      {detailsFace === "normal" ? t("Normal faces only") : t("Radiant faces only")}
                    </p>
                  )}
                  {/* R3887000: with Plain text on, the card's base and Radiant text, then every card it names */}
                  {plain.shown && drillDownDef && (
                    <>
                      <section className="stats-drilldown-section" data-testid={statsTestid.drillDownPlain}>
                        <PlainCard def={drillDownDef} />
                      </section>
                      {drillDownRefs.length > 0 && (
                        <section className="stats-drilldown-section" data-testid={statsTestid.drillDownRefs}>
                          <h3>{t("Reference cards")}</h3>
                          <ul className="plain-refs">
                            {drillDownRefs.map((named) => (
                              <li
                                key={named.id}
                                className="plain-refs__item"
                                data-card-id={named.id}
                                data-testid={statsTestid.drillDownRef}
                              >
                                <PlainCard def={named} />
                                <a
                                  className="stats-drilldown-action button-secondary"
                                  href={paths.almanacCard(named.id)}
                                  onClick={followInApp(paths.almanacCard(named.id))}
                                  aria-label={t("View {name} in the Almanac", { name: readerName(named) })}
                                >
                                  {t("View in Almanac")}
                                </a>
                              </li>
                            ))}
                          </ul>
                        </section>
                      )}
                    </>
                  )}
                  {drillDownLoading ? (
                    <Skeleton variant="rows" label={t("Loading this card's details…")} />
                  ) : drillDownError ? (
                    <div className="stats-error">
                      <p>{t("Details unavailable.")}</p>
                      <button type="button" className="stats-retry-btn" onClick={() => setDrillDownAttempt((n) => n + 1)}>
                        {t("Retry")}
                      </button>
                    </div>
                  ) : drillDownData && extended !== null ? (
                    <div className="stats-extended" data-testid={statsTestid.extendedPanel}>
                      <h3 className="stats-extended-title">{t("Extended stats")}</h3>
                      {extendedSections.length > 1 && extendedShown !== undefined && (
                        <Segmented
                          label={t("Extended stats")}
                          value={extendedShown}
                          onChange={setExtended}
                          options={extendedSections.map((section) => ({
                            value: section,
                            ...sectionOption(section),
                            className: "stats-view-btn",
                          }))}
                        />
                      )}
                      {extendedShown === "choices" && (
                        <ChoicesTables
                          variants={drillDownVariants}
                          def={drillDownDef}
                          radiant={detailsFace === "radiant"}
                          showRanges={showRanges}
                        />
                      )}
                      {/* R4707002: a card that rolled but made nothing shows its rolled tables alone. */}
                      {extendedShown === "outputs" && (drillDownOutputs.length > 0 || drillDownRolled.length === 0) && (
                        <OutputsTable outputs={drillDownOutputs} showRanges={showRanges} onOpen={openOutput} />
                      )}
                      {extendedShown === "outputs" &&
                        drillDownRolled.map((group) => <RolledTable key={group.kind} group={group} showRanges={showRanges} />)}
                      {extendedShown === "faces" && drillDownFaces && <FacesTable faces={drillDownFaces} showRanges={showRanges} />}
                      {extendedShown === undefined && <p className="stats-empty-sub">{t("Details unavailable.")}</p>}
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
