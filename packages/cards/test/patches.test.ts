// R388 (B4.2): card patches are data. `packages/cards/patches/patches.json` lists every shipped
// patch in ship order, each `<version>.json` is the whole catalog as that patch left it,
// `index.json` says in which versions each card changed, and `shipped.json` carries each patch's
// shipping commit and snapshot blob. The catalog version is the patch: `CATALOG_VERSION` is the
// newest patch's version, everywhere the string lives. A version is opaque (R105): its order is
// patches.json's, never a comparison of strings.
//
// Several patches are built at once (R635), so branches change `catalog.json` and add one
// fragment under `patches/pending/` instead of editing the history: while a fragment is pending,
// the catalog differs from the newest snapshot on exactly the claimed cards, and `patches ship`
// promotes each fragment after it merges.
//
// The history before v0.2.0 was rebuilt from `git log --follow packages/cards/catalog.json`; the
// table the brief checked on 2026-09-30 is asserted below, card by card where it names cards.

import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { CATALOG, CATALOG_VERSION } from "../src/catalog-data";
import {
  INDEX_JSON,
  buildIndex,
  diffCatalogs,
  gitBlobHash,
  readFragments,
  readPatches,
  readShipped,
  readSnapshot,
  revertPending,
  sameCatalog,
  snapshotPath,
  type Catalog,
} from "../scripts/patches-io";
import { versionsAtSites } from "../scripts/patch";

const PATCHES = readPatches();
const VERSIONS = PATCHES.map((patch) => patch.version);
const changesOf = (version: string) => PATCHES.find((patch) => patch.version === version)?.changes ?? [];
const idsOf = (version: string, kind: string): string[] =>
  changesOf(version)
    .filter((change) => change.kind === kind)
    .map((change) => change.id);

