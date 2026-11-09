// R3340000: a unit's art wears its state: the level its missing health makes (`damageLevel`) and the cracks that level draws
// (`crackPaths`), each crack the start of the same crack one level up. The glow and the frost are CSS, read in artLive.test.tsx.

import { describe, expect, it } from "vitest";

import { ART_WEAR_LEVELS } from "../constants.ts";
import { CRACK_STEPS_BASE, crackPaths, damageLevel } from "./wear.ts";

describe("R3340000 damageLevel", () => {
  it("R3340000 is 0 for a whole unit and rises a step for each quarter of its health it is missing", () => {
    expect(ART_WEAR_LEVELS).toBe(4);
    expect([4, 3, 2, 1, 0].map((health) => damageLevel(health, 4))).toEqual([0, 1, 2, 3, 4]);
    // A bigger unit steps by its own share: a tenth is the first step, three quarters the third.
    expect(damageLevel(9, 10)).toBe(1);
    expect(damageLevel(7, 10)).toBe(2);
    expect(damageLevel(2, 10)).toBe(4);
    expect(damageLevel(25, 30)).toBe(1);
  });

  it("R3340000 is 0 where nothing is missing or there is no maximum, and never past the last step", () => {
    expect(damageLevel(5, 4)).toBe(0);
    expect(damageLevel(4, 4)).toBe(0);
    expect(damageLevel(3, 0)).toBe(0);
    expect(damageLevel(3, -1)).toBe(0);
    expect(damageLevel(-2, 4)).toBe(ART_WEAR_LEVELS);
    expect(damageLevel(0, 1)).toBe(ART_WEAR_LEVELS);
  });
});

describe("R3340000 crackPaths", () => {
  it("R3340000 draws one crack for each level, the same ones for the same card, others for another", () => {
    for (let level = 0; level <= ART_WEAR_LEVELS; level += 1) expect(crackPaths("core-004", level)).toHaveLength(level);
    expect(crackPaths("core-004", 3)).toEqual(crackPaths("core-004", 3));
    expect(crackPaths("core-004", 3)).not.toEqual(crackPaths("core-001", 3));
    // Levels past the last draw no more cracks; one below the first draws none.
    expect(crackPaths("core-004", ART_WEAR_LEVELS + 3)).toHaveLength(ART_WEAR_LEVELS);
    expect(crackPaths("core-004", -1)).toEqual([]);
  });

  it("R3340000 each crack starts on the picture's border and runs inward in segments, one more of them for each level", () => {
    for (const id of ["core-004", "stinkyscape-006", "traininggrounds-016"]) {
      for (const [level, paths] of [1, 2, 3, 4].map((n) => [n, crackPaths(id, n)] as const)) {
        for (const path of paths) {
          expect(path.startsWith("M")).toBe(true);
          const points = [...path.matchAll(/[ML]\s*(-?[\d.]+) (-?[\d.]+)/gu)].map((match) => [Number(match[1]), Number(match[2])] as const);
          expect(points).toHaveLength(CRACK_STEPS_BASE + level + 1);
          const [x, y] = points[0] ?? [0, 0];
          expect(Math.max(Math.abs(x - 50), Math.abs(y - 50)), `${id} level ${level}: a start on the border`).toBeCloseTo(50, 1);
          const last = points[points.length - 1] ?? [0, 0];
          const toMiddle = (point: readonly [number, number]): number => Math.hypot(point[0] - 50, point[1] - 50);
          expect(toMiddle(last), `${id}: it runs inward`).toBeLessThan(toMiddle([x, y]));
        }
      }
    }
  });

  it("R3340000 a crack at one level is the start of the same crack at the next", () => {
    for (const id of ["core-004", "stinkyscape-006", "traininggrounds-016"]) {
      for (let level = 1; level < ART_WEAR_LEVELS; level += 1) {
        const lower = crackPaths(id, level);
        const higher = crackPaths(id, level + 1);
        lower.forEach((path, k) => {
          expect(higher[k]?.startsWith(path), `${id}: crack ${k} at level ${level + 1}`).toBe(true);
          expect(higher[k]?.length, `${id}: crack ${k} grows`).toBeGreaterThan(path.length);
        });
      }
    }
  });
});
