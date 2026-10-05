// R388 (B4.2): card patches are data. `packages/cards/patches/patches.json` lists every shipped
// patch in ship order, each `<version>.json` is the whole catalog as that patch left it,
// `index.json` says in which versions each card changed, and `shipped.json` carries each patch's
// shipping commit and snapshot blob. The catalog version is the patch: `CATALOG_VERSION` is the
// newest patch's version, everywhere the string lives. A version is opaque (R105): its order is
// patches.json's, never a comparison of strings.
//
// Several patches are built at once (R646), so branches change `catalog.json` and add one
// fragment under `patches/pending/` instead of editing the history: while a fragment is pending,
// the catalog differs from the newest snapshot on exactly the claimed cards, and `patches ship`
// promotes each fragment after it merges.
//
// The history before v0.2.0 was rebuilt from `git log --follow packages/cards/catalog.json`; the
// table the brief checked on 2026-09-30 is asserted below, card by card where it names cards.

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { CATALOG, CATALOG_VERSION } from "../src/catalog-data";
import {
  INDEX_JSON,
  buildIndex,
  changedFields,
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

const ROOT = fileURLToPath(new URL("../../..", import.meta.url));
const PATCHES = readPatches();
const VERSIONS = PATCHES.map((patch) => patch.version);
const changesOf = (version: string) => PATCHES.find((patch) => patch.version === version)?.changes ?? [];
const idsOf = (version: string, kind: string): string[] =>
  changesOf(version)
    .filter((change) => change.kind === kind)
    .map((change) => change.id);

describe("R388 card patch history (B4.2)", () => {
  it("R388 lists every patch once, in the order they were made, each with its snapshot", () => {
    // Promotions only ever append (R646), so the history the file holds today can grow past
    // this prefix but never move it: assert the prefix, plus the newest patch main shipped.
    expect(VERSIONS.slice(0, 8)).toEqual([
      "v0.1.0",
      "v0.1.0b",
      "v0.1.0c",
      "v0.1.0d",
      "v0.1.1",
      "v0.2.0",
      "v0.2.4",
      "v0.2.5",
    ]);
    expect(VERSIONS).toContain("v0.2.10");
    expect(new Set(VERSIONS).size).toBe(VERSIONS.length);
    for (const patch of PATCHES) {
      expect(existsSync(snapshotPath(patch.version)), `${patch.version}.json`).toBe(true);
      expect(patch.date, patch.version).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(patch.title.length, patch.version).toBeGreaterThan(0);
      expect(patch.source.length, patch.version).toBeGreaterThan(0);
      expect(patch.notes.length, patch.version).toBeGreaterThan(0);
    }
  });

  it("R388 makes the catalog version the newest patch, and catalog.json its snapshot apart from pending fragments (R646)", () => {
    // Never a literal: `patches ship` moves the newest patch, and its pull request cannot edit tests.
    expect(CATALOG_VERSION).toBe(VERSIONS[VERSIONS.length - 1]);
    const snapshot = readSnapshot(CATALOG_VERSION);
    // Pending fragments hold the catalog ahead of the newest snapshot on exactly their claimed
    // cards (R646): reverted to the snapshot, the catalog is the snapshot. With no fragments
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

  it("R646 lists every shipped patch once in shipped.json, with the commit that shipped it and its snapshot's blob", () => {
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

  it("R388 gives Render's start command the catalog version from the patch list, so a stale dashboard value is never served", () => {
    const script = join(ROOT, "scripts/catalog-version.mjs");
    // The version a deploy serves and stamps is the newest patch, which is CATALOG_VERSION.
    expect(execFileSync(process.execPath, [script], { encoding: "utf8" })).toBe(CATALOG_VERSION);

    // render.yaml runs it, as its own command, and exports the result before `release` stamps the
    // database and before the server starts.
    const start = /^ *startCommand: *(.*)$/mu.exec(readFileSync(join(ROOT, "render.yaml"), "utf8"))?.[1] ?? "";
    const commands = start.split("&&").map((command) => command.trim());
    expect(commands[0]).toBe("CATALOG_VERSION=$(node scripts/catalog-version.mjs)");
    expect(commands[1]).toBe("export CATALOG_VERSION");
    expect(commands[2]).toContain("release");

    // A patch list that names no newest version stops the chain with nothing on stdout.
    const empty = join(mkdtempSync(join(tmpdir(), "catalog-version-")), "patches.json");
    writeFileSync(empty, "[]");
    const failed = spawnSync(process.execPath, [script, empty], { encoding: "utf8" });
    expect(failed.status).toBe(1);
    expect(failed.stdout).toBe("");
    expect(failed.stderr).toContain("names no newest version");
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

  it("R388 records patch v0.2.10: eighteen Field Spells Animated and Ivory Tower's text (issue #113)", () => {
    expect(idsOf("v0.2.10", "added")).toEqual([]);
    expect(idsOf("v0.2.10", "changed")).toEqual([
      "core-014", "core-033", "core-038", "core-065", "core-073",
      "classic-004", "classic-007", "classic-052", "classic-062", "classic-064", "classic-087",
      "classicplus-007", "classicplus-012-5", "classicplus-012-7", "classicplus-031", "classicplus-033",
      "classicplus-061", "classicplus-063", "classicplus-070", "classicplus-078",
    ]);
    const after = readSnapshot("v0.2.10");
    const keywords = (id: string): unknown => (after[id]?.["base"] as { keywords?: { kind: string }[] } | undefined)?.keywords?.[0]?.kind;
    expect(keywords("core-073")).toBe("Animated");
    expect((after["classicplus-033"]?.["base"] as { text?: string } | undefined)?.text).toBe("The first Unit you stack onto this is fused into it.");
    // Final Gambit's follow-up gained its R216 guard: the script's lines move, nothing printed does.
    expect(changesOf("v0.2.10").find((change) => change.id === "classic-052")).toMatchObject({ kind: "changed", fields: ["loc"] });
  });

  it("R388 records patch v0.2.11: aimed random casts and the Deft keyword (issue #181), pending or shipped (R646)", () => {
    // Pending until `patches ship` promotes it, then shipped: either way the patch is these five
    // cards' changes against v0.2.10's snapshot.
    const five = ["core-045", "classic-003", "classicplus-010", "classicplus-038-1", "classicplus-040"];
    const shipped = VERSIONS.includes("v0.2.11");
    const pending = readFragments().find(({ fragment }) => fragment.version === "v0.2.11")?.fragment;
    expect(shipped || pending !== undefined, "v0.2.11 is pending or shipped").toBe(true);
    if (!shipped) expect(pending?.cards).toEqual(five);
    const changes = shipped
      ? changesOf("v0.2.11")
      : diffCatalogs(readSnapshot("v0.2.10"), CATALOG as unknown as Catalog).filter((change) =>
          five.includes(change.id),
        );
    expect(changes.filter((change) => change.kind !== "changed")).toEqual([]);
    expect(changes.map((change) => change.id)).toEqual(five);
    const fieldsOf = (id: string): string[] => {
      const change = changes.find((c) => c.id === id);
      return change?.kind === "changed" ? [...change.fields].sort() : [];
    };
    expect(fieldsOf("core-045")).toEqual(["base.keywords", "base.text", "loc", "radiant.keywords", "radiant.text"]);
    expect(fieldsOf("classic-003")).toEqual(["loc"]);
    expect(fieldsOf("classicplus-010")).toEqual(["loc"]);
    expect(fieldsOf("classicplus-038-1")).toEqual(["base.text", "radiant.text"]);
    expect(fieldsOf("classicplus-040")).toEqual(["base.text", "radiant.text"]);
  });

  it("R388 records patch v0.2.13: Classic and Classic+ balance patch 1 (issue #88), pending (R646)", () => {
    // Pending until `patches ship` promotes it (R646): the fragment claims the balance cards,
    // and the catalog differs from v0.2.12's snapshot on exactly those cards.
    const fragment = readFragments().find(({ fragment }) => fragment.version === "v0.2.13")?.fragment;
    expect(fragment, "v0.2.13 is pending").toBeDefined();
    const claimed = fragment?.cards ?? [];
    const changes = diffCatalogs(readSnapshot("v0.2.12"), CATALOG as unknown as Catalog).filter((change) =>
      claimed.includes(change.id),
    );
    expect(changes.filter((change) => change.kind !== "changed")).toEqual([]);
    expect(changes.map((change) => change.id)).toEqual(claimed);
    expect(claimed).toHaveLength(52);
  });

  it("R388 records patch v0.2.12: Animated removed from eighteen Field Spells (issue #218)", () => {
    // The eighteen, in catalog order — the order a fragment's `cards` and a patch's `changes` use.
    const unanimated = [
      "core-014", "core-033", "core-038", "core-065", "core-073",
      "classic-004", "classic-007", "classic-062", "classic-064", "classic-087",
      "classicplus-007", "classicplus-012-5", "classicplus-012-7", "classicplus-031",
      "classicplus-061", "classicplus-063", "classicplus-070", "classicplus-078",
    ];
    // The undo is in the catalog either way: no stats and no Animated on either face.
    const face = (id: string) => CATALOG[id]?.base;
    expect(face("core-073")?.attack).toBeUndefined();
    expect(face("core-073")?.keywords).toEqual([]);
    // Pending, the fragment is the patch's whole record (R646); shipped, `patches ship` has
    // promoted it to the list with a snapshot of this catalog. The test holds on both sides
    // of the promotion, which cannot edit it.
    const fragment = readFragments().find(({ fragment }) => fragment.version === "v0.2.12")?.fragment;
    if (fragment !== undefined) expect(fragment.cards).toEqual(unanimated);
    else {
      expect(idsOf("v0.2.12", "added")).toEqual([]);
      expect(idsOf("v0.2.12", "removed")).toEqual([]);
      expect(idsOf("v0.2.12", "changed")).toEqual(unanimated);
    }
  });

  it("R388 records patch v0.2.4: card text pass (issue #45)", () => {
    expect(idsOf("v0.2.4", "added")).toHaveLength(0);
    expect(idsOf("v0.2.4", "removed")).toHaveLength(0);
    expect(idsOf("v0.2.4", "changed")).toHaveLength(22);
    expect(
      changesOf("v0.2.4").every(
        (change) => change.kind === "changed" && change.fields.every((f) => f === "base.text" || f === "radiant.text"),
      ),
    ).toBe(true);
  });

  it("R388 records patch v0.2.5: small set of mechanics changes (issue #149)", () => {
    expect(idsOf("v0.2.5", "added")).toHaveLength(0);
    expect(idsOf("v0.2.5", "removed")).toHaveLength(0);
    expect(idsOf("v0.2.5", "changed")).toHaveLength(22);
    const fieldsOf = (id: string): string[] => {
      const change = changesOf("v0.2.5").find((c) => c.id === id);
      return change?.kind === "changed" ? [...change.fields].sort() : [];
    };
    // Seventeen cards gain the Plague tag and nothing else.
    const plague = idsOf("v0.2.5", "changed").filter((id) => fieldsOf(id).includes("tags"));
    expect(plague).toHaveLength(17);
    expect(plague.every((id) => JSON.stringify(fieldsOf(id)) === JSON.stringify(["tags"]))).toBe(true);
    // The mechanics: Exile's threshold, Joro's Spell-only text, Blade Storm's Whirlwind cast and
    // refs, Adaptive Growth's numbers, Chaos Machine's other-card text. A face counts as changed
    // when its printed words move, even when only a param's value moved (Exile's base face,
    // Adaptive Growth's Radiant face).
    expect(fieldsOf("classic-010")).toEqual(["base.text", "params"]);
    expect(fieldsOf("classic-033")).toEqual(["base.text", "radiant.text"]);
    expect(fieldsOf("classicplus-032-3")).toEqual(["base.text", "refs"]);
    expect(fieldsOf("classicplus-050")).toEqual(["base.text", "loc", "params", "radiant.text"]);
    expect(fieldsOf("classicplus-070")).toEqual(["base.text", "loc", "radiant.text"]);
    const before = readSnapshot("v0.2.4");
    const after = readSnapshot("v0.2.5");
    const param = (snapshot: Catalog, id: string, key: string): unknown =>
      (snapshot[id]?.["params"] as { key: string; base: number; radiant: number }[] | undefined)?.find(
        (p) => p.key === key,
      );
    expect(param(before, "classic-010", "threshold")).toMatchObject({ base: 1, radiant: 3 });
    expect(param(after, "classic-010", "threshold")).toMatchObject({ base: 2, radiant: 3 });
    expect(param(before, "classicplus-050", "debuff")).toMatchObject({ base: 4, radiant: 4 });
    expect(param(after, "classicplus-050", "debuff")).toMatchObject({ base: 2, radiant: 3 });
  });

  it("R388 changedFields compares a face's text as it prints, its params filled in", () => {
    const face = (threshold: number, tail: string): Record<string, unknown> => ({
      params: [{ key: "threshold", base: threshold, radiant: 3 }],
      base: { keywords: [], text: `Counter a ({threshold}) Cost or less card. ${tail}` },
      radiant: { keywords: [], text: "No placeholder here." },
    });
    // Only the value moved: the template is untouched, but the printed base face is reworded.
    expect(changedFields(face(1, ""), face(2, ""))).toEqual(["params", "base.text"]);
    // The template moved: the raw and the printed faces differ together.
    expect(changedFields(face(1, "Draw 1."), face(1, "Draw 2."))).toEqual(["base.text"]);
    // Nothing moved: no fields.
    expect(changedFields(face(1, ""), face(1, ""))).toEqual([]);
  });

  it("R388 indexes each card by the versions that added or changed it", () => {
    const index = buildIndex(PATCHES);
    expect(index["core-t-coin"]?.[0]).toBe("v0.1.0c");
    expect(index["core-t-ghoul"]?.[0]).toBe("v0.1.1");
    expect(index["classic-001"]?.[0]).toBe("v0.2.0");
    expect(index["core-016"]).toContain("v0.2.0");
    // Every catalog entry was added by some patch, or is claimed by a pending fragment — which is
    // not a shipped patch yet, so the index does not name it (R646).
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
