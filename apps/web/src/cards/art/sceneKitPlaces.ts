// R3390000: the scene kit's places (ART.md, "Scene recipes and kits"), which the Classic+ scenes (#3390)
// added: terrain that spans the box from its placement's height down (water, stairs, mountains, a cave's
// mouth, an ice floor); buildings and furniture that stand on their anchor (a temple, a house and its doors,
// a bridge, an altar, a lectern, shelves, a table, a statue, a menhir, a portcullis, a hearth, a server
// obelisk, a dome, a fallen column, a throne, a fountain, a cot, a market stall); the rings and wheels
// centred on it (a portal ring, a rune circle, a fortune wheel, a clock face); and three big things (a jet in
// flight, a hand cart, a chest of drawers). sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a
// palette.css name, none in Radiant's gold window; a glow is a stack of translucent shapes, never a
// gradient; shapes are filled even-odd, so a hole is a second sub-path and no two sub-paths of one shape
// overlap otherwise.

import { fmt } from "./hash.ts";
import {
  between,
  DEGREES,
  EDGE,
  ellipsePath,
  FAR_EDGE,
  INKED,
  leant,
  local,
  MAIN,
  MAIN_INKED,
  pt,
  rectPath,
  shape,
  sparkle,
  turned,
  whole,
  type Local,
  type Point,
} from "./kitDraw.ts";
import type { KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";

/** The box as a frame of its own, for terrain drawn in the box's coordinates. */
const BOX_FRAME = local({ x: 0, y: 0, s: 1, flip: 1 });

/** A point `r` from (`cx`, `cy`) at `degrees`, clockwise from the right on screen. */
function polar(r: number, degrees: number, cx = 0, cy = 0): Point {
  return [cx + r * Math.cos(degrees * DEGREES), cy + r * Math.sin(degrees * DEGREES)];
}

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** An opening with a round head, its sides `half` from the middle, standing on `base` and springing at `spring`. */
function roundHead(f: Local, half: number, spring: number, base = 0): string {
  const r = fmt(f.r(half));
  return `M${f.p(-half, base)}L${f.p(-half, spring)}A${r} ${r} 0 0 ${f.sweep(1)} ${f.p(half, spring)}L${f.p(half, base)}Z`;
}

/** A ring about (`dx`, `dy`), from `inner` out to `outer`. */
function ring(f: Local, dx: number, dy: number, outer: number, inner: number): string {
  return f.circle(dx, dy, outer) + f.circle(dx, dy, inner);
}

// Terrain

/** A pool or river: water from the placement's y down across the box, ripples widening toward the viewer, a pale reflection under the placement's x. */
function water(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const out = [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "blue-800", 1, MAIN),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 0.4 + 0.3 * at.s), "blue-200", 0.5),
  ];
  const rows = whole(rng, 4, 5);
  const slots = 4;
  const slot = (FAR_EDGE - EDGE) / slots;
  for (let k = 1; k <= rows; k += 1) {
    const t = (k / (rows + 0.5)) ** 1.5;
    const thick = (0.25 + 0.7 * t) * at.s;
    const half = Math.min(slot * 0.42, (3 + 9 * t) * at.s);
    let d = "";
    for (let i = 0; i < slots; i += 1) {
      const cx = EDGE + slot * (i + 0.5) + between(rng, -1, 1) * (slot * 0.5 - half);
      const cy = top + depth * t + between(rng, -0.6, 0.6) * thick;
      d += `M${pt(cx - half, cy)}Q${pt(cx, cy - thick)} ${pt(cx + half, cy)}Q${pt(cx, cy + thick * 0.4)} ${pt(cx - half, cy)}Z`;
    }
    out.push(shape(d, k % 2 === 0 ? "blue-200" : "sky-300", 0.22 + 0.2 * t));
  }
  const x = at.x + between(rng, -2, 2);
  const spread = 7 * at.s;
  out.push(shape(`M${pt(x - 0.6, top)}L${pt(x + 0.6, top)}L${pt(x + spread, FAR_EDGE)}L${pt(x - spread, FAR_EDGE)}Z`, "white", 0.1));
  let glints = "";
  for (let k = 0; k < 4; k += 1) {
    const t = (k + 0.6) / 4.4;
    const half = 0.8 + spread * 0.7 * t;
    glints += ellipsePath(x + between(rng, -0.3, 0.3) * half, top + depth * t ** 1.3, half, Math.min((0.2 + 0.5 * t) * at.s, depth * 0.04));
  }
  out.push(shape(glints, "slate-100", 0.45));
  return out;
}

/** A flight of stone steps climbing away from the viewer over dark ground: its top step at the placement's y, 8 wide and centred on its x, widening to about 66 at the box's bottom (both scaled). */
function stairs(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const cx = at.x;
  const far = 4 * at.s;
  const near = between(rng, 30, 36) * at.s;
  const halfAt = (y: number): number => far + ((near - far) * (y - top)) / depth;
  const strip = (y0: number, y1: number): string =>
    `M${pt(cx - halfAt(y0), y0)}L${pt(cx + halfAt(y0), y0)}L${pt(cx + halfAt(y1), y1)}L${pt(cx - halfAt(y1), y1)}Z`;
  const steps = whole(rng, 6, 8);
  const yAt = (i: number): number => top + depth * (i / steps) ** 1.4;
  let risers = "";
  let noses = "";
  for (let i = 0; i < steps; i += 1) {
    const y0 = yAt(i);
    const y1 = yAt(i + 1);
    const lip = y0 + (y1 - y0) * 0.42;
    risers += strip(lip, y1);
    noses += strip(lip - (y1 - y0) * 0.1, lip);
  }
  const cracked = whole(rng, 1, steps - 2);
  const cy = (yAt(cracked) + yAt(cracked + 0.4)) / 2;
  const crackX = cx + between(rng, -0.6, 0.6) * halfAt(cy);
  const step = halfAt(cy) * 0.08;
  let moss = "";
  for (const [k, side] of [
    [whole(rng, 1, 2), -1],
    [whole(rng, 3, steps - 1), 1],
  ] as const) {
    const y = yAt(k) + (yAt(k + 1) - yAt(k)) * 0.3;
    const rx = 0.8 + 3 * ((y - top) / depth) * at.s;
    moss += ellipsePath(cx + side * (halfAt(y) - rx * 0.4), y, rx, rx * 0.35);
  }
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "slate-800", 1),
    shape(strip(top, FAR_EDGE), "stone-500", 1, MAIN),
    shape(risers, "slate-700", 0.95),
    shape(noses, "slate-100", 0.3),
    shape(`M${pt(cx - far, top)}L${pt(cx - far + 0.6 * at.s, top)}L${pt(cx - near + 3 * at.s, FAR_EDGE)}L${pt(cx - near, FAR_EDGE)}Z`, "slate-100", 0.3),
    shape(`M${pt(cx + far - 0.6 * at.s, top)}L${pt(cx + far, top)}L${pt(cx + near, FAR_EDGE)}L${pt(cx + near - 4 * at.s, FAR_EDGE)}Z`, "slate-900", 0.35),
    shape(BOX_FRAME.band([[crackX, cy - step], [crackX + step * 2, cy], [crackX + step, cy + step * 1.5]], 0.4 * at.s), "slate-900", 0.45),
    shape(moss, "green-700", 0.7),
  ];
}

/** A jagged range across the box, its peaks up to about 26 above the placement's y (a fainter range behind to 31), its body down past the bottom, snow on the near peaks. */
function mountains(at: PartPlace, rng: Rng): KitShape[] {
  const ridge = (peaks: number, high: number): Point[] => {
    const main: Point[] = [];
    const count = peaks * 2;
    for (let k = 0; k <= count; k += 1) {
      const x = EDGE + ((FAR_EDGE - EDGE) * k) / count + (k === 0 || k === count ? 0 : between(rng, -2.5, 2.5));
      const rise = k % 2 === 1 ? between(rng, 0.62, 1) : between(rng, 0.12, 0.4);
      main.push([x, at.y - at.s * high * rise]);
    }
    const points: Point[] = [];
    main.forEach((point, k) => {
      points.push(point);
      const next = main[k + 1];
      if (next === undefined) return;
      const t = between(rng, 0.35, 0.65);
      points.push([point[0] + (next[0] - point[0]) * t, point[1] + (next[1] - point[1]) * t + between(rng, -0.1, 0.1) * high * at.s]);
    });
    return points;
  };
  const body = (points: readonly Point[]): string => BOX_FRAME.poly([[EDGE, FAR_EDGE], ...points, [FAR_EDGE, FAR_EDGE]]);
  const peaks = (points: readonly Point[]): number[] => points.flatMap((_, i) => (i % 4 === 2 ? [i] : []));
  const caps = (points: readonly Point[]): string => {
    let d = "";
    for (const i of peaks(points)) {
      const [px, py] = points[i] ?? [0, 0];
      const [ax, ay] = points[i - 1] ?? [px, py];
      const [bx, by] = points[i + 1] ?? [px, py];
      const c = between(rng, 0.4, 0.6);
      const left: Point = [px + (ax - px) * c, py + (ay - py) * c];
      const right: Point = [px + (bx - px) * c, py + (by - py) * c];
      const drip = (t: number, drop: number): Point => [right[0] + (left[0] - right[0]) * t, right[1] + (left[1] - right[1]) * t + drop * at.s];
      d += BOX_FRAME.poly([[px, py], right, drip(0.3, between(rng, 0.8, 1.8)), drip(0.55, between(rng, 0.1, 0.5)), drip(0.78, between(rng, 0.8, 1.8)), left]);
    }
    return d;
  };
  const back = ridge(whole(rng, 4, 5), 31);
  const front = ridge(whole(rng, 3, 4), 26);
  let lit = "";
  let gullies = "";
  for (const i of peaks(front)) {
    const [px, py] = front[i] ?? [0, 0];
    const [ax, ay] = front[i - 1] ?? [px, py];
    lit += BOX_FRAME.poly([[px, py], [ax, ay], [px + 1.2 * at.s, py + (ay - py) * 1.6]]);
    const [vx, vy] = front[i + 2] ?? [px, py];
    if (i + 2 < front.length - 1) gullies += BOX_FRAME.band([[vx, vy + 0.6], [vx + 2 * at.s, vy + 6 * at.s], [vx + at.s, vy + 12 * at.s]], 0.6 * at.s);
  }
  const out = [
    shape(body(back), "slate-700", 0.5, MAIN),
    shape(caps(back), "slate-100", 0.45),
    shape(body(front), "slate-700", 1, MAIN_INKED),
    shape(lit, "slate-400", 0.3),
    shape(caps(front), "white", 0.9),
  ];
  if (gullies !== "") out.push(shape(gullies, "slate-900", 0.35));
  return out;
}

