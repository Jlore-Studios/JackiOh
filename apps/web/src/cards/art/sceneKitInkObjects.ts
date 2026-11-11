// R3401000: the scene kit's ink objects (ART.md, "Scene recipes and kits"), which the Meditative scenes (#3401)
// added: the one red seal every Meditative picture stamps and a red thread, both drawn from their anchor (the seal
// centred on it, the thread running out from it to one side); the lanterns, a round paper one hanging from its
// anchor and a string of them sagging between two points level either side of it; a temple bell hanging from its
// beam at the anchor; the things of a scholar's table and a teahouse standing on their anchor (a calligraphy brush
// on its tip, a teapot, an abacus, a tripod censer and its smoke, a go board, a clepsydra) or lying on it (a drift
// of sealed envelopes); a hand of cards and a folding fan opened from a pivot at their anchor; and a lacquered cane
// centred on its anchor. R3404000: Meditative's Alt art (#3404) added a fisherman's oar lying level, centred on its
// anchor, and a sofa standing on it. sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css name, none in
// Radiant's gold window; a glow is a stack of translucent shapes, never a gradient; shapes are filled even-odd, so
// a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { between, DEGREES, INKED, leant, local, MAIN, MAIN_INKED, shape, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** A ring about (`dx`, `dy`), from `inner` out to `outer`. */
function ring(f: Local, dx: number, dy: number, outer: number, inner: number): string {
  return f.circle(dx, dy, outer) + f.circle(dx, dy, inner);
}

/** A point `r` from (`cx`, `cy`) at `degrees`, clockwise from the right on screen. */
function polar(r: number, degrees: number, cx = 0, cy = 0): Point {
  return [cx + r * Math.cos(degrees * DEGREES), cy + r * Math.sin(degrees * DEGREES)];
}

/** `steps + 1` points along the quadratic curve from `a` by the control `c` to `b`. */
function curve(a: Point, c: Point, b: Point, steps: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const u = 1 - t;
    out.push([u * u * a[0] + 2 * u * t * c[0] + t * t * b[0], u * u * a[1] + 2 * u * t * c[1] + t * t * b[1]]);
  }
  return out;
}

/** A band along the points that tapers from `from` wide at the first to `to` at the last. */
function taper(f: Local, points: readonly Point[], from: number, to: number): string {
  const left: Point[] = [];
  const right: Point[] = [];
  const last = Math.max(1, points.length - 1);
  points.forEach(([x, y], i) => {
    const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
    const [nx, ny] = points[Math.min(last, i + 1)] ?? [x, y];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (from + ((to - from) * i) / last) / 2;
    left.push([x - ((ny - py) / length) * half, y + ((nx - px) / length) * half]);
    right.push([x + ((ny - py) / length) * half, y - ((nx - px) / length) * half]);
  });
  return f.poly([...left, ...right.reverse()]);
}

/** A thread of smoke or steam rising `height` from (`x`, `y`), swaying wider as it climbs and thinning out. */
function wisp(f: Local, rng: Rng, x: number, y: number, height: number, sway: number, width: number): string {
  const phase = between(rng, 0, 2 * Math.PI);
  const drift = between(rng, -0.12, 0.12) * height;
  const points: Point[] = [];
  for (let i = 0; i <= 9; i += 1) {
    const t = i / 9;
    points.push([x + drift * t + sway * (0.25 + t) * Math.sin(phase + t * 2.6 * Math.PI), y - height * t]);
  }
  return taper(f, points, width, width * 0.35);
}

/** A falling drop, its tip at (`x`, `y`) and its round end `r` wide each side below it. */
function drop(f: Local, x: number, y: number, r: number): string {
  return (
    `M${f.p(x, y)}Q${f.p(x + r, y + r * 1.2)} ${f.p(x + r, y + r * 2.2)}Q${f.p(x + r, y + r * 3.2)} ${f.p(x, y + r * 3.2)}` +
    `Q${f.p(x - r, y + r * 3.2)} ${f.p(x - r, y + r * 2.2)}Q${f.p(x - r, y + r * 1.2)} ${f.p(x, y)}Z`
  );
}

/** A four-pointed lozenge about (`x`, `y`), `w` its reach each side and `h` up and down. */
function lozenge(f: Local, x: number, y: number, w: number, h: number): string {
  return f.poly([
    [x - w, y],
    [x, y - h],
    [x + w, y],
    [x, y + h],
  ]);
}

// Seals and threads

/** The one red seal every Meditative picture stamps: a worn square chop centred on the anchor, two or three pale abstract strokes in it (no glyph); about 6 wide and 6 tall at scale 1. */
function seal(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 3;
  const j = (): number => between(rng, -0.2, 0.2);
  const corners: Point[] = [
    [-half, -half],
    [half, -half],
    [half, half],
    [-half, half],
  ];
  const rim: Point[] = [];
  corners.forEach(([ax, ay], k) => {
    const [bx, by] = corners[(k + 1) % corners.length] ?? [ax, ay];
    // A corner a little worn in, then two nicks along the side.
    rim.push([ax * 0.96 + j(), ay * 0.96 + j()]);
    for (const t of [0.36, 0.68]) rim.push([ax + (bx - ax) * t + j(), ay + (by - ay) * t + j()]);
  });
  const strokes =
    f.band([[-1.6 + j(), -1.9], [-1.85, 0], [-1.55 + j(), 1.9]], 0.75) +
    f.band([[-0.3, -1.75 + j()], [1.75, -1.65], [1.55 + j(), -0.1]], 0.6) +
    (rng() < 0.6 ? f.band([[-0.3, 0.75 + j()], [0.75, 1.25], [1.85, 1.85 + j()]], 0.6) : "");
  const specks = f.circle(between(rng, -2.4, -2.1), between(rng, -2.5, -2.2), 0.22) + f.circle(between(rng, 2.1, 2.4), between(rng, 2.2, 2.5), 0.2);
  return [shape(f.poly(rim), "red-600", 0.92), shape(strokes, "slate-100", 0.85), shape(specks, "slate-100", 0.5)];
}

