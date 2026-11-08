// R3331000: the procedural painter's first scene kit (ART.md, "Scene recipes and kits"): the scenery a
// scene recipe (sceneRecipes.ts) places, in seven kinds (terrain, architecture, plants, creatures,
// objects, weather and light, and R3391000's craft: the ships, drones and vehicles of Stinkyscape's
// pass, whose parts of every kind follow the first kit's). Each part draws one to twelve filled shapes
// about its anchor, scaled and faced as it is placed, varied by the scene's stream and never by the face: a Radiant face draws
// the same shapes, only repainted (scene.ts `withScene`). Every colour is a palette.css name, none in
// Radiant's gold window (no `gold-*`, `yellow-400` or `parchment`); a glow is a stack of translucent
// shapes, never a gradient. A terrain part spans the box from its placement's height down; the rest
// stand on their anchor (a plant, a building, an object), hang from it (a lantern, a light shaft) or
// centre on it (a creature in flight, a moon, a spark). Shapes are filled even-odd, as the painter
// fills every ridge, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.
// A pointing part (a beam, a light ribbon, a spotlight, a meteor, a smoke trail, a comet) also turns by
// its placement's `rotate` about its anchor, extending along +x unturned; every other part ignores it.

import { fmt } from "./hash.ts";
import type { KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";

/** The scene's box (scene.ts re-exports it, and `scene.test.ts` holds it to the painter's `ART_BOX`). */
export const SCENE_BOX = 100;
/** Where a terrain band starts and ends: a little past the box, so no seam shows at its edges. */
const EDGE = -2;
const FAR_EDGE = SCENE_BOX + 2;
const FULL_TURN = 360;
const DEGREES = Math.PI / 180;

type Point = readonly [number, number];

function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

function whole(rng: Rng, low: number, high: number): number {
  return low + Math.floor(rng() * (high - low + 1));
}

export function pt(x: number, y: number): string {
  return `${fmt(x)} ${fmt(y)}`;
}

export function circlePath(cx: number, cy: number, r: number): string {
  return `M${pt(cx - r, cy)}A${fmt(r)} ${fmt(r)} 0 1 0 ${pt(cx + r, cy)}A${fmt(r)} ${fmt(r)} 0 1 0 ${pt(cx - r, cy)}Z`;
}

export function ellipsePath(cx: number, cy: number, rx: number, ry: number): string {
  return `M${pt(cx - rx, cy)}A${fmt(rx)} ${fmt(ry)} 0 1 0 ${pt(cx + rx, cy)}A${fmt(rx)} ${fmt(ry)} 0 1 0 ${pt(cx - rx, cy)}Z`;
}

export function rectPath(x: number, y: number, w: number, h: number): string {
  return `M${pt(x, y)}L${pt(x + w, y)}L${pt(x + w, y + h)}L${pt(x, y + h)}Z`;
}

export function polygonPath(cx: number, cy: number, sides: number, radius: number, offset: number): string {
  let d = "";
  for (let i = 0; i < sides; i += 1) {
    const angle = (offset + (i * FULL_TURN) / sides) * DEGREES;
    d += `${i === 0 ? "M" : "L"}${pt(cx + radius * Math.cos(angle), cy + radius * Math.sin(angle))}`;
  }
  return `${d}Z`;
}

/** A band `width` wide along the points: how a kit draws a line, since every shape is filled. */
export function bandPath(points: readonly Point[], width: number): string {
  const left: string[] = [];
  const right: string[] = [];
  for (let i = 0; i < points.length; i += 1) {
    const [px, py] = points[Math.max(0, i - 1)] ?? [0, 0];
    const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [0, 0];
    const [x, y] = points[i] ?? [0, 0];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const ox = (-(ny - py) / length) * (width / 2);
    const oy = ((nx - px) / length) * (width / 2);
    left.push(pt(x + ox, y + oy));
    right.push(pt(x - ox, y - oy));
  }
  return `M${left.join("L")}L${right.reverse().join("L")}Z`;
}

/** A part's own frame: offsets from its anchor, scaled by its `s` and mirrored when it faces left. */
export type Local = {
  x(dx: number): number;
  y(dy: number): number;
  p(dx: number, dy: number): string;
  /** A length in the part's frame, scaled. */
  r(length: number): number;
  /** An arc's sweep flag, `sweep` as drawn facing right. */
  sweep(sweep: 0 | 1): 0 | 1;
  poly(points: readonly Point[]): string;
  circle(dx: number, dy: number, r: number): string;
  ellipse(dx: number, dy: number, rx: number, ry: number): string;
  band(points: readonly Point[], width: number): string;
};

export function local(at: PartPlace): Local {
  const x = (dx: number): number => at.x + at.flip * at.s * dx;
  const y = (dy: number): number => at.y + at.s * dy;
  const p = (dx: number, dy: number): string => pt(x(dx), y(dy));
  const r = (length: number): number => at.s * length;
  return {
    x,
    y,
    p,
    r,
    sweep: (sweep) => (at.flip === 1 ? sweep : sweep === 1 ? 0 : 1),
    poly: (points) => `M${points.map(([dx, dy]) => p(dx, dy)).join("L")}Z`,
    circle: (dx, dy, radius) => circlePath(x(dx), y(dy), r(radius)),
    ellipse: (dx, dy, rx, ry) => ellipsePath(x(dx), y(dy), r(rx), r(ry)),
    band: (points, width) => bandPath(points.map(([dx, dy]) => [x(dx), y(dy)] as const), r(width)),
  };
}

type Flags = { readonly inked?: boolean; readonly main?: boolean };
const MAIN: Flags = { main: true };
const INKED: Flags = { inked: true };
const MAIN_INKED: Flags = { main: true, inked: true };

function shape(d: string, paint: PaletteName, opacity: number, flags?: Flags): KitShape {
  return flags === undefined ? { d, paint, opacity } : { d, paint, opacity, ...flags };
}

/** `point` turned `degrees` about the part's anchor (clockwise on screen). */
function turned([dx, dy]: Point, degrees: number): Point {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  return [dx * c - dy * s, dx * s + dy * c];
}

/** `point` leant by `lean` (a shear): its top moves sideways by `lean` per unit of height. */
function leant([dx, dy]: Point, lean: number): Point {
  return [dx - lean * dy, dy];
}

/** A four-pointed sparkle about a point, `size` its reach. */
function sparkle(f: Local, cx: number, cy: number, size: number): string {
  const points: Point[] = [];
  for (let i = 0; i < 8; i += 1) {
    const angle = (i * FULL_TURN) / 8 - 90;
    const reach = i % 2 === 0 ? size : size * 0.28;
    points.push([cx + reach * Math.cos(angle * DEGREES), cy + reach * Math.sin(angle * DEGREES)]);
  }
  return f.poly(points);
}

/** A wobbly round blob, smoothed through the midpoints of its jittered rim. */
function blob(f: Local, rng: Rng, cx: number, cy: number, r: number, points: number): string {
  const rim: Point[] = [];
  for (let i = 0; i < points; i += 1) {
    const angle = (i * FULL_TURN) / points;
    const reach = r * between(rng, 0.75, 1.15);
    rim.push([cx + reach * Math.cos(angle * DEGREES), cy + reach * Math.sin(angle * DEGREES)]);
  }
  const mid = (a: Point, b: Point): string => f.p((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
  const at = (i: number): Point => rim[i % points] ?? [cx, cy];
  let d = `M${mid(at(0), at(1))}`;
  for (let i = 1; i <= points; i += 1) d += `Q${f.p(...at(i))} ${mid(at(i), at(i + 1))}`;
  return `${d}Z`;
}

/** The ground from a wavy top edge at `top` down past the box, across all of it. */
function groundBand(rng: Rng, top: number, amplitude: number, bumps: number): string {
  const ys: number[] = [];
  for (let i = 0; i <= bumps; i += 1) ys.push(top + between(rng, -amplitude, amplitude));
  const xAt = (i: number): number => EDGE + ((FAR_EDGE - EDGE) * i) / bumps;
  const yAt = (i: number): number => ys[i] ?? top;
  let d = `M${pt(EDGE, FAR_EDGE)}L${pt(EDGE, yAt(0))}`;
  for (let i = 1; i < bumps; i += 1) d += `Q${pt(xAt(i), yAt(i))} ${pt((xAt(i) + xAt(i + 1)) / 2, (yAt(i) + yAt(i + 1)) / 2)}`;
  return `${d}L${pt(FAR_EDGE, yAt(bumps))}L${pt(FAR_EDGE, FAR_EDGE)}Z`;
}

/** A crescent along the side the part faces: an arc of `r`, hollowed by a narrower one. */
function crescent(f: Local, r: number, inner: number): string {
  return (
    `M${f.p(0, -r)}A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(0, r)}` +
    `A${fmt(f.r(r * inner))} ${fmt(f.r(r))} 0 0 ${f.sweep(0)} ${f.p(0, -r)}Z`
  );
}

/** A tapering spiral arm about the anchor, from `r0` out to `r1`, `sweep` degrees round. */
function spiralArm(f: Local, start: number, sweep: number, r0: number, r1: number, width: number): string {
  const steps = 8;
  const outer: Point[] = [];
  const inner: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const r = r0 + (r1 - r0) * t;
    const half = (width * (1 - t)) / 2 + 0.15;
    const angle = (start + sweep * t) * DEGREES;
    outer.push([(r + half) * Math.cos(angle), (r + half) * Math.sin(angle)]);
    inner.push([Math.max(0.3, r - half) * Math.cos(angle), Math.max(0.3, r - half) * Math.sin(angle)]);
  }
  return f.poly([...outer, ...inner.reverse()]);
}

// Terrain

function field(at: PartPlace, rng: Rng): KitShape[] {
  const out = [shape(groundBand(rng, at.y, 1.2 * at.s, 4), "green-700", 1, MAIN)];
  const rows = whole(rng, 2, 3);
  for (let k = 1; k <= rows; k += 1) {
    const y = at.y + (SCENE_BOX - at.y) * (k / (rows + 1)) ** 1.3;
    const width = 0.6 + 0.5 * k * at.s;
    out.push(shape(bandPath([[EDGE, y + between(rng, -1, 1)], [SCENE_BOX / 2, y], [FAR_EDGE, y + between(rng, -1, 1)]], width), "green-500", 0.3));
  }
  return out;
}

/** Tilled rows running to a point on the horizon at the placement's x. */
function furrows(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const out = [shape(`M${pt(EDGE, top)}L${pt(FAR_EDGE, top)}L${pt(FAR_EDGE, FAR_EDGE)}L${pt(EDGE, FAR_EDGE)}Z`, "wood-800", 1, MAIN)];
  const rows = whole(rng, 6, 7);
  const from = -40;
  const to = 140;
  for (let i = 0; i < rows; i += 1) {
    const foot = from + ((to - from) * (i + 0.5)) / rows + between(rng, -2, 2);
    const half = ((to - from) / rows) * 0.2 * at.s;
    const apex = at.x + (foot - at.x) * 0.03;
    out.push(shape(`M${pt(apex - 0.15, top)}L${pt(apex + 0.15, top)}L${pt(foot + half, FAR_EDGE)}L${pt(foot - half, FAR_EDGE)}Z`, "wood-700", 0.95));
  }
  return out;
}

function graveMound(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 7 * between(rng, 0.85, 1.15);
  const h = 3.4 * between(rng, 0.8, 1.2);
  return [
    shape(`M${f.p(-w, 0)}C${f.p(-w * 0.6, -h * 1.35)} ${f.p(w * 0.6, -h * 1.35)} ${f.p(w, 0)}Z`, "wood-600", 1, MAIN_INKED),
    shape(`M${f.p(-w, 0)}Q${f.p(0, -h * 0.6)} ${f.p(w, 0)}Z`, "wood-800", 0.6),
    shape(`M${f.p(-w * 0.7, -h * 0.5)}Q${f.p(-w * 0.2, -h * 1.12)} ${f.p(w * 0.3, -h * 0.95)}Q${f.p(-w * 0.2, -h * 0.9)} ${f.p(-w * 0.7, -h * 0.5)}Z`, "brass-600", 0.35),
  ];
}

/** A cliff rising to the anchor, its mass behind (to the left as drawn facing right) and its face lit. */
function cliff(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = 66;
  const top: Point[] = [
    [-64, between(rng, -2, 0)],
    [-48, between(rng, -6, -3)],
    [-34, between(rng, -2, 0)],
    [-22, between(rng, -7, -4)],
    [-10, between(rng, -3, -1)],
    [0, 0],
  ];
  const face: Point[] = [
    [between(rng, 4, 7), 14],
    [between(rng, 1, 4), 28],
    [between(rng, 6, 9), 44],
    [between(rng, 3, 6), h],
  ];
  const lit: Point[] = [[0, 0], ...face, ...face.map(([dx, dy]) => [dx - 5, dy] as const).reverse(), [-3, 1]];
  return [
    shape(f.poly([[-64, h], ...top, ...face]), "slate-700", 1, MAIN_INKED),
    shape(f.poly(lit), "slate-400", 0.3),
    shape(f.band([[-20, -3], [-18, 10], [-22, 24], [-19, 40]], 0.8), "slate-900", 0.5),
    shape(f.band([[-42, -2], [-40, 14], [-44, 30]], 0.7), "slate-900", 0.45),
    shape(f.band(top, 1.2), "slate-400", 0.35),
  ];
}

function rockFloor(at: PartPlace, rng: Rng): KitShape[] {
  const out = [shape(groundBand(rng, at.y, 1.5 * at.s, 5), "slate-800", 1, MAIN)];
  const stones = whole(rng, 4, 6);
  for (let k = 0; k < stones; k += 1) {
    const x = between(rng, 4, 96);
    const y = between(rng, at.y + 3, SCENE_BOX - 3);
    const rx = between(rng, 2, 4.5) * at.s * (0.5 + (y - at.y) / Math.max(1, SCENE_BOX - at.y));
    const ry = rx * between(rng, 0.35, 0.55);
    out.push(shape(ellipsePath(x, y, rx, ry), "slate-700", 1));
    if (k < 3) out.push(shape(ellipsePath(x - rx * 0.25, y - ry * 0.35, rx * 0.5, ry * 0.35), "slate-400", 0.45));
  }
  const crackX = between(rng, 20, 80);
  out.push(shape(bandPath([[crackX, at.y + 2], [crackX + 3, at.y + 6], [crackX + 1, at.y + 10]], 0.5), "slate-900", 0.5));
  return out;
}

/** A floor of tiles in perspective, its lines meeting at the placement's x on its horizon. */
function tileFloor(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const out = [shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "stone-500", 1, MAIN)];
  for (let k = 1; k <= 4; k += 1) out.push(shape(rectPath(EDGE, top + depth * (k / 4.5) ** 1.7, FAR_EDGE - EDGE, 0.3 + k * 0.15), "slate-700", 0.45));
  for (let i = 0; i < 5; i += 1) {
    const foot = -30 + (160 * (i + 0.5)) / 5;
    const apex = at.x + (foot - at.x) * 0.25;
    out.push(shape(`M${pt(apex - 0.12, top)}L${pt(apex + 0.12, top)}L${pt(foot + 0.5, FAR_EDGE)}L${pt(foot - 0.5, FAR_EDGE)}Z`, "slate-700", 0.45));
  }
  out.push(shape(ellipsePath(at.x + between(rng, -8, 8), top + depth * 0.45, 26, depth * 0.2), "slate-100", 0.1));
  return out;
}

/** A room's back wall from the top of the box down to the floor at the placement's height, with a window. */
function roomWall(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const wx = between(rng, 14, 66);
  const wy = floor * 0.22;
  const ww = 14 * at.s;
  const wh = 18 * at.s;
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor - EDGE), "stone-500", 1, MAIN),
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor * 0.18), "slate-900", 0.25),
    shape(rectPath(wx, wy, ww, wh), "sky-300", 0.55),
    shape(rectPath(wx + ww / 2 - 0.4, wy, 0.8, wh) + rectPath(wx, wy + wh / 2 - 0.4, ww / 2 - 0.4, 0.8) + rectPath(wx + ww / 2 + 0.4, wy + wh / 2 - 0.4, ww / 2 - 0.4, 0.8), "slate-800", 0.9),
    shape(rectPath(EDGE, floor - 1.6, FAR_EDGE - EDGE, 1.6), "wood-700", 1),
  ];
}

/** A street running to the horizon at the placement's x, walks either side and its centre line. */
function street(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const vx = at.x;
  const left = between(rng, 10, 16);
  const right = SCENE_BOX - between(rng, 10, 16);
  const out = [
    shape(`M${pt(vx - 1.5, top)}L${pt(vx + 1.5, top)}L${pt(right, FAR_EDGE)}L${pt(left, FAR_EDGE)}Z`, "slate-700", 1, MAIN),
    shape(`M${pt(EDGE, top)}L${pt(vx - 1.5, top)}L${pt(left, FAR_EDGE)}L${pt(EDGE, FAR_EDGE)}Z`, "stone-500", 1),
    shape(`M${pt(vx + 1.5, top)}L${pt(FAR_EDGE, top)}L${pt(FAR_EDGE, FAR_EDGE)}L${pt(right, FAR_EDGE)}Z`, "stone-500", 1),
    shape(`M${pt(vx - 1.5, top)}L${pt(vx - 1.2, top)}L${pt(left + 1.4, FAR_EDGE)}L${pt(left, FAR_EDGE)}Z`, "slate-400", 0.8),
    shape(`M${pt(vx + 1.2, top)}L${pt(vx + 1.5, top)}L${pt(right, FAR_EDGE)}L${pt(right - 1.4, FAR_EDGE)}Z`, "slate-400", 0.8),
  ];
  for (const t of [0.22, 0.48, 0.82]) {
    const y = top + depth * t ** 1.4;
    const half = 0.15 + 0.8 * t;
    const length = 1 + 6 * t;
    out.push(shape(rectPath(vx - half, y, half * 2, length), "slate-100", 0.75));
  }
  return out;
}

/** Two rolling ranges across the box, the far one fainter. */
function hills(at: PartPlace, rng: Rng): KitShape[] {
  return [
    shape(groundBand(rng, at.y - 5 * at.s, 4 * at.s, 4), "green-700", 0.6, MAIN),
    shape(groundBand(rng, at.y, 3 * at.s, 3), "green-900", 1, MAIN),
  ];
}

function asteroid(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rim: Point[] = [];
  for (let i = 0; i < 9; i += 1) {
    const angle = (i * FULL_TURN) / 9;
    const reach = 4 * between(rng, 0.75, 1.08);
    rim.push([reach * Math.cos(angle * DEGREES), reach * Math.sin(angle * DEGREES)]);
  }
  return [
    shape(f.poly(rim), "slate-700", 1, MAIN_INKED),
    shape(f.ellipse(between(rng, -1.6, 0), between(rng, -0.5, 1.4), 1, 0.7), "slate-800", 0.8),
    shape(f.ellipse(between(rng, 1, 2), between(rng, -1.6, 0), 0.6, 0.45), "slate-800", 0.8),
    shape(f.ellipse(-1.2, -1.5, 1.6, 0.9), "slate-400", 0.4),
  ];
}

// Architecture

function fence(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 26;
  const out = [
    shape(f.poly([[-half, -2.6], [half, -2.6], [half, -1.9], [-half, -1.9]]), "wood-700", 1, MAIN_INKED),
    shape(f.poly([[-half, -5.4], [half, -5.4], [half, -4.7], [-half, -4.7]]), "wood-700", 1, MAIN_INKED),
  ];
  const pickets = 8;
  for (let i = 0; i < pickets; i += 1) {
    const x = -half + (2 * half * (i + 0.5)) / pickets;
    const broken = rng() < 0.15;
    const h = broken ? between(rng, 2.6, 3.6) : between(rng, 5.6, 7);
    const lean = between(rng, -0.06, 0.06);
    const w = 0.75;
    const points: Point[] = [[x - w, 0], [x - w, -h + 1], [x, -h], [x + w, -h + 1], [x + w, 0]];
    out.push(shape(f.poly(points.map((point) => leant(point, lean))), "wood-700", 1, MAIN_INKED));
  }
  return out;
}

/** A post with an arm at its top, its hook at (4.8, -11.8) of the post's frame: where a lantern hangs. */
function fencePost(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = 15;
  const lean = between(rng, -0.03, 0.03);
  const P = (point: Point): Point => leant(point, lean);
  return [
    shape(f.poly([P([-1, 0]), P([-1, -h]), P([0, -h - 1.2]), P([1, -h]), P([1, 0])]), "wood-700", 1, MAIN_INKED),
    shape(f.poly([P([0, -h + 1.8]), P([5.5, -h + 1.8]), P([5.5, -h + 0.8]), P([0, -h + 0.8])]), "wood-700", 1, MAIN_INKED),
    shape(f.band([P([4.8, -h + 1.8]), P([4.8, -h + 3.2])], 0.35), "slate-900", 1),
    shape(f.band([P([-0.2, -1]), P([-0.2, -h + 2])], 0.25), "wood-900", 0.5),
  ];
}

function gate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = 17;
  const sag = between(rng, 5, 7);
  const curve = (t: number): Point => [-7.4 + 14.8 * t, -13 - sag * 4 * t * (1 - t)];
  const out = [
    shape(f.poly([[-10.6, 0], [-10.6, -h], [-7.4, -h], [-7.4, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[7.4, 0], [7.4, -h], [10.6, -h], [10.6, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-11.1, -h], [-6.9, -h], [-6.9, -h - 1.4], [-11.1, -h - 1.4]]) + f.poly([[6.9, -h], [11.1, -h], [11.1, -h - 1.4], [6.9, -h - 1.4]]), "slate-400", 1, INKED),
    shape(f.band([0, 0.17, 0.33, 0.5, 0.67, 0.83, 1].map(curve), 0.7), "slate-900", 1),
  ];
  for (const t of [0.14, 0.38, 0.62, 0.86]) {
    const [x, y] = curve(t);
    out.push(shape(f.band([[x, 0], [x, y + 0.4]], 0.45), "slate-900", 1));
  }
  return out;
}

function ruinWall(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const jag = (dx: number, dy: number): Point => [dx, dy + between(rng, -0.8, 0.8)];
  const top: Point[] = [jag(-12, -8), jag(-8, -10.5), [-5, -12], jag(-2, -8.5), jag(1, -10), jag(4, -6), jag(7, -7.5), jag(12, -3.5)];
  const wall = f.poly([[-12, 0], ...top, [12, 0]]);
  const hole = `M${f.p(-9, -2.5)}L${f.p(-9, -6.5)}Q${f.p(-7.25, -9.4)} ${f.p(-5.5, -6.5)}L${f.p(-5.5, -2.5)}Z`;
  const brick = (x: number, y: number): string => f.poly([[x, y], [x + 3, y], [x + 3, y + 0.35], [x, y + 0.35]]);
  return [
    shape(wall + hole, "stone-500", 1, MAIN_INKED),
    shape(brick(between(rng, 0, 2), -4) + brick(between(rng, 4, 6), -2.2) + brick(between(rng, -3, -1), -1.2), "slate-700", 0.45),
    shape(f.ellipse(between(rng, -4, 4), 0, 5, 1.2), "green-900", 0.55),
    shape(f.poly([[13, 0], [13.6, -1.6], [15.4, -1.2], [16, 0]]), "stone-500", 1, INKED),
    shape(f.poly([[-15, 0], [-14.2, -1.1], [-13, -0.8], [-12.8, 0]]), "stone-500", 1, INKED),
  ];
}

function tower(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 26, 34);
  const base = 4.2;
  const crown = 3.6;
  const conical = rng() < 0.5;
  const roof = conical
    ? f.poly([[-crown - 1.2, -h], [0, -h - between(rng, 9, 13)], [crown + 1.2, -h]])
    : f.poly([
        [-crown - 0.6, -h],
        [-crown - 0.6, -h - 3],
        [-crown + 1, -h - 3],
        [-crown + 1, -h - 1.8],
        [-0.8, -h - 1.8],
        [-0.8, -h - 3],
        [0.8, -h - 3],
        [0.8, -h - 1.8],
        [crown - 1, -h - 1.8],
        [crown - 1, -h - 3],
        [crown + 0.6, -h - 3],
        [crown + 0.6, -h],
      ]);
  const window = (y: number): string => `M${f.p(-0.8, y)}L${f.p(-0.8, y - 2)}Q${f.p(0, y - 3.2)} ${f.p(0.8, y - 2)}L${f.p(0.8, y)}Z`;
  return [
    shape(f.poly([[-base, 0], [-crown, -h], [crown, -h], [base, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[base * 0.35, 0], [crown * 0.35, -h], [crown, -h], [base, 0]]), "slate-900", 0.3),
    shape(roof, "slate-700", 1, INKED),
    shape(window(-h * 0.72) + window(-h * 0.42), rng() < 0.6 ? "orange-400" : "black", 0.85),
    shape(`M${f.p(-1.4, 0)}L${f.p(-1.4, -3)}Q${f.p(0, -4.8)} ${f.p(1.4, -3)}L${f.p(1.4, 0)}Z`, "black", 0.7),
  ];
}

function column(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 24, 28);
  const w = 2.2;
  const out = [
    shape(f.poly([[-3.2, 0], [3.2, 0], [3.2, -1.6], [-3.2, -1.6]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-w, -1.6], [w, -1.6], [w, -h + 1.6], [-w, -h + 1.6]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-3.4, -h], [3.4, -h], [2.6, -h + 1.6], [-2.6, -h + 1.6]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-1.05, -2], [-0.75, -2], [-0.75, -h + 2], [-1.05, -h + 2]]) + f.poly([[0.75, -2], [1.05, -2], [1.05, -h + 2], [0.75, -h + 2]]), "slate-700", 0.3),
    shape(f.poly([[-w + 0.3, -2], [-w + 0.9, -2], [-w + 0.9, -h + 2], [-w + 0.3, -h + 2]]), "slate-100", 0.3),
  ];
  if (rng() < 0.5) out.push(shape(f.band([[w, -h * 0.6], [0.6, -h * 0.55], [1, -h * 0.47]], 0.3), "slate-900", 0.5));
  return out;
}

