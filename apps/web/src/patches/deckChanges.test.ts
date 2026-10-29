// R2097000: a deck holding a card the newest patch changed says so, until it is saved under it.

import { describe, expect, it } from "vitest";

import type { CardDef } from "@jackioh/shared";

import { sourceFromData, type Snapshot } from "./source.ts";
import { diffCard } from "./diff.ts";
import { deckChanges, loadNewestChanges, newestChanges } from "./deckChanges.ts";

function card(id: string, fields: Partial<CardDef> = {}): CardDef {
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
    base: { attack: 2, health: 3, keywords: [], text: "" },
    radiant: { attack: 4, health: 6, keywords: [], text: "" },
    ...fields,
  };
}

const ids = ["core-001", "core-002", "core-003", "core-004", "core-005"];

function snapshot(extra: Readonly<Record<string, Partial<CardDef>>>): Snapshot {
  return Object.fromEntries(ids.map((id) => [id, card(id, extra[id] ?? {})]));
}

const BEFORE = snapshot({ "core-003": { loc: 10 }, "core-004": { flavor: "Fire" } });
const AFTER = snapshot({
  "core-001": { cost: 1 },
  "core-002": { base: { attack: 2, health: 3, keywords: [], text: "Draw 1." } },
  "core-003": { loc: 12 },
  "core-004": { flavor: "Dark" },
});

const OLD = "v0-old";
const NEW = "v0-new";

const NEWEST = {
  version: NEW,
  date: "2026-10-01",
  title: "The newest patch",
  source: "Test",
  notes: "Test",
  changes: [
    { id: "core-001", name: "Card core-001", kind: "changed" as const, fields: ["cost"] },
    { id: "core-002", name: "Card core-002", kind: "changed" as const, fields: ["base.text"] },
    { id: "core-003", name: "Card core-003", kind: "changed" as const, fields: ["loc"] },
    { id: "core-004", name: "Card core-004", kind: "changed" as const, fields: ["flavor"] },
  ],
};

const OLD_PATCH = {
  version: OLD,
  date: "2026-09-01",
  title: "The older patch",
  source: "Test",
  notes: "Test",
  changes: [] as typeof NEWEST.changes,
};

function changes() {
  return newestChanges(NEWEST, AFTER, BEFORE);
}

describe("R2097000 the newest patch's changed cards", () => {
  it("R2097000 the newest patch's changed cards are every card it changed, printed or not", () => {
    const read = changes();
    expect(read.version).toBe(NEW);
    expect([...read.cards.keys()]).toEqual(["core-001", "core-002", "core-003"]);
    expect(read.cards.get("core-001")?.changes.map((change) => change.field)).toEqual(["cost"]);
  });

  it("R2097000 a change to flavor alone is no change (R1437000)", () => {
    expect(diffCard(BEFORE["core-001"], AFTER["core-001"])).not.toBeNull();
    expect(diffCard(BEFORE["core-004"], AFTER["core-004"])).toBeNull();
    expect(changes().cards.has("core-004")).toBe(false);
    const costlier = card("core-004", { cost: 1, flavor: "Dark" });
    const delta = diffCard(BEFORE["core-004"], costlier);
    expect(delta?.kind).toBe("changed");
    expect(delta?.kind === "changed" ? delta.changes.map((change) => change.field) : []).toEqual(["cost"]);
  });

  it("R2097000 a deck's changed cards are exactly those it holds, once each", () => {
    const read = changes();
    const held = deckChanges({ cards: ["core-005", "core-002", "core-004", "core-001", "core-001"], catalogVersion: OLD }, read);
    expect(held.map((delta) => delta.id)).toEqual(["core-001", "core-002"]);
    expect(deckChanges({ cards: ["core-004", "core-005"], catalogVersion: OLD }, read)).toEqual([]);
  });

  it("R2097000 a deck saved under the newest patch has none; one with no stamp has them", () => {
    const read = changes();
    expect(deckChanges({ cards: ["core-001", "core-002"], catalogVersion: NEW }, read)).toEqual([]);
    expect(deckChanges({ cards: ["core-001", "core-002"] }, read).map((delta) => delta.id)).toEqual(["core-001", "core-002"]);
  });

  it("R2097000 the changes load only while the newest patch is the catalog shown", async () => {
    const data = { index: {}, snapshots: { [OLD]: BEFORE, [NEW]: AFTER } };
    const newest = sourceFromData({ patches: [OLD_PATCH, NEWEST], ...data });
    expect((await loadNewestChanges(newest, NEW))?.cards.size).toBe(3);
    expect(await loadNewestChanges(newest, OLD)).toBeNull();
    expect(await loadNewestChanges(sourceFromData({ patches: [], ...data }), NEW)).toBeNull();
    const missingNew = sourceFromData({ patches: [OLD_PATCH, NEWEST], index: {}, snapshots: { [OLD]: BEFORE } });
    expect(await loadNewestChanges(missingNew, NEW)).toBeNull();
  });
});
