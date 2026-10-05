// R388, R507: a patch's cards as the Patch notes page sorts them, and one card's history, newest
// first. Order always comes from patches.json (R105 as B4.2 rewrites it): the fixture versions sort
// the wrong way as strings.

import type { CardDef } from "@jackioh/shared";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import { describe, expect, it } from "vitest";

import { FIXTURE_INDEX, FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, V1, V2, V3, fixtureSource } from "./fixtures.ts";
import {
  cardHistory,
  currentDef,
  nameMatches,
  newestFirst,
  patchCards,
  unchangedSince,
  versionBefore,
  versionsForCard,
} from "./history.ts";
import { countsLine } from "./PatchNotes.tsx";
import { realPatchSource, type Patch, type Snapshot } from "./source.ts";

function patch(patches: readonly Patch[], version: string): Patch {
  const found = patches.find((candidate) => candidate.version === version);
  if (found === undefined) throw new Error(`no patch ${version}`);
  return found;
}

function snapshot(version: string): Snapshot {
  const found = FIXTURE_SNAPSHOTS[version];
  if (found === undefined) throw new Error(`no snapshot ${version}`);
  return found;
}

function snapshotsOf(versions: readonly string[]): Map<string, Snapshot | null> {
  return new Map(versions.map((version) => [version, FIXTURE_SNAPSHOTS[version] ?? null]));
}

describe("R388 patch order is the file's", () => {
  it("R388 newest first is patches.json read backwards, never a sort of version strings", () => {
    expect(newestFirst(FIXTURE_PATCHES).map((entry) => entry.version)).toEqual([V3, V2, V1]);
    expect([V1, V2, V3].sort()).not.toEqual([V1, V2, V3]);
  });

  it("R388 the version before a patch is the entry before it in the file", () => {
    expect(versionBefore(FIXTURE_PATCHES, V1)).toBeNull();
    expect(versionBefore(FIXTURE_PATCHES, V2)).toBe(V1);
    expect(versionBefore(FIXTURE_PATCHES, V3)).toBe(V2);
    expect(versionBefore(FIXTURE_PATCHES, "v0.9.9")).toBeNull();
  });
});

describe("R507 a patch's cards", () => {
  it("R507 sorts a patch into faces, data-only names, added names by set with tokens apart, and removals", () => {
    const cards = patchCards(patch(FIXTURE_PATCHES, V3), snapshot(V3), snapshot(V2));
    expect(cards.changed.map((delta) => delta.id)).toEqual(["core-002"]);
    expect(cards.dataOnly.map((delta) => delta.id)).toEqual(["core-004"]);
    expect(cards.added.map((group) => [group.label, group.cards.map((def) => def.name)])).toEqual([
      ["Classic", ["Fresh Face"]],
      ["Classic+", ["Pancake Pal"]],
      ["Classic+ tokens", ["Syrup Token"]],
    ]);
    expect(cards.removed).toEqual([]);

    const second = patchCards(patch(FIXTURE_PATCHES, V2), snapshot(V2), snapshot(V1));
    expect(second.changed.map((delta) => delta.id)).toEqual(["core-001"]);
    expect(second.removed.map((def) => def.name)).toEqual(["Old Relic"]);
  });

  it("R507 the first patch, with no snapshot before it, adds every card it records", () => {
    const cards = patchCards(patch(FIXTURE_PATCHES, V1), snapshot(V1), null);
    expect(cards.added.map((group) => [group.label, group.cards.length])).toEqual([["Core", 4]]);
    expect(cards.changed).toEqual([]);
  });

  it("R507 a name filter matches every word of the query, case aside", () => {
    expect(nameMatches("Snom Bunny Mind Control", "")).toBe(true);
    expect(nameMatches("Snom Bunny Mind Control", "mind snom")).toBe(true);
    expect(nameMatches("Snom Bunny Mind Control", "BUNNY")).toBe(true);
    expect(nameMatches("Snom Bunny Mind Control", "bunny cube")).toBe(false);
  });

  it("R507 the real v0.2.0 shows as faces the changed cards whose recorded changes a face prints, the rest by name, and 206 added cards by set", async () => {
    const patches = await realPatchSource.patches();
    const [after, before] = await Promise.all([realPatchSource.snapshot("v0.2.0"), realPatchSource.snapshot("v0.1.1")]);
    if (after === null || before === null) throw new Error("expected the v0.2.0 and v0.1.1 snapshots");
    const record = patch(patches, "v0.2.0");
    const cards = patchCards(record, after, before);
    const unprinted = new Set(["loc", "refs", "params"]);
    const changed = record.changes.filter((change) => change.kind === "changed");
    const printed = changed.filter((change) => (change.fields ?? []).some((name) => !unprinted.has(name))).map((change) => change.id);
    expect(cards.changed.map((delta) => delta.id)).toEqual(printed);
    expect(cards.changed.map((delta) => delta.id)).toEqual(expect.arrayContaining(["core-065", "core-021", "core-095"]));
    expect(cards.dataOnly.map((delta) => delta.id)).toEqual(changed.map((change) => change.id).filter((id) => !printed.includes(id)));
    expect(cards.dataOnly.length).toBeGreaterThan(0);
    expect(cards.added.map((group) => [group.label, group.cards.length])).toEqual([
      ["Classic", 90],
      ["Classic+", 78],
      ["Classic+ tokens", 38],
    ]);
  });

  it("R507 the real v0.1.0 adds Core's 100 cards and its tokens apart", async () => {
    const patches = await realPatchSource.patches();
    const first = patches[0];
    if (first === undefined) throw new Error("expected a first patch");
    const after = await realPatchSource.snapshot(first.version);
    if (after === null) throw new Error("expected the first snapshot");
    const cards = patchCards(first, after, null);
    expect(cards.added.map((group) => [group.label, group.cards.length])).toEqual([
      ["Core", 100],
      ["Core tokens", 9],
    ]);
  });
});