function arch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 10 * between(rng, 0.9, 1.1);
  const piers = 12;
  const t = 2.6;
  const opening = (half: number): string =>
    `M${f.p(-half, 0)}L${f.p(-half, -piers)}C${f.p(-half, -piers - half * 1.33)} ${f.p(half, -piers - half * 1.33)} ${f.p(half, -piers)}L${f.p(half, 0)}Z`;
  const crownY = -piers - (w + t);
  return [
    shape(opening(w + t) + opening(w), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-1.2, crownY - 0.3], [1.2, crownY - 0.3], [0.8, crownY + t + 0.3], [-0.8, crownY + t + 0.3]]), "slate-400", 1, INKED),
    shape(f.poly([[-w, 0], [-w, -piers], [-w + 0.8, -piers], [-w + 0.8, 0]]), "slate-900", 0.3),
  ];
}

function tombstone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 2.4, 3.2);
  const h = between(rng, 6, 8.5);
  const lean = between(rng, -0.12, 0.12);
  const P = (dx: number, dy: number): string => f.p(...leant([dx, dy], lean));
  const cross: Point[] = [
    [-0.25, -h + 1.5],
    [0.25, -h + 1.5],
    [0.25, -h + 2.4],
    [1.1, -h + 2.4],
    [1.1, -h + 2.9],
    [0.25, -h + 2.9],
    [0.25, -h + 4.6],
    [-0.25, -h + 4.6],
    [-0.25, -h + 2.9],
    [-1.1, -h + 2.9],
    [-1.1, -h + 2.4],
    [-0.25, -h + 2.4],
  ];
  return [
    shape(`M${P(-w, 0)}L${P(-w, -h + w)}Q${P(-w, -h)} ${P(0, -h)}Q${P(w, -h)} ${P(w, -h + w)}L${P(w, 0)}Z`, "stone-500", 1, MAIN_INKED),
    shape(f.poly(cross.map((point) => leant(point, lean))), "slate-800", 0.65),
    shape(`M${P(w * 0.5, 0)}L${P(w * 0.5, -h + w)}L${P(w, -h + w)}L${P(w, 0)}Z`, "slate-900", 0.3),
  ];
}

// Plants

function sprout(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.8, 0.8);
  const h = between(rng, 3.2, 4.4);
  const reach = between(rng, 2.2, 3);
  return [
    shape(f.band([[0, 0], [lean * 0.5, -h * 0.5], [lean, -h]], 0.5), "green-400", 1, MAIN),
    shape(`M${f.p(lean, -h + 0.3)}Q${f.p(lean - reach * 0.8, -h - 0.8)} ${f.p(lean - reach, -h + 0.6)}Q${f.p(lean - reach * 0.45, -h + 1.2)} ${f.p(lean, -h + 0.3)}Z`, "green-400", 1, MAIN),
    shape(`M${f.p(lean, -h + 0.1)}Q${f.p(lean + reach * 0.7, -h - 1.2)} ${f.p(lean + reach * 1.1, -h - 0.2)}Q${f.p(lean + reach * 0.5, -h + 0.8)} ${f.p(lean, -h + 0.1)}Z`, "green-500", 1),
  ];
}

function grassTuft(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const blades = whole(rng, 3, 5);
  const out: KitShape[] = [];
  for (let i = 0; i < blades; i += 1) {
    const base = (i - (blades - 1) / 2) * 0.8;
    const tip: Point = [base * 1.8 + between(rng, -1, 1), -between(rng, 3.5, 6.5)];
    out.push(shape(f.poly([[base - 0.45, 0], tip, [base + 0.45, 0]]), i % 2 === 0 ? "green-500" : "green-700", 1, MAIN));
  }
  return out;
}

function deadTree(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 12, 16);
  const out = [shape(f.poly([[-1.3, 0], [-0.5, -h], [0.5, -h], [1.3, 0]]), "wood-900", 1, MAIN)];
  const limbs = whole(rng, 3, 5);
  for (let k = 0; k < limbs; k += 1) {
    const side = k % 2 === 0 ? 1 : -1;
    const height = h * between(rng, 0.4, 0.92);
    const length = between(rng, 4, 7) * (1.1 - (height / h) * 0.5);
    const angle = between(rng, 25, 60) * DEGREES;
    const tip: Point = [side * length * Math.cos(angle), -height - length * Math.sin(angle)];
    out.push(shape(f.poly([[side * 0.1, -height + 0.6], tip, [side * 0.1, -height - 0.6]]), "wood-900", 1, MAIN));
    if (k < 2) {
      const twig: Point = [tip[0] + side * between(rng, 1, 2), tip[1] - between(rng, 1.4, 2.4)];
      const root: Point = [tip[0] * 0.7, tip[1] * 0.9 - height * 0.1];
      out.push(shape(f.poly([[root[0], root[1] + 0.25], twig, [root[0], root[1] - 0.25]]), "wood-900", 1, MAIN));
    }
  }
  return out;
}

function tree(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-1, 0], [-0.7, -7], [0.7, -7], [1, 0]]), "wood-700", 1, INKED),
    shape(f.circle(j(), -11 + j(), 5), "green-700", 1, MAIN),
    shape(f.circle(-3.4 + j(), -8.6 + j(), 3.6), "green-700", 1, MAIN),
    shape(f.circle(3.4 + j(), -8.8 + j(), 3.8), "green-700", 1, MAIN),
    shape(f.circle(-1.5, -12.2, 2.2), "green-500", 0.5),
    shape(f.circle(1.8, -8, 2.6), "green-900", 0.45),
  ];
}

function vine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const side = rng() < 0.5 ? -1 : 1;
  const height = between(rng, 13, 18);
  const stem: Point[] = [];
  for (let i = 0; i <= 6; i += 1) {
    const t = i / 6;
    stem.push([Math.sin(t * Math.PI * 1.5) * 1.6 * side, -t * height]);
  }
  const out = [shape(f.band(stem, 0.55), "green-700", 1, MAIN)];
  for (let k = 1; k <= 4; k += 1) {
    const [x, y] = stem[Math.min(6, k + 1)] ?? [0, 0];
    const dir = k % 2 === 0 ? 1 : -1;
    out.push(shape(`M${f.p(x, y)}Q${f.p(x + dir * 1.6, y - 1.8)} ${f.p(x + dir * 2.8, y - 0.6)}Q${f.p(x + dir * 1.4, y + 0.4)} ${f.p(x, y)}Z`, "green-500", 1));
  }
  return out;
}

function bush(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lobes = whole(rng, 3, 4);
  const out: KitShape[] = [];
  for (let k = 0; k < lobes; k += 1) {
    const x = -4 + (8 * k) / Math.max(1, lobes - 1);
    out.push(shape(f.circle(x, -between(rng, 2.6, 3.6), between(rng, 3, 4.5)), "green-700", 1, MAIN));
  }
  out.push(shape(f.circle(-1.6, -4.6, 2), "green-500", 0.45));
  out.push(shape(f.circle(between(rng, -3, 0), -2.2, 0.5) + f.circle(between(rng, 1.5, 3.5), -3.4, 0.5), "red-500", 0.9));
  return out;
}

