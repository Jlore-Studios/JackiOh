// The scene kit's drawing helpers (cards/art/kitDraw.ts, R3331000/R3390000): a part's frame, the
// shapes it draws them with and its flags. Pure: every draw is the stream's. No test names this
// module today; the scene suites draw through it. Each helper's arithmetic is asserted, so a mutant
// moving a bound, a flag or a sweep fails here.

import { describe, expect, it } from "vitest";

import {
  bandPath,
  between,
  blob,
  circlePath,
  crescent,
  DEGREES,
  EDGE,
  ellipsePath,
  FAR_EDGE,
  FULL_TURN,
  groundBand,
  leant,
  local,
  MAIN,
  MAIN_INKED,
  INKED,
  polygonPath,
  pt,
  rectPath,
  SCENE_BOX,
  shape,
  sparkle,
  spiralArm,
  turned,
  whole,
} from "./kitDraw.ts";

const stream = (values: readonly number[]): (() => number) => {
  let i = 0;
  return () => values[i++ % values.length] ?? 0;
};

describe("kit numbers", () => {
  it("pins the scene box, its edges and the turn", () => {
    expect(SCENE_BOX).toBe(100);
    expect(EDGE).toBe(-2);
    expect(FAR_EDGE).toBe(102);
    expect(FULL_TURN).toBe(360);
    expect(DEGREES).toBe(Math.PI / 180);
  });

  it("between spans the stream across its bounds, whole lands on integers", () => {
    expect(between(stream([0]), 3, 9)).toBe(3);
    expect(between(stream([1]), 3, 9)).toBe(9);
    expect(between(stream([0.5]), 3, 9)).toBe(6);
    expect(whole(stream([0]), 3, 9)).toBe(3);
    expect(whole(stream([0.999]), 3, 9)).toBe(9);
    expect(Number.isInteger(whole(stream([0.37]), 3, 9))).toBe(true);
  });

  it("pt prints a point, and the path builders close their shapes", () => {
    expect(pt(1.5, 2.25)).toBe("1.5 2.25");
    expect(pt(0, 0)).toBe("0 0");
    expect(circlePath(0, 0, 1)).toBe("M-1 0A1 1 0 1 0 1 0A1 1 0 1 0 -1 0Z");
    expect(ellipsePath(0, 0, 2, 1)).toBe("M-2 0A2 1 0 1 0 2 0A2 1 0 1 0 -2 0Z");
    expect(rectPath(1, 2, 3, 4)).toBe("M1 2L4 2L4 6L1 6Z");
  });

  it("polygonPath walks the sides round the circle and closes", () => {
    expect(polygonPath(0, 0, 4, 1, 0)).toBe("M1 0L0 1L-1 0L0 -1Z");
    const hex = polygonPath(0, 0, 6, 2, 0);
    expect(hex.endsWith("Z")).toBe(true);
    expect(hex.split("L")).toHaveLength(6);
  });

  it("bandPath runs a band along the points and back", () => {
    expect(bandPath([[0, 0]], 2)).toBe("M0 0L0 0Z");
    expect(bandPath([[0, 0], [4, 0]], 2)).toBe("M0 1L4 1L4 -1L0 -1Z");
  });
});

describe("the part frame", () => {
  it("scales offsets by s, and mirrors x and the sweep facing left", () => {
    const right = local({ x: 10, y: 20, s: 2, flip: 1 });
    expect(right.x(1)).toBe(12);
    expect(right.y(1)).toBe(22);
    expect(right.p(1, 1)).toBe("12 22");
    expect(right.r(3)).toBe(6);
    expect(right.sweep(1)).toBe(1);
    expect(right.sweep(0)).toBe(0);

    const left = local({ x: 10, y: 20, s: 2, flip: -1 });
    expect(left.x(1)).toBe(8);
    expect(left.sweep(1)).toBe(0);
    expect(left.sweep(0)).toBe(1);
  });

  it("draws its shapes through the frame", () => {
    const f = local({ x: 0, y: 0, s: 1, flip: 1 });
    expect(f.poly([[0, 0], [2, 0], [2, 2]])).toBe("M0 0L2 0L2 2Z");
    expect(f.circle(0, 0, 1)).toBe(circlePath(0, 0, 1));
  });

  it("shape carries the flags, and the flag constants name their bits", () => {
    expect(MAIN).toEqual({ main: true });
    expect(INKED).toEqual({ inked: true });
    expect(MAIN_INKED).toEqual({ main: true, inked: true });
    expect(shape("d", "black", 1)).toEqual({ d: "d", paint: "black", opacity: 1 });
    expect(shape("d", "black", 1, MAIN)).toEqual({ d: "d", paint: "black", opacity: 1, main: true });
  });

  it("turned rotates clockwise, leant shears the top sideways", () => {
    const [rx, ry] = turned([1, 0], 0);
    expect([rx, ry]).toEqual([1, 0]);
    const [qx, qy] = turned([1, 0], 90);
    expect(qx).toBeCloseTo(0, 10);
    expect(qy).toBeCloseTo(1, 10);
    expect(leant([3, 2], 0.5)).toEqual([2, 2]);
    expect(leant([3, 2], 0)).toEqual([3, 2]);
  });
});

describe("the kit's figures", () => {
  const f = local({ x: 0, y: 0, s: 1, flip: 1 });

  it("sparkle throws eight points and closes", () => {
    const d = sparkle(f, 0, 0, 4);
    expect(d.startsWith("M")).toBe(true);
    expect(d.endsWith("Z")).toBe(true);
    expect(d.split("L")).toHaveLength(8);
  });

  it("blob smooths the same rim from the same stream, twice", () => {
    const first = blob(f, stream([0.5]), 0, 0, 4, 6);
    expect(first).toBe(blob(f, stream([0.5]), 0, 0, 4, 6));
    expect(first.startsWith("M")).toBe(true);
    expect(first.endsWith("Z")).toBe(true);
    expect(first).toContain("Q");
    expect(blob(f, stream([0.1]), 0, 0, 4, 6)).not.toBe(first);
  });

  it("groundBand runs edge to edge past the box and closes", () => {
    const d = groundBand(stream([0.5]), 80, 2, 4);
    expect(d.startsWith(`M${pt(EDGE, FAR_EDGE)}`)).toBe(true);
    expect(d.endsWith("Z")).toBe(true);
  });

  it("crescent hollows an arc, and a spiral arm tapers round", () => {
    const c = crescent(f, 4, 0.5);
    expect(c.startsWith("M")).toBe(true);
    expect(c.endsWith("Z")).toBe(true);
    expect(c).toContain("A");
    const arm = spiralArm(f, 0, 90, 1, 5, 1);
    expect(arm.startsWith("M")).toBe(true);
    expect(arm.endsWith("Z")).toBe(true);
  });
});
