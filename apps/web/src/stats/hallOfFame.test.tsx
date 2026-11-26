// /stats' Experimental tab's Hall of Fame view (SPEC §9.11, R4681000, R4681001): the four all-time lists,
// each entry its rank, its before/after faces with the arrow, versions, dates, Adjusted, tiers and balance
// line, opening X2's drill-down on its before version.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { clearReadCache } from "../net/readCache.ts";
import { PatchSourceProvider } from "../patches/context.tsx";
import { sourceFromData, type PatchSource, type Snapshot } from "../patches/source.ts";
import StatsRoute from "../routes/stats.tsx";
import { at } from "../test/stubServer.ts";
import { statsTestid } from "./testids.ts";

const SLOW = { timeout: 10_000 } as const;

const json = (body: unknown): Response => new Response(JSON.stringify(body), { status: 200 });

/** The card changed twice: its live definition is the catalog's, and each older one a catalog of its own. */
const CARD = "core-001";
const LIVE = CATALOG[CARD] as CardDef;
const OLDEST: CardDef = { ...LIVE, name: "Ember Pup" };
const MIDDLE: CardDef = { ...LIVE, name: "Spark Pup" };

/** The catalogs the browser's history holds: each old version's at its last version. */
const SNAPSHOTS: Record<string, Snapshot> = { "v0.1.4": { [CARD]: OLDEST }, "v0.1.9": { [CARD]: MIDDLE } };

/** A play version as `GET /api/stats/versions` sends it: no games and no figure unless given. */
function playVersion(card: string, name: string, first: string, last: string, live: boolean, figures: Record<string, unknown> = {}) {
  return {
    id: `${card}~${first}`,
    card,
    name,
    first,
    last,
    live,
    patches: 1,
    games: 0,
    adjustedWinRate: null,
    adjustedLow: null,
    adjustedHigh: null,
    tier: null,
    hasEnoughGames: false,
    ...figures,
  };
}

const figures = (games: number, adjusted: number, tier: string) => ({
  games,
  adjustedWinRate: adjusted,
  adjustedLow: adjusted - 0.02,
  adjustedHigh: adjusted + 0.02,
  tier,
  hasEnoughGames: true,
});

/** A short version keeps its figures but ranks nowhere alone. */
const shortFigures = (games: number, adjusted: number, tier: string) => ({ ...figures(games, adjusted, tier), hasEnoughGames: false });

const V_OLD = playVersion(CARD, "Ember Pup", "v0.1.0", "v0.1.4", false, figures(900, 0.61, "S"));
const V_MID = playVersion(CARD, "Spark Pup", "v0.1.5", "v0.1.9", false, shortFigures(12, 0.5, "C"));
const V_LIVE = playVersion(CARD, "Spark Pup", "v0.2.0", "v0.2.0", true, figures(140, 0.55, "A"));
const V_OTHER = playVersion("core-003", "Aqua Warden", "v0.2.0", "v0.2.0", true, figures(60, 0.455, "F"));

/** Every play version, read against the versions' own band. */
const VERSIONS = {
  pilot: "merged",
  mode: "all",
  field: { center: 0, spread: 0.05, cards: 4, baseline: 0.5, drawnFloor: 20 },
  bandLow: 0.47,
  bandHigh: 0.53,
  minSample: 20,
  versions: [V_OLD, V_MID, V_LIVE, V_OTHER],
};

/** One Hall of Fame entry as `GET /api/stats/hall-of-fame` sends it. */
function hallEntry(before: unknown, after: unknown, extra: Record<string, unknown> = {}) {
  return {
    before,
    after,
    beforeDate: "2026-09-18",
    afterDate: "2026-11-25",
    balance: null,
    changesBetween: null,
    ...extra,
  };
}

/** The four lists: the S version broken beside its live one, its fall and the live one's rise, and its changes. */
const HALL = {
  pilot: "merged",
  mode: "all",
  minSample: 20,
  bandLow: 0.47,
  bandHigh: 0.53,
  broken: [hallEntry(V_OLD, V_LIVE)],
  nerfed: [hallEntry(V_OLD, V_MID, { balance: "Ember Pup scorched the ladder." })],
  buffed: [hallEntry(V_MID, V_LIVE, { balance: "Spark Pup found its spark again." })],
  changed: [hallEntry(V_OLD, V_LIVE, { changesBetween: 4 })],
};

const CARDS = {
  patch: "0.2.0",
  previousPatch: "0.1.0",
  gate: { cleared: false, liveGames: 412, minLiveGames: 1000 },
  source: "provisional",
  sourceLabel: "AI games + live games (provisional)",
  minSample: 20,
  totalGames: 1200,
  tiers: { center: 0, spread: 0.05, cards: 2, bandLow: 0.45, bandHigh: 0.55 },
  patches: [{ versions: ["0.2.0"], games: 1200, liveGames: 0 }],
  cards: [
    { id: CARD, name: "Spark Pup", cost: 1, rarity: "Common", set: "core", games: 50, winRate: 0.6, adjustedWinRate: 0.534, playRate: 0.5, hasEnoughGames: true },
    { id: "core-003", name: "Aqua Warden", cost: 2, rarity: "Rare", set: "core", games: 30, winRate: 0.5, adjustedWinRate: 0.486, playRate: 0.3, hasEnoughGames: true },
  ],
  summary: { totalGames: 1200, liveGames: 412, activePatch: "0.2.0", source: "provisional" },
};