function mushroom(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 2.6, 3.6);
  const r = between(rng, 2.2, 3);
  return [
    shape(f.poly([[-0.7, 0], [-0.55, -h], [0.55, -h], [0.7, 0]]), "slate-100", 1, INKED),
    shape(`M${f.p(-r, -h + 0.2)}C${f.p(-r, -h - r * 1.3)} ${f.p(r, -h - r * 1.3)} ${f.p(r, -h + 0.2)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.band([[-r * 0.85, -h + 0.25], [r * 0.85, -h + 0.25]], 0.35), "slate-400", 0.6),
    shape(f.circle(-r * 0.4, -h - r * 0.5, r * 0.16) + f.circle(r * 0.3, -h - r * 0.68, r * 0.13) + f.circle(r * 0.6, -h - r * 0.2, r * 0.1), "white", 0.85),
  ];
}

function wheat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stalks = whole(rng, 3, 4);
  const out: KitShape[] = [];
  for (let i = 0; i < stalks; i += 1) {
    const root = (i - (stalks - 1) / 2) * 0.6;
    const lean = root * 1.5 + between(rng, -0.4, 0.4);
    const h = between(rng, 7, 9);
    out.push(shape(f.band([[root, 0], [lean * 0.6, -h * 0.6], [lean, -h]], 0.3), "wood-600", 1));
    const tip: Point = [lean * 1.15, -h - 3];
    out.push(shape(`M${f.p(lean, -h)}Q${f.p(lean + 0.9, -h - 1.5)} ${f.p(...tip)}Q${f.p(lean - 0.9, -h - 1.5)} ${f.p(lean, -h)}Z`, "brass-600", 1, MAIN));
  }
  return out;
}

// Creatures

function crow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() < 0.5) {
    return [
      shape(`M${f.p(-2.4, 0.2)}Q${f.p(-1, -1.6)} ${f.p(1.4, -1.2)}Q${f.p(2.2, 0.4)} ${f.p(0.6, 1.2)}Q${f.p(-1, 1.4)} ${f.p(-2.4, 0.2)}Z`, "black", 1, MAIN),
      shape(f.circle(1.9, -1.6, 0.9), "black", 1, MAIN),
      shape(f.poly([[2.6, -1.9], [3.8, -1.45], [2.6, -1.2]]), "slate-700", 1),
      shape(f.poly([[-2.2, 0], [-4.2, 1.6], [-3.6, 2.1], [-1.8, 0.9]]), "black", 1, MAIN),
      shape(f.band([[0.2, 1.2], [0.2, 2.2]], 0.2) + f.band([[0.9, 1.1], [0.9, 2.2]], 0.2), "slate-900", 1),
      shape(f.circle(2.1, -1.8, 0.18), "white", 0.8),
    ];
  }
  const flap = between(rng, -1, 1.2);
  return [
    shape(f.ellipse(0, 0, 1.6, 0.6), "black", 1, MAIN),
    shape(f.circle(1.7, -0.2, 0.6), "black", 1, MAIN),
    shape(`M${f.p(-0.4, -0.2)}Q${f.p(-2.5, -3.8 + flap)} ${f.p(-5.6, -2.4 + flap)}Q${f.p(-3, -1.2)} ${f.p(0.4, 0.3)}Z`, "black", 1, MAIN),
    shape(`M${f.p(0.2, -0.2)}Q${f.p(1.6, -3.4 + flap)} ${f.p(4.4, -3.2 + flap)}Q${f.p(2.2, -1.2)} ${f.p(0.9, 0.2)}Z`, "black", 0.85, MAIN),
    shape(f.poly([[-1.4, 0], [-3, 0.6], [-2.8, -0.4]]), "black", 1, MAIN),
  ];
}

function bat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.8, 0.8);
  const wing = (side: number): string =>
    `M${f.p(side * 0.6, -0.6)}Q${f.p(side * 3, -2.8 + lift)} ${f.p(side * 5.5, -1.5 + lift)}Q${f.p(side * 4.6, -0.4)} ${f.p(side * 4.2, 0.4)}` +
    `Q${f.p(side * 3.4, -0.4)} ${f.p(side * 2.6, 0.6)}Q${f.p(side * 1.8, -0.2)} ${f.p(side * 0.6, 0.6)}Z`;
  return [
    shape(f.ellipse(0, 0, 0.9, 1.4), "black", 1, MAIN),
    shape(wing(-1), "black", 1, MAIN),
    shape(wing(1), "black", 1, MAIN),
    shape(f.poly([[-0.7, -1], [-0.5, -2], [-0.15, -1.2]]) + f.poly([[0.15, -1.2], [0.5, -2], [0.7, -1]]), "black", 1, MAIN),
    shape(f.circle(-0.3, -0.5, 0.14) + f.circle(0.3, -0.5, 0.14), "red-500", 0.9),
  ];
}

/** A pale hand reaching up out of the ground, its wrist at the anchor. */
function paleHand(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.25, 0.25);
  const spread = between(rng, 0.8, 1.2);
  const P = (point: Point): Point => leant(point, lean);
  const out = [
    shape(`M${f.p(-3, 0.4)}Q${f.p(0, -1.6)} ${f.p(3, 0.4)}Z`, "wood-800", 1),
    shape(f.poly([P([-1.1, 0.2]), P([-0.95, -4.6]), P([0.95, -4.6]), P([1.1, 0.2])]), "slate-100", 0.95, MAIN),
    shape(f.poly([P([-1.5, -4.4]), P([-1.7, -7.2]), P([0, -7.6]), P([1.7, -7.2]), P([1.5, -4.4])]), "slate-100", 0.95, MAIN),
  ];
  const roots = [-1.25, -0.42, 0.42, 1.25];
  const lengths = [2.4, 3, 3.1, 2.6];
  roots.forEach((root, k) => {
    const curl = between(rng, -0.5, 0.5);
    const length = lengths[k] ?? 2.6;
    out.push(shape(f.band([P([root, -7.1]), P([root * 1.15 * spread + curl * 0.3, -7.1 - length * 0.55]), P([root * 1.3 * spread + curl, -7.1 - length])], 0.62), "slate-100", 0.95, MAIN));
  });
  out.push(shape(f.band([P([-1.6, -5.4]), P([-2.6, -6.4]), P([-3.1, -7.6])], 0.66), "slate-100", 0.95, MAIN));
  return out;
}

/** A skeletal hand clawing up out of the ground, its wrist at the anchor. */
function boneHand(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.25, 0.25);
  const P = (point: Point): Point => leant(point, lean);
  const out = [
    shape(`M${f.p(-3, 0.4)}Q${f.p(0, -1.6)} ${f.p(3, 0.4)}Z`, "wood-800", 1),
    shape(f.band([P([-0.55, 0.2]), P([-0.5, -4.4])], 0.5) + f.band([P([0.6, 0.2]), P([0.5, -4.4])], 0.42), "slate-100", 1, MAIN),
    shape(f.ellipse(...P([0, -5.2]), 1.2, 0.75), "slate-100", 1, MAIN),
  ];
  const knuckles: string[] = [];
  for (const root of [-0.9, -0.3, 0.3, 0.9]) {
    const curl = between(rng, 0.3, 1.1) * (root < 0 ? 1 : -1);
    const mid: Point = [root * 1.5, -8.2];
    const tip: Point = [root * 1.3 + curl, -9.8];
    out.push(shape(f.band([P([root, -6]), P(mid), P(tip)], 0.38), "slate-100", 1, MAIN));
    knuckles.push(f.circle(...P(mid), 0.32));
  }
  out.push(shape(f.band([P([-1.1, -5]), P([-2.2, -5.9]), P([-2.4, -7.1])], 0.4), "slate-100", 1, MAIN));
  out.push(shape(knuckles.join(""), "slate-400", 0.8));
  return out;
}

function wisp(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  const drop = (k: number, lift: number): string =>
    `M${f.p(sway * k, -4.2 * k + lift)}C${f.p(2.5 * k, -1.6 * k + lift)} ${f.p(2.1 * k, 1.9 * k + lift)} ${f.p(0, 2.1 * k + lift)}` +
    `C${f.p(-2.1 * k, 1.9 * k + lift)} ${f.p(-2.5 * k, -1.6 * k + lift)} ${f.p(sway * k, -4.2 * k + lift)}Z`;
  return [shape(drop(1.6, 0), "green-200", 0.22, MAIN), shape(drop(1, 0.3), "green-400", 0.5, MAIN), shape(drop(0.5, 0.9), "white", 0.9)];
}

function worm(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.3, 0.3);
  const body: Point[] = [[0, 0.3], [0.4 + j(), -1], [-0.3 + j(), -2.2], [0.3 + j(), -3.3], [1.1, -3.9 + j()], [1.9, -3.7 + j()]];
  const head = body[5] ?? [1.9, -3.7];
  const ring = (a: Point, b: Point): string => {
    const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    const length = Math.hypot(dx, dy) || 1;
    const [nx, ny] = [(-dy / length) * 0.55, (dx / length) * 0.55];
    return f.band([[a[0] - nx, a[1] - ny], [a[0] + nx, a[1] + ny]], 0.22);
  };
  return [
    shape(f.band(body, 0.9), "red-300", 1, MAIN_INKED),
    shape(f.circle(head[0], head[1], 0.55), "red-300", 1, MAIN),
    shape(ring(body[1] ?? [0, 0], body[2] ?? [0, 0]) + ring(body[3] ?? [0, 0], body[4] ?? [0, 0]), "red-400", 0.6),
  ];
}

function moth(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 0.75, 1.05);
  const upper = (side: number): string =>
    `M${f.p(side * 0.2, -0.2)}Q${f.p(side * 3.6 * reach, -4.4)} ${f.p(side * 5 * reach, -1.4)}Q${f.p(side * 3.4 * reach, 0.8)} ${f.p(side * 0.2, 0.4)}Z`;
  const lower = (side: number): string =>
    `M${f.p(side * 0.2, 0.4)}Q${f.p(side * 3 * reach, 1.4)} ${f.p(side * 2.2 * reach, 3.2)}Q${f.p(side * 0.9, 2.6)} ${f.p(side * 0.15, 0.9)}Z`;
  return [
    shape(upper(-1), "violet-300", 1, MAIN),
    shape(upper(1), "violet-300", 1, MAIN),
    shape(lower(-1), "violet-300", 0.85, MAIN),
    shape(lower(1), "violet-300", 0.85, MAIN),
    shape(f.circle(-2.6 * reach, -1.5, 0.5) + f.circle(2.6 * reach, -1.5, 0.5), "violet-500", 0.7),
    shape(f.ellipse(0, 0.6, 0.45, 1.6), "slate-700", 1),
    shape(f.band([[-0.15, -1], [-1, -2.6]], 0.15) + f.band([[0.15, -1], [1, -2.6]], 0.15), "slate-700", 1),
  ];
}

// Objects

/** A lantern hanging from the anchor, lit. */
function lantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flicker = between(rng, -0.25, 0.25);
  return [
    shape(f.circle(0, 4.2, 3.4), "orange-400", 0.22),
    shape(f.ellipse(0, 0.9, 0.9, 0.9) + f.ellipse(0, 0.9, 0.55, 0.55), "slate-900", 1),
    shape(f.poly([[-0.8, 1.6], [0.8, 1.6], [1.5, 2.5], [-1.5, 2.5]]), "slate-700", 1, INKED),
    shape(f.poly([[-1.3, 2.5], [1.3, 2.5], [1.3, 5.9], [-1.3, 5.9]]), "orange-400", 0.95, MAIN),
    shape(`M${f.p(flicker, 3.2)}Q${f.p(0.8, 4.4)} ${f.p(0, 5.2)}Q${f.p(-0.8, 4.4)} ${f.p(flicker, 3.2)}Z`, "white", 0.9),
    shape(f.poly([[-0.6, 2.5], [-0.35, 2.5], [-0.35, 5.9], [-0.6, 5.9]]) + f.poly([[0.35, 2.5], [0.6, 2.5], [0.6, 5.9], [0.35, 5.9]]), "slate-900", 0.85),
    shape(f.poly([[-1.5, 5.9], [1.5, 5.9], [1.1, 6.7], [-1.1, 6.7]]), "slate-700", 1, INKED),
  ];
}

/** A scarecrow of bones on a crossed post, in rags and a battered hat, its post's foot at the anchor. */
function boneScarecrow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.04, 0.04);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const p = (dx: number, dy: number): string => f.p(...P(dx, dy));
  const j = (): number => between(rng, -0.6, 0.6);
  const hem: Point[] = [
    P(8, -21),
    P(6.5 + j(), -19.5),
    P(5.5, -15.5 + j()),
    P(3.6, -17.5),
    P(2, -13.5 + j()),
    P(0.4, -16.8),
    P(-1.4, -13.8 + j()),
    P(-3, -17),
    P(-4.8, -14.6 + j()),
    P(-6, -18.4),
    P(-8, -20 + j()),
  ];
  const ribs: string[] = [];
  for (let k = 0; k < 4; k += 1) {
    const y = -25 + k * 1.8;
    for (const side of [-1, 1]) ribs.push(f.band([P(side * 0.55, y), P(side * (2.6 - k * 0.2), y + 0.4), P(side * (2.9 - k * 0.3), y + 1.4)], 0.5));
  }
  const arm = (side: number): string =>
    f.band([P(side * 1.6, -25.4), P(side * 6, -25.2), P(side * 10.2, -24.8)], 0.9) +
    f.band([P(side * 10, -24), P(side * 10.8, -22.4)], 0.32) +
    f.band([P(side * 10.6, -24), P(side * 11.8, -22.8)], 0.32) +
    f.band([P(side * 11, -24.4), P(side * 12.4, -23.8)], 0.32);
  return [
    shape(f.band([P(0, 0), P(0, -33)], 1.5), "wood-700", 1, INKED),
    shape(f.band([P(-11, -25), P(11, -25.6)], 1.2), "wood-700", 1, INKED),
    shape(f.poly([P(-7, -26.2), P(7, -26.6), ...hem]), "slate-700", 0.92, MAIN_INKED),
    shape(arm(-1), "slate-100", 1),
    shape(arm(1), "slate-100", 1),
    shape(f.band([P(0, -27), P(0, -15.5)], 0.8), "slate-100", 1),
    shape(ribs.join(""), "slate-100", 1),
    shape(`M${p(-2, -15.6)}Q${p(0, -14.4)} ${p(2, -15.6)}L${p(1.6, -13.6)}Q${p(0, -12.6)} ${p(-1.6, -13.6)}Z`, "slate-100", 1),
    shape(f.band([P(-1, -13.2), P(-1.4, -8), P(-1.2, -3.6)], 0.6) + f.band([P(1, -13.2), P(1.5, -8.2), P(1.8 + j(), -4)], 0.6), "slate-100", 1),
    shape(`M${p(-2.8, -29.4)}C${p(-3.2, -34.2)} ${p(3.2, -34.2)} ${p(2.8, -29.4)}L${p(1.9, -27.4)}L${p(-1.9, -27.4)}Z`, "slate-100", 1, INKED),
    shape(f.ellipse(...P(-1.15, -30), 0.75, 0.9) + f.ellipse(...P(1.15, -30), 0.75, 0.9) + f.poly([P(0, -29), P(0.45, -28.2), P(-0.45, -28.2)]), "black", 0.9),
    shape(
      f.band([P(-4.2, -32.6), P(4.4, -32.9)], 0.8) + f.poly([P(-2.6, -33.2), P(-1.6, -36.4), P(0.6 + j(), -38.2), P(2.4, -36.2), P(2.8, -33.4)]),
      "wood-800",
      1,
      INKED,
    ),
  ];
}

function pitchfork(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.15, 0.15);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  return [
    shape(f.band([P(0, 0), P(0, -15)], 0.6), "wood-600", 1, MAIN),
    shape(f.band([P(-1.7, -15.2), P(1.7, -15.2)], 0.55), "slate-400", 1),
    shape([-1.5, 0, 1.5].map((x) => f.band([P(x, -15.5), P(x, -19.4)], 0.32)).join(""), "slate-400", 1),
  ];
}

function shovel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.12, 0.12);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const p = (dx: number, dy: number): string => f.p(...P(dx, dy));
  return [
    shape(f.band([P(0, -4.6), P(0, -15)], 0.6), "wood-600", 1, MAIN),
    shape(f.band([P(-1.2, -15.4), P(1.2, -15.4)], 0.6), "wood-700", 1),
    shape(`M${p(-1.6, -4.8)}L${p(1.6, -4.8)}L${p(1.7, -1.6)}Q${p(0, 1.2)} ${p(-1.7, -1.6)}Z`, "slate-400", 1, INKED),
  ];
}

function crate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 3.6 * between(rng, 0.9, 1.1);
  const h = w * 2 * between(rng, 0.85, 1);
  const box = (inset: number): string => f.poly([[-w + inset, -inset], [w - inset, -inset], [w - inset, -h + inset], [-w + inset, -h + inset]]);
  return [
    shape(box(0), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-w + 0.7, -h / 3], [w - 0.7, -h / 3]], 0.35) + f.band([[-w + 0.7, (-2 * h) / 3], [w - 0.7, (-2 * h) / 3]], 0.35), "wood-700", 0.85),
    shape(f.band([[-w + 0.9, -0.9], [w - 0.9, -h + 0.9]], 0.9), "wood-700", 0.9),
    shape(box(0) + box(0.7), "wood-700", 0.9),
  ];
}

function banner(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  return [
    shape(f.band([[0, 0], [0, -21]], 0.6), "wood-800", 1, INKED),
    shape(f.circle(0, -21.6, 0.7), "brass-600", 1),
    shape(f.band([[-0.2, -20], [6.4, -20]], 0.5), "wood-800", 1),
    shape(f.poly([[0.7, -19.7], [6, -19.7], [6 + sway, -10], [3.35 + sway * 0.8, -11.8], [0.7 + sway * 0.6, -10]]), "red-600", 1, MAIN_INKED),
    shape(f.band([[0.9, -18.4], [5.8, -18.4]], 0.7), "brass-600", 0.9),
  ];
}

function orb(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 5;
  const turn = between(rng, -0.3, 0.3);
  return [
    shape(f.circle(0, 0, r * 1.9), "cyan-400", 0.12),
    shape(f.circle(0, 0, r * 1.4), "sky-300", 0.2),
    shape(f.circle(0, 0, r), "blue-600", 1, MAIN),
    shape(`M${f.p(-r * 0.7, r * (0.2 + turn))}Q${f.p(0, -r * 0.9)} ${f.p(r * 0.75, -r * (0.1 - turn))}Q${f.p(0, -r * 0.4)} ${f.p(-r * 0.7, r * (0.2 + turn))}Z`, "sky-300", 0.6),
    shape(f.circle(r * 0.1, r * 0.1, r * 0.35), "white", 0.55),
    shape(f.ellipse(-r * 0.4, -r * 0.45, r * 0.28, r * 0.18), "white", 0.85),
  ];
}

/** A crystal shard standing on the anchor, tipped a little either way. */
function crystal(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -28, 28);
  const h = between(rng, 5, 8);
  const w = h * between(rng, 0.22, 0.3);
  const T = (dx: number, dy: number): Point => turned([dx, dy], tilt);
  return [
    shape(f.poly([T(-w, 0), T(-w * 1.15, -h * 0.68), T(0, -h), T(w * 1.15, -h * 0.68), T(w, 0), T(0, w * 0.5)]), "violet-300", 1, MAIN_INKED),
    shape(f.poly([T(-w, 0), T(-w * 1.15, -h * 0.68), T(0, -h), T(0, w * 0.5)]), "white", 0.28),
    shape(f.band([T(0, w * 0.3), T(0, -h * 0.95)], 0.25), "white", 0.5),
  ];
}

// Weather and light

function moon(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 6;
  const sea = (): KitShape => shape(f.ellipse(between(rng, -r * 0.5, r * 0.5), between(rng, -r * 0.5, r * 0.5), r * between(rng, 0.16, 0.3), r * between(rng, 0.12, 0.22)), "slate-400", 0.4);
  return [
    shape(f.circle(0, 0, r * 2.3), "blue-200", 0.07),
    shape(f.circle(0, 0, r * 1.5), "blue-200", 0.13),
    shape(f.circle(0, 0, r), "slate-100", 1, MAIN),
    sea(),
    sea(),
    sea(),
    shape(crescent(f, r, 0.75), "slate-400", 0.18),
  ];
}

/** Stars over a field about the anchor, 40 by 18 at scale 1. */
function stars(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 8, 12);
  const out: KitShape[] = [];
  for (let k = 0; k < count; k += 1) {
    const x = between(rng, -20, 20);
    const y = between(rng, -9, 9);
    const size = between(rng, 0.25, 0.85);
    const d = rng() < 0.35 ? sparkle(f, x, y, size * 2.2) : f.circle(x, y, size * 0.6);
    out.push(shape(d, "white", between(rng, 0.45, 0.95), MAIN));
  }
  return out;
}

function cloud(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 9, 13);
  const h = w * 0.55;
  return [
    shape(
      `M${f.p(-w, 0)}Q${f.p(-w, -h * 0.55)} ${f.p(-w * 0.62, -h * 0.62)}Q${f.p(-w * 0.5, -h * 1.18)} ${f.p(-w * 0.05, -h * 1.05)}` +
        `Q${f.p(w * 0.25, -h * 1.55)} ${f.p(w * 0.6, -h * 0.85)}Q${f.p(w * 1.02, -h * 0.8)} ${f.p(w, 0)}Z`,
      "slate-100",
      0.85,
      MAIN,
    ),
    shape(`M${f.p(-w * 0.95, 0)}Q${f.p(0, -h * 0.42)} ${f.p(w * 0.97, 0)}Z`, "slate-400", 0.35),
    shape(f.ellipse(-w * 0.15, -h * 0.85, w * 0.3, h * 0.18), "white", 0.35),
  ];
}

/** Low banks of mist, 40 or so long, drifting about the anchor. */
function mist(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const banks = whole(rng, 2, 3);
  const out: KitShape[] = [];
  for (let k = 0; k < banks; k += 1) {
    const y = k * 1.6;
    const half = between(rng, 16, 24);
    const thick = between(rng, 1.6, 2.8);
    out.push(
      shape(
        `M${f.p(-half, y)}Q${f.p(-half * 0.5, y - thick * 1.2)} ${f.p(0, y - thick * 0.4)}Q${f.p(half * 0.5, y - thick * 1.4)} ${f.p(half, y)}` +
          `Q${f.p(half * 0.5, y + thick * 0.7)} ${f.p(0, y + thick * 0.3)}Q${f.p(-half * 0.5, y + thick * 0.8)} ${f.p(-half, y)}Z`,
        "green-400",
        between(rng, 0.1, 0.18),
        MAIN,
      ),
    );
  }
  return out;
}

/** A shaft of light falling from the anchor. */
function lightShaft(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slant = between(rng, -6, 6);
  const length = 62;
  return [
    shape(f.poly([[-1.6, 0], [1.6, 0], [5.5 + slant, length], [-5.5 + slant, length]]), "white", 0.1, MAIN),
    shape(f.poly([[-0.6, 0], [0.6, 0], [2.4 + slant, length], [-2.4 + slant, length]]), "white", 0.1, MAIN),
  ];
}

function halo(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 6 * between(rng, 0.92, 1.08);
  return [shape(f.circle(0, 0, r), "orange-400", 0.12, MAIN), shape(f.circle(0, 0, r * 0.66), "orange-400", 0.16, MAIN), shape(f.circle(0, 0, r * 0.38), "orange-400", 0.24, MAIN)];
}

/** Rays fanning down from the anchor. */
function godRays(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rays = whole(rng, 5, 7);
  const fan = between(rng, 70, 100);
  const length = 75;
  const out: KitShape[] = [];
  for (let k = 0; k < rays; k += 1) {
    const angle = 90 - fan / 2 + (fan * (k + 0.5)) / rays + between(rng, -4, 4);
    const width = between(rng, 3, 7);
    const edge = (degrees: number): Point => [length * Math.cos(degrees * DEGREES), length * Math.sin(degrees * DEGREES)];
    out.push(shape(f.poly([[0, 0], edge(angle - width / 2), edge(angle + width / 2)]), "white", between(rng, 0.06, 0.12), MAIN));
  }
  return out;
}

/** Sparks about the anchor, within 6 at scale 1. */
function sparks(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const out: KitShape[] = [];
  for (let k = 0; k < count; k += 1) out.push(shape(sparkle(f, between(rng, -6, 6), between(rng, -6, 6), between(rng, 0.8, 2)), "white", between(rng, 0.75, 0.95), MAIN));
  return out;
}

/** Embers rising about the anchor, within 6 at scale 1. */
function embers(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const out: KitShape[] = [];
  for (let k = 0; k < count; k += 1) {
    const x = between(rng, -6, 6);
    const y = between(rng, -6, 6);
    const size = between(rng, 0.5, 1);
    out.push(shape(f.poly([[x, y - 1.1 * size], [x + 0.45 * size, y], [x, y + 0.6 * size], [x - 0.45 * size, y]]), k % 2 === 0 ? "ember-500" : "orange-400", 0.85, MAIN));
  }
  return out;
}

/** Motes of dust about the anchor, 30 by 20 at scale 1. */
function motes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 6);
  const out: KitShape[] = [];
  for (let k = 0; k < count; k += 1) out.push(shape(f.circle(between(rng, -15, 15), between(rng, -10, 10), between(rng, 0.25, 0.7)), "white", between(rng, 0.4, 0.85), MAIN));
  return out;
}

/** A void turning about the anchor: a dark core, its rim and its arms. */
function voidSwirl(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 12;
  const arms = whole(rng, 3, 4);
  const start = rng() * FULL_TURN;
  const out = [shape(f.circle(0, 0, r) + f.circle(0, 0, r * 0.9), "violet-700", 0.35)];
  for (let k = 0; k < arms; k += 1) {
    out.push(shape(spiralArm(f, start + (FULL_TURN / arms) * k, 200, r * 0.18, r * 0.95, r * 0.22), k % 2 === 0 ? "purple-400" : "violet-500", k % 2 === 0 ? 0.45 : 0.55, MAIN));
  }
  out.push(shape(f.circle(0, 0, r * 0.3), "black", 0.95));
  out.push(shape(f.circle(0, 0, r * 0.36) + f.circle(0, 0, r * 0.3), "violet-300", 0.55));
  return out;
}

/** A bolt striking down from the anchor. */
function lightning(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bolt: Point[] = [[0, 0]];
  let x = 0;
  for (let k = 1; k <= 5; k += 1) {
    x += between(rng, -3, 3);
    bolt.push([x, k * 6]);
  }
  const [bx, by] = bolt[2] ?? [0, 12];
  const branch: Point[] = [[bx, by], [bx + between(rng, 2, 4), by + 3], [bx + between(rng, 3, 6), by + 7]];
  return [shape(f.band(bolt, 2.4), "sky-300", 0.3), shape(f.band(bolt, 0.8), "white", 0.95, MAIN), shape(f.band(branch, 0.45), "white", 0.8)];
}

function nebula(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(blob(f, rng, 0, 0, 16, 10), "violet-500", 0.16, MAIN),
    shape(blob(f, rng, between(rng, 4, 8), between(rng, -5, 2), 12, 9), "purple-400", 0.13),
    shape(blob(f, rng, between(rng, -9, -5), between(rng, 1, 6), 10, 9), "cyan-400", 0.09),
    shape(blob(f, rng, between(rng, -2, 2), between(rng, -2, 2), 6, 8), "violet-300", 0.12),
  ];
}

function planet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 8;
  const tilt = between(rng, -20, -8) * at.flip;
  const cx = f.x(0);
  const cy = f.y(0);
  const ring = (rx: number, ry: number): string => {
    const ux = Math.cos(tilt * DEGREES);
    const uy = Math.sin(tilt * DEGREES);
    const a = pt(cx - f.r(rx) * ux, cy - f.r(rx) * uy);
    const b = pt(cx + f.r(rx) * ux, cy + f.r(rx) * uy);
    const arc = `${fmt(f.r(rx))} ${fmt(f.r(ry))} ${fmt(tilt)}`;
    return `M${a}A${arc} 1 0 ${b}A${arc} 1 0 ${a}Z`;
  };
  return [
    shape(f.circle(0, 0, r), "blue-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -r * 0.3, r * 0.82, r * 0.12) + f.ellipse(0, r * 0.25, r * 0.86, r * 0.1), "sky-300", 0.35),
    shape(crescent(f, r, 0.45), "slate-900", 0.45),
    shape(ring(r * 1.75, r * 0.42) + ring(r * 1.5, r * 0.3), "slate-100", 0.55),
  ];
}

/** Light along the rim of what stands at the anchor, on the side the part faces. */
function rimLight(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 14 * between(rng, 0.95, 1.05);
  return [shape(crescent(f, r, 0.45), "white", 0.06, MAIN), shape(crescent(f, r * 0.98, 0.68), "white", 0.06, MAIN), shape(crescent(f, r * 0.96, 0.86), "white", 0.07, MAIN)];
}

// Craft (Stinkyscape's pass, R3391000): ships, drones and vehicles, each facing +x, centred on its anchor unless it says otherwise

/** An engine's glow behind nozzles at x, one per height in `ys`: a translucent plume under a white-hot core. */
function craftGlow(f: Local, x: number, ys: readonly number[], rx: number, ry: number, paint: PaletteName = "cyan-400"): KitShape[] {
  return [
    shape(ys.map((y) => f.ellipse(x - rx * 0.5, y, rx, ry)).join(""), paint, 0.32),
    shape(ys.map((y) => f.ellipse(x - rx * 0.15, y, rx * 0.45, ry * 0.5)).join(""), "white", 0.85),
  ];
}

/** A small space fighter seen from above: a wedge hull, a canopy, two wings with guns and an engine's glow; 19 long (x -11 to 8), 14 wide. */
function fighter(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sweep = between(rng, -1, 1);
  const wing = (side: number): Point[] => [[1.5, side * 1.4], [-3.5 + sweep, side * 6.8], [-6 + sweep, side * 6.8], [-5, side * 1.6]];
  return [
    ...craftGlow(f, -7.4, [0], 2.2, 1.3),
    shape(f.poly(wing(-1)) + f.poly(wing(1)), "slate-700", 1, INKED),
    shape(f.poly([[8, 0], [3, -1.5], [-6, -1.9], [-7.4, -1], [-7.4, 1], [-6, 1.9], [3, 1.5]]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-5.5, 0], [1, 0]], 0.7), rng() < 0.5 ? "red-600" : "cyan-400", 0.85),
    shape(f.ellipse(2.6, 0, 1.9, 0.85), "cyan-400", 1, INKED),
    shape(f.band([[-4.6 + sweep, -6.8], [-0.8, -6.8]], 0.5) + f.band([[-4.6 + sweep, 6.8], [-0.8, 6.8]], 0.5), "slate-900", 1),
  ];
}

/** A slender needle-nosed fighter from above: small wings, a scratched hull and a long blue engine streak; 22 long (x -11 to 11) and its streak 10 to 14 more. */
function needleFighter(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const streak = between(rng, 10, 14);
  const s1 = between(rng, -6, -2);
  const s2 = between(rng, 3, 6);
  const wing = (side: number): Point[] => [[-3, side * 0.9], [-7, side * 4.2], [-8.8, side * 4.2], [-7.6, side * 1]];
  return [
    shape(f.poly([[-9, -1.2], [-9 - streak, -0.3], [-9 - streak, 0.3], [-9, 1.2]]), "sky-400", 0.4),
    shape(f.poly([[-9, -0.55], [-9 - streak * 0.7, -0.12], [-9 - streak * 0.7, 0.12], [-9, 0.55]]), "white", 0.8),
    shape(f.poly(wing(-1)) + f.poly(wing(1)), "slate-700", 1, INKED),
    shape(f.poly([[11, 0], [3, -0.9], [-8, -1.3], [-9.2, -0.8], [-9.2, 0.8], [-8, 1.3], [3, 0.9]]), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(0.4, 0, 1.9, 0.6), "cyan-400", 1, INKED),
    shape(f.band([[s1, -1], [s1 + 1.6, -0.35]], 0.2) + f.band([[s2, 0.25], [s2 + 1.4, 0.8]], 0.2), "slate-900", 0.75),
  ];
}

/** A sleek white arrowhead fighter from above, a delta with blue stripes and a nose cannon; 21 long (x -9.5 to 11.5), 14 wide. */
function arrowFighter(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const span = between(rng, 6.4, 7.4);
  return [
    ...craftGlow(f, -7, [-1, 1], 1.8, 0.8),
    shape(f.poly([[9, 0], [-6, -span], [-4.4, -1.6], [-7, -1.6], [-7, 1.6], [-4.4, 1.6], [-6, span]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[7, 0], [-3, -1.3], [-7, -1.1], [-7, 1.1], [-3, 1.3]]), "slate-400", 1, INKED),
    shape(f.band([[-4.4, -span * 0.72], [2.4, -1.5]], 0.45) + f.band([[-4.4, span * 0.72], [2.4, 1.5]], 0.45), "blue-600", 0.9),
    shape(f.ellipse(1.8, 0, 1.9, 0.7), "cyan-400", 1, INKED),
    shape(f.band([[8, 0], [11.5, 0]], 0.55), "slate-700", 1, INKED),
  ];
}

/** Paragon-X from above, a black-and-white blade of a fighter: long swept black wings edged white, black canards and a faceted white prism hull; 28 long (x -15 to 13), 23 wide. */
function bladeFighter(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sweep = between(rng, -0.8, 0.8);
  const wing = (side: number): Point[] => [[3, side * 1.6], [-9 + sweep, side * 11.5], [-11.5 + sweep, side * 11.5], [-7, side * 2]];
  const edge = (side: number): Point[] => [[3, side * 1.8], [-9.2 + sweep, side * 11.3]];
  const canard = (side: number): Point[] => [[7.5, side * 1.2], [4.4, side * 4.4], [3, side * 4.4], [3.6, side * 1.4]];
  return [
    ...craftGlow(f, -13, [-1.5, 1.5], 2.6, 1),
    shape(f.poly(wing(-1)) + f.poly(wing(1)), "black", 1, INKED),
    shape(f.band(edge(-1), 0.6) + f.band(edge(1), 0.6), "white", 1),
    shape(f.poly(canard(-1)) + f.poly(canard(1)), "black", 1, INKED),
    shape(f.poly([[-11.5, -2.4], [-13, -2.8], [-13, -0.3], [-11.5, -0.5]]) + f.poly([[-11.5, 0.5], [-13, 0.3], [-13, 2.8], [-11.5, 2.4]]), "slate-700", 1, INKED),
    shape(f.poly([[13, 0], [4, -2.2], [-9, -2.4], [-11.5, 0], [-9, 2.4], [4, 2.2]]), "white", 1, MAIN_INKED),
    shape(f.poly([[13, 0], [4, -2.2], [-9, -2.4], [-11.5, 0]]), "slate-400", 0.55),
    shape(f.band([[12, 0], [-10.5, 0]], 0.22), "slate-900", 0.7),
    shape(f.poly([[6.2, 0], [3.6, -1], [0.6, 0], [3.6, 1]]), "cyan-400", 1, INKED),
  ];
}

/** A small rounded scout, side on: a teardrop pod with a big canopy, stub fins, a stripe and an antenna; 14 long (x -8 to 6), 12 tall (y -6 to 5). */
function scout(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.4, 0.4);
  return [
    ...craftGlow(f, -6, [0], 1.8, 1.1),
    shape(f.poly([[-3, -2.2], [-6.6, -4.6 + tilt], [-7, -1.6]]) + f.poly([[-3, 2.2], [-6.6, 4.6 - tilt], [-7, 1.6]]), "slate-700", 1, INKED),
    shape(`M${f.p(6, 0)}C${f.p(6, -3.6)} ${f.p(-5, -3.6)} ${f.p(-6.2, 0)}C${f.p(-5, 3.6)} ${f.p(6, 3.6)} ${f.p(6, 0)}Z`, "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(2.4, -0.7, 2.2, 1.3), "cyan-400", 1, INKED),
    shape(f.band([[-3.6, 1.5], [3, 1.7]], 0.5), "orange-400", 0.8),
    shape(f.band([[-1, -2.4], [-2, -5]], 0.3) + f.circle(-2.15, -5.6, 0.45), "slate-900", 1),
  ];
}

/** A parked mech walker: a boxy cockpit body on two jointed legs, a gun under its nose; stands on its anchor, 18 tall (y -18 to 0), 17 wide (x -6.5 to 10.5). */
function walker(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const knee = between(rng, 4.6, 5.8);
  return [
    shape(f.band([[-3, -9], [-knee, -4.6], [-3.6, -0.6]], 1.7) + f.band([[3, -9], [knee + 0.6, -4.6], [4.2, -0.6]], 1.7), "slate-700", 1, INKED),
    shape(f.poly([[-6, 0], [-5.4, -1.2], [-1.8, -1.2], [-1.2, 0]]) + f.poly([[2, 0], [2.6, -1.2], [6.2, -1.2], [6.8, 0]]), "slate-800", 1, INKED),
    shape(f.circle(-knee, -4.6, 1.1) + f.circle(knee + 0.6, -4.6, 1.1), "slate-900", 1, INKED),
    shape(f.poly([[-6.5, -8.4], [-6.5, -14], [-3.5, -17.6], [4.5, -17.6], [7.5, -13.6], [6.5, -8.4]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[3.4, -16.4], [6.2, -13.6], [3.6, -13]]), "cyan-400", 1, INKED),
    shape(f.band([[-5.6, -11.4], [4.6, -11.4]], 0.5), "orange-400", 0.85),
    shape(f.band([[4, -9.6], [10.5, -9.6]], 1.1), "slate-700", 1, INKED),
  ];
}

/** Tylannt's getaway skiff, side on: a sleek hull, its rear hatch open and its ramp lowered at the back (a figure stands on it at (-12, 3)), its alarm light spilling out, an engine on top; 36 long (x -21 to 15), 13 tall (y -7.6 to 5.4). */
function skiff(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const strobe = rng() < 0.5 ? "red-500" : "red-600";
  return [
    ...craftGlow(f, -10.4, [-5.6], 3, 1.1),
    shape(f.poly([[-9, -7.6], [-4.6, -4.4], [-10.6, -4.4]]), "slate-700", 1, INKED),
    shape(f.poly([[15, 1], [11, -2.4], [3, -4.6], [-9, -4.6], [-11, -2.2], [-11, 2], [8, 2.6]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-1, -4.4], [-2, -6.6], [-9.6, -6.6], [-10.4, -5.6], [-9.6, -4.4]]), "slate-700", 1, INKED),
    shape(f.poly([[-10.4, -3.4], [-7.8, -3.4], [-7.8, 1.6], [-10.4, 1.6]]), "red-900", 0.95),
    shape(f.poly([[-9.8, -2.6], [-8.4, -2.6], [-8.4, -0.6], [-9.8, -0.6]]), strobe, 0.85),
    shape(f.poly([[-8.4, 1.2], [-19.4, 3.8], [-19.4, 5.4], [-8.4, 2.8]]), "slate-400", 1, INKED),
    shape(f.poly([[-8.4, 1.1], [-17, 3.2], [-15, 0], [-8.4, -1.6]]), "red-500", 0.22),
    shape(f.poly([[12.6, -0.6], [9.6, -2.6], [5.4, -3.4], [5.4, -0.6]]), "cyan-400", 1, INKED),
    shape(f.band([[-6, 0.4], [10, 0.6]], 0.6), "slate-100", 0.75),
    shape(f.circle(-12.5, 3.3, 0.35) + f.circle(-16.4, 4.3, 0.35), "orange-400", 0.95),
  ];
}

/** A small drone: a round body with one glowing eye-lens and two fins, now and then two tiny arms; 9 wide (x -4.6 to 4.3), 7 tall (y -3.6 to 3.6), 9 with its arms. */
function drone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.6, 0.6);
  const out = [
    shape(f.poly([[-1.6, -2.2], [-4.6, -3.6 + lift], [-3.4, -0.6]]) + f.poly([[-1.6, 2.2], [-4.6, 3.6 - lift], [-3.4, 0.6]]), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 3), "slate-400", 1, MAIN_INKED),
    shape(f.circle(1.3, -0.2, 1.3), "cyan-400", 1, INKED),
    shape(f.circle(1.75, -0.6, 0.45), "white", 0.9),
  ];
  if (rng() < 0.5) out.push(shape(f.band([[-0.8, 2.6], [-1.4, 4.2], [-0.4, 4.9]], 0.45) + f.band([[1.4, 2.4], [2.6, 3.8], [3.8, 3.4]], 0.45), "slate-700", 1));
  return out;
}

/** A helper drone: the drone with its tiny arms always out, reaching forward (+x) to hand something on; 10 wide. */
function helperDrone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const arms = f.band([[1.6, 2.2], [3.6, 3.2], [5.4, 2.4]], 0.5) + f.band([[0.2, 2.8], [2.4, 4.4], [4.8, 4.6]], 0.5);
  return [...drone(at, rng).slice(0, 4), shape(arms, "slate-700", 1, INKED)];
}

/** A bulky, blunt drone corvette, side on, shouldered in three oversized armour plates, a sensor eye and a gun at its nose; 30 long (x -14.5 to 15.5), 14 tall (y -7 to 6.2). */
function droneCorvette(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bulge = between(rng, 0, 0.8);
  const plate = (x0: number, x1: number): Point[] => [[x1, -5.6 - bulge], [x0, -6.2 - bulge], [x0 - 0.6, 5.4 + bulge], [x1 - 0.4, 5 + bulge]];
  const rim = (x0: number, x1: number): Point[] => [[x0 + 0.4, -5.5 - bulge], [x1 - 0.4, -5.1 - bulge]];
  return [
    ...craftGlow(f, -12, [-2, 2], 2.4, 1.2),
    shape(f.poly([[12.5, -1], [11.5, -4.2], [7, -5.2], [-10, -5], [-12, -2], [-12, 3], [-9.5, 4.8], [10.5, 4]]), "slate-700", 1, INKED),
    shape(f.band([[11.6, 2.2], [15.5, 2.2]], 0.9), "slate-900", 1, INKED),
    shape(f.poly(plate(4.4, 9)) + f.poly(plate(-1.6, 3.6)) + f.poly(plate(-7.6, -2.4)), "slate-400", 1, MAIN_INKED),
    shape(f.band(rim(4.4, 9), 0.45) + f.band(rim(-1.6, 3.6), 0.45) + f.band(rim(-7.6, -2.4), 0.45), "slate-100", 0.5),
    shape(f.circle(11, -1, 1.3), "cyan-400", 1, INKED),
  ];
}

/** The vast Drone Battleship, side on: its keel with glowing core vents, its hangar deck with lit bay doors and its command superstructure with spires and a dome; 68 long (x -36 to 32), 30 tall (y -17 to 12). */
function droneBattleship(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const heat = between(rng, 0.8, 0.95);
  const vents: string[] = [];
  const bays: string[] = [];
  const doors: string[] = [];
  const windows: string[] = [];
  for (let k = 0; k < 7; k += 1) vents.push(f.poly([[-22 + k * 7, 8.4], [-18.4 + k * 7, 8.4], [-18.4 + k * 7, 9.6], [-22 + k * 7, 9.6]]));
  for (let k = 0; k < 6; k += 1) {
    const x = -20 + k * 8;
    bays.push(f.poly([[x - 0.8, -3.4], [x + 5.8, -3.4], [x + 5.8, 3], [x - 0.8, 3]]));
    doors.push(f.poly([[x, -2.6], [x + 5, -2.6], [x + 5, 2.2], [x, 2.2]]));
  }
  for (let k = 0; k < 5; k += 1) windows.push(f.poly([[-9 + k * 3.6, -8.4], [-7 + k * 3.6, -8.4], [-7 + k * 3.6, -7.4], [-9 + k * 3.6, -7.4]]));
  const dome = `M${f.p(-1, -11)}A${fmt(f.r(3))} ${fmt(f.r(3))} 0 0 ${f.sweep(1)} ${f.p(5, -11)}Z`;
  return [
    ...craftGlow(f, -32, [-0.5, 8], 4, 2.2),
    shape(f.poly([[24, 4.4], [30, 8], [22, 12], [-26, 12], [-32, 8], [-28, 4.4]]), "slate-700", 1, INKED),
    shape(vents.join(""), "orange-400", heat),
    shape(f.poly([[28, 4.6], [32, 0], [27, -5], [-24, -5], [-30, -1], [-28, 4.6]]), "slate-400", 1, MAIN_INKED),
    shape(bays.join(""), "cyan-400", 0.28),
    shape(doors.join(""), "cyan-400", 0.85),
    shape(f.poly([[-11.4, -9.6], [-10.2, -16.6], [-9, -9.8]]) + f.poly([[-6, -10.6], [-5.2, -14.4], [-4.4, -10.8]]) + f.poly([[7.4, -10.2], [8.6, -15.4], [9.6, -10]]), "slate-700", 1, INKED),
    shape(f.poly([[14, -5], [10, -10], [-4, -11.4], [-14, -9.4], [-18, -5]]), "slate-400", 1, INKED),
    shape(dome, "sky-300", 0.85, INKED),
    shape(windows.join(""), "cyan-400", 0.9),
  ];
}

/** A tortoise-plated gunship, side on: a domed shell of plates bristling with gun barrels like quills over a flat belly, a cockpit head at its nose; 30 long (x -15 to 16), 22 tall (y -13 to 4.2). */
function gunship(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = between(rng, -6, 6);
  const quills: string[] = [];
  for (let k = 0; k < 9; k += 1) {
    const a = (-162 + k * 18 + turn) * DEGREES;
    quills.push(f.band([[9.5 * Math.cos(a), 1 + 7 * Math.sin(a)], [15 * Math.cos(a), 1 + 13 * Math.sin(a)]], 0.75));
  }
  const dome = `M${f.p(-12, 1)}C${f.p(-12, -11)} ${f.p(12, -11)} ${f.p(12, 1)}Z`;
  const seams =
    f.band([[-10.6, -1.4], [-6, -4.8], [0, -6], [6, -4.8], [10.6, -1.4]], 0.4) +
    f.band([[-4.2, 0.6], [-4, -4.4]], 0.4) +
    f.band([[4.2, 0.6], [4, -4.4]], 0.4) +
    f.band([[-3.4, -6], [-2.8, -7.4]], 0.4) +
    f.band([[3.4, -6], [2.8, -7.4]], 0.4);
  return [
    ...craftGlow(f, -12.6, [2.6], 2, 1),
    shape(quills.join(""), "slate-900", 1, INKED),
    shape(f.poly([[13, 1], [10.5, 4.2], [-10.5, 4.2], [-13, 1]]), "slate-700", 1, INKED),
    shape(dome, "slate-400", 1, MAIN_INKED),
    shape(seams, "slate-700", 0.75),
    shape(`M${f.p(-9.6, -2.4)}Q${f.p(-8, -6.8)} ${f.p(-2.6, -7.6)}Q${f.p(-7, -5.4)} ${f.p(-9.6, -2.4)}Z`, "slate-100", 0.35),
    shape(f.ellipse(13.4, -0.6, 2.6, 2), "slate-400", 1, INKED),
    shape(f.circle(14.6, -1, 0.7), "cyan-400", 1),
  ];
}

/** A stubby little mining fighter from above in black-and-white badger stripes, its grabber claws reaching from its nose; 18 long (x -9 to 9), 10 wide. */
function minerCraft(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const grip = between(rng, -0.5, 0.5);
  return [
    ...craftGlow(f, -6.8, [-1.8, 1.8], 1.6, 0.9),
    shape(f.poly([[-1.6, -2.8], [-5.2, -5.2], [-6.6, -4.6], [-5.4, -2.4]]) + f.poly([[-1.6, 2.8], [-5.2, 5.2], [-6.6, 4.6], [-5.4, 2.4]]), "slate-700", 1, INKED),
    shape(f.band([[3.4, -1.8], [6.6, -2.7 - grip], [8.6, -1.1]], 0.8) + f.band([[3.4, 1.8], [6.6, 2.7 + grip], [8.6, 1.1]], 0.8), "slate-400", 1, INKED),
    shape(`M${f.p(5, 0)}C${f.p(5, -4.2)} ${f.p(-7, -4.4)} ${f.p(-7, 0)}C${f.p(-7, 4.4)} ${f.p(5, 4.2)} ${f.p(5, 0)}Z`, "slate-900", 1, MAIN_INKED),
    shape(f.poly([[4.9, -0.7], [-6.6, -1.1], [-6.6, 1.1], [4.9, 0.7]]), "white", 0.95),
    shape(f.ellipse(1.8, 0, 1.3, 0.8), "cyan-400", 1, INKED),
  ];
}

/** A wide rusty mining barge, side on: a row of open bays along its flank, a bridge tower aft and a lit rent slot forward; 42 long (x -22 to 19), 17 tall (y -10.4 to 6). */
function barge(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bays: string[] = [];
  const lights: string[] = [];
  for (let k = 0; k < 5; k += 1) {
    const x = -12 + k * 5.4;
    bays.push(f.poly([[x, -3.6], [x + 4, -3.6], [x + 4, 2.4], [x, 2.4]]));
    lights.push(f.poly([[x + 0.4, 0.9], [x + 3.6, 0.9], [x + 3.6, 2], [x + 0.4, 2]]));
  }
  const rust = [-16, -9.4, 2.6, 13].map((x) => {
    const at0 = x + between(rng, -1, 1);
    return f.band([[at0, -5.2], [at0 + 0.4, -1], [at0 - 0.2, 4.4]], 0.5);
  });
  return [
    ...craftGlow(f, -19, [-1, 3], 2.4, 1.3),
    shape(f.poly([[-11, -5.6], [-10, -10.4], [-3.6, -10.4], [-2.6, -5.6]]), "wood-600", 1, INKED),
    shape(f.poly([[19, 2], [16.5, -5.6], [-16, -6], [-19, -2.4], [-19, 4], [-15.5, 6], [15.5, 5.4]]), "wood-700", 1, MAIN_INKED),
    shape(rust.join(""), "wood-800", 0.6),
    shape(bays.join(""), "slate-900", 0.9),
    shape(lights.join(""), "orange-400", 0.6),
    shape(f.poly([[-9.4, -9.2], [-7.6, -9.2], [-7.6, -8.2], [-9.4, -8.2]]) + f.poly([[-6.8, -9.2], [-5, -9.2], [-5, -8.2], [-6.8, -8.2]]), "cyan-400", 0.85),
    shape(f.poly([[12.6, -3.2], [15.4, -3.2], [15.4, -2], [12.6, -2]]), "green-400", 0.9, INKED),
  ];
}

/** A slim long-winged trader, side on and banking: long swept wings, an orange-striped hull and claws below holding a crate; 25 long (x -13 to 12), 20 tall (y -10 to 10). */
function trader(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bank = between(rng, -0.8, 0.8);
  return [
    ...craftGlow(f, -10.6, [0], 2, 1),
    shape(f.poly([[1, -1.2], [-9, -9 + bank], [-12, -9 + bank], [-5, -1.4]]), "slate-700", 1, INKED),
    shape(f.poly([[2.6, 5], [8.2, 5], [8.2, 9.4], [2.6, 9.4]]), "wood-600", 1, INKED),
    shape(f.band([[3, 5.4], [7.8, 9]], 0.55), "wood-800", 0.85),
    shape(f.band([[3.6, 1.2], [3.2, 4], [3.9, 5.6]], 0.5) + f.band([[7, 1], [7.4, 4], [6.8, 5.6]], 0.5), "slate-700", 1, INKED),
    shape(f.poly([[11.5, 0], [6, -2], [-8, -2], [-10.6, -0.2], [-8, 1.6], [6, 1.6]]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-8, 0.3], [7, 0.4]], 0.6), "orange-400", 0.9),
    shape(f.poly([[0, 1.2], [-8, 9 - bank], [-11, 9 - bank], [-4.6, 1.4]]), "slate-400", 1, INKED),
    shape(f.poly([[10, -0.4], [7, -1.8], [4, -1.8], [4, -0.4]]), "cyan-400", 1, INKED),
  ];
}

/** Galaxy, a towering flagship side on: a long hull with a mine shaft bored through its belly, ore streaming up it from twin drills below, and a glass bridge on top flickering with market charts; 49 long (x -26 to 23), 29 tall (y -15 to 13.6). */
function flagship(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const chart: Point[] = [];
  for (let k = 0; k < 7; k += 1) chart.push([-4.6 + k * 1.2, -12 + between(rng, -1, 1)]);
  const windows: string[] = [];
  const ore: Point[] = [[-0.8, 10.6], [0.9, 8.2], [-0.6, 5.2], [1, 2.4], [-1, -0.4], [0.6, -2.8]];
  for (const x of [-19, -15.4, -11.8, -8.2, 5, 8.6, 12.2, 15.8]) windows.push(f.poly([[x, -3.6], [x + 1.8, -3.6], [x + 1.8, -2.6], [x, -2.6]]));
  return [
    ...craftGlow(f, -23, [-1.5, 2.5], 3, 1.6),
    shape(f.poly([[6, -6], [4, -11], [-6, -11], [-9, -6]]), "slate-700", 1, INKED),
    shape(f.poly([[-9.6, 6], [-4.4, 6], [-7, 13.6]]) + f.poly([[4.4, 6], [9.6, 6], [7, 13.6]]), "slate-700", 1, INKED),
    shape(f.band([[-8.6, 8], [-5.6, 9]], 0.35) + f.band([[-8, 10.4], [-6.2, 11.1]], 0.35) + f.band([[5.4, 8], [8.4, 9]], 0.35) + f.band([[6, 10.4], [7.8, 11.1]], 0.35), "slate-100", 0.6),
    shape(f.poly([[23, 0], [16, -6], [-14, -7], [-23, -3], [-23, 4], [-14, 7], [16, 6]]) + f.poly([[-3, -4], [3, -4], [3, 5.6], [-3, 5.6]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-3, -4], [3, -4], [3, 5.6], [-3, 5.6]]), "slate-900", 0.9),
    shape(f.poly([[-1.8, 12.4], [1.8, 12.4], [2.4, -3.4], [-2.4, -3.4]]), "orange-400", 0.45),
    shape(ore.map(([x, y]) => f.poly([[x - 0.7, y], [x, y - 0.8], [x + 0.7, y], [x, y + 0.7]])).join(""), "orange-400", 1),
    shape(windows.join(""), "cyan-400", 0.85),
    shape(f.ellipse(-1, -12, 4.8, 2.4), "sky-300", 0.8, INKED),
    shape(f.band(chart, 0.3), "green-400", 0.95),
  ];
}

/** A bare skeletal ship frame, side on: open ribs on two longerons and a keel, two empty hardpoints and dangling wires; 24 long (x -13 to 11), 15 tall (y -6 to 8.6). */
function frameShip(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ribs = [6, 2.6, -1, -4.6, -8].map((x) => f.band([[x, -3.6], [x + 0.6, 0], [x, 3.6]], 0.5)).join("");
  const w1 = between(rng, -7, -3);
  const w2 = between(rng, 2.4, 6);
  return [
    shape(f.poly([[-10.5, -2], [-13, -3], [-13, 3], [-10.5, 2]]), "slate-700", 1, INKED),
    shape(ribs, "slate-700", 1, INKED),
    shape(f.band([[11, -0.6], [5, -4], [-8, -4.2], [-10.5, -2.4]], 0.9) + f.band([[11, 0.6], [5, 4], [-8, 4.2], [-10.5, 2.4]], 0.9), "slate-400", 1, MAIN_INKED),
    shape(f.band([[10.4, 0], [-10.5, 0]], 0.6), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-1.6, -4.4], [1.6, -4.4], [1.2, -5.8], [-1.2, -5.8]]) + f.poly([[-1.6, 4.4], [1.6, 4.4], [1.2, 5.8], [-1.2, 5.8]]), "slate-700", 1, INKED),
    shape(f.circle(0, -5.1, 0.4) + f.circle(0, 5.1, 0.4), "black", 0.9),
    shape(f.band([[w1, 4.4], [w1 + 0.7, 6.2], [w1 - 0.1, 8.4]], 0.3), "red-600", 1),
    shape(f.band([[w2, 4.2], [w2 - 0.5, 5.8], [w2 + 0.3, 7]], 0.3), "blue-600", 1),
  ];
}

/** Prototype-X2, side on: a brutal hull fused from five ships, mismatched chunks joined by glowing weld seams, five odd cockpits along its spine and three engines snarling; 47 long (x -26 to 21), 21 tall (y -11 to 10). */
function fusedHull(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.6, 0.6);
  const a: Point[][] = [
    [[21, 0], [14, -5 + j()], [8, -6], [8, 6], [15, 4.6]],
    [[-1, -7.4], [-8, -7 + j()], [-8, 7.6], [-1, 8]],
    [[-15, -6], [-21, -4.6], [-22, 5], [-15, 6.6]],
  ];
  const b: Point[][] = [
    [[9.4, -7.2], [-2.4, -9 + j()], [-2.4, 8.8], [9.4, 6.6]],
    [[-6.6, -8.4], [-16.4, -7.4 + j()], [-16.4, 7.6], [-6.6, 8.6]],
  ];
  const seams: Point[][] = [
    [[9.4, -7], [10.1, -3], [9, 1], [9.8, 6.4]],
    [[-2.4, -8.8], [-1.7, -4], [-2.8, 0.6], [-2, 8.6]],
    [[-6.6, -8.2], [-7.3, -3.4], [-6.2, 2], [-7, 8.4]],
    [[-16.4, -7.2], [-15.7, -2], [-16.8, 3], [-16, 7.4]],
  ];
  const nozzle = (y: number): string => f.poly([[-21.6, y - 1], [-24, y - 1.4], [-24, y + 1.4], [-21.6, y + 1]]);
  const barbs = f.poly([[4, 7.6], [5.6, 10], [6.4, 7.2]]) + f.poly([[-11, 8], [-10, 10.2], [-9, 8.2]]) + f.poly([[0.6, -8.6], [-0.4, -11], [-1.4, -8.8]]);
  return [
    ...craftGlow(f, -24, [-3, 1, 5], 2.6, 1.2, "orange-400"),
    shape(nozzle(-3) + nozzle(1) + nozzle(5), "slate-900", 1, INKED),
    shape(a.map((chunk) => f.poly(chunk)).join(""), "slate-700", 1, MAIN_INKED),
    shape(b.map((chunk) => f.poly(chunk)).join(""), "slate-400", 1, INKED),
    shape(barbs, "slate-900", 1, INKED),
    shape(seams.map((seam) => f.band(seam, 2.2)).join(""), "orange-400", 0.4),
    shape(seams.map((seam) => f.band(seam, 0.6)).join(""), "ember-500", 0.95),
    shape(f.ellipse(14, -4.6, 2.2, 1.2) + f.poly([[-6.2, -7.4], [-3.4, -7.4], [-3.4, -9], [-6.2, -9]]) + f.poly([[-18.6, -4.4], [-17.4, -5.6], [-18.6, -6.8], [-19.8, -5.6]]), "cyan-400", 1, INKED),
    shape(f.poly([[2.2, -8.2], [6.2, -7.6], [3.6, -10.4]]) + `M${f.p(-13.4, -7.8)}A${fmt(f.r(1.8))} ${fmt(f.r(1.8))} 0 0 ${f.sweep(1)} ${f.p(-9.8, -7.8)}Z`, "sky-300", 1, INKED),
  ];
}

/** A broken, burning wreck: a ship snapped in two, its halves drifting apart round a fire, bent plates flying and scorch marks; 22 long (x -11 to 11), 14 tall (y -8 to 5). */
function wreck(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gap = between(rng, 2.4, 3.4);
  const tilt = between(rng, 12, 20);
  const j = (): number => between(rng, -0.4, 0.4);
  const rear: Point[] = [[-gap - 0.4, -2.8], [-gap + 0.6 + j(), -1.4], [-gap - 0.6, 0], [-gap + 0.4 + j(), 1.4], [-gap - 0.2, 2.8], [-8, 3], [-10.5, 1.4], [-10.5, -1.6], [-8, -3]];
  const tipped = rear.map(([x, y]): Point => {
    const [dx, dy] = turned([x + gap, y], tilt);
    return [dx - gap, dy];
  });
  const mid = -gap / 2;
  const tongue = (x0: number, h: number, w: number): string => `M${f.p(x0 - w, -1)}Q${f.p(x0 - w * 1.3, -1 - h * 0.55)} ${f.p(x0 + 0.3, -1 - h)}Q${f.p(x0 + w * 0.2, -1 - h * 0.4)} ${f.p(x0 + w, -1)}Z`;
  const heights = [between(rng, 4.4, 6), between(rng, 2.6, 3.6), between(rng, 3.4, 5)];
  const xs = [mid - 1.9, mid, mid + 1.9];
  return [
    shape(f.ellipse(mid, -2.6, 3.4, 4.2), "ember-500", 0.28),
    shape(f.poly([[11, 0.4], [6, -2.6], [1, -3.2], [0.4 + j(), -1.6], [1.4, -0.4], [0.2, 1], [1.2, 2.6], [7, 2.4]]) + f.poly(tipped), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[8, -0.6], [5.4, -2], [3.6, -2], [3.6, -0.6]]), "slate-900", 0.9),
    shape(f.band([[2, -2], [4.6, 1.6]], 0.6) + f.band([[-gap - 1.6, -2], [-gap - 4, 1.8]], 0.7), "black", 0.45),
    shape(f.poly([[mid + 2.8, -6.4], [mid + 4.6, -7.4], [mid + 4.2, -5.8]]) + f.poly([[mid - 3, 3.6], [mid - 1.6, 4.8], [mid - 3.4, 5]]), "slate-400", 1, INKED),
    shape(xs.map((x, k) => tongue(x, heights[k] ?? 4, 0.85)).join(""), "ember-500", 0.9),
    shape(xs.map((x, k) => tongue(x, (heights[k] ?? 4) * 0.5, 0.4)).join(""), "orange-400", 0.95),
  ];
}

/** A ghostly ship, pale and see-through, with a sail fin and faint lit ports, trailing wisps; 22 long with its wisps (x -14 to 8), 11 tall (y -6 to 4.5). */
function ghostShip(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -1, 1);
  return [
    shape(f.ellipse(-1, 0, 11, 6), "slate-100", 0.07),
    shape(f.band([[-6, -1.4], [-10, -2 + sway], [-14, -1 + sway]], 0.9) + f.band([[-6, 1.4], [-10, 2.4 - sway], [-13, 1.8 - sway]], 0.7), "slate-100", 0.2),
    shape(f.poly([[8, 0], [3, -2.4], [-4, -3], [-7, -1.4], [-7, 1.4], [-4, 3], [3, 2.4]]), "slate-100", 0.38, MAIN),
    shape(f.poly([[-1.6, -2.9], [-5.2, -5.8 + sway * 0.3], [-6.2, -2.2]]), "slate-100", 0.3, MAIN),
    shape(f.band([[-5, 0.6], [6, 0.6]], 0.3), "cyan-400", 0.35),
    shape(f.circle(3, -0.7, 0.45) + f.circle(0.6, -0.9, 0.45) + f.circle(-1.8, -1, 0.45), "cyan-400", 0.75),
  ];
}

/** A classic rocket standing upright on its anchor, nose up: a tall white body, a red nose cone, three fins and a porthole; 26 tall (y -26 to 0), 14 wide at its fins. */
function rocket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const fin = between(rng, 6.4, 7.4);
  return [
    shape(f.poly([[-3, -7], [-fin, -0.4], [-6, 0.6], [-3, -1.6]]) + f.poly([[3, -7], [fin, -0.4], [6, 0.6], [3, -1.6]]), "red-600", 1, INKED),
    shape(f.poly([[-2, -1.4], [2, -1.4], [2.6, 0.6], [-2.6, 0.6]]), "slate-700", 1, INKED),
    shape(`M${f.p(-3, -1.4)}L${f.p(-3.4, -14)}Q${f.p(-3, -22)} ${f.p(0, -26)}Q${f.p(3, -22)} ${f.p(3.4, -14)}L${f.p(3, -1.4)}Z`, "slate-100", 1, MAIN_INKED),
    shape(`M${f.p(1.2, -1.6)}L${f.p(1.6, -14)}Q${f.p(1.6, -20)} ${f.p(0.6, -24)}Q${f.p(3, -21)} ${f.p(3.4, -14)}L${f.p(3, -1.6)}Z`, "slate-400", 0.45),
    shape(`M${f.p(-2.4, -20)}Q${f.p(-1.6, -24)} ${f.p(0, -26)}Q${f.p(1.6, -24)} ${f.p(2.4, -20)}Z`, "red-600", 1, INKED),
    shape(f.band([[0, -7.4], [0, -1.4]], 1), "red-600", 1, INKED),
    shape(f.circle(0, -13.6, 1.5), "cyan-400", 1, INKED),
  ];
}

/** A station tram car, side on: a rounded body, a row of windows with passengers' silhouettes, two doors, a stripe and a lit nose, on its rail; 48 long (x -23 to 25.6), 14 tall (y -6 to 6.6). */
function tram(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glass: string[] = [];
  const people: string[] = [];
  const doors: string[] = [];
  for (let k = 0; k < 7; k += 1) {
    const x = -20 + k * 6;
    if (k === 2 || k === 5) {
      doors.push(f.poly([[x, -4.6], [x + 4.4, -4.6], [x + 4.4, 4.4], [x, 4.4]]) + f.poly([[x + 0.6, -4], [x + 3.8, -4], [x + 3.8, 4.4], [x + 0.6, 4.4]]));
      glass.push(f.poly([[x + 0.9, -3.6], [x + 3.5, -3.6], [x + 3.5, -0.4], [x + 0.9, -0.4]]));
      continue;
    }
    glass.push(f.poly([[x, -4], [x + 4.4, -4], [x + 4.4, 0], [x, 0]]));
    if (k === 0 || rng() < 0.75) {
      const cx = x + 2.2 + between(rng, -0.6, 0.6);
      people.push(f.circle(cx, -1.95, 0.75) + f.poly([[cx - 1.4, 0], [cx - 1.1, -1.05], [cx + 1.1, -1.05], [cx + 1.4, 0]]));
    }
  }
  const body = `M${f.p(-21, -6)}L${f.p(19, -6)}Q${f.p(23, -6)} ${f.p(23, -1)}L${f.p(23, 3.6)}Q${f.p(23, 5)} ${f.p(21.6, 5)}L${f.p(-21.6, 5)}Q${f.p(-23, 5)} ${f.p(-23, 3.6)}L${f.p(-23, -3)}Q${f.p(-23, -6)} ${f.p(-21, -6)}Z`;
  return [
    shape(f.band([[-20, 5.8], [20, 5.8]], 1.6), "slate-700", 1, INKED),
    shape(body, "slate-400", 1, MAIN_INKED),
    shape(glass.join(""), "sky-300", 0.8),
    shape(people.join(""), "slate-900", 0.85),
    shape(f.band([[-23, 1.8], [23, 1.8]], 0.8), "cyan-400", 0.85),
    shape(doors.join(""), "slate-700", 1, INKED),
    shape(f.circle(23.6, 0.8, 2.2), "cyan-400", 0.25),
    shape(f.circle(22.2, 0.8, 0.6), "white", 0.95),
  ];
}

/** Halo's ring of guns, turning about its anchor: a heavy ring with eight gun housings and barrels pointing outward, some flaring, lights between; 45 across (radius 22.3 to its muzzles' glow). */
function gunRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = rng() * 45;
  const ring = 13.5;
  const barrels: string[] = [];
  const housings: string[] = [];
  const flares: string[] = [];
  const lights: string[] = [];
  for (let k = 0; k < 8; k += 1) {
    const angle = turn + k * 45;
    const T = (point: Point): Point => turned(point, angle);
    housings.push(f.poly([T([ring - 0.6, -1.4]), T([ring + 3, -1.4]), T([ring + 3, 1.4]), T([ring - 0.6, 1.4])]));
    barrels.push(f.band([T([ring + 3, 0]), T([ring + 7, 0])], 0.9));
    if (k % 2 === 0 || rng() < 0.3) flares.push(f.circle(...T([ring + 7.8, 0]), 1.2));
    lights.push(f.circle(...turned([ring, 0], angle + 22.5), 0.5));
  }
  return [
    shape(f.circle(0, 0, ring + 1.6) + f.circle(0, 0, ring - 1.6), "slate-400", 1, MAIN_INKED),
    shape(f.circle(0, 0, ring + 0.3) + f.circle(0, 0, ring - 0.3), "slate-900", 0.45),
    shape(barrels.join(""), "slate-700", 1, INKED),
    shape(housings.join(""), "slate-700", 1, INKED),
    shape(flares.join(""), "cyan-400", 0.7),
    shape(lights.join(""), "cyan-400", 0.9),
  ];
}

// Space architecture and terrain (Stinkyscape's pass, R3391000)

/** A ring about (0, cy) of the part's frame, `rx` by `ry` and `width` thick, tipped `tilt` degrees (mirrored with the part). */
function archRing(f: Local, cy: number, rx: number, ry: number, width: number, tilt: number, flip: 1 | -1): string {
  const cx = f.x(0);
  const y = f.y(cy);
  const angle = tilt * flip;
  const ux = Math.cos(angle * DEGREES);
  const uy = Math.sin(angle * DEGREES);
  const ellipse = (a: number, b: number): string => {
    const from = pt(cx - f.r(a) * ux, y - f.r(a) * uy);
    const to = pt(cx + f.r(a) * ux, y + f.r(a) * uy);
    const arc = `${fmt(f.r(a))} ${fmt(f.r(b))} ${fmt(angle)}`;
    return `M${from}A${arc} 1 0 ${to}A${arc} 1 0 ${from}Z`;
  };
  return ellipse(rx, ry) + ellipse(rx - width, Math.max(0.2, ry - width * (ry / rx)));
}

/** A point `t` of the way from `a` to `b`. */
function archLerp(a: Point, b: Point, t: number): Point {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
}

/** A space station seen from afar, centred on its anchor: a tilted ring on spokes about a hub and spine, modules and lit windows; 32 wide, 22 tall. */
function station(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rx = 16;
  const ry = 5.6;
  const windows: string[] = [];
  const lit = whole(rng, 6, 9);
  for (let k = 0; k < lit; k += 1) {
    const angle = ((k + between(rng, 0.1, 0.9)) * FULL_TURN) / lit;
    windows.push(f.circle(14.5 * Math.cos(angle * DEGREES), 4.9 * Math.sin(angle * DEGREES), 0.45));
  }
  const modules: string[] = [];
  const pods = whole(rng, 2, 3);
  for (let k = 0; k < pods; k += 1) {
    const y = -9.5 + k * 6.2 + between(rng, -0.6, 0.6);
    const w = between(rng, 2.6, 3.8);
    modules.push(f.poly([[-w, y], [w, y], [w, y + 3.4], [-w, y + 3.4]]));
  }
  return [
    shape(f.ellipse(0, 0, rx, ry) + f.ellipse(0, 0, rx - 2.6, ry - 1.4), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-13.4, 0], [-3.4, 0]], 0.7) + f.band([[3.4, 0], [13.4, 0]], 0.7) + f.band([[0, 3.4], [0, 4.3]], 0.7), "slate-700", 1),
    shape(f.poly([[-1.3, -11], [1.3, -11], [1.3, 10], [-1.3, 10]]), "slate-700", 1, INKED),
    shape(modules.join(""), "slate-400", 1, MAIN_INKED),
    shape(f.circle(0, 0, 3.6), "slate-400", 1, MAIN_INKED),
    shape(windows.join("") + f.circle(0, 0, 1.4), "cyan-400", 0.9),
    shape(f.circle(0, -11.8, 0.7), "red-500", 0.95),
    shape(f.ellipse(0, 1.2, rx - 0.6, ry * 0.55), "slate-900", 0.25),
  ];
}

/** An orbital dock's truss gantry, standing on its anchor: two towers, a top beam, berth arms and lights; 44 wide, 30 tall. */
function dockGantry(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const high = -19 + between(rng, -1.2, 1.2);
  const low = -9 + between(rng, -1, 1);
  const zig = (x0: number, x1: number): Point[] => [0, 1, 2, 3, 4, 5, 6].map((k) => [k % 2 === 0 ? x0 : x1, -k * 4.4] as const);
  const arm = (x0: number, x1: number, y: number): string => f.poly([[x0, y], [x1, y], [x1, y + 1.5], [x0, y + 1.5]]);
  return [
    shape(f.poly([[-22, 0], [-18, 0], [-18, -27], [18, -27], [18, 0], [22, 0], [22, -30.5], [-22, -30.5]]), "slate-700", 1, MAIN_INKED),
    shape(f.band(zig(-21.4, -18.6), 0.45) + f.band(zig(18.6, 21.4), 0.45) + f.band([[-17, -28.8], [17, -28.8]], 0.4), "slate-400", 0.8),
    shape(arm(-18, -7, high) + arm(7, 18, high) + arm(-18, -10, low) + arm(10, 18, low), "slate-400", 1, INKED),
    shape(f.poly([[-7, high - 1], [-5.6, high - 1], [-5.6, high + 2.5], [-7, high + 2.5]]) + f.poly([[5.6, high - 1], [7, high - 1], [7, high + 2.5], [5.6, high + 2.5]]), "slate-800", 1, INKED),
    shape(f.band([[-12, -27], [-12, -23]], 0.3) + f.band([[3, -27], [3, -24.5]], 0.3) + f.band([[13, -27], [13, -22.6]], 0.3), "slate-900", 0.7),
    shape(f.circle(-20, -32, 2.2) + f.circle(20, -32, 2.2) + f.circle(-6.3, high + 3.4, 1.6) + f.circle(6.3, high + 3.4, 1.6), "cyan-400", 0.22),
    shape(f.circle(-20, -32, 0.75) + f.circle(20, -32, 0.75) + f.circle(-6.3, high + 3.4, 0.55) + f.circle(6.3, high + 3.4, 0.55), "cyan-400", 0.95),
  ];
}

/** A hangar's back wall from the top of the box down to the floor at the placement's y: ribbed panels, a bay door about its x, a hazard stripe and lights. Full width. */
function hangarWall(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const ribs: string[] = [];
  const offset = between(rng, 0, 6);
  for (let x = offset - 2; x < FAR_EDGE; x += 14) ribs.push(rectPath(x, EDGE, 2, floor - EDGE - 4));
  const dw = 21 * at.s;
  const dh = Math.min(floor * 0.62, 34 * at.s);
  const dx = at.x - dw;
  const dy = floor - 4 - dh;
  const stripes: string[] = [];
  for (let x = EDGE; x < FAR_EDGE; x += 5) stripes.push(`M${pt(x, floor - 1.6)}L${pt(x + 2.2, floor - 1.6)}L${pt(x + 3.4, floor - 4)}L${pt(x + 1.2, floor - 4)}Z`);
  const lights: string[] = [];
  const glows: string[] = [];
  for (let x = 7 + between(rng, 0, 4); x < SCENE_BOX; x += 21) {
    lights.push(rectPath(x - 1.6, floor * 0.12, 3.2, 1));
    glows.push(ellipsePath(x, floor * 0.12 + 0.5, 4.6, 2.4));
  }
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor - EDGE), "slate-800", 1, MAIN),
    shape(ribs.join(""), "slate-700", 1),
    shape(rectPath(dx - 1.2, dy - 1.2, dw * 2 + 2.4, dh + 1.2) + rectPath(dx, dy, dw * 2, dh), "slate-400", 0.75, INKED),
    shape(rectPath(at.x - 0.3, dy, 0.6, dh) + rectPath(dx, dy + dh * 0.33, dw * 2, 0.4) + rectPath(dx, dy + dh * 0.66, dw * 2, 0.4), "slate-900", 0.55),
    shape(rectPath(EDGE, floor - 4, FAR_EDGE - EDGE, 2.4), "orange-400", 0.85),
    shape(stripes.join(""), "slate-900", 0.9),
    shape(glows.join(""), "cyan-400", 0.18),
    shape(lights.join(""), "cyan-400", 0.95),
  ];
}

/** A round vault door, centred on its anchor: its frame, its face, three nested lock rings turned by the stream, bolts and a wheel; radius 14.5. */
function vaultDoor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ring = (outer: number, inner: number): string => f.circle(0, 0, outer) + f.circle(0, 0, inner);
  const notches: string[] = [];
  for (const [r, w] of [[9.9, 1.2], [6.5, 1], [3.1, 0.9]] as const) {
    const turn = rng() * FULL_TURN;
    const T = (dx: number, dy: number): Point => turned([dx, dy], turn);
    notches.push(f.poly([T(r - 0.7, -w / 2), T(r + 0.7, -w / 2), T(r + 0.7, w / 2), T(r - 0.7, w / 2)]));
  }
  const bolts: string[] = [];
  const start = rng() * 45;
  for (let k = 0; k < 8; k += 1) {
    const angle = (start + k * 45) * DEGREES;
    bolts.push(f.circle(11.8 * Math.cos(angle), 11.8 * Math.sin(angle), 0.55));
  }
  const wheel = rng() * FULL_TURN;
  const spokes = [0, 120, 240].map((a) => f.band([turned([1.9, 0], wheel + a), turned([5.4, 0], wheel + a)], 0.6)).join("");
  return [
    shape(ring(14.6, 13), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 13), "slate-400", 1, MAIN_INKED),
    shape(ring(10.6, 9.2) + ring(7.2, 5.8) + ring(3.8, 2.4), "slate-700", 0.9),
    shape(notches.join(""), "cyan-400", 0.9),
    shape(bolts.join(""), "slate-100", 0.85),
    shape(spokes + f.circle(0, 0, 1.6), "slate-100", 1, INKED),
    shape(crescent(f, 13, 0.6), "slate-900", 0.3),
  ];
}

/** A towering computer core, standing on its anchor: a plinth, a dark tower and its lit status bands; 38 tall, 13 wide. */
function serverCore(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bands: string[] = [];
  for (let k = 0; k < 9; k += 1) {
    const y = -5.5 - k * 3.4;
    if (rng() < 0.18) continue;
    const w = between(rng, 2.6, 3.6);
    bands.push(f.poly([[-w, y], [w, y], [w, y - 1], [-w, y - 1]]));
  }
  if (bands.length === 0) bands.push(f.poly([[-3, -10], [3, -10], [3, -11], [-3, -11]]));
  return [
    shape(f.poly([[-7, -1], [7, -1], [7, -37], [-7, -37]]), "cyan-400", 0.12, MAIN),
    shape(f.poly([[-5, -1.8], [-5, -34], [-3.4, -38], [3.4, -38], [5, -34], [5, -1.8]]), "slate-800", 1, INKED),
    shape(f.poly([[2.6, -1.8], [2.6, -37.4], [3.4, -38], [5, -34], [5, -1.8]]), "slate-900", 0.45),
    shape(bands.join(""), "cyan-400", 0.92, MAIN),
    shape(f.circle(-3.6, -36, 0.4) + f.circle(between(rng, 2.2, 3.4), -3.6, 0.35), "white", 0.85),
    shape(f.poly([[-6.5, 0], [6.5, 0], [6.5, -2], [-6.5, -2]]), "slate-700", 1, INKED),
  ];
}

/** A prison cell's bars across the whole box, from its top down to the floor at the placement's y, two crossbars and their shadow. */
function cellBars(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const w = 1.5 * at.s;
  const gap = 9 * at.s;
  const offset = between(rng, 0, gap);
  const xs: number[] = [];
  for (let x = EDGE + offset; x < FAR_EDGE; x += gap) xs.push(x);
  const bars = xs.map((x) => rectPath(x, EDGE, w, floor - EDGE)).join("");
  const shine = xs.map((x) => rectPath(x + w * 0.2, EDGE, w * 0.22, floor - EDGE)).join("");
  const shadow = xs.map((x) => `M${pt(x, floor)}L${pt(x + w, floor)}L${pt(x + w + 5, floor + 9)}L${pt(x + 5 - w * 0.2, floor + 9)}Z`).join("");
  const cross = (y: number): string => rectPath(EDGE, y, FAR_EDGE - EDGE, 1.8 * at.s);
  return [
    shape(shadow, "black", 0.3),
    shape(bars, "slate-700", 1, MAIN_INKED),
    shape(shine, "slate-100", 0.3),
    shape(cross(floor * 0.22) + cross(floor * 0.72), "slate-400", 1, INKED),
  ];
}

/** A control console, standing on its anchor: a desk, its sloped panel of glowing screens and buttons; 18 wide, 10 tall. */
function consolePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.3, 0.3);
  const screen = (x0: number, x1: number): string => f.poly([[x0, -5.8], [x1, -5.8], [x1 - 0.4, -9.2 + tilt], [x0 + 0.4, -9.2 + tilt]]);
  return [
    shape(f.poly([[-9, 0], [9, 0], [9, -5], [-9, -5]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-9, -5], [9, -5], [7.5, -10], [-7.5, -10]]), "slate-800", 1, INKED),
    shape(f.ellipse(0, -9.4, 8, 3.4), "cyan-400", 0.12),
    shape(screen(-6.8, -1.2) + screen(1.2, 6.8), "cyan-400", 0.88),
    shape(f.band([[-6, -7.2], [-2.2, -7.2]], 0.3) + f.band([[2, -8], [5.4, -8]], 0.3) + f.band([[2, -6.8], [4.4, -6.8]], 0.3), "white", 0.55),
    shape(f.circle(-6.4, -2.4, 0.55) + f.circle(-4.6, -2.4, 0.55), "red-500", 0.9),
    shape(f.circle(4.6, -2.4, 0.55) + f.circle(6.4, -2.4, 0.55) + f.poly([[-1.8, -1.8], [1.8, -1.8], [1.8, -3], [-1.8, -3]]), "green-400", 0.85),
  ];
}

/** A mast standing on its anchor: a lattice tower, two dishes, crossarms and a blinking light at its top; 32 tall. */
function antennaMast(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const blink = between(rng, 0.7, 1);
  const zig: Point[] = [0, 1, 2, 3, 4, 5, 6, 7].map((k) => [(k % 2 === 0 ? -1 : 1) * (1 - k * 0.07), -2 - k * 3.6] as const);
  return [
    shape(f.poly([[-3.6, 0], [3.6, 0], [2.2, -2.2], [-2.2, -2.2]]), "slate-700", 1, INKED),
    shape(f.poly([[-1.3, -2], [-0.45, -32], [0.45, -32], [1.3, -2]]), "slate-400", 1, MAIN_INKED),
    shape(f.band(zig, 0.3) + f.band([[-4.2, -27.6], [4.2, -27.6]], 0.5) + f.band([[0.9, -20.4], [3.6, -21.6]], 0.45) + f.band([[-0.9, -12.4], [-2.8, -13.4]], 0.45), "slate-700", 0.9),
    shape(`M${f.p(1, -21)}Q${f.p(5.4, -25.4)} ${f.p(9, -20.6)}Q${f.p(4.6, -19.4)} ${f.p(1, -21)}Z`, "slate-100", 1, INKED),
    shape(`M${f.p(-1, -13)}Q${f.p(-4.6, -16.6)} ${f.p(-7.4, -12.6)}Q${f.p(-3.8, -11.6)} ${f.p(-1, -13)}Z`, "slate-100", 1, INKED),
    shape(f.circle(0, -32.8, 2.4), "red-500", 0.25 * blink),
    shape(f.circle(0, -32.8, 0.8), "red-500", blink),
  ];
}

/** A cramped sleeping pod, centred on its anchor: a rounded capsule open along its front, its dark inside, a pillow at its +x end, its hatch swung up and a reading lamp; 38 wide, 16 tall. */
function bunkPod(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 8;
  const capsule = (half: number, top: number, bottom: number, radius: number): string =>
    `M${f.p(-half, top)}L${f.p(half, top)}A${fmt(f.r(radius))} ${fmt(f.r(radius))} 0 0 ${f.sweep(1)} ${f.p(half, bottom)}L${f.p(-half, bottom)}A${fmt(f.r(radius))} ${fmt(f.r(radius))} 0 0 ${f.sweep(1)} ${f.p(-half, top)}Z`;
  const lift = between(rng, 0, 1.2);
  return [
    shape(capsule(11, -r, r, r), "slate-400", 1, MAIN_INKED),
    shape(capsule(11, -5, 5.6, 5.3), "slate-900", 0.95),
    shape(f.poly([[-14.5, 3.2], [14.5, 3.2], [14.6, 5.4], [-14.6, 5.4]]), "slate-700", 1),
    shape(f.ellipse(12, 1.8, 2.6, 1.4), "slate-100", 0.85),
    shape(`M${f.p(-11, -8.4)}Q${f.p(0, -18 - lift)} ${f.p(11, -8.4)}L${f.p(10, -8.2)}Q${f.p(0, -15.6 - lift)} ${f.p(-10, -8.2)}Z`, "slate-400", 1, INKED),
    shape(`M${f.p(-6, -10.6)}Q${f.p(0, -14.4 - lift)} ${f.p(6, -10.6)}Q${f.p(0, -13.2 - lift)} ${f.p(-6, -10.6)}Z`, "cyan-400", 0.45),
    shape(f.circle(-12, -2.8, 3), "cyan-400", 0.16),
    shape(f.circle(-12, -2.8, 0.7) + f.circle(-6, 7, 0.4) + f.circle(0, 7.2, 0.4) + f.circle(6, 7, 0.4), "cyan-400", 0.9),
  ];
}

/** A bridge chair standing on its anchor: a base, a stem, its seat's top at y -13 (where a seated figure's hip sits), its backrest rising to -41 on the -x side, an armrest and a light strip; 18 wide. */
function pilotSeat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const recline = between(rng, 0, 1.6);
  return [
    shape(f.poly([[-6.5, 0], [6.5, 0], [4.4, -2.2], [-4.4, -2.2]]), "slate-800", 1, INKED),
    shape(f.band([[0, -2], [0, -10]], 2), "slate-700", 1),
    shape(f.poly([[-10.2, -14], [-5, -13.4], [-6 - recline, -38], [-8 - recline, -40.6], [-11 - recline, -39], [-11, -14.8]]), "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(-8, -10)}L${f.p(7, -10)}Q${f.p(8.4, -12.6)} ${f.p(6.4, -13.4)}L${f.p(-7, -13.4)}Q${f.p(-8.6, -12)} ${f.p(-8, -10)}Z`, "slate-700", 1, MAIN_INKED),
    shape(f.band([[-6.5 - recline, -39.4], [-9.5 - recline, -37.4]], 1.6), "slate-800", 1, INKED),
    shape(f.band([[-4, -19.2], [6, -19.2]], 1.3) + f.band([[5, -18.4], [5, -13.6]], 0.8), "slate-400", 1, INKED),
    shape(f.band([[-6.2, -15], [-7.2 - recline, -36.6]], 0.45), "cyan-400", 0.8),
  ];
}

