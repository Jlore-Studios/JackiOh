// B32 (DR14 part G, #2708): the device leaderboards rank the deepest row, the most curse
// points, the top score and the fastest win, and each entry opens that run's stats. Only
// `../store.ts`'s `readRunHistory` and `contentOf` are stubbed; part D owns that module, so its types are
// imported, never copied.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { RunStats } from "@jackioh/run";

import { emptyStateTestid } from "../../empty/EmptyState.tsx";
import { formatDateTime, formatNumber } from "../../i18n/i18n.ts";
import { navTestid } from "../../routes/nav.tsx";
import type { RunRecord } from "../store.ts";
import { expectOnePageTitle } from "../../test/pageTitle.ts";
import { DungeonStats } from "./DungeonStats.tsx";
import { dungeonStatsTestid, type DungeonLeaderboard } from "./testids.ts";

const historyPlan = vi.hoisted<{ records: unknown[]; calls: number }>(() => ({ records: [], calls: 0 }));

/** #3501: the run content a record's curse ids are named from; only its curses are read. */
const content = vi.hoisted(() => ({
  curses: [
    { id: "heavy-hands", name: "Heavy Hands", text: "Enemies start each fight with 5 more health.", points: 2 },
  ],
}));

vi.mock("../store.ts", () => ({
  readRunHistory: () => {
    historyPlan.calls += 1;
    return historyPlan.records;
  },
  contentOf: () => ({ pool: [], starterAccess: [], enemies: [], passives: [], events: [], curses: content.curses }),
}));

/** Every `RunStats` field's player label, as the details table shows it. */
const STAT_LABELS: Record<keyof RunStats, string> = {
  nodesVisited: "Nodes visited",
  fightsWon: "Fights won",
  elitesWon: "Elites won",
  ghostsWon: "Ghosts won",
  miniBossesWon: "Mini-bosses won",
  bossesWon: "Bosses won",
  damageDealt: "Damage dealt",
  healingDone: "Healing done",
  cardsDrawn: "Cards drawn",
  turns: "Turns",
  damageTaken: "Damage taken",
  healed: "Healed",
  coinsEarned: "Coins earned",
  coinsSpent: "Coins spent",
  ticketsEarned: "Tickets earned",
  ticketsSpent: "Tickets spent",
  cardsKept: "Cards kept",
  cardsBought: "Cards bought",
  cardsRemoved: "Cards removed",
  cardsMadeRadiant: "Cards made Radiant",
  cardsBuffed: "Cards buffed",
  packsSkipped: "Packs skipped",
  artifactsTaken: "Relics taken",
  artifactsLeveled: "Relics leveled",
  blessingsTaken: "Blessings taken",
  eventsSeen: "Events seen",
  chestsOpened: "Chests opened",
};

function statsFixture(overrides: Partial<RunStats> = {}): RunStats {
  return {
    nodesVisited: 21,
    fightsWon: 9,
    elitesWon: 2,
    ghostsWon: 1,
    miniBossesWon: 2,
    bossesWon: 1,
    damageDealt: 342,
    healingDone: 58,
    cardsDrawn: 117,
    turns: 64,
    damageTaken: 93,
    healed: 41,
    coinsEarned: 210,
    coinsSpent: 150,
    ticketsEarned: 12,
    ticketsSpent: 7,
    cardsKept: 18,
    cardsBought: 5,
    cardsRemoved: 3,
    cardsMadeRadiant: 2,
    cardsBuffed: 4,
    packsSkipped: 1,
    artifactsTaken: 3,
    artifactsLeveled: 2,
    blessingsTaken: 2,
    eventsSeen: 4,
    chestsOpened: 2,
    ...overrides,
  };
}

function recordFixture(overrides: Partial<RunRecord> & { seed: string }): RunRecord {
  return {
    daily: false,
    mode: "sandbox",
    curses: [],
    cursePoints: 0,
    rowReached: 1,
    won: false,
    score: 0,
    embers: 0,
    timeMs: 60_000,
    endedAt: "2026-10-01T12:00:00.000Z",
    stats: statsFixture(),
    ...overrides,
  } as RunRecord;
}

/** A board's entries in rank order; empty when the board ranks none, as the fastest may. */
function entriesOf(board: DungeonLeaderboard): HTMLElement[] {
  return within(screen.getByTestId(dungeonStatsTestid.board(board))).queryAllByRole("button");
}

/** One ranked entry; a board ranking fewer fails the test rather than clicking nothing. */
function entryAt(board: DungeonLeaderboard, at: number): HTMLElement {
  const found = entriesOf(board)[at];
  if (found === undefined) throw new Error(`board ${board} ranks no entry ${String(at)}`);
  return found;
}

/** A board's ranked seeds in order. */
function seedsOf(board: DungeonLeaderboard): (string | null)[] {
  return entriesOf(board).map((entry) => entry.textContent?.match(/[A-Z]{6}/)?.[0] ?? null);
}

function endedOn(record: RunRecord): string {
  return formatDateTime(new Date(record.endedAt), { dateStyle: "medium" });
}

