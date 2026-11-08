// R3401000: the scene kit's ink-world places (ART.md, "Scene recipes and kits"), which the Meditative scenes (#3401)
// added: terrain that spans the box from its placement's height down (ink-wash peaks in mist, rice terraces, a raked
// gravel bed, cracked earth, a tatami floor, a still lake), a needle of rock standing on its anchor; and the ink
// world's buildings and garden stones, each standing on its anchor (a pagoda, a pavilion, a moon bridge, a shoji
// panel, a vermilion gate, a well, a village house, a stone lantern) but the moon window, centred on it.
// sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css name, none in Radiant's gold window; a
// glow is a stack of translucent shapes, never a gradient; shapes are filled even-odd, so a hole is a second
// sub-path and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, EDGE, FAR_EDGE, INKED, leant, local, MAIN, MAIN_INKED, pt, rectPath, shape, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** The box as a frame of its own, for terrain drawn in the box's coordinates. */
const BOX = local({ x: 0, y: 0, s: 1, flip: 1 });

/** The middle of the box's span, where a terrain's curves put their control points. */
const MID = (EDGE + FAR_EDGE) / 2;

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** A brush stroke from `a` to `b` tapering to both ends: a lens `width` across its middle, bowed `bow` to its left. */
function stroke(f: Local, [ax, ay]: Point, [bx, by]: Point, width: number, bow = 0): string {
  const length = Math.hypot(bx - ax, by - ay) || 1;
  const nx = -(by - ay) / length;
  const ny = (bx - ax) / length;
  const mx = (ax + bx) / 2 + nx * bow * 2;
  const my = (ay + by) / 2 + ny * bow * 2;
  return `M${f.p(ax, ay)}Q${f.p(mx + nx * width, my + ny * width)} ${f.p(bx, by)}Q${f.p(mx - nx * width, my - ny * width)} ${f.p(ax, ay)}Z`;
}

/** A bank of mist in the box: a long mound from `x0` to `x1` on `y`, `h` high and a third of that below. */
function streak(x0: number, x1: number, y: number, h: number): string {
  const xm = (x0 + x1) / 2;
  return `M${pt(x0, y)}Q${pt(xm, y - h * 2)} ${pt(x1, y)}Q${pt(xm, y + h * 0.6)} ${pt(x0, y)}Z`;
}

/** A curve across the box: its y at the left and right edges and how far its middle rises above their chord. */
type Contour = { readonly left: number; readonly right: number; readonly lift: number };

/** The band of the box between two contours, `upper` above `lower`. */
function strip(upper: Contour, lower: Contour): string {
  const control = (c: Contour): number => (c.left + c.right) / 2 - 2 * c.lift;
  return (
    `M${pt(EDGE, upper.left)}Q${pt(MID, control(upper))} ${pt(FAR_EDGE, upper.right)}` +
    `L${pt(FAR_EDGE, lower.right)}Q${pt(MID, control(lower))} ${pt(EDGE, lower.left)}Z`
  );
}

/** A contour moved down by `dy`. */
function lowered(c: Contour, dy: number): Contour {
  return { left: c.left + dy, right: c.right + dy, lift: c.lift };
}

/** A point along a cubic from `a` to `b` through the handles `c1` and `c2`. */
function bezier(a: Point, c1: Point, c2: Point, b: Point, t: number): Point {
  const u = 1 - t;
  const at = (k: 0 | 1): number => u * u * u * a[k] + 3 * u * u * t * c1[k] + 3 * u * t * t * c2[k] + t * t * t * b[k];
  return [at(0), at(1)];
}

// Ink-wash ranges: each a run of bell-shaped peaks across the box.

/** One peak of a range in the box: its summit's x and y and how far its flanks reach either side. */
type Peak = { readonly x: number; readonly top: number; readonly half: number };

/** A node of a range's top edge: a summit (`peak`), a saddle between two peaks, or a foot. */
type RangeNode = { readonly x: number; readonly y: number; readonly peak: boolean };

/** A range's top edge, left to right: its feet at and past the box's edges, each summit, a saddle where two peaks meet and a foot either side of a gap. */
function rangeNodes(peaks: readonly Peak[], foot: number, rng: Rng): RangeNode[] {
  const sorted = [...peaks].sort((a, b) => a.x - b.x);
  const nodes: RangeNode[] = [];
  const add = (x: number, y: number, peak = false): void => {
    nodes.push({ x, y, peak });
  };
  sorted.forEach((p, i) => {
    const prev = sorted[i - 1];
    if (prev === undefined) {
      if (p.x - p.half > EDGE) add(EDGE, foot);
      add(p.x - p.half, foot);
    } else if (prev.x + prev.half <= p.x - p.half) {
      add(prev.x + prev.half, foot);
      add(p.x - p.half, foot);
    } else {
      const low = Math.max(prev.top, p.top);
      add(prev.x + ((p.x - prev.x) * prev.half) / (prev.half + p.half), low + (foot - low) * between(rng, 0.4, 0.65));
    }
    add(p.x, p.top, true);
  });
  const last = sorted[sorted.length - 1];
  if (last !== undefined) {
    add(last.x + last.half, foot);
    if (last.x + last.half < FAR_EDGE) add(FAR_EDGE, foot);
  }
  return nodes;
}

/** The handles of the cubic from `a` to `b` when either is a summit (a bell, level at both ends), else none (a straight run). */
function handles(a: RangeNode, b: RangeNode): readonly [Point, Point] | undefined {
  if (!a.peak && !b.peak) return undefined;
  const dx = b.x - a.x;
  return [
    [a.x + dx * (a.peak ? 0.3 : 0.5), a.y],
    [b.x - dx * (b.peak ? 0.3 : 0.5), b.y],
  ];
}

/** A range's top edge through its nodes, each moved down by `drop(node)`, as path commands from its first node to its last (or back). */
function rangeEdge(nodes: readonly RangeNode[], drop: (node: RangeNode) => number, back: boolean): string {
  const steps: string[] = [];
  for (let i = 1; i < nodes.length; i += 1) {
    const a = nodes[i - 1];
    const b = nodes[i];
    if (a === undefined || b === undefined) continue;
    const h = handles(a, b);
    const [from, to] = back ? [b, a] : [a, b];
    const [h1, h2] = h === undefined ? [undefined, undefined] : back ? [h[1], h[0]] : h;
    const end = pt(to.x, to.y + drop(to));
    steps.push(h1 === undefined || h2 === undefined ? `L${end}` : `C${pt(h1[0], h1[1] + drop(from))} ${pt(h2[0], h2[1] + drop(to))} ${end}`);
  }
  return (back ? steps.reverse() : steps).join("");
}