/** The inside of a cave's mouth: dark rock over the top and both sides of the box, its opening centred on the placement's x and about 60 wide at scale 1, its ceiling 8 to 14 above the placement's y and its longest stalactite hanging down to it. */
function cave(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hw = 30;
  const foot = Math.max(4, (FAR_EDGE - at.y) / at.s);
  const j = (): number => between(rng, -1.5, 1.5);
  const up: Point = [0, -1];
  const stalactite = (x: number, base: number, tip: number, width: number): [Point, Point][] => [
    [[x - width, base], up],
    [[x, tip], up],
    [[x + width, base], up],
  ];
  const mainX = hw * 0.1 + j();
  const edge: [Point, Point][] = [
    [[-hw - 2, foot], [-1, 0]],
    [[-hw + 1 + j(), foot * 0.6], [-1, 0]],
    [[-hw + 3 + j(), foot * 0.25], [-1, 0]],
    [[-hw * 0.9, -2 + j()], [-1, -1]],
    [[-hw * 0.72, -8 + j()], up],
    ...stalactite(-hw * 0.45 + j(), -11, between(rng, -5, -2), 2.2),
    [[-hw * 0.18, -14 + j()], up],
    ...stalactite(mainX, -13.5, 0, 2.6),
    [[hw * 0.38, -12.5 + j()], up],
    ...(rng() < 0.6 ? stalactite(hw * 0.6 + j(), -10, between(rng, -6, -3), 1.7) : []),
    [[hw * 0.8, -7 + j()], up],
    [[hw * 0.92, -1.5 + j()], [1, -1]],
    [[hw - 3 + j(), foot * 0.25], [1, 0]],
    [[hw - 1 + j(), foot * 0.6], [1, 0]],
    [[hw + 2, foot], [1, 0]],
  ];
  const clamp = (value: number): number => Math.min(FAR_EDGE - 1, Math.max(EDGE + 1, value));
  const frame = (push: number): string => {
    const points = edge.map(([[x, y], [nx, ny]]) => pt(clamp(f.x(x + nx * push)), Math.min(FAR_EDGE, clamp(f.y(y + ny * push)))));
    const ordered = at.flip === 1 ? [...points].reverse() : points;
    return `M${pt(EDGE, FAR_EDGE)}L${pt(EDGE, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, FAR_EDGE)}L${ordered.join("L")}Z`;
  };
  const wall = (from: number, to: number, push: number): Point[] => edge.slice(from, to).map(([[x, y], [nx, ny]]) => [x + nx * push, y + ny * push]);
  const out = [
    shape(frame(0), "slate-800", 1, MAIN),
    shape(frame(3.5), "slate-900", 1, MAIN),
    shape(f.band(wall(0, 4, 0.6), 1) + f.band(wall(edge.length - 4, edge.length, 0.6), 1.2), "slate-400", 0.28),
    shape(
      f.band([[-hw - 14, foot * 0.2], [-hw - 8, foot * 0.35], [-hw - 11, foot * 0.55]], 0.7) +
        f.band([[hw + 9, foot * 0.15], [hw + 13, foot * 0.4], [hw + 8, foot * 0.6]], 0.7) +
        f.band([[-hw * 0.5, -21], [-hw * 0.1, -19.5], [hw * 0.35, -22]], 0.7),
      "slate-700",
      0.45,
    ),
  ];
  if (rng() < 0.5) out.push(shape(`M${f.p(mainX, 1.2)}Q${f.p(mainX + 0.7, 2.6)} ${f.p(mainX, 3)}Q${f.p(mainX - 0.7, 2.6)} ${f.p(mainX, 1.2)}Z`, "sky-300", 0.7));
  return out;
}

/** A frozen floor from the placement's y down across the box: pale ice, frost patches, cracks and a glint. */
function iceFloor(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const patches = whole(rng, 3, 5);
  const slot = (FAR_EDGE - EDGE) / patches;
  let frost = "";
  for (let i = 0; i < patches; i += 1) {
    const t = between(rng, 0.3, 0.92);
    const rx = Math.min(slot * 0.45, (5 + 10 * t) * at.s);
    const cx = EDGE + slot * (i + 0.5) + between(rng, -1, 1) * (slot * 0.5 - rx);
    frost += ellipsePath(cx, top + depth * t, rx, Math.min(rx * between(rng, 0.18, 0.28), depth * 0.08));
  }
  const crack = (x0: number, dir: number): Point[] => {
    const points: Point[] = [[x0, top + depth * between(rng, 0.2, 0.4)]];
    for (let k = 0; k < 4; k += 1) {
      const [x, y] = points[k] ?? [x0, top];
      const near = 0.5 + (y - top) / depth;
      points.push([x + dir * between(rng, 0.5, 3.5) * near * at.s, y + between(rng, 1.5, 3.5) * near * at.s]);
    }
    return points;
  };
  const branch = (points: readonly Point[], dir: number): Point[] => {
    const [x, y] = points[2] ?? [0, top];
    return [[x, y], [x + dir * 2.2 * at.s, y + 1.6 * at.s], [x + dir * 3 * at.s, y + 3.6 * at.s]];
  };
  const left = crack(between(rng, 12, 38), -1);
  const right = crack(between(rng, 62, 88), 1);
  const width = 0.45 * at.s;
  const gx = between(rng, 15, 40);
  const gy = top + depth * between(rng, 0.35, 0.6);
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "blue-200", 1, MAIN),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth * 0.22), "sky-300", 0.3),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 0.5 * at.s), "white", 0.6),
    shape(frost, "slate-100", 0.6),
    shape(BOX_FRAME.band(left, width) + BOX_FRAME.band(right, width), "sky-300", 0.85),
    shape(BOX_FRAME.band(branch(left, 1), width * 0.7) + BOX_FRAME.band(branch(right, -1), width * 0.7), "sky-300", 0.7),
    shape(`M${pt(gx - 12 * at.s, gy + 1)}Q${pt(gx, gy + 0.2)} ${pt(gx + 12 * at.s, gy + 1)}Q${pt(gx, gy + 1.4)} ${pt(gx - 12 * at.s, gy + 1)}Z`, "white", 0.35),
    shape(sparkle(BOX_FRAME, gx, gy, 2.4 * at.s) + sparkle(BOX_FRAME, between(rng, 62, 85), top + depth * between(rng, 0.12, 0.25), 1.4 * at.s), "white", 0.9),
  ];
}

// Architecture

/** A temple front standing on the anchor: three steps, four columns before a dark doorway, an entablature and a pediment; about 40 wide and 32 tall at scale 1. */
function temple(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const floor = 4.5;
  const head = 21.5;
  const beam = 25;
  const apex = between(rng, 30.6, 32.4);
  const xs = [-13, -4.5, 4.5, 13];
  const column = (x: number): string =>
    f.poly([
      [x - 1.7, -floor],
      [x + 1.7, -floor],
      [x + 1.7, -floor - 0.8],
      [x + 1.25, -floor - 0.8],
      [x + 1.1, -head + 1.1],
      [x + 2.1, -head],
      [x - 2.1, -head],
      [x - 1.1, -head + 1.1],
      [x - 1.25, -floor - 0.8],
      [x - 1.7, -floor - 0.8],
    ]);
  const steps: Point[] = [
    [-20, 0],
    [20, 0],
    [20, -1.5],
    [18.5, -1.5],
    [18.5, -3],
    [17, -3],
    [17, -floor],
    [-17, -floor],
    [-17, -3],
    [-18.5, -3],
    [-18.5, -1.5],
    [-20, -1.5],
  ];
  const triglyph = (x: number): string => rect(f, x - 0.4, -beam + 0.8, x + 0.4, -head - 0.8);
  const out = [
    shape(rect(f, -16, -head, 16, -floor), "slate-700", 0.9),
    shape(roundHead(f, 2.6, -12.4, -floor), rng() < 0.3 ? "orange-400" : "black", 0.8),
    shape(xs.map(column).join(""), "stone-500", 1, MAIN_INKED),
    shape(xs.map((x) => rect(f, x + 0.35, -head + 1.2, x + 1.1, -floor - 0.9)).join(""), "slate-900", 0.3),
    shape(xs.map((x) => rect(f, x - 0.95, -head + 1.2, x - 0.5, -floor - 0.9)).join(""), "slate-100", 0.3),
    shape(f.band([[-16, -head + 0.7], [16, -head + 0.7]], 1.2), "slate-900", 0.25),
    shape(f.poly(steps) + rect(f, -18.5, -beam, 18.5, -head) + f.poly([[-20, -beam], [20, -beam], [0, -apex]]), "stone-500", 1, MAIN_INKED),
    shape([-17, -8.75, 0, 8.75, 17].map(triglyph).join("") + f.poly([[-15.5, -beam - 1], [15.5, -beam - 1], [0, -apex + 1.7]]), "slate-700", 0.4),
    shape(f.band([[-20, -beam - 0.4], [0, -apex + 0.3]], 0.5), "slate-100", 0.3),
  ];
  if (rng() < 0.5) {
    const x = between(rng, -14, 10);
    out.push(shape(f.band([[x, -head - 0.2], [x + 1.2, -head - 1.8], [x + 0.6, -beam + 0.3]], 0.3), "slate-900", 0.55));
  }
  return out;
}