/** A taut thin red thread (main, red-600) from the anchor out to 30 along and 6 up the way it faces, sagging a little, a small knot at each end; about 30 wide and 6 tall at scale 1. */
function redThread(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const end: Point = [30, -6];
  const sag = between(rng, 0.6, 1.4);
  const thread = f.band(curve([0, 0], [15, -3 + 2 * sag], end, 10), 0.45);
  const knots =
    f.circle(0, 0, 0.6) +
    f.band([[-0.5, 0.45], [-1.3, 1.7]], 0.3) +
    f.circle(...end, 0.6) +
    f.band([[end[0] + 0.5, end[1] + 0.45], [end[0] + 1.1, end[1] + 1.8]], 0.3);
  return [shape(thread, "red-600", 1, MAIN), shape(knots, "red-600", 1, MAIN)];
}

// Lanterns and bells

/** A round paper lantern hanging from the anchor on a short cord: a ribbed body (main, red-600) between a black cap and foot, a tassel and a soft glow; about 5 wide and 8.2 tall at scale 1, cord and tassel included. */
function paperLantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cy = 3.9;
  const rx = 2.5;
  const ry = 2.1;
  const flicker = between(rng, -0.2, 0.2);
  const ribs = whole(rng, 3, 4);
  let lines = "";
  for (let k = 1; k <= ribs; k += 1) {
    const y = cy - ry + (2 * ry * k) / (ribs + 1);
    const w = rx * Math.sqrt(1 - ((y - cy) / ry) ** 2) - 0.2;
    lines += f.band([[-w, y], [0, y + 0.3], [w, y]], 0.2);
  }
  const tassel = f.poly([[-0.1, 6.4], [0.1, 6.4], [0.1, 6.9], [0.3, 6.9], [0.5, 8.2], [-0.5, 8.2], [-0.3, 6.9], [-0.1, 6.9]]);
  return [
    shape(f.circle(0, cy, 4.4), "orange-400", 0.12),
    shape(f.circle(0, cy, 3.3), "red-400", 0.16),
    shape(f.ellipse(0, cy, rx, ry), "red-600", 1, MAIN_INKED),
    shape(f.ellipse(flicker, cy + 0.2, 1.4, 1.2), "orange-400", 0.4),
    shape(lines, "red-900", 0.5),
    shape(f.ellipse(-1.2, cy - 0.9, 0.45, 0.7), "slate-100", 0.35),
    shape(f.band([[0, 0], [0, 1.2]], 0.25) + rect(f, -1.1, 1.2, 1.1, 1.95) + rect(f, -0.9, 5.8, 0.9, 6.4), "slate-900", 1),
    shape(tassel, "red-600", 1),
  ];
}

/** A cord sagging between two points 24 either side of the anchor (both at its height, about 5 below it at the middle), hung with five to seven small round lanterns (main, red-600), each in a faint glow; about 48 wide and 10 tall at scale 1. */
function lanternString(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 24;
  const sag = between(rng, 4.5, 5.5);
  const cordY = (x: number): number => sag * (1 - (x / half) ** 2);
  const cord: Point[] = [];
  for (let i = 0; i <= 10; i += 1) {
    const x = -half + (2 * half * i) / 10;
    cord.push([x, cordY(x)]);
  }
  const n = whole(rng, 5, 7);
  let glows = "";
  let bodies = "";
  let ribs = "";
  let hangers = "";
  let tassels = "";
  let lights = "";
  for (let k = 0; k < n; k += 1) {
    const x = -half + (2 * half * (k + 1)) / (n + 1) + between(rng, -0.3, 0.3);
    const top = cordY(x) + 0.15;
    const cy = top + 2.3;
    glows += f.circle(x, cy, 2.3);
    bodies += f.ellipse(x, cy, 1.25, 1.4);
    ribs += f.band([[x - 1, cy], [x, cy + 0.2], [x + 1, cy]], 0.18);
    hangers +=
      f.poly([[x - 0.1, top], [x + 0.1, top], [x + 0.1, top + 0.6], [x + 0.7, top + 0.6], [x + 0.7, top + 1], [x - 0.7, top + 1], [x - 0.7, top + 0.6], [x - 0.1, top + 0.6]]) +
      rect(f, x - 0.5, top + 3.6, x + 0.5, top + 3.95);
    tassels += f.poly([[x - 0.12, top + 3.95], [x + 0.12, top + 3.95], [x + 0.3, top + 5], [x - 0.3, top + 5]]);
    lights += f.ellipse(x - 0.55, cy - 0.5, 0.22, 0.4);
  }
  return [
    shape(glows, "orange-400", 0.16),
    shape(f.band(cord, 0.3), "slate-900", 0.9),
    shape(bodies, "red-600", 1, MAIN_INKED),
    shape(ribs, "red-900", 0.5),
    shape(lights, "slate-100", 0.4),
    shape(hangers, "slate-900", 1),
    shape(tassels, "red-600", 1),
  ];
}