/** A range's wash: its top edge and its body down past the box's bottom. */
function rangeBody(nodes: readonly RangeNode[]): string {
  const first = nodes[0] ?? { x: EDGE, y: FAR_EDGE, peak: false };
  const last = nodes[nodes.length - 1] ?? first;
  return `M${pt(first.x, first.y)}${rangeEdge(nodes, () => 0, false)}L${pt(last.x, FAR_EDGE)}L${pt(first.x, FAR_EDGE)}Z`;
}

/** A band under a range's top edge, `drop(node)` deep at each node: the ink gathered on a crest, or the brush line along it. */
function rangeCrest(nodes: readonly RangeNode[], drop: (node: RangeNode) => number): string {
  const first = nodes[0] ?? { x: EDGE, y: FAR_EDGE, peak: false };
  const last = nodes[nodes.length - 1] ?? first;
  return `M${pt(first.x, first.y)}${rangeEdge(nodes, () => 0, false)}L${pt(last.x, last.y + drop(last))}${rangeEdge(nodes, drop, true)}Z`;
}

// Terrain

/** The set's backdrop: three to five ink-wash peaks across the box rising up to about 30 above the placement's y, the farther ones higher, paler and smaller, mist along their feet, and the nearest wash (main, slate-700) brushed along its ridge and filling the box below the placement's y. */
function inkPeaks(at: PartPlace, rng: Rng): KitShape[] {
  const s = at.s;
  const x = (dx: number): number => 50 + at.flip * (dx - 50);
  const count = whole(rng, 3, 5);
  const peak = (dx: number, high: number, half: number): Peak => ({ x: x(dx), top: at.y - s * high, half });
  const far = [peak(between(rng, 38, 50), between(rng, 27, 30), between(rng, 9, 11))];
  if (count >= 4) far.push(peak(between(rng, 84, 94), between(rng, 21, 24), between(rng, 8, 10)));
  const mid = [peak(between(rng, 62, 72), between(rng, 17, 20), between(rng, 12, 15))];
  const near = [peak(between(rng, 16, 28), between(rng, 11, 14), between(rng, 21, 25))];
  if (count === 5) near.push(peak(between(rng, 80, 90), between(rng, 6, 8), between(rng, 13, 16)));
  const farFoot = at.y - 9 * s;
  const midFoot = at.y - 3.5 * s;
  const farNodes = rangeNodes(far, farFoot, rng);
  const midNodes = rangeNodes(mid, midFoot, rng);
  const nearNodes = rangeNodes(near, at.y - between(rng, 0.8, 2) * s, rng);
  const depth = (summit: number, saddle: number, foot: number) => (node: RangeNode) =>
    s * (node.peak ? summit : node.x <= EDGE || node.x >= FAR_EDGE ? foot : saddle);
  let cun = "";
  let dots = "";
  for (let i = 1; i < nearNodes.length; i += 1) {
    const a = nearNodes[i - 1];
    const b = nearNodes[i];
    const h = a === undefined || b === undefined ? undefined : handles(a, b);
    if (a === undefined || b === undefined || h === undefined) continue;
    const out = b.peak ? -1 : 1;
    for (const t of b.peak ? [0.5, 0.78] : [0.22, 0.5]) {
      const [px, py] = bezier([a.x, a.y], h[0], h[1], [b.x, b.y], t + between(rng, -0.06, 0.06));
      cun += stroke(BOX, [px, py + 1.4 * s], [px + out * 1.6 * s, py + between(rng, 5, 7) * s], 0.8 * s, out * 0.4 * s);
    }
    const summit = b.peak ? b : a;
    const side = b.peak ? -1 : 1;
    dots += BOX.ellipse(summit.x + side * between(rng, 1.2, 3.2) * s, summit.y + 0.5 * s, 0.6 * s, 0.4 * s);
  }
  const soft = 2.6 * s;
  return [
    shape(rangeBody(farNodes), "slate-400", 0.32),
    shape(rangeCrest(farNodes, depth(5, 2, 0)), "slate-400", 0.3),
    shape(rangeBody(midNodes), "slate-700", 0.3),
    shape(rangeCrest(midNodes, depth(4.5, 1.5, 0)), "slate-700", 0.25),
    shape(streak(x(between(rng, -4, 6)), x(between(rng, 92, 104)), farFoot + 0.4 * s, soft) + streak(x(between(rng, 18, 30)), x(104), midFoot, soft * 0.9), "slate-100", 0.4),
    shape(
      streak(x(between(rng, 6, 16)), x(between(rng, 34, 42)), farFoot, 1.4 * s) +
        streak(x(between(rng, 56, 64)), x(between(rng, 84, 96)), farFoot - 0.6 * s, 1.3 * s) +
        streak(x(between(rng, 34, 44)), x(between(rng, 62, 74)), midFoot, 1.2 * s),
      "slate-100",
      0.7,
    ),
    shape(rangeBody(nearNodes), "slate-700", 0.86, MAIN),
    shape(rangeCrest(nearNodes, depth(4.5, 1.5, 0.6)), "slate-900", 0.22),
    shape(rangeCrest(nearNodes, depth(1.5, 0.6, 0.35)), "slate-900", 0.85),
    shape(cun, "slate-900", 0.38),
    shape(dots, "slate-900", 0.75),
  ];
}