/** A crooked stone townhouse standing on the anchor: walls 20 wide, a steep slate roof, a chimney and one lit window, its door left to a `door` or `doorway` placed at the anchor; about 24 wide and 30 tall at scale 1. */
function house(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.05, 0.05);
  const j = (): number => between(rng, -0.6, 0.6);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const poly = (points: readonly Point[]): string => f.poly(points.map(([dx, dy]) => P(dx, dy)));
  const eave = -15.5 + j();
  const apex = -29 + j();
  const ridge = j() * 1.5;
  const gable = rng() < 0.5;
  const [wx, wy] = gable ? [ridge * 0.4, eave - 4.6] : [rng() < 0.5 ? -6 : 6, -10.5];
  const pane = (l: number, t: number, r: number, b: number): string => poly([[wx + l, wy + t], [wx + r, wy + t], [wx + r, wy + b], [wx + l, wy + b]]);
  const stones: string[] = [];
  for (const [k, y] of [-2.5, -6, -9.5, -13].entries()) {
    const x = (k % 2 === 0 ? -1 : 1) * between(rng, 5.5, 8);
    stones.push(f.band([P(x - 1.2, y), P(x + 1.2, y)], 0.35));
  }
  return [
    shape(poly([[5, eave], [7.4, eave], [7.4, -26], [7.9, -26], [7.9, -27.1], [4.5, -27.1], [4.5, -26], [5, -26]]), "stone-500", 1, INKED),
    shape(poly([[-10, 0], [10, 0], [10 + j() * 0.5, eave], [-10 + j() * 0.5, eave]]), "stone-500", 1, MAIN_INKED),
    shape(poly([[6.5, 0], [10, 0], [10, eave], [6.5, eave]]), "slate-900", 0.25),
    shape(stones.join(""), "slate-700", 0.45),
    shape(f.band([P(-10, eave + 1.4), P(10, eave + 1.4)], 1), "slate-900", 0.3),
    shape(poly([[-12, eave + 0.9], [12, eave + 0.9 + j() * 0.5], [ridge, apex]]), "slate-700", 1, INKED),
    shape(poly([[-12, eave + 0.9], [ridge, apex], [ridge - 2, eave + 0.9]]), "slate-400", 0.25),
    shape(f.circle(...P(wx, wy), 3.4), "orange-400", 0.22),
    shape(pane(-1.5, -1.8, 1.5, 1.8), "orange-400", 0.95),
    shape(pane(-0.2, -1.8, 0.2, 1.8) + pane(-1.5, -0.2, -0.2, 0.2) + pane(0.2, -0.2, 1.5, 0.2), "wood-800", 0.9),
  ];
}

/** A closed planked door in a stone frame standing on the anchor, iron bands and a ring; about 7 wide and 12 tall at scale 1. */
function door(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pointed = rng() < 0.4;
  const spring = pointed ? -7.8 : -8.5;
  const head = (half: number): string =>
    pointed
      ? `M${f.p(-half, 0)}L${f.p(-half, spring)}Q${f.p(-half, spring - half * 0.9)} ${f.p(0, spring - half * 1.25)}Q${f.p(half, spring - half * 0.9)} ${f.p(half, spring)}L${f.p(half, 0)}Z`
      : roundHead(f, half, spring);
  const crown = (half: number): number => spring - half * (pointed ? 1.25 : 1);
  const plank = (x: number): string => f.band([[x, -0.3], [x, spring - Math.sqrt(2.6 ** 2 - x ** 2) * 0.85]], 0.2);
  const straps = [between(rng, -2.8, -2.2), between(rng, -7.4, -6.8)];
  return [
    shape(head(2.6), "wood-700", 1, MAIN_INKED),
    shape([-1.3, 0, 1.3].map(plank).join(""), "wood-900", 0.55),
    shape(rect(f, -2.6, spring, -2, 0), "slate-900", 0.3),
    shape(straps.map((y) => f.band([[-2.6, y], [1.2, y]], 0.6)).join("") + f.circle(1.5, -5.6, 0.35), "slate-800", 1),
    shape(ring(f, 1.5, -5, 0.75, 0.45), "slate-400", 1),
    shape(head(3.5) + head(2.6), "stone-500", 1, INKED),
    shape([-3, -6].map((y) => f.band([[-3.5, y], [-2.6, y]], 0.25) + f.band([[2.6, y + 0.8], [3.5, y + 0.8]], 0.25)).join(""), "slate-700", 0.5),
    shape(f.poly([[-0.7, crown(3.5) - 0.3], [0.7, crown(3.5) - 0.3], [0.45, crown(2.6) + 0.3], [-0.45, crown(2.6) + 0.3]]), "slate-400", 1, INKED),
  ];
}

/** An open doorway standing on the anchor: a stone frame, its door swung in ajar at one side, a dark room with a warm light that spills 2 before it; about 8 wide and 12 tall at scale 1. */
function doorway(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const side = rng() < 0.5 ? -1 : 1;
  const spring = -8;
  const flicker = between(rng, -0.3, 0.3);
  const leaf: Point[] = [
    [side * 3, 0.2],
    [side * 3, -11],
    [side * 1.2, -9.8],
    [side * 1.2, -0.7],
  ];
  const across = (t: number, y0: number, y1: number): Point => [side * (3 - 1.8 * t), y0 + (y1 - y0) * t];
  return [
    shape(f.poly([[-2.9, 0], [2.9, 0], [4.4, 2.2], [-4.4, 2.2]]), "orange-400", 0.18),
    shape(roundHead(f, 3, spring), "slate-900", 1),
    shape(f.ellipse(-side * 0.7 + flicker, -3.8, 2.2, 3.8), "orange-400", 0.35),
    shape(f.ellipse(-side * 0.9 + flicker, -3.2, 1.1, 2.2), "orange-400", 0.6),
    shape(f.poly(leaf), "wood-700", 1, MAIN_INKED),
    shape(f.band([across(0, -3, -3.2), across(1, -3, -3.2)], 0.5) + f.band([across(0, -8, -7.6), across(1, -8, -7.6)], 0.5), "slate-800", 0.9),
    shape(roundHead(f, 4, spring) + roundHead(f, 3, spring), "stone-500", 1, INKED),
    shape([-3, -6].map((y) => f.band([[-4, y], [-3, y]], 0.25) + f.band([[3, y + 0.8], [4, y + 0.8]], 0.25)).join(""), "slate-700", 0.5),
    shape(f.poly([[-0.75, -12.3], [0.75, -12.3], [0.5, -10.7], [-0.5, -10.7]]), "slate-400", 1, INKED),
  ];
}

/** An arched stone bridge, its deck at the anchor: two or three arches in a body 16 deep below it, a parapet 4 above; about 60 wide at scale 1. */
function bridge(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 30;
  const depth = 16;
  const pier = 3.4;
  const arches = whole(rng, 2, 3);
  const span = (2 * half - (arches + 1) * pier) / arches;
  const spring = depth - between(rng, 3, 4.5);
  const rise = Math.min(span / 2, spring - 4);
  const lefts = Array.from({ length: arches }, (_, k) => -half + pier + k * (span + pier));
  const over = (from: number, to: number, lift: number): string =>
    `C${f.p(from, spring - lift * 1.33)} ${f.p(to, spring - lift * 1.33)} ${f.p(to, spring)}`;
  let body = `M${f.p(-half, 0)}L${f.p(half, 0)}L${f.p(half, depth)}`;
  let rings = "";
  let soffits = "";
  for (const l of [...lefts].reverse()) {
    const r = l + span;
    body += `L${f.p(r, depth)}L${f.p(r, spring)}${over(r, l, rise)}L${f.p(l, depth)}`;
    rings += `M${f.p(r + 1.5, spring)}${over(r + 1.5, l - 1.5, rise + 1.5)}L${f.p(l, spring)}${over(l, r, rise)}Z`;
    soffits += `M${f.p(r, spring)}${over(r, l, rise)}L${f.p(l + 1, spring)}${over(l + 1, r - 1, rise - 1)}Z`;
  }
  body += `L${f.p(-half, depth)}Z`;
  const parapet = f.poly([[-half, 0], [-half, -3.2], [-half - 0.6, -3.2], [-half - 0.6, -4], [half + 0.6, -4], [half + 0.6, -3.2], [half, -3.2], [half, 0]]);
  const joints = [-20, -10, 0, 10, 20].map((x) => f.band([[x + between(rng, -2, 2), -3], [x, -0.3]], 0.3)).join("");
  return [
    shape(body + parapet, "stone-500", 1, MAIN_INKED),
    shape(rings, "slate-400", 0.35),
    shape(soffits, "slate-900", 0.45),
    shape(f.band([[-half - 0.4, -3.6], [half + 0.4, -3.6]], 0.5), "slate-100", 0.3),
    shape(f.band([[-half, 0.6], [half, 0.6]], 0.7) + joints, "slate-700", 0.45),
    shape(f.ellipse(between(rng, -25, -6), -4, between(rng, 1.2, 2.2), 0.5) + f.ellipse(between(rng, 6, 25), -4, between(rng, 1, 2), 0.45), "green-700", 0.6),
  ];
}