/** Answers the cards, the play versions, the Hall of Fame, and the server's catalog for a missing version. */
function serve(catalogs: Record<string, Snapshot> = {}): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = new URL(String(input), window.location.origin);
      if (url.pathname === "/api/stats/hall-of-fame") return Promise.resolve(json(HALL));
      if (url.pathname === "/api/stats/versions") return Promise.resolve(json(VERSIONS));
      if (url.pathname === "/api/stats/cards") return Promise.resolve(json(CARDS));
      const version = /^\/api\/catalog\/(.+)$/u.exec(url.pathname)?.[1];
      const defs = version === undefined ? undefined : catalogs[decodeURIComponent(version)];
      if (defs !== undefined) return Promise.resolve(json({ version, defs }));
      return Promise.resolve(new Response("{}", { status: 404 }));
    }),
  );
}

const address = (): URLSearchParams => new URLSearchParams(window.location.search);

function renderStats(source?: PatchSource): void {
  const inner = source ?? sourceFromData({ patches: [], index: {}, snapshots: SNAPSHOTS });
  render(
    <PatchSourceProvider source={inner}>
      <StatsRoute />
    </PatchSourceProvider>,
  );
}

/** The Hall of Fame's sections by their names, and one section's entries. */
function sections(): HTMLElement[] {
  return screen.getAllByTestId(statsTestid.hallSection);
}

function entriesOf(section: HTMLElement): HTMLElement[] {
  return within(section).getAllByTestId(statsTestid.hallEntry);
}

beforeEach(() => {
  window.localStorage.clear();
  clearReadCache();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R4681001 the Experimental tab's Hall of Fame view", () => {
  it("R4681001 the Segmented gains Hall of Fame after Versions with four sections in order", async () => {
    at("/stats?tab=experimental");
    serve();
    renderStats();
    await screen.findByTestId(statsTestid.runningTable, undefined, SLOW);
    const panel = screen.getByRole("tabpanel");
    const views = within(panel).getByRole("group", { name: "Experimental views" });
    expect(within(views).getAllByRole("button").map((button) => button.textContent)).toEqual([
      "Major patch running",
      "Historical",
      "Versions",
      "Hall of fame",
    ]);

    fireEvent.click(screen.getByTestId(statsTestid.experimentalHall));
    await screen.findAllByTestId(statsTestid.hallSection, undefined, SLOW);
    await waitFor(() => expect(address().get("experimental")).toBe("hall"), SLOW);
    expect(screen.getByTestId(statsTestid.experimentalHall)).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId(statsTestid.experimentalLine)).toHaveTextContent(
      /^The most broken, nerfed, buffed and changed versions of all time$/,
    );
    expect(sections().map((section) => section.getAttribute("aria-label"))).toEqual([
      "Most broken",
      "Most nerfed",
      "Most buffed",
      "Most changed",
    ]);
  });

  it("R4681000 R4681001 an entry shows rank, both faces with the arrow, versions, dates, Adjusted, tiers and the balance line", async () => {
    at("/stats?tab=experimental&experimental=hall");
    serve();
    renderStats();
    await screen.findAllByTestId(statsTestid.hallSection, undefined, SLOW);
    expect(sections()).toHaveLength(4);

    // Most broken: the S version beside its live one, both with figures.
    const broken = entriesOf(sections()[0]!);
    expect(broken).toHaveLength(1);
    const top = broken[0]!;
    expect(top).toHaveAttribute("aria-label", "#1 Ember Pup, v0.1.0 → v0.2.0");
    const topBefore = within(top).getByTestId(statsTestid.hallBefore);
    const topAfter = within(top).getByTestId(statsTestid.hallAfter);
    await waitFor(() => expect(topBefore.querySelector(".patch-face")).toHaveTextContent("Ember Pup"), SLOW);
    await waitFor(() => expect(topAfter.querySelector(".patch-face")).toHaveTextContent(LIVE.name), SLOW);
    expect(within(top).getAllByText("→")).toHaveLength(2);
    expect(top).toHaveTextContent("v0.1.0 (2026-09-18) → v0.2.0 (2026-11-25)");
    expect(top).toHaveTextContent("61.0%");
    expect(top).toHaveTextContent("55.0%");
    expect(within(top).getAllByTestId(statsTestid.tierCell).map((cell) => cell.textContent)).toEqual(["S", "A"]);

    // Most nerfed: the fall's after side is short of games, with its balance line.
    const nerfed = entriesOf(sections()[1]!);
    const fall = nerfed[0]!;
    expect(within(fall).getByTestId(statsTestid.hallBefore).querySelector(".patch-face")).toHaveTextContent("Ember Pup");
    expect(fall).toHaveTextContent("61.0%");
    expect(fall).toHaveTextContent("not enough games");
    expect(within(fall).getByTestId(statsTestid.hallBalance)).toHaveTextContent("Ember Pup scorched the ladder.");

    // Most changed: how many changes lie between its first version and its live one.
    const changed = entriesOf(sections()[3]!);
    expect(changed[0]).toHaveTextContent("4 changes");
  });

  it("R4681001 pressing an entry opens the drill-down on its before version", async () => {
    at("/stats?tab=experimental&experimental=hall");
    serve();
    renderStats();
    await screen.findAllByTestId(statsTestid.hallSection, undefined, SLOW);
    const fall = entriesOf(sections()[1]!)[0]!;
    fireEvent.click(fall);

    const modal = await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    expect(modal).toHaveAttribute("data-version-id", `${CARD}~v0.1.0`);
    await waitFor(() => expect(address().get("version")).toBe("v0.1.0"), SLOW);
    expect(address().get("card")).toBe(CARD);
    expect(screen.getByTestId(statsTestid.versionDrillDown)).toBeInTheDocument();
  });
});