/** A needle of rock standing on the anchor: a thin pinnacle (main) leaning a little, darker strokes down it, a jade wash on a ledge and on its flat summit, mist round its foot; about 8 wide and 45 tall at scale 1. */
function needlePeak(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.05, 0.05);
  const j = (): number => between(rng, -0.3, 0.3);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const ledge = between(rng, -30, -25);
  const left: Point[] = [P(-5, 0), P(-3.4 + j(), -7), P(-4.1 + j(), -13), P(-2.8 + j(), -20), P(-3.2 + j(), -27), P(-2.2, -34), P(-1.9, -40), P(-1.3, -45)];
  const right: Point[] = [P(1.2, -45.3), P(1.7, -40), P(2.5 + j(), -34), P(2.1, ledge - 1.4), P(3.9, ledge), P(2.9, ledge + 1.2), P(3.4 + j(), -17), P(4.3 + j(), -8), P(5.4, 0)];
  const run = (points: readonly Point[]): string => {
    let d = "";
    for (let i = 1; i < points.length - 1; i += 1) {
      const [x, y] = points[i] ?? [0, 0];
      const [nx, ny] = points[i + 1] ?? [x, y];
      d += `Q${f.p(x, y)} ${f.p((x + nx) / 2, (y + ny) / 2)}`;
    }
    const [lx, ly] = points[points.length - 1] ?? [0, 0];
    return `${d}L${f.p(lx, ly)}`;
  };
  const body = `M${f.p(...(left[0] ?? [0, 0]))}${run(left)}L${f.p(...(right[0] ?? [0, 0]))}${run(right)}Z`;
  const inner = right.map(([x, y]): Point => [x - 1.2 - (y / -45) * -0.6 - 0.8 * (1 + y / 45), y]);
  const shade = `M${f.p(...(right[0] ?? [0, 0]))}${run(right)}L${f.p(...(inner[inner.length - 1] ?? [0, 0]))}${run([...inner].reverse())}Z`;
  let strokes = "";
  for (const [x, y0, y1] of [
    [-1.6, -12, -4],
    [0.4, -24, -15],
    [-0.6, -38, -30],
    [1.2, -9, -2],
  ] as const) {
    strokes += stroke(f, P(x + j(), y0 + j()), P(x + j() * 2, y1), 0.7, 0.3);
  }
  const out = [
    shape(body, "slate-400", 1, MAIN_INKED),
    shape(shade, "slate-700", 0.45),
    shape(strokes, "slate-900", 0.45),
    shape(f.band([P(-3.6, -6), P(-3.4, -14), P(-2.4, -26), P(-1.6, -40)], 0.5), "slate-100", 0.35),
    shape(f.ellipse(...P(2.2, ledge - 0.5), 1.9, 0.8) + f.ellipse(...P(-0.05, -45.6), 1.3, 0.45), "jade-400", 0.8),
    shape(f.ellipse(0, -1, 10, 2.4), "slate-100", 0.4),
    shape(f.ellipse(0, -0.4, 7, 1.3), "slate-100", 0.7),
  ];
  if (rng() < 0.5) out.push(shape(stroke(f, P(-7, -21), P(8, -19.5), 1.6, 0.4), "slate-100", 0.55));
  return out;
}

/** Three to five stepped terraces from the placement's y down across the box, nearer ones deeper: each tread (main, green-700) a gently curving band, an inked lip at its front and a paler riser below it, a jade wash at its back. */
function terraces(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const rows = whole(rng, 3, 5);
  const lift = between(rng, 1.2, 2.4) * at.s;
  const tilt = between(rng, -1.5, 1.5) * at.s * at.flip;
  const contour = (k: number): Contour => {
    const y = top + depth * (k / rows) ** 1.25;
    const bow = lift * (1 + 0.25 * k);
    return { left: y + bow + tilt, right: y + bow - tilt, lift: bow };
  };
  let treads = "";
  let backs = "";
  let lips = "";
  let joints = "";
  let sheen = "";
  for (let k = 0; k < rows; k += 1) {
    const upper = contour(k);
    const lower = contour(k + 1);
    const tall = lower.left - upper.left;
    const lip = lowered(upper, tall * 0.6);
    const ink = (0.45 + 0.12 * k) * at.s;
    treads += strip(upper, lip);
    backs += strip(upper, lowered(upper, tall * 0.18));
    lips += strip(lowered(lip, -ink), lip);
    const y = lip.left + (lower.left - lip.left) * 0.5;
    for (let n = 0; n < 3; n += 1) {
      const jx = between(rng, 8, 92);
      const yAt = y - (lip.lift * 2 * (1 - ((jx - 50) / 52) ** 2)) / 2;
      joints += BOX.band([[jx, yAt - tall * 0.1], [jx + 0.3, yAt + tall * 0.1]], 0.3 * at.s);
    }
    if (rng() < 0.6) {
      const sx = between(rng, 10, 70);
      const sy = upper.left + tall * 0.35 - upper.lift * 1.2;
      sheen += stroke(BOX, [sx, sy], [sx + between(rng, 12, 22), sy], 0.5 * at.s);
    }
  }
  const out = [
    shape(strip(contour(0), { left: FAR_EDGE, right: FAR_EDGE, lift: 0 }), "stone-500", 0.75),
    shape(treads, "green-700", 0.92, MAIN),
    shape(backs, "jade-400", 0.3),
    shape(lips, "slate-900", 0.8),
    shape(joints, "slate-700", 0.35),
  ];
  if (sheen !== "") out.push(shape(sheen, "slate-100", 0.35));
  return out;
}