/** A bronze temple bell (main, green-700 verdigris) hanging by its loop under a short wooden beam whose top is the anchor: raised bands, rows of bosses, a striking boss low on it; about 11 wide and 13 tall at scale 1. */
function bell(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = 2.75;
  const shoulder = 4.4;
  const lip = 12.4;
  const width = (y: number): number => 3.35 + (0.35 * (y - shoulder)) / (lip - shoulder) - 0.12;
  const across = (y: number): string => f.band([[-width(y), y], [width(y), y]], 0.45);
  const middle = between(rng, 8.2, 8.6);
  const rows = whole(rng, 2, 3);
  let bosses = "";
  for (let r = 0; r < rows; r += 1) {
    const y = 5.2 + ((middle - 5.8) * (r + 0.5)) / rows;
    for (const x of [-2.35, -1.2, 1.2, 2.35]) bosses += f.circle(x, y, 0.3);
  }
  const side = rng() < 0.5 ? -1 : 1;
  const body = `M${f.p(-3.7, lip)}L${f.p(-3.35, shoulder)}Q${f.p(-3.3, top + 0.05)} ${f.p(0, top)}Q${f.p(3.3, top + 0.05)} ${f.p(3.35, shoulder)}L${f.p(3.7, lip)}Q${f.p(0, lip + 0.5)} ${f.p(-3.7, lip)}Z`;
  return [
    shape(f.poly([[-5.7, -0.5], [-4.8, 0], [4.8, 0], [5.7, -0.5], [5.4, 1.2], [-5.4, 1.2]]), "wood-700", 1, INKED),
    shape(f.band([[-5.2, 0.85], [5.2, 0.85]], 0.4), "slate-900", 0.3),
    shape(ring(f, 0, 2, 0.8, 0.42), "slate-800", 1),
    shape(body, "green-700", 1, MAIN_INKED),
    shape(f.poly([[1.4, 3], [2.5, 3.25], [3.3, 4.4], [3.65, 12.3], [2.2, 12.6]]), "slate-900", 0.25),
    shape(f.band([[-2.55, 3.8], [-2.8, 12.1]], 0.7), "slate-100", 0.22),
    shape(across(4.2) + across(middle) + across(11.2) + f.band([[0, 4.45], [0, middle - 0.25]], 0.45) + ring(f, side * 1.8, (middle + 11.2) / 2, 0.75, 0.35), "green-900", 0.8),
    shape(bosses + f.circle(side * 1.8, (middle + 11.2) / 2, 0.35), "jade-400", 0.6),
  ];
}

// Things on a table

/** A hand of five to seven playing cards fanned from a point at the anchor, their bottom ends gathered there: each card face (main, slate-100) inked, a red or black pip at its corner and a mark on the top card; about 17 wide and 12 tall at scale 1. */
function cardFan(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  const spread = between(rng, 74, 86);
  const [l, t, r, b, c] = [-2.5, -10.5, 2.5, -0.3, 0.8];
  const red = rng() < 0.5 ? 0 : 1;
  const out: KitShape[] = [];
  let reds = "";
  let blacks = "";
  for (let k = 0; k < n; k += 1) {
    const angle = -spread / 2 + (spread * k) / (n - 1);
    const p = (x: number, y: number): string => f.p(...turned([x, y], angle));
    const face =
      `M${p(l + c, t)}L${p(r - c, t)}Q${p(r, t)} ${p(r, t + c)}L${p(r, b - c)}Q${p(r, b)} ${p(r - c, b)}` +
      `L${p(l + c, b)}Q${p(l, b)} ${p(l, b - c)}L${p(l, t + c)}Q${p(l, t)} ${p(l + c, t)}Z`;
    out.push(shape(face, "slate-100", 1, MAIN_INKED));
    const isRed = k % 2 === red;
    const mark = (x: number, y: number, size: number): string =>
      isRed ? f.poly([turned([x - size, y], angle), turned([x, y - size * 1.4], angle), turned([x + size, y], angle), turned([x, y + size * 1.4], angle)]) : f.circle(...turned([x, y], angle), size);
    let marks = mark(-1.5, -9.2, 0.5);
    if (k === n - 1) marks += mark(0, -5.4, 1.3) + mark(1.5, -1.6, 0.5);
    if (isRed) reds += marks;
    else blacks += marks;
  }
  out.push(shape(reds, "red-600", 1), shape(blacks, "slate-900", 1));
  return out;
}

