// R375: a card's history, `src/history.ts`, against the real snapshots and against made-up ones.
//
// The real ones are `packages/cards/patches/` (held to git and catalog.json by patches.test.ts):
// the cases issue #39 names, read over the versions up to v0.1.1 (the script's BACKFILL) so a later
// patch cannot move them, then every catalog entry over every version. The made-up ones reach what
// the real history has not needed yet: a removal, a field the history has no name for, an embiggen
// cost.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { CardDef, CardDefs } from "@jackioh/shared";
import { CATALOG_IDS } from "../src/catalog-data";
import {
  RADIANT_TEXT_IN_FULL_SINCE,
  cardHistory,
  historyVersions,
  newestPatch,
  patchCards,
  type CardChange,
  type HistoryEntry,
  type Patch,
  type PatchCards,
  type Snapshots,
} from "../src/history";
import { BACKFILL, CHANGES_PATH, PATCHES_PATH, snapshotPath } from "../scripts/patches";

const PATCHES = JSON.parse(readFileSync(PATCHES_PATH, "utf8")) as Patch[];
const CHANGES = JSON.parse(readFileSync(CHANGES_PATH, "utf8")) as Record<string, PatchCards>;
const SNAPSHOTS: Snapshots = Object.fromEntries(
  PATCHES.map((patch) => [patch.version, JSON.parse(readFileSync(snapshotPath(patch.version), "utf8")) as CardDefs]),
);

/** The history as git has it: v0.1.0 to v0.1.1. */
const RECORDED: readonly Patch[] = PATCHES.slice(0, BACKFILL.length);

/** A card's history up to v0.1.1, which no later patch changes. */
function historyOf(id: string): HistoryEntry[] {
  return cardHistory(id, RECORDED, SNAPSHOTS);
}

/** A card's whole history, every version patches.json lists. */
function wholeHistoryOf(id: string): HistoryEntry[] {
  return cardHistory(id, PATCHES, SNAPSHOTS);
}

function entryAt(id: string, version: string): HistoryEntry {
  const entry = historyOf(id).find((candidate) => candidate.patch.version === version);
  if (entry === undefined) throw new Error(`${id} has no ${version} entry`);
  return entry;
}