/** A zen garden's raked gravel bed (main, slate-100) from the placement's y down across the box behind a dark curb: four to six raked lines and two or three rings of ripples round a point at the placement's x, 10 below its y at scale 1. */
function rakedGravel(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const s = at.s;
  const cx = at.x + between(rng, -2, 2) * s;
  const cy = top + Math.min(depth * 0.45, 10 * s);
  const rings = whole(rng, 2, 3);
  const step = 2.6 * s;
  const outer = 4 * s + rings * step;
  const squash = 0.36;
  const width = 0.5 * s;
  let ripples = "";
  for (let k = 1; k <= rings; k += 1) {
    const r = 4 * s + k * step;
    ripples += BOX.ellipse(cx, cy, r, r * squash) + BOX.ellipse(cx, cy, r - width, r * squash - width);
  }
  const line = (x0: number, x1: number, y: number, w: number): string => {
    const xm = (x0 + x1) / 2;
    return `M${pt(x0, y)}Q${pt(xm, y - 0.6 * s)} ${pt(x1, y)}L${pt(x1, y + w)}Q${pt(xm, y - 0.6 * s + w)} ${pt(x0, y + w)}Z`;
  };
  const lines = whole(rng, 4, 6);
  let rakes = "";
  for (let k = 0; k < lines; k += 1) {
    const t = (k + 0.7) / (lines + 0.2);
    const y = top + 1.8 * s + (depth - 1.8 * s) * t ** 1.35;
    const w = (0.35 + 0.35 * t) * s;
    const gap = outer + 1.4 * s;
    const dy = (y - cy) / (gap * squash + 1.2 * s);
    if (Math.abs(dy) < 1) {
      const chord = gap * Math.sqrt(1 - dy * dy);
      rakes += line(EDGE, cx - chord, y, w) + line(cx + chord, FAR_EDGE, y, w);
    } else rakes += line(EDGE, FAR_EDGE, y, w);
  }
  let grit = "";
  for (let k = 0; k < 7; k += 1) grit += BOX.circle(between(rng, 2, 98), top + depth * between(rng, 0.15, 0.95), between(rng, 0.2, 0.4) * s);
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "slate-100", 1, MAIN),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth * 0.3), "slate-400", 0.16),
    shape(rakes + ripples, "slate-400", 0.9),
    shape(BOX.ellipse(cx, cy, 3.4 * s, 3.4 * s * squash), "slate-400", 0.22),
    shape(grit, "slate-700", 0.35),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 1.1 * s), "slate-800", 0.9),
  ];
}

/** A cracked dry plateau (main, stone-500) from the placement's y down across the box: a paler far edge and a network of thin dark cracks, its cells growing toward the viewer. */
function crackedEarth(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const s = at.s;
  const rows = whole(rng, 3, 4);
  const yOf = (k: number): number => top + depth * ((k + 0.45) / (rows + 0.2)) ** 1.45;
  let across = "";
  const seams: number[][] = [];
  for (let k = 0; k < rows; k += 1) {
    const y = yOf(k);
    const t = (y - top) / depth;
    const points: Point[] = [];
    const ys: number[] = [];
    const stepX = 10 + 12 * t;
    for (let x = EDGE; x < FAR_EDGE + stepX; x += stepX) {
      const py = y + between(rng, -1, 1) * (0.5 + 1.5 * t) * s;
      points.push([Math.min(x + between(rng, -2, 2), FAR_EDGE + 2), py]);
      ys.push(py);
    }
    seams.push(ys);
    across += BOX.band(points, (0.3 + 0.45 * t) * s);
  }
  let down = "";
  for (let k = -1; k < rows; k += 1) {
    const y0 = k < 0 ? top + 0.8 * s : yOf(k);
    const y1 = k + 1 < rows ? yOf(k + 1) : FAR_EDGE + 1;
    const t = ((y0 + y1) / 2 - top) / depth;
    const cell = 12 + 20 * t;
    for (let x = EDGE + between(rng, 0.2, 0.8) * cell; x < FAR_EDGE; x += cell * between(rng, 0.8, 1.2)) {
      const bend = between(rng, -2, 2) * s;
      const end = rng() < 0.2 ? y0 + (y1 - y0) * 0.6 : y1;
      down += stroke(BOX, [x, y0], [x + bend, end], (0.45 + 0.6 * t) * s, between(rng, -0.6, 0.6) * s);
    }
  }
  let pebbles = "";
  for (let k = 0; k < 4; k += 1) {
    const t = between(rng, 0.3, 0.9);
    pebbles += BOX.ellipse(between(rng, 4, 96), top + depth * t, (0.4 + 0.8 * t) * s, (0.25 + 0.45 * t) * s);
  }
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "stone-500", 1, MAIN),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth * 0.35), "slate-100", 0.18),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth * 0.15), "slate-100", 0.25),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 0.6 * s), "slate-700", 0.55),
    shape(across, "slate-900", 0.55),
    shape(down, "slate-900", 0.5),
    shape(pebbles, "slate-700", 0.5),
  ];
}

/** An interior floor of tatami (main, a straw lime-200 greyed by a stone wash) from the placement's y down across the box: rows of mats, their dark borders converging toward the placement's x, their seams staggered, alternate mats catching the light, a dark skirting at the far edge. */
function tatami(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const s = at.s;
  const vx = at.x;
  const vy = top - depth * 1.6;
  const span = FAR_EDGE - vy;
  const far = (top - vy) / span;
  const xAt = (xb: number, y: number): number => vx + ((xb - vx) * (y - vy)) / span;
  const w = Math.max(14, 22 * s);
  const offset = between(rng, 0, w);
  const first = Math.floor((EDGE - vx) / far / w) - 1;
  const last = Math.ceil((FAR_EDGE - vx) / far / w) + 1;
  const half = 0.8 * s;
  const quad = (x0: number, x1: number): string =>
    `M${pt(xAt(x0, top), top)}L${pt(xAt(x1, top), top)}L${pt(x1, FAR_EDGE)}L${pt(x0, FAR_EDGE)}Z`;
  const focal = 6 * w;
  const zTop = focal / far;
  const phase = rng();
  let borders = "";
  let lit = "";
  let seams = "";
  for (let k = first; k <= last; k += 1) {
    const xb = vx + offset + k * w;
    borders += quad(xb - half, xb + half);
    if (k === last) continue;
    const odd = ((k % 2) + 2) % 2;
    if (odd === 1) lit += quad(xb + half, xb + w - half);
    for (let z = focal + (odd + phase) * w; z < zTop; z += 2 * w) {
      const u = focal / z;
      const y = vy + span * u;
      const thick = 0.45 * s * u;
      seams += rectPath(xAt(xb + half, y), y - thick / 2, xAt(xb + w - half, y) - xAt(xb + half, y), thick);
    }
  }
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "lime-200", 1, MAIN),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "stone-500", 0.38),
    shape(lit, "slate-100", 0.2),
    shape(seams, "slate-700", 0.45),
    shape(borders, "slate-800", 0.95),
    shape(rectPath(EDGE, top - 0.2 * s, FAR_EDGE - EDGE, 1.1 * s), "slate-900", 0.85),
  ];
}

