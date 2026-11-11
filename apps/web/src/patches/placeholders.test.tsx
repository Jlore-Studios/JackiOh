// R388: a raw `{…}` never reaches the screen. A v0.2.0 snapshot's texts write their tunable numbers as
// `{key}` and their agreeing words as `{key|singular|plural}` (B3.4), and every text is filled
// (`fillParams`) before it is drawn or diffed.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { daily } from "../test/daily.ts";
import type { CardDef } from "@jackioh/shared";

import { ChangeList } from "./ChangeList.tsx";
import { changedWords, diffCard, type TextChange } from "./diff.ts";
import { versionBefore } from "./history.ts";
import { PatchFace } from "./PatchFace.tsx";
import { realPatchSource, type Snapshot } from "./source.ts";

afterEach(cleanup);

const BRACE = /[{}]/;

async function snapshotOf(version: string): Promise<Snapshot> {
  const snapshot = await realPatchSource.snapshot(version);
  if (snapshot === null) throw new Error(`no snapshot ${version}`);
  return snapshot;
}

/** The history's first six versions, as the brief checked them. */
const BRIEF_VERSIONS = ["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0"];
/** How many of the newest snapshots each pull request loads through the real source (#3418). */
const NEWEST_SNAPSHOTS = 3;
/** The time a walk over every shipped snapshot gets. */
const EVERY_SNAPSHOT_MS = 180_000;

