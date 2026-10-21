// C+ #62 KY's Papaya's curve as the equation its cells make (curveEquation.ts, SPEC §8.7 row 62).

import type { PlayerId, Row, ZoneRef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { curveEquation } from "./curveEquation.ts";

const cell = (player: PlayerId, row: Row, lane: number): ZoneRef => ({ player, row, lane });
const OFF_GRID_LANES = [0, 6, 999] as const;
const OVERSIZED_CURVE_CELL_COUNT = 120;

describe("curveEquation: KY's Papaya's curve as the equation its cells make", () => {
  it("a constant crosses its whole row, and reads y = 0 on the caster's backrow", () => {
    expect(curveEquation([1, 2, 3, 4, 5].map((lane) => cell("p1", "units", lane)), "p1")).toBe("y = 1");
    expect(curveEquation([1, 2, 3, 4, 5].map((lane) => cell("p1", "backrow", lane)), "p1")).toBe("y = 0");
    expect(curveEquation([cell("p2", "units", 3)], "p1")).toBe("y = 2");
  });

  it("a line with a negative slope leaves out a coefficient of one", () => {
    expect(
      curveEquation(
        [cell("p2", "backrow", 1), cell("p2", "units", 2), cell("p1", "units", 3), cell("p1", "backrow", 4)],
        "p1",
      ),
    ).toBe("y = −x + 3");
  });

  it("a line through a lane nobody picked, and fractional coefficients", () => {
    expect(curveEquation([cell("p1", "backrow", 1), cell("p1", "units", 3), cell("p2", "units", 5)], "p1")).toBe("y = ¹⁄₂x");
    expect(curveEquation([cell("p1", "backrow", 2), cell("p1", "units", 4)], "p1")).toBe("y = ¹⁄₂x − ¹⁄₂");
  });

  it("a parabola whose cells include lanes nobody picked", () => {
    expect(
      curveEquation(
        [
          cell("p1", "backrow", 1),
          cell("p2", "units", 2),
          cell("p2", "backrow", 3),
          cell("p2", "backrow", 4),
          cell("p2", "units", 5),
        ],
        "p1",
      ),
    ).toBe("y = −¹⁄₂x² + ⁵⁄₂x");
  });

  it("a cubic, its fractions in superscript and subscript digits", () => {
    expect(
      curveEquation(
        [cell("p1", "units", 1), cell("p1", "backrow", 2), cell("p2", "backrow", 4), cell("p1", "backrow", 5)],
        "p1",
      ),
    ).toBe("y = −⁷⁄₁₂x³ + ¹⁹⁄₆x² − ⁴³⁄₁₂x + 1");
    expect(
      curveEquation(
        [
          cell("p1", "backrow", 1),
          cell("p1", "backrow", 2),
          cell("p2", "units", 3),
          cell("p2", "backrow", 4),
          cell("p1", "backrow", 5),
        ],
        "p1",
      ),
    ).toBe("y = −¹⁄₂x³ + ⁵⁄₂x² − 2x");
  });

  it("reads the grid from the caster's side, whichever seat cast it", () => {
    expect(
      curveEquation(
        [
          cell("p2", "backrow", 1),
          cell("p1", "units", 2),
          cell("p1", "backrow", 3),
          cell("p1", "backrow", 4),
          cell("p1", "units", 5),
        ],
        "p2",
      ),
    ).toBe("y = −¹⁄₂x² + ⁵⁄₂x");
    expect(
      curveEquation(
        [
          cell("p1", "backrow", 1),
          cell("p2", "units", 2),
          cell("p2", "backrow", 3),
          cell("p2", "backrow", 4),
          cell("p2", "units", 5),
        ],
        "p2",
      ),
    ).toBe("y = ¹⁄₂x² − ⁵⁄₂x + 3");
  });

  it("names no equation for empty, duplicate-lane, off-grid, or oversized cells", () => {
    expect(curveEquation([], "p1")).toBeNull();
    expect(curveEquation([cell("p1", "units", 1), cell("p2", "units", 1)], "p1")).toBeNull();
    for (const lane of OFF_GRID_LANES) expect(curveEquation([cell("p1", "units", lane)], "p1")).toBeNull();
    expect(
      curveEquation(
        Array.from({ length: OVERSIZED_CURVE_CELL_COUNT }, (_, index) => cell("p1", "units", index + 1)),
        "p1",
      ),
    ).toBeNull();
  });
});