/** A still lake (main, translucent blue-200) from the placement's y down across the box: a far shoreline and a glassy glare under it, two to four thin pale ripples and a soft reflection strip under the placement's x. */
function stillWater(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const s = at.s;
  const x = at.x + between(rng, -1, 1) * s;
  const ripples = whole(rng, 2, 4);
  let lines = "";
  for (let k = 0; k < ripples; k += 1) {
    const t = (k + 0.6) / (ripples + 0.3);
    const y = top + depth * t ** 1.3;
    const length = (16 + 26 * t) * (0.8 + 0.4 * rng());
    const x0 = k % 2 === 0 ? between(rng, 4, 40) : between(rng, 46, 90) - length * 0.5;
    lines += stroke(BOX, [x0, y], [x0 + length, y + between(rng, -0.3, 0.3)], (0.3 + 0.4 * t) * s);
  }
  let shimmer = "";
  for (let k = 0; k < 5; k += 1) {
    const t = (k + 0.5) / 5;
    const y = top + depth * t ** 1.2;
    const rx = (1.2 + 2.4 * t) * s;
    shimmer += BOX.ellipse(x + between(rng, -0.5, 0.5) * rx, y, rx, Math.min((0.25 + 0.4 * t) * s, depth * 0.04));
  }
  const fan = (near: number, far: number): string => `M${pt(x - far, top)}L${pt(x + far, top)}L${pt(x + near, FAR_EDGE)}L${pt(x - near, FAR_EDGE)}Z`;
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "blue-200", 0.55, MAIN),
    shape(rectPath(EDGE, top + depth * 0.45, FAR_EDGE - EDGE, depth * 0.55 + 1), "slate-400", 0.16),
    shape(rectPath(EDGE, top + 0.5 * s, FAR_EDGE - EDGE, 1.8 * s), "white", 0.3),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 0.5 * s), "slate-700", 0.4),
    shape(fan(6 * s, 2 * s), "white", 0.12),
    shape(shimmer, "white", 0.3),
    shape(lines, "white", 0.55),
  ];
}

// Architecture

/** A pagoda of three to five tiers standing on the anchor: a stone plinth, red-600 walls between posts under each tier's upswept dark roof (main, slate-800), a ringed spire and a finial; about 16 wide and 34 tall at scale 1. */
function pagoda(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tiers = whole(rng, 3, 5);
  const plinth = 2.2;
  const crown = -26.5;
  const weights = Array.from({ length: tiers }, (_, k) => 1 - 0.1 * k);
  const total = weights.reduce((sum, w) => sum + w, 0);
  let base = -plinth;
  let walls = "";
  let openings = "";
  let roofs = "";
  let sheen = "";
  let bells = "";
  weights.forEach((weight, k) => {
    const h = ((crown + plinth) * weight) / total;
    const t = tiers === 1 ? 0 : k / (tiers - 1);
    const R = 8.2 - 3.4 * t;
    const w = R * 0.6;
    const rt = Math.min(2.8, -h * 0.42);
    const roofTop = base + h;
    const eave = roofTop + rt;
    const tip = eave - rt * 0.78;
    walls += rect(f, -w, eave, w, base);
    if (base - eave > 1.6) openings += rect(f, -w + 0.75, eave + 0.8, -0.4, base - 0.4) + rect(f, 0.4, eave + 0.8, w - 0.75, base - 0.4);
    roofs +=
      `M${f.p(-R - 0.6, tip)}Q${f.p(-R * 0.55, eave + 0.55)} ${f.p(0, eave + 0.55)}Q${f.p(R * 0.55, eave + 0.55)} ${f.p(R + 0.6, tip)}` +
      `Q${f.p(R * 0.72, eave - rt * 0.5)} ${f.p(w * 0.85, roofTop)}L${f.p(-w * 0.85, roofTop)}Q${f.p(-R * 0.72, eave - rt * 0.5)} ${f.p(-R - 0.6, tip)}Z`;
    sheen += f.band([[-w * 0.8, roofTop + 0.45], [w * 0.8, roofTop + 0.45]], 0.5);
    bells += f.circle(-R - 0.45, tip + 0.75, 0.32) + f.circle(R + 0.45, tip + 0.75, 0.32);
    base = roofTop + 0.4;
  });
  let rings = "";
  for (let k = 0; k < 5; k += 1) rings += f.ellipse(0, crown - 1.6 - k * 1.05, 1.3 - 0.1 * k, 0.3);
  const out = [
    shape(f.poly([[-7.4, 0], [7.4, 0], [7.4, -1], [6.6, -1], [6.6, -plinth], [-6.6, -plinth], [-6.6, -1], [-7.4, -1]]), "stone-500", 1, INKED),
    shape(walls, "red-600", 1),
    shape(openings, "slate-900", 0.7),
    shape(f.band([[0, crown + 0.6], [0, -32.4]], 0.6), "slate-800", 1),
    shape(roofs, "slate-800", 1, MAIN_INKED),
    shape(sheen, "slate-400", 0.35),
    shape(rings + bells + f.circle(0, -33.3, 0.8), "slate-900", 1),
  ];
  if (rng() < 0.5) out.push(shape(f.ellipse(0, -0.4, 11, 1.7), "slate-100", 0.5));
  return out;
}

/** An open pavilion standing on the anchor: a raised stone platform, four red-600 posts with a low railing in the side bays and a beam across their tops, under an upturned dark tiled roof (main, slate-800); about 28 wide and 21 tall at scale 1. */
function pavilion(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const floor = -3.2;
  const head = -13.2;
  const posts = [-10.5, -3.8, 3.8, 10.5];
  const pw = 0.6;
  const rail = -6.2;
  let railing = "";
  for (const [a, b] of [
    [-10.5, -3.8],
    [3.8, 10.5],
  ] as const) {
    railing += rect(f, a + pw, rail, b - pw, rail + 0.6);
    for (const x of [a + (b - a) / 3, a + (2 * (b - a)) / 3]) railing += rect(f, x - 0.25, rail + 0.6, x + 0.25, floor);
  }
  const lift = between(rng, 0, 0.8);
  const roof =
    `M${f.p(-14.4, -15.8 - lift)}Q${f.p(-9, -12.3)} ${f.p(0, -12.5)}Q${f.p(9, -12.3)} ${f.p(14.4, -15.8 - lift)}` +
    `Q${f.p(10, -15.2)} ${f.p(7.6, -18.6)}L${f.p(7, -21)}L${f.p(6.1, -19.7)}L${f.p(-6.1, -19.7)}L${f.p(-7, -21)}L${f.p(-7.6, -18.6)}Q${f.p(-10, -15.2)} ${f.p(-14.4, -15.8 - lift)}Z`;
  let tiles = "";
  for (const x of [-9, -6, -3, 0, 3, 6, 9]) tiles += stroke(f, [x * 0.72, -19.2], [x, -13.4], 0.4);
  return [
    shape(rect(f, -12.6, floor, 12.6, 0) + f.poly([[-3.4, 0], [3.4, 0], [3.4, 1.2], [-3.4, 1.2]]), "stone-500", 1, INKED),
    shape(f.band([[-12.6, floor + 0.35], [12.6, floor + 0.35]], 0.6), "slate-100", 0.35),
    shape(posts.map((x) => rect(f, x - pw, head, x + pw, floor)).join(""), "red-600", 1, INKED),
    shape(railing, "red-600", 0.9),
    shape(rect(f, -11.6, head - 0.4, 11.6, head + 0.8), "red-900", 1),
    shape(roof, "slate-800", 1, MAIN_INKED),
    shape(tiles, "slate-400", 0.3),
    shape(f.band([[-6.1, -19.2], [6.1, -19.2]], 0.5), "slate-900", 0.5),
  ];
}

