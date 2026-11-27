// /stats' Experimental tab's Versions view and a play version's drill-down (SPEC §9.11, R4680000, R4680001): every
// play version of each card the Cards tab lists as its own row, drawn and opened as its own catalog printed it.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { clearReadCache } from "../net/readCache.ts";
import { PatchSourceProvider } from "../patches/context.tsx";
import { patchTestid } from "../patches/testids.ts";
import { sourceFromData, type PatchSource, type Snapshot } from "../patches/source.ts";
import StatsRoute from "../routes/stats.tsx";
import { at } from "../test/stubServer.ts";
import { statsTestid } from "./testids.ts";

const SLOW = { timeout: 10_000 } as const;

const json = (body: unknown): Response => new Response(JSON.stringify(body), { status: 200 });

/** The card changed twice: its live definition is the catalog's, and each older one a catalog of its own. */
const CARD = "core-001";
const LIVE = CATALOG[CARD] as CardDef;
const LIVE_COST = typeof LIVE.cost === "number" ? LIVE.cost : 0;
const OLDEST: CardDef = { ...LIVE, name: "Ember Pup", cost: LIVE_COST + 2 };
const MIDDLE: CardDef = { ...LIVE, name: "Spark Pup", cost: LIVE_COST + 1 };

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
    iteration: "v1",
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

/** Every play version, read against the versions' own band. */
const VERSIONS = {
  pilot: "merged",
  mode: "all",
  field: { center: 0, spread: 0.05, cards: 4, baseline: 0.5, drawnFloor: 20 },
  bandLow: 0.47,
  bandHigh: 0.53,
  minSample: 20,
  versions: [
    playVersion(CARD, "Ember Pup", "v0.1.0", "v0.1.4", false, figures(900, 0.61, "S")),
    // R5035000: each renamed and recosted, so each a new number.
    playVersion(CARD, "Spark Pup", "v0.1.5", "v0.1.9", false, { ...figures(300, 0.52, "B"), patches: 2, iteration: "v2" }),
    playVersion(CARD, "Spark Pup", "v0.2.0", "v0.2.0", true, { ...figures(140, 0.512, "B"), patches: 3, iteration: "v3" }),
    playVersion("core-003", "Aqua Warden", "v0.2.0", "v0.2.0", true, figures(60, 0.455, "F")),
    // A card the Cards tab does not list: none of its versions is a row.
    playVersion("core-099", "Unlisted", "v0.0.1", "v0.0.1", false, figures(50, 0.5, "C")),
  ],
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

/** Answers the cards, the play versions and, for a version the browser's history lacks, the server's catalog. */
function serve(catalogs: Record<string, Snapshot> = {}): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = new URL(String(input), window.location.origin);
      if (url.pathname === "/api/stats/versions") return Promise.resolve(json(VERSIONS));
      if (url.pathname === "/api/stats/cards") return Promise.resolve(json(CARDS));
      const version = /^\/api\/catalog\/(.+)$/u.exec(url.pathname)?.[1];
      const defs = version === undefined ? undefined : catalogs[decodeURIComponent(version)];
      if (defs !== undefined) return Promise.resolve(json({ version, defs }));
      return Promise.resolve(new Response("{}", { status: 404 }));
    }),
  );
}

function requested(pathname: string): URL[] {
  return vi
    .mocked(fetch)
    .mock.calls.map(([input]) => new URL(String(input), window.location.origin))
    .filter((url) => url.pathname === pathname);
}

/** A patch source over `snapshots` that notes every version it is asked for. */
function notingSource(snapshots: Record<string, Snapshot> = SNAPSHOTS): { source: PatchSource; asked: string[] } {
  const asked: string[] = [];
  const inner = sourceFromData({ patches: [], index: {}, snapshots });
  return {
    asked,
    source: {
      ...inner,
      snapshot: (version) => {
        asked.push(version);
        return inner.snapshot(version);
      },
    },
  };
}

function renderStats(source: PatchSource = notingSource().source): void {
  render(
    <PatchSourceProvider source={source}>
      <StatsRoute />
    </PatchSourceProvider>,
  );
}

