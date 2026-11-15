// R3944001, R3944002: the daily patch on the page's data side (releases.ts): a major or a patch holds the
// micros its `includes` names as its snapshots, the micros no daily patch has rolled up yet wait apart, the
// micros shipped before the daily patch keep the patch before them, and a daily patch's cards are its
// micros'.

import { describe, expect, it } from "vitest";

import { DAILY_PATCHES, DM1, DM2, DM3, DP, FIXTURE_PATCHES, M1, M2, TIERED_PATCHES, V1, V2, V3 } from "./fixtures.ts";
import { cardsBasis, nestMicros, pendingMicros, releaseGroups, rolledUpChanges, tiers, type Release } from "./releases.ts";
import type { Patch } from "./source.ts";

const LINES: readonly Release[] = [
  { id: "older", name: "Older line", summary: "", technical: [] },
  { id: "newer", name: "Newer line", summary: "", technical: [] },
];

/** A major after the daily history that ships the set and rolls up DM3, the micro before it. */
const MAJOR: Patch = {
  version: "v0-next-line",
  tier: "major",
  date: "2026-04-03",
  title: "A set ships",
  source: "Fixture commit d4",
  notes: "A new set.",
  player: { title: "A new set", new: ["Fresh Face."], balance: [], changes: [], text: [], fixes: [] },
  includes: [DM3],
  changes: [{ id: "classic-001", name: "Fresh Face", kind: "added" }],
};

/** Each line's id, its versions newest first, and its nests as [entry, its micros oldest first]. */
function laidOut(patches: readonly Patch[]) {
  const tierList = tiers(patches);
  const tierOf = new Map(patches.map((patch, at) => [patch.version, tierList[at] ?? "patch"] as const));
  const pending = pendingMicros(patches, tierList).map((patch) => patch.version);
  const lines = releaseGroups(patches, LINES).map((group) => ({
    id: group.release.id,
    versions: group.patches.map((patch) => patch.version),
    nests: nestMicros(group.patches, tierOf, new Set(pending)).map((nest) => [nest.patch.version, nest.micros.map((micro) => micro.version)]),
  }));
  return { pending, lines };
}

describe("R3944001 the daily patch's snapshots", () => {
  it("R3944001 a daily patch holds the micros it rolls up as its snapshots, and the micros after it wait apart", () => {
    expect(laidOut(DAILY_PATCHES)).toEqual({
      pending: [DM3],
      lines: [
        {
          id: "older",
          versions: [DM3, DP, DM2, DM1, V1],
          nests: [
            [DP, [DM1, DM2]],
            [V1, []],
          ],
        },
      ],
    });
  });

  it("R3944001 a major shipped after micros holds them in its own line, and leaves none waiting", () => {
    expect(laidOut([...DAILY_PATCHES, MAJOR])).toEqual({
      pending: [],
      lines: [
        { id: "newer", versions: [MAJOR.version, DM3], nests: [[MAJOR.version, [DM3]]] },
        {
          id: "older",
          versions: [DP, DM2, DM1, V1],
          nests: [
            [DP, [DM1, DM2]],
            [V1, []],
          ],
        },
      ],
    });
  });

  it("R3944001 the micros shipped before the daily patch still sit under the patch before them", () => {
    // M1 and M2 follow V2 and come before V3, a patch: neither waits, and both stay under V2 (R3012000).
    expect(laidOut(TIERED_PATCHES)).toEqual({
      pending: [],
      lines: [
        {
          id: "older",
          versions: [V3, M2, M1, V2, V1],
          nests: [
            [V3, []],
            [V2, [M1, M2]],
            [V1, []],
          ],
        },
      ],
    });
  });

  it("R3944001 a micro after the newest patch waits, and a history of micros alone has none waiting", () => {
    const tail: Patch = { ...(TIERED_PATCHES[2] as Patch), version: "v6-tail" };
    expect(pendingMicros([...TIERED_PATCHES, tail]).map((patch) => patch.version)).toEqual(["v6-tail"]);
    expect(pendingMicros(FIXTURE_PATCHES)).toEqual([]);
    // Without the waiting set, a nest takes such a micro as one that follows it, as before the daily patch.
    const tierOf = new Map([...TIERED_PATCHES, tail].map((patch) => [patch.version, patch.tier ?? "patch"] as const));
    expect(nestMicros([tail, ...[...TIERED_PATCHES].reverse()], tierOf)[0]?.micros.map((micro) => micro.version)).toEqual(["v6-tail"]);
  });

  it("R3944001 an entry that names a micro it does not follow leaves the micro where its order puts it", () => {
    const odd: readonly Patch[] = TIERED_PATCHES.map((patch) => (patch.version === V1 ? { ...patch, includes: [M1] } : patch));
    expect(laidOut(odd).lines[0]?.nests).toEqual([
      [V3, []],
      [V2, [M1, M2]],
      [V1, []],
    ]);
  });
});

describe("R3944002 a daily patch's cards", () => {
  it("R3944002 are its micros' cards, each once in the order they first name it, with what the day did to it", () => {
    expect(rolledUpChanges(DAILY_PATCHES[3] as Patch, DAILY_PATCHES)).toEqual([
      { id: "core-001", name: "Test Knight", kind: "changed" },
      { id: "core-003", name: "Old Relic", kind: "removed" },
      { id: "core-002", name: "Test Bolt", kind: "changed" },
    ]);
    // A card added and removed the same day is no change; one removed and added again changed.
    const micro = (version: string, changes: Patch["changes"]): Patch => ({ ...(DAILY_PATCHES[1] as Patch), version, changes });
    const day: readonly Patch[] = [
      DAILY_PATCHES[0] as Patch,
      micro("a", [
        { id: "x-1", name: "Passing", kind: "added" },
        { id: "core-003", name: "Old Relic", kind: "removed" },
        { id: "y-1", name: "Newcomer", kind: "added" },
      ]),
      micro("b", [
        { id: "x-1", name: "Passing", kind: "removed" },
        { id: "core-003", name: "Old Relic", kind: "added" },
        { id: "y-1", name: "Newcomer", kind: "changed" },
      ]),
      { ...(DAILY_PATCHES[3] as Patch), version: "daily", includes: ["a", "b"] },
    ];
    expect(rolledUpChanges(day[3] as Patch, day)).toEqual([
      { id: "core-003", name: "Old Relic", kind: "changed" },
      { id: "y-1", name: "Newcomer", kind: "added" },
    ]);
  });

  it("R3944002 read against the snapshot before the first micro, and every other entry against the one before it", () => {
    const daily = cardsBasis(DAILY_PATCHES, DAILY_PATCHES[3] as Patch);
    expect(daily.before).toBe(V1);
    expect(daily.patch.version).toBe(DP);
    expect(daily.patch.changes.map((change) => change.id)).toEqual(["core-001", "core-003", "core-002"]);
    expect(cardsBasis(DAILY_PATCHES, DAILY_PATCHES[4] as Patch)).toEqual({ patch: DAILY_PATCHES[4], before: DP });
    expect(cardsBasis(DAILY_PATCHES, DAILY_PATCHES[0] as Patch)).toEqual({ patch: DAILY_PATCHES[0], before: null });
    // A major that rolls micros up keeps its own cards: its record is the set it ships.
    const major = cardsBasis([...DAILY_PATCHES, MAJOR], MAJOR);
    expect(major).toEqual({ patch: MAJOR, before: DM3 });
  });
});