/** A cockpit seen from inside, framing the whole box: its canopy's frame and centre strut above, and its dashboard from the placement's y down, with gauges, screens and their glow. Full width. */
function cockpitFrame(at: PartPlace, rng: Rng): KitShape[] {
  const dash = at.y;
  const sag = between(rng, 2, 4);
  const window = `M${pt(12, 7)}L${pt(88, 7)}L${pt(97, dash + 1)}L${pt(3, dash + 1)}Z`;
  const dials = [16, 30, 70, 84];
  const g = 1.8 * at.s;
  const gauges = dials.map((x) => circlePath(x + between(rng, -1.5, 1.5), dash + 6 + g, g));
  const screens = [rectPath(40, dash + 3.4, 8.6, 5.4), rectPath(51.4, dash + 3.4, 8.6, 5.4)];
  return [
    shape(`M${pt(EDGE, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, dash + 4)}L${pt(EDGE, dash + 4)}Z${window}`, "slate-900", 1, MAIN_INKED),
    shape(`M${pt(49.2, 7)}L${pt(50.8, 7)}L${pt(51.4, dash + 1)}L${pt(48.6, dash + 1)}Z`, "slate-900", 1, INKED),
    shape(`M${pt(18, 9)}L${pt(22, 9)}L${pt(12, dash - 2)}L${pt(8, dash - 2)}Z` + `M${pt(70, 9)}L${pt(72, 9)}L${pt(66, dash - 6)}L${pt(64, dash - 6)}Z`, "white", 0.08),
    shape(`M${pt(EDGE, dash + 4)}Q${pt(50, dash - sag)} ${pt(FAR_EDGE, dash + 4)}L${pt(FAR_EDGE, FAR_EDGE)}L${pt(EDGE, FAR_EDGE)}Z`, "slate-800", 1, INKED),
    shape(`M${pt(20, dash + 2)}Q${pt(50, dash - sag - 6)} ${pt(80, dash + 2)}Q${pt(50, dash - sag + 1)} ${pt(20, dash + 2)}Z`, "cyan-400", 0.14),
    shape(gauges.join(""), "cyan-400", 0.85),
    shape(screens.join(""), "green-400", 0.8),
    shape(dials.map((x) => rectPath(x - 1.5, dash + 13, 3, 0.8)).join("") + rectPath(42, dash + 11, 16, 0.8), "red-500", 0.75),
  ];
}

