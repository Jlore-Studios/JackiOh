// R375: the patch history's data, `packages/cards/patches/`, against git and catalog.json.
//
// Every snapshot is catalog.json exactly as its version left it. A shipped version's is held to the
// git blob `scripts/patches.ts` records for its commit: the blob id is git's own SHA-1 of the file's
// bytes, so a match is byte-equality with `git show <commit>:packages/cards/catalog.json`. CI's
// checkout is a shallow clone without those commits, so that test needs no history; the next one
// runs `git show` itself wherever the clone has the commits, and a full clone that lacks one fails.
// The newest snapshot is catalog.json itself, so a change to the catalog without a new patch is red.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { CardDefs } from "@jackioh/shared";
import { CATALOG } from "../src/catalog-data";
import { newestPatch, patchCards, type Patch, type PatchCards, type Snapshots } from "../src/history";
import {
  BACKFILL,
  CATALOG_PATH,
  CHANGES_PATH,
  PATCHES_PATH,
  REPO_ROOT,
  SHIPPED,
  catalogAt,
  gitBlobId,
  snapshotPath,
} from "../scripts/patches";

const PATCHES = JSON.parse(readFileSync(PATCHES_PATH, "utf8")) as Patch[];
const CHANGES = JSON.parse(readFileSync(CHANGES_PATH, "utf8")) as Record<string, PatchCards>;
const SNAPSHOTS: Snapshots = Object.fromEntries(
  PATCHES.map((patch) => [patch.version, JSON.parse(readFileSync(snapshotPath(patch.version), "utf8")) as CardDefs]),
);

/** The history as issue #39 gives it from `git log --follow packages/cards/catalog.json`. */
const EXPECTED = [
  { version: "v0.1.0", date: "2026-09-18", commits: ["4626690"], sources: [] },
  { version: "v0.1.0-r1", date: "2026-09-22", commits: ["1539fa7", "cd780db"], sources: ["issue 1", "pr 2"] },
  { version: "v0.1.0-r2", date: "2026-09-24", commits: ["f5b94bc", "a17a9e8"], sources: ["pr 11", "pr 14"] },
  { version: "v0.1.0-r3", date: "2026-09-25", commits: ["c219bb4"], sources: ["pr 18"] },
  { version: "v0.1.1", date: "2026-09-27", commits: ["1005c50"], sources: ["issue 27", "pr 28"] },
] as const;

function isShallow(): boolean {
  return execFileSync("git", ["rev-parse", "--is-shallow-repository"], { cwd: REPO_ROOT, encoding: "utf8" }).trim() === "true";
}

describe("R375 the patch list", () => {
  it("R375 lists v0.1.0, v0.1.0-r1, v0.1.0-r2, v0.1.0-r3 and v0.1.1 in order, with their dates, commits and sources", () => {
    expect(PATCHES.slice(0, EXPECTED.length).map((patch) => patch.version)).toEqual(EXPECTED.map((row) => row.version));
    EXPECTED.forEach((row, at) => {
      const patch = PATCHES[at];
      expect(patch?.date, row.version).toBe(row.date);
      expect(patch?.commits.map((commit) => commit.slice(0, 7)), row.version).toEqual(row.commits);
      expect(patch?.sources.map((source) => `${source.kind} ${String(source.number)}`), row.version).toEqual(row.sources);
    });
  });

  it("R375 marks every version before v0.1.1 reconstructed, and v0.1.1 and every later one not", () => {
    const named = PATCHES.findIndex((patch) => patch.version === "v0.1.1");
    expect(named).toBe(4);
    expect(PATCHES.slice(0, named).every((patch) => patch.reconstructed)).toBe(true);
    expect(PATCHES.slice(named).some((patch) => patch.reconstructed)).toBe(false);
  });

  it("R375 gives every patch a version, a date, a title, full commit ids and sources, and notes", () => {
    const versions = new Set<string>();
    for (const patch of PATCHES) {
      expect(Object.keys(patch).sort()).toEqual(["commits", "date", "notes", "reconstructed", "sources", "title", "version"]);
      expect(versions.has(patch.version), `${patch.version} twice`).toBe(false);
      versions.add(patch.version);
      expect(patch.date, patch.version).toMatch(/^\d{4}-\d{2}-\d{2}$/u);
      expect(patch.title, patch.version).not.toBe("");
      for (const commit of patch.commits) expect(commit, patch.version).toMatch(/^[0-9a-f]{40}$/u);
      for (const source of patch.sources) {
        expect(["issue", "pr"], patch.version).toContain(source.kind);
        expect(Number.isInteger(source.number), patch.version).toBe(true);
      }
    }
    // Dates never go back: the list is in order.
    const dates = PATCHES.map((patch) => patch.date);
    expect([...dates].sort()).toEqual(dates);
  });

  it("R375 writes its first entries from the script's record, so the two cannot drift", () => {
    expect(PATCHES.slice(0, BACKFILL.length)).toEqual(BACKFILL);
  });

  it("R375 records, for each shipped version, the last of its commits", () => {
    for (const shipped of SHIPPED) {
      const patch = PATCHES.find((entry) => entry.version === shipped.version);
      expect(patch, shipped.version).toBeDefined();
      expect(patch?.commits[patch.commits.length - 1], shipped.version).toBe(shipped.commit);
    }
  });
});

