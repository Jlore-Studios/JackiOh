// R3546004: a GIGA permanent drawn across its four zones (issue #3546). The engine marks only
// the anchor's view with `giga: true` and leaves the three covered zones empty, so the board
// stamps each zone `data-giga` (`game/giga.ts`) and the stylesheet spans the anchor's card.

import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { testid } from "./contract.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "../test/fixtures.ts";
import { renderBoard } from "../test/board.tsx";

afterEach(() => {
  cleanup();
});

function gigaOf(board: HTMLElement, side: "you" | "opponent", row: "units" | "backrow", lane: number): string | null {
  const zone = board.querySelector(`[data-testid="${testid.zone(side, row, lane)}"]`);
  if (zone === null) throw new Error(`no zone ${side} ${row} ${lane}`);
  return zone.getAttribute("data-giga");
}

describe("R3546004 a GIGA permanent drawn across its four zones", () => {
  it("R3546004 your GIGA unit's zone is the anchor and its three others are covered", () => {
    const board = renderBoard(
      baseView({
        you: {
          ...emptySide("p1"),
          units: [unit("p1", { giga: true }), null, null, null, null],
        },
      }),
    );
    expect(gigaOf(board, "you", "units", 1)).toBe("anchor");
    expect(gigaOf(board, "you", "units", 2)).toBe("covered");
    expect(gigaOf(board, "you", "backrow", 1)).toBe("covered");
    expect(gigaOf(board, "you", "backrow", 2)).toBe("covered");
    expect(gigaOf(board, "you", "units", 3)).toBeNull();
    expect(gigaOf(board, "you", "backrow", 3)).toBeNull();
  });

  it("R3546004 an enemy GIGA field spell covers its unit zones", () => {
    const board = renderBoard(
      baseView({
        opponent: {
          ...emptySide("p2"),
          backrow: [faceUpBackrow("p2", { giga: true }), null, null, null, null],
        },
      }),
    );
    expect(gigaOf(board, "opponent", "backrow", 1)).toBe("anchor");
    expect(gigaOf(board, "opponent", "backrow", 2)).toBe("covered");
    expect(gigaOf(board, "opponent", "units", 1)).toBe("covered");
    expect(gigaOf(board, "opponent", "units", 2)).toBe("covered");
    expect(gigaOf(board, "opponent", "units", 3)).toBeNull();
  });

  it("R3546004 a unit holding the GIGA keyword renders with no chip for it", () => {
    const board = renderBoard(
      baseView({
        you: {
          ...emptySide("p1"),
          units: [unit("p1", { keywords: [{ kind: "GIGA" }] }), null, null, null, null],
        },
      }),
    );
    // No throw above: a kind with no visual draws nothing. No visible chip and no treatment.
    expect(board.querySelector('.keywords > [data-keyword="GIGA"]:not([data-chip="hidden"])')).toBeNull();
    expect(board.querySelector('[data-keyword-fx="GIGA"]')).toBeNull();
  });
});