/** A round porthole, centred on its anchor: its riveted rim, its glass (translucent, so what lies behind shows) and a glint; radius 10.5. */
function porthole(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = rng() * 45;
  const rivets: string[] = [];
  for (let k = 0; k < 8; k += 1) {
    const angle = (start + k * 45) * DEGREES;
    rivets.push(f.circle(9.25 * Math.cos(angle), 9.25 * Math.sin(angle), 0.42));
  }
  return [
    shape(f.circle(0, 0, 8), "sky-300", 0.1),
    shape(f.circle(0, 0, 10.5) + f.circle(0, 0, 8), "slate-400", 1, MAIN_INKED),
    shape(f.circle(0, 0, 8) + f.circle(0, 0, 7.3), "slate-800", 0.6),
    shape(rivets.join(""), "slate-700", 0.9),
    shape(`M${f.p(-5.6, -1.6)}Q${f.p(-5, -5.6)} ${f.p(-1.2, -6.4)}Q${f.p(-4, -4.6)} ${f.p(-5.6, -1.6)}Z`, "white", 0.4),
    shape(f.circle(3.8, 4, 0.7), "white", 0.25),
  ];
}

/** A glass display case on a plinth, standing on its anchor: a translucent case, its frame, lid and highlight; 12 wide, 19.5 tall. */
function glassCase(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sheen = between(rng, -0.6, 0.6);
  const box = (inset: number): string => f.poly([[-5 + inset, -3 - inset], [5 - inset, -3 - inset], [5 - inset, -18 + inset], [-5 + inset, -18 + inset]]);
  return [
    shape(f.poly([[-6, 0], [6, 0], [6, -3], [-6, -3]]), "slate-700", 1, MAIN_INKED),
    shape(box(0), "cyan-400", 0.1),
    shape(box(0) + box(0.5), "slate-400", 0.9, INKED),
    shape(f.poly([[-3.8 + sheen, -4], [-2.8 + sheen, -4], [-1.6 + sheen, -17], [-2.6 + sheen, -17]]), "white", 0.32),
    shape(f.poly([[-5.6, -18], [5.6, -18], [5.6, -19.4], [-5.6, -19.4]]), "slate-700", 1, INKED),
    shape(f.poly([[-4, -1.2], [4, -1.2], [4, -1.8], [-4, -1.8]]), "cyan-400", 0.8),
  ];
}

/** A floating bazaar stall standing on its anchor: a counter, posts, a scalloped awning, its stripes, two hanging lanterns and the glow it floats on; 24 wide, 18 tall. */
function marketStall(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.5, 0.5);
  const scallops: string[] = [`M${f.p(-12, -18)}L${f.p(12, -18)}L${f.p(12, -15)}`];
  for (let k = 0; k < 6; k += 1) {
    const x1 = 12 - 4 * k;
    scallops.push(`Q${f.p(x1 - 2, -12.8)} ${f.p(x1 - 4, -15)}`);
  }
  const stripes = [-8, 0, 8].map((x) => f.poly([[x - 2, -18], [x + 2, -18], [x + 2, -15], [x - 2, -15]])).join("");
  return [
    shape(f.ellipse(0, 1.6, 11, 1.8), "cyan-400", 0.22),
    shape(f.poly([[-9.8, -8.4], [-8.8, -8.4], [-8.8, -15], [-9.8, -15]]) + f.poly([[8.8, -8.4], [9.8, -8.4], [9.8, -15], [8.8, -15]]), "slate-700", 1),
    shape(f.poly([[-10, 0], [10, 0], [10, -7], [-10, -7]]), "wood-700", 1, INKED),
    shape(f.poly([[-11, -7], [11, -7], [11, -8.4], [-11, -8.4]]), "wood-600", 1, INKED),
    shape(`${scallops.join("")}Z`, "red-600", 1, MAIN_INKED),
    shape(stripes, "slate-100", 0.45),
    shape(f.circle(-7 + sway, -11.2, 2.2) + f.circle(7 + sway, -11.2, 2.2), "orange-400", 0.25),
    shape(f.circle(-7 + sway, -11.2, 0.9) + f.circle(7 + sway, -11.2, 0.9) + f.poly([[-6, -4.8], [-2, -4.8], [-2, -2.2], [-6, -2.2]]), "orange-400", 0.95),
  ];
}

/** A warp disruptor beacon, centred on its anchor: a spiked pylon wound with two or three tilted glowing rings and its core; 20 tall, 24 wide. */
function disruptorBeacon(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rings = whole(rng, 2, 3);
  const out = [
    shape(f.ellipse(0, 0, 13, 10), "purple-400", 0.14, MAIN),
    shape(f.poly([[0, -10], [2, -3], [1.4, 6], [0, 10], [-1.4, 6], [-2, -3]]), "slate-700", 1, INKED),
    shape(f.poly([[2, -1.2], [6.4, 0], [2, 1.2]]) + f.poly([[-2, -1.2], [-6.4, 0], [-2, 1.2]]) + f.poly([[-0.5, -10.2], [0, -13], [0.5, -10.2]]), "slate-400", 1, INKED),
  ];
  for (let k = 0; k < rings; k += 1) {
    const tilt = (k % 2 === 0 ? -1 : 1) * between(rng, 6, 14);
    out.push(shape(archRing(f, -4.4 + k * 4.4, 9.6 - k * 1.4, 3.2 - k * 0.3, 0.8, tilt, at.flip), "purple-400", 0.85 - k * 0.1, MAIN));
  }
  out.push(shape(f.circle(0, 0, 3.4), "purple-400", 0.35));
  out.push(shape(f.circle(0, 0, 1.3) + f.circle(0, -13.6, 0.5), "white", 0.9));
  return out;
}

/** An assembly-line conveyor centred on its anchor: its belt's top in perspective, its front, belt slats, rollers, legs and a hazard edge; 60 long, 15 tall. */
function conveyor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const shift = between(rng, 0, 4);
  const slats: string[] = [];
  for (let x = -26 + shift; x < 26; x += 5) slats.push(f.poly([[x, -2.1], [x + 0.5, -2.1], [x * 0.9 + 0.5, -6.6], [x * 0.9, -6.6]]));
  const rollers: string[] = [];
  for (let x = -26.25; x <= 27; x += 7.5) rollers.push(f.circle(x, -0.2, 1.2));
  const leg = (x: number): string => f.poly([[x - 0.9, 1.6], [x + 0.9, 1.6], [x + 0.9, 8], [x - 0.9, 8]]);
  return [
    shape(leg(-24) + leg(-8) + leg(8) + leg(24), "slate-900", 1),
    shape(f.poly([[-30, -2], [30, -2], [27, -7], [-27, -7]]), "slate-700", 1, MAIN_INKED),
    shape(slats.join(""), "slate-900", 0.5),
    shape(f.poly([[-30, -2], [30, -2], [30, 1.6], [-30, 1.6]]), "slate-800", 1, INKED),
    shape(rollers.join(""), "slate-400", 1, INKED),
    shape(f.band([[-30, -2.4], [30, -2.4]], 0.5), "orange-400", 0.8),
  ];
}