/** A moon bridge standing on the anchor: a steep stone hump (main, slate-400) round a semicircular opening, a ring of voussoirs, a railing of posts along its deck; about 30 wide and 15 tall at scale 1 (its railing included). */
function moonBridge(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 15;
  const crest = between(rng, -12.4, -11.6);
  const r = 8.2;
  const deck: [Point, Point, Point, Point][] = [
    [[-half, 0], [-11, -1.4], [-6.6, crest], [0, crest]],
    [[0, crest], [6.6, crest], [11, -1.4], [half, 0]],
  ];
  const curve = (dy: number, back: boolean): string =>
    (back ? [...deck].reverse() : deck)
      .map(([a, c1, c2, b]) => (back ? `C${f.p(c2[0], c2[1] + dy)} ${f.p(c1[0], c1[1] + dy)} ${f.p(a[0], a[1] + dy)}` : `C${f.p(c1[0], c1[1] + dy)} ${f.p(c2[0], c2[1] + dy)} ${f.p(b[0], b[1] + dy)}`))
      .join("");
  const arc = (radius: number, sweep: 0 | 1, x: number): string => `A${fmt(f.r(radius))} ${fmt(f.r(radius))} 0 0 ${f.sweep(sweep)} ${f.p(x, 0)}`;
  const body = `M${f.p(-half, 0)}${curve(0, false)}L${f.p(r, 0)}${arc(r, 0, -r)}Z`;
  const ring = `M${f.p(r + 1.4, 0)}${arc(r + 1.4, 0, -r - 1.4)}L${f.p(-r, 0)}${arc(r, 1, r)}Z`;
  const soffit = `M${f.p(r, 0)}${arc(r, 0, -r)}L${f.p(-r + 1, 0)}${arc(r - 1, 1, r - 1)}Z`;
  let posts = "";
  for (const [a, c1, c2, b] of deck) {
    for (const t of [0.15, 0.4, 0.65, 0.9]) {
      const [x, y] = bezier(a, c1, c2, b, t);
      posts += rect(f, x - 0.3, y - 2.1, x + 0.3, y + 0.1);
    }
  }
  return [
    shape(body, "slate-400", 1, MAIN_INKED),
    shape(ring, "slate-700", 0.35),
    shape(soffit, "slate-900", 0.35),
    shape(`M${f.p(-half, 0)}${curve(0, false)}L${f.p(half, 0.7)}${curve(0.7, true)}Z`, "slate-100", 0.4),
    shape(f.poly([[-0.8, crest + 0.4], [0.8, crest + 0.4], [0.6, -r - 1.4], [-0.6, -r - 1.4]]), "slate-700", 0.4),
    shape(posts, "slate-700", 0.9),
    shape(`M${f.p(-half, -2.1)}${curve(-2.1, false)}L${f.p(half, -2.7)}${curve(-2.7, true)}Z`, "slate-700", 0.9),
  ];
}

/** A shoji panel standing on the anchor: pale translucent paper (main, slate-100) behind a wooden lattice, in a frame with a kickplate at its foot and a small recessed pull; about 14 wide and 24 tall at scale 1. */
function paperScreen(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cols = whole(rng, 3, 4);
  const rows = whole(rng, 5, 6);
  const [left, right, top, foot] = [-6, 6, -23, -4];
  const bar = 0.5;
  const pw = (right - left - bar * (cols - 1)) / cols;
  const ph = (foot - top - bar * (rows - 1)) / rows;
  let lattice = rect(f, left, top, right, foot);
  for (let c = 0; c < cols; c += 1) {
    for (let r = 0; r < rows; r += 1) {
      const x = left + c * (pw + bar);
      const y = top + r * (ph + bar);
      lattice += rect(f, x, y, x + pw, y + ph);
    }
  }
  const out = [
    shape(rect(f, left, top, right, foot), "slate-100", 0.85, MAIN),
    shape(lattice, "wood-700", 1),
    shape(rect(f, -7, -24, 7, 0) + rect(f, left, top, right, foot), "wood-700", 1, INKED),
    shape(rect(f, -5.4, -3.2, 5.4, -0.9), "wood-900", 0.35),
    shape(f.ellipse(6.5, -12, 0.3, 0.7), "slate-900", 0.85),
  ];
  if (rng() < 0.5) out.splice(1, 0, shape(rect(f, left + 1.5, top + 2, right - 1.5, foot - 4), "white", 0.3));
  return out;
}