/** A calligraphy brush standing on its ink-black tip at the anchor, leant a little the way it faces: a bamboo handle (main, wood-600) with two nodes, a collar, a cap and a hanging loop at its top, a little ink pooled at its tip; about 3 wide and 19.5 tall at scale 1. */
function brush(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0.08, 0.2);
  const P = (x: number, y: number): Point => leant([x, y], lean);
  const p = (x: number, y: number): string => f.p(...P(x, y));
  const poly = (points: readonly Point[]): string => f.poly(points.map(([x, y]) => P(x, y)));
  const box = (left: number, top: number, right: number, bottom: number): string => poly([[left, top], [right, top], [right, bottom], [left, bottom]]);
  const nodes = [between(rng, -10, -9.2), between(rng, -14, -13.2)];
  return [
    shape(f.ellipse(0.2, 0.15, 1.6, 0.45), "slate-900", 0.3),
    shape(box(-0.85, -17.2, 0.85, -5.4), "wood-600", 1, MAIN_INKED),
    shape(nodes.map((y) => box(-0.85, y - 0.2, 0.85, y + 0.2)).join(""), "wood-800", 0.8),
    shape(box(-0.55, -16.6, -0.25, -6), "slate-100", 0.3),
    shape(box(-1.15, -5.4, 1.15, -4.4) + box(-1, -17.8, 1, -17.2), "slate-800", 1),
    shape(`M${p(0, 0)}C${p(-0.3, -1.2)} ${p(-1.25, -2.4)} ${p(-1.05, -4.4)}L${p(1.05, -4.4)}C${p(1.25, -2.4)} ${p(0.3, -1.2)} ${p(0, 0)}Z`, "slate-900", 1),
    shape(`M${p(0.2, -0.9)}Q${p(0.85, -2.6)} ${p(0.65, -4.2)}L${p(0.3, -4.2)}Q${p(0.45, -2.5)} ${p(0.2, -0.9)}Z`, "slate-700", 0.6),
    shape(ring(f, ...P(0, -18.6), 0.75, 0.42), "slate-700", 1),
  ];
}

/** A round teapot standing on the anchor: its body and lid (main, slate-700 iron), a spout the way it faces, a loop handle behind, a knob, one or two curling wisps of steam; about 10 wide and 7 tall at scale 1, its steam 5 more. */
function teapot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const body =
    `M${f.p(-1.8, 0)}L${f.p(1.8, 0)}L${f.p(2.1, -0.5)}C${f.p(3.6, -0.9)} ${f.p(3.7, -4.8)} ${f.p(1.5, -5.2)}` +
    `L${f.p(-1.5, -5.2)}C${f.p(-3.7, -4.8)} ${f.p(-3.6, -0.9)} ${f.p(-2.1, -0.5)}Z` +
    `M${f.p(-1.5, -5.2)}Q${f.p(-1.4, -6.3)} ${f.p(0, -6.3)}Q${f.p(1.4, -6.3)} ${f.p(1.5, -5.2)}Z`;
  const spout = taper(f, [[2.2, -2], [3.8, -2.6], [4.5, -3.8], [5.1, -4.9]], 1.5, 0.65);
  const handle = f.band([[-2.6, -4.3], [-4.1, -4.3], [-4.7, -3], [-4, -1.6], [-2.8, -1.5]], 0.7);
  const out = [
    shape(spout + handle, "slate-700", 1, MAIN_INKED),
    shape(body, "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(1.5, -5.2)}C${f.p(3.7, -4.8)} ${f.p(3.6, -0.9)} ${f.p(2.1, -0.5)}L${f.p(0.9, -0.5)}C${f.p(2.4, -1.2)} ${f.p(2.6, -4.2)} ${f.p(1.5, -5.2)}Z`, "slate-900", 0.3),
    shape(f.band([[-2.3, -4.5], [0, -4.15], [2.3, -4.5]], 0.3), "slate-900", 0.35),
    shape(f.ellipse(-1.5, -3.2, 0.5, 0.95) + f.ellipse(-0.6, -5.6, 0.45, 0.2), "slate-100", 0.45),
    shape(f.circle(0, -6.75, 0.5), "slate-900", 1),
    shape(wisp(f, rng, 5.2, -5.4, 5, 0.6, 0.55), "slate-400", 0.45),
  ];
  if (rng() < 0.6) out.push(shape(wisp(f, rng, 0.9, -7.4, 4, 0.5, 0.45), "slate-400", 0.35));
  return out;
}

/** An abacus standing on the anchor: a wood frame (wood-600) with a divider bar and two feet, five to seven rods of lozenge beads (main, slate-800), one above the bar and four below, some pushed to it; about 16 wide and 10.2 tall at scale 1. */
function abacus(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const [inLeft, inRight, upper, bar, lower, bottom] = [-7.1, 7.1, -9.3, -7.5, -6.7, -1.5];
  const n = whole(rng, 5, 7);
  const pitch = (inRight - inLeft) / n;
  const bw = Math.min(0.95, pitch * 0.42);
  const bh = 0.42;
  const step = 2 * bh + 0.05;
  let rods = "";
  let beads = "";
  for (let k = 0; k < n; k += 1) {
    const x = inLeft + pitch * (k + 0.5);
    rods += f.band([[x, upper], [x, bottom]], 0.22);
    beads += lozenge(f, x, rng() < 0.5 ? upper + bh + 0.03 : bar - bh - 0.03, bw, bh);
    const up = whole(rng, 0, 4);
    for (let i = 0; i < 4; i += 1) beads += lozenge(f, x, i < up ? lower + bh + 0.03 + i * step : bottom - bh - 0.03 - (3 - i) * step, bw, bh);
  }
  const frame = rect(f, -8, -10.2, 8, -0.6) + rect(f, inLeft, upper, inRight, bar) + rect(f, inLeft, lower, inRight, bottom) + rect(f, -8, -0.6, -6.6, 0) + rect(f, 6.6, -0.6, 8, 0);
  return [
    shape(rods, "wood-800", 0.9),
    shape(frame, "wood-600", 1, INKED),
    shape(f.band([[-7.6, -9.85], [7.6, -9.85]], 0.3) + f.band([[inLeft, bar + 0.15], [inRight, bar + 0.15]], 0.2), "slate-100", 0.3),
    shape(f.band([[inLeft, bottom + 0.3], [inRight, bottom + 0.3]], 0.5) + f.band([[inLeft, lower + 0.3], [inRight, lower + 0.3]], 0.3), "slate-900", 0.25),
    shape(beads, "slate-800", 1, MAIN),
  ];
}