/** A reactor core, standing on its anchor: a cracked cylinder, blue energy glowing through its window and cracks, hoops, cap and base; 26 tall, 14 wide. */
function reactor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.8, 0.8);
  const crack = (side: number, y: number): Point[] => [[side * 2.4, y], [side * 3.8, y - 1.6 + j()], [side * 4.6, y + 0.6 + j()], [side * 5.8, y - 0.8]];
  return [
    shape(f.circle(0, -13, 14), "cyan-400", 0.12, MAIN),
    shape(f.circle(0, -13, 9), "cyan-400", 0.18, MAIN),
    shape(f.poly([[-7.4, 0], [7.4, 0], [6.4, -2.4], [-6.4, -2.4]]), "slate-800", 1, INKED),
    shape(`M${f.p(-6, -2)}L${f.p(-6, -23)}Q${f.p(0, -26)} ${f.p(6, -23)}L${f.p(6, -2)}Q${f.p(0, 0.6)} ${f.p(-6, -2)}Z`, "slate-700", 1, INKED),
    shape(f.poly([[-2.4, -21], [2.4, -21], [2.4, -4], [-2.4, -4]]), "cyan-400", 0.9, MAIN),
    shape(f.band(crack(1, -16), 0.5) + f.band(crack(-1, -9), 0.5), "cyan-400", 0.9, MAIN),
    shape(f.band([[0, -19.5], [0.8, -15], [-0.6, -11], [0.4, -6]], 0.5), "white", 0.85),
    shape(f.poly([[-6.3, -5.4], [-2.6, -5.4], [-2.6, -6.8], [-6.3, -6.8]]) + f.poly([[2.6, -5.4], [6.3, -5.4], [6.3, -6.8], [2.6, -6.8]]) + f.poly([[-6.3, -19], [-2.6, -19], [-2.6, -20.4], [-6.3, -20.4]]) + f.poly([[2.6, -19], [6.3, -19], [6.3, -20.4], [2.6, -20.4]]), "slate-400", 1, INKED),
    shape(f.ellipse(0, -23.4, 5.2, 1.5), "slate-400", 1, INKED),
  ];
}

/** A ship's metal deck from the placement's y down across the box: plates, seams running to the placement's x, a hazard edge and deck lights. Full width. */
function deckFloor(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const rows: string[] = [];
  for (let k = 1; k <= 4; k += 1) rows.push(rectPath(EDGE, top + depth * (k / 4.6) ** 1.6, FAR_EDGE - EDGE, 0.25 + k * 0.12));
  const seams: string[] = [];
  for (let i = 0; i < 6; i += 1) {
    const foot = -40 + (180 * (i + 0.5)) / 6;
    const apex = at.x + (foot - at.x) * 0.3;
    seams.push(`M${pt(apex - 0.12, top + 1.4)}L${pt(apex + 0.12, top + 1.4)}L${pt(foot + 0.45, FAR_EDGE)}L${pt(foot - 0.45, FAR_EDGE)}Z`);
  }
  const stripes: string[] = [];
  for (let x = EDGE; x < FAR_EDGE; x += 5) stripes.push(`M${pt(x, top + 1.4)}L${pt(x + 2, top + 1.4)}L${pt(x + 3, top)}L${pt(x + 1, top)}Z`);
  const ly = top + depth * 0.3;
  const lights: string[] = [];
  const glows: string[] = [];
  for (let x = 8 + between(rng, 0, 6); x < SCENE_BOX; x += 16) {
    lights.push(rectPath(x - 1.4, ly, 2.8, 0.7));
    glows.push(ellipsePath(x, ly + 0.35, 4.2, 1.4));
  }
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "slate-700", 1, MAIN),
    shape(rows.join(""), "slate-900", 0.45),
    shape(seams.join(""), "slate-900", 0.4),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 1.4), "orange-400", 0.85),
    shape(stripes.join(""), "slate-900", 0.85),
    shape(glows.join(""), "cyan-400", 0.2),
    shape(lights.join(""), "cyan-400", 0.9),
    shape(ellipsePath(at.x + between(rng, -6, 6), top + depth * 0.55, 30, depth * 0.18), "slate-100", 0.07),
  ];
}

/** A station corridor in perspective filling the box: its ceiling, walls and floor running to a lit doorway centred on the anchor, ribs, and emergency light strips along the floor. */
function corridor(at: PartPlace, rng: Rng): KitShape[] {
  const hw = 5.5 * at.s;
  const hh = 7.5 * at.s;
  const tl: Point = [EDGE, EDGE];
  const tr: Point = [FAR_EDGE, EDGE];
  const br: Point = [FAR_EDGE, FAR_EDGE];
  const bl: Point = [EDGE, FAR_EDGE];
  const dtl: Point = [at.x - hw, at.y - hh];
  const dtr: Point = [at.x + hw, at.y - hh];
  const dbr: Point = [at.x + hw, at.y + hh];
  const dbl: Point = [at.x - hw, at.y + hh];
  const poly = (points: readonly Point[]): string => `M${points.map(([x, y]) => pt(x, y)).join("L")}Z`;
  const frame = (t: number, w: number): string => {
    const outer = [archLerp(tl, dtl, t), archLerp(tr, dtr, t), archLerp(br, dbr, t), archLerp(bl, dbl, t)];
    const inner = [archLerp(tl, dtl, t + w), archLerp(tr, dtr, t + w), archLerp(br, dbr, t + w), archLerp(bl, dbl, t + w)];
    return poly(outer) + poly(inner);
  };
  const ribs = [0.18, 0.42, 0.64, 0.8].map((t) => frame(t + between(rng, -0.02, 0.02), 0.035)).join("");
  const strip = (a: Point, b: Point, width: number): string => bandPath([archLerp(a, b, 0.02), archLerp(a, b, 0.97)], width);
  const lowL: Point = [EDGE + 10, FAR_EDGE];
  const lowR: Point = [FAR_EDGE - 10, FAR_EDGE];
  return [
    shape(poly([tl, tr, dtr, dtl]), "slate-900", 1),
    shape(poly([tl, dtl, dbl, bl]) + poly([tr, br, dbr, dtr]), "slate-700", 1),
    shape(poly([bl, dbl, dbr, br]), "slate-800", 1),
    shape(ribs, "slate-900", 0.55),
    shape(poly([[at.x - hw * 2, at.y - hh * 1.6], [at.x + hw * 2, at.y - hh * 1.6], [at.x + hw * 2, at.y + hh * 1.4], [at.x - hw * 2, at.y + hh * 1.4]]), "sky-300", 0.14),
    shape(poly([dtl, dtr, dbr, dbl]), "white", 0.85),
    shape(strip(lowL, [at.x - hw * 0.7, at.y + hh], 3) + strip(lowR, [at.x + hw * 0.7, at.y + hh], 3), "red-500", 0.2, MAIN),
    shape(strip(lowL, [at.x - hw * 0.7, at.y + hh], 0.8) + strip(lowR, [at.x + hw * 0.7, at.y + hh], 0.8), "red-500", 0.9, MAIN),
  ];
}

/** A planet's or moon's surface from the placement's y down: the ground, its farther shade, craters with lit rims, rocks and a dusty horizon. Full width. */
function regolith(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = SCENE_BOX - top;
  const bowls: string[] = [];
  const rims: string[] = [];
  const slots = 4;
  for (let k = 0; k < slots; k += 1) {
    const x = ((k + between(rng, 0.25, 0.75)) * SCENE_BOX) / slots;
    const t = between(rng, 0.2, 0.85);
    const y = top + 3 + depth * t;
    const rx = (3 + 5 * t) * at.s;
    const ry = rx * 0.3;
    bowls.push(ellipsePath(x, y, rx, ry));
    rims.push(`M${pt(x - rx, y)}Q${pt(x, y + ry * 2.2)} ${pt(x + rx, y)}Q${pt(x, y + ry * 1.2)} ${pt(x - rx, y)}Z`);
  }
  const rocks: string[] = [];
  const glints: string[] = [];
  for (let k = 0; k < 3; k += 1) {
    const x = ((k + between(rng, 0.3, 0.7)) * SCENE_BOX) / 3 + 8;
    const y = top + depth * between(rng, 0.35, 0.95);
    const r = (1.2 + ((y - top) / Math.max(1, depth)) * 2.4) * at.s;
    rocks.push(`M${pt(x - r, y)}L${pt(x - r * 0.6, y - r * 0.9)}L${pt(x + r * 0.3, y - r)}L${pt(x + r, y - r * 0.3)}L${pt(x + r * 0.9, y)}Z`);
    glints.push(`M${pt(x - r * 0.6, y - r * 0.9)}L${pt(x + r * 0.3, y - r)}L${pt(x, y - r * 0.55)}Z`);
  }
  return [
    shape(groundBand(rng, top, 1.2 * at.s, 5), "stone-500", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.5, 1.5 * at.s, 4), "slate-900", 0.18),
    shape(bowls.join(""), "slate-900", 0.38),
    shape(rims.join(""), "white", 0.2),
    shape(rocks.join(""), "slate-700", 1, INKED),
    shape(glints.join(""), "slate-400", 0.6),
    shape(rectPath(EDGE, top - 1.6, FAR_EDGE - EDGE, 2.2), "white", 0.08),
  ];
}

/** A river of lava from the horizon at `xTop` to the box's foot at `xBottom`, widening as it nears, `half` its half-width at the foot, `phase` where it winds. */
function archRiver(top: number, xTop: number, xBottom: number, half: number, phase: number): string {
  const steps = 6;
  const left: string[] = [];
  const right: string[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const y = top + (FAR_EDGE - top) * t;
    const x = xTop + (xBottom - xTop) * t + Math.sin(t * Math.PI * 2 + phase) * 3.2 * t;
    const w = 0.15 + half * t;
    left.push(pt(x - w, y));
    right.push(pt(x + w, y));
  }
  return `M${left.join("L")}L${right.reverse().join("L")}Z`;
}

/** Dark volcanic ground from the placement's y down, its glowing lava rivers running to the horizon at the placement's x, hot cracks and a haze. Full width. */
function lavaFlats(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = SCENE_BOX - top;
  const feet = [between(rng, 6, 20), between(rng, 42, 58), between(rng, 80, 94)].map((foot) => [foot, between(rng, 0, Math.PI * 2)] as const);
  const rivers = (half: number): string => feet.map(([foot, phase]) => archRiver(top + 0.8, at.x + (foot - at.x) * 0.12, foot, half, phase)).join("");
  const cracks: string[] = [];
  for (let k = 0; k < 4; k += 1) {
    const x = ((k + between(rng, 0.2, 0.8)) * SCENE_BOX) / 4;
    const y = top + depth * between(rng, 0.3, 0.9);
    cracks.push(ellipsePath(x, y, 1.6 * at.s, 0.4 * at.s));
  }
  return [
    shape(groundBand(rng, top, 1 * at.s, 5), "slate-900", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.45, 1.4 * at.s, 4), "slate-800", 0.45),
    shape(rectPath(EDGE, top - 2.4, FAR_EDGE - EDGE, 3.2), "orange-400", 0.14),
    shape(rivers(5.2 * at.s), "orange-400", 0.22),
    shape(rivers(2.6 * at.s), "ember-500", 0.95),
    shape(rivers(0.9 * at.s), "orange-400", 0.9),
    shape(cracks.join(""), "ember-500", 0.8),
  ];
}

/** A floating platform disc centred on its anchor: the glow beneath it, its rim with lights, its deck and an inner ring; 32 wide, 9 tall with its glow. */
function hoverPlatform(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rx = 16;
  const ry = 4.5;
  const side = `M${f.p(-rx, 0)}L${f.p(-rx, 3)}A${fmt(f.r(rx))} ${fmt(f.r(ry))} 0 0 ${f.sweep(0)} ${f.p(rx, 3)}L${f.p(rx, 0)}Z`;
  const lights: string[] = [];
  const count = whole(rng, 5, 7);
  for (let k = 0; k < count; k += 1) {
    const angle = (180 * (k + 0.5)) / count;
    lights.push(f.circle(-rx * Math.cos(angle * DEGREES), 1.6 + ry * Math.sin(angle * DEGREES), 0.42));
  }
  return [
    shape(f.ellipse(0, 6, 12, 3.6), "cyan-400", 0.16),
    shape(f.ellipse(0, 5, 7, 2), "cyan-400", 0.28),
    shape(side, "slate-700", 1, INKED),
    shape(f.ellipse(0, 0, rx, ry), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(0, 0, 12, 3.3) + f.ellipse(0, 0, 11, 2.9), "slate-700", 0.5),
    shape(lights.join(""), "cyan-400", 0.9),
    shape(f.ellipse(-4, -1.4, 6, 1.2), "white", 0.12),
  ];
}

// Space objects, creatures and plants (Stinkyscape's pass, R3391000)

/** A hexagon about a point in the part's frame, `turn` degrees round. */
function objHex(f: Local, cx: number, cy: number, r: number, turn: number): string {
  const points: Point[] = [];
  for (let i = 0; i < 6; i += 1) {
    const angle = (turn + i * 60) * DEGREES;
    points.push([cx + r * Math.cos(angle), cy + r * Math.sin(angle)]);
  }
  return f.poly(points);
}

/** `count` of the cells of a `cols` by `rows` grid `step` apart about the anchor, jittered by up to `jitter`, none twice. */
function objCells(rng: Rng, count: number, cols: number, rows: number, step: number, jitter: number): Point[] {
  const cells: Point[] = [];
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) cells.push([(col - (cols - 1) / 2) * step, (row - (rows - 1) / 2) * step]);
  }
  const out: Point[] = [];
  while (out.length < count && cells.length > 0) {
    const [cell] = cells.splice(Math.floor(rng() * cells.length), 1);
    if (cell !== undefined) out.push([cell[0] + between(rng, -jitter, jitter), cell[1] + between(rng, -jitter, jitter)]);
  }
  return out;
}

/** A chunky retro ray gun pointing +x, centred, 14 long and 9 tall, its emitter glowing (MAIN, sky-300). */
function rayGun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 0, 0.8);
  const muzzle = 5.8 + reach;
  return [
    shape(f.poly([[-4.6, 0.8], [-2.4, 0.8], [-1.6, 5], [-4.2, 5.2]]), "slate-700", 1, INKED),
    shape(`M${f.p(-6.4, -3)}L${f.p(0.6, -3.4)}Q${f.p(3, -3.6)} ${f.p(3, -1.2)}L${f.p(3, 1.2)}L${f.p(-6.4, 1.6)}Q${f.p(-7.6, -0.7)} ${f.p(-6.4, -3)}Z`, "slate-400", 1, INKED),
    shape(f.band([[3.8, -2.6], [3.8, 1.6]], 0.8) + f.poly([[3, -1], [muzzle, -1.9], [muzzle, 1.1], [3, 0.2]]), "slate-700", 1, INKED),
    shape(f.circle(-3.6, -1, 0.8) + f.band([[-1.6, -2.2], [1.4, -2.4]], 0.5), "red-500", 0.9),
    shape(f.circle(muzzle + 1, -0.4, 3.4), "sky-300", 0.25, MAIN),
    shape(f.circle(muzzle + 1, -0.4, 1.4), "sky-300", 1, MAIN_INKED),
    shape(f.circle(muzzle + 1.1, -0.5, 0.6), "white", 0.9),
  ];
}

/** A drone core: an armoured hexagonal casing round a glowing heart (MAIN, cyan-400), centred, radius 5 (its glow 8). */
function droneCore(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = between(rng, 0, 60);
  const plates = [0, 2, 4].map((i) => {
    const angle = (turn + 30 + i * 60) * DEGREES;
    return f.band([[2.7 * Math.cos(angle), 2.7 * Math.sin(angle)], [4.4 * Math.cos(angle), 4.4 * Math.sin(angle)]], 0.4);
  });
  return [
    shape(f.circle(0, 0, 6.8), "cyan-400", 0.16, MAIN),
    shape(objHex(f, 0, 0, 5, turn) + f.circle(0, 0, 2.6), "slate-400", 1, INKED),
    shape(plates.join(""), "slate-700", 0.7),
    shape(f.circle(0, 0, 2.6), "cyan-400", 0.95, MAIN),
    shape(f.circle(-0.6, -0.6, 1), "white", 0.85),
  ];
}

/** A scatter of four to seven glittering hexagonal credit chips about the anchor, within 8 (MAIN, jade-400). */
function creditChips(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cells = objCells(rng, whole(rng, 4, 7), 3, 3, 5, 1.1);
  const turns = cells.map(() => between(rng, 0, 60));
  const [gx, gy] = cells[0] ?? [0, 0];
  return [
    shape(cells.map(([x, y], k) => objHex(f, x, y, 1.3, turns[k] ?? 0)).join(""), "jade-400", 1, MAIN_INKED),
    shape(cells.map(([x, y], k) => objHex(f, x, y, 0.6, turns[k] ?? 0)).join(""), "green-200", 0.6),
    shape(sparkle(f, gx + 0.6, gy - 0.8, 2), "white", 0.9),
  ];
}

/** A bulging duffel bag with its strap up, centred, 13 wide and 11 tall (MAIN, slate-800). */
function duffel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bulge = between(rng, 4.2, 5.2);
  return [
    shape(f.band([[-3.2, -2.6], [-1.2, -6.4], [1.6, -6.6], [3.4, -2.6]], 0.8), "slate-900", 1),
    shape(`M${f.p(-5.6, -2.4)}Q${f.p(0, -bulge)} ${f.p(5.6, -2.4)}Q${f.p(6.6, 0.4)} ${f.p(5.6, 3)}L${f.p(-5.6, 3)}Q${f.p(-6.6, 0.4)} ${f.p(-5.6, -2.4)}Z`, "slate-800", 1, MAIN_INKED),
    shape(f.ellipse(-5.6, 0.3, 1, 2.6) + f.ellipse(5.6, 0.3, 1, 2.6), "slate-900", 1, INKED),
    shape(f.band([[-4.2, -2.4], [0, -bulge * 0.62], [4.2, -2.4]], 0.35), "slate-400", 0.85),
    shape(f.ellipse(-1.2, -1, 3, 0.9), "white", 0.15),
  ];
}

/** A square herb-flecked cracker, centred, 6 square, tipped a little either way (MAIN, brass-600). */
function cracker(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -12, 12);
  const T = (dx: number, dy: number): Point => turned([dx, dy], tilt);
  const holes: string[] = [];
  for (const x of [-1.6, 0, 1.6]) for (const y of [-1.6, 0, 1.6]) holes.push(f.circle(...T(x, y), 0.28));
  const flecks = objCells(rng, 5, 3, 2, 1.8, 0.3).map(([x, y]) => {
    const angle = between(rng, 0, 180);
    const [ax, ay] = turned([0.45, 0], angle);
    return f.band([T(x + 0.8 - ax, y + 0.8 - ay), T(x + 0.8 + ax, y + 0.8 + ay)], 0.3);
  });
  return [
    shape(f.poly([T(-3, -3), T(3, -3), T(3, 3), T(-3, 3)]), "brass-600", 1, MAIN_INKED),
    shape(f.poly([T(-3, -3), T(3, -3), T(2.4, -2.4), T(-2.4, -2.4)]), "white", 0.25),
    shape(holes.join(""), "wood-700", 0.8),
    shape(flecks.join(""), "green-700", 0.95),
  ];
}

/** A zero-g coffee bulb drifting, centred, 6 wide and 9 tall: a round pouch, its sipping straw and a curl of steam (MAIN, the pouch, wood-600). */
function coffeeBulb(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -16, 16);
  const T = (dx: number, dy: number): Point => turned([dx, dy], tilt);
  const curl = between(rng, -0.6, 0.6);
  return [
    shape(f.band([T(0.4, -2.2), T(0.9, -5), T(2.2, -6)], 0.5), "slate-100", 1, INKED),
    shape(f.circle(...T(0, 0), 3), "wood-600", 1, MAIN_INKED),
    shape(f.band([T(-2.6, 0.2), T(2.6, 0.2)], 1.2), "slate-100", 0.85),
    shape(f.circle(...T(-1, -1), 0.9), "white", 0.35),
    shape(f.band([[2.6, -6.6], [3.4 + curl, -7.8], [2.8, -9], [3.6 - curl, -10.2]], 0.35), "white", 0.4),
  ];
}

/** A small war drum standing on its anchor, 8 wide and 7 tall, a stick resting on it (MAIN, the shell, red-600). */
function warDrum(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stick = between(rng, -0.6, 0.6);
  const zig: Point[] = [];
  for (let k = 0; k <= 6; k += 1) zig.push([-3.4 + k * 1.15, k % 2 === 0 ? -5 : -0.9]);
  return [
    shape(f.poly([[-4, -5.6], [4, -5.6], [3.4, 0], [-3.4, 0]]), "red-600", 1, MAIN_INKED),
    shape(f.band(zig, 0.3), "slate-100", 0.8),
    shape(f.band([[-3.5, -0.5], [3.5, -0.5]], 0.8) + f.band([[-4.1, -5], [4.1, -5]], 0.6), "wood-700", 1),
    shape(f.ellipse(0, -5.6, 4, 1.2), "slate-100", 1, INKED),
    shape(f.band([[-1.6, -6.4 + stick], [3.6, -8.6 + stick]], 0.5), "wood-600", 1, INKED),
  ];
}

/** A perfect block of blue-white ice, a translucent cube in three-quarter view, centred, 26 wide and 22 tall (MAIN, the front, blue-200). */
function iceBlock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const crackX = between(rng, -9, 1);
  const crack: Point[] = [[crackX, -7], [crackX + 1.6, -3], [crackX - 0.4, 0], [crackX + 2, 4]];
  const crack2: Point[] = [[7, between(rng, -4, 2)], [9, -5], [10.4, -8]];
  return [
    shape(f.poly([[-14, -8], [-7.4, -12.2], [14, -12.2], [14, 7.4], [7.8, 12], [-14, 12]]), "blue-200", 0.1),
    shape(f.poly([[-13, -7], [7, -7], [7, 11], [-13, 11]]), "blue-200", 0.36, MAIN_INKED),
    shape(f.poly([[-13, -7], [7, -7], [13, -11], [-7, -11]]), "white", 0.42, INKED),
    shape(f.poly([[7, -7], [13, -11], [13, 7], [7, 11]]), "sky-400", 0.35, INKED),
    shape(f.band([[-11, 8], [-4, -5]], 0.9) + f.band([[-8.4, 9.4], [-5, 3]], 0.5), "white", 0.5),
    shape(f.band(crack, 0.3) + f.band(crack2, 0.3), "white", 0.75),
  ];
}

/** A boot-lock clamp, its lock glowing and its hazard light lit, centred, 10 wide and 10 tall, its chain running 14 toward +x (MAIN, the clamp, orange-400). */
function bootClamp(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 0.6, 1.8);
  const links: string[] = [];
  for (let k = 0; k < 5; k += 1) {
    const x = 6.6 + k * 2.6;
    const y = 2 + sag * Math.sin((k / 4) * Math.PI);
    const ry = k % 2 === 0 ? 0.7 : 0.3;
    links.push(f.ellipse(x, y, 1.2, ry) + f.ellipse(x, y, 0.66, ry * 0.4));
  }
  const arch = (outer: number, inner: number): string =>
    `M${f.p(-outer, 4)}L${f.p(-outer, -1)}Q${f.p(-outer, -outer)} ${f.p(0, -outer)}Q${f.p(outer, -outer)} ${f.p(outer, -1)}L${f.p(outer, 4)}` +
    `L${f.p(inner, 4)}L${f.p(inner, -0.4)}Q${f.p(inner, -inner)} ${f.p(0, -inner)}Q${f.p(-inner, -inner)} ${f.p(-inner, -0.4)}L${f.p(-inner, 4)}Z`;
  return [
    shape(f.circle(0, -5.8, 2.8), "orange-400", 0.3),
    shape(arch(5, 2.4), "orange-400", 1, MAIN_INKED),
    shape(f.band([[-5, 2.6], [-2.4, 2.6]], 0.8) + f.band([[2.4, 2.6], [5, 2.6]], 0.8), "slate-900", 0.6),
    shape(f.circle(-3.7, 0.4, 1.9), "cyan-400", 0.4),
    shape(f.circle(-3.7, 0.4, 1.1) + f.circle(-3.7, 0.4, 0.4), "slate-400", 1, INKED),
    shape(f.circle(0, -5.6, 0.9), "ember-500", 1, INKED),
    shape(links.join(""), "slate-400", 1, INKED),
  ];
}

/** A battered pilot's chrome mask split open down the middle, red light in the crack, centred, 14 wide and 14 tall (MAIN, the chrome, slate-100). */
function crackedMask(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gap = between(rng, 0.6, 1.4);
  const L = (dx: number, dy: number): Point => turned([dx - gap, dy], -5);
  const R = (dx: number, dy: number): Point => turned([dx + gap, dy], 5);
  const left = `M${f.p(...L(-0.6, -7))}Q${f.p(...L(-6, -7))} ${f.p(...L(-6, -1))}Q${f.p(...L(-6, 5))} ${f.p(...L(-1, 7))}L${f.p(...L(-0.6, 3.4))}L${f.p(...L(-1.6, 1))}L${f.p(...L(-0.3, -2))}L${f.p(...L(-1.2, -4.6))}Z`;
  const right = `M${f.p(...R(0.6, -7))}Q${f.p(...R(6, -7))} ${f.p(...R(6, -1))}Q${f.p(...R(6, 5))} ${f.p(...R(1, 7))}L${f.p(...R(1.2, 3.4))}L${f.p(...R(0.2, 1))}L${f.p(...R(1.4, -2))}L${f.p(...R(0.4, -4.6))}Z`;
  return [
    shape(f.band([[0, -7.6], [0, 7.6]], 3.4), "red-500", 0.35),
    shape(f.band([[0, -6.8], [0.4, -2], [-0.3, 1.6], [0.2, 6.8]], 1), "ember-500", 0.9),
    shape(left, "slate-100", 1, MAIN_INKED),
    shape(right, "slate-100", 1, MAIN_INKED),
    shape(f.band([L(-5.2, -3.6), L(-5.4, 0.4), L(-3.8, 4.8)], 1.3) + f.band([R(5.2, -3.6), R(5.4, 0.4), R(3.8, 4.8)], 1.3), "slate-400", 0.7),
    shape(f.poly([L(-4.6, -2.4), L(-1.6, -2.2), L(-1.8, -0.9), L(-4.2, -1.2)]) + f.poly([R(1.6, -2.2), R(4.6, -2.4), R(4.2, -1.2), R(1.8, -0.9)]), "black", 0.85),
    shape(f.band([L(-3.4, 2.4), L(-2.2, 3.6)], 0.3) + f.band([R(3, 1.6), R(4.2, 2.8)], 0.3), "slate-700", 0.7),
  ];
}