describe("R674 Glitch in the patch notes", () => {
  it("R674 a patch that records Glitch neither lists nor counts it", () => {
    const glitch: CardDef = {
      id: GLITCH_DEF_ID,
      index: "T-glitch",
      name: "Glitch",
      set: "Classic",
      type: "Spell",
      tags: ["Token"],
      rarity: "Token",
      token: true,
      cost: 0,
      base: { keywords: [], text: "" },
      radiant: { keywords: [], text: "" },
    };
    const record: Patch = {
      version: "v9.9.9",
      date: "2026-10-05",
      title: "Glitch",
      source: "#170",
      notes: "",
      changes: [{ kind: "added", id: GLITCH_DEF_ID, name: "Glitch" }],
    };
    const cards = patchCards(record, { [GLITCH_DEF_ID]: glitch }, {});
    expect(cards.added).toEqual([]);
    expect(countsLine(record)).toBe("");
  });
});

describe("R388 one card's history", () => {
  it("R388 lists each patch that changed the card, newest first, diffed against the patch before", () => {
    const versions = versionsForCard("core-001", FIXTURE_PATCHES, FIXTURE_INDEX);
    expect(versions).toEqual([V1, V2]);
    const entries = cardHistory("core-001", FIXTURE_PATCHES, FIXTURE_INDEX, snapshotsOf(versions));
    expect(entries.map((entry) => [entry.patch.version, entry.delta.kind])).toEqual([
      [V2, "changed"],
      [V1, "added"],
    ]);
    expect(entries[0]?.def.cost).toBe(3);
    expect(unchangedSince(entries)).toBeNull();
  });

  it("R388 reads the snapshot before each change, even of a patch that did not touch the card", () => {
    expect(versionsForCard("core-002", FIXTURE_PATCHES, FIXTURE_INDEX)).toEqual([V1, V2, V3]);
  });

  it("R388 a removal shows the card as it stood before it was removed", () => {
    const versions = versionsForCard("core-003", FIXTURE_PATCHES, FIXTURE_INDEX);
    const entries = cardHistory("core-003", FIXTURE_PATCHES, FIXTURE_INDEX, snapshotsOf(versions));
    expect(entries.map((entry) => [entry.patch.version, entry.delta.kind, entry.def.name])).toEqual([
      [V2, "removed", "Old Relic"],
      [V1, "added", "Old Relic"],
    ]);
  });

  it("R388 a card whose only patch added it is unchanged since that patch", () => {
    const versions = versionsForCard("classic-001", FIXTURE_PATCHES, FIXTURE_INDEX);
    const entries = cardHistory("classic-001", FIXTURE_PATCHES, FIXTURE_INDEX, snapshotsOf(versions));
    expect(unchangedSince(entries)).toBe(V3);
  });

  it("R388 the card as it stands now: the newest snapshot that holds it, or the last before its removal", async () => {
    const source = fixtureSource();
    expect((await currentDef("core-002", FIXTURE_PATCHES, FIXTURE_INDEX, source.snapshot))?.cost).toEqual({ base: 2, embiggen: 4 });
    expect((await currentDef("core-003", FIXTURE_PATCHES, FIXTURE_INDEX, source.snapshot))?.name).toBe("Old Relic");
    expect(await currentDef("core-999", FIXTURE_PATCHES, FIXTURE_INDEX, source.snapshot)).toBeNull();
  });

  it("R388 the real history of Right-house defender (core-003) runs v0.2.0, v0.1.1, v0.1.0d, v0.1.0b, v0.1.0", async () => {
    const patches = await realPatchSource.patches();
    const index = await realPatchSource.index();
    const versions = versionsForCard("core-003", patches, index);
    const snapshots = new Map<string, Snapshot | null>();
    for (const version of versions) snapshots.set(version, await realPatchSource.snapshot(version));
    const entries = cardHistory("core-003", patches, index, snapshots);
    // Promotions only append to a card's index, so in this newest-first list they prepend:
    // the five versions the brief checked are the tail.
    expect(entries.map((entry) => entry.patch.version).slice(-5)).toEqual(["v0.2.0", "v0.1.1", "v0.1.0d", "v0.1.0b", "v0.1.0"]);
    expect(entries.at(-1)?.delta.kind).toBe("added");
  });

  it("R388 a real Classic card is unchanged since the patch that added it", async () => {
    const patches = await realPatchSource.patches();
    const index = await realPatchSource.index();
    const versions = versionsForCard("classic-001", patches, index);
    expect(versions.slice(0, 2)).toEqual(["v0.1.1", "v0.2.0"]);
    const snapshots = new Map<string, Snapshot | null>();
    for (const version of versions) snapshots.set(version, await realPatchSource.snapshot(version));
    // Added in v0.2.0 and unchanged since — until a promoted patch touches it again (R646),
    // which turns "unchanged since" off by adding a second history entry.
    expect(unchangedSince(cardHistory("classic-001", patches, index, snapshots))).toBe(
      (index["classic-001"] ?? []).length === 1 ? "v0.2.0" : null,
    );
  });
});
