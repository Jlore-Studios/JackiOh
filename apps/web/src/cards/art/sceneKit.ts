// R3331000: the procedural painter's first scene kit (ART.md, "Scene recipes and kits"): the scenery a
// scene recipe (sceneRecipes.ts) places, in six kinds (terrain, architecture, plants, creatures,
// objects, and weather and light). Each part draws one to twelve filled shapes about its anchor, scaled
// and faced as it is placed, varied by the scene's stream and never by the face: a Radiant face draws
// the same shapes, only repainted (scene.ts `withScene`). Every colour is a palette.css name, none in
// Radiant's gold window (no `gold-*`, `yellow-400` or `parchment`); a glow is a stack of translucent
// shapes, never a gradient. A terrain part spans the box from its placement's height down; the rest
// stand on their anchor (a plant, a building, an object), hang from it (a lantern, a light shaft) or
// centre on it (a creature in flight, a moon, a spark). Shapes are filled even-odd, as the painter
// fills every ridge, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

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
} as const satisfies Readonly<Record<string, KitPart>>;

export type SceneryPartId = keyof typeof SCENE_KIT;