/** The Rod of Ages: an ancient rod standing vertical about its anchor, 64 tall and 5 wide, twelve carved bands round it, two to five lit pale silver from the bottom (MAIN, its stone, stone-500). */
function ancientRod(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lit = whole(rng, 2, 5);
  const band = (y: number): string => f.poly([[-3.1, y - 0.6], [3.1, y - 0.6], [3.1, y + 0.6], [-3.1, y + 0.6]]);
  const ys = Array.from({ length: 12 }, (_, k) => 27 - k * 4.9);
  const crackX = between(rng, -1.4, 1);
  return [
    shape(f.poly([[-2.3, 30], [-2.7, -27], [-1.3, -31.6], [1.3, -31.6], [2.7, -27], [2.3, 30], [0, 33.6]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[0.6, 30], [0.8, -28], [1.3, -31.6], [2.7, -27], [2.3, 30], [0, 33.6]]), "slate-900", 0.35),
    shape(f.band([[crackX, -20], [crackX + 0.8, -15], [crackX - 0.2, -10]], 0.3) + f.band([[crackX + 0.6, 6], [crackX - 0.4, 11], [crackX + 0.4, 15]], 0.3), "slate-700", 0.6),
    shape(ys.slice(lit).map(band).join(""), "slate-700", 1, INKED),
    shape(ys.slice(0, lit).map((y) => f.ellipse(0, y, 6.4, 1.9)).join(""), "white", 0.14),
    shape(ys.slice(0, lit).map((y) => f.ellipse(0, y, 4.4, 1.2)).join(""), "slate-100", 0.3),
    shape(ys.slice(0, lit).map(band).join(""), "slate-100", 1, INKED),
    shape(f.circle(0, -34.6, 4.2), "slate-100", 0.16),
    shape(f.poly([[0, -38], [1.8, -34.6], [0, -31.4], [-1.8, -34.6]]), "slate-400", 1, INKED),
  ];
}

/** A scrap-metal anvil standing on its anchor, 13 wide and 8 tall, a red-hot axe head lying on it (MAIN, the anvil's top, slate-400). */
function anvil(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slide = between(rng, -1, 1);
  return [
    shape(f.circle(2.4 + slide, -8.6, 3.4), "orange-400", 0.28),
    shape(f.poly([[-4, 0], [4, 0], [3, -1.6], [1.6, -2.4], [1.6, -4.6], [-1.6, -4.6], [-1.6, -2.4], [-3, -1.6]]), "slate-700", 1, INKED),
    shape(`M${f.p(-6.6, -7)}Q${f.p(-4.4, -7.6)} ${f.p(-3, -7.8)}L${f.p(5.6, -7.8)}L${f.p(5.6, -5.6)}L${f.p(4, -4.6)}L${f.p(-1.8, -4.6)}Q${f.p(-3.6, -5.6)} ${f.p(-6.6, -7)}Z`, "slate-400", 1, MAIN_INKED),
    shape(f.poly([[0.4, -7.4], [3.6, -7.4], [3.6, -5.4], [0.4, -5.4]]) + f.circle(-2.6, -1.2, 0.35) + f.circle(2.6, -1.2, 0.35), "slate-700", 0.55),
    shape(`M${f.p(slide - 0.6, -7.8)}L${f.p(slide - 0.6, -9.6)}L${f.p(slide + 1.4, -9.4)}Q${f.p(slide + 3, -10.6)} ${f.p(slide + 4.8, -11)}Q${f.p(slide + 4, -9.4)} ${f.p(slide + 4.8, -7.8)}Z`, "ember-500", 1, INKED),
    shape(f.band([[slide + 4.5, -10.6], [slide + 4, -9.4], [slide + 4.4, -8.2]], 0.4), "white", 0.85),
  ];
}

/** A cracking glass containment sphere on a buoy base, violet light leaking from its cracks, centred, radius 12, its base to 15 below (MAIN, the leaking light, violet-500). */
function containmentSphere(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -60, -20);
  const b = between(rng, 110, 160);
  const crackFrom = (angle: number, spin: number): Point[] => [turned([12, 0], angle), turned([8.4, 1.4 * spin], angle), turned([6.6, -spin], angle), turned([4, 0.6 * spin], angle)];
  const [lx, ly] = turned([9, 0], a);
  const [mx, my] = turned([9, 0], b);
  return [
    shape(f.circle(0, 0, 15.5), "violet-500", 0.15, MAIN),
    shape(f.poly([[-5, 10], [5, 10], [8, 15], [-8, 15]]) + f.poly([[-1, 15], [1, 15], [1, 17], [-1, 17]]), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 12), "blue-200", 0.16, INKED),
    shape(f.band([[-12, 0], [-6, 2.6], [0, 3.2], [6, 2.6], [12, 0]], 1), "slate-400", 1, INKED),
    shape(f.band(crackFrom(a, 1), 0.35) + f.band(crackFrom(b, -1), 0.35), "white", 0.8),
    shape(f.circle(lx, ly, 2.4) + f.circle(mx, my, 2), "purple-400", 0.45, MAIN),
    shape(f.ellipse(-5, -6.4, 3, 1.5), "white", 0.35),
    shape(f.circle(5.6, 12.6, 0.8), "red-500", 0.9),
  ];
}

/** A merchant's balance scale standing on its anchor, tipped to one side, 16 wide and 14 tall (MAIN, its pans, brass-600). */
function scalePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 8, 14);
  const end = (side: number): Point => {
    const [x, y] = turned([side * 7, 0], tilt);
    return [x, y - 12];
  };
  const [lx, ly] = end(-1);
  const [rx, ry] = end(1);
  const strings = (x: number, y: number): string => f.band([[x, y], [x - 2.2, y + 5]], 0.22) + f.band([[x, y], [x + 2.2, y + 5]], 0.22);
  const pan = (x: number, y: number): string => `M${f.p(x - 2.8, y + 5)}Q${f.p(x, y + 7.6)} ${f.p(x + 2.8, y + 5)}Z`;
  return [
    shape(f.poly([[-3, 0], [3, 0], [2, -1.2], [-2, -1.2]]), "slate-700", 1, INKED),
    shape(f.band([[0, -1.2], [0, -12]], 0.7), "slate-400", 1, INKED),
    shape(f.band([[lx, ly], [rx, ry]], 0.6), "brass-600", 1, INKED),
    shape(strings(lx, ly) + strings(rx, ry), "slate-400", 0.85),
    shape(pan(lx, ly) + pan(rx, ry), "brass-600", 1, MAIN_INKED),
    shape(objHex(f, rx - 0.8, ry + 4.4, 0.8, 0) + objHex(f, rx + 0.9, ry + 4.3, 0.8, 30), "jade-400", 1),
    shape(f.circle(0, -12.6, 0.8), "brass-600", 1, INKED),
  ];
}

/**
 * One endless silver wire, bent first like a paperclip at the anchor and then into steps that grow as they climb
 * up and to +x, 40 long (to about 30 right and 26 up) (MAIN, the wire, slate-100).
 */
function wireCoil(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const arc = (cx: number, cy: number, r: number, from: number, to: number): Point[] =>
    [0, 1, 2, 3, 4].map((k) => {
      const angle = (from + ((to - from) * k) / 4) * DEGREES;
      return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)] as Point;
    });
  // The paperclip: in from its free end, round its inner loop, back along, round its left end and out along the top,
  // round its right end, and out of the clip into the steps.
  const wire: Point[] = [
    [1.2, -3.2],
    ...arc(5.6, -2.15, 1.05, -90, 90),
    ...arc(-1.4, -2.75, 1.65, 90, 270),
    ...arc(7, -2.2, 2.2, -90, 90),
  ];
  let [x, y] = [7, 0];
  let run = 3.4;
  let rise = 2.4;
  while (x < 26) {
    x += run;
    wire.push([x - 0.4, y], [x, y - 0.4]);
    y -= rise;
    wire.push([x, y + 0.4], [x + 0.4, y]);
    run *= between(rng, 1.12, 1.22);
    rise *= between(rng, 1.12, 1.22);
  }
  wire.push([x + run * 0.6, y]);
  const steps = wire.slice(15);
  const tail = wire[wire.length - 1] ?? [x, y];
  return [
    shape(f.band(steps, 2.6), "slate-100", 0.12),
    shape(f.band(wire.map(([sx, sy]) => [sx + 0.3, sy + 0.4] as Point), 0.9), "slate-700", 0.45),
    shape(f.band(wire, 0.8), "slate-100", 1, MAIN),
    shape(f.band(wire.map(([sx, sy]) => [sx - 0.15, sy - 0.18] as Point), 0.25), "white", 0.8),
    shape(sparkle(f, tail[0] + 0.6, tail[1] - 0.6, 2.4) + sparkle(f, 9.2, -2.2, 1.4), "white", 0.9),
  ];
}

/** A student's lectern standing on its anchor, 9 wide and 14 tall, a glowing slate or an open book on it (MAIN, its stand, slate-700). */
function lectern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const book = rng() < 0.5;
  const top = [
    shape(f.poly([[-3.5, 0], [3.5, 0], [2.4, -1.2], [-2.4, -1.2]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-1, -1.2], [1, -1.2], [1.2, -10], [-1.2, -10]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-4.4, -10.2], [4.4, -12.4], [4.8, -11.2], [-4, -9]]), "slate-400", 1, INKED),
    shape(f.ellipse(0, -12.4, 4.6, 2), "cyan-400", book ? 0.12 : 0.2),
  ];
  return book
    ? [
        ...top,
        shape(f.poly([[-3.6, -10.8], [-0.1, -11.4], [-0.1, -13.8], [-3.6, -13]]) + f.poly([[0.1, -11.5], [3.6, -12.4], [3.6, -14.6], [0.1, -13.9]]), "white", 1, INKED),
        shape(f.band([[-3, -12], [-0.8, -12.4]], 0.2) + f.band([[0.8, -12.7], [3, -13.2]], 0.2), "slate-400", 0.7),
      ]
    : [...top, shape(f.poly([[-3.4, -10.9], [3.4, -12.6], [3.4, -13.6], [-3.4, -11.9]]), "cyan-400", 0.95, INKED), shape(f.band([[-2.6, -11.6], [1.6, -12.6]], 0.2), "white", 0.7)];
}

/** An industrial robot arm, its base standing on the anchor, reaching up toward +x, 22 long, a weld sparking at its tip (MAIN, the arm, orange-400). */
function robotArm(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -2, 2);
  const shoulder: Point = [0, -3.4];
  const elbow: Point = [4 + bend * 0.4, -13 - bend];
  const wrist: Point = [14, -15 + bend * 0.5];
  const tip: Point = [16.6, -11.4 + bend * 0.5];
  return [
    shape(f.poly([[-3.6, 0], [3.6, 0], [2.6, -2.4], [-2.6, -2.4]]), "slate-700", 1, INKED),
    shape(f.band([[-1.6, -3.4], [elbow[0] - 1.4, elbow[1] + 1.2]], 0.5), "slate-400", 0.9),
    shape(f.band([shoulder, elbow], 2.4), "orange-400", 1, MAIN_INKED),
    shape(f.band([elbow, wrist], 1.9), "orange-400", 1, MAIN_INKED),
    shape(f.circle(...shoulder, 1.7) + f.circle(...elbow, 1.5) + f.circle(...wrist, 1.2), "slate-900", 1, INKED),
    shape(f.poly([[wrist[0] - 0.6, wrist[1] + 0.6], [wrist[0] + 0.9, wrist[1] - 0.2], [tip[0] + 0.3, tip[1]], [tip[0] - 0.4, tip[1] + 0.3]]), "slate-400", 1, INKED),
    shape(f.circle(...tip, 2.6), "cyan-400", 0.3),
    shape(sparkle(f, tip[0] + 0.4, tip[1] + 0.4, 2.2), "white", 0.95),
  ];
}

/** Debris drifting about the anchor, within 8: bent hull plates, a strut, a bolt, a shard (MAIN, the plates, slate-400). */
function debris(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cells = objCells(rng, whole(rng, 3, 5), 3, 2, 6, 1.2);
  return cells.map(([x, y], k) => {
    const spin = between(rng, 0, 180);
    const T = (dx: number, dy: number): Point => {
      const [tx, ty] = turned([dx, dy], spin);
      return [x + tx, y + ty];
    };
    const kind = k === 0 ? 0 : whole(rng, 0, 3);
    if (kind === 0) return shape(f.poly([T(-2.6, -1.2), T(0.4, -1.6), T(2.4, -0.4), T(2, 1.2), T(-0.2, 0.6), T(-2.4, 1)]), "slate-400", 1, MAIN_INKED);
    if (kind === 1) return shape(f.band([T(-2.6, 0), T(2.6, 0.3)], 0.6), "slate-700", 1, INKED);
    if (kind === 2) return shape(objHex(f, x, y, 0.8, spin) + f.circle(x, y, 0.3), "slate-400", 1, INKED);
    return shape(f.poly([T(-1.4, 1), T(1.6, 0.6), T(0, -1.8)]), "slate-700", 1, INKED);
  });
}

/** A rope hanging from the anchor (a breached skylight), swinging a little, a loop at its end: 40 long at scale 1 (MAIN, the rope, wood-600). */
function rope(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -4, 4);
  const line: Point[] = [[0, 0], [swing * 0.3, 14], [swing * 0.7, 28], [swing, 38]];
  const [ex, ey] = line[3] ?? [0, 38];
  return [
    shape(f.band(line, 1), "wood-600", 1, MAIN_INKED),
    shape(f.band(line.map(([x, y]) => [x - 0.25, y] as const), 0.3), "brass-600", 0.6),
    shape(f.circle(ex, ey + 1.6, 1.8) + f.circle(ex, ey + 1.6, 1.1), "wood-600", 1, MAIN_INKED),
  ];
}

/** A blank playing card, tipped a little either way, centred on its anchor: 6 by 8.4 at scale 1 (MAIN, its face, slate-100). */
function blankCard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -24, 24);
  const T = (dx: number, dy: number): Point => turned([dx, dy], tilt);
  const face = (w: number, h: number): string => f.poly([T(-w, -h), T(w, -h), T(w, h), T(-w, h)]);
  return [shape(face(3, 4.2), "slate-100", 1, MAIN_INKED), shape(face(3, 4.2) + face(2.3, 3.5), "slate-400", 0.8)];
}

/** An outstretched gloved hand, its wrist at the anchor, fingers spread reaching up, 12 tall and 9 wide (MAIN, the glove, slate-700). */
function glovedHand(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spread = between(rng, 0.8, 1.2);
  const fingers = [-1.9, -0.65, 0.65, 1.9].map((root, k) => {
    const length = [4.4, 5.4, 5.6, 4.8][k] ?? 5;
    return f.band([[root, -5.4], [root * (1 + 0.35 * spread), -5.4 - length * 0.5], [root * (1 + 1.1 * spread), -5.4 - length]], 1.15);
  });
  return [
    shape(f.poly([[-2.3, 0.8], [2.3, 0.8], [2.5, -1.8], [-2.5, -1.8]]), "slate-900", 1, INKED),
    shape(f.band([[-4.8, -6.6], [-3.6, -4.8], [-2.2, -3.2]], 1.3), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-2.5, -1.8], [2.5, -1.8], [2.9, -6], [-2.7, -6.2]]), "slate-700", 1, MAIN_INKED),
    shape(fingers.join(""), "slate-700", 1, MAIN_INKED),
    shape(f.band([[-2.2, -5.2], [2.4, -5]], 0.3) + f.band([[-2.1, -0.4], [2.1, -0.4]], 0.4), "slate-900", 0.55),
  ];
}

/** A creature of starstuff, violet and specked with stars, trailing nebula wisps, its one huge eye burning, centred, 13 across (MAIN, its body, violet-700). */
function cosmality(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = rng() * FULL_TURN;
  const wisps = [0, 1, 2].map((k) => spiralArm(f, start + k * 120, 70, 5, 10, 1.6)).join("");
  const specks = objCells(rng, 5, 3, 3, 3.2, 0.5).map(([x, y]) => sparkle(f, x, y, between(rng, 0.5, 0.9))).join("");
  const look = between(rng, -0.4, 0.4);
  return [
    shape(blob(f, rng, 0, 0, 8.6, 9), "violet-500", 0.2),
    shape(wisps, "purple-400", 0.45),
    shape(blob(f, rng, 0, 0, 6, 9), "violet-700", 1, MAIN_INKED),
    shape(specks, "white", 0.85),
    shape(f.circle(look, -0.4, 4), "ember-500", 0.3),
    shape(f.circle(look, -0.4, 2.6), "orange-400", 1, INKED),
    shape(f.ellipse(look, -0.4, 0.6, 1.9), "black", 0.95),
    shape(f.circle(look - 0.9, -1.4, 0.45), "white", 0.9),
  ];
}

/** A person asleep on their back under a blanket, head on a pillow at the +x end, centred, 27 long and 8 tall (MAIN, the blanket, blue-800). */
function sleeper(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const chest = between(rng, -4.6, -3.8);
  const hand = between(rng, 0.6, 3);
  return [
    shape(f.ellipse(10.4, 0, 3.6, 1.8), "slate-100", 1, INKED),
    shape(`M${f.p(-13.2, 1.8)}L${f.p(-13.2, -1)}Q${f.p(-12.4, -3.4)} ${f.p(-9.4, -2.8)}Q${f.p(-4.4, chest - 0.8)} ${f.p(1, chest)}Q${f.p(5, chest - 0.4)} ${f.p(8.2, -2.8)}L${f.p(8.6, 1.8)}Z`, "blue-800", 1, MAIN_INKED),
    shape(f.band([[7.2, -3.2], [7.5, 1.6]], 0.9) + f.band([[-6, -2.6], [-3.4, 0.8]], 0.3), "slate-100", 0.7),
    shape(f.circle(10.2, -2.3, 2.6), "red-300", 1, INKED),
    shape(`M${f.p(9.6, -4.9)}Q${f.p(13.8, -5)} ${f.p(12.8, -1)}Q${f.p(12.4, -3.4)} ${f.p(9.6, -4.9)}Z`, "wood-800", 1),
    shape(f.circle(hand, chest + 0.1, 0.9), "red-300", 1, INKED),
    shape(f.band([[9.2, -3.2], [10.2, -3]], 0.3) + f.band([[8.4, -1.6], [9.2, -1.2]], 0.25), "slate-900", 0.85),
  ];
}

/** A silver vine curling up from its anchor, silver leaves along it and one big glowing blue berry at its tip, 17 tall (MAIN, the vine, slate-400). */
function berryVine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, 0.8, 1.6);
  const stem: Point[] = [];
  for (let i = 0; i <= 6; i += 1) {
    const t = i / 6;
    stem.push([Math.sin(t * Math.PI * 1.4) * sway * 1.6, -t * 13]);
  }
  const [ex, ey] = stem[6] ?? [0, -13];
  stem.push([ex + 0.6, ey - 1.2], [ex + 1.8, ey - 1.6], [3.4, -14.6], [3.9, -13.6]);
  const leaf = ([x, y]: Point, dir: number): string => `M${f.p(x, y)}Q${f.p(x + dir * 1.6, y - 2)} ${f.p(x + dir * 3.2, y - 0.8)}Q${f.p(x + dir * 1.6, y + 0.4)} ${f.p(x, y)}Z`;
  const [bx, by] = [3.8, -11.6];
  return [
    shape(f.band(stem, 0.7), "slate-400", 1, MAIN_INKED),
    shape(leaf(stem[1] ?? [0, 0], -1) + leaf(stem[3] ?? [0, 0], 1) + leaf(stem[5] ?? [0, 0], -1), "slate-100", 0.9, INKED),
    shape(f.band([[3.9, -13.6], [bx, by - 1.6]], 0.3), "slate-400", 1),
    shape(f.circle(bx, by, 4.6), "sky-400", 0.18),
    shape(f.circle(bx, by, 3), "sky-300", 0.3),
    shape(f.circle(bx, by, 1.9), "blue-600", 1, INKED),
    shape(f.circle(bx - 0.6, by - 0.6, 0.6), "white", 0.9),
  ];
}

/** A rosemary sprig standing on its anchor, needle leaves either side of its stem, 9 tall (MAIN, the needles, green-500 and green-700). */
function herbSprig(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -1.2, 1.2);
  const along = (t: number): Point => [lean * t * t, -9 * t];
  const needles = (side: number): string =>
    [0.25, 0.45, 0.65, 0.85]
      .map((t) => {
        const [x, y] = along(t + (side > 0 ? 0.08 : 0));
        return f.poly([[x, y + 0.15], [x + side * 2.4, y - 1.4], [x, y - 0.35]]);
      })
      .join("");
  const [tx, ty] = along(1);
  return [
    shape(f.band([along(0), along(0.5), along(1)], 0.45), "wood-700", 1),
    shape(needles(-1), "green-700", 1, MAIN),
    shape(needles(1) + f.poly([[tx - 0.3, ty + 0.2], [tx + 0.4, ty - 2.2], [tx + 0.3, ty + 0.2]]), "green-500", 1, MAIN),
  ];
}

// Space weather and light (Stinkyscape's pass, R3391000): the pointing parts turn by their placement's rotate

/** A pointing part's frame: each local point turned by the placement's `rotate` about the anchor, then placed. */
type WxTurned = {
  readonly p: (dx: number, dy: number) => string;
  readonly poly: (points: readonly Point[]) => string;
  readonly band: (points: readonly Point[], width: number) => string;
  readonly circle: (dx: number, dy: number, r: number) => string;
};

function wxTurned(at: PartPlace): WxTurned {
  const f = local(at);
  const turn = at.rotate ?? 0;
  const T = ([dx, dy]: Point): Point => turned([dx, dy], turn);
  return {
    p: (dx, dy) => f.p(...T([dx, dy])),
    poly: (points) => f.poly(points.map(T)),
    band: (points, width) => f.band(points.map(T), width),
    circle: (dx, dy, r) => f.circle(...T([dx, dy]), r),
  };
}

/** A many-pointed star about a point, its long points `long` and its waist `waist` from the centre. */
function wxStar(poly: (points: readonly Point[]) => string, cx: number, cy: number, long: number, waist: number, points: number, start: number): string {
  const rim: Point[] = [];
  for (let i = 0; i < points * 2; i += 1) {
    const angle = (start + (i * FULL_TURN) / (points * 2)) * DEGREES;
    const reach = i % 2 === 0 ? long : waist;
    rim.push([cx + reach * Math.cos(angle), cy + reach * Math.sin(angle)]);
  }
  return poly(rim);
}

/** A flame's tongue standing on the ground at `cx`, `w` either side of it and `h` tall, its tip swayed. */
function wxTongue(p: (dx: number, dy: number) => string, cx: number, w: number, h: number, sway: number): string {
  return `M${p(cx - w, 0)}Q${p(cx - w * 1.3, -h * 0.45)} ${p(cx + sway, -h)}Q${p(cx + w * 1.3, -h * 0.45)} ${p(cx + w, 0)}Q${p(cx, w * 0.7)} ${p(cx - w, 0)}Z`;
}

/** An energy beam from its muzzle at the anchor along +x, 40 long at scale 1, flaring where it strikes; turns. */
function beam(at: PartPlace, rng: Rng): KitShape[] {
  const t = wxTurned(at);
  const length = 40 * between(rng, 0.96, 1.04);
  return [
    shape(t.poly([[0, -1.3], [length, -2.8], [length, 2.8], [0, 1.3]]), "cyan-400", 0.2, MAIN),
    shape(t.poly([[0, -0.7], [length, -1.4], [length, 1.4], [0, 0.7]]), "cyan-400", 0.45, MAIN),
    shape(t.poly([[0, -0.25], [length, -0.45], [length, 0.45], [0, 0.25]]), "white", 0.95),
    shape(t.circle(0, 0, 2.2), "cyan-400", 0.4, MAIN),
    shape(t.circle(length, 0, 5.2), "cyan-400", 0.28, MAIN),
    shape(wxStar(t.poly, length, 0, between(rng, 4.2, 5.6), 0.8, 4, between(rng, 0, 30)), "white", 0.9),
  ];
}

/** A flowing ribbon of light from the anchor along +x, 40 long and 3 wide at scale 1, twisting as it goes; turns. */
function lightRibbon(at: PartPlace, rng: Rng): KitShape[] {
  const t = wxTurned(at);
  const length = 40;
  const steps = 10;
  const wave = between(rng, 2.4, 3.6);
  const phase = between(rng, 0, Math.PI * 2);
  const twist = between(rng, 0, Math.PI);
  const centre: Point[] = [];
  const upper: Point[] = [];
  const lower: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const s = i / steps;
    const y = wave * Math.sin(s * Math.PI * 2.2 + phase) * Math.min(1, s * 3);
    const half = 1.5 * (0.3 + 0.7 * Math.abs(Math.cos(s * Math.PI * 1.6 + twist))) * Math.min(1, 0.3 + s * 2);
    centre.push([length * s, y]);
    upper.push([length * s, y - half]);
    lower.push([length * s, y + half]);
  }
  const glints = centre.filter((_, i) => i === 4 || i === 8).map(([x, y]) => t.circle(x, y, 0.55)).join("");
  return [
    shape(t.band(centre, 4.4), "red-300", 0.16, MAIN),
    shape(t.poly([...upper, ...lower.reverse()]), "red-300", 0.6, MAIN),
    shape(t.band(centre, 0.4), "white", 0.75),
    shape(glints, "white", 0.85),
  ];
}

