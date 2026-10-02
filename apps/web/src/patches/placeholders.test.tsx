// R388: a raw `{…}` never reaches the screen. Every face the Patch notes page and the History section
// draw comes from a snapshot, and a v0.2.0 snapshot's texts write their tunable numbers as `{key}` and
// their agreeing words as `{key|singular|plural}` (B3.4); every text is filled (`fillParams`) before
// it is drawn or diffed. Checked over every card of every real snapshot, face by face, and over every
// change of every patch as ChangeList prints it.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

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

  it("R388 every face of every card of every snapshot prints with its numbers and words filled in", async () => {
    const patches = await realPatchSource.patches();
    const drawn = new Set<string>();
    const raw: string[] = [];
    let faces = 0;
    for (const patch of patches) {
      for (const def of Object.values(await snapshotOf(patch.version))) {
        // A card most patches left alone is the same definition in each snapshot: drawn once.
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
    expect(faces).toBeGreaterThan(2 * 317);
    expect(raw).toEqual([]);
  }, 180_000);

  it("R388 every change of every card in every patch prints with its numbers and words filled in", async () => {
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
  }, 60_000);
});
