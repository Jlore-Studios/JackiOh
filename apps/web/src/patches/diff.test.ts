// R388: what a patch changed in a card, from hand-made fixtures (fixtures.ts), then the real history
// (crates/cards/patches/).

import { describe, expect, it } from "vitest";

import type { CardDef } from "@jackioh/shared";

import { wordDiff } from "../cards/radiantDiff.ts";
import {
  changedWords,
  costText,
  dataOnly,
  diffCard,
  faceShown,
  statsText,
  type CardDelta,
  type FieldChange,
  type TextChange,
} from "./diff.ts";
import { FIXTURE_SNAPSHOTS, V1, V2, V3, fixtureDef } from "./fixtures.ts";
import { versionBefore } from "./history.ts";
import { realPatchSource, type PatchChange, type Snapshot } from "./source.ts";

function changed(delta: CardDelta | null): readonly FieldChange[] {
  if (delta?.kind !== "changed") throw new Error(`expected a changed card, got ${delta?.kind ?? "nothing"}`);
  return delta.changes;
}

function field(changes: readonly FieldChange[], name: string): FieldChange {
  const found = changes.find((change) => change.field === name);
  if (found === undefined) throw new Error(`no change to ${name} in ${changes.map((change) => change.field).join(", ")}`);
  return found;
}

function text(changes: readonly FieldChange[], name: "base.text" | "radiant.text"): TextChange {
  const found = field(changes, name);
  if (found.kind !== "text") throw new Error(`${name} is not a text change`);
  return found;
}

/** patches.json names the raw fields; the diff reads a face's attack, health and X stats as its stats. */
function asDiffField(name: string): string {
  return name.replace(/^(base|radiant)\.(attack|health|xStats)$/, "$1.stats");
}

/** R388: a record and a card's diff agree: its kind, and for a changed card exactly the fields it names. */
function expectRecorded(record: PatchChange, delta: CardDelta | null, at: string): void {
  expect(delta?.kind, at).toBe(record.kind);
  if (delta?.kind === "changed") {
    expect(new Set(delta.changes.map((change) => change.field)), at).toEqual(new Set((record.fields ?? []).map(asDiffField)));
  }
}

async function real(version: string): Promise<Snapshot> {
  const snapshot = await realPatchSource.snapshot(version);
  if (snapshot === null) throw new Error(`no snapshot ${version}`);
  return snapshot;
}