/** A three-legged bronze censer standing on the anchor (main, brass-600), two or three incense sticks glowing in it and one or two threads of smoke curling about 14 up from it; about 6.5 wide and 6.3 tall at scale 1, its sticks and smoke above. */
function incense(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sticks = whole(rng, 2, 3);
  const tips: Point[] = [];
  let stems = "";
  for (let k = 0; k < sticks; k += 1) {
    const x = (k - (sticks - 1) / 2) * 1.3;
    const tip: Point = [x * 1.25 + between(rng, -0.2, 0.2), between(rng, -9.4, -8.2)];
    stems += f.band([[x, -4.6], tip], 0.22);
    tips.push(tip);
  }
  const body =
    `M${f.p(-2.8, -4.4)}L${f.p(2.8, -4.4)}C${f.p(2.8, -2.2)} ${f.p(1.7, -1.3)} ${f.p(0, -1.3)}C${f.p(-1.7, -1.3)} ${f.p(-2.8, -2.2)} ${f.p(-2.8, -4.4)}Z` +
    rect(f, -3.2, -5, 3.2, -4.4);
  const ears = [-2.3, 2.3].map((x) => rect(f, x - 0.55, -6.3, x + 0.55, -5) + rect(f, x - 0.25, -6, x + 0.25, -5.3)).join("");
  const legs =
    f.poly([[-2.1, -1.8], [-1.3, -1.6], [-1.9, 0], [-2.6, 0]]) + f.poly([[1.3, -1.6], [2.1, -1.8], [2.6, 0], [1.9, 0]]) + f.poly([[-0.35, -1.5], [0.35, -1.5], [0.3, 0], [-0.3, 0]]);
  const first = tips[0] ?? [0, -9];
  const last = tips[tips.length - 1] ?? first;
  const out = [
    shape(legs, "brass-800", 1),
    shape(ears, "brass-600", 1, MAIN),
    shape(stems, "red-900", 1),
    shape(body, "brass-600", 1, MAIN_INKED),
    shape(f.band([[-2.55, -3.5], [2.55, -3.5]], 0.5), "brass-800", 0.6),
    shape(f.band([[-2.2, -4], [-1.6, -2.1]], 0.45), "slate-100", 0.3),
    shape(tips.map(([x, y]) => f.circle(x, y, 0.6)).join(""), "orange-400", 0.3),
    shape(tips.map(([x, y]) => f.circle(x, y, 0.22)).join(""), "ember-500", 1),
    shape(wisp(f, rng, first[0], first[1] - 0.3, between(rng, 9.5, 11), 1.2, 0.5), "slate-400", 0.45),
  ];
  if (rng() < 0.6) out.push(shape(wisp(f, rng, last[0], last[1] - 0.3, between(rng, 7, 9), 1, 0.45), "slate-400", 0.3));
  return out;
}

/** A go board on its feet standing on the anchor, seen a little from above: its top and front (main, wood-600) ruled with a grid, three to six black and white stones on it; about 20 wide and 6.4 tall at scale 1. */
function goBoard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const [front, back, base] = [-3.4, -6.4, -1.2];
  const on = (u: number, v: number): Point => [(-10 + 20 * u) * (1 - v) + (-8.6 + 17.2 * u) * v, front + (back - front) * v];
  const cols = 9;
  const rows = 6;
  const u = (i: number): number => 0.07 + (0.86 * i) / (cols - 1);
  const v = (j: number): number => 0.12 + (0.76 * j) / (rows - 1);
  let lines = "";
  for (let i = 0; i < cols; i += 1) lines += f.band([on(u(i), v(0)), on(u(i), v(rows - 1))], 0.16);
  let ranks = "";
  for (let j = 0; j < rows; j += 1) ranks += f.band([on(u(0), v(j)), on(u(cols - 1), v(j))], 0.16);
  const stones = whole(rng, 3, 6);
  const start = whole(rng, 0, cols - 1);
  const stride = rng() < 0.5 ? 2 : 4;
  let black = "";
  let white = "";
  let shine = "";
  for (let k = 0; k < stones; k += 1) {
    const [x, y] = on(u((start + k * stride) % cols), v(whole(rng, 0, rows - 1)));
    const stone = f.ellipse(x, y, 0.62, 0.38);
    if (k % 2 === 0) {
      black += stone;
      shine += f.ellipse(x - 0.2, y - 0.12, 0.2, 0.1);
    } else white += stone;
  }
  const foot = (x: number): string => `M${f.p(x - 0.6, base)}L${f.p(x + 0.6, base)}Q${f.p(x + 0.9, -0.4)} ${f.p(x + 0.3, 0)}L${f.p(x - 0.3, 0)}Q${f.p(x - 0.9, -0.4)} ${f.p(x - 0.6, base)}Z`;
  return [
    shape(foot(-7.8) + foot(7.8), "wood-800", 1),
    shape(f.poly([[-8.6, back], [8.6, back], [10, front], [10, base], [-10, base], [-10, front]]), "wood-600", 1, MAIN_INKED),
    shape(rect(f, -10, front, 10, base), "slate-900", 0.3),
    shape(f.band([[-8.4, back + 0.3], [8.4, back + 0.3]], 0.3), "slate-100", 0.25),
    shape(lines, "slate-900", 0.55),
    shape(ranks, "slate-900", 0.55),
    shape(black, "slate-900", 1),
    shape(white, "slate-100", 1),
    shape(shine, "slate-400", 0.7),
  ];
}