/** A carved stone altar block standing on the anchor, a red cloth runner down its front, often two candles on it; about 16 wide and 9 tall at scale 1 (its candles 3 more). */
function altar(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const runner = between(rng, 1.8, 2.6);
  const drop = between(rng, 2, 3.4);
  const block: Point[] = [
    [-8, 0],
    [8, 0],
    [8, -1.2],
    [7, -1.2],
    [7, -7.6],
    [8.2, -7.6],
    [8.2, -9],
    [-8.2, -9],
    [-8.2, -7.6],
    [-7, -7.6],
    [-7, -1.2],
    [-8, -1.2],
  ];
  const panel = (l: number, r: number): string => rect(f, l, -6.8, r, -2) + rect(f, l + 0.4, -6.4, r - 0.4, -2.4);
  const out = [
    shape(f.poly(block), "stone-500", 1, MAIN_INKED),
    shape(panel(-6.2, -runner - 0.7) + panel(runner + 0.7, 6.2), "slate-700", 0.45),
    shape(rect(f, 4.6, -7.6, 7, -1.2), "slate-900", 0.3),
    shape(f.band([[-8.2, -8.75], [8.2, -8.75]], 0.4), "slate-100", 0.35),
    shape(f.poly([[-runner, -9.2], [runner, -9.2], [runner, -drop], [0, -drop + 1.2], [-runner, -drop]]), "red-600", 1, INKED),
    shape(f.band([[-runner, -drop - 0.7], [runner, -drop - 0.7]], 0.35) + f.poly([[0, -6.9], [0.7, -5.9], [0, -4.9], [-0.7, -5.9]]), "brass-600", 0.9),
  ];
  if (rng() < 0.6) {
    const flame = (x: number): string => `M${f.p(x, -12.3)}Q${f.p(x + 0.45, -11.4)} ${f.p(x, -11.1)}Q${f.p(x - 0.45, -11.4)} ${f.p(x, -12.3)}Z`;
    out.push(shape(f.circle(-5.5, -11.6, 1.6) + f.circle(5.5, -11.6, 1.6), "orange-400", 0.2));
    out.push(shape(rect(f, -5.9, -11.1, -5.1, -9) + rect(f, 5.1, -10.6, 5.9, -9), "slate-100", 1, INKED));
    out.push(shape(flame(-5.5) + `M${f.p(5.5, -11.8)}Q${f.p(5.95, -10.9)} ${f.p(5.5, -10.6)}Q${f.p(5.05, -10.9)} ${f.p(5.5, -11.8)}Z`, "orange-400", 1));
  }
  return out;
}

/** A slanted wooden reading stand on a pillar foot standing on the anchor, its desk rising to the right as drawn facing right, often an open book on it; about 6 wide and 11 tall at scale 1. */
function lectern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.3, 0.3);
  const desk: Point[] = [
    [-3.2, -8.1 + tilt],
    [3, -10.3 - tilt],
    [3.1, -11 - tilt],
    [-2.8, -8.85 + tilt],
    [-2.9, -9.5 + tilt],
    [-3.5, -9.4 + tilt],
  ];
  const out = [
    shape(f.poly([[-2.6, 0], [2.6, 0], [1.8, -0.9], [-1.8, -0.9]]), "wood-800", 1, INKED),
    shape(f.poly([[-0.6, -0.9], [0.6, -0.9], [0.5, -7.4], [2.6, -9.6], [2.6, -10.2], [-0.5, -8.6]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[0.3, -1.2], [0.25, -7.6]], 0.35), "wood-900", 0.45),
    shape(f.poly(desk), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-3, -8.6 + tilt], [2.9, -10.75 - tilt]], 0.25), "wood-600", 0.8),
  ];
  if (rng() < 0.75) {
    out.push(shape(`M${f.p(-2.3, -9.1 + tilt)}Q${f.p(-1, -10.6)} ${f.p(0.2, -10.1)}Q${f.p(1.4, -11.4)} ${f.p(2.6, -11.1 - tilt)}L${f.p(2.5, -10.6 - tilt)}L${f.p(-2.3, -8.8 + tilt)}Z`, "slate-100", 1, INKED));
    out.push(shape(f.band([[0.2, -10.1], [0.1, -9.6]], 0.2), "slate-400", 0.8));
  }
  return out;
}

/** A tall wooden bookshelf standing on the anchor, four rows of book spines; about 14 wide and 22 tall at scale 1. */
function bookshelf(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const inner = 6.1;
  const plinth = 1.2;
  const crown = 20.7;
  const shelf = 0.6;
  const bay = (crown - plinth - 3 * shelf) / 4;
  const colours: readonly PaletteName[] = ["red-600", "blue-800", "green-700", "brass-600"];
  const spines = colours.map(() => "");
  let backs = "";
  for (let row = 0; row < 4; row += 1) {
    const bottom = -plinth - row * (bay + shelf);
    backs += rect(f, -inner, bottom - bay, inner, bottom);
    let x = -inner + between(rng, 0, 0.5);
    const end = inner - between(rng, 0.5, 3);
    while (x < end - 0.6) {
      const width = Math.min(between(rng, 0.9, 1.7), end - x);
      const k = whole(rng, 0, colours.length - 1);
      spines[k] = (spines[k] ?? "") + rect(f, x, bottom - bay * between(rng, 0.62, 0.92), x + width, bottom);
      x += width + 0.12;
    }
  }
  return [
    shape(f.poly([[-7, 0], [7, 0], [7, -crown], [7.5, -crown], [7.5, -22], [-7.5, -22], [-7.5, -crown], [-7, -crown]]), "wood-700", 1, MAIN_INKED),
    shape(backs, "wood-900", 0.95),
    ...colours.flatMap((paint, k) => (spines[k] === "" ? [] : [shape(spines[k] ?? "", paint, 1)])),
    shape(rect(f, inner - 1, -crown, inner, -plinth), "wood-900", 0.4),
    shape(f.band([[-7.5, -21.8], [7.5, -21.8]], 0.4) + rect(f, -7, -crown, -6.4, 0), "wood-600", 0.7),
  ];
}

/** A long wooden table standing on the anchor; about 22 wide and 8 tall at scale 1. */
function table(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 11 * between(rng, 0.93, 1.05);
  const splay = between(rng, 0, 0.4);
  const leg = (x: number, dir: number): string => f.poly([[x - 0.55, -5.6], [x + 0.55, -5.6], [x + 0.45 + dir * splay, 0], [x - 0.45 + dir * splay, 0]]);
  return [
    shape(f.ellipse(0, 0.2, half - 1, 0.9), "black", 0.25),
    shape(f.poly([[-half + 3.3, -5.6], [-half + 4.2, -5.6], [-half + 4.1, -0.5], [-half + 3.4, -0.5]]) + f.poly([[half - 4.2, -5.6], [half - 3.3, -5.6], [half - 3.4, -0.5], [half - 4.1, -0.5]]), "wood-800", 1, INKED),
    shape(rect(f, -half, -8, half, -6.8) + rect(f, -half + 1.2, -6.8, half - 1.2, -5.6) + leg(-half + 1.6, -1) + leg(half - 1.6, 1), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-half, -7.8], [half, -7.8]], 0.4), "wood-600", 0.9),
    shape(f.band([[-half + 1.2, -6.55], [half - 1.2, -6.55]], 0.5) + rect(f, half - 1.6, -5.6, half - 1.2 + splay, -0.3), "wood-900", 0.45),
    shape(f.band([[between(rng, -8, 0), -7.35], [between(rng, 2, 8), -7.3]], 0.15), "wood-800", 0.6),
  ];
}

/** A stone warrior statue on a plinth standing on the anchor: a helmeted figure with a round shield and a spear or a planted sword, facing the way the part faces; about 8 wide and 22 tall at scale 1. */
function statue(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spear = rng() < 0.5;
  const plinth: Point[] = [
    [-4, 0],
    [4, 0],
    [4, -1.1],
    [3.3, -1.1],
    [3.3, -4.6],
    [3.8, -4.6],
    [3.8, -5.4],
    [-3.8, -5.4],
    [-3.8, -4.6],
    [-3.3, -4.6],
    [-3.3, -1.1],
    [-4, -1.1],
  ];
  const body: Point[] = [
    [-1.7, -5.4],
    [-0.25, -5.4],
    [0, -8.8],
    [0.45, -5.4],
    [2.1, -5.4],
    [1.4, -6.2],
    [1.5, -9.8],
    [2.3, -10.6],
    [1.7, -15.6],
    [2, -16.6],
    [0.7, -17.4],
    [-0.7, -17.4],
    [-2, -16.6],
    [-1.8, -15.6],
    [-2.4, -10.6],
    [-1.6, -9.8],
    [-1.55, -6.2],
  ];
  const weapon = spear
    ? f.band([[3, -5.6], [3, -21]], 0.45) + f.poly([[2.55, -20.9], [3, -22.6], [3.45, -20.9]])
    : f.band([[0.4, -6.2], [0.4, -12.6]], 0.6) + f.band([[-0.9, -12.85], [1.7, -12.85]], 0.4) + f.band([[0.4, -13.05], [0.4, -14.1]], 0.35);
  const arms = spear ? f.band([[1.7, -16.2], [2.6, -13.4], [3, -14.6]], 0.75) + f.circle(3, -14.6, 0.5) : f.band([[1.8, -16.2], [1.5, -14.2], [0.6, -14.4]], 0.75) + f.circle(0.4, -14.5, 0.5);
  const crest = f.band([[1.1, -19.9], [0, -21.1], [-1.4, -20.8], [-2.3, -19.5]], 0.6);
  const lichen = between(rng, -2.6, 2.6);
  return [
    shape(f.poly(plinth), "stone-500", 1, MAIN_INKED),
    shape(f.poly(body), "stone-500", 1, MAIN_INKED),
    shape(weapon, "stone-500", 1, MAIN_INKED),
    shape(arms + crest, "stone-500", 1, MAIN_INKED),
    shape(f.circle(0.1, -18.6, 1.25) + f.circle(-2.3, -13.2, 2.3), "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(-1.45, -18.2)}C${f.p(-1.45, -20.4)} ${f.p(1.45, -20.4)} ${f.p(1.45, -18.2)}L${f.p(0.9, -18.4)}L${f.p(-1.45, -18.2)}Z`, "stone-500", 1, MAIN_INKED),
    shape(f.circle(-2.3, -13.2, 0.6) + rect(f, -2, -3.6, 2, -2.4), "slate-700", 0.45),
    shape(f.poly([[0.6, -5.4], [2.1, -5.4], [1.4, -6.2], [1.5, -9.8], [2.3, -10.6], [1.7, -15.6], [2, -16.6], [0.9, -17.2], [0.8, -15.2], [1.1, -10.4]]) + rect(f, 2.2, -4.6, 3.3, -1.1), "slate-900", 0.3),
    shape(f.band([[-1.3, -6.4], [-1.3, -9.7]], 0.35) + f.band([[-3.3, -14.6], [-3.9, -13], [-3.4, -11.6]], 0.4), "slate-100", 0.3),
    shape(f.ellipse(lichen, -5.1, 1.1, 0.35) + f.ellipse(lichen > 0 ? -2.6 : 2.6, -0.9, 0.9, 0.3), "green-700", 0.6),
  ];
}

/** A tall rough menhir standing on the anchor, a spiral carved in it and lichen on it; about 6 wide and 16 tall at scale 1. */
function standingStone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.08, 0.08);
  const j = (): number => between(rng, -0.35, 0.35);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const outline: Point[] = [P(-3 + j(), 0), P(-3.2 + j(), -5), P(-2.7 + j(), -10.5), P(-1.7 + j(), -14.8), P(0.2 + j(), -16 + j()), P(1.8 + j(), -14.6), P(2.7 + j(), -9.5), P(3.1 + j(), -4), P(3 + j(), 0)];
  const start = rng() * 360;
  const turn = rng() < 0.5 ? 1 : -1;
  const spiral: Point[] = [];
  for (let i = 0; i <= 16; i += 1) {
    const t = i / 16;
    const [x, y] = polar(0.3 + 1.7 * t, start + turn * t * 2.2 * 360, 0, -8.5);
    spiral.push(P(x, y));
  }
  const lichen = (x: number, y: number, rx: number): string => f.ellipse(...P(x, y), rx, rx * 0.6);
  return [
    shape(f.ellipse(0, 0.2, 4.5, 0.8), "black", 0.25),
    shape(f.poly(outline), "stone-500", 1, MAIN_INKED),
    shape(f.poly([P(0.6, -15.2), ...outline.slice(5), P(1, 0)]), "slate-900", 0.3),
    shape(f.band([P(-2.4, -1), P(-2.6, -5), P(-2.1, -10.4), P(-1.2, -14.4)], 0.5), "slate-100", 0.3),
    shape(f.band(spiral, 0.35), "slate-800", 0.6),
    shape(lichen(-1.8 + j(), -2.6, 1.1) + lichen(1.4, -12.4 + j(), 0.75) + lichen(-1, -14 + j() * 0.5, 0.5), "green-700", 0.8),
  ];
}