/** The opened run's complete details: metadata, time, curses and all 27 labeled stat values. */
function expectRunDetails(record: RunRecord, time: string): void {
  const dialog = screen.getByRole("dialog", { name: `Run ${record.seed}` });
  expect(dialog).toHaveAttribute("aria-modal", "true");
  // R3221000: it closes by the dialog's ✕, which holds the focus from the start.
  const close = within(dialog).getByTestId(dungeonStatsTestid.dialogClose);
  expect(close).toHaveAccessibleName("Close");
  expect(close).toHaveFocus();
  const meta = within(dialog).getByTestId(dungeonStatsTestid.detailMeta);
  for (const text of [
    record.seed,
    record.mode === "sandbox" ? "Sandbox" : "Campaign",
    endedOn(record),
    record.won ? "Won" : "Lost",
    record.daily ? "Daily run" : "Not a daily run",
    formatNumber(record.rowReached),
    formatNumber(record.cursePoints),
    formatNumber(record.score),
    formatNumber(record.embers),
  ]) {
    expect(meta).toHaveTextContent(text);
  }
  expect(within(dialog).getByTestId(dungeonStatsTestid.detailTime)).toHaveTextContent(time);
  const curses = within(dialog).getByTestId(dungeonStatsTestid.detailCurses);
  if (record.curses.length === 0) {
    expect(curses).toHaveTextContent("No curses");
  } else {
    for (const curse of record.curses) expect(curses).toHaveTextContent(curse);
  }
  for (const [field, label] of Object.entries(STAT_LABELS)) {
    const key = field as keyof RunStats;
    const value = within(dialog).getByTestId(dungeonStatsTestid.stat(key));
    expect(value).toHaveTextContent(formatNumber(record.stats[key]));
    expect(value.parentElement).toHaveTextContent(label);
  }
}

