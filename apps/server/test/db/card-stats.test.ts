/**
 * `src/db/card-stats.ts` (stats:cards) and `src/db/import-dev-records.ts` (stats:import), SPEC §9.11,
 * R377 and R378, against the in-memory store: what the options read, that a development run is read
 * only when asked for, and that a file can add development records and nothing else. The store's own
 * filtering is `test/db/contract.ts`'s, against both stores; the arithmetic is packages/shared's.
 */

import { resolve } from "node:path";

import { describe, expect, it } from "vitest";
import type { GameRecord } from "@jackioh/shared";

import { cardStatsReport, parseCardStatsArgs, renderCardStats, type CardStatsOptions } from "../../src/db/card-stats";
import { importDevRecords, runFilePath } from "../../src/db/import-dev-records";
import { createMemoryStore } from "../fakes/store";

function gameRecord(id: string, partial: Partial<GameRecord> = {}): GameRecord {
  return {
    id,
    source: "live",
    mode: "bo1",
    patch: "v0.2.5",
    pilots: { p1: "human", p2: "human" },
    game: {
      first: "p1",
      winner: "p1",
      reason: "hero-death",
      turns: 9,
      seats: {
        p1: { deck: ["core-001"], opening: ["core-001"], drawn: [], played: ["core-001"] },
        p2: { deck: ["core-002"], opening: [], drawn: ["core-002"], played: [] },
      },
    },
    ...partial,
  };
}

const dev = (id: string, partial: Partial<GameRecord> = {}): GameRecord =>
  gameRecord(id, { source: "dev", mode: "random", pilots: { p1: "ai", p2: "ai" }, ...partial });

const lines = (records: readonly GameRecord[]): string => records.map((record) => JSON.stringify(record)).join("\n");

describe("stats:cards's options", () => {
  it("R378 reads live games of every mode, patch and pilot when it names nothing", () => {
    expect(parseCardStatsArgs([])).toEqual({
      filter: { source: "live", mode: null, patch: null, pilot: "unified" },
      card: null,
      json: false,
    });
  });

  it("R377 takes any combination of match type, patch and pilot, and one card", () => {
    expect(
      parseCardStatsArgs(["--source=all", "--mode=random", "--patch=v0.2.5", "--pilot=ai", "--card=core-001", "--json"]),
    ).toEqual({
      filter: { source: "all", mode: "random", patch: "v0.2.5", pilot: "ai" },
      card: "core-001",
      json: true,
    });
    expect(parseCardStatsArgs(["--pilot=human"]).filter.pilot).toBe("human");
    expect(parseCardStatsArgs(["--source=dev"]).filter.source).toBe("dev");
  });

  it("refuses an option or a value it does not know rather than reading something else", () => {
    expect(() => parseCardStatsArgs(["--source=practice"])).toThrow(/--source must be one of live, dev, all/);
    expect(() => parseCardStatsArgs(["--mode=bo5"])).toThrow(/--mode must be one of bo1, bo3, random/);
    expect(() => parseCardStatsArgs(["--pilot=robot"])).toThrow(/--pilot/);
    expect(() => parseCardStatsArgs(["--deck=1"])).toThrow(/Unrecognised option --deck/);
    expect(() => parseCardStatsArgs(["--patch"])).toThrow(/Unrecognised argument/);
    expect(() => parseCardStatsArgs(["v0.2.5"])).toThrow(/Unrecognised argument/);
  });
});