describe("R375 a card's history, from the real snapshots", () => {
  it("R375 gives #44 True Strike three entries: v0.1.0, v0.1.0-r3 and v0.1.1", () => {
    const history = historyOf("core-044");
    expect(history.map((entry) => [entry.patch.version, entry.kind])).toEqual([
      ["v0.1.0", "created"],
      ["v0.1.0-r3", "changed"],
      ["v0.1.1", "changed"],
    ]);
  });

  it("R375 marks a Radiant text shorthand before v0.1.0-r3, so its r3 entry is the text written out in full", () => {
    expect(RADIANT_TEXT_IN_FULL_SINCE).toBe("v0.1.0-r3");
    expect(entryAt("core-044", "v0.1.0").radiantShorthand).toBe(true);
    expect(entryAt("core-044", "v0.1.0").def.radiant.text).toBe("9");
    expect(entryAt("core-044", "v0.1.0-r3").changes).toEqual<CardChange[]>([
      {
        kind: "text",
        face: "radiant",
        before: "9",
        after: "Deal 9 damage to a target, ignoring Armor; exile this",
        beforeShorthand: true,
        afterShorthand: false,
      },
    ]);
    const v011 = entryAt("core-044", "v0.1.1");
    expect(v011.radiantShorthand).toBe(false);
    expect(v011.changes.map((change) => (change.kind === "field" ? change.field : `${change.face} text`))).toEqual([
      "keywords",
      "base text",
      "radiantKeywords",
      "radiant text",
    ]);
    expect(v011.changes[0]).toEqual({ kind: "field", field: "keywords", before: "none", after: "Pierce" });
  });

  it("R375 makes #68's v0.1.0-r1 entry the name change", () => {
    expect(entryAt("core-068", "v0.1.0-r1").changes).toEqual([
      { kind: "field", field: "name", before: "Twisted Sourcerer", after: "Twisted Sorcerer" },
    ]);
    expect(entryAt("core-068", "v0.1.0-r1").def.name).toBe("Twisted Sorcerer");
  });

  it("R375 creates The Coin in v0.1.0-r2 and the Ghoul Token in v0.1.1", () => {
    // The Coin's text then gained its full stop in v0.1.1.
    expect(historyOf("core-t-coin").map((entry) => [entry.patch.version, entry.kind])).toEqual([
      ["v0.1.0-r2", "created"],
      ["v0.1.1", "changed"],
    ]);
    expect(historyOf("core-t-ghoul").map((entry) => [entry.patch.version, entry.kind])).toEqual([["v0.1.1", "created"]]);
  });

  it("R375 shows #85's v0.1.1 cost 1 → 2", () => {
    expect(entryAt("core-085", "v0.1.1").changes).toContainEqual({ kind: "field", field: "cost", before: "1", after: "2" });
  });

  it("R375 gives #11 Tempo Timmy only its creation entry", () => {
    expect(historyOf("core-011").map((entry) => [entry.patch.version, entry.kind])).toEqual([["v0.1.0", "created"]]);
  });

  it("R375 gives every catalog entry exactly one creation entry, first, and no removal", () => {
    expect(RECORDED.map((patch) => patch.version)).toEqual(["v0.1.0", "v0.1.0-r1", "v0.1.0-r2", "v0.1.0-r3", "v0.1.1"]);
    for (const id of CATALOG_IDS) {
      const history = wholeHistoryOf(id);
      expect(history.filter((entry) => entry.kind === "created"), id).toHaveLength(1);
      expect(history[0]?.kind, id).toBe("created");
      expect(history.some((entry) => entry.kind === "removed"), id).toBe(false);
      expect(history.filter((entry) => entry.kind === "changed").every((entry) => entry.changes.length > 0), id).toBe(true);
    }
  });

  it("R375 ends every card's history at the card as catalog.json prints it", () => {
    const newest = newestPatch(PATCHES).version;
    for (const id of CATALOG_IDS) {
      const history = wholeHistoryOf(id);
      const last = history[history.length - 1];
      expect(last?.def, id).toEqual(SNAPSHOTS[newest]?.[id]);
    }
  });

  it("R375 counts, off changes.json, exactly the versions each card's history has", () => {
    for (const id of CATALOG_IDS) {
      expect(historyVersions(id, PATCHES, CHANGES), id).toEqual(wholeHistoryOf(id).map((entry) => entry.patch.version));
    }
  });

  it("R375 prints each field's change as it reads: stats, tags, keywords and the cards a text names", () => {
    expect(entryAt("core-001", "v0.1.1").changes).toContainEqual({ kind: "field", field: "stats", before: "0/8", after: "0/7" });
    expect(entryAt("core-001", "v0.1.1").changes).toContainEqual({ kind: "field", field: "radiantStats", before: "0/16", after: "0/14" });
    expect(entryAt("core-013", "v0.1.0-r3").changes).toContainEqual({ kind: "field", field: "tags", before: "none", after: "Jlockeed" });
    expect(entryAt("core-003", "v0.1.0-r1").changes).toContainEqual({
      kind: "field",
      field: "keywords",
      before: "Divine Shield, Reborn",
      after: "Taunt, Divine Shield, Reborn",
    });
    // Two shorthand texts are compared as they stand.
    expect(entryAt("core-003", "v0.1.0-r1").changes).toContainEqual(
      expect.objectContaining({ kind: "text", face: "radiant", beforeShorthand: true, afterShorthand: true }),
    );
    // R279's `refs` arrived with v0.1.0-r3, named by that version's card names.
    expect(entryAt("core-003", "v0.1.0-r3").changes).toEqual([
      { kind: "field", field: "refs", before: "none", after: "Right-house defender" },
    ]);
    expect(entryAt("core-050", "v0.1.1").changes).toContainEqual({
      kind: "field",
      field: "name",
      before: "Kpop Fanatic",
      after: "K-Pop Fanatic",
    });
  });
});

/* ------------------------------------------------------------------------------------------- *
 * made-up snapshots
 * ------------------------------------------------------------------------------------------- */

function patch(version: string): Patch {
  return { version, date: "2026-10-01", title: version, commits: [], sources: [], reconstructed: false, notes: "" };
}