/** An iron portcullis in a stone arch standing on the anchor, its grid of bars down or raised a little; about 16 wide and 18 tall at scale 1. */
function portcullis(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 5.4;
  const spring = -10;
  const raised = rng() < 0.4 ? between(rng, 1, 3) : 0;
  const topAt = (x: number): number => spring - Math.sqrt(Math.max(0, half ** 2 - x ** 2)) + 0.2;
  const xs = [-4.32, -2.16, 0, 2.16, 4.32];
  const bars = xs.map((x) => f.poly([[x - 0.35, topAt(Math.abs(x) + 0.35)], [x + 0.35, topAt(Math.abs(x) + 0.35)], [x + 0.35, -raised - 1], [x, -raised], [x - 0.35, -raised - 1]])).join("");
  let rails = "";
  for (const lift of [2.5, 6, 9.5, 13]) {
    const y = -raised - lift;
    const reach = (y >= spring ? half : Math.sqrt(Math.max(0, half ** 2 - (y - spring) ** 2))) - 0.15;
    if (reach > 1) rails += f.band([[-reach, y], [reach, y]], 0.55);
  }
  const joints = [200, 240, 300, 340].map((a) => f.band([polar(half + 0.2, a, 0, spring), polar(7.8, a, 0, spring)], 0.25)).join("");
  return [
    shape(roundHead(f, half, spring), "black", 0.8),
    shape(bars, "slate-800", 1),
    shape(rails, "slate-800", 1),
    shape(xs.map((x) => f.band([[x - 0.15, topAt(Math.abs(x)) + 0.4], [x - 0.15, -raised - 1.2]], 0.2)).join(""), "slate-400", 0.55),
    shape(roundHead(f, 8, spring) + roundHead(f, half, spring), "stone-500", 1, MAIN_INKED),
    shape(joints + f.band([[-8, -4], [-half, -4]], 0.25) + f.band([[half, -6], [8, -6]], 0.25), "slate-700", 0.5),
    shape(rect(f, -half - 0.9, spring, -half, 0), "slate-900", 0.3),
    shape(f.poly([[-1, -18.3], [1, -18.3], [0.6, -15.1], [-0.6, -15.1]]), "slate-400", 1, INKED),
  ];
}

/** A stone hearth standing on the anchor: a fire in its arched mouth, a mantel, and a hood rising to its chimney, its glow spilling before it; about 20 wide and 18 tall at scale 1. */
function hearth(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 4.5;
  const spring = -3.5;
  const r = fmt(f.r(half));
  const block = `M${f.p(-10, 0)}L${f.p(-half, 0)}L${f.p(-half, spring)}A${r} ${r} 0 0 ${f.sweep(1)} ${f.p(half, spring)}L${f.p(half, 0)}L${f.p(10, 0)}L${f.p(10, -10)}L${f.p(-10, -10)}Z`;
  const hood = f.poly([[-11, -10], [11, -10], [11, -11.2], [8.5, -11.2], [3, -15.4], [3, -18], [-3, -18], [-3, -15.4], [-8.5, -11.2], [-11, -11.2]]);
  const j = (): number => between(rng, -0.6, 0.6);
  const tongues = (scale: number, lift: number): string =>
    `M${f.p(-3.2 * scale, -0.9)}Q${f.p(-3 * scale, -2.6 * scale - lift)} ${f.p(-2.2 * scale, -3.6 * scale - lift + j())}Q${f.p(-1.6 * scale, -2.4 * scale)} ${f.p(-0.4 * scale, -5.6 * scale - lift + j())}` +
    `Q${f.p(0.6 * scale, -2.6 * scale)} ${f.p(1.6 * scale, -4.4 * scale - lift + j())}Q${f.p(2.4 * scale, -2.4 * scale)} ${f.p(3.2 * scale, -0.9)}Z`;
  return [
    shape(f.circle(0, -4, 11), "orange-400", 0.08),
    shape(f.ellipse(0, 0.6, 8, 1.4), "orange-400", 0.2),
    shape(block + hood, "stone-500", 1, MAIN_INKED),
    shape(roundHead(f, half, spring), "black", 0.9),
    shape(f.ellipse(0, -2.5, 4, 3), "orange-400", 0.35),
    shape(f.band([[-3.2, -0.5], [3.2, -0.5]], 1) + f.band([[-2.4, -1.45], [2.2, -1.5]], 0.9), "wood-800", 1),
    shape(tongues(1, 0), "ember-500", 0.9),
    shape(tongues(0.6, 0), "orange-400", 0.95),
    shape([-6.6, -3.6].map((y) => f.band([[-10, y], [-half - 1, y]], 0.25) + f.band([[half + 1, y + 1.2], [10, y + 1.2]], 0.25)).join(""), "slate-700", 0.45),
    shape(rect(f, 7.4, -10, 10, 0) + f.band([[-8.5, -11.6], [8.5, -11.6]], 0.6), "slate-900", 0.3),
    shape(f.ellipse(0, -9.8, 3.2, 1.2), "black", 0.3),
  ];
}

/** A crystal server obelisk standing on the anchor: a dark slab with rows of cyan status lights, crowned by a violet crystal; about 8 wide and 24 tall at scale 1. */
function serverRack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const halfAt = (y: number): number => 3.8 - (0.6 * (-1 - y)) / 17.6;
  let grooves = "";
  let lights = "";
  for (let k = 0; k < 7; k += 1) {
    const y = -3 - k * 2.2;
    const w = halfAt(y) - 0.4;
    grooves += f.band([[-w, y + 1], [w, y + 1]], 0.3);
    lights += rect(f, -w + 0.3, y - 0.2, -w + 0.8, y + 0.25);
    const strip = between(rng, 0.8, 3);
    if (rng() < 0.8) lights += rect(f, -w + 1.2, y - 0.1, -w + 1.2 + strip, y + 0.15);
  }
  return [
    shape(f.circle(0, -21.6, 4.2), "violet-300", 0.18),
    shape(rect(f, -4.4, -1, 4.4, 0) + f.poly([[-3.6, -18.6], [3.6, -18.6], [3, -19.4], [-3, -19.4]]), "slate-700", 1, INKED),
    shape(f.poly([[-3.8, -1], [3.8, -1], [3.2, -18.6], [-3.2, -18.6]]), "slate-800", 1, MAIN_INKED),
    shape(grooves, "slate-900", 0.7),
    shape(f.band([[-halfAt(-3) + 0.95, -2.2], [-halfAt(-17) + 0.95, -17.6]], 1.6), "cyan-400", 0.15),
    shape(lights, "cyan-400", 1),
    shape(f.poly([[2.2, -1], [3.8, -1], [3.2, -18.6], [2, -18.6]]), "slate-900", 0.35),
    shape(f.band([[-3.6, -1.4], [-3.05, -18.2]], 0.35), "slate-400", 0.3),
    shape(f.poly([[-2, -19.4], [2, -19.4], [2.4, -21 + between(rng, -0.3, 0.3)], [0, -24], [-2.4, -21]]), "violet-300", 1, INKED),
    shape(f.poly([[-2, -19.4], [-0.3, -19.4], [0, -24], [-2.4, -21]]), "white", 0.3),
  ];
}