describe("R375 the snapshots", () => {
  it("R375 holds each shipped snapshot byte-equal to the catalog.json blob its commit records", () => {
    // The history as git has it comes first; a patch shipped since follows.
    expect(SHIPPED.slice(0, EXPECTED.length).map((shipped) => shipped.version)).toEqual(EXPECTED.map((row) => row.version));
    for (const shipped of SHIPPED) {
      expect(gitBlobId(readFileSync(snapshotPath(shipped.version))), shipped.version).toBe(shipped.blob);
    }
  });

  it("R375 holds each shipped snapshot byte-equal to git show of its commit, wherever the clone has it", () => {
    const shallow = isShallow();
    for (const shipped of SHIPPED) {
      const atCommit = catalogAt(shipped.commit);
      if (atCommit === null) {
        // Only a shallow clone may lack the commit; a full clone that does is a wrong commit id.
        expect(shallow, `${shipped.commit} (${shipped.version}) is not in this full clone`).toBe(true);
        continue;
      }
      expect(gitBlobId(atCommit), shipped.version).toBe(shipped.blob);
      expect(readFileSync(snapshotPath(shipped.version)).equals(atCommit), shipped.version).toBe(true);
    }
  });

  it("R375 holds the newest snapshot equal to catalog.json", () => {
    const newest = newestPatch(PATCHES);
    expect(SNAPSHOTS[newest.version]).toEqual(CATALOG);
    expect(readFileSync(snapshotPath(newest.version)).equals(readFileSync(CATALOG_PATH))).toBe(true);
  });

  it("R375 has v0.1.0 as 100 cards and 9 tokens", () => {
    const first = Object.values(SNAPSHOTS["v0.1.0"] ?? {});
    expect(first.filter((def) => !def.token)).toHaveLength(100);
    expect(first.filter((def) => def.token)).toHaveLength(9);
  });
});

describe("R375 changes.json", () => {
  it("R375 is what the snapshots say each version created, changed and removed", () => {
    expect(CHANGES).toEqual(patchCards(PATCHES, SNAPSHOTS));
  });

  it("R375 counts the history issue #39 gives", () => {
    const counts = Object.fromEntries(
      EXPECTED.map(({ version }) => {
        const cards = CHANGES[version];
        return [version, [cards?.created.length, cards?.changed.length, cards?.removed.length]];
      }),
    );
    expect(counts).toEqual({
      "v0.1.0": [109, 0, 0],
      "v0.1.0-r1": [0, 7, 0],
      "v0.1.0-r2": [1, 1, 0],
      "v0.1.0-r3": [0, 99, 0],
      "v0.1.1": [1, 105, 0],
    });
    expect(CHANGES["v0.1.0-r1"]?.changed).toEqual([
      "core-003",
      "core-068",
      "core-081",
      "core-t-rush",
      "core-t-sheep",
      "core-t-felinor",
      "core-t-bread",
    ]);
    expect(CHANGES["v0.1.0-r2"]).toEqual({ created: ["core-t-coin"], changed: ["core-095"], removed: [] });
    expect(CHANGES["v0.1.1"]?.created).toEqual(["core-t-ghoul"]);
  });
});