/** An open folding fan, its pivot at the anchor: ribs (wood-700) under a pleated paper arc (main, slate-100) washed with distant hills and a red sun, two guard sticks and a rivet; about 15 wide and 8.2 tall at scale 1. */
function foldingFan(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = between(rng, 70, 78);
  const ribs = whole(rng, 10, 12);
  const [outer, inner, hinge] = [7.5, 3, 1.6];
  const angles = Array.from({ length: ribs + 1 }, (_, k) => -90 - half + (2 * half * k) / ribs);
  const first = angles[0] ?? -90 - half;
  const last = angles[ribs] ?? -90 + half;
  const sticks = angles.map((a) => f.band([polar(hinge, a), polar(inner + 0.4, a)], 0.28)).join("");
  const paper = f.poly([...angles.map((a) => polar(outer, a)), ...[...angles].reverse().map((a) => polar(inner, a))]);
  let pleats = "";
  for (let k = 0; k < ribs; k += 2) {
    const a = angles[k] ?? first;
    const b = angles[k + 1] ?? last;
    pleats += f.poly([polar(inner, a), polar(outer, a), polar(outer, b), polar(inner, b)]);
  }
  const hills: Point[] = [];
  const steps = 10;
  for (let i = 0; i <= steps; i += 1) {
    const a = first + 6 + ((last - first - 12) * i) / steps;
    hills.push(polar(i % 2 === 0 ? between(rng, 4.3, 4.8) : between(rng, 5, 5.6), a));
  }
  for (let i = steps; i >= 0; i -= 1) hills.push(polar(3.6, first + 6 + ((last - first - 12) * i) / steps));
  const sun = polar(6.3, between(rng, -125, -55));
  return [
    shape(f.poly([[0, 0], ...angles.map((a) => polar(hinge, a))]) + sticks, "wood-700", 1),
    shape(paper, "slate-100", 1, MAIN_INKED),
    shape(pleats, "slate-900", 0.1),
    shape(f.poly(hills), "jade-400", 0.4),
    shape(f.circle(...sun, 0.65), "red-600", 0.85),
    shape(f.band([polar(0.8, first), polar(outer + 0.2, first)], 0.55) + f.band([polar(0.8, last), polar(outer + 0.2, last)], 0.55), "wood-800", 1),
    shape(f.circle(0, 0, 0.6), "brass-600", 1),
  ];
}

/** A clepsydra standing on the anchor: two or three stepped bronze vessels (main, brass-600) on a wooden stand, each spilling a drip into the next below; about 8.8 wide and 16.5 tall at scale 1. */
function waterClock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const vessels =
    rng() < 0.6
      ? [
          { x: -1.3, w: 4, base: -12.6, h: 3 },
          { x: 0.1, w: 4.2, base: -7, h: 3 },
          { x: 1.3, w: 4.8, base: -0.8, h: 3.6 },
        ]
      : [
          { x: -0.9, w: 4.4, base: -11, h: 4 },
          { x: 1, w: 5, base: -0.8, h: 4.2 },
        ];
  const topOf = vessels[0] ?? { x: 0, w: 4, base: -12, h: 3 };
  let stand = rect(f, -4, topOf.base - topOf.h - 1, -3.2, -0.8) + rect(f, -4.4, -0.8, 4.4, 0);
  let bodies = "";
  let trims = "";
  let water = "";
  let stream = "";
  let drops = "";
  let lights = "";
  vessels.forEach(({ x, w, base, h }, k) => {
    const top = base - h;
    const [l, r] = [x - w / 2, x + w / 2];
    bodies +=
      `M${f.p(l - 0.3, top)}L${f.p(r + 0.3, top)}L${f.p(r + 0.3, top + 0.5)}L${f.p(r, top + 0.5)}Q${f.p(r, base)} ${f.p(r - 0.8, base)}` +
      `L${f.p(l + 0.8, base)}Q${f.p(l, base)} ${f.p(l, top + 0.5)}L${f.p(l - 0.3, top + 0.5)}Z`;
    trims += f.band([[l + 0.25, top + h * 0.55], [r - 0.25, top + h * 0.55]], 0.4);
    water += f.ellipse(x, top, w / 2 + 0.1, 0.32);
    lights += f.band([[l + 0.6, top + 0.9], [l + 0.75, base - 0.5]], 0.35);
    const next = vessels[k + 1];
    if (next === undefined) return;
    stand += rect(f, -3.2, base, r - 0.3, base + 0.6);
    trims += f.band([[r + 0.1, top + 0.75], [r + 0.8, top + 1.1]], 0.45);
    const fall = next.base - next.h - (top + 1.3);
    stream += f.band([[r + 0.85, top + 1.3], [r + 0.85, top + 1.3 + fall - 0.35]], 0.14);
    for (const t of [between(rng, 0.2, 0.35), between(rng, 0.6, 0.75)]) drops += drop(f, r + 0.85, top + 1.3 + fall * t, 0.28);
  });
  return [
    shape(stand, "wood-700", 1, INKED),
    shape(bodies, "brass-600", 1, MAIN_INKED),
    shape(trims, "brass-800", 0.9),
    shape(lights, "slate-100", 0.3),
    shape(water, "sky-300", 0.75),
    shape(stream, "sky-300", 0.45),
    shape(drops, "blue-200", 0.95),
  ];
}