/** A carved stone ring centred on the anchor, about 30 across and 6 thick, spirals carved round it and blocks on its rim, its middle open for a vortex drawn behind or before it. */
function portalRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const outer = 15;
  const inner = 9;
  const start = between(rng, 0, 60);
  const turn = rng() < 0.5 ? 1 : -1;
  let spirals = "";
  let blocks = "";
  for (let k = 0; k < 6; k += 1) {
    const a = start + k * 60;
    const [cx, cy] = polar(12, a);
    const arm: Point[] = [];
    for (let i = 0; i <= 6; i += 1) arm.push(polar(0.25 + 1.35 * (i / 6), a + turn * (i / 6) * 470, cx, cy));
    spirals += f.band(arm, 0.4);
    const b = a + 30;
    blocks += f.poly([polar(outer - 1, b - 4.6), polar(outer + 1.6, b - 3.6), polar(outer + 1.6, b + 3.6), polar(outer - 1, b + 4.6)]);
  }
  const lit = [150, 170, 190, 210, 230, 250].map((a) => polar(outer - 0.7, a));
  return [
    shape(ring(f, 0, 0, outer, inner), "stone-500", 1, MAIN_INKED),
    shape(ring(f, 0, 0, inner + 1.1, inner), "slate-900", 0.35),
    shape(f.band(lit, 1), "slate-100", 0.3),
    shape(spirals, "slate-700", 0.6),
    shape(blocks, "slate-400", 1, INKED),
  ];
}

/** A glowing circle of runes lying on the ground, centred on the anchor: a flat ellipse about 30 wide and 8 tall, two rings and rune marks between them, its glow a little past it. */
function runeCircle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flat = 0.27;
  const onGround = (r: number, degrees: number): Point => {
    const [x, y] = polar(r, degrees);
    return [x, y * flat];
  };
  const start = between(rng, 0, 30);
  let runes = "";
  for (let k = 0; k < 12; k += 1) {
    const a = start + k * 30;
    const kind = whole(rng, 0, 3);
    const glyph: Point[] =
      kind === 0
        ? [onGround(11.2, a), onGround(13.4, a)]
        : kind === 1
          ? [onGround(13.4, a - 3), onGround(11.2, a), onGround(13.4, a + 3)]
          : kind === 2
            ? [onGround(11.2, a - 3), onGround(13.4, a), onGround(11.2, a + 3)]
            : [onGround(11.2, a - 3), onGround(13.4, a - 3), onGround(11.2, a + 3), onGround(13.4, a + 3)];
    runes += f.band(glyph, 0.45);
  }
  return [
    shape(f.ellipse(0, 0, 18, 18 * flat + 1.2), "violet-300", 0.12, MAIN),
    shape(f.ellipse(0, 0, 13, 13 * flat), "violet-300", 0.14, MAIN),
    shape(f.ellipse(0, 0, 15, 15 * flat) + f.ellipse(0, 0, 14.2, 14.2 * flat), "violet-300", 0.9, MAIN),
    shape(f.ellipse(0, 0, 10.6, 10.6 * flat) + f.ellipse(0, 0, 10, 10 * flat), "violet-300", 0.8, MAIN),
    shape(runes, "violet-300", 1, MAIN),
    shape(f.ellipse(0, 0, 3, 3 * flat) + f.ellipse(0, 0, 2.5, 2.5 * flat), "white", 0.5),
    shape(sparkle(f, between(rng, -9, -3), between(rng, -4, -1.5), 1.2) + sparkle(f, between(rng, 3, 9), between(rng, -5, -2), 0.9), "white", 0.8),
  ];
}

/** A translucent bubble dome standing on the anchor on a slim base, about 40 wide and 22 tall at scale 1: its faint fill, a grid of lines, a bright rim and a highlight on its left. */
function dome(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rx = 20;
  const ry = between(rng, 21, 23);
  const on = (x: number, lat: number): Point => [x * Math.cos(lat * DEGREES), -ry * Math.sin(lat * DEGREES)];
  const lats = [0, 12, 24, 36, 48, 60, 72];
  const meridians = whole(rng, 2, 3);
  let grid = "";
  for (let k = -meridians; k <= meridians; k += 1) {
    const x = (rx * k) / (meridians + 1);
    grid += f.band(lats.map((lat) => on(x, lat)), 0.3);
  }
  let rings = "";
  for (const lat of [22, 46]) {
    const y = -ry * Math.sin(lat * DEGREES);
    const w = rx * Math.cos(lat * DEGREES);
    const bulge = 1.2 * Math.cos(lat * DEGREES);
    rings += f.band([[-w, y], [-w * 0.7, y - bulge * 0.5], [0, y - bulge], [w * 0.7, y - bulge * 0.5], [w, y]], 0.3);
  }
  const rim = [0, 15, 30, 45, 60, 75, 90, 105, 120, 135, 150, 165, 180].map((a) => polar(1, 180 + a)).map(([x, y]): Point => [x * (rx - 0.3), y * (ry - 0.3)]);
  const shine = between(rng, -2, 2);
  return [
    shape(`M${f.p(-rx, 0)}A${fmt(f.r(rx))} ${fmt(f.r(ry))} 0 0 ${f.sweep(1)} ${f.p(rx, 0)}Z`, "sky-300", 0.18, MAIN),
    shape(grid, "white", 0.2),
    shape(rings, "white", 0.2),
    shape(f.band(rim, 0.7), "white", 0.55),
    shape(f.band([on(-rx * 0.8, 18 + shine), on(-rx * 0.8, 36 + shine), on(-rx * 0.8, 54 + shine), on(-rx * 0.8, 66 + shine)], 1.6), "white", 0.45),
    shape(f.circle(...on(-rx * 0.5, 62 + shine), 0.9), "white", 0.7),
    shape(f.band([[-rx - 0.4, -0.3], [rx + 0.4, -0.3]], 1.2), "slate-400", 1, INKED),
  ];
}

/** A carved stone fortune wheel centred on the anchor, about 28 across: a pegged rim, eight spokes between red, green, blue and violet segments, a hub, a pointer on top and a lever reaching 20 to its right. */
function fortuneWheel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const outer = 14;
  const rim = 12;
  const hub = 2.6;
  const gap = 0.8;
  const turn = between(rng, 0, 45);
  const wedge = (k: number): string => {
    const a = turn + k * 45;
    const near = hub + 0.4;
    const out = Math.asin(gap / rim) / DEGREES;
    const inn = Math.asin(gap / near) / DEGREES;
    const big = fmt(f.r(rim));
    const small = fmt(f.r(near));
    return (
      `M${f.p(...polar(near, a + inn))}L${f.p(...polar(rim, a + out))}A${big} ${big} 0 0 ${f.sweep(1)} ${f.p(...polar(rim, a + 45 - out))}` +
      `L${f.p(...polar(near, a + 45 - inn))}A${small} ${small} 0 0 ${f.sweep(0)} ${f.p(...polar(near, a + inn))}Z`
    );
  };
  const segments: readonly PaletteName[] = ["red-600", "green-700", "blue-800", "violet-500"];
  const pegs = [0, 1, 2, 3, 4, 5, 6, 7].map((k) => f.circle(...polar(13, turn + k * 45), 0.5)).join("");
  return [
    shape(f.circle(0, 0, outer), "stone-500", 1, MAIN_INKED),
    ...segments.map((paint, k) => shape(wedge(k) + wedge(k + 4), paint, 1)),
    shape(pegs, "brass-500", 1),
    shape(f.circle(0, 0, hub), "stone-500", 1, INKED),
    shape(f.circle(0, 0, 1), "slate-700", 1),
    shape(f.band([polar(outer - 0.8, 160), polar(outer - 0.8, 190), polar(outer - 0.8, 220), polar(outer - 0.8, 250)], 1), "slate-100", 0.3),
    shape(f.band([[13.2, 1.5], [16.5, -1.5], [19, -6]], 1) + f.poly([[13.4, 0.4], [15.6, 0.4], [15.6, 3.2], [13.4, 3.2]]), "wood-700", 1, INKED),
    shape(f.circle(19.3, -6.6, 1) + f.poly([[-1.1, -outer - 1.8], [1.1, -outer - 1.8], [0, -outer + 1.2]]), "red-600", 1, INKED),
  ];
}

/** A clock face centred on the anchor, about 20 across: a stone rim, a pale face with its twelve hour marks, two hands and a boss. */
function clockFace(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const face = 8.3;
  const hour = whole(rng, 0, 11);
  const minute = whole(rng, 0, 11) * 5 + whole(rng, 0, 4);
  let ticks = "";
  for (let k = 0; k < 12; k += 1) {
    const major = k % 3 === 0;
    ticks += f.band([polar(major ? 5.9 : 6.5, k * 30 - 90), polar(7.5, k * 30 - 90)], major ? 0.75 : 0.4);
  }
  const hand = (degrees: number, length: number, width: number): string => {
    const [ux, uy] = polar(1, degrees);
    const P = (along: number, side: number): Point => [ux * along - uy * side, uy * along + ux * side];
    return f.poly([P(-1.2, width * 0.8), P(length * 0.72, width), P(length, 0), P(length * 0.72, -width), P(-1.2, -width * 0.8)]);
  };
  const notches = [15, 45, 75, 105, 135, 165, 195, 225, 255, 285, 315, 345].map((a) => f.band([polar(face + 0.3, a), polar(9.7, a)], 0.25)).join("");
  return [
    shape(f.circle(0, 0, face), "slate-100", 1),
    shape(ring(f, 0, 0, 10, face), "stone-500", 1, MAIN_INKED),
    shape(notches, "slate-700", 0.45),
    shape(f.band([polar(9.2, 160), polar(9.2, 190), polar(9.2, 220), polar(9.2, 250)], 0.6), "slate-100", 0.35),
    shape(f.circle(0.9, 0.9, face - 0.9) + f.circle(0, 0, face - 0.9), "slate-400", 0.25),
    shape(ticks, "slate-900", 0.9),
    shape(hand(hour * 30 + minute / 2 - 90, 4.4, 0.55), "slate-900", 1),
    shape(hand(minute * 6 - 90, 6.6, 0.38), "slate-900", 1),
    shape(f.circle(0, 0, 0.9), "brass-600", 1),
  ];
}