/** A cone of light from its lamp at the anchor along +x, 56 long and 20 wide at its end at scale 1; turns. */
function spotlight(at: PartPlace, rng: Rng): KitShape[] {
  const t = wxTurned(at);
  const length = 56;
  const spread = between(rng, 9, 11);
  return [
    shape(t.poly([[0, -1.2], [length, -spread], [length, spread], [0, 1.2]]), "white", 0.1, MAIN),
    shape(t.poly([[0, -0.6], [length, -spread * 0.5], [length, spread * 0.5], [0, 0.6]]), "white", 0.12, MAIN),
    shape(t.circle(0, 0, 3.6), "white", 0.22, MAIN),
    shape(t.circle(0, 0, 1.6), "white", 0.85),
  ];
}

/** A burning meteor travelling along +x: its tail from the anchor, its flaming rock head at +26 at scale 1; turns. */
function meteor(at: PartPlace, rng: Rng): KitShape[] {
  const t = wxTurned(at);
  const head = 26;
  const r = 3 * between(rng, 0.9, 1.1);
  const rim: Point[] = [];
  const lit: Point[] = [];
  for (let i = 0; i < 7; i += 1) {
    const angle = (i * FULL_TURN) / 7;
    const reach = r * between(rng, 0.8, 1.1);
    rim.push([head + reach * Math.cos(angle * DEGREES), reach * Math.sin(angle * DEGREES)]);
  }
  for (const angle of [-75, -35, 0, 35, 75]) lit.push([head + r * 1.05 * Math.cos(angle * DEGREES), r * 1.05 * Math.sin(angle * DEGREES)]);
  for (const angle of [60, 0, -60]) lit.push([head + r * 0.45 * Math.cos(angle * DEGREES), r * 0.55 * Math.sin(angle * DEGREES)]);
  const ember = (x: number, y: number, size: number): string => t.poly([[x - size, y], [x, y - size * 0.45], [x + size * 0.6, y], [x, y + size * 0.45]]);
  return [
    shape(t.circle(head, 0, 7), "orange-400", 0.2),
    shape(t.poly([[0, 0], [14, -2.2], [head - 2, -4.4], [head + 3.4, 0], [head - 2, 4.4], [14, 2.2]]), "orange-400", 0.4, MAIN),
    shape(t.poly([[4, 0], [16, -1.6], [head - 1, -3.2], [head + 2.6, 0], [head - 1, 3.2], [16, 1.6]]), "ember-500", 0.65),
    shape(t.poly([[12, 0], [head - 3, -1.5], [head + 1, 0], [head - 3, 1.5]]), "white", 0.7),
    shape(t.poly(rim), "wood-800", 1, INKED),
    shape(t.poly(lit), "ember-500", 0.9),
    shape(ember(between(rng, 6, 9), between(rng, -3.5, -2.5), 1) + ember(between(rng, 10, 13), between(rng, 2.6, 3.6), 0.8), "orange-400", 0.85),
  ];
}

/** A trail of smoke from its source at the anchor along +x, 34 long and widening to 10 at scale 1; turns. */
function smokeTrail(at: PartPlace, rng: Rng): KitShape[] {
  const t = wxTurned(at);
  const length = 34;
  const out = [shape(t.poly([[0, -0.4], [length, -4.4], [length, 4.4], [0, 0.4]]), "slate-700", 0.2, MAIN)];
  const glints: string[] = [];
  for (let k = 0; k < 5; k += 1) {
    const s = (k + 0.6) / 5;
    const r = 1.1 + 4 * s;
    const x = length * s - r * 0.3;
    const y = between(rng, -1, 1) * s * 2.4;
    out.push(shape(t.circle(x, y, r), "slate-700", 0.65 - 0.09 * k, MAIN));
    if (k % 2 === 1) glints.push(t.circle(x - r * 0.3, y - r * 0.35, r * 0.4));
  }
  out.push(shape(glints.join(""), "slate-400", 0.35));
  return out;
}

/** A comet, its glowing head at +34 and its long tapering tail back to the anchor at scale 1; turns. */
function comet(at: PartPlace, rng: Rng): KitShape[] {
  const t = wxTurned(at);
  const head = 34;
  const fan = between(rng, 2.8, 3.6);
  return [
    shape(t.circle(head, 0, 6.5), "sky-300", 0.12, MAIN),
    shape(t.poly([[0, -0.2], [18, -fan * 0.5], [head - 4, -fan], [head, -fan * 0.8], [head, fan * 0.8], [head - 4, fan], [18, fan * 0.5], [0, 0.2]]), "sky-300", 0.25, MAIN),
    shape(t.poly([[6, 0], [head - 8, -1.4], [head, -1.3], [head, 1.3], [head - 8, 1.4]]), "sky-300", 0.45, MAIN),
    shape(t.poly([[2, -fan * 1.1], [head - 2, -fan * 0.9], [head - 1, -fan * 0.5]]), "blue-200", 0.25),
    shape(t.circle(head, 0, 3.2), "sky-300", 0.4, MAIN),
    shape(t.circle(head, 0, 1.6), "white", 0.95),
  ];
}

/** Flattened rings of force spreading about the anchor, 30 wide and 10 tall at scale 1. */
function shockwave(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rings = whole(rng, 2, 3);
  const out = [shape(f.ellipse(0, 0, 12, 3.6), "white", 0.1, MAIN)];
  for (let k = 0; k < rings; k += 1) {
    const rx = (8 + k * 5) * between(rng, 0.94, 1.06);
    const ry = rx * 0.32;
    const band = 0.9 + k * 0.3;
    out.push(shape(f.ellipse(0, 0, rx, ry) + f.ellipse(0, 0, rx - band, ry - band * 0.45), "white", 0.6 - k * 0.17, MAIN));
  }
  out.push(shape(f.ellipse(0, 0, 4.4, 1.3), "white", 0.5));
  return out;
}

/** A bright star with a long four-way flare and glow rings, centred on the anchor, 24 across at scale 1. */
function starGlare(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 10.5, 12.5);
  const cross: Point[] = [[reach, 0], [0.7, -0.7], [0, -reach * 0.8], [-0.7, -0.7], [-reach, 0], [-0.7, 0.7], [0, reach * 0.8], [0.7, 0.7]];
  return [
    shape(f.circle(0, 0, 11), "white", 0.08, MAIN),
    shape(f.circle(0, 0, 6.5), "white", 0.14, MAIN),
    shape(f.circle(0, 0, 3.4), "white", 0.35, MAIN),
    shape(f.poly(cross), "white", 0.8),
    shape(wxStar(f.poly, 0, 0, between(rng, 4, 5.4), 0.45, 4, 45), "white", 0.5),
    shape(f.circle(0, 0, 1.7), "white", 1),
  ];
}

/** A storm-banded gas giant centred on the anchor, radius 15 at scale 1, its shadow away from the star on the right. */
function gasGiant(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 15;
  const R = fmt(f.r(r));
  const span = (y: number): number => Math.sqrt(Math.max(0, r * r - y * y));
  const belt = (top: number, bottom: number): string =>
    `M${f.p(-span(top), top)}L${f.p(span(top), top)}A${R} ${R} 0 0 ${f.sweep(1)} ${f.p(span(bottom), bottom)}L${f.p(-span(bottom), bottom)}A${R} ${R} 0 0 ${f.sweep(1)} ${f.p(-span(top), top)}Z`;
  const j = (): number => between(rng, -0.8, 0.8);
  const shadow = `M${f.p(0, -r)}A${R} ${R} 0 0 ${f.sweep(0)} ${f.p(0, r)}A${fmt(f.r(r * 0.55))} ${R} 0 0 ${f.sweep(1)} ${f.p(0, -r)}Z`;
  return [
    shape(f.circle(0, 0, r * 1.12), "orange-400", 0.07, MAIN),
    shape(f.circle(0, 0, r), "orange-400", 1, MAIN_INKED),
    shape(belt(-10 + j(), -7 + j()) + belt(2.5 + j(), 5.5 + j()), "red-600", 0.45),
    shape(belt(-4.5 + j(), -2.5 + j()) + belt(8.5 + j(), 10.5 + j()), "white", 0.25),
    shape(f.ellipse(between(rng, 2, 6), between(rng, 3.4, 4.6), 2.8, 1.5), "red-900", 0.7),
    shape(shadow, "slate-900", 0.5),
    shape(crescent(f, r, 0.86), "white", 0.25),
  ];
}

/** Space shattered into jagged lensing shards about the anchor, within 20 at scale 1, cracks running between them. */
function spaceShards(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const shards = whole(rng, 5, 7);
  const sector = FULL_TURN / shards;
  const even: string[] = [];
  const odd: string[] = [];
  const edges: string[] = [];
  const cracks: string[] = [];
  const polar = (r: number, degrees: number): Point => [r * Math.cos(degrees * DEGREES), r * Math.sin(degrees * DEGREES)];
  const start = between(rng, 0, sector);
  for (let k = 0; k < shards; k += 1) {
    const mid = start + k * sector;
    const reach = between(rng, 6, 15);
    const size = Math.min(7, reach * Math.sin((sector / 2) * DEGREES) * 0.8);
    const [cx, cy] = polar(reach, mid + between(rng, -sector * 0.1, sector * 0.1));
    const spin = between(rng, 0, FULL_TURN);
    const points: Point[] = [0, 1, 2, 3].map((i) => {
      const [dx, dy] = polar(size * (i === 0 ? 1 : between(rng, 0.45, 0.85)), spin + i * 90 + between(rng, -25, 25));
      return [cx + dx, cy + dy] as const;
    });
    (k % 2 === 0 ? even : odd).push(f.poly(points));
    edges.push(f.band([points[3] ?? [cx, cy], points[0] ?? [cx, cy], points[1] ?? [cx, cy]], 0.3));
    if (k % 2 === 0) {
      const crack = mid + sector / 2;
      cracks.push(f.band([polar(2, crack), polar(9, crack + between(rng, -6, 6)), polar(between(rng, 15, 20), crack + between(rng, -4, 4))], 0.25));
    }
  }
  return [
    shape(even.join(""), "purple-400", 0.42, MAIN),
    shape(odd.join(""), "violet-300", 0.34),
    shape(edges.join(""), "white", 0.65),
    shape(cracks.join(""), "slate-100", 0.5),
  ];
}

/** A grid of light lines in skewed perspective, 44 wide and 24 deep, centred on the anchor at scale 1. */
function lightGrid(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const skew = between(rng, -6, 6);
  const back = 15;
  const front = 22;
  const depth = 12;
  const corner = (u: number, v: number): Point => {
    const half = back + (front - back) * v;
    return [skew * (1 - v) + half * (2 * u - 1), -depth + 2 * depth * v ** 1.2];
  };
  const rows: string[] = [];
  const columns: string[] = [];
  const nodes: string[] = [];
  for (let k = 0; k <= 5; k += 1) rows.push(f.band([corner(0, k / 5), corner(1, k / 5)], 0.3 + 0.25 * (k / 5)));
  for (let k = 0; k <= 6; k += 1) columns.push(f.band([corner(k / 6, 0), corner(k / 6, 1)], 0.35));
  for (let k = 0; k < 3; k += 1) nodes.push(f.circle(...corner(whole(rng, 0, 6) / 6, whole(rng, 1, 5) / 5), 0.7));
  return [
    shape(f.poly([corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)]), "jade-400", 0.1, MAIN),
    shape(rows.join(""), "jade-400", 0.75, MAIN),
    shape(columns.join(""), "jade-400", 0.6, MAIN),
    shape(nodes.join(""), "white", 0.85),
  ];
}

/** A ringed rift: a dark core inside a glowing torn rim, centred on the anchor, 16 wide and 22 tall at scale 1. */
function rift(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const torn: Point[] = [];
  for (let i = 0; i < 14; i += 1) {
    const angle = (i * FULL_TURN) / 14;
    const reach = i % 2 === 0 ? between(rng, 1.15, 1.35) : between(rng, 1, 1.08);
    torn.push([8 * reach * Math.cos(angle * DEGREES), 11 * reach * Math.sin(angle * DEGREES)]);
  }
  const start = between(rng, 0, FULL_TURN);
  return [
    shape(f.poly(torn), "violet-500", 0.25, MAIN),
    shape(f.ellipse(0, 0, 6.8, 9.6), "black", 0.92),
    shape(spiralArm(f, start, 230, 0.8, 6, 1.8), "violet-300", 0.35),
    shape(spiralArm(f, start + 180, 200, 0.8, 5.4, 1.4), "purple-400", 0.35),
    shape(f.ellipse(0, 0, 8, 11) + f.ellipse(0, 0, 6.6, 9.4), "violet-500", 0.85, MAIN),
    shape(f.band([-70, -45, -20, 5].map((degrees): Point => [7.3 * Math.cos(degrees * DEGREES), 10.2 * Math.sin(degrees * DEGREES)]), 0.5), "white", 0.4),
  ];
}

/** Frost spreading from the anchor: three to six pale ice spikes within 7 at scale 1. */
function frostShard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spikes = whole(rng, 3, 6);
  const sector = FULL_TURN / spikes;
  const start = between(rng, 0, sector);
  const bodies: string[] = [];
  const glints: string[] = [];
  for (let k = 0; k < spikes; k += 1) {
    const angle = (start + k * sector + between(rng, -sector * 0.15, sector * 0.15)) * DEGREES;
    const length = between(rng, 4, 7);
    const width = length * 0.14;
    const [ux, uy] = [Math.cos(angle), Math.sin(angle)];
    const along = (t: number, side: number): Point => [ux * t - uy * side, uy * t + ux * side];
    bodies.push(f.poly([along(0.8, 0), along(length * 0.35, -width), along(length, 0), along(length * 0.35, width)]));
    glints.push(f.band([along(1.2, 0), along(length * 0.8, 0)], 0.2));
  }
  return [
    shape(f.circle(0, 0, 5), "sky-300", 0.1),
    shape(bodies.join(""), "blue-200", 0.9, MAIN_INKED),
    shape(glints.join(""), "white", 0.7),
    shape(sparkle(f, between(rng, -3, 3), between(rng, -3, 3), 1.4), "white", 0.9),
  ];
}

/** A spray of small missiles flying up and to +x, each trailing exhaust, about the anchor, 20 across at scale 1. */
function missiles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const heading = -35;
  const bodies: string[] = [];
  const trails: string[] = [];
  const flares: string[] = [];
  for (let k = 0; k < count; k += 1) {
    const offset = (k - (count - 1) / 2) * 4.2;
    const stagger = between(rng, -3, 3);
    const base = turned([stagger, offset], heading);
    const angle = heading + between(rng, -6, 6);
    const P = (u: number, v: number): Point => {
      const [dx, dy] = turned([u, v], angle);
      return [base[0] + dx, base[1] + dy];
    };
    bodies.push(f.poly([P(2.4, 0), P(1, -0.55), P(-1.6, -0.55), P(-2.3, -1.2), P(-1.9, 0), P(-2.3, 1.2), P(-1.6, 0.55), P(1, 0.55)]));
    trails.push(f.poly([P(-2, -0.5), P(-8.5, 0), P(-2, 0.5)]));
    flares.push(f.circle(...P(-2.2, 0), 0.7));
  }
  return [
    shape(trails.join(""), "violet-500", 0.4),
    shape(bodies.join(""), "violet-300", 1, MAIN_INKED),
    shape(flares.join(""), "white", 0.85),
  ];
}

/** A column of flame rising from the anchor, 22 tall and 8 wide at scale 1, in tongues of ember, orange and white. */
function flameColumn(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = (): number => between(rng, -1.6, 1.6);
  const lick = (x: number, y: number, size: number): string => f.poly([[x, y - size], [x + size * 0.4, y], [x, y + size * 0.5], [x - size * 0.4, y]]);
  return [
    shape(f.ellipse(0, -10, 8, 12.5), "orange-400", 0.1),
    shape(wxTongue(f.p, -2.8, 2, between(rng, 11, 14), sway()) + wxTongue(f.p, 2.8, 2, between(rng, 12, 15), sway()), "ember-500", 0.8, MAIN),
    shape(wxTongue(f.p, 0, 4, between(rng, 20, 23), sway()), "ember-500", 0.9, MAIN),
    shape(wxTongue(f.p, 0, 2.8, between(rng, 14, 17), sway()), "orange-400", 0.95),
    shape(wxTongue(f.p, 0, 1.4, between(rng, 8, 10), sway() * 0.5), "white", 0.85),
    shape(lick(between(rng, -4, -2), between(rng, -20, -16), 1) + lick(between(rng, 2, 4), between(rng, -24, -19), 0.8), "orange-400", 0.85),
  ];
}

/** A hexagonal shield bubble centred on the anchor, radius 14 at scale 1: its glow, its lattice and its rim. */
function hexShield(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 14;
  const cell = 3;
  const step = cell * Math.sqrt(3);
  const cells: Point[] = [[0, 0]];
  for (let ring = 1; ring <= 2; ring += 1) {
    for (let side = 0; side < 6; side += 1) {
      const a = (side * 60 - 90) * DEGREES;
      const b = ((side + 2) * 60 - 90) * DEGREES;
      for (let k = 0; k < ring; k += 1) cells.push([step * (ring * Math.cos(a) + k * Math.cos(b)), step * (ring * Math.sin(a) + k * Math.sin(b))]);
    }
  }
  const flare = new Set([whole(rng, 1, 18), whole(rng, 1, 18), whole(rng, 1, 18)]);
  const hex = ([dx, dy]: Point): string => polygonPath(f.x(dx), f.y(dy), 6, f.r(cell * 0.84), 0);
  return [
    shape(f.circle(0, 0, r * 1.12), "cyan-400", 0.1, MAIN),
    shape(f.circle(0, 0, r), "cyan-400", 0.12, MAIN),
    shape(cells.filter((_, k) => !flare.has(k)).map(hex).join(""), "cyan-400", 0.2, MAIN),
    shape(cells.filter((_, k) => flare.has(k)).map(hex).join(""), "white", 0.35),
    shape(f.circle(0, 0, r) + f.circle(0, 0, r * 0.93), "cyan-400", 0.6, MAIN),
    shape(f.ellipse(-5.5, -8, 3.6, 1.4), "white", 0.2),
  ];
}

/** R3331000: the scenery parts, by id, each with its kind. */
export const SCENE_KIT = {
  field: { category: "terrain", draw: field },
  furrows: { category: "terrain", draw: furrows },
  "grave-mound": { category: "terrain", draw: graveMound },
  cliff: { category: "terrain", draw: cliff },
  "rock-floor": { category: "terrain", draw: rockFloor },
  "tile-floor": { category: "terrain", draw: tileFloor },
  "room-wall": { category: "terrain", draw: roomWall },
  street: { category: "terrain", draw: street },
  hills: { category: "terrain", draw: hills },
  asteroid: { category: "terrain", draw: asteroid },
  fence: { category: "architecture", draw: fence },
  "fence-post": { category: "architecture", draw: fencePost },
  gate: { category: "architecture", draw: gate },
  "ruin-wall": { category: "architecture", draw: ruinWall },
  tower: { category: "architecture", draw: tower },
  column: { category: "architecture", draw: column },
  arch: { category: "architecture", draw: arch },
  tombstone: { category: "architecture", draw: tombstone },
  sprout: { category: "plants", draw: sprout },
  "grass-tuft": { category: "plants", draw: grassTuft },
  "dead-tree": { category: "plants", draw: deadTree },
  tree: { category: "plants", draw: tree },
  vine: { category: "plants", draw: vine },
  bush: { category: "plants", draw: bush },
  mushroom: { category: "plants", draw: mushroom },
  wheat: { category: "plants", draw: wheat },
  crow: { category: "creatures", draw: crow },
  bat: { category: "creatures", draw: bat },
  "pale-hand": { category: "creatures", draw: paleHand },
  "bone-hand": { category: "creatures", draw: boneHand },
  wisp: { category: "creatures", draw: wisp },
  worm: { category: "creatures", draw: worm },
  moth: { category: "creatures", draw: moth },
  lantern: { category: "objects", draw: lantern },
  "bone-scarecrow": { category: "objects", draw: boneScarecrow },
  pitchfork: { category: "objects", draw: pitchfork },
  shovel: { category: "objects", draw: shovel },
  crate: { category: "objects", draw: crate },
  banner: { category: "objects", draw: banner },
  orb: { category: "objects", draw: orb },
  crystal: { category: "objects", draw: crystal },
  moon: { category: "weather", draw: moon },
  stars: { category: "weather", draw: stars },
  cloud: { category: "weather", draw: cloud },
  mist: { category: "weather", draw: mist },
  "light-shaft": { category: "weather", draw: lightShaft },
  halo: { category: "weather", draw: halo },
  "god-rays": { category: "weather", draw: godRays },
  sparks: { category: "weather", draw: sparks },
  embers: { category: "weather", draw: embers },
  motes: { category: "weather", draw: motes },
  "void-swirl": { category: "weather", draw: voidSwirl },
  lightning: { category: "weather", draw: lightning },
  nebula: { category: "weather", draw: nebula },
  planet: { category: "weather", draw: planet },
  "rim-light": { category: "weather", draw: rimLight },
  fighter: { category: "craft", draw: fighter },
  "needle-fighter": { category: "craft", draw: needleFighter },
  "arrow-fighter": { category: "craft", draw: arrowFighter },
  "blade-fighter": { category: "craft", draw: bladeFighter },
  scout: { category: "craft", draw: scout },
  walker: { category: "craft", draw: walker },
  skiff: { category: "craft", draw: skiff },
  drone: { category: "craft", draw: drone },
  "helper-drone": { category: "craft", draw: helperDrone },
  "drone-corvette": { category: "craft", draw: droneCorvette },
  "drone-battleship": { category: "craft", draw: droneBattleship },
  gunship: { category: "craft", draw: gunship },
  "miner-craft": { category: "craft", draw: minerCraft },
  barge: { category: "craft", draw: barge },
  trader: { category: "craft", draw: trader },
  flagship: { category: "craft", draw: flagship },
  "frame-ship": { category: "craft", draw: frameShip },
  "fused-hull": { category: "craft", draw: fusedHull },
  wreck: { category: "craft", draw: wreck },
  "ghost-ship": { category: "craft", draw: ghostShip },
  rocket: { category: "craft", draw: rocket },
  tram: { category: "craft", draw: tram },
  "gun-ring": { category: "craft", draw: gunRing },
  station: { category: "architecture", draw: station },
  "dock-gantry": { category: "architecture", draw: dockGantry },
  "hangar-wall": { category: "architecture", draw: hangarWall },
  "vault-door": { category: "architecture", draw: vaultDoor },
  "server-core": { category: "architecture", draw: serverCore },
  "cell-bars": { category: "architecture", draw: cellBars },
  console: { category: "architecture", draw: consolePart },
  "antenna-mast": { category: "architecture", draw: antennaMast },
  "bunk-pod": { category: "architecture", draw: bunkPod },
  "pilot-seat": { category: "architecture", draw: pilotSeat },
  "cockpit-frame": { category: "architecture", draw: cockpitFrame },
  porthole: { category: "architecture", draw: porthole },
  "glass-case": { category: "architecture", draw: glassCase },
  "market-stall": { category: "architecture", draw: marketStall },
  "disruptor-beacon": { category: "architecture", draw: disruptorBeacon },
  conveyor: { category: "architecture", draw: conveyor },
  reactor: { category: "architecture", draw: reactor },
  "deck-floor": { category: "terrain", draw: deckFloor },
  corridor: { category: "terrain", draw: corridor },
  regolith: { category: "terrain", draw: regolith },
  "lava-flats": { category: "terrain", draw: lavaFlats },
  "hover-platform": { category: "terrain", draw: hoverPlatform },
  "ray-gun": { category: "objects", draw: rayGun },
  "drone-core": { category: "objects", draw: droneCore },
  "credit-chips": { category: "objects", draw: creditChips },
  duffel: { category: "objects", draw: duffel },
  cracker: { category: "objects", draw: cracker },
  "war-drum": { category: "objects", draw: warDrum },
  "coffee-bulb": { category: "objects", draw: coffeeBulb },
  "ice-block": { category: "objects", draw: iceBlock },
  "boot-clamp": { category: "objects", draw: bootClamp },
  "cracked-mask": { category: "objects", draw: crackedMask },
  "ancient-rod": { category: "objects", draw: ancientRod },
  anvil: { category: "objects", draw: anvil },
  "containment-sphere": { category: "objects", draw: containmentSphere },
  scale: { category: "objects", draw: scalePart },
  "wire-coil": { category: "objects", draw: wireCoil },
  lectern: { category: "objects", draw: lectern },
  "robot-arm": { category: "objects", draw: robotArm },
  debris: { category: "objects", draw: debris },
  rope: { category: "objects", draw: rope },
  "blank-card": { category: "objects", draw: blankCard },
  "gloved-hand": { category: "creatures", draw: glovedHand },
  cosmality: { category: "creatures", draw: cosmality },
  sleeper: { category: "creatures", draw: sleeper },
  "berry-vine": { category: "plants", draw: berryVine },
  "herb-sprig": { category: "plants", draw: herbSprig },
  beam: { category: "weather", draw: beam },
  "light-ribbon": { category: "weather", draw: lightRibbon },
  spotlight: { category: "weather", draw: spotlight },
  meteor: { category: "weather", draw: meteor },
  "smoke-trail": { category: "weather", draw: smokeTrail },
  comet: { category: "weather", draw: comet },
  shockwave: { category: "weather", draw: shockwave },
  "star-glare": { category: "weather", draw: starGlare },
  "gas-giant": { category: "weather", draw: gasGiant },
  "space-shards": { category: "weather", draw: spaceShards },
  "light-grid": { category: "weather", draw: lightGrid },
  rift: { category: "weather", draw: rift },
  "frost-shard": { category: "weather", draw: frostShard },
  missiles: { category: "weather", draw: missiles },
  "flame-column": { category: "weather", draw: flameColumn },
  "hex-shield": { category: "weather", draw: hexShield },
} as const satisfies Readonly<Record<string, KitPart>>;

export type SceneryPartId = keyof typeof SCENE_KIT;