function defIn(snapshot: Snapshot, id: string): CardDef {
  const def = snapshot[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

describe("R388 a card's changes, from two snapshots", () => {
  it("R388 writes a cost as card text does (R432): numbers, X and embiggen", () => {
    expect(costText(3)).toBe("(3) Cost");
    expect(costText("X")).toBe("(X) Cost");
    expect(costText({ base: 2, embiggen: 4 })).toBe("(2) Cost, Embiggened (4)");
  });

  it("R388 writes stats as attack/health, X stats in brackets, and nothing for a face with none", () => {
    expect(statsText({ attack: 2, health: 3, keywords: [], text: "" })).toBe("2/3");
    expect(statsText({ attack: 0, health: 0, xStats: { attack: 3, health: 3 }, keywords: [], text: "" })).toBe("[3X/3X]");
    expect(statsText({ keywords: [], text: "Deal 1 damage." })).toBe("");
  });

  it("R388 reports a new cost, new stats on each face and both faces' new words, in that order", () => {
    const changes = changed(diffCard(fixtureDef(V1, "core-001"), fixtureDef(V2, "core-001")));
    expect(changes.map((change) => change.field)).toEqual(["cost", "base.stats", "radiant.stats", "base.text", "radiant.text"]);
    expect(field(changes, "cost")).toMatchObject({ label: "Cost", before: "(2) Cost", after: "(3) Cost" });
    expect(field(changes, "base.stats")).toMatchObject({ label: "Stats", before: "2/3", after: "3/3" });
    expect(field(changes, "radiant.stats")).toMatchObject({ label: "Radiant stats", before: "4/6", after: "6/6" });
    expect(changedWords(text(changes, "base.text"))).toEqual({ added: ["Draw 1"], removed: [] });
    expect(changedWords(text(changes, "radiant.text"))).toEqual({ added: ["Draw 2"], removed: [] });
  });

  it("R388 fills {key} params before diffing, so only the words that print differently are marked", () => {
    const changes = changed(diffCard(fixtureDef(V2, "core-002"), fixtureDef(V3, "core-002")));
    const base = text(changes, "base.text");
    expect(base.after).toBe("Deal 4 damage. Your Spells cost (1) less.");
    expect(changedWords(base)).toEqual({ added: ["4", "Your"], removed: ["3", "this turn"] });
    // The Radiant number is 6 before and after: the placeholder alone changes nothing.
    expect(changedWords(text(changes, "radiant.text"))).toEqual({ added: ["Your"], removed: ["this turn"] });
    expect(field(changes, "cost")).toMatchObject({ before: "(1) Cost", after: "(2) Cost, Embiggened (4)" });
    expect(field(changes, "radiant.keywords")).toMatchObject({ label: "Radiant keywords", before: "none", after: "Pierce" });
    expect(field(changes, "params")).toMatchObject({ before: "none", after: "damage 4, Radiant 6, step 1, at least 1, more is better" });
    expect(changes.some((change) => change.field === "base.keywords")).toBe(false);
  });

  it("R388 lists a face's granted ability (ME-GRANT) that changes, its {key} numbers filled in", () => {
    const before = fixtureDef(V3, "core-002");
    const grant = (text: string): CardDef["base"] => ({ ...before.base, grants: { dealt: text } });
    const changes = changed(diffCard({ ...before, base: grant("Death: Deal {damage} damage.") }, { ...before, base: grant("Death: Deal {damage} damage twice.") }));
    expect(changes).toEqual([{ kind: "value", field: "base.grants", label: "Granted ability", before: "Death: Deal 4 damage.", after: "Death: Deal 4 damage twice." }]);
    expect(diffCard(before, { ...before, base: grant("Death: Deal {damage} damage.") })).toMatchObject({ kind: "changed", changes: [{ field: "base.grants", before: "none" }] });
  });

  it("R388 says added, removed or nothing when a snapshot lacks the card or both hold it alike", () => {
    expect(diffCard(undefined, fixtureDef(V3, "classic-001"))).toMatchObject({ kind: "added", id: "classic-001" });
    expect(diffCard(fixtureDef(V1, "core-003"), undefined)).toMatchObject({ kind: "removed", id: "core-003" });
    expect(diffCard(fixtureDef(V1, "core-004"), fixtureDef(V2, "core-004"))).toBeNull();
    expect(diffCard(undefined, undefined)).toBeNull();
  });

  it("R388 a change only to what a face does not print (lines of code) is data only", () => {
    const delta = diffCard(fixtureDef(V2, "core-004"), fixtureDef(V3, "core-004"));
    expect(changed(delta)).toEqual([{ kind: "value", field: "loc", label: "Lines of code", before: "not recorded", after: "12" }]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
    const knight = diffCard(fixtureDef(V1, "core-001"), fixtureDef(V2, "core-001"));
    expect(knight !== null && dataOnly(knight)).toBe(false);
  });

  it("R388 names the cards a text names by their names in each snapshot", () => {
    const pal = fixtureDef(V3, "classicplus-012");
    const before = { ...pal, refs: [] };
    const changes = changed(diffCard(before, pal, { after: FIXTURE_SNAPSHOTS[V3] }));
    expect(field(changes, "refs")).toMatchObject({ label: "Cards it names", before: "none", after: "Syrup Token" });
  });

  it("R388 R2511001 a copy's Core card (snapshotOf) is data only, under the name patches ship records, the card by its name", () => {
    const golem = fixtureDef(V3, "core-004");
    const copy = { ...golem, snapshotOf: "core-002" };
    const delta = diffCard(copy, golem, { before: FIXTURE_SNAPSHOTS[V3] });
    expect(changed(delta)).toEqual([{ kind: "value", field: "snapshotOf", label: "Copy of", before: "Test Bolt", after: "none" }]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
  });

  it("R388 a fused definition's ingredients diff under their own name, as data only", () => {
    const bolt = fixtureDef(V3, "core-002");
    const delta = diffCard(bolt, { ...bolt, ingredients: [] });
    expect(changed(delta)).toEqual([{ kind: "value", field: "ingredients", label: "Ingredients", before: "none", after: "[]" }]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
  });

  it("R388 shows a card by its Radiant face when every printed change is the Radiant face's", () => {
    const knight = fixtureDef(V2, "core-001");
    const radiantOnly = diffCard(knight, { ...knight, radiant: { ...knight.radiant, text: "Cry: Deal 3 damage to a target. Draw 2." } });
    expect(radiantOnly === null ? null : faceShown(radiantOnly)).toBe("radiant");
    const both = diffCard(fixtureDef(V1, "core-001"), knight);
    expect(both === null ? null : faceShown(both)).toBe("base");
  });

  it("R388 reports a face's own type and a card field it does not know, so no change is dropped", () => {
    const bolt = fixtureDef(V3, "core-002");
    const retyped = { ...bolt, radiant: { ...bolt.radiant, type: "Field Spell" as const } };
    expect(field(changed(diffCard(bolt, retyped)), "radiant.type")).toMatchObject({ before: "Spell", after: "Field Spell" });
    const grown = { ...bolt, artist: "Someone" } as unknown as CardDef;
    expect(field(changed(diffCard(bolt, grown)), "data:artist")).toMatchObject({ label: "artist", before: "none", after: "Someone" });
  });

  it("R388 R749 a number newly tuned on the Radiant face only is a params change, so the card is not dropped", () => {
    const bolt = fixtureDef(V3, "core-002");
    const radiantOnly = { ...bolt, params: (bolt.params ?? []).map((param) => ({ ...param, tunedOn: "radiant" as const })) };
    const delta = diffCard(bolt, radiantOnly);
    expect(changed(delta)).toEqual([
      {
        kind: "value",
        field: "params",
        label: expect.any(String) as string,
        before: "damage 4, Radiant 6, step 1, at least 1, more is better",
        after: "damage 4, Radiant 6, step 1, at least 1, tuned on the Radiant face only, more is better",
      },
    ]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
  });

  it("R388 R1431 R1430 a number newly tuned on the base face only, or newly its power's, is a params change", () => {
    const bolt = fixtureDef(V3, "core-002");
    const baseOnly = { ...bolt, params: (bolt.params ?? []).map((param) => ({ ...param, tunedOn: "base" as const })) };
    expect(changed(diffCard(bolt, baseOnly))).toEqual([
      {
        kind: "value",
        field: "params",
        label: expect.any(String) as string,
        before: "damage 4, Radiant 6, step 1, at least 1, more is better",
        after: "damage 4, Radiant 6, step 1, at least 1, tuned on the base face only, more is better",
      },
    ]);
    const powered = { ...bolt, params: (bolt.params ?? []).map((param) => ({ ...param, power: "burn" })) };
    const delta = diffCard(bolt, powered);
    expect(changed(delta)).toEqual([
      {
        kind: "value",
        field: "params",
        label: expect.any(String) as string,
        before: "damage 4, Radiant 6, step 1, at least 1, more is better",
        after: "damage 4, Radiant 6, step 1, at least 1, tuned only while the card has Steady Shot, more is better",
      },
    ]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
  });

  it("R388 R1748000 a number newly marked cry is a params change, so the card is not dropped", () => {
    const bolt = fixtureDef(V3, "core-002");
    const cry = { ...bolt, params: (bolt.params ?? []).map((param) => ({ ...param, cry: true })) };
    const delta = diffCard(bolt, cry);
    expect(changed(delta)).toEqual([
      {
        kind: "value",
        field: "params",
        label: expect.any(String) as string,
        before: "damage 4, Radiant 6, step 1, at least 1, more is better",
        after: "damage 4, Radiant 6, step 1, at least 1, tuned only off the field, more is better",
      },
    ]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
  });

  it("R388 R2261001 a number newly marked cry on one face is a params change that names the face", () => {
    const bolt = fixtureDef(V3, "core-002");
    const cry = { ...bolt, params: (bolt.params ?? []).map((param) => ({ ...param, cry: "base" as const })) };
    const delta = diffCard(bolt, cry);
    expect(changed(delta)).toEqual([
      {
        kind: "value",
        field: "params",
        label: expect.any(String) as string,
        before: "damage 4, Radiant 6, step 1, at least 1, more is better",
        after: "damage 4, Radiant 6, step 1, at least 1, tuned only off the field on the base face, more is better",
      },
    ]);
    expect(delta !== null && dataOnly(delta)).toBe(true);
  });

  it("R388 R277 the text diff is R277's: one alignment, marks never start or end on a separator", () => {
    const diff = wordDiff("Cast on draw: Your opponent has 1 less mana next turn.", "Cast on draw: Your opponent has 1 less mana next turn. Discard 1.");
    expect(diff.removed).toEqual([]);
    expect(diff.added).toHaveLength(1);
    const swapped = wordDiff("Summon 3 random Cost (3) Units", "Summon 3 random (3) Cost Units");
    const words = (source: string, ranges: typeof swapped.added): string[] => ranges.map((range) => source.slice(range.start, range.end));
    expect(words("Summon 3 random (3) Cost Units", swapped.added)).toEqual(["Cost"]);
    expect(words("Summon 3 random Cost (3) Units", swapped.removed)).toEqual(["Cost"]);
  });
});

describe("R388 the real history's changes", () => {
  it("R388 v0.2.0: Masochism Mask costs (1), down from (2), and records its lines of code", async () => {
    const [before, after] = await Promise.all([real("v0.1.1"), real("v0.2.0")]);
    const changes = changed(diffCard(defIn(before, "core-065"), defIn(after, "core-065")));
    expect(field(changes, "cost")).toMatchObject({ before: "(2) Cost", after: "(1) Cost" });
    expect(field(changes, "loc")).toMatchObject({ before: "not recorded", after: String(defIn(after, "core-065").loc) });
    expect(changes.map((change) => change.field).sort()).toEqual(["cost", "loc"]);
  });

  it("R388 v0.2.0: Hinder's base face adds \"Discard 1.\" and its Radiant face is unchanged", async () => {
    const [before, after] = await Promise.all([real("v0.1.1"), real("v0.2.0")]);
    const changes = changed(diffCard(defIn(before, "core-021"), defIn(after, "core-021")));
    expect(changedWords(text(changes, "base.text"))).toEqual({ added: ["Discard 1"], removed: [] });
    expect(changes.some((change) => change.field === "radiant.text")).toBe(false);
  });

  it("R388 v0.2.0: Call to Chaos's Radiant face becomes three different effects, the recursion moved to the list", async () => {
    const [before, after] = await Promise.all([real("v0.1.1"), real("v0.2.0")]);
    const words = changedWords(text(changed(diffCard(defIn(before, "core-095"), defIn(after, "core-095"))), "radiant.text"));
    // Also marked: the (N) Cost style's moved words (R432).
    expect(words.added).toEqual(expect.arrayContaining(["Three different", "resolved in the order listed", "Cost", "cast a random Call to Chaos"]));
    expect(words.added[0]).toBe("Three different");
    expect(words.added.at(-1)).toBe("cast a random Call to Chaos");
    expect(words.removed).toEqual(expect.arrayContaining(["Two", "Cast a random Call to Chaos, plus one of", "Cost"]));
    expect(words.removed[0]).toBe("Two");
  });

  it("R388 R2521009 a Resource's prices diff under the name patches ship records, resource, and print on its face", async () => {
    const korrelite = defIn(await real("v0.5.0"), "stinkyscape-t-korrelite");
    const resource = korrelite.resource;
    if (resource === undefined) throw new Error("Korrelite is no Resource");
    const repriced = { ...korrelite, resource: { ...resource, base: { ...resource.base, sellMax: 6 } } };
    const delta = diffCard(korrelite, repriced);
    expect(changed(delta)).toEqual([
      {
        kind: "value",
        field: "resource",
        label: "Resource prices",
        before: "Common, sells for 2–5 Credits, bought for 5 Credits; Radiant sells for 4–10 Credits, bought for 10 Credits",
        after: "Common, sells for 2–6 Credits, bought for 5 Credits; Radiant sells for 4–10 Credits, bought for 10 Credits",
      },
    ]);
    expect(delta !== null && dataOnly(delta)).toBe(false);
    expect(delta === null ? null : faceShown(delta)).toBe("base");
    expectRecorded({ id: korrelite.id, name: korrelite.name, kind: "changed", fields: ["resource"] }, delta, korrelite.id);
  });

  it("R388 every card every patch records diffs to what the record says: its kind and exactly its fields", async () => {
    const patches = await realPatchSource.patches();
    expect(patches.length).toBeGreaterThan(0);
    for (const patch of patches) {
      const after = await real(patch.version);
      const previous = versionBefore(patches, patch.version);
      const before = previous === null ? {} : await real(previous);
      for (const record of patch.changes) {
        expectRecorded(record, diffCard(before[record.id], after[record.id]), `${patch.version} ${record.id}`);
      }
    }
  }, 30_000);
});