/** A toppled stone column lying on its side on the anchor, its capital at the back end and its shaft broken off at the front, chips of it up to 4 beyond; about 26 long and 5 tall at scale 1. */
function fallenColumn(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.5, 0.5);
  const breakX = 11 + j();
  const broken: Point[] = [
    [breakX - 0.4 + j(), -4.4],
    [breakX + 1.4 + j(), -3.5],
    [breakX + 0.6 + j(), -2.3],
    [breakX + 2 + j(), -1.2],
    [breakX + 1.2, 0],
  ];
  const outline: Point[] = [[-13, 0.3], [-13, -5.4], [-11.9, -5.4], [-11.9, -5], [-10.6, -4.4], ...broken, [-10.6, 0], [-11.9, 0.3]];
  const face: Point[] = broken.map(([x, y]) => [x - 0.9, y]);
  const out = [
    shape(f.ellipse(0, 0.3, 14, 0.8), "black", 0.25),
    shape(f.poly(outline), "stone-500", 1, MAIN_INKED),
    shape(f.poly([...broken, ...face.reverse()]), "slate-100", 0.3),
    shape([-3.3, -2.1, -0.9].map((y) => f.band([[-10.2, y], [breakX - 1.2, y]], 0.3)).join(""), "slate-700", 0.35),
    shape(f.band([[-10.4, -3.95], [breakX - 0.6, -3.95]], 0.6), "slate-100", 0.3),
    shape(f.band([[-10.4, -0.45], [breakX + 0.8, -0.45]], 0.9) + rect(f, -12.5, -5, -11.9, 0), "slate-900", 0.3),
  ];
  const chips = whole(rng, 1, 3);
  let rubble = "";
  for (let k = 0; k < chips; k += 1) {
    const x = breakX + 3 + k * 1.5 + between(rng, 0, 0.5);
    const size = between(rng, 0.45, 0.8);
    rubble += f.poly([[x - size, 0], [x - size * 0.6, -size * 1.2], [x + size * 0.7, -size], [x + size, 0]]);
  }
  out.push(shape(rubble, "stone-500", 1, INKED));
  if (rng() < 0.5) out.push(shape(f.ellipse(between(rng, -6, 4), -4.3, 1.8, 0.45), "green-700", 0.6));
  return out;
}

/** A carved stone throne standing on the anchor: a tall pointed back with finials, arms and a seat on a step, a red drape down its back; about 12 wide and 18 tall at scale 1. */
function throne(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const peak = -18 + between(rng, -0.5, 0.5);
  const back: Point[] = [
    [-4.4, -6.4],
    [4.4, -6.4],
    [4.4, -15.4],
    [4.9, -15.6],
    [4.5, -17],
    [3.9, -15.9],
    [0.8, -16.7],
    [0, peak],
    [-0.8, -16.7],
    [-3.9, -15.9],
    [-4.5, -17],
    [-4.9, -15.6],
    [-4.4, -15.4],
  ];
  const arm = (side: number): string => f.poly([[side * 4.4, -6.4], [side * 6.2, -6.4], [side * 6.2, -9.6], [side * 6.5, -10.2], [side * 4.4, -10.2]]);
  const crest = rng() < 0.5 ? f.poly([[0, -14.6], [0.8, -13.6], [0, -12.6], [-0.8, -13.6]]) : f.circle(0, -13.6, 0.75);
  return [
    shape(f.poly(back) + rect(f, -6.2, -1.2, 6.2, 0) + rect(f, -5.2, -6.4, 5.2, -1.2) + arm(-1) + arm(1), "stone-500", 1, MAIN_INKED),
    shape(f.circle(-5.3, -9.2, 0.45) + f.circle(5.3, -9.2, 0.45) + f.band([[-4.6, -3.8], [4.6, -3.8]], 0.3) + rect(f, -3.6, -15.2, -3.2, -7), "slate-700", 0.45),
    shape(rect(f, 3.6, -15.4, 4.4, -10.2) + rect(f, 4.6, -9.6, 6.2, -6.4) + rect(f, 4, -6.4, 5.2, -1.2), "slate-900", 0.3),
    shape(f.band([[-4.15, -6.8], [-4.15, -15]], 0.35) + f.band([[-6.2, -1], [6.2, -1]], 0.3), "slate-100", 0.3),
    shape(f.poly([[-2.6, -15.6], [2.6, -15.6], [2.6, -7.2], [0, -8.4], [-2.6, -7.2]]) + rect(f, -4.4, -6.9, 4.4, -6.4), "red-600", 1, INKED),
    shape(f.band([[-2.6, -15.1], [2.6, -15.1]], 0.4) + crest, "brass-600", 0.9),
  ];
}

/** A round stone fountain standing on the anchor: a wide basin, a pillar with a small bowl, and a jet rising 14 above the ground and falling back; about 20 wide and 14 tall at scale 1. */
function fountain(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spray = between(rng, -0.5, 0.5);
  const arc = (side: number): Point[] => [
    [side * 7.4, -5.2],
    [side * 6.2, -9.5 + spray],
    [side * 4.2, -12.6 + spray],
    [side * 2, -13.8],
  ];
  const jet: Point[] = [...arc(-1), [0, -14], ...arc(1).reverse()];
  const falls = f.band([[-3.6, -9.2], [-3.9, -6.6], [-4.1, -4.4]], 0.5) + f.band([[3.6, -9.2], [3.9, -6.6], [4.1, -4.4]], 0.5);
  return [
    shape(`M${f.p(-10, -4)}L${f.p(10, -4)}L${f.p(10, -1.2)}Q${f.p(10, 0)} ${f.p(8.8, 0)}L${f.p(-8.8, 0)}Q${f.p(-10, 0)} ${f.p(-10, -1.2)}Z`, "stone-500", 1, MAIN_INKED),
    shape(rect(f, 6.8, -4, 10, -0.6), "slate-900", 0.3),
    shape(f.band([[-9.6, -3], [-9.6, -1]], 0.6), "slate-100", 0.3),
    shape(f.ellipse(0, -4, 9, 1.25), "sky-300", 0.7),
    shape(f.ellipse(0, -4, 10.2, 1.7) + f.ellipse(0, -4, 9, 1.25), "slate-400", 1, INKED),
    shape(f.poly([[-0.7, -7.95], [0.7, -7.95], [1, -4.6], [-1, -4.6]]) + `M${f.p(-3.6, -9.2)}Q${f.p(0, -6.8)} ${f.p(3.6, -9.2)}Z`, "stone-500", 1, MAIN_INKED),
    shape(f.band(jet, 1.3) + falls, "sky-300", 0.45),
    shape(f.band(jet, 0.45), "white", 0.6),
    shape(f.band([[0, -9.3], [0, -13.6]], 0.8), "white", 0.55),
    shape(f.ellipse(-7.4, -4.2, 1.4, 0.4) + f.ellipse(7.4, -4.2, 1.4, 0.4) + f.circle(-6.4 + spray, -6.2, 0.3) + f.circle(6.6, -6.8 - spray, 0.3), "white", 0.6),
  ];
}

/** A low bed standing on the anchor: a stone frame, a pale blanket over it and a pillow at its head (behind, as drawn facing right); about 16 wide and 6 tall at scale 1. */
function cot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lump = rng() < 0.4 ? between(rng, 0.8, 1.4) : 0;
  const frame =
    `M${f.p(-8.4, 0)}L${f.p(-8.4, -5.2)}Q${f.p(-7.8, -6.2)} ${f.p(-7.2, -5.2)}L${f.p(-7.2, 0)}Z` +
    rect(f, -7.2, -2.8, 7, -1.4) +
    rect(f, 7, -3.6, 8, 0) +
    rect(f, -1, -1.4, 0, 0);
  const blanket = `M${f.p(-4.6, -3.4)}Q${f.p(-2, -3.9 - lump)} ${f.p(1.2, -3.7 - lump * 0.6)}Q${f.p(4.4, -3.5)} ${f.p(7.4, -3.7)}L${f.p(7.9, -2.2)}L${f.p(7.5, -1.5)}L${f.p(-4.4, -1.7)}Z`;
  return [
    shape(f.ellipse(0, 0.2, 8.6, 0.7), "black", 0.25),
    shape(frame, "stone-500", 1, INKED),
    shape(f.ellipse(-5.6, -3.6, 1.7, 0.9), "white", 1, INKED),
    shape(blanket, "slate-100", 1, MAIN_INKED),
    shape(f.band([[-2.6, -3.2], [-1.6, -2]], 0.25) + f.band([[2.4, -3.3], [3.2, -2]], 0.25) + f.band([[6.6, -3.3], [7, -2]], 0.25), "slate-400", 0.6),
    shape(rect(f, 4.6, -2.8, 7, -1.4) + rect(f, -7.6, -5.2, -7.2, 0), "slate-900", 0.3),
  ];
}

