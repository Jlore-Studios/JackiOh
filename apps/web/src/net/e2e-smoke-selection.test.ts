// e2e/scripts/shard-specs.mjs: pull request e2e smoke spec selection (#5099).

import { spawnSync } from "node:child_process";
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { afterEach, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const E2E = join(HERE, "../../../../e2e");
const SCRIPT = join(E2E, "scripts/shard-specs.mjs");

interface ReachedInfo {
  how: "names" | "deals";
  cards: string[];
}

interface Selection {
  specs: string[];
  added: string[];
  left: string[];
  load: number;
  budget: number;
  reached: Map<string, ReachedInfo>;
}

interface World {
  sources: Record<string, string>;
  decks: Record<string, string[]>;
  names: Record<string, string>;
}

interface Script {
  SMOKE: string[];
  SMOKE_EXTRA_BUDGET_S: number;
  specWeight: (spec: string) => number;
  shardSpecs: (specs: string[], k: number, count: number, weight?: (spec: string) => number) => string[];
  smokeSpecs: (specs: string[]) => string[];
  smokeSelection: (every: string[], ids: string[], count: number, world: World) => Selection;
  reachedSpecs: (ids: string[], specs: string[], world: World) => Map<string, ReachedInfo>;
  readWorld: (root: string, ids: string[]) => World;
}

describe("e2e-smoke-selection", () => {
  let script: Script;
  let every: string[];
  const stubDirs: string[] = [];

  beforeAll(async () => {
    script = (await import(pathToFileURL(SCRIPT).href)) as Script;
    every = readdirSync(join(E2E, "cypress/e2e")).filter((name) => name.endsWith(".cy.ts"));
  });

  afterEach(() => {
    for (const dir of stubDirs) {
      rmSync(dir, { recursive: true, force: true });
    }
    stubDirs.length = 0;
  });

  function makeStub(content: string): string {
    const dir = mkdtempSync(join(tmpdir(), "jackioh-stub-"));
    stubDirs.push(dir);
    const stubPath = join(dir, "jackioh");
    writeFileSync(stubPath, content, { mode: 0o755 });
    return stubPath;
  }

  function select(ids: string[], count = 4): Selection {
    return script.smokeSelection(every, ids, count, script.readWorld(E2E, ids));
  }

  it("#5099 with core-077 changed, selection includes 29 and 03 alongside SMOKE specs", () => {
    const res = select(["core-077"]);
    expect(res.specs).toContain("29-animated-trap.cy.ts");
    expect(res.specs).toContain("03-trap-opponent-turn.cy.ts");
    for (const smoke of script.smokeSpecs(every)) {
      expect(res.specs).toContain(smoke);
    }
  });

  it("#5099 with core-093 changed, selection includes 28-patch-011-ui.cy.ts", () => {
    const res = select(["core-093"]);
    expect(res.specs).toContain("28-patch-011-ui.cy.ts");
  });

  it("#5099 with no changed cards, selection is exactly SMOKE", () => {
    const res = select([]);
    expect(res.specs).toEqual(script.smokeSpecs(every));
    expect(res.added).toEqual([]);
    expect(res.left).toEqual([]);
  });

  it("#5099 with core-025 changed, respects budget, ranks names before deals, and reports left", () => {
    const res = select(["core-025"]);
    expect(res.load).toBeLessThanOrEqual(script.SMOKE_EXTRA_BUDGET_S * 4);
    const addedWeightSum = res.added.reduce((sum, spec) => sum + script.specWeight(spec), 0);
    expect(res.load).toBe(addedWeightSum);

    let seenDeals = false;
    for (const spec of res.added) {
      const how = res.reached.get(spec)?.how;
      if (how === "deals") {
        seenDeals = true;
      } else if (how === "names") {
        expect(seenDeals).toBe(false);
      }
    }

    expect(res.left.length).toBeGreaterThan(0);
    for (const spec of res.left) {
      expect(res.added).not.toContain(spec);
    }
  });

  it.each([1, 4, 8])("#5099 partitions specs into %i shards without duplicates", (count) => {
    for (const ids of [["core-077"], ["core-025"]]) {
      const res = select(ids, count);
      const partitioned: string[] = [];
      for (let k = 1; k <= count; k++) {
        partitioned.push(...script.shardSpecs(res.specs, k, count));
      }
      expect([...partitioned].sort()).toEqual([...res.specs].sort());
      expect(new Set(partitioned).size).toBe(partitioned.length);
    }
  });

  it("#5120 specs 46 and 47 are never added even when sources quote changed cards and decks", () => {
    const changedCard = "core-077";
    const deckId = "fixture-deck";
    const base = script.readWorld(E2E, [changedCard]);
    const world: World = {
      sources: {
        ...base.sources,
        "46-menus-fit-one-screen.cy.ts": `"${changedCard}" "${deckId}"`,
        "47-tap-areas.cy.ts": `"${deckId}"`,
        "29-animated-trap.cy.ts": `"${changedCard}"`,
      },
      decks: {
        ...base.decks,
        [deckId]: [changedCard],
      },
      names: base.names,
    };

    const candidateSpecs = ["46-menus-fit-one-screen.cy.ts", "47-tap-areas.cy.ts", "29-animated-trap.cy.ts"];
    const reached = script.reachedSpecs([changedCard], candidateSpecs, world);
    expect(reached.has("46-menus-fit-one-screen.cy.ts")).toBe(false);
    expect(reached.has("47-tap-areas.cy.ts")).toBe(false);
    expect(reached.has("29-animated-trap.cy.ts")).toBe(true);

    const res = script.smokeSelection(every, [changedCard], 4, world);
    expect(res.reached.has("46-menus-fit-one-screen.cy.ts")).toBe(false);
    expect(res.reached.has("47-tap-areas.cy.ts")).toBe(false);
    expect(res.added).not.toContain("46-menus-fit-one-screen.cy.ts");
    expect(res.added).not.toContain("47-tap-areas.cy.ts");
    expect(res.left).not.toContain("46-menus-fit-one-screen.cy.ts");
    expect(res.left).not.toContain("47-tap-areas.cy.ts");
    expect(res.specs).not.toContain("46-menus-fit-one-screen.cy.ts");
    expect(res.specs).not.toContain("47-tap-areas.cy.ts");

    expect(res.reached.has("29-animated-trap.cy.ts")).toBe(true);
    expect(res.added).toContain("29-animated-trap.cy.ts");
  });

  it("#5099 CLI without binary prints SMOKE shard 1 and shards together are SMOKE", () => {
    const missingBin = join(tmpdir(), "nonexistent-jackioh-bin");
    const run = (k: number) =>
      spawnSync(process.execPath, [SCRIPT, "--smoke", String(k), "4"], {
        cwd: E2E,
        env: { ...process.env, JACKIOH_BIN: missingBin },
        encoding: "utf8",
      });

    const res1 = run(1);
    expect(res1.status).toBe(0);
    expect(res1.stdout.trim()).toBe("cypress/e2e/01-hotseat-full-game.cy.ts");

    const all = [1, 2, 3, 4].flatMap((k) => run(k).stdout.trim().split(","));
    const smokeFiles = script.smokeSpecs(every).map((s) => `cypress/e2e/${s}`);
    expect(all.sort()).toEqual(smokeFiles.sort());
  });

  it("#5099 CLI with core-077 changed stub includes 29 and 03 and logs to stderr", () => {
    const stub = makeStub(`#!/bin/sh\necho '{"ids":["core-077"],"notes":[],"warning":null}'\n`);
    const run = (k: number) =>
      spawnSync(process.execPath, [SCRIPT, "--smoke", String(k), "4"], {
        cwd: E2E,
        env: { ...process.env, JACKIOH_BIN: stub },
        encoding: "utf8",
      });

    const res1 = run(1);
    expect(res1.status).toBe(0);
    expect(res1.stderr).toContain("core-077");
    expect(res1.stderr).toContain("smoke: + 29-animated-trap.cy.ts");

    const all = [1, 2, 3, 4].flatMap((k) => run(k).stdout.trim().split(","));
    expect(all).toContain("cypress/e2e/29-animated-trap.cy.ts");
    expect(all).toContain("cypress/e2e/03-trap-opponent-turn.cy.ts");
    for (const smoke of script.smokeSpecs(every)) {
      expect(all).toContain(`cypress/e2e/${smoke}`);
    }
  });

  it("#5099 CLI with empty ids stub or failing stub stays SMOKE", () => {
    const emptyStub = makeStub(`#!/bin/sh\necho '{"ids":[],"notes":[],"warning":null}'\n`);
    const failStub = makeStub(`#!/bin/sh\nexit 1\n`);
    const smokeFiles = script.smokeSpecs(every).map((s) => `cypress/e2e/${s}`);

    const runWith = (stub: string) => {
      const all = [1, 2, 3, 4].flatMap((k) => {
        const res = spawnSync(process.execPath, [SCRIPT, "--smoke", String(k), "4"], {
          cwd: E2E,
          env: { ...process.env, JACKIOH_BIN: stub },
          encoding: "utf8",
        });
        expect(res.status).toBe(0);
        return res.stdout.trim().split(",");
      });
      expect(all.sort()).toEqual(smokeFiles.sort());
    };

    runWith(emptyStub);

    const failRun1 = spawnSync(process.execPath, [SCRIPT, "--smoke", "1", "4"], {
      cwd: E2E,
      env: { ...process.env, JACKIOH_BIN: failStub },
      encoding: "utf8",
    });
    expect(failRun1.status).toBe(0);
    expect(failRun1.stderr).toContain("the list stays SMOKE");
    runWith(failStub);
  });
});