/** A moon window centred on the anchor: a whitewashed wall section (main, white) about 30 by 30 at scale 1 with a round opening 21.6 across (a hole), a stone frame ring, a dark tiled coping and a grey plinth. */
function roundWindow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 10.8;
  let stains = "";
  for (let k = 0; k < 3; k += 1) {
    const x = between(rng, -14, 14);
    if (Math.abs(x) < r + 1) continue;
    stains += stroke(f, [x, -12.6], [x + between(rng, -0.4, 0.4), between(rng, -4, 6)], between(rng, 0.8, 1.6));
  }
  let tiles = "";
  for (let x = -14; x <= 14; x += 2.8) tiles += f.band([[x, -14.6], [x * 1.04, -12.7]], 0.35);
  const out = [
    shape(rect(f, -15, -13, 15, 15) + f.circle(0, 0, r), "white", 1, MAIN_INKED),
    shape(rect(f, -15, 12.8, 15, 15), "slate-400", 0.5),
    shape(f.circle(0, 0, r) + f.circle(0, 0, r - 1.3), "slate-400", 1, INKED),
    shape(f.circle(0, 0, r - 1.3) + f.circle(0.7, 0.7, r - 2.3), "slate-700", 0.3),
    shape(f.poly([[-16.4, -12.4], [16.4, -12.4], [15.4, -14.2], [14.6, -15], [-14.6, -15], [-15.4, -14.2]]), "slate-800", 1, INKED),
    shape(tiles, "slate-400", 0.35),
  ];
  if (stains !== "") out.splice(1, 0, shape(stains, "slate-400", 0.18));
  return out;
}

/** A vermilion torii standing on the anchor: two posts leaning in (main, red-600) on black sleeves, a lower and an upper lintel with a strut and a dark plaque between them (main), a dark curved cap with upswept ends; about 28 wide and 23 tall at scale 1. */
function redGate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, 0, 0.6);
  const posts = f.poly([[-9.6, 0], [-7.8, 0], [-7.4, -19.2], [-8.8, -19.2]]) + f.poly([[7.8, 0], [9.6, 0], [8.8, -19.2], [7.4, -19.2]]);
  const lintels = rect(f, -11.4, -15.6, 11.4, -14.3) + rect(f, -12.4, -20.4, 12.4, -19.2) + rect(f, -0.7, -19.2, 0.7, -15.6);
  const cap =
    `M${f.p(-14 - flare, -22 - flare)}Q${f.p(-7, -20.1)} ${f.p(0, -20.4)}Q${f.p(7, -20.1)} ${f.p(14 + flare, -22 - flare)}` +
    `L${f.p(14.3 + flare, -23.1 - flare)}Q${f.p(7, -21.5)} ${f.p(0, -21.8)}Q${f.p(-7, -21.5)} ${f.p(-14.3 - flare, -23.1 - flare)}Z`;
  return [
    shape(posts, "red-600", 1, MAIN_INKED),
    shape(f.poly([[-8.3, 0], [-7.8, 0], [-7.4, -19.2], [-7.9, -19.2]]) + f.poly([[9.1, 0], [9.6, 0], [8.8, -19.2], [8.3, -19.2]]), "red-900", 0.35),
    shape(rect(f, -9.9, -1.6, -7.5, 0) + rect(f, 7.5, -1.6, 9.9, 0), "slate-900", 1),
    shape(lintels, "red-600", 1, MAIN_INKED),
    shape(rect(f, -11.4, -14.7, 11.4, -14.3) + rect(f, -12.4, -19.6, 12.4, -19.2), "red-900", 0.35),
    shape(rect(f, -1.4, -18.8, 1.4, -16), "slate-800", 1),
    shape(cap, "slate-800", 1, INKED),
    shape(f.band([[-12, -21.6], [0, -21.35], [12, -21.6]], 0.4), "slate-400", 0.35),
  ];
}

/** A round stone well standing on the anchor: its stone drum with courses, a pale rim, a dark shaft and a glowing water surface (main, jade-400, with the faint glow above it), a two-post frame with a crossbeam and a small roof, a rope and a bucket; about 14 wide and 11 tall at scale 1. */
function stoneWell(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 6;
  const rim = -5;
  const ry = 1.5;
  const ellipseArc = (rx: number, rr: number, x: number, y: number): string => `A${fmt(f.r(rx))} ${fmt(f.r(rr))} 0 0 ${f.sweep(0)} ${f.p(x, y)}`;
  const drum = `M${f.p(-half, -ry)}${ellipseArc(half, ry, half, -ry)}L${f.p(half, rim)}${ellipseArc(half, ry, -half, rim)}Z`;
  let courses = "";
  for (const y of [-2.9, -4.2]) courses += `M${f.p(-half, y)}Q${f.p(0, y + ry * 2)} ${f.p(half, y)}L${f.p(half, y + 0.3)}Q${f.p(0, y + ry * 2 + 0.3)} ${f.p(-half, y + 0.3)}Z`;
  for (const [x, y] of [
    [-3.4, -2.2],
    [1.6, -2],
    [-1, -3.4],
    [3.8, -3.5],
  ] as const) {
    courses += f.band([[x, y - 0.5], [x, y + 0.5]], 0.3);
  }
  const bucketX = between(rng, 1.2, 2.2);
  const flicker = between(rng, -0.2, 0.2);
  return [
    shape(drum, "slate-400", 1, INKED),
    shape(courses, "slate-700", 0.45),
    shape(f.ellipse(0, rim, half, ry) + f.ellipse(0, rim, half - 1.1, ry - 0.4), "slate-400", 1, INKED),
    shape(f.ellipse(0, rim, half - 1.1, ry - 0.4), "slate-900", 0.85),
    shape(f.ellipse(0, rim + 0.15, half - 1.9, ry - 0.75), "jade-400", 0.85, MAIN),
    shape(f.ellipse(0, rim - 2.2 + flicker, 4.8, 2.8), "jade-400", 0.2, MAIN),
    shape(f.ellipse(0, rim - 3.2 + flicker, 6.6, 4.4), "jade-400", 0.1, MAIN),
    shape(rect(f, -5.4, -8.6, -4.6, rim + 0.8) + rect(f, 4.6, -8.6, 5.4, rim + 0.8), "wood-800", 1),
    shape(rect(f, -4.6, -7.8, 4.6, -7.1) + rect(f, bucketX - 0.1, -7.1, bucketX + 0.1, -6.3), "wood-700", 1),
    shape(f.poly([[-7, -8.2], [0, -11.2], [7, -8.2], [6.2, -7.8], [0, -10.1], [-6.2, -7.8]]), "slate-800", 1, INKED),
    shape(f.poly([[bucketX - 0.8, -6.3], [bucketX + 0.8, -6.3], [bucketX + 0.6, -5], [bucketX - 0.6, -5]]), "wood-600", 1),
  ];
}