describe("R388 card patch history (B4.2)", () => {
  it("R388 lists every patch once, in the order they were made, each with its snapshot", () => {
    // The table the brief checked is the prefix: promotions only ever append (R635), so the
    // history the file holds today can grow past it but never move it.
    expect(VERSIONS.slice(0, 6)).toEqual(["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0"]);
    expect(new Set(VERSIONS).size).toBe(VERSIONS.length);
    for (const patch of PATCHES) {
      expect(existsSync(snapshotPath(patch.version)), `${patch.version}.json`).toBe(true);
      expect(patch.date, patch.version).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(patch.title.length, patch.version).toBeGreaterThan(0);
      expect(patch.source.length, patch.version).toBeGreaterThan(0);
      expect(patch.notes.length, patch.version).toBeGreaterThan(0);
    }
  });

  it("R388 makes the catalog version the newest patch, and catalog.json its snapshot apart from pending fragments (R635)", () => {
    expect(CATALOG_VERSION).toBe(VERSIONS[VERSIONS.length - 1]);
    const snapshot = readSnapshot(CATALOG_VERSION);
    // Pending fragments hold the catalog ahead of the newest snapshot on exactly their claimed
    // cards (R635): reverted to the snapshot, the catalog is the snapshot. With no fragments
    // pending this is the old equality, entry for entry.
    const claimed = new Set(readFragments().flatMap(({ fragment }) => fragment.cards));
    const catalog = CATALOG as unknown as Catalog;
    const reverted = revertPending(catalog, snapshot, claimed);
    expect(
      sameCatalog(reverted, snapshot),
      "catalog.json with every pending-claimed entry reverted is the newest snapshot: " +
        "claim the difference with `pnpm --filter @jackioh/cards patches <version> \"<title>\"`",
    ).toBe(true);
    if (claimed.size === 0) {
      expect(JSON.stringify(Object.keys(snapshot))).toBe(JSON.stringify(Object.keys(catalog)));
    }
  });

  it("R635 lists every shipped patch once in shipped.json, with the commit that shipped it and its snapshot's blob", () => {
    const shipped = readShipped();
    expect(shipped.map((entry) => entry.version)).toEqual(VERSIONS);
    expect(new Set(shipped.map((entry) => entry.version)).size).toBe(shipped.length);
    for (const entry of shipped) {
      expect(entry.commit, `${entry.version} commit`).toMatch(/^[0-9a-f]{40}$/);
      expect(entry.blob, `${entry.version} blob`).toMatch(/^[0-9a-f]{40}$/);
      // The blob is the snapshot file's bytes as git hashes them, so a rewritten snapshot fails.
      const bytes = readFileSync(snapshotPath(entry.version), "utf8");
      expect(entry.blob, `${entry.version}.json`).toBe(gitBlobHash(bytes));
    }
  });

  it("R388 bumps the version everywhere the string lives: the server's env example, render.yaml and its end-to-end default", () => {
    for (const site of versionsAtSites()) {
      expect(site.version, site.file).toBe(CATALOG_VERSION);
    }
  });

  it("R388 derives each patch's card-by-card changes and the per-card index from the snapshots alone", () => {
    let previous: Catalog | null = null;
    for (const patch of PATCHES) {
      const snapshot = readSnapshot(patch.version);
      expect(patch.changes, `${patch.version} changes`).toEqual(diffCatalogs(previous, snapshot));
      previous = snapshot;
    }
    expect(JSON.parse(readFileSync(INDEX_JSON, "utf8"))).toEqual(buildIndex(PATCHES));
  });

  it("R388 rebuilds the history the brief checked: v0.1.0 to v0.1.1 from git", () => {
    // v0.1.0: the initial commit, Core as first built, 100 cards and 9 tokens.
    expect(idsOf("v0.1.0", "added")).toHaveLength(109);
    expect(changesOf("v0.1.0")).toHaveLength(109);
    // v0.1.0b: #3, #68's name, #81 (twice), the Rush, Sheep, Felinor and Bread Tokens' Radiant faces.
    expect(idsOf("v0.1.0b", "changed")).toEqual([
      "core-003",
      "core-068",
      "core-081",
      "core-t-rush",
      "core-t-sheep",
      "core-t-felinor",
      "core-t-bread",
    ]);
    // v0.1.0c: #95's text; The Coin added.
    expect(idsOf("v0.1.0c", "changed")).toEqual(["core-095"]);
    expect(idsOf("v0.1.0c", "added")).toEqual(["core-t-coin"]);
    // v0.1.0d: the Radiant pass, 99 entries.
    expect(idsOf("v0.1.0d", "changed")).toHaveLength(99);
    // v0.1.1: the Ghoul Token added and 105 entries changed.
    expect(idsOf("v0.1.1", "added")).toEqual(["core-t-ghoul"]);
    expect(idsOf("v0.1.1", "changed")).toHaveLength(105);
    // Nothing has ever been removed.
    expect(PATCHES.flatMap((patch) => patch.changes).filter((change) => change.kind === "removed")).toEqual([]);
  });

  it("R388 records patch v0.2.0: the two new sets added and the Core patches of issue #40", () => {
    expect(idsOf("v0.2.0", "added")).toHaveLength(206);
    const before = readSnapshot("v0.1.1");
    const after = readSnapshot("v0.2.0");
    const cost = (snapshot: Catalog, id: string): unknown => snapshot[id]?.["cost"];
    expect([16, 17, 34, 43, 49, 65, 88].map((n) => cost(before, `core-0${String(n).padStart(2, "0")}`))).toEqual([2, 3, 3, 3, 3, 2, 3]);
    expect([16, 17, 34, 43, 49, 65, 88].map((n) => cost(after, `core-0${String(n).padStart(2, "0")}`))).toEqual([3, 4, 4, 4, 4, 1, 4]);
    const changed = changesOf("v0.2.0").find((change) => change.id === "core-016");
    expect(changed?.kind === "changed" ? changed.fields : []).toContain("cost");
  });

  it("R388 indexes each card by the versions that added or changed it", () => {
    const index = buildIndex(PATCHES);
    expect(index["core-t-coin"]?.[0]).toBe("v0.1.0c");
    expect(index["core-t-ghoul"]?.[0]).toBe("v0.1.1");
    expect(index["classic-001"]?.[0]).toBe("v0.2.0");
    expect(index["core-016"]).toContain("v0.2.0");
    // Every catalog entry was added by some patch, or is claimed by a pending fragment — which is
    // not a shipped patch yet, so the index does not name it (R635).
    const claimed = new Set(readFragments().flatMap(({ fragment }) => fragment.cards));
    expect(Object.keys(CATALOG).filter((id) => index[id] === undefined && !claimed.has(id))).toEqual([]);
  });
});

// R375: issue #39's first build of the history was replaced by R388's when the two met on main, and
// what both agreed on is held here: the versions before v0.2.0, their order, and the cards they hold.
describe("R375 issue #39's versions of the patch history", () => {
  it("R375 keeps v0.1.0, v0.1.0b, v0.1.0c, v0.1.0d and v0.1.1, in that order, before v0.2.0", () => {
    expect(VERSIONS.slice(0, VERSIONS.indexOf("v0.2.0"))).toEqual(["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1"]);
  });

  it("R375 has v0.1.0 as 100 cards and 9 tokens, and The Coin first in v0.1.0c", () => {
    const first = Object.values(readSnapshot("v0.1.0"));
    const tokens = first.filter((def) => def.token === true || (Array.isArray(def.tags) && def.tags.includes("Token")));
    expect(first.length - tokens.length).toBe(100);
    expect(tokens).toHaveLength(9);
    expect(readSnapshot("v0.1.0")["core-t-coin"]).toBeUndefined();
    expect(readSnapshot("v0.1.0c")["core-t-coin"]).toBeDefined();
  });
});