/** A drift of three to five sealed envelopes lying on the anchor at angles, the near ones over the far: each sheet (main, slate-100) inked, its flap line and a red wax seal; about 15 wide and 6.3 tall at scale 1. */
function envelope(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 3, 5);
  const step = n === 5 ? 2.6 : n === 4 ? 3.3 : 4.4;
  const [hw, hh] = [2.2, 1.6];
  const papers = ["", ""];
  const flaps = ["", ""];
  const seals = ["", ""];
  for (let k = 0; k < n; k += 1) {
    const near = k % 2;
    const cx = (k - (n - 1) / 2) * step + between(rng, -0.15, 0.15);
    const cy = (near === 1 ? -1.6 : -4) + between(rng, -0.15, 0.15);
    const angle = between(rng, -8, 8);
    const corner = (x: number, y: number): Point => {
      const [dx, dy] = turned([x, y], angle);
      return [cx + dx, cy + dy];
    };
    papers[near] += f.poly([corner(-hw, -hh), corner(hw, -hh), corner(hw, hh), corner(-hw, hh)]);
    flaps[near] += f.band([corner(-hw + 0.3, -hh + 0.25), corner(0, -0.1), corner(hw - 0.3, -hh + 0.25)], 0.25);
    seals[near] += f.circle(...corner(0, -0.1), 0.5);
  }
  return [0, 1].flatMap((near) => [
    shape(papers[near] ?? "", "slate-100", 1, MAIN_INKED),
    shape(flaps[near] ?? "", "slate-700", 0.6),
    shape(seals[near] ?? "", "red-600", 1),
  ]);
}

/** A lacquered bamboo cane (main, wood-700) about 28 long, centred on the anchor and slanting the way it faces: its node bands, a red cord wrapped below its grip with a tassel, a metal tip. */
function cane(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 24, 34) * DEGREES;
  const [ux, uy] = [Math.sin(tilt), -Math.cos(tilt)];
  const a = (t: number, s: number): Point => [ux * t - uy * s, uy * t + ux * s];
  const p = (t: number, s: number): string => f.p(...a(t, s));
  const w = 0.55;
  const body = `M${p(-14, -w)}L${p(13.4, -w)}Q${p(14.1, -w)} ${p(14.1, 0)}Q${p(14.1, w)} ${p(13.4, w)}L${p(-14, w)}Z`;
  const nodes = [-9, -3.5, 2].map((t) => t + between(rng, -0.4, 0.4));
  const shine = [-13, ...nodes, 7.6].slice(0, -1).map((t, k) => f.band([a(t + 0.5, -0.25), a((nodes[k] ?? 7.6) - 0.5, -0.25)], 0.22));
  const wraps = [8, 8.75, 9.5, 10.25, 11].map((t) => f.band([a(t, -0.62), a(t + 0.55, 0.62)], 0.34)).join("");
  const [hx, hy] = a(8.6, 0.85);
  const tassel = f.poly([[hx - 0.12, hy], [hx + 0.12, hy], [hx + 0.12, hy + 2.2], [hx + 0.4, hy + 2.2], [hx + 0.55, hy + 3.6], [hx - 0.55, hy + 3.6], [hx - 0.4, hy + 2.2], [hx - 0.12, hy + 2.2]]);
  return [
    shape(body, "wood-700", 1, MAIN_INKED),
    shape(f.poly([a(-14, -w), a(-12.9, -w), a(-12.9, w), a(-14, w)]), "slate-800", 1),
    shape(nodes.map((t) => f.band([a(t, -0.8), a(t, 0.8)], 0.35)).join(""), "wood-900", 0.85),
    shape(shine.join(""), "slate-100", 0.4),
    shape(wraps, "red-600", 1),
    shape(tassel, "red-600", 1),
  ];
}