describe("R388 no raw placeholder reaches the screen", () => {
  it("R388 fills an agreeing placeholder before diffing: \"Draw {draw|card|cards}.\" reads \"Draw 1 card.\" then \"Draw 2 cards.\"", () => {
    const before: CardDef = {
      id: "core-900",
      index: "900",
      name: "Plural Test",
      set: "Core",
      type: "Spell",
      tags: [],
      rarity: "Common",
      token: false,
      cost: 1,
      params: [{ key: "draw", base: 1, radiant: 2, better: "up", step: 1, min: 1 }],
      base: { keywords: [], text: "Draw {draw|card|cards}." },
      radiant: { keywords: [], text: "Draw {draw|card|cards}." },
    };
    const after: CardDef = { ...before, params: [{ key: "draw", base: 2, radiant: 2, better: "up", step: 1, min: 1 }] };
    const delta = diffCard(before, after);
    if (delta?.kind !== "changed") throw new Error("expected a change");
    const base = delta.changes.find((change): change is TextChange => change.kind === "text" && change.field === "base.text");
    expect(base?.before).toBe("Draw 1 card.");
    expect(base?.after).toBe("Draw 2 cards.");
    expect(base === undefined ? null : changedWords(base)).toEqual({ added: ["2 cards"], removed: ["1 card"] });
    // The Radiant face prints "Draw 2 cards." both times: nothing to mark there.
    expect(delta.changes.some((change) => change.field === "radiant.text")).toBe(false);
  });

  // daily: it renders every shipped snapshot's faces, so it slows with each `patches ship` and ran
  // minutes on `web (unit 2/8)` (#3629). Per pull request the case below holds placeholders on the
  // first six snapshots and the newest three.
  daily("R388 every face of every card of every snapshot prints with its numbers and words filled in", async () => {
    const patches = await realPatchSource.patches();
    const drawn = new Set<string>();
    const raw: string[] = [];
    let faces = 0;
    for (const patch of patches) {
      for (const def of Object.values(await snapshotOf(patch.version))) {
        // An unchanged card is the same definition in each snapshot: drawn once.
        const key = JSON.stringify(def);
        if (drawn.has(key)) continue;
        drawn.add(key);
        for (const face of ["base", "radiant"] as const) {
          const { container, unmount } = render(<PatchFace def={def} face={face} />);
          faces += 1;
          if (BRACE.test(container.textContent ?? "")) raw.push(`${patch.version} ${def.id} ${face}: ${container.textContent ?? ""}`);
          unmount();
        }
      }
    }
    const newest = await snapshotOf(patches.at(-1)?.version ?? "");
    expect(faces).toBeGreaterThan(2 * Object.keys(newest).length);
    expect(raw).toEqual([]);
  }, EVERY_SNAPSHOT_MS);

  it("R388 every face of every card of the first six snapshots and the newest three prints with its numbers and words filled in", async () => {
    const patches = await realPatchSource.patches();
    // A bounded set of snapshots, so the case does not slow as the history grows (#3629): its start and
    // its newest end, where a promotion appends. The daily case above draws every one.
    const read = patches.filter((patch, at) => at < BRIEF_VERSIONS.length || at >= patches.length - NEWEST_SNAPSHOTS);
    const drawn = new Set<string>();
    const raw: string[] = [];
    let faces = 0;
    for (const patch of read) {
      for (const def of Object.values(await snapshotOf(patch.version))) {
        // An unchanged card is the same definition in each snapshot: drawn once.
        const key = JSON.stringify(def);
        if (drawn.has(key)) continue;
        drawn.add(key);
        for (const face of ["base", "radiant"] as const) {
          const { container, unmount } = render(<PatchFace def={def} face={face} />);
          faces += 1;
          if (BRACE.test(container.textContent ?? "")) raw.push(`${patch.version} ${def.id} ${face}: ${container.textContent ?? ""}`);
          unmount();
        }
      }
    }
    const newest = await snapshotOf(patches.at(-1)?.version ?? "");
    expect(faces).toBeGreaterThan(2 * Object.keys(newest).length);
    expect(raw).toEqual([]);
  // Nine snapshots still draw thousands of faces (2730 on 2026-11-10: ~60 s on the shared bot machine),
  // so the bounded walk shares the every-snapshot budget rather than the 60 s one (#3629).
  }, EVERY_SNAPSHOT_MS);

  // daily: it walks every shipped patch's changes, so it slows with each `patches ship` and ran
  // minutes on `web (unit 2/8)` (#3629). Per pull request the case below holds placeholders on the
  // first six patches and the newest three.
  daily("R388 every change of every card in every patch prints with its numbers and words filled in", async () => {
    const patches = await realPatchSource.patches();
    const raw: string[] = [];
    let changes = 0;
    for (const patch of patches) {
      const after = await snapshotOf(patch.version);
      const previous = versionBefore(patches, patch.version);
      const before = previous === null ? {} : await snapshotOf(previous);
      for (const record of patch.changes) {
        const delta = diffCard(before[record.id], after[record.id], { before, after });
        if (delta?.kind !== "changed") continue;
        for (const change of delta.changes) {
          changes += 1;
          const words = change.kind === "text" ? [change.before, change.after] : [change.before, change.after, change.label];
          if (words.some((text) => BRACE.test(text))) raw.push(`${patch.version} ${record.id} ${change.field}`);
        }
        const { container, unmount } = render(<ChangeList delta={delta} />);
        if (BRACE.test(container.textContent ?? "")) raw.push(`${patch.version} ${record.id}: ${container.textContent ?? ""}`);
        unmount();
      }
    }
    expect(changes).toBeGreaterThan(0);
    expect(raw).toEqual([]);
  }, EVERY_SNAPSHOT_MS);

  it("R388 every change of every card in the first six patches and the newest three prints with its numbers and words filled in", async () => {
    const patches = await realPatchSource.patches();
    // A bounded set of patches, so the case does not slow as the history grows (#3629): its start and
    // its newest end, where a promotion appends. The daily case above walks every one. The change's
    // predecessor still comes from the full list, so each diff sees the snapshot before it.
    const read = patches.filter((patch, at) => at < BRIEF_VERSIONS.length || at >= patches.length - NEWEST_SNAPSHOTS);
    const raw: string[] = [];
    let changes = 0;
    for (const patch of read) {
      const after = await snapshotOf(patch.version);
      const previous = versionBefore(patches, patch.version);
      const before = previous === null ? {} : await snapshotOf(previous);
      for (const record of patch.changes) {
        const delta = diffCard(before[record.id], after[record.id], { before, after });
        if (delta?.kind !== "changed") continue;
        for (const change of delta.changes) {
          changes += 1;
          const words = change.kind === "text" ? [change.before, change.after] : [change.before, change.after, change.label];
          if (words.some((text) => BRACE.test(text))) raw.push(`${patch.version} ${record.id} ${change.field}`);
        }
        const { container, unmount } = render(<ChangeList delta={delta} />);
        if (BRACE.test(container.textContent ?? "")) raw.push(`${patch.version} ${record.id}: ${container.textContent ?? ""}`);
        unmount();
      }
    }
    expect(changes).toBeGreaterThan(0);
    expect(raw).toEqual([]);
  }, 60_000);
});