/** A market stall standing on the anchor: a wooden counter with goods on it under a striped cloth awning on two poles; about 24 wide and 20 tall at scale 1. */
function awning(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stripes = rng() < 0.5 ? 7 : 9;
  const sag = between(rng, 1.2, 2);
  const back = -19.5;
  const front = -15.4;
  let red = "";
  let pale = "";
  for (let k = 0; k < stripes; k += 1) {
    const t0 = k / stripes;
    const t1 = (k + 1) / stripes;
    const tx = (t: number): number => -11 + 22 * t;
    const bx = (t: number): number => -12 + 24 * t;
    const d = `M${f.p(tx(t0), back)}L${f.p(tx(t1), back)}L${f.p(bx(t1), front)}Q${f.p((bx(t0) + bx(t1)) / 2, front + sag * 2)} ${f.p(bx(t0), front)}Z`;
    if (k % 2 === 0) red += d;
    else pale += d;
  }
  let goods = "";
  let more = "";
  const count = whole(rng, 3, 5);
  for (let k = 0; k < count; k += 1) {
    const x = -8.5 + (17 * (k + 0.5)) / count;
    const r = between(rng, 0.8, 1.1);
    if (k % 2 === 0) goods += f.circle(x, -7.5 - r, r);
    else more += f.circle(x, -7.5 - r, r);
  }
  return [
    shape(f.band([[-10, 0], [-10, -18.8]], 0.9) + f.band([[10, 0], [10, -18.8]], 0.9), "wood-800", 1, INKED),
    shape(rect(f, -10.5, -6.6, 10.5, 0), "wood-600", 1, INKED),
    shape(rect(f, -11.2, -7.5, 11.2, -6.6), "wood-700", 1, INKED),
    shape([-6, -1.5, 3, 7.5].map((x) => f.band([[x, -6.3], [x, -0.4]], 0.25)).join("") + rect(f, 7.8, -6.6, 10.5, 0), "wood-800", 0.5),
    shape(goods, rng() < 0.5 ? "red-500" : "orange-400", 1),
    shape(more === "" ? f.circle(0, -8.2, 0.7) : more, "green-500", 1),
    shape(red, "red-600", 1, MAIN_INKED),
    shape(pale, "slate-100", 1, INKED),
    shape(f.band([[-11, back + 0.6], [11, back + 0.6]], 0.6), "white", 0.25),
  ];
}

// Big things

/** A grey fighter jet in flight centred on the anchor, its nose the way it faces: swept wings, tail fins, a cockpit glint and its engine's glow; about 30 long and 10 tall at scale 1. */
function jet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pitch = between(rng, -7, 7);
  const poly = (points: readonly Point[]): string => f.poly(points.map((point) => turned(point, pitch)));
  const band = (points: readonly Point[], width: number): string => f.band(points.map((point) => turned(point, pitch)), width);
  const out = [
    shape(poly([[2, -0.8], [-6.5, -0.9], [-9, -3.8], [-7, -3.8]]) + poly([[-10.5, -0.6], [-14.2, -0.8], [-15, -2.4], [-13.5, -2.4]]), "slate-700", 1, INKED),
    shape(poly([[-8.6, -1.4], [-12.4, -6], [-14.2, -6], [-13.6, -1.2]]), "slate-400", 1, MAIN_INKED),
    shape(
      poly([[15, 0.2], [11, -0.9], [5.5, -1.5], [-3, -1.6], [-12, -1.3], [-14.6, -1.1], [-15.2, -0.6], [-15.2, 0.9], [-12, 1.3], [-2, 1.6], [2, 1.9], [8, 1.2], [12, 0.6]]),
      "slate-400",
      1,
      MAIN_INKED,
    ),
    shape(band([[-13, 0.95], [-2, 1.2], [11, 0.55]], 0.7), "slate-700", 0.45),
    shape(band([[1, 0.6], [-2.4, 0.7]], 0.6), "slate-900", 0.7),
    shape(poly([[4, 0.5], [-6, 0.8], [-8.5, 4.2], [-6.2, 4.2]]) + poly([[-11, 0.8], [-14.8, 0.9], [-15.6, 2.6], [-14, 2.6]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly(([[3.2, -1.4], [6, -2.9], [8.4, -2.4], [9.6, -1]] as const).map((point) => turned(point, pitch))), "slate-900", 1, INKED),
    shape(poly([[5.4, -2.2], [6.6, -2.7], [7.4, -2.5], [6.2, -2.1]]), "white", 0.85),
  ];
  if (rng() < 0.6) out.push(shape(poly([[-15.2, -0.5], [-17.6, 0.15], [-15.2, 0.8]]), "orange-400", 0.85));
  if (rng() < 0.5) out.push(shape(band([[-12.2, -4.5], [-13.6, -4.5]], 0.6), "red-600", 1));
  return out;
}

/** A two-wheeled wooden hand cart standing on the anchor: its bed over a spoked wheel (the far one peeking behind), two handles reaching forward and a stand under them, often a load on it; about 16 wide and 9 tall at scale 1. */
function cart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = between(rng, 0, 60);
  const [wx, wy] = [-1.8, -3.1];
  const spokes = [0, 60, 120, 180, 240, 300].map((a) => f.band([polar(0.6, turn + a, wx, wy), polar(2.6, turn + a, wx, wy)], 0.35)).join("");
  const out = [
    shape(ring(f, wx + 0.8, wy - 0.3, 3.1, 2.5), "wood-900", 1),
    shape(f.band([[2.8, -5], [8.8, -3.6]], 0.6) + f.band([[2.6, -3.4], [2.8, 0]], 0.6), "wood-800", 1, INKED),
    shape(f.poly([[-6.4, -3.4], [3.2, -3.4], [3.8, -7.6], [-6.8, -7.6]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-6.5, -5], [3.4, -5]], 0.3) + f.band([[-6.7, -6.5], [3.6, -6.5]], 0.3), "wood-800", 0.6),
  ];
  if (rng() < 0.6) {
    out.push(shape(`M${f.p(-6.2, -7.6)}Q${f.p(-5.6, -9.4)} ${f.p(-2.6, -9.2)}Q${f.p(0.6, -9.8)} ${f.p(3.2, -7.6)}Z`, rng() < 0.5 ? "brass-600" : "wood-600", 1, INKED));
  }
  out.push(
    shape(f.band([[3, -4.4], [9.4, -2.4]], 0.65), "wood-600", 1, INKED),
    shape(ring(f, wx, wy, 3.1, 2.5), "slate-700", 1, INKED),
    shape(spokes, "wood-700", 1),
    shape(f.circle(wx, wy, 0.75), "slate-700", 1),
  );
  return out;
}

/** A chest of stone drawers standing on the anchor: three rows of drawers with knobs under a top slab; about 12 wide and 14 tall at scale 1. */
function cabinet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drawers: [number, number, number, number][] = [
    [-4.9, -12.3, -0.25, -9.2],
    [0.25, -12.3, 4.9, -9.2],
    [-4.9, -8.6, 4.9, -5],
    [-4.9, -4.4, 4.9, -1.5],
  ];
  const groove = ([l, t, r, b]: [number, number, number, number]): string => rect(f, l, t, r, b) + rect(f, l + 0.3, t + 0.3, r - 0.3, b - 0.3);
  const knobs = [f.circle(-2.6, -10.75, 0.42), f.circle(2.6, -10.75, 0.42), f.circle(-2.4, -6.8, 0.42), f.circle(2.4, -6.8, 0.42), f.circle(-2.4, -2.95, 0.42), f.circle(2.4, -2.95, 0.42)];
  const out = [
    shape(rect(f, -5.6, -13, 5.6, -0.8) + rect(f, -5.6, -0.8, -4.2, 0) + rect(f, 4.2, -0.8, 5.6, 0) + rect(f, -6.2, -14, 6.2, -13), "stone-500", 1, MAIN_INKED),
    shape(drawers.map(groove).join(""), "slate-800", 0.55),
    shape(knobs.join(""), "brass-600", 1),
    shape(rect(f, 4, -13, 5.6, -0.8), "slate-900", 0.3),
    shape(rect(f, -5.6, -13, -5, -0.8) + f.band([[-6.2, -13.8], [6.2, -13.8]], 0.3), "slate-100", 0.3),
  ];
  const ajar = drawers[whole(rng, 0, drawers.length - 1)];
  if (ajar !== undefined && rng() < 0.4) out.push(shape(rect(f, ajar[0] + 0.3, ajar[1] + 0.3, ajar[2] - 0.3, ajar[1] + 0.75), "black", 0.55));
  if (rng() < 0.5) {
    const x = between(rng, -4, 3);
    out.push(shape(f.band([[x, -13], [x + 0.6, -11.8], [x + 0.2, -10.6]], 0.25), "slate-900", 0.5));
  }
  return out;
}

/** R3390000: the places the Classic+ scenes added, by id, each with its kind. */
export const PLACES_KIT = {
  water: { category: "terrain", draw: water },
  stairs: { category: "terrain", draw: stairs },
  mountains: { category: "terrain", draw: mountains },
  cave: { category: "terrain", draw: cave },
  "ice-floor": { category: "terrain", draw: iceFloor },
  temple: { category: "architecture", draw: temple },
  house: { category: "architecture", draw: house },
  door: { category: "architecture", draw: door },
  doorway: { category: "architecture", draw: doorway },
  bridge: { category: "architecture", draw: bridge },
  altar: { category: "architecture", draw: altar },
  lectern: { category: "architecture", draw: lectern },
  bookshelf: { category: "architecture", draw: bookshelf },
  table: { category: "architecture", draw: table },
  statue: { category: "architecture", draw: statue },
  "standing-stone": { category: "architecture", draw: standingStone },
  portcullis: { category: "architecture", draw: portcullis },
  hearth: { category: "architecture", draw: hearth },
  "server-rack": { category: "architecture", draw: serverRack },
  "portal-ring": { category: "architecture", draw: portalRing },
  "rune-circle": { category: "architecture", draw: runeCircle },
  dome: { category: "architecture", draw: dome },
  "fortune-wheel": { category: "architecture", draw: fortuneWheel },
  "clock-face": { category: "architecture", draw: clockFace },
  "fallen-column": { category: "architecture", draw: fallenColumn },
  throne: { category: "architecture", draw: throne },
  fountain: { category: "architecture", draw: fountain },
  cot: { category: "architecture", draw: cot },
  awning: { category: "architecture", draw: awning },
  jet: { category: "objects", draw: jet },
  cart: { category: "objects", draw: cart },
  cabinet: { category: "objects", draw: cabinet },
} as const satisfies Readonly<Record<string, KitPart>>;