/** A Versions table's rows as their play versions, and each row's cells. */
function rowsOf(table: HTMLElement): { ids: (string | null)[]; cells: string[][] } {
  const rows = [...table.querySelectorAll("tbody tr")];
  return {
    ids: rows.map((row) => row.getAttribute("data-version-id")),
    cells: rows.map((row) => [...row.querySelectorAll("td")].map((td) => td.textContent ?? "")),
  };
}

const address = (): URLSearchParams => new URLSearchParams(window.location.search);
const row = (id: string): HTMLElement => screen.getByTestId(statsTestid.versionsTable).querySelector<HTMLElement>(`tbody tr[data-version-id="${id}"]`)!;

beforeEach(() => {
  window.localStorage.clear();
  clearReadCache();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R4680000 the Experimental tab's Versions view", () => {
  it("R4680000 the Segmented's Versions lists every play version of each card the Cards tab lists, a card changed twice as three rows, each with its span, its live one marked, and its Card as its own catalog printed it", async () => {
    at("/stats?tab=experimental");
    serve();
    const { source, asked } = notingSource();
    renderStats(source);
    await screen.findByTestId(statsTestid.runningTable, undefined, SLOW);
    expect(screen.getByTestId(statsTestid.experimentalLine)).toHaveTextContent(
      /^Every card's current version, over this major's patches since it last changed$/,
    );

    fireEvent.click(screen.getByTestId(statsTestid.experimentalVersions));
    const table = await screen.findByTestId(statsTestid.versionsTable, undefined, SLOW);
    await waitFor(() => expect(address().get("experimental")).toBe("versions"), SLOW);
    expect(screen.getByTestId(statsTestid.experimentalVersions)).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId(statsTestid.experimentalLine)).toHaveTextContent(/^Every version of every card, each over the patches it was played on$/);
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Card",
      "Name",
      "Games",
      "Tier",
      "Adjusted ↓",
      "Decisiveness",
      "Win rate",
      "Play rate",
      "Since",
      "Iteration",]);

    // Adjusted best first, on one scale across the versions; the old ones' cost is their own catalog's once it loads.
    await waitFor(() => expect(rowsOf(table).cells[0]?.[0]).toBe(String(LIVE_COST + 2)), SLOW);
    expect(rowsOf(table)).toEqual({
      ids: [`${CARD}~v0.1.0`, `${CARD}~v0.1.5`, `${CARD}~v0.2.0`, "core-003~v0.2.0"],
      cells: [
        [String(LIVE_COST + 2), "Ember Pup", "900", "S", "61.0%", "—", "—", "—", "v0.1.0 – v0.1.4 across 1 patch", "v1"],
        [String(LIVE_COST + 1), "Spark Pup", "300", "B", "52.0%", "—", "—", "—", "v0.1.5 – v0.1.9 across 2 patches", "v2"],
        ["1", "Spark Pup", "140", "B", "51.2%", "—", "—", "—", "v0.2.0 – live across 3 patches", "v3"],
        ["2", "Aqua Warden", "60", "F", "45.5%", "—", "—", "—", "v0.2.0 – live across 1 patch", "v1"],
      ],
    });
    // The live ones are marked, read the Cards tab's cost and rarity, and load no catalog; the old ones wear their own rarity.
    expect(row(`${CARD}~v0.2.0`)).toHaveAttribute("data-live", "true");
    expect(within(row(`${CARD}~v0.2.0`)).getByText("live")).toHaveClass("stats-version-live");
    expect(row(`${CARD}~v0.1.5`)).not.toHaveAttribute("data-live");
    expect(row(`${CARD}~v0.1.5`).querySelector(".stats-card-thumb-badge")).toHaveAttribute("data-rarity", MIDDLE.rarity);
    expect(row("core-003~v0.2.0").querySelector(".stats-card-thumb-badge")).toHaveAttribute("data-rarity", "Rare");
    expect([...asked].sort()).toEqual(["v0.1.4", "v0.1.9"]);
    expect(row(`${CARD}~v0.1.5`)).toHaveAccessibleName("Spark Pup, v0.1.5 – v0.1.9");

    // The Grid draws each version's face from its own definition.
    fireEvent.click(screen.getByTestId(statsTestid.viewGrid));
    const grid = await screen.findByTestId(statsTestid.versionsGrid, undefined, SLOW);
    const oldest = grid.querySelector<HTMLElement>(`li[data-version-id="${CARD}~v0.1.0"]`)!;
    expect(within(oldest).getByRole("button", { name: "Ember Pup, v0.1.0 – v0.1.4" })).toHaveTextContent("Ember Pup");
    expect(within(oldest).getByTestId(statsTestid.versionsSpan)).toHaveTextContent("v0.1.0 – v0.1.4 across 1 patch");
  });

  it("R4680000 Past versions only, one pressed button kept in the address, leaves the live versions out", async () => {
    at("/stats?tab=experimental&experimental=versions");
    serve();
    renderStats();
    const table = await screen.findByTestId(statsTestid.versionsTable, undefined, SLOW);
    const button = screen.getByTestId(statsTestid.versionsPastOnly);
    expect(button).toHaveTextContent(/^Past versions only$/);
    expect(button).toHaveAttribute("aria-pressed", "false");
    expect(button).toHaveClass("segmented__item");
    expect(rowsOf(table).ids).toHaveLength(4);

    fireEvent.click(button);
    expect(button).toHaveAttribute("aria-pressed", "true");
    expect(rowsOf(table).ids).toEqual([`${CARD}~v0.1.0`, `${CARD}~v0.1.5`]);
    await waitFor(() => expect(address().get("past")).toBe("1"), SLOW);

    // Running has no such button; a link keeps it.
    fireEvent.click(screen.getByTestId(statsTestid.experimentalRunning));
    await screen.findByTestId(statsTestid.runningTable, undefined, SLOW);
    expect(screen.queryByTestId(statsTestid.versionsPastOnly)).toBeNull();
    await waitFor(() => expect(address().has("experimental")).toBe(false), SLOW);
    cleanup();
    at("/stats?tab=experimental&experimental=versions&past=1");
    renderStats();
    const again = await screen.findByTestId(statsTestid.versionsTable, undefined, SLOW);
    expect(rowsOf(again).ids).toEqual([`${CARD}~v0.1.0`, `${CARD}~v0.1.5`]);
  });

  it("R5035000 Versions shows each play version's Iteration right of its span in the table and grid, and Past versions only moves no number", async () => {
    at("/stats?tab=experimental&experimental=versions");
    serve();
    renderStats();
    const table = await screen.findByTestId(statsTestid.versionsTable, undefined, SLOW);
    const iterations = (root: HTMLElement): (string | null)[] =>
      within(root)
        .getAllByTestId(statsTestid.versionsIteration)
        .map((cell) => cell.textContent);
    expect(
      within(table)
        .getAllByRole("columnheader")
        .map((th) => th.textContent)
        .slice(-2),
    ).toEqual(["Since", "Iteration"]);
    expect(iterations(table)).toEqual(["v1", "v2", "v3", "v1"]);
    expect(within(row(`${CARD}~v0.1.5`)).getByTestId(statsTestid.versionsIteration)).toHaveTextContent(/^v2$/);

    // Past versions only leaves the live rows out; the old ones keep their numbers.
    fireEvent.click(screen.getByTestId(statsTestid.versionsPastOnly));
    expect(iterations(table)).toEqual(["v1", "v2"]);
    fireEvent.click(screen.getByTestId(statsTestid.viewGrid));
    const grid = await screen.findByTestId(statsTestid.versionsGrid, undefined, SLOW);
    expect(iterations(grid)).toEqual(["v1", "v2"]);
  });

  it("R4680000 the rows load only the catalogs of the rows on screen, and a version the browser's history lacks from GET /api/catalog/:version", async () => {
    // jsdom has no IntersectionObserver: one stands in, fired by hand.
    const observers: { callback: IntersectionObserverCallback; targets: Set<Element> }[] = [];
    vi.stubGlobal(
      "IntersectionObserver",
      class {
        readonly state: { callback: IntersectionObserverCallback; targets: Set<Element> };
        constructor(callback: IntersectionObserverCallback) {
          this.state = { callback, targets: new Set() };
          observers.push(this.state);
        }
        observe(target: Element): void {
          this.state.targets.add(target);
        }
        unobserve(target: Element): void {
          this.state.targets.delete(target);
        }
        disconnect(): void {
          this.state.targets.clear();
        }
      },
    );
    const onScreen = (element: Element): void => {
      const watching = observers.find((observer) => observer.targets.has(element));
      expect(watching, "the row is watched").toBeDefined();
      act(() => {
        watching!.callback([{ target: element, isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver);
      });
    };

    at("/stats?tab=experimental&experimental=versions");
    // The browser's history holds the middle version's catalog only; the server has the oldest one's.
    serve({ "v0.1.4": { [CARD]: OLDEST } });
    const { source, asked } = notingSource({ "v0.1.9": { [CARD]: MIDDLE } });
    renderStats(source);
    const table = await screen.findByTestId(statsTestid.versionsTable, undefined, SLOW);
    // The old rows wait for their catalogs, and nothing is asked before a row nears the screen.
    expect(row(`${CARD}~v0.1.5`)).toHaveAttribute("data-needs", "v0.1.9");
    expect(row(`${CARD}~v0.1.0`)).toHaveAttribute("data-needs", "v0.1.4");
    expect(row(`${CARD}~v0.2.0`)).not.toHaveAttribute("data-needs");
    expect(row(`${CARD}~v0.1.5`).querySelector(".stats-card-thumb-badge")).toHaveTextContent(/^$/);
    expect(asked).toEqual([]);

    onScreen(row(`${CARD}~v0.1.5`));
    await waitFor(() => expect(rowsOf(table).cells[1]?.[0]).toBe(String(LIVE_COST + 1)), SLOW);
    expect(asked).toEqual(["v0.1.9"]);
    expect(requested("/api/catalog/v0.1.9")).toEqual([]);
    expect(row(`${CARD}~v0.1.5`)).not.toHaveAttribute("data-needs");
    expect(row(`${CARD}~v0.1.0`)).toHaveAttribute("data-needs", "v0.1.4");

    onScreen(row(`${CARD}~v0.1.0`));
    await waitFor(() => expect(rowsOf(table).cells[0]?.[0]).toBe(String(LIVE_COST + 2)), SLOW);
    expect(asked).toEqual(["v0.1.9", "v0.1.4"]);
    expect(requested("/api/catalog/v0.1.4")).toHaveLength(1);
  });
});