/** An ink-world village house standing on the anchor: whitewashed walls with weathering streaks and a grey plinth, a dark curved tile roof with upturned ends (main, slate-800), a dark door and a lattice window; about 22 wide and 16 tall at scale 1. */
function tiledHouse(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const doorX = between(rng, -4, -2);
  const winX = between(rng, 3.4, 4.6);
  const [wl, wt, wr, wb] = [winX - 1.9, -6.6, winX + 1.9, -3.6];
  let lattice = rect(f, wl, wt, wr, wb);
  for (const [x0, x1] of [
    [wl + 0.35, winX - 0.15],
    [winX + 0.15, wr - 0.35],
  ] as const) {
    for (const [y0, y1] of [
      [wt + 0.35, (wt + wb) / 2 - 0.15],
      [(wt + wb) / 2 + 0.15, wb - 0.35],
    ] as const) {
      lattice += rect(f, x0, y0, x1, y1);
    }
  }
  let stains = "";
  for (const x of [-7, between(rng, -0.5, 0.8), 7.4]) stains += stroke(f, [x, -8.4], [x + between(rng, -0.3, 0.3), between(rng, -5, -2.5)], between(rng, 0.8, 1.4));
  const lift = between(rng, 0, 0.6);
  const roof =
    `M${f.p(-11.2, -9.9 - lift)}Q${f.p(-6, -8.1)} ${f.p(0, -8.3)}Q${f.p(6, -8.1)} ${f.p(11.2, -9.9 - lift)}` +
    `Q${f.p(8.6, -10.4)} ${f.p(6.8, -14.6)}L${f.p(7.4, -16.2)}L${f.p(6.2, -15.4)}L${f.p(-6.2, -15.4)}L${f.p(-7.4, -16.2)}L${f.p(-6.8, -14.6)}Q${f.p(-8.6, -10.4)} ${f.p(-11.2, -9.9 - lift)}Z`;
  let tiles = "";
  for (const x of [-8, -5.3, -2.7, 0, 2.7, 5.3, 8]) tiles += stroke(f, [x * 0.76, -15], [x, -9], 0.35);
  return [
    shape(rect(f, -8.6, -9.2, 8.6, 0), "white", 1, INKED),
    shape(stains, "slate-400", 0.2),
    shape(rect(f, -8.6, -1.2, 8.6, 0), "slate-400", 0.55),
    shape(rect(f, doorX - 1.7, -5.8, doorX + 1.7, 0), "slate-900", 0.9),
    shape(lattice, "slate-800", 1),
    shape(roof, "slate-800", 1, MAIN_INKED),
    shape(tiles, "slate-400", 0.3),
    shape(f.band([[-6.2, -15], [6.2, -15]], 0.6), "slate-900", 0.5),
  ];
}

/** A stone garden lantern (toro) standing on the anchor: base, post, platform, firebox with its window (a hole), an upturned cap and a jewel (main, slate-400), the window lit warm with a small glow and moss on the cap; about 7 wide and 12.6 tall at scale 1. */
function stoneLantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glow = between(rng, 0.8, 1.1);
  const stone =
    f.poly([[-2.6, 0], [2.6, 0], [2.2, -1.1], [-2.2, -1.1]]) +
    f.poly([[-0.9, -1.1], [0.9, -1.1], [0.75, -5.2], [-0.75, -5.2]]) +
    f.poly([[-1.5, -5.2], [1.5, -5.2], [2.3, -5.9], [2.3, -6.5], [-2.3, -6.5], [-2.3, -5.9]]) +
    rect(f, -1.7, -9, 1.7, -6.5) +
    rect(f, -0.9, -8.6, 0.9, -6.9) +
    `M${f.p(-3.5, -9.7)}Q${f.p(-2.6, -9.1)} ${f.p(-2, -9)}L${f.p(2, -9)}Q${f.p(2.6, -9.1)} ${f.p(3.5, -9.7)}Q${f.p(1.6, -10.2)} ${f.p(0.8, -11)}L${f.p(-0.8, -11)}Q${f.p(-1.6, -10.2)} ${f.p(-3.5, -9.7)}Z` +
    `M${f.p(-0.6, -11)}Q${f.p(-1, -11.8)} ${f.p(0, -12.6)}Q${f.p(1, -11.8)} ${f.p(0.6, -11)}Z`;
  return [
    shape(f.circle(0, -7.75, 4.4 * glow), "orange-400", 0.1),
    shape(stone, "slate-400", 1, MAIN_INKED),
    shape(rect(f, -0.9, -8.6, 0.9, -6.9), "orange-400", 0.9),
    shape(f.circle(0, -7.75, 2.5 * glow), "orange-400", 0.22),
    shape(rect(f, 0.3, -5.1, 0.7, -1.2) + rect(f, 1.05, -8.9, 1.6, -6.6), "slate-700", 0.35),
    shape(f.ellipse(-1.5, -9.75, 1.3, 0.35) + f.ellipse(1.4, -0.4, 1, 0.3), "jade-400", 0.6),
  ];
}

/** R3401000: the ink world's places the Meditative scenes added, by id, each with its kind. */
export const INK_PLACES_KIT = {
  "ink-peaks": { category: "terrain", draw: inkPeaks },
  "needle-peak": { category: "terrain", draw: needlePeak },
  terraces: { category: "terrain", draw: terraces },
  "raked-gravel": { category: "terrain", draw: rakedGravel },
  "cracked-earth": { category: "terrain", draw: crackedEarth },
  tatami: { category: "terrain", draw: tatami },
  "still-water": { category: "terrain", draw: stillWater },
  pagoda: { category: "architecture", draw: pagoda },
  "ink-pavilion": { category: "architecture", draw: pavilion },
  "moon-bridge": { category: "architecture", draw: moonBridge },
  "paper-screen": { category: "architecture", draw: paperScreen },
  "ink-round-window": { category: "architecture", draw: roundWindow },
  "red-gate": { category: "architecture", draw: redGate },
  "stone-well": { category: "architecture", draw: stoneWell },
  "tiled-house": { category: "architecture", draw: tiledHouse },
  "stone-lantern": { category: "architecture", draw: stoneLantern },
} as const satisfies Readonly<Record<string, KitPart>>;
