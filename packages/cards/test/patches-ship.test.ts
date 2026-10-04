// R641: several card patches are built at once, so branches add a pending fragment under
// `patches/pending/` instead of editing the history; `patches check` proves every catalog change
// against the newest shipped snapshot, and `patches ship` promotes each fragment in ship order.
// The pure rules are proved here on fixtures; the promotion is proved below on a throwaway git
// repo that replays the issue's acceptance scenario (v0.2.5 landing before v0.2.0).

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  checkFragments,
  gitBlobHash,
  nextShipName,
  readFragments,
  readPatches,
  readShipped,
  readSnapshot,
  revertPending,
  sameCatalog,
  type Catalog,
} from "../scripts/patches-io";
import { checkPatches, shipPatches, utcDateOf, writeFragment } from "../scripts/patches";

const card = (id: string, cost: number): Record<string, unknown> => ({ id, name: `Card ${id}`, cost });

describe("R641 pending fragments and the check that proves them", () => {
  it("R641 claims every catalog change exactly once, and only changes", () => {
    const newest: Catalog = { aaa: card("aaa", 1), bbb: card("bbb", 1) };
    const files = [
      { name: "v0.2.5.json", fragment: { version: "v0.2.5", title: "t", sources: "s", notes: "n", cards: ["aaa"] } },
    ];
    // Changed and claimed: clean.
    expect(checkFragments({ files, catalog: { aaa: card("aaa", 2), bbb: card("bbb", 1) }, newest })).toEqual([]);
    // Changed and unclaimed: names the card.
    expect(checkFragments({ files: [], catalog: { aaa: card("aaa", 2), bbb: card("bbb", 1) }, newest })).toEqual([
      '"aaa" differs from the newest shipped snapshot but no pending fragment claims it',
    ]);
    // Removed and unclaimed: names the card too.
    expect(checkFragments({ files: [], catalog: { aaa: card("aaa", 1) }, newest })).toEqual([
      '"bbb" differs from the newest shipped snapshot but no pending fragment claims it',
    ]);
    // Claimed twice: names the card and both fragments.
    const twice = [
      ...files,
      { name: "v0.2.0.json", fragment: { version: "v0.2.0", title: "t", sources: "s", notes: "n", cards: ["aaa"] } },
    ];
    expect(checkFragments({ files: twice, catalog: { aaa: card("aaa", 2), bbb: card("bbb", 1) }, newest })).toEqual([
      '"aaa" is claimed by "v0.2.5" and "v0.2.0", but one card ships in one patch',
    ]);
    // Claimed but unchanged: names the card.
    expect(checkFragments({ files, catalog: { aaa: card("aaa", 1), bbb: card("bbb", 1) }, newest })).toEqual([
      '"aaa" is claimed by "v0.2.5" but identical to the newest shipped snapshot',
    ]);
  });

  it("R641 holds every fragment to a bare patch number matching its file", () => {
    const newest: Catalog = { aaa: card("aaa", 1) };
    const catalog: Catalog = { aaa: card("aaa", 1) };
    const bad = [{ name: "v9.json", fragment: { version: "9.9", title: "t", sources: "s", notes: "n", cards: [] } }];
    expect(checkFragments({ files: bad, catalog, newest }).join("\n")).toContain('"9.9"');
    const renamed = [
      { name: "v0.2.5.json", fragment: { version: "v0.2.0", title: "t", sources: "s", notes: "n", cards: [] } },
    ];
    expect(checkFragments({ files: renamed, catalog, newest }).join("\n")).toContain("its file names");
  });

  it("R641 holds a fragment to what a shipped patch carries: a title, a source and notes", () => {
    const newest: Catalog = { aaa: card("aaa", 1) };
    const empty = [
      { name: "v0.2.5.json", fragment: { version: "v0.2.5", title: "", sources: " ", notes: "", cards: ["aaa"] } },
    ];
    expect(checkFragments({ files: empty, catalog: { aaa: card("aaa", 2) }, newest })).toEqual([
      "pending/v0.2.5.json has an empty title, but a shipped patch needs one",
      "pending/v0.2.5.json has an empty sources, but a shipped patch needs one",
      "pending/v0.2.5.json has an empty notes, but a shipped patch needs one",
    ]);
  });

  it("R641 reverts the catalog to the newest snapshot on exactly the claimed cards", () => {
    const newest: Catalog = { aaa: card("aaa", 1), bbb: card("bbb", 1) };
    // Changed, added and removed entries all come back; unclaimed entries are untouched.
    const catalog: Catalog = { aaa: card("aaa", 2), ccc: card("ccc", 1) };
    const reverted = revertPending(catalog, newest, new Set(["aaa", "bbb", "ccc"]));
    expect(sameCatalog(reverted, newest)).toBe(true);
    expect(sameCatalog(catalog, newest)).toBe(false);
    expect(sameCatalog(revertPending(catalog, newest, new Set()), newest)).toBe(false);
    expect(sameCatalog(newest, newest)).toBe(true);
  });

  it("R641 ships a taken version as the next revision letter, never by reopening it", () => {
    expect(nextShipName(new Set(), "v0.2.0")).toBe("v0.2.0");
    expect(nextShipName(new Set(["v0.2.0"]), "v0.2.0")).toBe("v0.2.0b");
    expect(nextShipName(new Set(["v0.2.0", "v0.2.0b"]), "v0.2.0")).toBe("v0.2.0c");
    expect(nextShipName(new Set(["v0.2.5"]), "v0.2.0")).toBe("v0.2.0");
  });

  it("R641 dates a patch by its commit's UTC day, never the local one", () => {
    expect(utcDateOf(0)).toBe("1970-01-01");
    // 2026-10-01T12:00:00Z.
    expect(utcDateOf(1790856000)).toBe("2026-10-01");
    // 2026-10-01T00:30:00+02:00 is still 2026-09-30 in UTC.
    expect(utcDateOf(1790807400)).toBe("2026-09-30");
  });

  it("R641 hashes a snapshot's bytes the way git does", () => {
    expect(gitBlobHash("test\n")).toBe("9daeafb9864cf43055ae93beb0afd6c7d144bfa4");
    expect(gitBlobHash("")).toBe("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
  });
});

describe("R641 promotion in ship order", () => {
  const IDENT = {
    GIT_AUTHOR_NAME: "night bot",
    GIT_AUTHOR_EMAIL: "bot@example.invalid",
    GIT_COMMITTER_NAME: "night bot",
    GIT_COMMITTER_EMAIL: "bot@example.invalid",
    GIT_CONFIG_COUNT: "1",
    GIT_CONFIG_KEY_0: "commit.gpgsign",
    GIT_CONFIG_VALUE_0: "false",
  };

  function git(root: string, args: string[], date?: string): string {
    const dated =
      date === undefined ? IDENT : { ...IDENT, GIT_AUTHOR_DATE: date, GIT_COMMITTER_DATE: date };
    return execFileSync("git", args, { cwd: root, encoding: "utf8", env: { ...process.env, ...dated } });
  }

  function writeFiles(root: string, files: Record<string, string>): void {
    for (const [rel, text] of Object.entries(files)) {
      const path = join(root, rel);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, text, "utf8");
    }
  }

  const json = (value: unknown): string => `${JSON.stringify(value, null, 2)}\n`;
  const costOf = (snapshot: Catalog, id: string): unknown => snapshot[id]?.["cost"];

  // The issue's acceptance scenario on a three-card catalog: branch A adds fragment v0.2.5
  // changing card aaa, branch B adds fragment v0.2.0 changing card bbb. A merges, then B merges
  // with no conflict under pending/, and promotion ships v0.2.5 before v0.2.0 whatever the names
  // say. A later fragment named v0.2.0 ships as v0.2.0b, then v0.2.0c.
  it("R641 ships pending fragments oldest merge first, snapshots each merge's catalog, and letters revisions", () => {
    const root = mkdtempSync(join(tmpdir(), "jackioh-ship-"));
    try {
      const dir = `${root}/packages/cards/patches/`;
      const base: Catalog = { aaa: card("aaa", 1), bbb: card("bbb", 1), ccc: card("ccc", 1) };
      git(root, ["init", "-q", "-b", "main"]);
      writeFiles(root, {
        "packages/cards/catalog.json": json(base),
        "packages/cards/patches/patches.json": json([
          {
            version: "v0.1.1",
            date: "2026-09-27",
            title: "base",
            source: "test",
            notes: "base notes",
            changes: [
              { id: "aaa", name: "Card aaa", kind: "added" },
              { id: "bbb", name: "Card bbb", kind: "added" },
              { id: "ccc", name: "Card ccc", kind: "added" },
            ],
          },
        ]),
        "packages/cards/patches/v0.1.1.json": json(base),
        "packages/cards/patches/index.json": json({ aaa: ["v0.1.1"], bbb: ["v0.1.1"], ccc: ["v0.1.1"] }),
        "packages/cards/patches/shipped.json": json([
          { version: "v0.1.1", commit: "0".repeat(40), blob: "0".repeat(40) },
        ]),
        "packages/cards/src/catalog-data.ts": 'export const CATALOG_VERSION = "v0.1.1";\n',
        "apps/server/.env.example": "CATALOG_VERSION=v0.1.1\n",
        "render.yaml": "x:\n- key: CATALOG_VERSION\n  value: v0.1.1\n",
        "apps/server/src/index.ts": 'export const env = {\n  CATALOG_VERSION: "v0.1.1",\n};\n',
      });
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "base"], "2026-09-27T12:00:00+00:00");
      const c0 = git(root, ["rev-parse", "HEAD"]).trim();
      writeFiles(root, {
        "packages/cards/patches/shipped.json": json([
          {
            version: "v0.1.1",
            commit: c0,
            blob: gitBlobHash(readFileSync(`${dir}v0.1.1.json`, "utf8")),
          },
        ]),
      });

      // Branch A merges: card aaa changes, fragment v0.2.5 claims it (cards diffed, not listed).
      const afterA: Catalog = { ...base, aaa: card("aaa", 2) };
      writeFiles(root, { "packages/cards/catalog.json": json(afterA) });
      expect(writeFragment(root, { version: "v0.2.5", title: "Patch v0.2.5", sources: "issue #48", notes: "X" }).cards).toEqual([
        "aaa",
      ]);
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "fragment v0.2.5"], "2026-10-01T12:00:00+00:00");
      const c1 = git(root, ["rev-parse", "HEAD"]).trim();

      // Branch B merges: card bbb changes, fragment v0.2.0 claims it (cards listed).
      const afterB: Catalog = { ...afterA, bbb: card("bbb", 3) };
      writeFiles(root, { "packages/cards/catalog.json": json(afterB) });
      writeFragment(root, { version: "v0.2.0", title: "Patch v0.2.0", sources: "issue #40", notes: "Y", cards: ["bbb"] });
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "fragment v0.2.0"], "2026-10-02T12:00:00+00:00");
      const c2 = git(root, ["rev-parse", "HEAD"]).trim();

      expect(checkPatches(root)).toEqual([]);
      expect(shipPatches(root)).toEqual({ shipped: ["v0.2.5", "v0.2.0"] });

      const patches = readPatches(`${dir}patches.json`);
      expect(patches.map((patch) => patch.version)).toEqual(["v0.1.1", "v0.2.5", "v0.2.0"]);
      const v25 = patches.find((patch) => patch.version === "v0.2.5");
      const v20 = patches.find((patch) => patch.version === "v0.2.0");
      expect({ date: v25?.date, title: v25?.title, source: v25?.source, notes: v25?.notes }).toEqual({
        date: "2026-10-01",
        title: "Patch v0.2.5",
        source: "issue #48",
        notes: "X",
      });
      expect({ commits: v25?.commits, reconstructed: v25?.reconstructed }).toEqual({
        commits: [c1],
        reconstructed: false,
      });
      expect(v20?.date).toBe("2026-10-02");
      // Each snapshot is the catalog as its merge left it: v0.2.0 carries v0.2.5's change.
      expect(costOf(readSnapshot("v0.2.5", dir), "aaa")).toBe(2);
      expect(costOf(readSnapshot("v0.2.5", dir), "bbb")).toBe(1);
      expect(costOf(readSnapshot("v0.2.0", dir), "aaa")).toBe(2);
      expect(costOf(readSnapshot("v0.2.0", dir), "bbb")).toBe(3);
      // …so v0.2.0's card-by-card changes list only its own card.
      expect(v20?.changes).toEqual([{ id: "bbb", name: "Card bbb", kind: "changed", fields: ["cost"] }]);
      expect(JSON.parse(readFileSync(`${dir}index.json`, "utf8"))).toEqual({
        aaa: ["v0.1.1", "v0.2.5"],
        bbb: ["v0.1.1", "v0.2.0"],
        ccc: ["v0.1.1"],
      });
      // The fragments are gone, the provenance is recorded, the version sites moved together.
      expect(readFragments(`${dir}pending/`)).toEqual([]);
      const shipped = readShipped(`${dir}shipped.json`);
      expect(shipped.map((entry) => entry.version)).toEqual(["v0.1.1", "v0.2.5", "v0.2.0"]);
      expect(shipped[1]).toEqual({
        version: "v0.2.5",
        commit: c1,
        blob: gitBlobHash(readFileSync(`${dir}v0.2.5.json`, "utf8")),
      });
      expect(shipped[2]?.commit).toBe(c2);
      expect(readFileSync(`${root}/packages/cards/src/catalog-data.ts`, "utf8")).toContain('CATALOG_VERSION = "v0.2.0"');
      expect(readFileSync(`${root}/apps/server/.env.example`, "utf8")).toContain("CATALOG_VERSION=v0.2.0");
      expect(readFileSync(`${root}/render.yaml`, "utf8")).toContain("value: v0.2.0");
      expect(readFileSync(`${root}/apps/server/src/index.ts`, "utf8")).toContain('CATALOG_VERSION: "v0.2.0"');
      // Promotion leaves the working catalog alone, and the tree is shippable again.
      expect(JSON.parse(readFileSync(`${root}/packages/cards/catalog.json`, "utf8"))).toEqual(afterB);
      expect(checkPatches(root)).toEqual([]);

      // Running ship twice changes nothing.
      const patchesBytes = readFileSync(`${dir}patches.json`, "utf8");
      expect(shipPatches(root)).toEqual({ shipped: [] });
      expect(readFileSync(`${dir}patches.json`, "utf8")).toBe(patchesBytes);

      // The promotion is committed, as the workflow's pull request would commit it: the next
      // fragment under a shipped name is added again, not modified.
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "ship v0.2.5 and v0.2.0"], "2026-10-02T12:00:00+00:00");

      // A later fragment under a shipped name ships as the next revision letter, appended last.
      const afterC: Catalog = { ...afterB, ccc: card("ccc", 5) };
      writeFiles(root, { "packages/cards/catalog.json": json(afterC) });
      writeFragment(root, { version: "v0.2.0", title: "follow-up", sources: "issue", notes: "Z" });
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "fragment v0.2.0 again"], "2026-10-03T12:00:00+00:00");
      expect(shipPatches(root)).toEqual({ shipped: ["v0.2.0b"] });
      const versions = readPatches(`${dir}patches.json`).map((patch) => patch.version);
      expect(versions).toEqual(["v0.1.1", "v0.2.5", "v0.2.0", "v0.2.0b"]);
      const v20b = readPatches(`${dir}patches.json`).find((patch) => patch.version === "v0.2.0b");
      expect(v20b?.date).toBe("2026-10-03");
      expect(costOf(readSnapshot("v0.2.0b", dir), "ccc")).toBe(5);
      expect(v20b?.changes).toEqual([{ id: "ccc", name: "Card ccc", kind: "changed", fields: ["cost"] }]);
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "ship v0.2.0b"], "2026-10-03T12:00:00+00:00");

      // …and the one after that is v0.2.0c.
      const afterD: Catalog = { ...afterC, ccc: card("ccc", 6) };
      writeFiles(root, { "packages/cards/catalog.json": json(afterD) });
      writeFragment(root, { version: "v0.2.0", title: "another follow-up", sources: "issue", notes: "Z2" });
      git(root, ["add", "-A"]);
      git(root, ["commit", "-q", "-m", "fragment v0.2.0 a third time"], "2026-10-04T12:00:00+00:00");
      expect(shipPatches(root)).toEqual({ shipped: ["v0.2.0c"] });
      expect(readPatches(`${dir}patches.json`).map((patch) => patch.version)).toEqual([
        "v0.1.1",
        "v0.2.5",
        "v0.2.0",
        "v0.2.0b",
        "v0.2.0c",
      ]);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("R641 fails check on the wiring, not only on fixtures: an unclaimed change in a real tree", () => {
    const root = mkdtempSync(join(tmpdir(), "jackioh-check-"));
    try {
      const base: Catalog = { aaa: card("aaa", 1) };
      writeFiles(root, {
        "packages/cards/catalog.json": json({ aaa: card("aaa", 2) }),
        "packages/cards/patches/patches.json": json([
          { version: "v0.1.1", date: "2026-09-27", title: "base", source: "test", notes: "n", changes: [] },
        ]),
        "packages/cards/patches/v0.1.1.json": json(base),
        "packages/cards/patches/index.json": json({}),
        "packages/cards/patches/shipped.json": json([]),
      });
      expect(checkPatches(root)).toEqual([
        '"aaa" differs from the newest shipped snapshot but no pending fragment claims it',
      ]);
      writeFragment(root, { version: "v0.2.5", title: "t", sources: "s", notes: "n" });
      expect(checkPatches(root)).toEqual([]);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