beforeEach(() => {
  historyPlan.records = [];
  historyPlan.calls = 0;
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("B32 the device leaderboards and each run's stats", () => {
  const runA = (): RunRecord =>
    recordFixture({
      seed: "AAAAAA",
      daily: false,
      curses: ["ember-oath"],
      cursePoints: 4,
      rowReached: 12,
      won: true,
      score: 900,
      embers: 45,
      timeMs: 3_723_000,
      endedAt: "2026-10-04T18:30:00.000Z",
    });
  const runB = (): RunRecord =>
    recordFixture({
      seed: "BBBBBB",
      daily: true,
      curses: ["ember-oath", "iron-rations"],
      cursePoints: 9,
      rowReached: 8,
      won: true,
      score: 1500,
      embers: 80,
      timeMs: 300_000,
      endedAt: "2026-10-05T09:15:00.000Z",
      stats: statsFixture({ damageDealt: 999, turns: 37 }),
    });
  const runC = (): RunRecord =>
    recordFixture({
      seed: "CCCCCC",
      daily: false,
      curses: [],
      cursePoints: 2,
      rowReached: 15,
      won: false,
      score: 400,
      embers: 12,
      timeMs: 120_000,
      endedAt: "2026-10-06T21:45:00.000Z",
    });

  it("R3223000 the screen has one visible h1, the PageTitle, named as its tab is, with and without runs", () => {
    render(<DungeonStats />);
    expectOnePageTitle("Dungeon run stats");
    cleanup();
    historyPlan.records = [runA()];
    render(<DungeonStats />);
    expectOnePageTitle("Dungeon run stats");
  });

  it("B32 ranks the four device leaderboards and every entry opens the run's complete stats", () => {
    const [a, b, c] = [runA(), runB(), runC()];
    historyPlan.records = [a, b, c];
    render(<DungeonStats />);

    expect(seedsOf("deepest")).toEqual(["CCCCCC", "AAAAAA", "BBBBBB"]);
    expect(seedsOf("curses")).toEqual(["BBBBBB", "AAAAAA", "CCCCCC"]);
    expect(seedsOf("score")).toEqual(["BBBBBB", "AAAAAA", "CCCCCC"]);
    // A loss, however quick, never enters the fastest board.
    expect(seedsOf("fastest")).toEqual(["BBBBBB", "AAAAAA"]);

    // Every row names its seed, mode, date and daily status.
    for (const entry of entriesOf("deepest")) {
      expect(entry).toHaveTextContent("Sandbox");
    }
    expect(entryAt("deepest", 0)).toHaveTextContent("Not a daily run");
    expect(entryAt("deepest", 0)).not.toHaveTextContent("Daily run");
    expect(entryAt("deepest", 1)).toHaveTextContent("Not a daily run");
    expect(entryAt("deepest", 2)).toHaveTextContent("Daily run");
    expect(entryAt("deepest", 0)).toHaveTextContent(endedOn(c));
    expect(entryAt("curses", 0)).toHaveTextContent("BBBBBB");
    expect(entryAt("curses", 0)).toHaveTextContent("Daily run");
    expect(entryAt("curses", 0)).toHaveTextContent(endedOn(b));
    expect(entryAt("fastest", 0)).toHaveTextContent("5:00");
    expect(entryAt("fastest", 1)).toHaveTextContent("1:02:03");

    // The deepest run: a loss with no curses.
    fireEvent.click(entryAt("deepest", 0));
    expectRunDetails(c, "2:00");
    fireEvent.click(screen.getByTestId(dungeonStatsTestid.dialogClose));
    expect(screen.queryByTestId(dungeonStatsTestid.dialog)).toBeNull();

    // The most-cursed run: a daily win, with its curses.
    fireEvent.click(entryAt("curses", 0));
    expectRunDetails(b, "5:00");
    fireEvent.click(screen.getByTestId(dungeonStatsTestid.dialogClose));
    expect(screen.queryByTestId(dungeonStatsTestid.dialog)).toBeNull();

    // The same run opens off every board that ranks it.
    fireEvent.click(entryAt("score", 0));
    expectRunDetails(b, "5:00");
    fireEvent.click(screen.getByTestId(dungeonStatsTestid.dialogClose));

    // The hour-long win, closed with Escape, hands the focus back to its entry.
    const opener = entryAt("fastest", 1);
    fireEvent.click(opener);
    expectRunDetails(a, "1:02:03");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(dungeonStatsTestid.dialog)).toBeNull();
    expect(opener).toHaveFocus();
  });

  it("#3501 a record's curses show by name from the content, and an id the content lacks shows as the id", () => {
    historyPlan.records = [recordFixture({ seed: "EEEEEE", curses: ["heavy-hands", "gone-curse"] })];
    render(<DungeonStats />);

    fireEvent.click(entryAt("deepest", 0));
    const curses = within(screen.getByTestId(dungeonStatsTestid.detailCurses));
    expect(curses.getByText("Heavy Hands")).toBeInTheDocument();
    expect(curses.getByText("gone-curse")).toBeInTheDocument();
    expect(curses.queryByText("heavy-hands")).not.toBeInTheDocument();
  });

  it("B32 a fastest board with matching runs but no wins says so", () => {
    historyPlan.records = [recordFixture({ seed: "DDDDDD", won: false, rowReached: 6, timeMs: 60_000 })];
    render(<DungeonStats />);

    expect(seedsOf("deepest")).toEqual(["DDDDDD"]);
    expect(screen.queryByTestId(dungeonStatsTestid.board("fastest"))).not.toBeNull();
    expect(entriesOf("fastest")).toHaveLength(0);
    expect(screen.getByTestId(dungeonStatsTestid.noWins)).toHaveTextContent("No winning runs");
  });
});

describe("the mode and daily filters", () => {
  const sandboxRun = (): RunRecord => recordFixture({ seed: "AAAAAA", mode: "sandbox", daily: false });
  const sandboxDaily = (): RunRecord =>
    recordFixture({ seed: "BBBBBB", mode: "sandbox", daily: true, endedAt: "2026-10-02T12:00:00.000Z" });
  const campaignRun = (): RunRecord =>
    recordFixture({ seed: "CCCCCC", mode: "campaign", daily: false, endedAt: "2026-10-03T12:00:00.000Z" });

  function showCampaign(): void {
    fireEvent.change(screen.getByTestId(dungeonStatsTestid.modeFilter), { target: { value: "campaign" } });
  }

  function showDailyOnly(): void {
    fireEvent.click(screen.getByTestId(dungeonStatsTestid.dailyFilter));
  }

  it("filters the boards by Sandbox or Campaign and by daily runs only", () => {
    historyPlan.records = [sandboxRun(), sandboxDaily(), campaignRun()];
    render(<DungeonStats />);

    // Sandbox is the default.
    expect(seedsOf("deepest")).toEqual(["AAAAAA", "BBBBBB"]);

    showCampaign();
    expect(seedsOf("deepest")).toEqual(["CCCCCC"]);
    expect(entryAt("deepest", 0)).toHaveTextContent("Campaign");

    showDailyOnly();
    expect(screen.getByTestId(dungeonStatsTestid.noMatches)).toHaveTextContent("No runs match these filters");
    expect(screen.queryByTestId(dungeonStatsTestid.board("deepest"))).toBeNull();

    // Daily-only Sandbox holds the one daily run.
    fireEvent.change(screen.getByTestId(dungeonStatsTestid.modeFilter), { target: { value: "sandbox" } });
    expect(seedsOf("deepest")).toEqual(["BBBBBB"]);
    expect(entryAt("deepest", 0)).toHaveTextContent("Daily run");
  });
});

describe("a device with no runs yet", () => {
  it("shows a device-empty state with ways back to Dungeon run", () => {
    historyPlan.records = [];
    render(<DungeonStats />);

    expect(screen.getByTestId(dungeonStatsTestid.noHistory)).toHaveTextContent("No finished runs");
    expect(screen.getByTestId(emptyStateTestid.action)).toHaveAttribute("href", "/dungeon");
    expect(screen.getByTestId(navTestid.back)).toHaveAttribute("data-route", "/dungeon");
    expect(historyPlan.calls).toBe(1);
  });
});