/** R3404000: a fisherman's oar lying level, centred on the anchor, its blade the way it faces: a long shaft and a broad round-tipped blade in one piece (main, wood-700), its wet tip darker, a worn grip, a red cord lashed where the blade meets the shaft and a drop or two falling from it; about 30 long and 4.2 tall at scale 1. */
function oar(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tip = between(rng, 14.6, 15.4);
  const neck = between(rng, 5.6, 6.4);
  const body =
    `M${f.p(-15, -0.45)}L${f.p(neck, -0.45)}L${f.p(neck + 1.6, -1.9)}L${f.p(tip - 1.2, -2.1)}Q${f.p(tip, -2.1)} ${f.p(tip, 0)}` +
    `Q${f.p(tip, 2.1)} ${f.p(tip - 1.2, 2.1)}L${f.p(neck + 1.6, 1.9)}L${f.p(neck, 0.45)}L${f.p(-15, 0.45)}Z`;
  const wet = `M${f.p(tip - 4, -2.05)}L${f.p(tip - 1.2, -2.1)}Q${f.p(tip, -2.1)} ${f.p(tip, 0)}Q${f.p(tip, 2.1)} ${f.p(tip - 1.2, 2.1)}L${f.p(tip - 4, 2.05)}Z`;
  const lashing = [0, 0.55, 1.1].map((dx) => f.band([[neck - 1.6 + dx, -0.75], [neck - 1.2 + dx, 0.75]], 0.32)).join("");
  const drops = whole(rng, 1, 2);
  let falling = "";
  for (let k = 0; k < drops; k += 1) falling += drop(f, tip - 1.5 - k * 2.2 + between(rng, -0.3, 0.3), 2.8 + k * 1.4, 0.32);
  return [
    shape(body, "wood-700", 1, MAIN_INKED),
    shape(wet, "slate-900", 0.3),
    shape(f.band([[-14.4, -0.12], [neck - 0.4, -0.12]], 0.22) + f.band([[neck + 2, -1.05], [tip - 1.6, -1.15]], 0.3), "slate-100", 0.4),
    shape(rect(f, -15, -0.6, -12.4, 0.6), "slate-800", 1),
    shape(lashing, "red-600", 1),
    shape(falling, "slate-100", 0.6),
  ];
}

/** R3404000: a low sofa standing on the anchor: its back, seat and two rolled arms in one inked piece (main, blue-800), two or three seat cushions marked by their seams, a plump throw cushion at one end, short dark feet and its shadow; about 20.4 wide and 9.2 tall at scale 1. */
function sofa(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 2, 3);
  const back = between(rng, -9.4, -8.8);
  const body = rect(f, -8.4, back, 8.4, -5) + rect(f, -8.4, -5, 8.4, -1.2) + rect(f, -10.2, -6.4, -8.4, -1.2) + rect(f, 8.4, -6.4, 10.2, -1.2);
  let seams = "";
  for (let k = 1; k < n; k += 1) {
    const x = -8.4 + (16.8 * k) / n;
    seams += f.band([[x, back + 0.5], [x, -5.3]], 0.22) + f.band([[x, -4.8], [x, -1.5]], 0.22);
  }
  const side = rng() < 0.5 ? -1 : 1;
  const cx = side * between(rng, 5.4, 6.4);
  const cushion = `M${f.p(cx - 2, -7.6)}Q${f.p(cx, -8.4)} ${f.p(cx + 2, -7.6)}Q${f.p(cx + 2.6, -6.1)} ${f.p(cx + 2, -4.6)}Q${f.p(cx, -4)} ${f.p(cx - 2, -4.6)}Q${f.p(cx - 2.6, -6.1)} ${f.p(cx - 2, -7.6)}Z`;
  return [
    shape(f.ellipse(0, 0, 10.8, 0.7), "slate-900", 0.25),
    shape(rect(f, -9.8, -1.2, -8.9, 0) + rect(f, 8.9, -1.2, 9.8, 0), "wood-900", 1),
    shape(body, "blue-800", 1, MAIN_INKED),
    shape(f.ellipse(-9.3, -6.4, 0.95, 0.5) + f.ellipse(9.3, -6.4, 0.95, 0.5), "slate-900", 0.3),
    shape(seams + f.band([[-8.2, -5], [8.2, -5]], 0.3), "slate-900", 0.35),
    shape(f.band([[-8, back + 0.4], [8, back + 0.4]], 0.3) + f.band([[-8, -4.6], [8, -4.6]], 0.25), "slate-100", 0.3),
    shape(cushion, "red-600", 1, INKED),
  ];
}

/** R3401000: the objects the Meditative scenes added, by id, each with its kind. R3404000: the oar and the sofa the Alt art briefs added after them. */
export const INK_OBJECTS_KIT = {
  "ink-seal": { category: "objects", draw: seal },
  "paper-lantern": { category: "objects", draw: paperLantern },
  "lantern-string": { category: "objects", draw: lanternString },
  "ink-card-fan": { category: "objects", draw: cardFan },
  brush: { category: "objects", draw: brush },
  teapot: { category: "objects", draw: teapot },
  abacus: { category: "objects", draw: abacus },
  incense: { category: "objects", draw: incense },
  "go-board": { category: "objects", draw: goBoard },
  "folding-fan": { category: "objects", draw: foldingFan },
  "water-clock": { category: "objects", draw: waterClock },
  "temple-bell": { category: "objects", draw: bell },
  "red-thread": { category: "objects", draw: redThread },
  "envelope-drift": { category: "objects", draw: envelope },
  cane: { category: "objects", draw: cane },
  oar: { category: "objects", draw: oar },
  sofa: { category: "objects", draw: sofa },
} as const satisfies Readonly<Record<string, KitPart>>;