describe("R4680001 a play version's drill-down", () => {
  it("R4680001 a row opens its version: its faces from its own catalog, its figures, and the versions before and after it, each with its face and the change between them", async () => {
    at("/stats?tab=experimental&experimental=versions");
    serve();
    renderStats();
    await screen.findByTestId(statsTestid.versionsTable, undefined, SLOW);
    const opener = row(`${CARD}~v0.1.5`);
    fireEvent.click(opener);

    const modal = await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    expect(modal).toHaveAttribute("data-version-id", `${CARD}~v0.1.5`);
    expect(within(modal).getByRole("heading", { level: 2 })).toHaveTextContent("Spark Pup");
    await waitFor(() => expect(address().get("version")).toBe("v0.1.5"), SLOW);
    expect(address().get("card")).toBe(CARD);
    // No patch's details are asked of the server.
    expect(vi.mocked(fetch).mock.calls.some(([input]) => String(input).includes(`/api/stats/cards/${CARD}`))).toBe(false);

    // Its base and Radiant faces, as its own catalog printed them.
    const faces = await within(modal).findByTestId(statsTestid.versionFaces, undefined, SLOW);
    expect([...faces.querySelectorAll(".patch-face")].map((face) => face.getAttribute("data-face"))).toEqual(["base", "radiant"]);
    expect(within(faces).getAllByText("Spark Pup")).toHaveLength(2);
    expect(within(faces).queryByText(LIVE.name)).toBeNull();

    // Its figures, on the versions' band.
    const figuresList = within(modal).getByTestId(statsTestid.versionFigures);
    expect(within(figuresList).getByTestId(statsTestid.tierCell)).toHaveTextContent("B");
    expect(within(figuresList).getByTestId(statsTestid.adjustedCell)).toHaveTextContent("52.0%");
    expect(within(figuresList).getByTestId(statsTestid.adjustedRange)).toHaveTextContent("50.0–54.0%");
    expect(within(figuresList).getByTestId(statsTestid.versionsSpan)).toHaveTextContent("v0.1.5 – v0.1.9 across 2 patches");
    expect(figuresList).toHaveTextContent("300");

    // The version before: its face and what changed from it to this one.
    const before = within(modal).getByTestId(statsTestid.versionBefore);
    expect(within(before).getByTestId(statsTestid.versionOpen)).toHaveTextContent(/^v0\.1\.0 – v0\.1\.4$/);
    expect(before.querySelector(".patch-face")).toHaveTextContent("Ember Pup");
    const was = within(before).getAllByTestId(patchTestid.change);
    expect(was.map((change) => change.getAttribute("data-field"))).toEqual(["name", "cost"]);
    expect(was[1]).toHaveTextContent(`(${String(LIVE_COST + 2)}) Cost`);
    expect(was[1]).toHaveTextContent(`(${String(LIVE_COST + 1)}) Cost`);

    // The version after, the live one: its face and what changed from this one to it.
    const after = within(modal).getByTestId(statsTestid.versionAfter);
    expect(within(after).getByTestId(statsTestid.versionOpen)).toHaveTextContent(/^v0\.2\.0 – live$/);
    expect(after.querySelector(".patch-face")).toHaveTextContent(LIVE.name);
    expect(within(after).getAllByTestId(patchTestid.change).map((change) => change.getAttribute("data-field"))).toEqual(["name", "cost"]);

    // A neighbour opens its own drill-down; the first version has none before it.
    fireEvent.click(within(before).getByTestId(statsTestid.versionOpen));
    await waitFor(() => expect(address().get("version")).toBe("v0.1.0"), SLOW);
    expect(screen.getByTestId(statsTestid.drillDownModal)).toHaveAttribute("data-version-id", `${CARD}~v0.1.0`);
    expect(within(screen.getByTestId(statsTestid.versionBefore)).getByText("None: this is its first version.")).toBeInTheDocument();
    expect(within(screen.getByTestId(statsTestid.versionAfter)).getByTestId(statsTestid.versionOpen)).toHaveTextContent(/^v0\.1\.5 – v0\.1\.9$/);

    // Closing it forgets the version and gives the focus back to the row.
    fireEvent.click(screen.getByTestId(statsTestid.drillDownClose));
    await waitFor(() => expect(screen.queryByTestId(statsTestid.drillDownModal)).toBeNull(), SLOW);
    await waitFor(() => expect(address().has("version")).toBe(false), SLOW);
    expect(address().has("card")).toBe(false);
    expect(document.activeElement).toBe(opener);
  });

  it("R4680001 the address names the version, so a link opens its drill-down, from any tab; the live one's after is none", async () => {
    at(`/stats?card=${CARD}&version=v0.2.0`);
    serve();
    renderStats();
    // Opened at once, behind a skeleton until the versions load.
    await waitFor(() => expect(screen.getByTestId(statsTestid.drillDownModal)).toHaveAttribute("data-version-id", `${CARD}~v0.2.0`), SLOW);
    const modal = screen.getByTestId(statsTestid.drillDownModal);
    expect(requested("/api/stats/versions")).toHaveLength(1);
    expect(within(modal).getByTestId(statsTestid.versionAfter)).toHaveTextContent("None: this is its live version.");
    expect(await within(modal).findByTestId(statsTestid.versionFaces, undefined, SLOW)).toHaveTextContent(LIVE.name);
    cleanup();

    // A version the card never had: its details are unavailable.
    at(`/stats?card=${CARD}&version=v9.9.9`);
    renderStats();
    const none = await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    expect(await within(none).findByText("Details unavailable.", undefined, SLOW)).toBeInTheDocument();
  });
});
