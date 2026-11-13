// Board geometry marks (issue #3554, R3554000–R3554007). The engine tells which lane the Black
// Hole holds and which the slash voided, and which damage chain a Unit is in, so the board stamps
// each lane `data-lane-mark` (`game/geometry.ts`) and each chained zone `data-chain` for
// `board.css` to paint (CLAUDE.md rule 7).

import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { testid } from "./contract.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { renderBoard } from "../test/board.tsx";

afterEach(() => {
  cleanup();
});

function laneMark(board: HTMLElement, lane: number): string | null {
  const column = board.querySelector(`[data-lane="${lane}"]`);
  if (column === null) throw new Error(`no lane ${lane}`);
  return column.getAttribute("data-lane-mark");
}

function chainOf(board: HTMLElement, side: "you" | "opponent", row: "units" | "backrow", lane: number): string | null {
  const zone = board.querySelector(`[data-testid="${testid.zone(side, row, lane)}"]`);
  if (zone === null) throw new Error(`no zone ${side} ${row} ${lane}`);
  return zone.getAttribute("data-chain");
}

describe("R3554000 the Black Hole lane and its count", () => {
  it("R3554000 draws the Black Hole lane and its count", () => {
    const board = renderBoard(
      baseView({
        you: { ...emptySide("p1") },
        blackHole: { lane: 3, swallowed: 4, capacity: 10 },
      }),
    );
    expect(laneMark(board, 3)).toBe("blackHole");
    expect(laneMark(board, 2)).toBeNull();
    const count = board.querySelector(`[data-testid="${testid.blackHoleCount}"]`);
    expect(count?.textContent).toBe("4/10");
  });
});

describe("R3554002 the slashed lane", () => {
  it("R3554002 draws the slash", () => {
    const board = renderBoard(
      baseView({
        you: { ...emptySide("p1") },
        slashedLane: 3,
      }),
    );
    expect(laneMark(board, 3)).toBe("slashed");
    expect(laneMark(board, 4)).toBeNull();
  });
});

describe("R3554007 chained units", () => {
  it("R3554007 marks chained units", () => {
    const board = renderBoard(
      baseView({
        you: {
          ...emptySide("p1"),
          units: [unit("p1", { sharedChain: 1 }), unit("p1", { splitChain: 2 }), null, null, null],
        },
      }),
    );
    expect(chainOf(board, "you", "units", 1)).toBe("shared");
    expect(chainOf(board, "you", "units", 2)).toBe("split");
    expect(chainOf(board, "you", "units", 3)).toBeNull();
  });
});