function card(id: string, overrides: Partial<CardDef> = {}): CardDef {
  return {
    id,
    index: id,
    name: `Card ${id}`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 2,
    base: { attack: 1, health: 1, keywords: [], text: "Draw a card." },
    radiant: { attack: 2, health: 2, keywords: [], text: "Draw 2 cards." },
    ...overrides,
  };
}

describe("R375 a card's history, from made-up snapshots", () => {
  const a = patch("a");
  const b = patch("b");
  const c = patch("c");

  it("R375 records a card leaving the catalog as a removal, and its return as a creation", () => {
    const snapshots: Snapshots = { a: { x: card("x") }, b: {}, c: { x: card("x") } };
    expect(cardHistory("x", [a, b, c], snapshots).map((entry) => [entry.patch.version, entry.kind])).toEqual([
      ["a", "created"],
      ["b", "removed"],
      ["c", "created"],
    ]);
    expect(cardHistory("x", [a, b, c], snapshots)[1]?.def).toEqual(card("x"));
    expect(patchCards([a, b, c], snapshots)).toEqual({
      a: { created: ["x"], changed: [], removed: [] },
      b: { created: [], changed: [], removed: ["x"] },
      c: { created: ["x"], changed: [], removed: [] },
    });
  });

  it("R375 reports a field it has no name for under its own key, so no change goes unreported", () => {
    const snapshots: Snapshots = {
      a: { x: card("x") },
      b: { x: { ...card("x", { index: "7", radiantFallback: true }), base: { ...card("x").base, loc: 12 } as CardDef["base"] } },
    };
    expect(cardHistory("x", [a, b], snapshots)[1]?.changes).toEqual([
      { kind: "field", field: "index", before: '"x"', after: '"7"' },
      { kind: "field", field: "radiantFallback", before: "none", after: "true" },
      { kind: "field", field: "base.loc", before: "none", after: "12" },
    ]);
  });

  it("R375 prints an embiggen cost and X, and a face without stats as none", () => {
    const snapshots: Snapshots = {
      a: { x: card("x", { cost: "X" }) },
      b: { x: card("x", { cost: { base: 2, embiggen: 4 }, type: "Spell", base: { keywords: [], text: "Draw a card." } }) },
    };
    expect(cardHistory("x", [a, b], snapshots)[1]?.changes).toEqual([
      { kind: "field", field: "cost", before: "X", after: "2 (Paid 4)" },
      { kind: "field", field: "type", before: "Unit", after: "Spell" },
      { kind: "field", field: "stats", before: "1/1", after: "none" },
    ]);
  });

  it("R375 sees no change where only the order of an entry's keys differs", () => {
    const x = card("x");
    const { base, radiant, ...rest } = x;
    const reordered: CardDef = {
      radiant: { text: radiant.text, keywords: radiant.keywords, health: radiant.health, attack: radiant.attack },
      base,
      ...rest,
    };
    expect(Object.keys(reordered)).not.toEqual(Object.keys(x));
    expect(cardHistory("x", [a, b], { a: { x }, b: { x: reordered } })).toHaveLength(1);
  });

  it("R375 marks no Radiant text shorthand in a list without the version that wrote them out", () => {
    const history = cardHistory("x", [a, b], { a: { x: card("x") }, b: { x: card("x", { radiant: { ...card("x").radiant, text: "Draw 3 cards." } }) } });
    expect(history.every((entry) => !entry.radiantShorthand)).toBe(true);
    expect(history[1]?.changes).toEqual([
      { kind: "text", face: "radiant", before: "Draw 2 cards.", after: "Draw 3 cards.", beforeShorthand: false, afterShorthand: false },
    ]);
  });

  it("R375 names the version a missing snapshot belongs to, and refuses an empty patch list", () => {
    expect(() => cardHistory("x", [a, b], { a: {} })).toThrow(/no snapshot for b/);
    expect(() => newestPatch([])).toThrow(/lists no patch/);
    expect(newestPatch([a, b])).toBe(b);
  });

  it("R375 counts a card only under the versions changes.json lists it in", () => {
    const changes = { a: { created: ["x"], changed: [], removed: [] }, b: { created: [], changed: ["y"], removed: [] } };
    expect(historyVersions("x", [a, b, c], changes)).toEqual(["a"]);
    expect(historyVersions("z", [a, b, c], changes)).toEqual([]);
  });
});