describe("stats:cards's report", () => {
  async function seeded(): Promise<ReturnType<typeof createMemoryStore>> {
    const store = createMemoryStore();
    for (const record of [
      gameRecord("live-1"),
      gameRecord("live-2", { patch: "v0.1.1" }),
      dev("dev:1"),
      dev("dev:2", { game: { ...gameRecord("x").game, winner: "p2" } }),
    ]) {
      await store.gameRecords.insert(record);
    }
    return store;
  }

  const options = (argv: string[]): CardStatsOptions => parseCardStatsArgs(argv);

  it("R378 counts a development run only when it is asked for by name", async () => {
    const store = await seeded();
    expect((await cardStatsReport(store, options([]))).games).toBe(2);
    expect((await cardStatsReport(store, options(["--source=dev"]))).games).toBe(2);
    expect((await cardStatsReport(store, options(["--source=all"]))).games).toBe(4);
  });

  it("R378 compares a pre-release run with the live games of the same patch, one query each", async () => {
    const store = await seeded();
    const live = await cardStatsReport(store, options(["--patch=v0.2.5"]));
    const prerelease = await cardStatsReport(store, options(["--patch=v0.2.5", "--source=dev"]));
    expect(live.games).toBe(1);
    expect(prerelease.games).toBe(2);
    const first = (report: typeof live) => report.cards.find((stats) => stats.card === "core-001")?.inDeck;
    expect(first(live)).toEqual({ games: 1, wins: 1, draws: 0 });
    expect(first(prerelease)).toEqual({ games: 2, wins: 1, draws: 0 });
  });

  it("R377 prints every breakdown of one card, named, with its games beside each rate", async () => {
    const store = await seeded();
    const one = options(["--card=core-001", "--patch=v0.2.5"]);
    const report = await cardStatsReport(store, one);
    expect(report.cards.map((stats) => stats.card)).toEqual(["core-001"]);

    const text = renderCardStats(report, one, (card) => (card === "core-001" ? "Big D Fender" : undefined));
    expect(text).toContain("Card win rates: live games, every mode, patch v0.2.5, human and AI pilots (unified).");
    const row = text.split("\n").find((line) => line.startsWith("core-001 Big D Fender"));
    expect(row?.split(/\s{2,}/).slice(1)).toEqual(["100.0% (1)", "100.0% (1)", "100.0% (1)", "— (0)", "100.0% (1)", "— (0)", "—"]);

    const absent = options(["--card=core-099"]);
    expect(renderCardStats(await cardStatsReport(store, absent), absent, () => undefined)).toContain(
      "No deck the filter counts held core-099.",
    );
    const json = options(["--json"]);
    expect(JSON.parse(renderCardStats(report, json, () => undefined))).toEqual(report);
  });
});

describe("stats:import", () => {
  it("R378 adds a development run's records, and the same run twice adds nothing more", async () => {
    const store = createMemoryStore();
    const run = [dev("dev:run:1"), dev("dev:run:2")];
    expect(await importDevRecords(store, `${lines(run)}\n`)).toEqual({ read: 2, written: 2, skipped: 0 });
    expect(await importDevRecords(store, lines(run))).toEqual({ read: 2, written: 0, skipped: 2 });
    expect(store.tables.gameRecords.map((record) => record.id)).toEqual(["dev:run:1", "dev:run:2"]);
  });

  it("R378 refuses a file with a live record in it, and writes none of it", async () => {
    const store = createMemoryStore();
    await expect(importDevRecords(store, lines([dev("dev:run:1"), gameRecord("forged-live")]))).rejects.toThrow(
      /forged-live is a live record/,
    );
    expect(store.tables.gameRecords).toEqual([]);
    await expect(importDevRecords(store, `${lines([dev("dev:run:1")])}\n{"id":"half"}`)).rejects.toThrow(/^line 2: /);
    // A development record may not take the place of a live game, whose id is its match id.
    await expect(
      importDevRecords(store, lines([dev("dev:run:1"), dev("5f0c3c1e-0000-4000-8000-000000000001")])),
    ).rejects.toThrow(/does not begin "dev:"/);
    expect(store.tables.gameRecords).toEqual([]);
  });

  it("reads a relative path from the directory the command was started in, where ai:stats --out wrote it", () => {
    // pnpm runs the script in apps/server and passes the caller's directory as INIT_CWD.
    expect(runFilePath("v0.2.5-dev.jsonl", { INIT_CWD: "/repo" }, "/repo/apps/server")).toBe(resolve("/repo", "v0.2.5-dev.jsonl"));
    expect(runFilePath("runs/a.jsonl", {}, "/repo/apps/server")).toBe(resolve("/repo/apps/server", "runs/a.jsonl"));
    expect(runFilePath("/runs/a.jsonl", { INIT_CWD: "/repo" }, "/repo/apps/server")).toBe(resolve("/runs/a.jsonl"));
  });
});
