// R2100000: a deck's kept versions as the History popover reads them, pure (the DOM half is
// DeckWorkshop.test.tsx, the store's sync.test.ts).

import { describe, expect, it } from "vitest";

import type { DeckVersion } from "../../net/api.ts";
import { deckVersionsFrom, versionChange } from "./deckHistory.ts";
import { fixtureCardId } from "./fixtures.ts";

function version(id: string, name: string, cards: readonly number[], savedAt: number): DeckVersion {
  return { id, name, cards: cards.map(fixtureCardId), savedAt };
}

describe("a deck's kept versions (R2100000)", () => {
  it("R2100000 a version's change is the cards it took in and let go", () => {
    const older = version("v1", "Aggro", [1, 2, 3, 4], 1_000);
    const newer = version("v2", "Aggro", [4, 2, 5, 6], 2_000);
    expect(versionChange(newer, older)).toEqual({
      added: [fixtureCardId(5), fixtureCardId(6)],
      removed: [fixtureCardId(1), fixtureCardId(3)],
      renamedFrom: null,
    });
    // Copy by copy: a second copy in is one card in, and a card that only moved is no change.
    expect(versionChange(version("v3", "Aggro", [2, 1, 1], 3_000), version("v2", "Aggro", [1, 2], 2_000))).toEqual({
      added: [fixtureCardId(1)],
      removed: [],
      renamedFrom: null,
    });
    expect(versionChange(version("v4", "Aggro", [2], 4_000), version("v3", "Aggro", [1, 2, 1], 3_000))?.removed).toEqual([
      fixtureCardId(1),
      fixtureCardId(1),
    ]);
  });

  it("R2100000 the oldest version kept has no change to show", () => {
    expect(versionChange(version("v1", "Aggro", [1, 2], 1_000), undefined)).toBeNull();
  });

  it("R2100000 a rename alone names the old name", () => {
    expect(versionChange(version("v2", "Big Tempo", [1, 2], 2_000), version("v1", "Aggro", [1, 2], 1_000))).toEqual({
      added: [],
      removed: [],
      renamedFrom: "Aggro",
    });
  });

  it("R2100000 a malformed entry is left out", () => {
    const kept = [version("v2", "Aggro", [1, 2], 2_000), version("v1", "Aggro", [1], 1_000)];
    expect(
      deckVersionsFrom({
        versions: [
          kept[0],
          null,
          "v9",
          { id: "", name: "No id", cards: [], savedAt: 1 },
          { id: "v8", name: 8, cards: [], savedAt: 1 },
          { id: "v7", name: "Card not a string", cards: [fixtureCardId(1), 7], savedAt: 1 },
          { id: "v6", name: "No time", cards: [] },
          { id: "v5", name: "Infinite time", cards: [], savedAt: Number.POSITIVE_INFINITY },
          kept[1],
        ],
      }),
    ).toEqual(kept);
    for (const answer of [null, {}, { versions: "nope" }, []]) expect(deckVersionsFrom(answer)).toEqual([]);
  });
});
