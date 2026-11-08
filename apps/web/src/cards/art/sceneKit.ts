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
// R3392000: Core's scenes added the places, creatures, props and weather their briefs name, each part after
// the old ones of its kind.

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

/** The box from `top` down past its bottom, across all of it. */
function belowKitA(top: number): string {
  return rectPath(EDGE, top, FAR_EDGE - EDGE, FAR_EDGE - top);
}

/** A zigzag across the box about `y`, `steps` segments, each vertex up to `amp` off. */
function acrossKitA(rng: Rng, y: number, amp: number, steps: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i <= steps; i += 1) out.push([EDGE + ((FAR_EDGE - EDGE) * i) / steps, y + (i % 2 === 0 ? -1 : 1) * amp * between(rng, 0.5, 1)]);
  return out;
}

/** Where a scene-wide path's offsets are the box's own coordinates. */
const WORLD_KIT_A: PartPlace = { x: 0, y: 0, s: 1, flip: 1 };

/** The `polygon`'s part with `nx*x + ny*y >= c`. */
function clipKitA(polygon: readonly Point[], nx: number, ny: number, c: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i < polygon.length; i += 1) {
    const a = polygon[i] ?? [0, 0];
    const b = polygon[(i + 1) % polygon.length] ?? [0, 0];
    const da = nx * a[0] + ny * a[1] - c;
    const db = nx * b[0] + ny * b[1] - c;
    if (da >= 0) out.push(a);
    if (da >= 0 !== db >= 0) {
      const t = da / (da - db);
      out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
    }
  }
  return out;
}

/** A sea or lake from the placement's height down: waves and glints. */
function waterPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const out: KitShape[] = [shape(belowKitA(top), "blue-600", 1, MAIN)];
  for (let k = 0; k < 3; k += 1) {
    const y = top + depth * (0.2 + 0.28 * k);
    out.push(shape(bandPath(acrossKitA(rng, y, 0.9 * at.s, 8), (1 + 0.7 * k) * at.s), "sky-300", 0.3 + 0.05 * k));
  }
  let glints = "";
  for (let i = 0; i < 5; i += 1) {
    const x = 10 + i * 19 + between(rng, -4, 4);
    const y = top + depth * (0.12 + ((i * 2) % 5) * 0.17) + between(rng, -1, 1);
    glints += bandPath([[x, y], [x + 5, y]], 0.5 * at.s);
  }
  out.push(shape(glints, "white", 0.8));
  return out;
}

/** Desert dunes from the placement's height down: two dune bands and faint ripples. */
function sandPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  let ripples = "";
  for (let k = 0; k < 3; k += 1) ripples += bandPath(acrossKitA(rng, top + depth * (0.3 + 0.25 * k) + 5, 0.8, 6), 0.6 * at.s);
  return [
    shape(belowKitA(top), "brass-500", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.3, 3 * at.s, 3), "brass-600", 0.55),
    shape(groundBand(rng, top + depth * 0.58, 3 * at.s, 3), "brass-500", 1, MAIN),
    shape(ripples, "wood-600", 0.14),
  ];
}

/** A snowfield from the placement's height down: drift shadows and a few sparkles. */
function snowPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const w = local(WORLD_KIT_A);
  let glitter = "";
  for (let i = 0; i < 3; i += 1) glitter += sparkle(w, 14 + i * 32 + between(rng, -5, 5), top + depth * (0.2 + 0.3 * ((i * 2) % 3)) + between(rng, -1, 1), between(rng, 1.6, 2.4) * at.s);
  return [
    shape(belowKitA(top), "white", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.28, 2 * at.s, 4), "blue-200", 0.55),
    shape(groundBand(rng, top + depth * 0.28 + 3 * at.s, 2 * at.s, 4), "white", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.66, 2 * at.s, 4), "blue-200", 0.5),
    shape(groundBand(rng, top + depth * 0.66 + 3 * at.s, 2 * at.s, 4), "white", 1, MAIN),
    shape(glitter, "sky-300", 0.8),
  ];
}

/** A planked floor, deck or jetty from the placement's height down: seams meeting at its x. */
function boardsPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const planks = 6;
  const foot = (i: number): number => -30 + (160 * i) / planks;
  const apex = (i: number): number => at.x + (foot(i) - at.x) * 0.25;
  const seamX = (i: number, t: number): number => apex(i) + (foot(i) - apex(i)) * t;
  let seams = "";
  for (let i = 0; i < planks; i += 1) {
    seams += `M${pt(apex(i) - 0.12, top)}L${pt(apex(i) + 0.12, top)}L${pt(foot(i) + 0.5, FAR_EDGE)}L${pt(foot(i) - 0.5, FAR_EDGE)}Z`;
  }
  let ends = "";
  for (let k = 0; k < 3; k += 1) {
    const t = 0.25 + 0.25 * k + between(rng, -0.04, 0.04);
    const i = (k * 2 + 1 + whole(rng, 0, 1)) % (planks - 1);
    const x0 = seamX(i, t);
    ends += rectPath(x0 + 0.3, top + depth * t, seamX(i + 1, t) - x0 - 0.6, 0.35 + 0.5 * t);
  }
  return [
    shape(belowKitA(top), "wood-600", 1, MAIN),
    shape(seams, "wood-900", 0.5),
    shape(ends, "wood-900", 0.4),
    shape(rectPath(EDGE, top + depth * 0.86, FAR_EDGE - EDGE, depth * 0.15), "brass-600", 0.3),
  ];
}

/** A raised stage from the placement's height down: a lower front lip, footlights and a sheen. */
function stagePart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const lip = top + depth * 0.8;
  let lights = "";
  for (let i = 0; i < 9; i += 1) lights += circlePath(5 + i * 11.5 + between(rng, -0.6, 0.6), lip + 1.8 * at.s, 0.75 * at.s);
  return [
    shape(belowKitA(top), "slate-800", 1, MAIN),
    shape(rectPath(EDGE, lip, FAR_EDGE - EDGE, FAR_EDGE - lip), "slate-900", 1),
    shape(rectPath(EDGE, lip, FAR_EDGE - EDGE, 0.7 * at.s), "slate-700", 0.9),
    shape(lights, "white", 0.95),
    shape(ellipsePath(at.x + between(rng, -8, 8), top + depth * 0.4, 34, depth * 0.17), "white", 0.07),
  ];
}

/** A dirt road winding from narrow at the placement's point to wide at the bottom edge, verges either side. */
function roadPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const bottomX = at.x + between(rng, -14, 14);
  const bend = between(rng, -12, 12);
  const centre = (t: number): number => at.x + (bottomX - at.x) * t + bend * Math.sin(Math.PI * t);
  const half = (t: number): number => (1.2 + 21 * t ** 1.2) * at.s;
  const steps = [0, 0.2, 0.4, 0.6, 0.8, 1];
  const side = (sign: number, extra: number, along: readonly number[]): Point[] => along.map((t) => [centre(t) + sign * (half(t) + extra * (0.4 + t)), top + depth * t] as const);
  const left = side(-1, 0, steps);
  const right = side(1, 0, steps).reverse();
  const verge = (sign: number): string => bandPath(side(sign, 1.8 * at.s, steps), (0.8 + 0.5) * at.s);
  const rut = (k: number): string => bandPath([0.15, 0.4, 0.65, 0.9, 1].map((t) => [centre(t) + k * half(t) * 0.38, top + depth * t] as const), 0.8 * at.s);
  return [
    shape(belowKitA(top), "green-700", 1),
    shape(`M${[...left, ...right].map(([x, y]) => pt(x, y)).join("L")}Z`, "wood-600", 1, MAIN),
    shape(verge(-1) + verge(1), "green-500", 0.3),
    shape(rut(-1) + rut(1), "wood-800", 0.3),
  ];
}

/** A chequered floor in perspective from the placement's height down, its lines meeting at its x. */
function checkerFloorPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const cols = 6;
  const rows = 5;
  const foot = (i: number): number => -30 + (160 * i) / cols;
  const apex = (i: number): number => at.x + (foot(i) - at.x) * 0.25;
  const xAt = (i: number, y: number): number => apex(i) + ((foot(i) - apex(i)) * (y - top)) / depth;
  const rowY = (k: number): number => top + depth * (k / rows) ** 1.6;
  let dark = "";
  for (let k = 0; k < rows; k += 1) {
    for (let i = 0; i < cols; i += 1) {
      if ((i + k) % 2 === 0) continue;
      const y0 = rowY(k);
      const y1 = rowY(k + 1);
      dark += `M${pt(xAt(i, y0), y0)}L${pt(xAt(i + 1, y0), y0)}L${pt(xAt(i + 1, y1), y1)}L${pt(xAt(i, y1), y1)}Z`;
    }
  }
  return [
    shape(belowKitA(top), "slate-100", 1, MAIN),
    shape(dark, "slate-800", 1),
    shape(ellipsePath(at.x + between(rng, -8, 8), top + depth * 0.45, 26, depth * 0.2), "white", 0.14),
  ];
}

/** A cobbled square from the placement's height down: cobbles in two shapes, smaller toward the top. */
function cobblesPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const widths = [10, 15, 22];
  const fractions = [0.15, 0.45, 0.8];
  let pale = "";
  let dark = "";
  for (let r = 0; r < 3; r += 1) {
    const w = (widths[r] ?? 10) * at.s;
    const rx = w * 0.36;
    const ry = rx * 0.38;
    const y = top + depth * (fractions[r] ?? 0.5);
    const cells = Math.ceil((FAR_EDGE - EDGE) / w);
    for (let i = 0; i < cells; i += 1) {
      const x = EDGE + w * (i + 0.5) + (r % 2) * w * 0.25 + between(rng, -0.1, 0.1) * w;
      const d = ellipsePath(x, y + between(rng, -0.1, 0.1) * ry, rx * between(rng, 0.9, 1), ry);
      if ((i + r) % 2 === 0) pale += d;
      else dark += d;
    }
  }
  return [shape(belowKitA(top), "stone-500", 1, MAIN), shape(pale, "slate-400", 1), shape(dark, "slate-700", 1)];
}

/** A lava field from the placement's height down: a dark crust, glowing cracks and a glow along its top. */
function lavaPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  let hot = "";
  let core = "";
  for (let k = 0; k < 3; k += 1) {
    const line = acrossKitA(rng, top + depth * (0.25 + 0.3 * k), 2.2 * at.s, 7);
    hot += bandPath(line, (1.4 + 0.5 * k) * at.s);
    core += bandPath(line, (0.7 + 0.25 * k) * at.s);
  }
  return [
    shape(belowKitA(top), "red-900", 1, MAIN),
    shape(rectPath(EDGE, top - 2, FAR_EDGE - EDGE, 6 * at.s), "orange-400", 0.3),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 1.8 * at.s), "ember-500", 0.3),
    shape(hot, "orange-400", 0.9),
    shape(core, "ember-500", 0.95),
  ];
}

/** A white-tiled wall from the top of the box down to the floor at the placement's height, a floor below it. */
function tileWallPart(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const rowGap = 9;
  const colGap = 13;
  const lines = Math.max(1, Math.floor((floor - 4) / rowGap));
  const offset = between(rng, 0, 3);
  const grout = 0.35 * at.s;
  let grid = "";
  for (let r = 0; r < lines; r += 1) grid += rectPath(EDGE, rowGap * (r + 1) - grout / 2, FAR_EDGE - EDGE, grout);
  for (let r = 0; r <= lines; r += 1) {
    const y0 = r === 0 ? EDGE : rowGap * r + grout / 2;
    const y1 = r === lines ? floor - 2.5 : rowGap * (r + 1) - grout / 2;
    for (let x = offset; x < FAR_EDGE; x += colGap) if (y1 > y0) grid += rectPath(x - grout / 2, y0, grout, y1 - y0);
  }
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor - EDGE), "slate-100", 1, MAIN),
    shape(grid, "slate-400", 0.35),
    shape(rectPath(EDGE, floor - 2.5, FAR_EDGE - EDGE, 2.5), "slate-700", 0.85),
    shape(belowKitA(floor), "slate-400", 1),
    shape(rectPath(EDGE, floor, FAR_EDGE - EDGE, 1.2), "slate-900", 0.25),
  ];
}

/** A cave round the box: rock walls left and right and a stalactite ceiling, a floor from the placement's height down. */
function cavernPart(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const wallL = between(rng, 12, 18);
  const wallR = SCENE_BOX - between(rng, 12, 18);
  const ceiling = floor * 0.2;
  const inner: Point[] = [
    [wallR, floor],
    [wallR + 3, floor * 0.7],
    [wallR - 2, floor * 0.45],
    [wallR - 4, ceiling],
  ];
  const teeth = 5;
  for (let i = 0; i < teeth; i += 1) {
    const x = wallR - 4 - ((wallR - wallL - 8) * (i + 0.5)) / teeth;
    inner.push([x + ((wallR - wallL - 8) / teeth) * 0.5, ceiling + between(rng, 0, 2)]);
    inner.push([x, ceiling + between(rng, 7, 15)]);
  }
  inner.push([wallL + 4, ceiling], [wallL - 2 + 4, floor * 0.45], [wallL - 3, floor * 0.7], [wallL, floor]);
  const rock = `M${pt(EDGE, floor)}L${pt(EDGE, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, floor)}L${inner.map(([x, y]) => pt(x, y)).join("L")}Z`;
  return [
    shape(belowKitA(floor), "slate-700", 1),
    shape(ellipsePath(50, floor + (FAR_EDGE - floor) * 0.45, 30, (FAR_EDGE - floor) * 0.2), "slate-400", 0.12),
    shape(rock, "slate-800", 1, MAIN),
    shape(bandPath(inner.slice(1), 1.4 * at.s), "slate-700", 1),
  ];
}

/** An endless warehouse aisle: shelf walls either side running to a vanishing point just above the placement, box rows, a floor. */
function aislePart(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const vy = floor - 10;
  const gap = 5;
  const lx = at.x - gap;
  const rx = at.x + gap;
  const farTop = vy - 6;
  const farBot = floor;
  const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
  const wallY = (u: number, v: number): number => lerp(lerp(EDGE, FAR_EDGE, v), lerp(farTop, farBot, v), u);
  const wallX = (side: number, u: number): number => (side < 0 ? lerp(EDGE, lx, u) : lerp(FAR_EDGE, rx, u));
  const boards = [0.34, 0.6, 0.86];
  const walls =
    `M${pt(EDGE, EDGE)}L${pt(lx, farTop)}L${pt(lx, farBot)}L${pt(EDGE, FAR_EDGE)}Z` +
    `M${pt(FAR_EDGE, EDGE)}L${pt(rx, farTop)}L${pt(rx, farBot)}L${pt(FAR_EDGE, FAR_EDGE)}Z`;
  let lines = "";
  for (const v of boards) for (const side of [-1, 1]) lines += bandPath([[wallX(side, 0), wallY(0, v)], [wallX(side, 1), wallY(1, v)]], 1.1 * at.s);
  const spans: readonly (readonly [number, number])[] = [[0.04, 0.3], [0.38, 0.58], [0.64, 0.78], [0.82, 0.9]];
  let crates = "";
  for (const v of [0.6, 0.86]) {
    for (const side of [-1, 1]) {
      for (const [a, b] of spans) {
        const h = 0.16 + between(rng, -0.02, 0.02);
        crates += `M${pt(wallX(side, a), wallY(a, v - h))}L${pt(wallX(side, b), wallY(b, v - h))}L${pt(wallX(side, b), wallY(b, v))}L${pt(wallX(side, a), wallY(a, v))}Z`;
      }
    }
  }
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor - EDGE), "slate-800", 1),
    shape(`M${pt(lx, farBot)}L${pt(rx, farBot)}L${pt(FAR_EDGE, FAR_EDGE)}L${pt(EDGE, FAR_EDGE)}Z`, "slate-400", 1),
    shape(walls, "wood-600", 1, MAIN),
    shape(lines, "wood-800", 0.8),
    shape(crates, "brass-600", 1),
    shape(rectPath(lx + 1, farTop + 3, gap * 2 - 2, farBot - farTop - 4), "slate-100", 0.3),
  ];
}

/** A sunken amphitheatre centred on the anchor: four tiered steps down to an oval floor, 80 wide and 26 tall at scale 1. */
function amphitheatrePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  for (let k = 0; k < 4; k += 1) {
    const rx = 40 - k * 7 + between(rng, -0.4, 0.4);
    const ry = 13 - k * 2.3;
    out.push(shape(f.ellipse(0, k * 1.4, rx, ry), k % 2 === 0 ? "stone-500" : "slate-400", 1, k % 2 === 0 ? MAIN : undefined));
  }
  out.push(shape(f.ellipse(0, 5.6, 12, 3.8), "slate-700", 1));
  out.push(shape(f.ellipse(-3, 4.8, 5, 1.2), "white", 0.12));
  return out;
}

/** A vast glass chessboard floating in perspective, centred on the anchor: 80 wide near, 40 far, 24 deep at scale 1. */
function chessboardPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = (dy: number): number => 20 + (20 * (dy + 12)) / 24;
  const cols = 6;
  const rows = 4;
  let dark = "";
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      if ((r + c) % 2 === 0) continue;
      const y0 = -12 + (24 * r) / rows;
      const y1 = -12 + (24 * (r + 1)) / rows;
      const u0 = -1 + (2 * c) / cols;
      const u1 = -1 + (2 * (c + 1)) / cols;
      dark += f.poly([[u0 * half(y0), y0], [u1 * half(y0), y0], [u1 * half(y1), y1], [u0 * half(y1), y1]]);
    }
  }
  const slide = between(rng, -6, 4);
  return [
    shape(f.ellipse(0, 19, 34, 3.5), "slate-900", 0.3),
    shape(f.poly([[-20, -12], [-20, -9], [-40, 15.2], [40, 15.2], [20, -9], [20, -12], [40, 12], [-40, 12]]), "slate-400", 1),
    shape(f.poly([[-20, -12], [20, -12], [40, 12], [-40, 12]]), "slate-100", 0.75, MAIN),
    shape(dark, "slate-700", 0.9),
    shape(f.poly([[slide, -12], [slide + 8, -12], [slide - 12, 12], [slide - 22, 12]]), "white", 0.16),
  ];
}

/** A flat-topped desert mesa standing on the anchor, 50 wide and 20 tall at scale 1. */
function mesaPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const knots: readonly (readonly [number, number])[] = [[0, 25], [-5, 21], [-12, 18.5], [-20, 16]];
  const hw = (y: number): number => {
    for (let i = 0; i + 1 < knots.length; i += 1) {
      const [y0, w0] = knots[i] ?? [0, 25];
      const [y1, w1] = knots[i + 1] ?? [0, 25];
      if (y <= y0 && y >= y1) return w0 + ((w1 - w0) * (y - y0)) / (y1 - y0);
    }
    return 16;
  };
  const band = (y0: number, y1: number): string => f.poly([[-hw(y0), y0], [hw(y0), y0], [hw(y1), y1], [-hw(y1), y1]]);
  const lip = between(rng, 0, 1);
  const outline: Point[] = [[-25, 0], [-21, -5], [-18.5, -12], [-16, -20], [16, -20], [18.5, -12], [21, -5], [25, 0]];
  return [
    shape(f.poly(outline), "orange-400", 1, MAIN_INKED),
    shape(band(-10.5 - lip, -7.5 - lip), "wood-600", 0.6),
    shape(band(-17, -13.5), "red-600", 0.6),
    shape(f.poly([[8, -20], [16, -20], [18.5, -12], [21, -5], [25, 0], [12, 0]]), "slate-900", 0.22),
    shape(f.poly([[-16, -20], [16, -20], [16.4, -18.6], [-16.4, -18.6]]), "white", 0.18),
  ];
}

/** A candy-striped mountain standing on the anchor, 50 wide and 36 tall at scale 1. */
function candyMountainPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const peak = between(rng, -2, 2);
  const outline: Point[] = [[-25, 0], [-14, -20], [-5 + peak * 0.5, -33], [peak, -36], [5 + peak * 0.5, -33], [14, -20], [25, 0]];
  const lean = 0.8;
  let stripes = "";
  const start = -32 + between(rng, 0, 3);
  for (let i = 0; i < 6; i += 1) {
    const u0 = start + i * 10;
    const strip = clipKitA(clipKitA(outline, 1, lean, u0), -1, -lean, -(u0 + 5));
    if (strip.length >= 3) stripes += f.poly(strip);
  }
  const shade = clipKitA(outline, 1, 0, 3);
  return [
    shape(f.poly(outline), "white", 1, MAIN_INKED),
    shape(stripes, "red-500", 1),
    shape(f.poly(shade), "slate-900", 0.15),
  ];
}

/** A floating island centred on the anchor, 60 wide, its rocky underside tapering 30 below to a point, roots and a thin waterfall. */
function floatingIslandPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const knots: readonly (readonly [number, number])[] = [[-3, 30], [6, 24], [16, 14], [24, 5], [30, 0]];
  const hw = (y: number): number => {
    for (let i = 0; i + 1 < knots.length; i += 1) {
      const [y0, w0] = knots[i] ?? [0, 0];
      const [y1, w1] = knots[i + 1] ?? [0, 0];
      if (y >= y0 && y <= y1) return w0 + ((w1 - w0) * (y - y0)) / (y1 - y0) - 0.2;
    }
    return 0;
  };
  const band = (y0: number, y1: number): string => f.poly([[-hw(y0), y0], [hw(y0), y0], [hw(y1), y1], [-hw(y1), y1]]);
  const tip = between(rng, -2, 2);
  const underside: Point[] = [[-30, -3], [-24, 6], [-14, 16], [-5, 24], [tip, 30], [5, 24], [14, 16], [24, 6], [30, -3]];
  let roots = "";
  for (const x of [-18, -6, 9]) {
    const y = x < 0 ? (x < -10 ? 11 : 19) : 19;
    const sway = between(rng, -2, 2);
    roots += f.band([[x, y], [x + sway, y + 4], [x + sway * 1.6, y + between(rng, 7, 10)]], 0.8);
  }
  return [
    shape(f.poly(underside), "slate-700", 1, INKED),
    shape(band(3, 5.2), "stone-500", 0.7),
    shape(band(11.5, 14), "stone-500", 0.6),
    shape(f.poly([[6, -3], [30, -3], [24, 6], [14, 16], [8, 22]]), "slate-900", 0.25),
    shape(roots, "wood-700", 1),
    shape(f.ellipse(0, -3, 30, 5), "green-700", 1, MAIN_INKED),
    shape(f.ellipse(-1.5, -4.4, 28.5, 3.4), "green-500", 0.8),
    shape(f.band([[26, -3], [27.4, 2], [27.8, 10], [27.4, 20]], 1.4), "sky-300", 0.85),
    shape(f.ellipse(27.4, 22, 2.4, 1), "white", 0.45),
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

/** An axis-aligned box in a part's frame, `(x, y)` its top-left corner. */
function archRect(f: Local, x: number, y: number, w: number, h: number): string {
  return f.poly([[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
}

/** An arched opening in a part's frame: its feet at `bottom`, its crown at `top`, `w` wide. */
function archDoor(f: Local, cx: number, bottom: number, w: number, top: number): string {
  const hw = w / 2;
  return (
    `M${f.p(cx - hw, bottom)}L${f.p(cx - hw, top + hw)}Q${f.p(cx - hw, top)} ${f.p(cx, top)}` +
    `Q${f.p(cx + hw, top)} ${f.p(cx + hw, top + hw)}L${f.p(cx + hw, bottom)}Z`
  );
}

/** A window with its sill on the anchor, 12 wide and 16 tall at scale 1. */
function windowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.4, 0.4);
  return [
    shape(archRect(f, -7, -1.6, 14, 1.6), "slate-100", 1, INKED),
    shape(archRect(f, -6, -16, 12, 14.4), "white", 1, INKED),
    shape(archRect(f, -4.8, -14.8, 9.6, 12), "sky-300", 0.6, MAIN),
    shape(archRect(f, -0.5, -14.8, 1, 12) + archRect(f, -4.8, -9.2 + j, 4.3, 1) + archRect(f, 0.5, -9.2 + j, 4.3, 1), "white", 0.95),
    shape(f.band([[-3.6, -4.4], [-0.6, -12.6]], 1.5) + f.band([[0.8, -4.4], [2.2, -8]], 0.7), "white", 0.35),
  ];
}

/** A door in its frame standing on the anchor, 9 wide and 16 tall at scale 1. */
function doorPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0, 0.5);
  return [
    shape(f.poly([[-4.5, 0], [-4.5, -16], [4.5, -16], [4.5, 0], [3.7, 0], [3.7, -15.2], [-3.7, -15.2], [-3.7, 0]]), "slate-100", 1, INKED),
    shape(archRect(f, -3.7, -15.2, 7.4, 15.2), "wood-600", 1, MAIN_INKED),
    shape(archRect(f, -2.6, -13.6, 5.2, 5.4 + lean) + archRect(f, -2.6, -6.8, 5.2, 5.4), "wood-700", 1),
    shape(f.circle(2.6, -7.5, 0.7), "brass-600", 1),
  ];
}

/** A wall shelf unit hung on the anchor's ground line, 24 wide and 20 tall at scale 1, three boards of stock. */
function shelfPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const floors = [-1.4, -8.2, -15];
  const kinds = [
    { w: 2.2, h: 3.5, d: "" },
    { w: 3, h: 2.5, d: "" },
    { w: 2.4, h: 2.2, d: "" },
    { w: 1.6, h: 4.5, d: "" },
  ];
  for (const floor of floors) {
    let x = -10.8;
    while (x < 10.4) {
      const k = kinds[whole(rng, 0, 3)] ?? kinds[0];
      if (k === undefined) break;
      if (x + k.w > 10.8) break;
      if (rng() < 0.85) {
        k.d += archRect(f, x, floor - k.h, k.w, k.h);
        x += k.w + between(rng, 0.2, 0.8);
      } else x += 1.4;
    }
  }
  return [
    shape(archRect(f, -12, -20, 1.1, 20) + archRect(f, 10.9, -20, 1.1, 20), "wood-800", 1, INKED),
    shape(archRect(f, -10.9, -1.4, 21.8, 1.4) + archRect(f, -10.9, -8.2, 21.8, 1.2) + archRect(f, -10.9, -15, 21.8, 1.2), "wood-700", 1, MAIN_INKED),
    shape(kinds[0]?.d || archRect(f, -9, -4.9, 2.2, 3.5), "sky-300", 0.85),
    shape(kinds[1]?.d || archRect(f, -5, -3.9, 3, 2.5), "brass-600", 1),
    shape(kinds[2]?.d || archRect(f, 0, -3.6, 2.4, 2.2), "red-600", 1),
    shape(kinds[3]?.d || archRect(f, 5, -5.9, 1.6, 4.5), "green-700", 1),
  ];
}

/** A shop counter standing on the anchor, 40 wide and 10 tall at scale 1. */
function counterPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let panels = "";
  for (let i = 0; i < 4; i += 1) panels += archRect(f, -18.6 + i * 9.4, -6.2, 8.6, 5);
  void rng;
  return [
    shape(archRect(f, -20, -10, 40, 2.6), "wood-600", 1, MAIN_INKED),
    shape(archRect(f, -20, -10, 40, 0.8), "brass-500", 0.4),
    shape(archRect(f, -19, -7.4, 38, 7.4), "wood-700", 1, INKED),
    shape(panels, "wood-800", 0.7),
  ];
}

/** A tidy whitewashed house standing on the anchor, 22 wide and 20 tall at scale 1. */
function housePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cx = rng() < 0.5 ? 5.4 : -5.4;
  return [
    shape(archRect(f, cx - 1.6, -20, 3.2, 8), "red-900", 1),
    shape(archRect(f, -9, -11.5, 18, 11.5), "slate-100", 1, MAIN_INKED),
    shape(archRect(f, 3, -11.5, 6, 11.5), "slate-900", 0.15),
    shape(f.poly([[-11, -10.8], [0, -19.6], [11, -10.8]]), "red-600", 1, INKED),
    shape(f.poly([[-10.4, -11.2], [0, -19], [0, -14.4], [-5, -11.2]]), "white", 0.22),
    shape(archRect(f, -1.7, -6.8, 3.4, 6.8), "wood-700", 1),
    shape(archRect(f, -7.4, -9, 3.6, 4.2) + archRect(f, 3.8, -9, 3.6, 4.2), "sky-300", 0.9),
  ];
}

/** A glass dome arching over the anchor, 50 wide and 30 tall at scale 1. */
function domePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, 28.5, 30);
  const rib = (k: number, stop: number): string => {
    const pts: Point[] = [];
    for (let i = 0; i <= 6; i += 1) {
      const t = (stop * i) / 6;
      pts.push([25 * k * Math.cos(t), -rise * Math.sin(t)]);
    }
    return f.band(pts, 0.8);
  };
  const half = Math.PI / 2;
  const outline: Point[] = [];
  for (let i = 0; i <= 10; i += 1) outline.push([-25 * Math.cos((Math.PI * i) / 10), -rise * Math.sin((Math.PI * i) / 10)]);
  const near = half * 0.83;
  const ribs = f.band(outline, 1) + rib(0.66, near) + rib(-0.66, near) + rib(0.33, near) + rib(-0.33, near) + f.band([[0, 0], [0, -rise * 0.9]], 0.8);
  return [
    shape(`M${f.p(-25, 0)}A${fmt(f.r(25))} ${fmt(f.r(rise))} 0 0 ${f.sweep(1)} ${f.p(25, 0)}Z`, "sky-300", 0.18),
    shape(ribs, "slate-100", 0.95, MAIN),
    shape(f.ellipse(0, 0, 26.5, 2), "slate-400", 0.95, INKED),
  ];
}

/** A low dry-stone wall standing on the anchor, 50 wide and 8 tall at scale 1. */
function stoneWallPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rows: readonly (readonly [number, number])[] = [[-8, -5.9], [-5.4, -3], [-2.5, 0]];
  let mortar = archRect(f, -25, -5.9, 50, 0.5) + archRect(f, -25, -3, 50, 0.5);
  let light = "";
  rows.forEach(([top, bottom]) => {
    let x = -25 + between(rng, 0, 3);
    while (x < 25) {
      const w = between(rng, 4.5, 8.5);
      const next = Math.min(25, x + w);
      if (next - x > 2) light += archRect(f, x + 0.5, top + 0.35, next - x - 1.2, 0.55);
      if (next < 25 - 1) mortar += archRect(f, next - 0.25, top, 0.5, bottom - top);
      x = next;
    }
  });
  const edge: Point[] = [[-25, -7.4], [-12, -8 + between(rng, -0.3, 0.3)], [0, -7.8], [12, -8 + between(rng, -0.3, 0.3)], [25, -7.4]];
  return [
    shape(f.poly([[-25, 0], ...edge, [25, 0]]), "stone-500", 1, MAIN_INKED),
    shape(light, "slate-400", 0.5),
    shape(mortar, "slate-700", 0.55),
    shape(archRect(f, -25, -1.4, 50, 1.4), "slate-900", 0.18),
  ];
}

/** A castle wall with merlons standing on the anchor, 50 wide and 16 tall at scale 1. */
function battlementPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 6;
  const mw = 5.4;
  const gap = (50 - n * mw) / (n - 1);
  const pts: Point[] = [[-25, 0]];
  let tops = "";
  for (let i = 0; i < n; i += 1) {
    const x = -25 + i * (mw + gap);
    pts.push([x, -12], [x, -16], [x + mw, -16], [x + mw, -12]);
    tops += archRect(f, x, -16, mw, 0.7);
    if (i < n - 1) pts.push([x + mw + gap, -12]);
  }
  pts.push([25, 0]);
  const slit = between(rng, -3, 3);
  return [
    shape(f.poly(pts), "stone-500", 1, MAIN_INKED),
    shape(archRect(f, -25, -3.2, 50, 3.2) + archRect(f, -25 + mw, -12, gap, 1.4), "slate-900", 0.25),
    shape(archRect(f, -25, -9.4, 50, 0.4) + archRect(f, -25, -6.4, 50, 0.4), "slate-700", 0.3),
    shape(tops, "white", 0.2),
    shape(archRect(f, slit - 0.6, -9, 1.2, 5), "black", 0.85),
  ];
}

/** A tall standing mirror on little feet, 12 wide and 26 tall at scale 1. */
function mirrorPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const o = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-6, -3)}L${f.p(-6, -21)}Q${f.p(-6, -26)} ${f.p(0, -26)}Q${f.p(6, -26)} ${f.p(6, -21)}L${f.p(6, -3)}Z`, "wood-700", 1, MAIN_INKED),
    shape(`M${f.p(-4.8, -4.2)}L${f.p(-4.8, -20.6)}Q${f.p(-4.8, -24.8)} ${f.p(0, -24.8)}Q${f.p(4.8, -24.8)} ${f.p(4.8, -20.6)}L${f.p(4.8, -4.2)}Z`, "slate-100", 0.95),
    shape(f.poly([[-4.8, -9 + o], [4.8, -17.5 + o], [4.8, -13.5 + o], [-4.8, -5.2 + o]]), "sky-300", 0.4),
    shape(f.band([[-3.6, -16.8], [-1.6, -20.8]], 1.1) + f.band([[-1.1, -14], [0.6, -17]], 0.6), "white", 0.6),
    shape(f.poly([[-6, -3], [-3, -3], [-4.2, 0], [-6.8, 0]]) + f.poly([[3, -3], [6, -3], [6.8, 0], [4.2, 0]]), "wood-700", 1, INKED),
  ];
}

/** A plinth standing on the anchor, 8 wide and 10 tall at scale 1. */
function pedestalPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  return [
    shape(archRect(f, -4, -10, 8, 1.8) + archRect(f, -2.8, -8.2, 5.6, 6.4) + archRect(f, -4, -1.8, 8, 1.8), "slate-100", 1, MAIN_INKED),
    shape(archRect(f, 1.2, -10, 2.8, 1.8) + archRect(f, 1.2, -8.2, 1.6, 6.4) + archRect(f, 1.2, -1.8, 2.8, 1.8), "slate-900", 0.25),
    shape(archRect(f, -2.8, -8.2, 1, 6.4), "white", 0.4),
  ];
}

/** A rope bridge from 30 left to 30 right of the anchor at the anchor's height, sagging 5 at the middle. */
function bridgePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = 5 * between(rng, 0.92, 1.08);
  const dip = (x: number): number => sag * (1 - (x / 30) ** 2);
  const slats = 20;
  let planks = "";
  for (let i = 0; i < slats; i += 1) {
    const x0 = -30 + i * 3 + 0.2;
    const x1 = x0 + 2.6;
    planks += f.poly([[x0, dip(x0)], [x1, dip(x1)], [x1, dip(x1) + 1.3], [x0, dip(x0) + 1.3]]);
  }
  const curve = (lift: number): Point[] => [-30, -22, -15, -7.5, 0, 7.5, 15, 22, 30].map((x) => [x, dip(x) - lift] as const);
  let posts = "";
  for (let i = 0; i <= 10; i += 1) {
    const x = -30 + i * 6;
    const x0 = Math.min(Math.max(x, -29.8), 29.6);
    posts += archRect(f, x0, dip(x0) - 7, 0.4, 7);
  }
  return [
    shape(planks, "wood-600", 1, MAIN_INKED),
    shape(f.band(curve(7), 0.8) + f.band(curve(3), 0.6), "wood-700", 1),
    shape(posts, "wood-700", 0.9),
  ];
}

/** A lighthouse standing on the anchor, 8 wide at its base and 36 tall at scale 1. */
function lighthousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = (y: number): number => 4 - 1.4 * (-y / 26);
  const band = (y0: number, y1: number): Point[] => [[-half(y0), y0], [-half(y1), y1], [half(y1), y1], [half(y0), y0]];
  const o = between(rng, -0.5, 0.5);
  let posts = "";
  for (let i = 0; i < 5; i += 1) posts += archRect(f, -4.1 + i * 2.05, -29, 0.4, 2.2);
  return [
    shape(f.ellipse(0, -31, 8.5, 8.5), "white", 0.12),
    shape(f.ellipse(0, -31, 5.5, 5.5), "orange-400", 0.22),
    shape(f.poly([[-4, 0], [-2.6, -26], [2.6, -26], [4, 0]]), "white", 1, MAIN_INKED),
    shape(f.poly(band(-4.5, -8.5 + o)) + f.poly(band(-13, -17 + o)) + f.poly(band(-21.5, -25)), "red-600", 1),
    shape(archRect(f, -4.4, -26.8, 8.8, 0.8) + posts + archRect(f, -4.4, -29.4, 8.8, 0.4), "slate-900", 0.85),
    shape(archRect(f, -2.2, -32.6, 4.4, 3.2), "orange-400", 0.6),
    shape(archRect(f, -1.2, -32, 2.4, 2), "white", 0.9),
    shape(f.poly([[-3, -32.6], [0, -36], [3, -32.6]]), "slate-900", 1, INKED),
  ];
}

/** A stone wishing well with a little roof standing on the anchor, 16 wide and 18 tall at scale 1. */
function wellPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drop = between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-6.4, 0], [-6.2, -6], [6.2, -6], [6.4, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.ellipse(0, -6, 6.2, 2), "slate-400", 1, INKED),
    shape(f.ellipse(0, -6, 4.8, 1.4), "slate-900", 0.95),
    shape(archRect(f, 0.9, -12.7, 0.3, 2.8 + drop) + f.poly([[-0.4, -9.9 + drop], [2.4, -9.9 + drop], [2, -8 + drop], [0, -8 + drop]]), "wood-600", 1),
    shape(archRect(f, -5.6, -15, 1.1, 9.2) + archRect(f, 4.5, -15, 1.1, 9.2), "wood-700", 1, INKED),
    shape(archRect(f, -4.5, -13.3, 9, 0.6) + archRect(f, 5.6, -13.3, 1.2, 0.6) + archRect(f, 6.2, -13.3, 0.6, 2.6), "brass-600", 1),
    shape(f.poly([[-8, -14.4], [0, -18], [8, -14.4]]), "wood-700", 1, INKED),
    shape(f.poly([[-7.4, -14.7], [0, -17.7], [0, -14.8]]), "white", 0.18),
  ];
}

/** A glass partition standing on the anchor, 30 wide and 26 tall at scale 1. */
function glassWallPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  return [
    shape(archRect(f, -15, -26, 30, 26), "sky-300", 0.15, MAIN),
    shape(
      archRect(f, -15, -26, 30, 1) + archRect(f, -15, -1, 30, 1) + archRect(f, -15, -25, 1, 24) + archRect(f, 14, -25, 1, 24) +
        archRect(f, -5.4, -25, 0.8, 24) + archRect(f, 4.6, -25, 0.8, 24) +
        archRect(f, -14, -13.6, 8.6, 0.8) + archRect(f, -4.6, -13.6, 9.2, 0.8) + archRect(f, 5.4, -13.6, 8.6, 0.8),
      "slate-400", 1, INKED,
    ),
    shape(f.band([[-12, -3], [-4, -23]], 2.2) + f.band([[-2, -3], [3, -15]], 1.2), "white", 0.25),
  ];
}

/** A city skyline across 80 centred on the anchor, 30 tall at scale 1, one tower with a spire. */
function skylinePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 7;
  const w = 80 / n;
  const spire = whole(rng, 2, 4);
  let towers = "";
  let lit = "";
  let dots = "";
  for (let i = 0; i < n; i += 1) {
    const x = -40 + i * w;
    const h = i === spire ? 22 : between(rng, 11, 21);
    towers += archRect(f, x, -h, w, h);
    lit += archRect(f, x, -h, w * 0.36, h);
    if (i === spire) towers += f.poly([[x + w / 2 - 0.5, -h], [x + w / 2, -30], [x + w / 2 + 0.5, -h]]);
    for (let row = 0; row < 3; row += 1) {
      for (let col = 0; col < 2; col += 1) {
        const dy = -h + 2.6 + row * 3.4;
        if (dy < -2 && rng() < 0.6) dots += archRect(f, x + 2.4 + col * (w * 0.45), dy, 1.1, 1.1);
      }
    }
  }
  return [
    shape(towers, "blue-800", 1, MAIN_INKED),
    shape(lit, "sky-300", 0.3),
    shape(dots || archRect(f, -30, -8, 1, 1), "white", 0.85),
  ];
}

/** A tall bookcase standing on the anchor, 20 wide and 34 tall at scale 1. */
function bookcasePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const colours: readonly PaletteName[] = ["red-600", "blue-800", "green-700", "brass-600", "slate-100"];
  const spines = colours.map(() => "");
  for (let row = 0; row < 4; row += 1) {
    const floor = -1 - 8.2 * row;
    let x = -8.6;
    while (x < 7.4) {
      const w = between(rng, 0.9, 1.7);
      if (rng() < 0.9) {
        const k = whole(rng, 0, 4);
        const h = between(rng, 4.6, 6.8);
        spines[k] = (spines[k] ?? "") + archRect(f, x, floor - h, w, h);
      }
      x += w + 0.1;
    }
  }
  return [
    shape(
      archRect(f, -10, -34, 1.2, 34) + archRect(f, 8.8, -34, 1.2, 34) + archRect(f, -8.8, -34, 17.6, 1.4) +
        [0, 1, 2, 3].map((row) => archRect(f, -8.8, -1 - 8.2 * row, 17.6, 1)).join(""),
      "wood-700", 1, MAIN_INKED,
    ),
    ...colours.map((c, k) => shape(spines[k] || archRect(f, 8, -1.5, 0.1, 0.1), c, 1)),
  ];
}

/** A greenhouse standing on the anchor, 70 wide and 40 tall at scale 1. */
function greenhousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const roofTop = (x: number): number => -22 - 18 * (1 - Math.abs(x) / 35);
  let bars = "";
  for (let i = 0; i < 8; i += 1) {
    const x = -35 + i * 10;
    const cx = i === 0 ? x + 0.4 : i === 7 ? x - 0.4 : x;
    bars += f.poly([[cx - 0.4, 0], [cx - 0.4, roofTop(cx) + 0.6], [cx + 0.4, roofTop(cx) + 0.6], [cx + 0.4, 0]]);
    if (i < 7) {
      const a = i === 0 ? x + 0.8 : x + 0.4;
      const b = i === 6 ? x + 10 - 0.8 : x + 10 - 0.4;
      bars += archRect(f, a, -22.4, b - a, 0.8) + archRect(f, a, -11.4, b - a, 0.8);
    }
  }
  const band = (sign: number): string =>
    f.band([[sign * 35, -22], [sign * 0.9, roofTop(0.9)]], 0.9);
  const lean = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-35, 0], [-35, -22], [0, -40], [35, -22], [35, 0]]), "sky-300", 0.15),
    shape(bars + band(-1) + band(1), "white", 1, MAIN),
    shape(f.poly([[-2, -38.8], [0, -40.8], [2, -38.8]]), "slate-400", 1),
    shape(archRect(f, -3.4, -13 - lean, 6.8, 13 + lean), "wood-700", 1),
    shape(archRect(f, -2.2, -11.6, 4.4, 5.4), "sky-300", 0.55),
  ];
}

/** A tall standing stone on the anchor, 5 wide and 12 tall at scale 1. */
function standingStonePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.25, 0.25);
  return [
    shape(f.poly([[-2.5, 0], [-2.2 + j(), -6], [-1.6 + j(), -10.5], [0.3, -12], [2 + j(), -10.2], [2.4 + j(), -5], [2.5, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-2.4, -0.5], [-2.1, -6], [-1.5, -10.3], [0.1, -11.6], [-0.2, -6], [-0.9, -0.5]]), "slate-100", 0.3),
    shape(f.poly([[0.9, -0.5], [1.6, -5], [1.5, -10], [2.4, -5], [2.5, 0]]), "slate-900", 0.22),
    shape(f.ellipse(1.1, -4.2, 1.1, 0.7) + f.ellipse(-0.7, -8.6, 0.8, 0.55) + f.ellipse(0, -0.2, 2.4, 0.6), "green-700", 0.85),
  ];
}

/** A distant fortress standing on the anchor, 40 wide and 22 tall at scale 1. */
function fortressPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const m = 6;
  const mw = 2.2;
  const gap = (22 - m * mw) / (m - 1);
  const wall: Point[] = [[-11, 0]];
  for (let i = 0; i < m; i += 1) {
    const x = -11 + i * (mw + gap);
    wall.push([x, -9], [x, -11], [x + mw, -11], [x + mw, -9]);
    if (i < m - 1) wall.push([x + mw + gap, -9]);
  }
  wall.push([11, 0]);
  const r = between(rng, -0.5, 0.5);
  return [
    shape(f.poly(wall) + archRect(f, -20, -15, 9, 15) + archRect(f, 11, -15, 9, 15), "slate-700", 1, MAIN_INKED),
    shape(archRect(f, -15.5, -15, 4.5, 15) + archRect(f, 15.5, -15, 4.5, 15), "slate-900", 0.28),
    shape(f.poly([[-21, -15], [-15.5, -19 + r], [-10, -15]]) + f.poly([[10, -15], [15.5, -19 + r], [21, -15]]), "slate-800", 1, INKED),
    shape(archDoor(f, 0, 0, 4.4, -6) + archRect(f, -15.8, -22, 0.4, 3.4) + archRect(f, 15.2, -22, 0.4, 3.4), "black", 0.9),
    shape(f.poly([[-15.4, -22], [-12.4, -21], [-15.4, -20]]) + f.poly([[15.6, -22], [18.6, -21], [15.6, -20]]), "red-600", 1),
  ];
}

/** A red barn standing on the anchor, 24 wide and 20 tall at scale 1. */
function barnPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  const leaf = (a: number, b: number, up: boolean): string =>
    up ? f.band([[a, -0.4], [b, -7]], 0.55) : f.band([[a, -7], [b, -0.4]], 0.55);
  return [
    shape(f.poly([[-10.5, 0], [-10.5, -9], [-8.5, -14], [0, -18.7], [8.5, -14], [10.5, -9], [10.5, 0]]), "red-600", 1, MAIN_INKED),
    shape(f.band([[-12, -8.6], [-9, -14.6], [0, -19.8], [9, -14.6], [12, -8.6]], 2.2), "slate-700", 1, INKED),
    shape(archRect(f, -3.4, -7.4, 6.8, 7.4), "red-900", 1),
    shape(archRect(f, -1.6, -16, 3.2, 3.8), "black", 0.85),
    shape(
      f.poly([[-4, 0], [-4, -8], [4, -8], [4, 0], [3.4, 0], [3.4, -7.4], [-3.4, -7.4], [-3.4, 0]]) + archRect(f, -0.3, -7.4, 0.6, 7.4) +
        f.poly([[-2.2, -11.6], [-2.2, -16.6], [2.2, -16.6], [2.2, -11.6], [1.6, -11.6], [1.6, -16], [-1.6, -16], [-1.6, -11.6]]),
      "white", 1,
    ),
    shape(leaf(-3.2, -0.6, true) + leaf(0.6, 3.2, true), "white", 0.9),
    shape(leaf(-3.2, -0.6, false) + leaf(0.6, 3.2, false), "white", 0.9),
  ];
}

/** A lakeside pavilion standing on the anchor, 30 wide and 26 tall at scale 1. */
function pavilionPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const o = between(rng, -0.4, 0.4);
  return [
    shape(archRect(f, -11, -14, 22, 11), "slate-900", 0.35),
    shape(archRect(f, -12.2, -16, 1.4, 13) + archRect(f, -4.7, -14.6, 1.4, 11.6) + archRect(f, 3.3, -14.6, 1.4, 11.6) + archRect(f, 10.8, -16, 1.4, 13), "red-900", 1),
    shape(archRect(f, -14, -3.2, 28, 1.6), "wood-600", 1, INKED),
    shape(archRect(f, -12.5, -1.6, 1, 1.6) + archRect(f, -3, -1.6, 1, 1.6) + archRect(f, 2, -1.6, 1, 1.6) + archRect(f, 11.5, -1.6, 1, 1.6), "wood-700", 1),
    shape(
      `M${f.p(-15, -18)}Q${f.p(-8, -14 + o)} ${f.p(0, -14 + o)}Q${f.p(8, -14 + o)} ${f.p(15, -18)}` +
        `Q${f.p(7, -19.5)} ${f.p(3, -24.5)}L${f.p(-3, -24.5)}Q${f.p(-7, -19.5)} ${f.p(-15, -18)}Z`,
      "red-600", 1, MAIN_INKED,
    ),
    shape(f.poly([[-2.5, -24], [2.5, -24], [4.5, -21.5], [-4.5, -21.5]]), "white", 0.25),
    shape(f.poly([[-0.7, -24.5], [0, -26], [0.7, -24.5]]), "red-900", 1),
  ];
}

/** A treehouse standing on the anchor, 34 wide and 40 tall at scale 1, a rope ladder hanging on its right. */
function treehousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const k = (): number => between(rng, -0.4, 0.4);
  let rungs = "";
  for (let y = -22; y < -3; y += 3.2) rungs += archRect(f, 7.6, y, 1.8, 0.5);
  return [
    shape(f.poly([[-5, 0], [-2.6, -6], [-2.2, -26], [2.2, -26], [2.6, -6], [5, 0]]), "wood-700", 1, INKED),
    shape(f.band([[-2.4, -14], [-9, -24.5]], 1.1) + f.band([[2.4, -14], [9, -24.5]], 1.1), "wood-700", 1),
    shape(archRect(f, -13, -26, 26, 1.6), "wood-600", 1, INKED),
    shape(archRect(f, -9, -26, 0.3, 1.6) + archRect(f, -4.5, -26, 0.3, 1.6) + archRect(f, 0, -26, 0.3, 1.6) + archRect(f, 4.5, -26, 0.3, 1.6) + archRect(f, 9, -26, 0.3, 1.6), "wood-800", 0.7),
    shape(archRect(f, -8, -34, 12, 8), "brass-600", 1, MAIN_INKED),
    shape(f.poly([[-9.5, -34], [-2, -39.5], [5.5, -34]]), "wood-800", 1, INKED),
    shape(archRect(f, -5, -32, 3.6, 3.4), "orange-400", 0.9),
    shape(f.circle(-14, -29, 5 + k()) + f.circle(-9.8, -37, 3.2 + k() * 0.4) + f.circle(11, -30, 5.5 + k()) + f.circle(9, -38.5, 3 + k() * 0.4), "green-700", 1, INKED),
    shape(f.circle(-15.5, -30.5, 2.2) + f.circle(9.5, -31.5, 2.4), "green-500", 0.5),
    shape(archRect(f, 7, -24.4, 0.4, 22) + archRect(f, 9.8, -24.4, 0.4, 22) + rungs, "brass-500", 0.9),
  ];
}

/** A skate quarter-pipe standing on the anchor, 26 wide and 12 tall at scale 1, its lip on the right. */
function rampPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  const arc = (t: number): Point => [-1 + 12 * Math.sin(t * DEGREES), -12 + 12 * Math.cos(t * DEGREES)];
  const [sx, sy] = arc(50);
  const rad = fmt(f.r(12));
  const hi: Point[] = [10, 25, 40, 55, 70, 82].map((t) => arc(t));
  return [
    shape(`M${f.p(-13, 0)}L${f.p(-1, 0)}A${rad} ${rad} 0 0 ${f.sweep(0)} ${f.p(11, -12)}L${f.p(13, -12)}L${f.p(13, 0)}Z`, "slate-400", 1, MAIN_INKED),
    shape(`M${f.p(sx, sy)}A${rad} ${rad} 0 0 ${f.sweep(0)} ${f.p(11, -12)}L${f.p(13, -12)}L${f.p(13, 0)}L${f.p(sx, 0)}Z`, "slate-900", 0.28),
    shape(archRect(f, 9.8, -12.9, 3.2, 0.9) + f.band(hi, 0.6), "slate-100", 0.9),
  ];
}

/** A volcanic forge standing on the anchor, 30 wide and 24 tall at scale 1. */
function forgePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.5, 0.5);
  return [
    shape(f.poly([[-15, 0], [-13.5, -11 + j()], [-9, -17 + j()], [-4, -18], [4, -18], [9, -17 + j()], [13.5, -11 + j()], [15, 0]]), "slate-800", 1, MAIN_INKED),
    shape(f.poly([[3, -18], [4, -24], [9, -24], [10, -17]]), "slate-700", 1, INKED),
    shape(f.ellipse(6.5, -24.5, 4.5, 2.2), "orange-400", 0.35),
    shape(f.ellipse(0, -5, 11, 9), "orange-400", 0.14),
    shape(archDoor(f, 0, 0, 12, -12), "orange-400", 1),
    shape(archDoor(f, 0, 0, 8.4, -9.4), "ember-500", 0.95),
    shape(f.ellipse(0, -3.6, 2.6, 3.4), "white", 0.85),
    shape(f.band([[-9, -13], [-9.4, -6]], 1) + f.band([[9, -12], [9.3, -3]], 1.1) + f.band([[-11.5, -8], [-11.6, -3.6]], 0.8) + f.circle(-9.4, -4.9, 0.8) + f.circle(9.3, -1.9, 0.8), "ember-500", 0.95),
  ];
}

/**
 * A twin-towered belfry standing on the anchor, 36 wide and 40 tall at scale 1. Its open bell chambers are
 * centred 22.5 above the anchor and 13 to each side: (-13, -22.5) and (13, -22.5) facing right.
 */
function belfryPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  const chamber = (cx: number): string => archDoor(f, cx, -19, 5.4, -26);
  return [
    shape(archRect(f, -18, -28, 10, 28) + archRect(f, 8, -28, 10, 28) + archRect(f, -8, -12, 16, 12), "slate-100", 1, MAIN_INKED),
    shape(archRect(f, -13, -28, 5, 28) + archRect(f, 13, -28, 5, 28) + archRect(f, 0, -12, 8, 12), "slate-900", 0.14),
    shape(chamber(-13) + chamber(13), "slate-900", 0.9),
    shape(f.poly([[-19, -28], [-13, -40], [-7, -28]]) + f.poly([[7, -28], [13, -40], [19, -28]]), "blue-800", 1, INKED),
    shape(f.poly([[-18.4, -28.4], [-13, -39], [-13, -28.4]]) + f.poly([[7.6, -28.4], [13, -39], [13, -28.4]]), "white", 0.2),
    shape(archDoor(f, 0, 0, 4.2, -7.6), "wood-700", 1),
  ];
}

const ROTUNDA_STEP = 11;
const ROTUNDA_REACH = 42;

/** The measure of the rotunda's doorway `i` (0 to 12, left to right): its offsets and size. */
function rotundaDoor(i: number): { x: number; base: number; w: number; top: number } {
  const phi = ((i - 6) * ROTUNDA_STEP * Math.PI) / 180;
  const c = Math.cos(phi);
  const base = -6 * c;
  return { x: ROTUNDA_REACH * Math.sin(phi), base, w: 6.3 * c, top: base - (8 + 12 * c) };
}

/** Offsets of the rotunda's 13 doorway centres from its anchor at scale 1, facing right, left to right. */
export const ROTUNDA_DOORS: readonly (readonly [number, number])[] = Array.from({ length: 13 }, (_, i) => {
  const d = rotundaDoor(i);
  return [Math.round(d.x * 10) / 10, Math.round(((d.base + d.top) / 2) * 10) / 10] as const;
});

/** A white marble rotunda round the anchor: a curved back wall of 13 arched doorways, 90 wide and 40 tall at scale 1. */
function rotundaPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 0.95, 1.05);
  const wallTop = (x: number): number => -30 + 6 * (x / 42) ** 2;
  const base: Point[] = [];
  for (let i = -90; i <= 90; i += ROTUNDA_STEP) {
    const phi = Math.abs(i) > 66 ? Math.sign(i) * 90 : i;
    base.push([ROTUNDA_REACH * Math.sin((phi * Math.PI) / 180), 0.9 - 6 * Math.cos((phi * Math.PI) / 180)]);
  }
  const edge: Point[] = [];
  for (let x = -42; x <= 42; x += 14) edge.push([x, wallTop(x)]);
  const doors: string[] = [];
  const glows: string[] = ["", "", "", "", ""];
  const group = [0, 1, 1, 2, 2, 3, 4];
  for (let i = 0; i < 13; i += 1) {
    const d = rotundaDoor(i);
    doors.push(archDoor(f, d.x, d.base, d.w, d.top));
    const g = group[Math.abs(i - 6)] ?? 0;
    glows[g] = (glows[g] ?? "") + archDoor(f, d.x, d.base, d.w * 0.7, d.top + 1.2);
  }
  const colours: readonly PaletteName[] = ["orange-400", "sky-300", "violet-300", "green-400", "cyan-400"];
  return [
    shape(f.ellipse(0, -2.5, 46, 5.5 * sag) + f.ellipse(0, -2.5, 36, 3.4), "stone-500", 1, INKED),
    shape(`M${f.p(-36, -25.6)}Q${f.p(0, -54.4)} ${f.p(36, -25.6)}Z`, "slate-100", 0.9, INKED),
    shape(`M${f.p(0, -40)}Q${f.p(18, -40)} ${f.p(36, -25.6)}L${f.p(0, -25.6)}Z`, "slate-900", 0.15),
    shape(f.poly([...base, ...edge.slice().reverse()]) + doors.join(""), "slate-100", 1, MAIN_INKED),
    shape(f.band(edge.map(([x, y]) => [x, y + 1.3] as const), 2.6), "slate-400", 0.9),
    ...colours.map((c, k) => shape(glows[k] ?? "", c, 0.8)),
  ];
}

/** A row of three townhouses at dusk standing on the anchor, 40 wide and 34 tall at scale 1, a cat-folk in each lit window. */
function townhousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 40 / 3;
  const heights = [between(rng, 19, 21), between(rng, 23, 25), between(rng, 20, 22)];
  const cat: readonly Point[] = [[-1.4, 2.3], [-1.3, 0.6], [-0.9, -0.3], [-1, -1.3], [-0.8, -2.1], [-0.2, -1.35], [0.2, -1.35], [0.8, -2.1], [1, -1.3], [0.9, -0.3], [1.3, 0.6], [1.4, 2.3]];
  let walls = "";
  let roofs = "";
  let windows = "";
  let cats = "";
  heights.forEach((h, i) => {
    const x = -20 + i * w;
    const cx = x + w / 2;
    walls += archRect(f, x, -h, w, h);
    roofs += f.poly([[x, -h], [cx, -h - (i === 1 ? 10 : 8)], [x + w, -h]]);
    for (const cy of [-6.2, -14.6]) {
      windows += archRect(f, cx - 1.7, cy - 2.3, 3.4, 4.6);
      cats += f.poly(cat.map(([dx, dy]) => [cx + dx, cy + dy] as const));
    }
  });
  return [
    shape(walls, "slate-700", 1, MAIN_INKED),
    shape(roofs, "slate-800", 1, INKED),
    shape(windows, "orange-400", 0.95),
    shape(cats, "black", 0.9),
    shape(archRect(f, -7, -22, 0.7, 22), "slate-400", 1),
  ];
}

/** A glass stadium of five stacked elliptical rings above the anchor, 80 wide and 50 tall at scale 1, dim below and blazing above. */
function stadiumPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tier = between(rng, 8.2, 8.6);
  const paints: readonly PaletteName[] = ["slate-700", "slate-400", "sky-300", "slate-100", "white"];
  const out: KitShape[] = [];
  for (let k = 0; k < 5; k += 1) {
    const rx = 40 - 6 * k;
    const ry = rx * 0.12;
    const yb = -ry - tier * k;
    const yt = yb - tier;
    const r = fmt(f.r(rx));
    const q = fmt(f.r(ry));
    const body =
      `M${f.p(-rx, yb)}A${r} ${q} 0 0 ${f.sweep(0)} ${f.p(rx, yb)}L${f.p(rx, yt)}A${r} ${q} 0 0 ${f.sweep(0)} ${f.p(-rx, yt)}Z`;
    const paint = paints[k] ?? "white";
    out.push(shape(body, paint, k === 4 ? 1 : 0.95, k === 0 ? MAIN_INKED : INKED));
    out.push(shape(f.ellipse(0, yt, rx, ry), k === 4 ? "blue-200" : "white", 0.3 + 0.1 * k));
  }
  out.push(shape(f.ellipse(0, -tier * 5 - 3, 14, 3.4), "white", 0.5));
  return out;
}

/** A frosted steel bastion standing on the anchor, 50 wide and 36 tall at scale 1, three overlapping glassy plates. */
function bastionPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const plates: readonly (readonly [number, number, number])[] = [
    [-15, 10, 30 + between(rng, -1, 1)],
    [15, 10, 27 + between(rng, -1, 1)],
    [0, 11, 36],
  ];
  const out: KitShape[] = [];
  let rivets = "";
  for (const [x, hw, h] of plates) {
    out.push(shape(f.poly([[x - hw, 0], [x - hw, -h + 5], [x - hw + 3, -h], [x + hw - 3, -h], [x + hw, -h + 5], [x + hw, 0]]), "slate-100", 0.92, MAIN_INKED));
    out.push(shape(f.poly([[x, 0], [x, -h], [x + hw - 3, -h], [x + hw, -h + 5], [x + hw, 0]]), "sky-300", 0.3));
    out.push(shape(archRect(f, x - hw, -h + 5, 1, h - 5) + f.poly([[x - hw, -h + 5], [x - hw + 3, -h], [x + hw - 3, -h], [x + hw - 3, -h + 1], [x - hw + 3.4, -h + 1], [x - hw + 0.7, -h + 5]]), "white", 0.9));
    rivets += f.circle(x - hw + 2.4, -3, 0.5) + f.circle(x - hw + 2.4, -h + 7, 0.5) + f.circle(x + hw - 2.4, -3, 0.5);
  }
  out.push(shape(rivets, "slate-400", 1));
  return out;
}

/** A warrior statue on a plinth standing on the anchor, 20 tall at scale 1, its spear on the right. */
function statuePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const o = between(rng, -0.3, 0.3);
  return [
    shape(archRect(f, -4.4, -3.4, 8.8, 0.8) + archRect(f, -3.6, -2.6, 7.2, 1.8) + archRect(f, -4.4, -0.8, 8.8, 0.8), "slate-400", 1, INKED),
    shape(
      f.poly([[-2.3, -3.4], [-2.1, -8.4], [-2.8, -11.2], [-2.5, -13.7], [-1.2, -14.2], [-1.4, -15.7], [-0.7, -17], [0.5, -17], [1.1, -15.7], [1, -14.2], [2.5, -13.7], [2.9, -11.2], [2.1, -8.4], [2.3, -3.4], [0.5, -3.4], [0.3, -7.6], [-0.3, -7.6], [-0.5, -3.4]]),
      "stone-500", 1, MAIN_INKED,
    ),
    shape(f.band([[2.4, -12.6], [4.35, -11.9 + o]], 0.9), "stone-500", 1, MAIN),
    shape(archRect(f, 4.1, -18.2, 0.5, 14.8) + f.poly([[3.6, -18.2], [4.35, -20], [5.1, -18.2]]), "slate-100", 1),
    shape(f.band([[-2.1, -8.4], [-2.8, -11.2], [-2.5, -13.7], [-1.2, -14.2]], 0.6), "white", 0.35),
  ];
}

/** A city tower bending flat like folded paper, standing on the anchor, 30 tall at scale 1, bending to the side it faces. */
function paperTowerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, 0.45, 0.6);
  const rise = Math.sqrt(1 - bend * bend);
  const up = (px: number, q: number): Point => [px + q * bend, -12 - q * rise];
  const len = 18 / rise;
  let windows = "";
  for (const cx of [-3.6, 1.6]) {
    for (const y of [-10.2, -6.6, -3]) windows += archRect(f, cx, y, 2, 1.8);
    for (const q of [3, 8.2, 13.4]) windows += f.poly([up(cx, q), up(cx + 2, q), up(cx + 2, q + 2.2), up(cx, q + 2.2)]);
  }
  return [
    shape(archRect(f, -5, -12, 10, 12) + f.poly([[-5, -12], [5, -12], up(5, len), up(-5, len)]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-5, -12], [5, -12], up(5, len), up(-5, len)]), "white", 0.26),
    shape(archRect(f, 0, -12, 5, 12), "slate-900", 0.22),
    shape(archRect(f, -5, -12.3, 10, 0.6), "slate-900", 0.6),
    shape(windows, "white", 0.6),
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

/** A fern standing on the anchor: five arching fronds, 8 tall at scale 1. */
function fernPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tips: readonly (readonly [number, number])[] = [[-7, -4.5], [7, -4.2], [-4, -7], [4, -7.2], [0, -8]];
  return tips.map(([tx, ty], i) => {
    const x = tx + between(rng, -0.6, 0.6);
    const y = ty * between(rng, 0.93, 1.05);
    const d = `M${f.p(0, 0)}Q${f.p(x * 0.2, y * 1.15)} ${f.p(x, y)}Q${f.p(x * 0.7, y * 0.7)} ${f.p(0, 0)}Z`;
    return i < 2 ? shape(d, "green-700", 1) : shape(d, i === 4 ? "green-500" : "green-700", 1, i === 4 ? MAIN : undefined);
  });
}

/** A pine tree standing on the anchor: three stacked triangles on a trunk, 18 tall at scale 1. */
function pinePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tiers: readonly (readonly [number, number, number])[] = [[-3, 6.2, -10], [-7, 5.2, -14], [-11, 4, -18]];
  const out: KitShape[] = [shape(f.poly([[-0.8, 0], [-0.8, -3.5], [0.8, -3.5], [0.8, 0]]), "wood-700", 1, INKED)];
  const lit: KitShape[] = [];
  for (const [base, hw, apex] of tiers) {
    const tip = between(rng, -0.4, 0.4);
    out.push(shape(f.poly([[-hw, base], [tip, apex], [hw, base]]), "green-900", 1, MAIN_INKED));
    lit.push(shape(f.poly([[-hw, base], [tip, apex], [tip, base]]), "green-700", 0.55));
  }
  return [...out, ...lit];
}

/** Reeds standing on the anchor with two cattails, 10 tall at scale 1. */
function reedPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let blades = "";
  for (const base of [-2, -0.7, 0.7, 2]) {
    const h = between(rng, 6.5, 9);
    blades += f.poly([[base - 0.35, 0], [base * 2.4 + between(rng, -0.4, 0.4), -h], [base + 0.35, 0]]);
  }
  const tall = between(rng, 9, 10.2);
  const short = tall - between(rng, 1.2, 2);
  return [
    shape(blades, "green-700", 1, MAIN),
    shape(f.band([[-1.4, 0], [-1.7, -short * 0.5], [-1.9, -short]], 0.4) + f.band([[1.4, 0], [1.8, -tall * 0.5], [2.1, -tall]], 0.4), "green-700", 1, MAIN),
    shape(f.ellipse(-1.9, -short - 1.1, 0.8, 2.2) + f.ellipse(2.1, -tall - 1.1, 0.8, 2.2), "wood-700", 1),
  ];
}

/** One tall pale flower standing on the anchor, 7 tall at scale 1. */
function flowerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.6, 0.6);
  let petals = "";
  for (let i = 0; i < 5; i += 1) {
    const a = ((i * 72 - 90) * Math.PI) / 180;
    petals += f.circle(lean + 1.8 * Math.cos(a), -5.8 + 1.8 * Math.sin(a), 1);
  }
  return [
    shape(f.band([[0, 0], [lean * 0.5, -3], [lean, -5.6]], 0.45), "green-500", 1),
    shape(`M${f.p(lean * 0.3, -2)}Q${f.p(2.6, -4)} ${f.p(3.6, -2.4)}Q${f.p(2, -1.4)} ${f.p(lean * 0.3, -2)}Z`, "green-500", 1),
    shape(petals, "white", 1, MAIN),
    shape(f.circle(lean, -5.8, 0.9), "orange-400", 1),
  ];
}

/** A clump of tall swaying grass standing on the anchor, 12 tall and 10 wide at scale 1. */
function tallGrassPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -1.2, 1.2);
  let light = "";
  let shade = "";
  for (let i = 0; i < 7; i += 1) {
    const base = (i - 3) * 1.3;
    const h = between(rng, 8, 12);
    const tip = base * 1.4 + lean * (h / 12) * 3 + between(rng, -0.3, 0.3);
    const d = f.poly([[base - 0.4, 0], [tip, -h], [base + 0.4, 0]]);
    if (i % 2 === 0) light += d;
    else shade += d;
  }
  return [shape(light, "brass-500", 1, MAIN), shape(shade, "brass-600", 1)];
}

/** A trimmed hedge block standing on the anchor, 20 wide and 10 tall at scale 1. */
function hedgePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const body = `M${f.p(-10, 0)}L${f.p(-10, -8.4)}Q${f.p(-10, -10)} ${f.p(-8.4, -10)}L${f.p(8.4, -10)}Q${f.p(10, -10)} ${f.p(10, -8.4)}L${f.p(10, 0)}Z`;
  let dots = "";
  for (let r = 0; r < 2; r += 1) {
    for (let c = 0; c < 5; c += 1) dots += f.circle(-8 + c * 4 + (r % 2) * 2 + between(rng, -0.5, 0.5), -3.2 - r * 3 + between(rng, -0.4, 0.4), 0.6);
  }
  return [
    shape(body, "green-700", 1, MAIN_INKED),
    shape(`M${f.p(-10, -6.8)}L${f.p(-10, -8.4)}Q${f.p(-10, -10)} ${f.p(-8.4, -10)}L${f.p(8.4, -10)}Q${f.p(10, -10)} ${f.p(10, -8.4)}L${f.p(10, -6.8)}Z`, "green-500", 0.85),
    shape(dots, "green-900", 0.6),
    shape(f.poly([[-10, -1.8], [10, -1.8], [10, 0], [-10, 0]]), "slate-900", 0.25),
  ];
}

/** A tree stump standing on the anchor, 8 wide and 5 tall at scale 1. */
function stumpPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wobble = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-4.4, 0], [-3.5, -1.3], [-3.4, -3.9], [3.4, -3.9], [3.5, -1.3], [4.4, 0]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-1.6, -0.4], [-1.5, -3]], 0.35) + f.band([[1.2, -0.4], [1.4 + wobble, -2.6]], 0.35), "wood-900", 0.45),
    shape(f.ellipse(0, -3.9, 3.5, 1.1), "brass-500", 1),
    shape(f.ellipse(0, -3.9, 2.5, 0.8) + f.ellipse(0, -3.9, 2.1, 0.66) + f.ellipse(0, -3.9, 1.2, 0.38) + f.ellipse(0, -3.9, 0.8, 0.24), "wood-600", 0.9),
  ];
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

/** A seagull, 5 wide, centred on the anchor: in flight with wings out, or perched standing on it (3 tall at scale 1). */
function gullPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() < 0.5) {
    const flap = between(rng, -0.8, 0.8);
    const wing = (side: number): string =>
      `M${f.p(side * 0.2, -0.2)}Q${f.p(side * 1.2, -1.8 + flap)} ${f.p(side * 2.5, -1.5 + flap)}Q${f.p(side * 1.5, -0.5)} ${f.p(side * 0.3, 0.4)}Z`;
    return [
      shape(wing(-1), "slate-400", 1),
      shape(wing(1), "slate-400", 0.9),
      shape(f.ellipse(0, 0.1, 1.2, 0.5), "white", 1, MAIN),
      shape(f.poly([[-1, 0], [-1.9, 0.2], [-1, 0.5]]), "white", 1, MAIN),
      shape(f.circle(1.3, -0.2, 0.38), "white", 1, MAIN),
      shape(f.poly([[1.6, -0.3], [2.3, -0.1], [1.6, 0.05]]), "orange-400", 1),
      shape(f.circle(1.4, -0.28, 0.08), "slate-900", 1),
    ];
  }
  return [
    shape(f.band([[0, -1], [0, 0]], 0.14) + f.band([[0.5, -1], [0.5, 0]], 0.14), "orange-400", 1),
    shape(f.poly([[-1.1, -1.7], [-2.5, -1.1], [-1, -1.1]]), "white", 1, MAIN),
    shape(f.ellipse(0, -1.6, 1.3, 0.75), "white", 1, MAIN),
    shape(f.ellipse(0.8, -2.1, 0.5, 0.8), "white", 1, MAIN),
    shape(f.circle(1.1, -2.7, 0.45), "white", 1, MAIN),
    shape(`M${f.p(-1.1, -1.9)}Q${f.p(-0.2, -2.4)} ${f.p(0.8, -1.8)}Q${f.p(-0.4, -1.2)} ${f.p(-2.2, -1.1)}Z`, "slate-400", 1),
    shape(f.poly([[1.5, -2.8], [2.3, -2.55], [1.5, -2.4]]), "orange-400", 1),
    shape(f.circle(1.25, -2.8, 0.09), "slate-900", 1),
  ];
}

/** A small house cat sitting on the anchor, facing the side the part faces, 6 tall at scale 1. */
function catPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-2.3, -0.4], [-3.4, -0.6], [-3.6 + curl, -2], [-2.8 + curl, -2.8]], 0.7), "orange-400", 1, MAIN),
    shape(`M${f.p(-2.4, 0)}Q${f.p(-2.6, -2.8)} ${f.p(-0.6, -3.4)}Q${f.p(1.8, -3.6)} ${f.p(2, -1.5)}Q${f.p(2.1, -0.4)} ${f.p(1.8, 0)}Z`, "orange-400", 1, MAIN_INKED),
    shape(f.circle(1.1, -4.3, 1.25), "orange-400", 1, MAIN_INKED),
    shape(f.poly([[0.1, -5.2], [0.3, -6.1], [1.1, -5.5]]) + f.poly([[1.4, -5.5], [2.2, -6.1], [2.4, -5.1]]), "orange-400", 1, MAIN),
    shape(f.ellipse(1.4, -1.8, 0.7, 1.3), "white", 0.95),
    shape(f.ellipse(1.9, -3.95, 0.65, 0.5), "white", 0.95),
    shape(f.ellipse(1.6, -0.15, 0.6, 0.25), "white", 0.95),
    shape(f.circle(1.0, -4.6, 0.26) + f.circle(1.85, -4.6, 0.26), "lime-400", 1),
    shape(f.ellipse(1.0, -4.6, 0.08, 0.2) + f.ellipse(1.85, -4.6, 0.08, 0.2), "slate-900", 1),
    shape(f.circle(2.35, -4.1, 0.15), "red-300", 1),
  ];
}

/** A fluffy, slightly puzzled sheep standing on the anchor, facing the side the part faces, 14 wide and 11 tall at scale 1. */
function sheepPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(
      [-3.8, -2.2, 2.6, 4.2].map((x) => f.band([[x, -3.6], [x, 0]], 1.1)).join(""),
      "black",
      1,
    ),
    shape(blob(f, rng, -0.5, -6, 4.4, 8), "white", 1, MAIN),
    shape(blob(f, rng, -3.2, -8.2, 3, 7), "white", 1, MAIN),
    shape(blob(f, rng, 1.8, -8.6, 3.2, 7), "white", 1, MAIN),
    shape(f.ellipse(-0.5, -3.5, 4.4, 0.9), "slate-400", 0.35),
    shape(f.ellipse(6, -6.3, 1.9, 2.3), "black", 1),
    shape(f.poly([[5.2, -7.8], [3.6, -9.4], [4.4, -7]]) + f.poly([[5, -6.4], [2.8, -5.8], [4.6, -5.3]]), "black", 1),
    shape(f.circle(5.4, -7, 0.55) + f.circle(7, -7, 0.55), "white", 1),
    shape(f.circle(5.3, -7.3, 0.25) + f.circle(6.9, -7.3, 0.25), "slate-900", 1),
    shape(f.band([[4.7, -8.1], [5.9, -8.4]], 0.3) + f.band([[6.4, -7.9], [7.5, -8.8]], 0.3), "slate-700", 1),
    shape(blob(f, rng, 5.8, -8.9, 1.1, 6), "white", 1, MAIN),
  ];
}

/** A bee, 3 wide, centred on the anchor, facing the side the part faces. */
function beePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flap = between(rng, -0.2, 0.2);
  return [
    shape(f.ellipse(-0.5, -1.1 + flap, 0.9, 0.4), "white", 0.6),
    shape(f.ellipse(0.5, -1.3 + flap, 0.8, 0.35), "white", 0.5),
    shape(f.ellipse(0, 0, 1.1, 0.8), "brass-500", 1, MAIN),
    shape(f.band([[-0.2, -0.65], [-0.2, 0.65]], 0.32) + f.band([[0.55, -0.55], [0.55, 0.55]], 0.3), "black", 1),
    shape(f.circle(1.25, 0.05, 0.45), "black", 1),
    shape(f.poly([[-1, -0.15], [-1.7, 0], [-1, 0.15]]), "black", 1),
  ];
}

/** A jewel-shelled scarab beetle, side-on and facing the side the part faces, standing on the anchor, 18 wide and 11 tall at scale 1. */
function scarabPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const feel = between(rng, -0.4, 0.4);
  const leg = (x: number, reach: number): string => f.band([[x, -3.4], [x + reach, -1.8], [x + reach * 1.5, 0]], 0.7);
  const far = (x: number): string => f.band([[x, -3.4], [x - 0.8, -1.6], [x - 1.4, 0]], 0.6);
  return [
    shape(far(-4) + far(0.5) + far(4.5), "slate-800", 1),
    shape(leg(-5, -1.2) + leg(-0.8, 0.2) + leg(3.4, 1.4), "slate-900", 1),
    shape(`M${f.p(-7, -3.5)}C${f.p(-7.4, -10)} ${f.p(4.6, -12.4)} ${f.p(6.6, -3.5)}Z`, "green-500", 1, MAIN_INKED),
    shape(f.poly([[-5.2, -4.4], [-4, -8], [-2, -4.4]]) + f.poly([[1.8, -4.4], [3, -8.2], [4.6, -4.4]]), "green-700", 0.6),
    shape(f.band([[-0.2, -9.6], [-0.6, -6.6], [-0.1, -3.6]], 0.4), "green-900", 0.8),
    shape(f.band([[-5.4, -6.6], [-3.6, -9.2], [-0.8, -10]], 0.8), "lime-200", 0.7),
    shape(f.ellipse(7.7, -4.6, 2, 1.7), "slate-900", 1),
    shape(f.poly([[9, -5.8], [10.8, -6.4], [9.6, -4.9]]) + f.poly([[9, -3.6], [10.8, -3], [9.6, -4.4]]), "slate-800", 1),
    shape(f.band([[8.4, -6], [10, -8.2 + feel], [11.6, -8.4 + feel]], 0.28) + f.band([[7.2, -6], [7.8, -8.6], [9.4, -9.6 + feel]], 0.28), "slate-900", 1),
  ];
}

/** A translucent green gelatin cube in three-quarter view standing on the anchor (front, top and right side), 30 wide and 30 tall at scale 1; its jagged-toothed mouth is on the front. */
function cubePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bite = between(rng, -0.5, 0.5);
  const top = (c: number): string => f.poly([[c - 1.3, -14], [c, -11.5 + bite], [c + 1.3, -14]]);
  const bottom = (c: number): string => f.poly([[c - 1.3, -8.5], [c, -11 - bite], [c + 1.3, -8.5]]);
  return [
    shape(f.poly([[-14.5, 0], [8.5, 0], [8.5, -23], [-14.5, -23]]), "green-400", 0.45, MAIN),
    shape(f.poly([[-14.5, -23], [8.5, -23], [15.5, -30], [-7.5, -30]]), "lime-200", 0.5),
    shape(f.poly([[8.5, 0], [15.5, -7], [15.5, -30], [8.5, -23]]), "green-700", 0.5),
    shape(f.poly([[-10.5, -14.2], [4.5, -14.2], [5, -8.3], [-11, -8.3]]), "slate-900", 0.55),
    shape([-8.3, -5.1, -1.9, 1.3].map(top).join(""), "white", 0.95),
    shape([-6.7, -3.5, -0.3, 2.9].map(bottom).join(""), "white", 0.95),
    shape(f.band([[-14, -1], [-14, -22.6], [8, -22.6]], 0.8), "white", 0.5),
    shape(f.circle(-9, -5, 1.4) + f.circle(-4, -18, 1) + f.circle(3, -4.5, 0.8) + f.circle(5, -19, 1.2) + f.circle(11.8, -12, 0.7), "lime-200", 0.5),
  ];
}

/** A sleek black panther bounding through the air, centred on the anchor and facing the side the part faces, 32 long and 14 tall at scale 1. */
function pantherPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const arch = between(rng, -0.5, 0.5);
  return [
    shape(f.band([[-9, -0.5], [-13, -2.5 + arch], [-15.5, -1.5], [-16, -5.5 - arch]], 0.9), "slate-900", 1, MAIN),
    shape(f.band([[-5.5, 1], [-10, 1.5], [-13, 2.5]], 1.8) + f.band([[5.5, 0.5], [10, 0.5], [14.5, 1.5]], 1.8), "slate-800", 1),
    shape(f.band([[-6.5, 1.5], [-11, 4], [-14, 6]], 2) + f.band([[6, 1.5], [11, 3.5], [15, 5.5]], 2), "slate-900", 1, MAIN),
    shape(
      `M${f.p(-9, 0)}Q${f.p(-8, -3.5)} ${f.p(-2, -3.8)}Q${f.p(5, -4.2)} ${f.p(8, -2.5)}Q${f.p(9, 0.5)} ${f.p(5, 2.4)}Q${f.p(-1, 3)} ${f.p(-5, 2.6)}Q${f.p(-9, 2.2)} ${f.p(-9, 0)}Z`,
      "slate-900",
      1,
      MAIN_INKED,
    ),
    shape(`M${f.p(9, -3.5)}Q${f.p(11, -5.5)} ${f.p(13.5, -4.4)}Q${f.p(16, -3.5)} ${f.p(16.2, -1.8)}Q${f.p(15, -0.5)} ${f.p(12, -0.7)}Q${f.p(9.5, -1)} ${f.p(9, -3.5)}Z`, "slate-900", 1, MAIN),
    shape(f.poly([[9.8, -4.6], [9.6, -6.9], [11.6, -5.3]]) + f.poly([[12, -5.1], [13.2, -6.8], [13.8, -4.6]]), "slate-900", 1, MAIN),
    shape(f.band([[-7, -3], [-2, -3.2], [4, -3.4], [7.5, -2.4]], 0.9), "blue-800", 0.8),
    shape(f.circle(13.6, -3.2, 0.45), "green-400", 1),
    shape(f.circle(16, -2.1, 0.35), "slate-700", 1),
  ];
}

/** A great sapphire dragon mother coiled and facing the side the part faces, standing on the anchor, 40 wide and 34 tall at scale 1. Her two forearms are held forward: the near claws are at (18.5, -11.6) and the far claws at (18, -14.6), and a recipe places two hatchlings standing on the forearms, their feet at offsets (15, -16) and (19, -15.8). */
function dragonPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const taper = (points: readonly Point[], widths: readonly number[]): string => {
    const left: string[] = [];
    const right: string[] = [];
    points.forEach(([x, y], i) => {
      const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
      const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [x, y];
      const length = Math.hypot(nx - px, ny - py) || 1;
      const half = (widths[i] ?? 1) / 2;
      left.push(f.p(x + (-(ny - py) / length) * half, y + ((nx - px) / length) * half));
      right.push(f.p(x - (-(ny - py) / length) * half, y - ((nx - px) / length) * half));
    });
    return `M${left.join("L")}L${right.reverse().join("L")}Z`;
  };
  const lift = between(rng, -0.6, 0.6);
  const claws = (hx: number, hy: number): string =>
    f.poly([[hx, hy - 1.2], [hx + 2.6, hy - 0.8], [hx, hy - 0.3]]) + f.poly([[hx, hy + 0.1], [hx + 2.8, hy + 0.6], [hx, hy + 1]]);
  return [
    shape(taper([[-8, -3], [-15, -1.5], [-20, -5], [-20, -11], [-16, -14.5 + lift]], [5.5, 4.6, 3.6, 2.4, 0.4]), "blue-700", 1, MAIN),
    shape(f.ellipse(0, -10, 14, 10), "blue-700", 1, MAIN_INKED),
    shape(taper([[7, -14], [10, -20], [10, -25.5]], [8, 6, 5]), "blue-700", 1, MAIN),
    shape(`M${f.p(-11, -13)}Q${f.p(-12, -25)} ${f.p(-5, -30)}Q${f.p(-1, -25)} ${f.p(2, -21)}Q${f.p(4, -17)} ${f.p(3, -12)}Q${f.p(-4, -14)} ${f.p(-11, -13)}Z`, "blue-800", 1),
    shape(f.band([[-5, -29], [-7, -20], [-8, -14]], 0.7) + f.band([[-3, -26], [0, -22], [3, -14]], 0.7), "blue-900", 0.6),
    shape(f.band([[12.5, -23], [11.5, -17], [10, -11], [5, -4], [-3, -1.5]], 3.2), "sky-300", 0.95),
    shape(`M${f.p(8, -28)}Q${f.p(9, -31)} ${f.p(12, -31)}Q${f.p(15, -30.5)} ${f.p(15.5, -27.5)}Q${f.p(20, -27)} ${f.p(20.5, -24.5)}Q${f.p(20, -22.5)} ${f.p(16, -22.5)}Q${f.p(12, -21.5)} ${f.p(9, -23)}Q${f.p(7, -25)} ${f.p(8, -28)}Z`, "blue-700", 1, MAIN_INKED),
    shape(f.poly([[9, -30], [12, -31], [5, -35]]) + f.poly([[12.5, -31], [14.5, -30.2], [10, -34]]), "slate-400", 1),
    shape(f.circle(12.8, -27.4, 1), "orange-400", 1),
    shape(f.ellipse(13, -27.4, 0.4, 0.8), "slate-900", 1),
    shape(f.band([[7, -12], [13, -11.8], [18.5, -11.6]], 2.6) + f.band([[8, -15.2], [13, -14.8], [18, -14.6]], 2.6), "blue-700", 1, MAIN),
    shape(claws(18.5, -11.6) + claws(18, -14.6), "slate-100", 1),
  ];
}

/** A baby dragon standing on the anchor, facing the side the part faces, 6 tall at scale 1. */
function hatchlingPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tail = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-1.2, -1], [-2.8, -0.6], [-3.4, -1.8 + tail]], 0.6), "blue-600", 1, MAIN),
    shape(f.poly([[-0.8, -3.2], [-2.6, -4.6], [-1.6, -2.4]]), "blue-800", 1),
    shape(f.ellipse(0, -2.2, 1.6, 1.7), "blue-600", 1, MAIN_INKED),
    shape(f.ellipse(0.6, -2, 0.9, 1.3), "sky-300", 0.95),
    shape(f.ellipse(0.5, -0.2, 0.7, 0.28) + f.ellipse(-0.9, -0.2, 0.7, 0.28), "blue-700", 1),
    shape(f.circle(0.9, -4.6, 1.5), "blue-600", 1, MAIN_INKED),
    shape(f.poly([[0.1, -5.9], [-0.3, -6.7], [0.8, -6.1]]) + f.poly([[1.3, -6.1], [1.9, -6.7], [2, -5.7]]), "slate-100", 1),
    shape(f.circle(1.5, -4.8, 0.6), "white", 1),
    shape(f.circle(1.7, -4.8, 0.32), "slate-900", 1),
  ];
}

/** A plump velvet pillow bristling with steel spikes, standing on the anchor, 24 wide and 14 tall at scale 1. */
function spikyPillowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let topSpikes = "";
  let shade = "";
  for (let i = 0; i < 6; i += 1) {
    const x = -9 + 3.6 * i;
    const tip = -14 + between(rng, -0.4, 0.4);
    topSpikes += f.poly([[x - 1.2, -11], [x, tip], [x + 1.2, -11]]);
    shade += f.poly([[x, -11], [x, tip], [x + 1.2, -11]]);
  }
  let side = "";
  for (const s of [-1, 1]) for (const y of [-4.5, -8]) side += f.poly([[s * 10.2, y - 1], [s * 12.6, y + between(rng, -0.3, 0.3)], [s * 10.2, y + 1]]);
  return [
    shape(`M${f.p(-10.5, -1)}Q${f.p(-11.5, -6)} ${f.p(-10.2, -10)}Q${f.p(-5, -12.2)} ${f.p(0, -11.4)}Q${f.p(5, -12.2)} ${f.p(10.2, -10)}Q${f.p(11.5, -6)} ${f.p(10.5, -1)}Q${f.p(0, 1)} ${f.p(-10.5, -1)}Z`, "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(-9.5, -9.5)}Q${f.p(-5, -11.6)} ${f.p(0, -10.9)}Q${f.p(5, -11.6)} ${f.p(9.5, -9.5)}Q${f.p(0, -7.5)} ${f.p(-9.5, -9.5)}Z`, "violet-500", 0.7),
    shape(f.circle(-10.8, -0.6, 1) + f.circle(10.8, -0.6, 1) + f.circle(-10.4, -10.2, 0.9) + f.circle(10.4, -10.2, 0.9), "brass-600", 1),
    shape(topSpikes + side, "slate-100", 1),
    shape(shade, "slate-400", 0.9),
    shape(f.ellipse(-5, -6.5, 3, 1.2), "white", 0.25),
  ];
}

/** A literal boulder with a weathered raised-eyebrow face and a tiny apron, standing on the anchor, 30 wide and 26 tall at scale 1. */
function boulderPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-15, 0)}Q${f.p(-16.5, -10)} ${f.p(-10, -19)}Q${f.p(-3, -26 + j)} ${f.p(6, -24)}Q${f.p(14, -20)} ${f.p(15.2, -9)}Q${f.p(15.5, -3)} ${f.p(14, 0)}Z`, "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(-12, -17)}Q${f.p(-7, -23.5)} ${f.p(2, -24)}Q${f.p(8, -24)} ${f.p(11, -21)}Q${f.p(4, -21.5)} ${f.p(-3, -19)}Q${f.p(-8, -17.5)} ${f.p(-12, -17)}Z`, "slate-100", 0.35),
    shape(f.band([[9, -23], [8.5, -18], [10.5, -14], [9.5, -10]], 0.6) + f.band([[-13, -9], [-10.5, -6], [-11.5, -2]], 0.6) + f.band([[11, -6], [8, -4], [9, -1.5]], 0.6), "slate-900", 0.45),
    shape(f.ellipse(-5, -14.5, 2.1, 1.7) + f.ellipse(4.5, -14.5, 2.1, 1.7), "white", 0.95),
    shape(f.circle(-4.4, -14.3, 0.8) + f.circle(5.1, -14.3, 0.8), "slate-900", 1),
    shape(f.band([[-8.5, -17.8], [-1.8, -17.8]], 1.3) + f.band([[1.8, -19.5], [5.5, -22.3], [9, -21.3]], 1.3), "slate-900", 0.85),
    shape(f.band([[-5, -9], [0, -8.4], [5, -9.2], [7.2, -11]], 0.9), "slate-900", 0.9),
    shape(f.poly([[-5, -7], [5, -7], [6.6, -0.4], [-6.6, -0.4]]), "white", 0.95),
    shape(f.poly([[-2.3, -4.2], [2.3, -4.2], [2.3, -2], [-2.3, -2]]), "slate-400", 0.5),
    shape(f.band([[-5, -6.4], [-14.5, -6.8]], 0.5) + f.band([[5, -6.4], [14.5, -6.8]], 0.5), "white", 0.9),
  ];
}

/** A plump crusty loaf with stubby legs standing on the anchor, facing the side the part faces, 18 wide and 12 tall at scale 1. */
function loafPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let flour = "";
  for (let i = 0; i < 5; i += 1) flour += f.circle(-7 + 3.4 * i + between(rng, -0.4, 0.4), -4.6 + between(rng, -0.6, 0.6), 0.28);
  return [
    shape([-6.5, -3.2, 3.2, 6.5].map((x) => f.band([[x, -3.2], [x, 0]], 1.8)).join(""), "wood-700", 1),
    shape(`M${f.p(-9, -3)}Q${f.p(-10, -10)} ${f.p(-4, -11.5)}Q${f.p(0, -12.4)} ${f.p(4, -11.5)}Q${f.p(10, -10)} ${f.p(9, -3)}Q${f.p(0, -1.5)} ${f.p(-9, -3)}Z`, "brass-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -9.2, 6.2, 2.2), "brass-500", 0.95),
    shape(f.band([[-3.8, -10.6], [-2.6, -8]], 0.5) + f.band([[-0.6, -11], [0.6, -8.2]], 0.5) + f.band([[2.6, -10.6], [3.8, -8]], 0.5), "brass-800", 0.5),
    shape(f.circle(-2.4, -6, 0.6) + f.circle(2.4, -6, 0.6), "slate-900", 1),
    shape(f.circle(-2.2, -6.2, 0.2) + f.circle(2.6, -6.2, 0.2), "white", 0.9),
    shape(f.band([[-1.8, -4.4], [-0.8, -3.7], [0.8, -3.7], [1.8, -4.4]], 0.38), "slate-900", 0.9),
    shape(f.circle(-4.6, -4.6, 0.8) + f.circle(4.6, -4.6, 0.8), "red-300", 0.5),
    shape(flour, "white", 0.8),
  ];
}

/** A small round-bodied sprinter with springy legs and a headband, running toward the side the part faces, standing on the anchor, 12 tall at scale 1. */
function rushTokenPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flutter = between(rng, -0.5, 0.5);
  const spring = (side: number): string =>
    f.band([[side * 1.2, -3.4], [side * 2.4, -2.8], [side * 1, -2.2], [side * 2.6, -1.6], [side * 1.6, -1], [side * 3.2, 0]], 0.5);
  return [
    shape(spring(-1) + spring(1), "slate-400", 1),
    shape(f.circle(0.3, -7.6, 4.3), "red-500", 1, MAIN_INKED),
    shape(f.ellipse(-1.6, -8.6, 1.3, 2.3), "white", 0.25),
    shape(f.band([[-3.6, -9.4], [0, -11], [3.8, -9.2]], 1.1), "white", 1),
    shape(f.poly([[-3.3, -9.8], [-6.4, -11 + flutter], [-5.7, -9.9]]) + f.poly([[-3.3, -9.1], [-6.8, -8.7 + flutter], [-5.6, -8.2]]), "white", 1),
    shape(f.ellipse(1.6, -7.9, 0.4, 0.55) + f.ellipse(3.3, -7.7, 0.4, 0.55), "slate-900", 1),
    shape(f.band([[0.9, -9], [2.2, -8.5]], 0.35) + f.band([[2.7, -8.8], [4.1, -8.2]], 0.35), "slate-900", 1),
    shape(f.band([[1.8, -6], [3.8, -5.2], [5, -6.6]], 0.9) + f.band([[-2.5, -6], [-4.4, -4.8], [-5.2, -6.2]], 0.9), "red-500", 1, MAIN),
  ];
}

/** A horned four-legged beast standing on the anchor, facing the side the part faces, 18 wide and 12 tall at scale 1. */
function beastPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swish = between(rng, -1, 1);
  return [
    shape(f.band([[-5.6, -7], [-7.6, -6 + swish], [-8.4, -4.6 + swish]], 0.9), "wood-700", 1, MAIN),
    shape(f.band([[-3.6, -4], [-3.8, 0]], 1.5) + f.band([[3.2, -4], [3.4, 0]], 1.5), "wood-800", 1),
    shape(f.ellipse(0, -6, 6, 3.2), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-4.9, -4], [-5, 0]], 1.5) + f.band([[4.6, -4], [4.8, 0]], 1.5), "wood-700", 1, MAIN),
    shape(f.ellipse(7, -8, 2.5, 2.1), "wood-700", 1, MAIN_INKED),
    shape(f.ellipse(9, -7.1, 1.5, 1.1), "wood-600", 1),
    shape(f.band([[6.2, -9.6], [5.6, -11.4], [7.4, -12.2]], 0.7) + f.band([[7.8, -9.7], [8.8, -11.4], [10.4, -11.2]], 0.7), "slate-100", 1),
    shape(f.circle(7.8, -8.4, 0.3), "orange-400", 1),
  ];
}

/** A small green goblin tumbling, centred on the anchor, 8 tall at scale 1 (tilted by the seed). */
function goblinPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -40, 40);
  const P = (x: number, y: number): Point => turned([x, y], tilt);
  const band = (points: readonly Point[], width: number): string => f.band(points.map(([x, y]) => P(x, y)), width);
  const poly = (points: readonly Point[]): string => f.poly(points.map(([x, y]) => P(x, y)));
  const ball = (x: number, y: number, r: number): string => f.circle(...P(x, y), r);
  return [
    shape(
      band([[-0.9, -0.3], [-2.3, -1.6], [-3, -2.8]], 0.6) + band([[0.9, -0.3], [2.4, 0.4], [3.4, -0.6]], 0.6) +
        band([[-0.5, 1.2], [-1.4, 2.6], [-0.6, 3.6]], 0.6) + band([[0.5, 1.2], [1.8, 2.2], [3, 1.6]], 0.6),
      "green-500",
      1,
      MAIN,
    ),
    shape(poly([[-1.3, -0.8], [1.3, -0.8], [1.6, 1.6], [-1.6, 1.6]]), "slate-800", 1),
    shape(poly([[-1, -2.4], [-3.8, -3.6], [-1.2, -1.4]]) + poly([[1, -2.4], [3.8, -3.6], [1.2, -1.4]]), "green-500", 1, MAIN),
    shape(ball(0, -2.3, 1.5), "green-500", 1, MAIN_INKED),
    shape(ball(-0.55, -2.5, 0.28) + ball(0.55, -2.5, 0.28), "slate-900", 1),
    shape(band([[-0.6, -1.7], [0, -1.45], [0.6, -1.7]], 0.2), "slate-900", 1),
  ];
}

/** A fluffy white snow bunny with hypnotic swirl eyes sitting on the anchor, facing the side the part faces, 14 tall at scale 1. */
function bunnyPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = between(rng, 0, 90);
  const swirl = (cx: number, cy: number): string => {
    const points: Point[] = [];
    for (let i = 0; i <= 10; i += 1) {
      const t = i / 10;
      const angle = (turn + t * 2.2 * FULL_TURN) * DEGREES;
      points.push([cx + 0.15 + t * 0.75 * Math.cos(angle), cy + t * 0.75 * Math.sin(angle)]);
    }
    return f.band(points, 0.2);
  };
  return [
    shape(f.circle(-4.2, -3.6, 1.3), "slate-100", 0.95),
    shape(f.ellipse(0, -3.5, 4, 3.5), "white", 1, MAIN_INKED),
    shape(f.ellipse(0, -1, 3, 1.2), "slate-400", 0.25),
    shape(f.ellipse(2.4, -0.3, 1.5, 0.6), "white", 1, MAIN_INKED),
    shape(`M${f.p(0, -10.6)}Q${f.p(-1.8, -13.5)} ${f.p(-0.4, -14)}Q${f.p(1.4, -13.2)} ${f.p(1.6, -10.6)}Z`, "white", 1, MAIN_INKED),
    shape(`M${f.p(2.2, -10.8)}Q${f.p(2.4, -13.8)} ${f.p(3.8, -13.6)}Q${f.p(4.8, -12.4)} ${f.p(3.8, -10.4)}Z`, "white", 1, MAIN_INKED),
    shape(f.circle(1.6, -8, 3), "white", 1, MAIN_INKED),
    shape(`M${f.p(0.2, -11)}Q${f.p(-0.9, -12.8)} ${f.p(-0.3, -13)}Q${f.p(0.8, -12.4)} ${f.p(1, -11)}Z`, "red-300", 0.9),
    shape(swirl(0.6, -8.4) + swirl(2.9, -8.4), "violet-500", 1),
    shape(f.ellipse(4.3, -7.2, 0.45, 0.32), "red-300", 1),
  ];
}

/** A grinning brown bear bursting up, its upper body standing on the anchor, facing the viewer, 26 wide and 26 tall at scale 1. */
function bearPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.5, 0.5);
  const clawSet = (s: number): string =>
    [10.4, 11.9, 13.3].map((c) => f.poly([[s * (c - 0.55), -24.3], [s * (c + 0.1), -26.4 + reach * 0.3], [s * (c + 0.55), -24.3]])).join("");
  const teeth = [-2.4, -0.9, 0.9, 2.4].map((c) => f.poly([[c - 0.7, -13.6], [c, -12.4], [c + 0.7, -13.6]])).join("");
  return [
    shape(f.band([[-8, -9], [-11.4, -15], [-11.8, -20]], 4.6) + f.band([[8, -9], [11.4, -15], [11.8, -20]], 4.6), "wood-600", 1, MAIN),
    shape(f.ellipse(0, -8, 9, 9.5), "wood-600", 1, MAIN_INKED),
    shape(f.circle(-4.6, -21.4, 2) + f.circle(4.6, -21.4, 2), "wood-600", 1, MAIN),
    shape(f.circle(0, -16.5, 5.6), "wood-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -14.4, 3.3, 2.5), "brass-500", 1),
    shape(f.ellipse(0, -15.7, 1.1, 0.75), "slate-900", 1),
    shape(`M${f.p(-3.2, -13.8)}Q${f.p(0, -9.8)} ${f.p(3.2, -13.8)}Q${f.p(0, -13.2)} ${f.p(-3.2, -13.8)}Z`, "slate-900", 1),
    shape(teeth, "white", 1),
    shape(f.circle(-2.4, -18.2, 0.75) + f.circle(2.4, -18.2, 0.75), "slate-900", 1),
    shape(f.circle(-11.8, -21.8, 2.6) + f.circle(11.8, -21.8, 2.6), "wood-600", 1, MAIN),
    shape(clawSet(-1) + clawSet(1), "slate-100", 1),
  ];
}

/** A packed concert crowd spanning 60 about the anchor (30 each way), standing on it, about 14 tall at scale 1: heads and raised arms in two rows, light sticks above. */
function crowdPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let heads2 = "";
  let arms2 = "";
  let heads = "";
  let arms = "";
  let blue = "";
  let red = "";
  for (let i = 0; i < 17; i += 1) heads2 += f.circle(-29 + 3.7 * i + between(rng, -0.4, 0.4), -9 + between(rng, -0.6, 0.6), 1.3);
  for (let i = 0; i < 8; i += 1) {
    const x = -27 + 7.7 * i + between(rng, -0.8, 0.8);
    const top = -15.5 - between(rng, 0, 1.5);
    arms2 += f.band([[x, -8], [x + between(rng, -0.8, 0.8), top]], 0.9);
  }
  for (let i = 0; i < 18; i += 1) heads += f.circle(-28.5 + 3.4 * i + between(rng, -0.4, 0.4), -5.6 + between(rng, -0.7, 0.7), 1.4);
  for (let i = 0; i < 9; i += 1) {
    const x = -28 + 7 * i + between(rng, -0.8, 0.8);
    const top = -11.5 - between(rng, 0, 2);
    const lean = between(rng, -1, 1);
    arms += f.band([[x, -3], [x + lean, top]], 1);
    const dot = f.circle(x + lean, top - 0.9, 0.55);
    if (i % 2 === 0) blue += dot;
    else red += dot;
  }
  return [
    shape(heads2, "slate-800", 1),
    shape(arms2, "slate-800", 1),
    shape(f.poly([[-30, 0], [-30, -3.6], [-15, -4.2], [0, -3.8], [15, -4.2], [30, -3.6], [30, 0]]), "slate-900", 1, MAIN),
    shape(heads, "slate-900", 1, MAIN),
    shape(arms, "slate-900", 1, MAIN),
    shape(blue, "sky-300", 0.95),
    shape(red, "red-300", 0.95),
  ];
}

/** A small simple soldier standing on the anchor, facing the side the part faces, 14 tall (9 when crouched) at scale 1; with a spear upright or levelled forward. */
function soldierPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() < 0.5) {
    return [
      shape(f.band([[-1, -5.5], [-1.2, 0]], 1.5) + f.band([[1, -5.5], [1.2, 0]], 1.5), "slate-700", 1),
      shape(f.poly([[-2.2, -5.8], [2.2, -5.8], [2.6, -10.5], [-2.6, -10.5]]), "blue-700", 1, MAIN_INKED),
      shape(f.band([[2.2, -9.5], [3.4, -7.8], [3, -6.6]], 1), "blue-700", 1, MAIN),
      shape(f.band([[3, -0.5], [3, -16]], 0.5), "wood-700", 1),
      shape(f.poly([[2.4, -16], [3, -18], [3.6, -16]]), "slate-100", 1),
      shape(f.circle(0.2, -11.8, 1.3), "red-300", 1),
      shape(`M${f.p(-1.6, -11.8)}Q${f.p(-1.6, -14.2)} ${f.p(0.2, -14.2)}Q${f.p(2.1, -14.2)} ${f.p(2.1, -11.8)}L${f.p(2.6, -11.5)}L${f.p(-2.1, -11.5)}Z`, "slate-400", 1),
    ];
  }
  return [
    shape(f.band([[-1.6, -3], [-3.2, -1.6], [-2.4, 0]], 1.5) + f.band([[1, -3], [2.8, -3.2], [2.4, 0]], 1.5), "slate-700", 1),
    shape(f.poly([[-2.4, -3.6], [2, -3.6], [3.4, -7], [-1, -7]]), "blue-700", 1, MAIN_INKED),
    shape(f.band([[2.8, -6.2], [4.4, -5], [5.6, -4.8]], 1), "blue-700", 1, MAIN),
    shape(f.band([[-2, -4.4], [11, -5.2]], 0.5), "wood-700", 1),
    shape(f.poly([[11, -6.1], [13, -5.3], [11, -4.5]]), "slate-100", 1),
    shape(f.circle(2.3, -8, 1.3), "red-300", 1),
    shape(`M${f.p(0.7, -8)}Q${f.p(0.7, -10.2)} ${f.p(2.3, -10.2)}Q${f.p(4, -10.2)} ${f.p(4, -8)}L${f.p(4.4, -7.7)}L${f.p(0.3, -7.7)}Z`, "slate-400", 1),
  ];
}

/** The same small soldier lying fallen on the anchor, head to the side the part faces, 14 long at scale 1, helmet off beside it. */
function soldierFallenPart(at: PartPlace): KitShape[] {
  const f = local(at);
  return [
    shape(f.band([[-7, -0.5], [-3, -0.4]], 1.2), "slate-700", 1),
    shape(f.poly([[-3.4, -0.1], [-3.2, -2.5], [2.6, -2.7], [2.8, -0.1]]), "blue-700", 1, MAIN_INKED),
    shape(f.band([[2.4, -1.1], [4.2, -0.5]], 0.9), "blue-700", 1, MAIN),
    shape(f.circle(4, -1.3, 1.25), "red-300", 1),
    shape(f.band([[-6, -0.15], [6.6, -0.1]], 0.3), "wood-700", 0.9),
    shape(`M${f.p(5.2, -0.1)}Q${f.p(5.3, -1.8)} ${f.p(6.8, -1.8)}Q${f.p(8, -1.8)} ${f.p(8.1, -0.1)}Z`, "slate-400", 1),
  ];
}

/** A faint ghost-figure standing on the anchor, 20 tall at scale 1: a translucent cloaked body with dark eye holes and a wispy hem. */
function ghostPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  return [
    shape(
      `M${f.p(-5, -1)}Q${f.p(-6.5, -12)} ${f.p(-4 + sway, -17)}Q${f.p(sway, -21.5)} ${f.p(4 + sway, -17)}Q${f.p(6.5, -12)} ${f.p(5, -1)}` +
        `Q${f.p(3.5, 1)} ${f.p(2.5, -1)}Q${f.p(1, 1.5)} ${f.p(0, -0.5)}Q${f.p(-1.5, 1.5)} ${f.p(-2.5, -1)}Q${f.p(-3.5, 1)} ${f.p(-5, -1)}Z`,
      "slate-100",
      0.35,
      MAIN,
    ),
    shape(f.ellipse(-0.5 + sway * 0.5, -11, 2.8, 7), "white", 0.18),
    shape(f.ellipse(-1.9 + sway * 0.5, -15, 0.9, 1.3) + f.ellipse(1.9 + sway * 0.5, -15, 0.9, 1.3), "slate-900", 0.75),
    shape(f.ellipse(sway * 0.5, -11.8, 0.7, 1.1), "slate-900", 0.6),
    shape(f.band([[-3, 0.5], [-3.6, 1.8]], 0.5) + f.band([[0.2, 0.5], [0.6, 2]], 0.5) + f.band([[3, 0.5], [3.8, 1.6]], 0.5), "slate-100", 0.2),
  ];
}

/** A little card-shaped imp centred on the anchor, 10 tall at scale 1, tilted by the seed. */
function cardImpPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -15, 15);
  const P = (x: number, y: number): Point => turned([x, y], tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(([x, y]) => P(x, y)));
  const band = (points: readonly Point[], width: number): string => f.band(points.map(([x, y]) => P(x, y)), width);
  const ball = (x: number, y: number, r: number): string => f.circle(...P(x, y), r);
  return [
    shape(band([[-2.7, -0.5], [-4.2, 0.5], [-4.6, -0.8]], 0.45) + band([[2.7, -0.5], [4.2, -1.8], [4.8, -3]], 0.45) + band([[-1, 2.2], [-1.4, 3.8], [-2.4, 4.4]], 0.45) + band([[1, 2.2], [1.6, 3.6], [2.8, 3.4]], 0.45), "slate-900", 1),
    shape(poly([[-2.7, -4.2], [2.7, -4.2], [2.7, 2.2], [-2.7, 2.2]]), "white", 1),
    shape(poly([[-2.2, -3.7], [2.2, -3.7], [2.2, 1.7], [-2.2, 1.7]]), "red-500", 1, MAIN),
    shape(poly([[-2.2, -4.2], [-2.9, -5.4], [-1.2, -4.2]]) + poly([[1.2, -4.2], [2.9, -5.4], [2.2, -4.2]]), "red-500", 1, MAIN),
    shape(ball(-1, -1.8, 0.55) + ball(1, -1.8, 0.55), "white", 1),
    shape(ball(-0.9, -1.7, 0.22) + ball(1.1, -1.7, 0.22), "slate-900", 1),
    shape(band([[-1.4, 0], [-0.5, 0.6], [0.5, 0.6], [1.4, 0]], 0.4), "white", 1),
  ];
}

/** An open hand, palm up, its wrist at the anchor, reaching toward the side the part faces, 10 long at scale 1. */
function openHandPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.4, 0.4);
  const finger = (y: number, spread: number, length: number): string =>
    f.band([[5.3, y], [7.4, y + spread * 0.5], [5.3 + length, y + spread + curl]], 0.7);
  return [
    shape(finger(-1.3, -0.4, 4.2) + finger(-0.45, -0.1, 4.7) + finger(0.45, 0.2, 4.5) + finger(1.3, 0.6, 3.8), "red-300", 1, MAIN),
    shape(f.ellipse(3.8, 0, 2.4, 1.7), "red-300", 1, MAIN_INKED),
    shape(f.band([[3.4, -1.4], [5.2, -2.6], [6.4, -3.4 + curl]], 0.8), "red-300", 1, MAIN),
    shape(f.ellipse(3.8, 0.1, 1.5, 1), "white", 0.25),
    shape(f.poly([[0, -1.8], [1.6, -1.8], [1.6, 1.8], [0, 1.8]]), "slate-100", 1),
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

type DkSeg = readonly ["L", number, number] | readonly ["Q", number, number, number, number];

/** A closed path from `start` through straight and quadratic segments, in the part's frame. */
function dkPath(f: Local, start: Point, segs: readonly DkSeg[]): string {
  let d = `M${f.p(start[0], start[1])}`;
  for (const s of segs) d += s[0] === "L" ? `L${f.p(s[1], s[2])}` : `Q${f.p(s[1], s[2])} ${f.p(s[3], s[4])}`;
  return `${d}Z`;
}

function dkBox(x0: number, y0: number, x1: number, y1: number): Point[] {
  return [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
}

function dkRot(points: readonly Point[], degrees: number): Point[] {
  return points.map((p) => turned(p, degrees));
}

/** An ellipse whose axis is tilted `degrees` (as drawn facing right). */
function dkEll(f: Local, flip: number, cx: number, cy: number, rx: number, ry: number, degrees: number): string {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  const a = f.p(cx - rx * c, cy - rx * s);
  const arc = `A${fmt(f.r(rx))} ${fmt(f.r(ry))} ${fmt(flip * degrees)} 1 0`;
  return `M${a}${arc} ${f.p(cx + rx * c, cy + rx * s)}${arc} ${a}Z`;
}

/** A round ring: the circle and a hole inside it. */
function dkRing(f: Local, cx: number, cy: number, r: number, inner: number): string {
  return f.circle(cx, cy, r) + f.circle(cx, cy, inner);
}

/** A swirl arm about an ellipse's centre, `sx` squashing its width. */
function dkSwirl(f: Local, cx: number, cy: number, sx: number, start: number, sweep: number, r0: number, r1: number, width: number): string {
  const steps = 8;
  const outer: Point[] = [];
  const inner: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const r = r0 + (r1 - r0) * t;
    const half = (width * (1 - t)) / 2 + 0.15;
    const angle = (start + sweep * t) * DEGREES;
    outer.push([cx + sx * (r + half) * Math.cos(angle), cy + (r + half) * Math.sin(angle)]);
    inner.push([cx + sx * Math.max(0.3, r - half) * Math.cos(angle), cy + Math.max(0.3, r - half) * Math.sin(angle)]);
  }
  return f.poly([...outer, ...inner.reverse()]);
}

/** A playing card upright on the anchor, 6 wide and 8.4 tall at scale 1, tilted a little. */
function cardPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const t = between(rng, -9, 9);
  const rect = (w: number, h: number): string => f.poly(dkRot(dkBox(-w / 2, -h / 2, w / 2, h / 2), t));
  return [
    shape(rect(6, 8.4), "blue-800", 1, MAIN_INKED),
    shape(rect(6, 8.4) + rect(4.8, 7.2), "white", 1),
    shape(f.poly(dkRot([[0, -1.9], [1.2, 0], [0, 1.9], [-1.2, 0]], t)), "cyan-400", 1),
  ];
}

/** A fan of five cards, base at the anchor, about 17 wide and 9 tall at scale 1. */
function cardFanPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  const diamonds: string[] = [];
  const tilt = between(rng, -3, 3);
  for (let i = 0; i < 5; i += 1) {
    const a = (i - 2) * 26 + tilt + between(rng, -1.5, 1.5);
    out.push(shape(f.poly(dkRot(dkBox(-3, -8.4, 3, 0), a)), "white", 1, INKED));
    out.push(shape(f.poly(dkRot(dkBox(-2.4, -7.8, 2.4, -0.6), a)), "blue-800", 1, MAIN));
    diamonds.push(f.poly(dkRot([[0, -7.2], [1.1, -6], [0, -4.8], [-1.1, -6]], a)));
  }
  out.push(shape(diamonds.join(""), "cyan-400", 1));
  return out;
}

/** A stack of cards on the anchor, 6 wide and 4 tall at scale 1. */
function deckPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const skew = between(rng, -0.3, 0.3);
  const lines = [-1.6, -1, -0.4].map((y) => f.band([[-2.9 + between(rng, 0, 0.3), y], [2.9, y]], 0.22)).join("");
  const top = (inset: number): string => f.poly([[-2.4 + skew + inset, -4 + inset], [3 + skew - inset, -4 + inset], [3 - inset, -2.2 - inset], [-3 + inset, -2.2 - inset]]);
  return [
    shape(f.poly(dkBox(-3, -2.2, 3, 0)), "white", 1, INKED),
    shape(lines, "slate-400", 0.9),
    shape(top(0), "white", 1),
    shape(top(0.35), "blue-800", 1, MAIN),
    shape(f.poly([[0.1 + skew, -3.75], [0.8 + skew, -3.1], [0.1 + skew, -2.45], [-0.6 + skew, -3.1]]), "cyan-400", 1),
  ];
}

/** A silver coin centred on the anchor, radius 2.5 at scale 1, face-on or edge-on. */
function coinPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() > 0.25) {
    return [
      shape(f.circle(0, 0, 2.5), "slate-400", 1, INKED),
      shape(f.circle(0, 0, 2.05), "slate-100", 1, MAIN),
      shape(dkRing(f, 0, 0, 1.45, 1.15), "slate-400", 0.8),
      shape(dkEll(f, at.flip, -0.9, -0.9, 0.55, 0.22, -45), "white", 0.85),
    ];
  }
  return [
    shape(f.ellipse(0, 0, 0.8, 2.5), "slate-400", 1, INKED),
    shape(f.ellipse(-0.15, 0, 0.4, 2.2), "slate-100", 1, MAIN),
    shape(f.band([[-0.3, -1.7], [-0.3, -0.7]], 0.25), "white", 0.85),
  ];
}

/** A stack of four silver coins on the anchor, 5 wide and 8 tall at scale 1. */
function coinStackPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  const seams: string[] = [];
  const glints: string[] = [];
  let top = { x: 0, y: 0 };
  for (let k = 0; k < 4; k += 1) {
    const x = between(rng, -0.3, 0.3);
    const yb = -0.8 - 1.6 * k;
    const yt = yb - 1.6;
    const arc = `A${fmt(f.r(2.5))} ${fmt(f.r(0.8))} 0 0 ${f.sweep(0)}`;
    out.push(shape(`M${f.p(x - 2.5, yt)}L${f.p(x - 2.5, yb)}${arc} ${f.p(x + 2.5, yb)}L${f.p(x + 2.5, yt)}Z`, "slate-400", 1, MAIN_INKED));
    const rim: Point[] = [];
    for (let i = 0; i <= 4; i += 1) rim.push([x - 2.5 * Math.cos((Math.PI * i) / 4), yb + 0.8 * Math.sin((Math.PI * i) / 4) + 0.15]);
    seams.push(f.band(rim, 0.3));
    glints.push(f.band([[x - 1.7, yt + 0.4], [x - 1.7, yb - 0.1]], 0.35));
    top = { x, y: yt };
  }
  out.push(shape(seams.join(""), "slate-900", 0.35));
  out.push(shape(f.ellipse(top.x, top.y, 2.5, 0.8), "slate-100", 1, MAIN));
  out.push(shape(f.ellipse(top.x, top.y, 1.5, 0.5) + f.ellipse(top.x, top.y, 1.2, 0.35), "slate-400", 0.6));
  out.push(shape(glints.join(""), "white", 0.6));
  return out;
}

/** A table on the anchor (ground level), 30 wide and 12 tall at scale 1, seen in slight perspective. */
function tablePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const legs = [
    ...[[-14, -12.4], [12.4, 14]].map(([a, b]) => dkBox(a ?? 0, -7.4, b ?? 0, 0)),
    ...[[-11.6, -10.3], [10.3, 11.6]].map(([a, b]) => dkBox(a ?? 0, -8.8, b ?? 0, -1.4)),
  ];
  const lean = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(0, 0.2, 16.5, 1.4), "slate-900", 0.25),
    shape(legs.map((l) => f.poly(l)).join(""), "wood-700", 1, INKED),
    shape(f.poly(dkBox(-14.4, -8.8, 14.4, -7.4)), "wood-700", 1),
    shape(f.poly([[-12.5 + lean, -12], [12.5 + lean, -12], [15, -8.8], [-15, -8.8]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-11, -10.9], [11, -10.9]], 0.3) + f.band([[-13, -9.8], [13, -9.8]], 0.3), "slate-900", 0.15),
    shape(f.band([[-15, -8.7], [15, -8.7]], 0.55), "white", 0.35),
  ];
}

/** A wooden chair on the anchor, 8 wide and 14 tall at scale 1, seen from the front. */
function chairPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slat = between(rng, 0, 0.3);
  const parts = [
    dkBox(-4, -6.5, 4, -5), dkBox(-3.9, -5, -2.9, 0), dkBox(2.9, -5, 3.9, 0),
    dkBox(-3.9, -14, -2.9, -6.5), dkBox(2.9, -14, 3.9, -6.5),
    dkBox(-2.9, -13.2 + slat, 2.9, -12.1 + slat), dkBox(-2.9, -10.6, 2.9, -9.6),
  ];
  return [
    shape(parts.map((p) => f.poly(p)).join(""), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-3.4, -13.6], [-3.4, -7]], 0.3) + f.band([[-3.8, -6.2], [3.8, -6.2]], 0.35), "white", 0.3),
  ];
}

/** A big closed book lying on the anchor, 10 wide and 4 tall at scale 1, spine to the left. */
function tomePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lock = between(rng, -2.4, -1.6);
  return [
    shape(f.ellipse(0.2, 0, 5.6, 0.5), "slate-900", 0.25),
    shape(f.poly(dkBox(-4.5, -3.2, 4.9, -0.8)), "slate-100", 1, INKED),
    shape([-2.6, -2, -1.4].map((y) => f.band([[-4.2, y], [4.9, y]], 0.12)).join(""), "slate-400", 0.5),
    shape(f.poly(dkBox(-4.5, -4, 5.2, -3.2)) + f.poly(dkBox(-4.5, -0.8, 5.2, 0)) + f.poly(dkBox(-5.3, -4, -4.5, 0)), "red-900", 1, MAIN_INKED),
    shape([-3.6, -2.4].map((x) => f.poly(dkBox(x, -4, x + 0.6, -3.2)) + f.poly(dkBox(x, -0.8, x + 0.6, 0))).join(""), "brass-600", 0.9),
    shape(f.poly(dkBox(4.4, -4.2, 5.4, 0.2)), "slate-400", 1, INKED),
    shape(f.circle(4.9, lock, 0.5), "slate-100", 1),
    shape(f.band([[-4.4, -3.85], [4.8, -3.85]], 0.25), "white", 0.25),
  ];
}

/** A glass jar on the anchor, 5 wide and 7 tall at scale 1, filled with something red. */
function jarPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const level = between(rng, -4.2, -3.4);
  const wave = between(rng, 0.3, 0.6);
  return [
    shape(dkPath(f, [-2.2, level], [["Q", -1.1, level - wave, 0, level], ["Q", 1.1, level + wave, 2.2, level], ["L", 2.2, -0.8], ["Q", 2.2, -0.2, 1.6, -0.2], ["L", -1.6, -0.2], ["Q", -2.2, -0.2, -2.2, -0.8]]), "red-500", 0.7, MAIN),
    shape(dkPath(f, [-1.8, -6], [["L", -1.8, -5.5], ["Q", -2.5, -5, -2.5, -3.5], ["L", -2.5, -0.8], ["Q", -2.5, 0, -1.7, 0], ["L", 1.7, 0], ["Q", 2.5, 0, 2.5, -0.8], ["L", 2.5, -3.5], ["Q", 2.5, -5, 1.8, -5.5], ["L", 1.8, -6]]), "sky-300", 0.35, INKED),
    shape(f.poly(dkBox(-2.1, -7, 2.1, -6)), "slate-400", 1, INKED),
    shape(f.band([[-1.7, -4.6], [-1.8, -1.6]], 0.5), "white", 0.5),
  ];
}

/** A teacup on a saucer, standing on the anchor, 4 wide at scale 1. */
function cupPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const level = between(rng, 0.2, 0.3);
  return [
    shape(dkRing(f, 1.6, -1.8, 0.7, 0.35), "white", 1, INKED),
    shape(f.ellipse(0, -0.35, 2, 0.4), "slate-100", 1, INKED),
    shape(dkPath(f, [-1.2, -2.6], [["L", -0.95, -1.1], ["Q", -0.8, -0.55, 0, -0.55], ["Q", 0.8, -0.55, 0.95, -1.1], ["L", 1.2, -2.6]]), "white", 1, MAIN_INKED),
    shape(f.ellipse(0, -2.6, 1.2, 0.35), "slate-400", 0.7),
    shape(f.ellipse(0, -2.55 + level * 0.1, 1, 0.25), "wood-700", 1),
  ];
}

/** A dagger centred on the anchor, 8 long at scale 1, turned up to the side it faces. */
function daggerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -70, -10);
  const [px, py] = turned([-3.5, 0], a);
  return [
    shape(f.poly(dkRot(dkBox(-3, -0.4, -1, 0.4), a)) + f.circle(px, py, 0.5), "wood-700", 1, INKED),
    shape(f.poly(dkRot(dkBox(-1, -1.5, -0.4, 1.5), a)), "brass-600", 1, INKED),
    shape(f.poly(dkRot([[-0.4, -0.55], [2.6, -0.55], [4, 0], [2.6, 0.55], [-0.4, 0.55]], a)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(dkRot([[-0.4, 0], [4, 0], [2.6, 0.55], [-0.4, 0.55]], a)), "slate-400", 0.5),
    shape(f.band(dkRot([[-0.2, -0.3], [3.2, -0.25]], a), 0.18), "white", 0.7),
  ];
}

/** A long floating sword blade centred on the anchor, 16 long at scale 1, turned. */
function bladePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -55, 25);
  const [px, py] = turned([-7.6, 0], a);
  const [gx, gy] = turned([4.6, -0.5], a);
  return [
    shape(f.poly(dkRot([[-3.9, -1.1], [6, -1], [8, 0], [6, 1], [-3.9, 1.1]], a)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(dkRot(dkBox(-7.1, -0.4, -4.6, 0.4), a)) + f.poly(dkRot(dkBox(-4.6, -2.4, -3.9, 2.4), a)) + f.circle(px, py, 0.5), "slate-700", 1, INKED),
    shape(f.band(dkRot([[-3.2, 0], [5.4, 0]], a), 0.5), "slate-400", 0.7),
    shape(f.band(dkRot([[-3.6, -0.85], [5.8, -0.8]], a), 0.2), "white", 0.55),
    shape(sparkle(f, gx, gy, 1.6), "white", 0.9),
  ];
}

/** An arrow in flight, tip at the anchor pointing the side it faces, 16 long at scale 1. */
function arrowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -12, 12);
  const r = (pts: readonly Point[]): Point[] => dkRot(pts, a);
  return [
    shape(f.band(r([[-2.4, 0], [-15.5, 0]]), 0.5), "wood-600", 1, INKED),
    shape(f.poly(r([[-12.4, 0], [-14.2, -1.7], [-16, -1.7], [-15.2, 0]])) + f.poly(r([[-12.4, 0], [-15.2, 0], [-16, 1.7], [-14.2, 1.7]])), "red-600", 1),
    shape(f.poly(r([[0, 0], [-2.8, -1.2], [-2.2, 0], [-2.8, 1.2]])), "slate-400", 1, MAIN_INKED),
    shape(f.band(r([[-0.4, -0.2], [-2.4, -0.9]]), 0.2), "white", 0.5),
  ];
}

/** A straw archery target on a stand, 12 across and about 17 tall at scale 1, standing on the anchor. */
function targetPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cy = -10.8 + between(rng, -0.3, 0.3);
  return [
    shape(f.band([[-2, -6], [-5, 0]], 0.9) + f.band([[2, -6], [5, 0]], 0.9), "wood-700", 1, INKED),
    shape(f.circle(0, cy, 6), "brass-500", 1, INKED),
    shape(dkRing(f, 0, cy, 6, 5.6), "brass-600", 0.7),
    shape(f.circle(0, cy, 5.2), "red-600", 1, MAIN),
    shape(f.circle(0, cy, 3.6), "white", 1),
    shape(f.circle(0, cy, 2), "red-600", 1, MAIN),
  ];
}

/** A treasure chest standing on the anchor, 10 wide and 8 tall at scale 1. */
function chestPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const plank = between(rng, -2.6, -2);
  return [
    shape(f.poly(dkBox(-5, -4.6, 5, 0)), "wood-600", 1, MAIN_INKED),
    shape(dkPath(f, [-5, -4.6], [["L", -5, -5.8], ["Q", -4.6, -8, 0, -8], ["Q", 4.6, -8, 5, -5.8], ["L", 5, -4.6]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-4.8, plank], [4.8, plank]], 0.25), "slate-900", 0.25),
    shape(f.band([[-4.9, -7.2], [-1.8, -7.8]], 1.2) + f.band([[1.8, -7.8], [4.9, -7.2]], 1.2), "white", 0.18),
    shape([-3.2, 3.2].map((x) => f.poly(dkBox(x - 0.45, -4.3, x + 0.45, 0)) + f.poly(dkBox(x - 0.45, -7.2, x + 0.45, -4.9))).join("") + f.poly(dkBox(-5, -4.9, 5, -4.3)), "slate-400", 1, INKED),
    shape(f.poly(dkBox(-0.9, -5.7, 0.9, -3.3)), "slate-100", 1, INKED),
    shape(f.circle(0, -4.9, 0.3) + f.poly(dkBox(-0.15, -4.7, 0.15, -3.9)), "slate-900", 0.9),
  ];
}

/** A die centred on the anchor, 3 wide at scale 1, in three-quarter view and tumbled a little. */
function dicePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -25, 25);
  const r = (pts: readonly Point[]): Point[] => dkRot(pts, a);
  const pip = (x: number, y: number): string => {
    const [px, py] = turned([x, y], a);
    return f.circle(px, py, 0.2);
  };
  const left = (u: number, v: number): [number, number] => [-1.5 + 1.5 * u, -0.85 + 0.85 * u + 1.7 * v];
  const right = (u: number, v: number): [number, number] => [1.5 * u, -0.85 * u + 1.7 * v];
  const pips = pip(0, -0.85) + pip(...left(0.3, 0.3)) + pip(...left(0.7, 0.7)) + pip(...right(0.25, 0.25)) + pip(...right(0.5, 0.5)) + pip(...right(0.75, 0.75));
  return [
    shape(f.poly(r([[0, -1.7], [1.5, -0.85], [0, 0], [-1.5, -0.85]])) + f.poly(r([[-1.5, -0.85], [0, 0], [0, 1.7], [-1.5, 0.85]])), "white", 1, MAIN_INKED),
    shape(f.poly(r([[0, 0], [1.5, -0.85], [1.5, 0.85], [0, 1.7]])), "slate-400", 1, INKED),
    shape(pips, "black", 0.9),
  ];
}

/** A chain from the anchor along the side it faces, 20 long at scale 1, sagging between its ends. */
function chainPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 3, 5);
  const links: string[] = [];
  const glints: string[] = [];
  for (let i = 0; i < 10; i += 1) {
    const x = 1 + 1.9 * i;
    const t = x / 20;
    const y = sag * 4 * t * (1 - t);
    links.push(dkRing(f, x, y, 0.9, 0.45));
    glints.push(f.circle(x - 0.55, y - 0.55, 0.18));
  }
  return [shape(links.join(""), "slate-400", 1, MAIN_INKED), shape(glints.join(""), "white", 0.7)];
}

/** A glowing rune circle flat on the ground, centred on the anchor, an ellipse 14 wide and 4 tall at scale 1. */
function runePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, 0, 45);
  const ticks: string[] = [];
  for (let k = 0; k < 8; k += 1) {
    const a = (phase + k * 45) * DEGREES;
    const c = Math.cos(a);
    const s = Math.sin(a);
    ticks.push(f.band([[4.8 * c, 1.35 * s], [6 * c, 1.7 * s]], 0.4));
  }
  return [
    shape(f.ellipse(0, 0, 7.6, 2.4), "cyan-400", 0.12),
    shape(f.ellipse(0, 0, 7, 2) + f.ellipse(0, 0, 6.4, 1.8), "cyan-400", 0.7, MAIN),
    shape(f.ellipse(0, 0, 4.4, 1.25) + f.ellipse(0, 0, 4, 1.1), "cyan-400", 0.6),
    shape(ticks.join(""), "cyan-400", 0.7),
    shape(f.poly([[0, -0.6], [1.8, 0], [0, 0.6], [-1.8, 0]]) + f.poly([[0, -0.3], [-0.9, 0], [0, 0.3], [0.9, 0]]), "cyan-400", 0.8),
  ];
}

/** A swirling portal standing on its base at the anchor, 14 wide and 22 tall at scale 1. */
function portalPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = between(rng, 0, 120);
  const arms = [0, 120, 240].map((k) => dkSwirl(f, 0, -11, 0.58, start + k, 200, 1.5, 9, 2.4)).join("");
  const sparks = [[-7.8, -17], [7.6, -5], [-7.9, -6]].map(([x, y]) => sparkle(f, (x ?? 0) + between(rng, -0.4, 0.4), y ?? 0, 1)).join("");
  return [
    shape(f.ellipse(0, -0.2, 6.5, 1), "slate-900", 0.3),
    shape(f.ellipse(0, -11, 8.2, 12), "sky-300", 0.18),
    shape(f.ellipse(0, -11, 7, 11) + f.ellipse(0, -11, 5.6, 9.6), "blue-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -11, 5.6, 9.6), "sky-300", 0.6),
    shape(f.ellipse(0, -11, 3.6, 6.8), "white", 0.2),
    shape(arms, "white", 0.85),
    shape(sparks, "white", 0.9),
  ];
}

/** A spotlight hanging from the anchor: lamp 7 wide, its beam 40 long widening to 24 wide, at scale 1. */
function spotlightPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -1, 1);
  return [
    shape(f.poly([[-3, 4.5], [3, 4.5], [12 + sway, 44.5], [-12 + sway, 44.5]]), "white", 0.15, MAIN),
    shape(f.poly([[-1.5, 4.5], [1.5, 4.5], [6 + sway, 44.5], [-6 + sway, 44.5]]), "white", 0.15),
    shape(f.band([[0, -3], [0, 0.5]], 0.8), "slate-900", 1),
    shape(f.poly([[-2.2, 0], [2.2, 0], [3.4, 4.5], [-3.4, 4.5]]), "slate-700", 1, INKED),
    shape(f.ellipse(0, 4.5, 2.6, 0.7), "white", 0.85),
  ];
}

/** A glass pipe from the anchor along the side it faces, 30 long at scale 1, with one bend down. */
function pipePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drop = between(rng, 5, 8);
  const from = between(rng, 8, 12);
  const ease = (x: number): number => {
    const t = Math.min(1, Math.max(0, (x - from) / 8));
    return drop * t * t * (3 - 2 * t);
  };
  const line = (off: number): Point[] => [0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 29].map((x) => [x, ease(x) + off] as const);
  return [
    shape(f.band(line(0), 2.6), "sky-300", 0.5, MAIN),
    shape(f.band(line(-0.65), 0.5), "white", 0.6),
    shape(f.poly(dkBox(-0.5, -1.9, 1, 1.9)) + f.poly(dkBox(from - 6, -1.9, from - 4.6, 1.9)) + f.poly(dkBox(28.2, drop - 1.9, 29.7, drop + 1.9)), "slate-400", 1, INKED),
  ];
}

/** A gear centred on the anchor, radius 5 at scale 1, turned. */
function gearPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rot = between(rng, 0, 36);
  const pts: Point[] = [];
  for (let i = 0; i < 10; i += 1) {
    for (const [da, r] of [[-9, 4], [-6, 5], [6, 5], [9, 4]] as const) {
      const a = (rot + i * 36 + da) * DEGREES;
      pts.push([r * Math.cos(a), r * Math.sin(a)]);
    }
  }
  const spokes = [0, 1, 2, 3, 4].map((k) => {
    const a = (rot + k * 72) * DEGREES;
    return f.band([[0, 0], [3.4 * Math.cos(a), 3.4 * Math.sin(a)]], 0.9);
  });
  return [
    shape(f.poly(pts) + f.circle(0, 0, 3.1), "slate-400", 1, MAIN_INKED),
    shape(spokes.join(""), "slate-400", 1, INKED),
    shape(f.circle(0, 0, 1.4), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 0.5), "slate-900", 0.9),
  ];
}

/** A heater shield centred on the anchor, 8 wide and 10 tall at scale 1. */
function shieldPart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(dkPath(f, [-4, -5], [["L", 4, -5], ["L", 4, 0], ["Q", 4, 3.5, 0, 5], ["Q", -4, 3.5, -4, 0]]), "slate-400", 1, INKED),
    shape(dkPath(f, [-3.4, -4.4], [["L", 3.4, -4.4], ["L", 3.4, 0], ["Q", 3.4, 3, 0, 4.3], ["Q", -3.4, 3, -3.4, 0]]), "blue-700", 1, MAIN),
    shape(dkPath(f, [-3.4, -4.4], [["L", 0, -4.4], ["L", 0, 4.3], ["Q", -3.4, 3, -3.4, 0]]), "white", 0.18),
    shape(f.circle(0, -0.5, 1.7), "slate-400", 1, INKED),
    shape(f.circle(0, -0.5, 1.2), "slate-100", 1),
  ];
}

/** A straw training dummy on a post, 16 tall at scale 1, standing on the anchor. */
function dummyPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bar = between(rng, -0.3, 0.3);
  const tuft = (s: 1 | -1): string => f.poly([[6 * s, -10.5 + bar], [7.4 * s, -11.4 + bar], [7 * s, -10 + bar], [7.6 * s, -8.9 + bar], [6 * s, -9.5 + bar]]);
  return [
    shape(f.band([[0, -0.5], [0, -15]], 1.2) + f.poly(dkBox(-2.5, -0.5, 2.5, 0)), "wood-700", 1, INKED),
    shape(f.band([[-6, -10 + bar], [6, -10 - bar]], 0.9), "wood-700", 1, INKED),
    shape(tuft(1) + tuft(-1), "brass-500", 1, MAIN),
    shape(dkPath(f, [-2.6, -11.8], [["Q", 0, -12.6, 2.6, -11.8], ["L", 3, -6], ["Q", 0, -5, -3, -6]]), "brass-500", 1, MAIN_INKED),
    shape(f.band([[-2.9, -7.6], [2.9, -7.6]], 0.5) + f.band([[-1.4, -12.3], [1.4, -12.3]], 0.4), "brass-800", 1),
    shape(f.circle(0, -14.2, 2), "stone-500", 1, INKED),
    shape(f.circle(-0.8, -14.4, 0.3) + f.circle(0.8, -14.4, 0.3), "black", 0.7),
  ];
}

/** A toaster-shaped trap snapping open, standing on the anchor, 18 wide and 12 tall at scale 1. */
function toasterPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const open = between(rng, -4, 4);
  const jaw = (hx: number, deg: number): string =>
    f.poly(dkRot(dkBox(0, -1.1, 5.5, 1.1), deg).map(([x, y]) => [x + hx, y - 7.8] as const));
  const slots = [[-6, -1.6], [1.6, 6]] as const;
  return [
    shape(f.ellipse(0, -9, 6.5, 3), "orange-400", 0.2),
    shape(f.ellipse(0, -8.6, 4, 1.8), "orange-400", 0.3),
    shape(dkPath(f, [-9, -0.8], [["L", -9, -5.5], ["Q", -9, -6.5, -8, -6.5], ["L", 8, -6.5], ["Q", 9, -6.5, 9, -5.5], ["L", 9, -0.8]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly(dkBox(-9, -5.6, -7.6, -0.8)), "slate-100", 0.8),
    shape(f.band([[-4, -5.5], [-5.5, -1.5]], 0.8), "white", 0.35),
    shape(f.poly([[-7.8, -8.3], [7.8, -8.3], [9, -6.5], [-9, -6.5]]), "slate-100", 0.6),
    shape(slots.map(([a, b]) => f.poly(dkBox(a, -7.7, b, -7))).join("") + f.band([[7.6, -5], [7.6, -1.5]], 0.6) + f.poly(dkBox(-8, -0.8, -6, 0)) + f.poly(dkBox(6, -0.8, 8, 0)), "slate-900", 1),
    shape(slots.map(([a, b]) => f.poly(dkBox(a + 0.2, -7.55, b - 0.2, -7.15))).join(""), "orange-400", 0.9),
    shape(f.band([[0, -7.8], [-0.9, -8.5], [0.9, -9.1], [-0.9, -9.7], [0, -10.4]], 0.45), "slate-700", 1),
    shape(jaw(-1.3, 215 + open) + jaw(1.3, 325 - open), "slate-700", 1, INKED),
    shape(f.circle(7.6, -3.2, 1), "slate-700", 1, INKED),
  ];
}

/** A glowing jelly bean centred on the anchor, 4 wide at scale 1, tilted. */
function jellyBeanPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -35, 35);
  return [
    shape(f.circle(0, 0, 3.4), "lime-200", 0.15),
    shape(f.circle(0, 0, 2.7), "lime-200", 0.25),
    shape(dkEll(f, at.flip, 0, 0, 2, 1.25, a), "lime-400", 1, MAIN_INKED),
    shape(dkEll(f, at.flip, 0, 0.45, 1.8, 0.6, a), "green-700", 0.45),
    shape(dkEll(f, at.flip, -0.7, -0.5, 0.6, 0.25, a - 20), "white", 0.85),
  ];
}

/** A battered over-taped parcel standing on the anchor, 12 wide and 9 tall at scale 1, top burst open. */
function parcelPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.4, 0.4);
  return [
    shape(
      f.poly([[-6, 0], [-6 + j(), -6], [6 + j(), -6], [6, 0]]) +
        f.poly([[-6, -6], [-0.8, -6], [-3.5, -8.6 + j()], [-8.4, -8 + j()]]) +
        f.poly([[0.8, -6], [6, -6], [8.6, -7.6 + j()], [3.6, -9 + j()]]),
      "brass-600",
      1,
      MAIN_INKED,
    ),
    shape(f.ellipse(0, -6, 4.7, 0.8), "slate-900", 0.7),
    shape(f.poly(dkBox(-2.6, -5.4, -1.4, 0)) + f.poly(dkBox(-6, -3.1, -2.6, -2.1)) + f.poly(dkBox(-1.4, -3.1, 6, -2.1)), "slate-100", 0.85),
    shape(f.poly([[3, -5], [4.2, -4.4], [3.2, -3.6]]) + f.poly([[-5.2, -1.6], [-4.4, -0.8], [-5.4, -0.6]]), "slate-900", 0.28),
    shape(f.poly([[6, 0], [6, -6], [4.6, -5], [4.6, 0]]), "slate-900", 0.15),
  ];
}

/** A bicycle standing on the anchor, 16 wide and 10 tall at scale 1, facing the side it faces. */
function bicyclePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hubs = (x: number): string => f.circle(x, -4.2, 0.5);
  const lean = between(rng, -0.4, 0.4);
  return [
    shape(dkRing(f, -5.5, -4.2, 4.2, 3.6) + dkRing(f, 5.5, -4.2, 4.2, 3.6), "slate-900", 1, INKED),
    shape(hubs(-5.5) + hubs(5.5), "slate-400", 0.9),
    shape(f.band([[-5.5, -4.2], [-2.5, -8.2], [3.2 + lean, -8], [5.5, -4.2]], 0.7), "red-600", 1, MAIN_INKED),
    shape(f.band([[-5.5, -4.2], [0, -3.2], [3.2 + lean, -8]], 0.7), "red-600", 1, MAIN),
    shape(f.band([[-2.5, -8.2], [0, -3.2]], 0.7), "red-600", 1, MAIN),
    shape(f.circle(0, -3.2, 1.1), "slate-400", 1),
    shape(f.band([[-3.4, -9], [-1.2, -9]], 0.9) + f.band([[3.2 + lean, -8], [3, -9.5], [4.8, -9.3]], 0.5), "slate-900", 1),
  ];
}

/** A striped deckchair standing on the anchor, 12 wide and 10 tall at scale 1. */
function deckchairPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, -0.3, 0.3);
  const path: Point[] = [[-4.3, -10], [-1.5, -3.5], [1.5, -2.8 + sag], [5, -5.5]];
  const pieces: Point[][] = [];
  const split = (a: Point, b: Point, n: number): void => {
    for (let i = 0; i < n; i += 1) pieces.push([[a[0] + ((b[0] - a[0]) * i) / n, a[1] + ((b[1] - a[1]) * i) / n], [a[0] + ((b[0] - a[0]) * (i + 1)) / n, a[1] + ((b[1] - a[1]) * (i + 1)) / n]]);
  };
  split(path[0] ?? [0, 0], path[1] ?? [0, 0], 4);
  split(path[1] ?? [0, 0], path[2] ?? [0, 0], 2);
  split(path[2] ?? [0, 0], path[3] ?? [0, 0], 2);
  const stripe = (parity: number): string => pieces.filter((_, i) => i % 2 === parity).map((p) => f.band(p, 2.4)).join("");
  return [
    shape(f.band([[-6, -0.2], [-4.5, -10]], 0.8) + f.band([[6, -0.2], [5, -5.8]], 0.8), "wood-600", 1, INKED),
    shape(f.band([[-6, -0.3], [6, -0.3]], 0.7), "wood-600", 1, INKED),
    shape(stripe(0), "red-600", 1, MAIN),
    shape(stripe(1), "white", 1),
  ];
}

/** A hat tumbling, centred on the anchor, 6 wide at scale 1, turned any way. */
function hatPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, 0, 360);
  const [bx, by] = turned([0, 1.6], a);
  const [tx, ty] = turned([0, -1.8], a);
  return [
    shape(dkEll(f, at.flip, bx, by, 3, 0.7, a), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkRot(dkBox(-1.7, -1.8, 1.7, 1.4), a)), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkRot(dkBox(-1.7, 0.2, 1.7, 1.2), a)), "slate-900", 1),
    shape(dkEll(f, at.flip, tx, ty, 1.7, 0.45, a), "slate-900", 0.3),
    shape(f.band(dkRot([[-1.1, -1.5], [-1.1, -0.1]], a), 0.35), "white", 0.2),
  ];
}

/** An assembly-line belt, its top at the anchor's height, spanning 30 either side of the anchor at scale 1. */
function conveyorPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, -1, 1);
  const rollers: string[] = [];
  for (let x = -28; x <= 28; x += 4) rollers.push(f.circle(x + phase, 1.5, 0.8));
  const legs = [-26, -13, 0, 13, 26].map((x) => f.poly(dkBox(x - 0.8, 3, x + 0.8, 11))).join("");
  return [
    shape(legs, "slate-700", 1, INKED),
    shape(f.poly(dkBox(-30, 0, 30, 3)), "slate-800", 1, MAIN_INKED),
    shape(rollers.join(""), "slate-400", 0.9),
    shape(f.band([[-30, 0.25], [30, 0.25]], 0.5), "white", 0.25),
  ];
}

/** A spinning circular saw blade centred on the anchor, radius 5 at scale 1. */
function sawPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rot = between(rng, 0, 30);
  const pts: Point[] = [];
  for (let i = 0; i < 12; i += 1) {
    for (const [da, r] of [[0, 4.2], [24, 5]] as const) {
      const a = (rot + i * 30 + da) * DEGREES;
      pts.push([r * Math.cos(a), r * Math.sin(a)]);
    }
  }
  const arc = (start: number): Point[] => [0, 1, 2, 3, 4].map((k) => {
    const a = (start + k * 17) * DEGREES;
    return [5.9 * Math.cos(a), 5.9 * Math.sin(a)] as const;
  });
  return [
    shape(f.poly(pts), "slate-100", 1, MAIN_INKED),
    shape(dkRing(f, 0, 0, 3.3, 3.1), "slate-400", 0.6),
    shape(f.circle(0, 0, 1.5), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 0.5), "slate-900", 0.9),
    shape(f.band(arc(rot + 20), 0.5) + f.band(arc(rot + 200), 0.5), "white", 0.4),
  ];
}

/** A sleek rifle lying level, centred on the anchor, 18 long at scale 1, muzzle to the side it faces. */
function riflePart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(f.poly([[-9, -0.2], [-5.5, -1.2], [1.6, -1.2], [1.6, 0.8], [-5.5, 0.8], [-8.6, 2.2]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkBox(1.6, -0.9, 9, -0.1)), "slate-800", 1, INKED),
    shape(
      f.poly(dkBox(-3, -2.75, 2.5, -1.65)) + f.poly(dkBox(-2.4, -1.65, -1.9, -1.2)) + f.poly(dkBox(1.2, -1.65, 1.7, -1.2)) +
        f.poly([[-1, 0.8], [0.8, 0.8], [1, 3.5], [-1.2, 3.5]]) + f.poly(dkBox(8.6, -1.2, 9.5, 0)) +
        f.poly(dkBox(2.5, 0, 6.3, 1)) + f.poly([[2.8, 1], [4.2, 1], [3.9, 3.3], [2.7, 3.3]]),
      "slate-900",
      1,
    ),
    shape(f.circle(2.5, -2.2, 0.5) + f.circle(-3, -2.2, 0.4), "sky-300", 0.7),
    shape(f.band([[-5, -1], [1.5, -1]], 0.4) + f.band([[-8.4, 0], [-5.6, -0.8]], 0.4), "white", 0.2),
  ];
}

/** A dented knight's helmet lying on the anchor, 5 wide at scale 1, visor to the side it faces. */
function helmetPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dx = between(rng, -0.3, 0.3);
  return [
    shape(dkPath(f, [-2.5, 0], [["L", -2.5, -1.8], ["Q", -2.5, -4.2, 0, -4.2], ["Q", 2.5, -4.2, 2.5, -1.8], ["L", 2.5, 0]]), "slate-400", 1, MAIN_INKED),
    shape(dkPath(f, [2.5, 0], [["L", 2.5, -1.8], ["Q", 2.5, -4.2, 0.8, -4.1], ["Q", 1.4, -2.5, 1.2, 0]]), "slate-900", 0.2),
    shape(f.band([[0.4, -2.2], [2.5, -2.2]], 0.45), "slate-900", 0.95),
    shape(f.ellipse(-0.9 + dx, -3.1, 0.8, 0.5), "slate-900", 0.3),
    shape(f.band([[-2.2, -1.2], [-2, -3], [-0.8, -3.9], [0.5, -4]], 0.5), "white", 0.35),
  ];
}

/** A blackboard standing on the anchor, 24 wide and 16 tall at scale 1, chalked with curves and squares. */
function blackboardPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wave: Point[] = [];
  for (let i = 0; i <= 6; i += 1) wave.push([-10 + 1.5 * i, -5 + between(rng, 0.4, 0.8) * Math.sin(i * 1.5)]);
  const arch: Point[] = [];
  for (let i = 0; i <= 6; i += 1) arch.push([-9 + (i * 8) / 6, -6 - 6 * Math.sin((Math.PI * i) / 6)]);
  const square = (x: number, y: number, w: number): string => f.poly(dkBox(x, y, x + w, y + w)) + f.poly(dkBox(x + 0.3, y + 0.3, x + w - 0.3, y + w - 0.3));
  return [
    shape(f.poly(dkBox(-12, -16, 12, -2.8)), "green-900", 1, MAIN),
    shape(f.poly(dkBox(-12, -16, 12, -2.8)) + f.poly(dkBox(-11, -15, 11, -3.8)), "wood-600", 1, INKED),
    shape(f.poly(dkBox(-12.4, -2.8, 12.4, -1.8)) + f.poly(dkBox(-10.5, -1.8, -9.5, 0)) + f.poly(dkBox(9.5, -1.8, 10.5, 0)), "wood-700", 1, INKED),
    shape(f.band(arch, 0.35) + f.band(wave, 0.35), "white", 0.6),
    shape(square(3.5, -13, 3.2) + square(7, -9, 2.6), "white", 0.6),
    shape(f.poly(dkBox(3, -3.4, 4.2, -2.9)), "white", 0.75),
  ];
}

/** A twin-chamber cloning pod standing on the anchor, 30 wide and 26 tall at scale 1. */
function podPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cs = [-8.8, 8.8];
  const each = (g: (c: number) => string): string => cs.map(g).join("");
  const body = (c: number): string =>
    f.circle(c, -18.6, 1.7) + f.poly([[c - 2.4, -16.2], [c + 2.4, -16.2], [c + 2, -11], [c + 1.6, -6.8], [c + 0.2, -6.8], [c, -10.8], [c - 0.2, -6.8], [c - 1.6, -6.8], [c - 2, -11]]);
  const bubbles = each((c) => f.circle(c + 3, -8 - between(rng, 0, 1), 0.35) + f.circle(c + 3.9, -12 - between(rng, 0, 1), 0.3) + f.circle(c - 3.2, -19, 0.3));
  return [
    shape(f.poly(dkBox(-15, -3.4, 15, 0)), "slate-700", 1, INKED),
    shape(f.band([[-2.6, -24.8], [2.6, -24.8]], 1.4) + f.band([[-2.6, -4.4], [2.6, -4.4]], 1.2), "slate-800", 1),
    shape(each((c) => f.poly(dkBox(c - 5.6, -23.5, c + 5.6, -5.5))), "sky-300", 0.3, INKED),
    shape(each((c) => f.ellipse(c, -12, 4.6, 9)), "cyan-400", 0.15),
    shape(each(body), "cyan-400", 0.6),
    shape(bubbles, "white", 0.5),
    shape(each((c) => f.poly(dkBox(c - 6.2, -26, c + 6.2, -23.5)) + f.poly(dkBox(c - 6.2, -5.5, c + 6.2, -3.4))), "slate-400", 1, MAIN_INKED),
    shape(each((c) => f.poly(dkBox(c - 4.8, -22.5, c - 4.1, -7))), "white", 0.3),
  ];
}

/** A crane wrecking ball hanging from the anchor: cable 20 down, ball radius 5 at scale 1 below it. */
function wreckingBallPart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  const cy = 27.2;
  const rim: Point[] = [0, 1, 2, 3, 4].map((k) => {
    const a = (15 + k * 22) * DEGREES;
    return [4.1 * Math.cos(a), cy + 4.1 * Math.sin(a)] as const;
  });
  return [
    shape(f.band([[0, 0], [0, 20.4]], 0.5), "slate-900", 1),
    shape(dkRing(f, 0, 21.2, 1, 0.6), "slate-400", 1, INKED),
    shape(f.circle(0, cy, 5), "slate-700", 1, MAIN_INKED),
    shape(f.band(rim, 1.6), "slate-900", 0.3),
    shape(f.circle(-1.6, cy - 1.6, 2.3), "white", 0.18),
    shape(f.circle(-1.9, cy - 1.9, 0.9), "white", 0.45),
  ];
}

/** A padlock centred on the anchor, 4 wide and 6 tall at scale 1. */
function padlockPart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  const arch: Point[] = [];
  for (let i = 0; i <= 6; i += 1) {
    const a = Math.PI * (i / 6);
    arch.push([-1.1 * Math.cos(a), -0.4 - 1.5 * Math.sin(a) - 1.2 * Math.sin(a) * 0]);
  }
  return [
    shape(f.band([[-1.1, 0], ...arch.slice(1, -1), [1.1, 0]], 0.7), "slate-400", 1, INKED),
    shape(f.poly([[-2, -0.2], [-1.6, -0.6], [1.6, -0.6], [2, -0.2], [2, 2.4], [1.6, 2.8], [-1.6, 2.8], [-2, 2.4]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkBox(-2, -0.2, -1.4, 2.4)), "white", 0.2),
    shape(f.circle(0, 0.9, 0.5) + f.poly([[-0.25, 1.4], [0.25, 1.4], [0.45, 2.2], [-0.45, 2.2]]), "slate-900", 0.95),
  ];
}

/** A lit candle standing on the anchor, 2 wide and 5 tall at scale 1. */
function candlePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.1, 0.1);
  return [
    shape(f.circle(0, -4.2, 1.6), "orange-400", 0.2),
    shape(f.poly(dkBox(-1, -3.2, 1, 0)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(dkBox(0.3, -3.2, 1, 0)), "slate-900", 0.15),
    shape(dkPath(f, [0.55, -3.2], [["L", 1.4, -3.2], ["L", 1.4, -2.2], ["Q", 1.4, -1.5, 1, -1.5], ["Q", 0.55, -1.5, 0.55, -2.2]]), "slate-400", 0.4),
    shape(f.band([[0, -3.2], [lean, -3.8]], 0.2), "slate-900", 1),
    shape(dkPath(f, [lean, -5], [["Q", 0.85, -4.1, 0.6, -3.7], ["Q", 0, -3.3, -0.6, -3.7], ["Q", -0.85, -4.1, 0, -5]]), "orange-400", 1),
    shape(dkPath(f, [lean, -4.4], [["Q", 0.3, -4, 0.2, -3.75], ["Q", 0, -3.6, -0.2, -3.75], ["Q", -0.3, -4, 0, -4.4]]), "white", 0.9),
  ];
}

/** A glowing chute: a funnel mouth 16 wide at the anchor narrowing 14 down, at scale 1. */
function chutePart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  const rivets = [4, 8, 12].map((y) => {
    const e = 8 - (5.8 * y) / 14 - 0.6;
    return f.circle(e, y, 0.3) + f.circle(-e, y, 0.3);
  });
  return [
    shape(f.poly([[-8, 0], [8, 0], [2.2, 14], [-2.2, 14]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6.8, 1], [6.8, 1], [1.5, 13], [-1.5, 13]]), "cyan-400", 0.55),
    shape(f.poly([[-3.5, 1.5], [3.5, 1.5], [0.8, 12.5], [-0.8, 12.5]]), "white", 0.25),
    shape(f.ellipse(0, 14.2, 2.2, 0.8), "cyan-400", 0.3),
    shape(f.ellipse(0, 0, 8, 1.5) + f.ellipse(0, 0, 7.2, 1), "slate-100", 0.6),
    shape(rivets.join(""), "slate-100", 0.8),
  ];
}

/** A huge two-stick game controller centred on the anchor, 30 wide and 16 tall at scale 1, tilted. */
function controllerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -7, 7);
  const T = (x: number, y: number): Point => turned([x, y], tilt);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const dot = (x: number, y: number, r: number): string => {
    const [px, py] = T(x, y);
    return f.circle(px, py, r);
  };
  const pad = ((cx: number, cy: number, l: number, w: number): Point[] => [
    [cx - w, cy - l], [cx + w, cy - l], [cx + w, cy - w], [cx + l, cy - w], [cx + l, cy + w], [cx + w, cy + w],
    [cx + w, cy + l], [cx - w, cy + l], [cx - w, cy + w], [cx - l, cy + w], [cx - l, cy - w], [cx - w, cy - w],
  ])(-3.4, 1.2, 1.7, 0.55);
  const button = (x: number, y: number): string => dot(9.6 + x, -1.2 + y, 1.05);
  return [
    shape(poly([[-9, -6.5], [9, -6.5], [13, -5.2], [15, -1], [15, 4], [14.2, 8], [11.6, 8.3], [9.2, 4.4], [5, 3.6], [-5, 3.6], [-9.2, 4.4], [-11.6, 8.3], [-14.2, 8], [-15, 4], [-15, -1], [-13, -5.2]]), "slate-800", 1, MAIN_INKED),
    shape(poly([[-13.2, -5.3], [-8.8, -6.5], [-8.8, -8], [-12.4, -7.2]]) + poly([[13.2, -5.3], [8.8, -6.5], [8.8, -8], [12.4, -7.2]]), "slate-700", 1, INKED),
    shape(dot(-8, -0.8, 2.8) + dot(3.6, 1, 2.4) + dot(-1.2, -3.6, 0.55) + dot(1.2, -3.6, 0.55), "slate-700", 1),
    shape(dot(-8, -0.8, 1.8) + dot(3.6, 1, 1.8), "slate-900", 1),
    shape(dot(-8, -0.8, 1) + dot(3.6, 1, 1) + dot(0, -5.2, 0.8), "slate-400", 0.5),
    shape(poly(pad), "slate-400", 1, INKED),
    shape(button(2.1, 0), "red-500", 1),
    shape(button(0, 2.1), "lime-400", 1),
    shape(button(-2.1, 0), "sky-400", 1),
    shape(button(0, -2.1), "violet-500", 1),
    shape(f.band([[-9, -6.1], [9, -6.1]].map(([x, y]) => T(x ?? 0, y ?? 0)), 0.5), "white", 0.12),
  ];
}

/** A rectangle in a part's frame: its top-left corner, width and height. */
function eRect(f: Local, x: number, y: number, w: number, h: number): string {
  return f.poly([[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
}

/** A chess piece standing on the anchor, 6 tall at scale 1: a pawn, a rook, a knight's head or a king. */
function chessPiecePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const kind = whole(rng, 0, 3);
  const base = shape(f.poly([[-2.3, 0], [2.3, 0], [2, -1], [-2, -1]]), "slate-400", 1, INKED);
  if (kind === 0) {
    return [
      base,
      shape(f.poly([[-1.3, -1], [1.3, -1], [0.7, -3.4], [-0.7, -3.4]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[-1.2, -3.2], [1.2, -3.2], [1.2, -3.8], [-1.2, -3.8]]), "slate-100", 1, MAIN_INKED),
      shape(f.circle(0, -4.7, 1.3), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[0.1, -1], [1.3, -1], [0.7, -3.4], [0.1, -3.4]]) + f.circle(0.5, -4.5, 0.75), "slate-900", 0.2),
      shape(f.circle(-0.5, -5.1, 0.45), "white", 0.7),
    ];
  }
  if (kind === 1) {
    return [
      base,
      shape(f.poly([[-1.5, -1], [1.5, -1], [1, -4.2], [-1, -4.2]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[-1.8, -4.2], [1.8, -4.2], [1.8, -6], [1.3, -6], [1.3, -5.2], [0.45, -5.2], [0.45, -6], [-0.45, -6], [-0.45, -5.2], [-1.3, -5.2], [-1.3, -6], [-1.8, -6]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[0.2, -1], [1.5, -1], [1, -4.2], [0.2, -4.2]]) + f.poly([[0.6, -4.2], [1.8, -4.2], [1.8, -6], [1.3, -6], [1.3, -5.2], [0.6, -5.2]]), "slate-900", 0.2),
      shape(f.band([[-0.9, -1.6], [-0.6, -4]], 0.4), "white", 0.7),
    ];
  }
  if (kind === 2) {
    return [
      base,
      shape(f.poly([[-1.6, -1], [1.6, -1], [1.3, -3], [1.7, -4.6], [0.7, -6.1], [0, -5.4], [-0.5, -6], [-1.3, -5.1], [-2.5, -3.8], [-2.3, -3], [-1.1, -3.4], [-0.3, -3.1]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[0.3, -1], [1.6, -1], [1.3, -3], [1.7, -4.6], [0.7, -6.1], [0.5, -4]]), "slate-900", 0.2),
      shape(f.circle(-0.7, -4.5, 0.32), "slate-900", 0.9),
      shape(f.band([[-0.6, -5.5], [0.4, -3.5]], 0.35), "white", 0.6),
    ];
  }
  return [
    base,
    shape(f.poly([[-1.5, -1], [1.5, -1], [0.9, -4], [-0.9, -4]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-1.4, -4], [1.4, -4], [1.7, -5], [-1.7, -5]]), "slate-100", 1, MAIN_INKED),
    shape(f.band([[0, -5], [0, -6.4]], 0.55) + f.band([[-0.7, -5.8], [-0.2, -5.8]], 0.5) + f.band([[0.2, -5.8], [0.7, -5.8]], 0.5), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[0.1, -1], [1.5, -1], [0.9, -4], [0.1, -4]]) + f.poly([[0.1, -4], [1.4, -4], [1.7, -5], [0.1, -5]]), "slate-900", 0.2),
    shape(f.band([[-0.8, -1.6], [-0.5, -3.6]], 0.4), "white", 0.7),
  ];
}

/** An hourglass standing on the anchor, 4 wide and 7 tall at scale 1. */
function hourglassPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = between(rng, 0, 0.6);
  return [
    shape(eRect(f, -2.1, -7, 4.2, 0.65) + eRect(f, -2.1, -0.65, 4.2, 0.65), "wood-700", 1, INKED),
    shape(f.band([[-1.8, -6.35], [-1.8, -0.65]], 0.4) + f.band([[1.8, -6.35], [1.8, -0.65]], 0.4), "wood-700", 1),
    shape(f.poly([[-1.5, -6.35], [1.5, -6.35], [0.2, -3.5], [1.5, -0.65], [-1.5, -0.65], [-0.2, -3.5]]), "sky-300", 0.45, INKED),
    shape(f.poly([[-1.05 + top, -5.4 + top], [1.05 - top, -5.4 + top], [0.12, -3.8], [-0.12, -3.8]]) + f.poly([[-1.35, -0.65], [1.35, -0.65], [0.5, -1.9], [-0.5, -1.9]]), "brass-500", 1, MAIN),
    shape(f.band([[0, -3.8], [0, -1.9]], 0.18), "brass-500", 0.9),
    shape(f.band([[-1, -6], [-0.55, -4.4]], 0.3), "white", 0.7),
  ];
}

/** A desk globe on its stand, 7 tall at scale 1, standing on the anchor. */
function globePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const d = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-1.9, 0], [1.9, 0], [1.3, -0.7], [-1.3, -0.7]]), "brass-600", 1, INKED),
    shape(f.band([[0, -0.7], [0, -1.4]], 0.6), "brass-600", 1),
    shape(f.circle(0, -4.1, 2.4), "blue-600", 1, MAIN_INKED),
    shape(f.poly([[-1.9, -4.9 + d], [-0.6, -5.6], [-0.2, -4.6], [-1.2, -3.9]]) + f.poly([[0.5, -3.7], [1.8, -3.9], [1.6, -2.7 + d], [0.6, -2.4]]), "green-500", 1),
    shape(f.circle(0, -4.1, 2.85) + f.circle(0, -4.1, 2.55), "slate-400", 1),
    shape(f.ellipse(-0.8, -5.2, 0.6, 0.35), "white", 0.7),
    shape(f.ellipse(1, -3.2, 1.4, 1.5), "slate-900", 0.15),
  ];
}

/** An open blank spiral notebook lying on the anchor, 14 wide at scale 1, a page lifted. */
function notebookPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -1.2, 1.2);
  const rings: string[] = [];
  for (let i = 0; i < 5; i += 1) rings.push(f.ellipse(0, -2.5 + i * 0.95, 0.75, 0.3));
  return [
    shape(f.ellipse(0, 1.1, 7.2, 1), "slate-900", 0.25),
    shape(f.poly([[-7, -3.6], [-0.3, -3], [-0.3, 1.5], [-7, 0.7]]), "white", 1, INKED),
    shape(f.poly([[0.3, -3], [7, -3.6], [7, 0.7], [0.3, 1.5]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[0.3, -3], [1.2, -2.95], [1.2, 1.4], [0.3, 1.5]]) + f.poly([[-0.3, -3], [-1.1, -3.05], [-1.1, 1.4], [-0.3, 1.5]]), "slate-900", 0.1),
    shape(rings.join(""), "slate-400", 1),
    shape(`M${f.p(0.5, -2.9)}Q${f.p(1.5 + lift, -8.4)} ${f.p(6, -6.6)}Q${f.p(4.6, -5)} ${f.p(6.6, -3.5)}Q${f.p(3.4, -4)} ${f.p(0.5, -2.9)}Z`, "white", 0.95, INKED),
    shape(f.band([[1, -3.6], [3.4, -5.4], [5.8, -6.2]], 0.3), "slate-400", 0.4),
  ];
}

/** A park bench standing on the anchor, 24 wide and 9 tall at scale 1. */
function benchPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 11.7, 12);
  return [
    shape(f.ellipse(0, 0, w + 1, 0.7), "slate-900", 0.25),
    shape(f.poly([[-w - 0.4, 0], [-w + 0.4, 0], [-w + 0.4, -3.4], [-w - 0.4, -3.4]]), "slate-900", 1),
    shape(f.poly([[w - 0.4, 0], [w + 0.4, 0], [w + 0.4, -3.4], [w - 0.4, -3.4]]), "slate-900", 1),
    shape(f.band([[-w + 0.4, -9.2], [-w + 0.4, -6.4]], 0.8) + f.band([[w - 0.4, -9.2], [w - 0.4, -6.4]], 0.8), "slate-900", 1),
    shape(f.poly([[-w - 0.4, -6.5], [-w + 0.8, -6.5], [-w + 0.8, -3.4], [-w - 0.4, -3.4]]) + f.poly([[w - 0.8, -6.5], [w + 0.4, -6.5], [w + 0.4, -3.4], [w - 0.8, -3.4]]), "slate-900", 1),
    shape(eRect(f, -w, -9, w * 2, 0.9) + eRect(f, -w, -7.9, w * 2, 0.9) + eRect(f, -w, -6.8, w * 2, 0.9) + eRect(f, -w, -4.6, w * 2, 0.9) + eRect(f, -w, -3.4, w * 2, 0.9), "wood-600", 1, MAIN_INKED),
    shape(eRect(f, -w, -9, w * 2, 0.3) + eRect(f, -w, -4.6, w * 2, 0.3), "white", 0.18),
  ];
}

/** A twig nest with two speckled eggs on the anchor, 16 wide and 4 tall at scale 1. */
function nestPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.3, 0.3);
  return [
    shape(`M${f.p(-8, -2.4)}Q${f.p(-7.2, 0.4)} ${f.p(0, 0.4)}Q${f.p(7.2, 0.4)} ${f.p(8, -2.4)}Z`, "wood-700", 1, INKED),
    shape(f.ellipse(0, -2.4, 8, 1.6), "wood-700", 1, MAIN_INKED),
    shape(f.ellipse(0, -2.5, 6.4, 1.1), "wood-800", 1),
    shape(f.ellipse(-1.9, -2.7, 1.5, 1.05) + f.ellipse(1.7 + j, -2.8, 1.5, 1.05), "slate-100", 1, INKED),
    shape(f.circle(-2.4, -2.9, 0.18) + f.circle(-1.5, -2.4, 0.15) + f.circle(1.3 + j, -3, 0.18) + f.circle(2.2 + j, -2.6, 0.15), "wood-700", 0.8),
    shape(f.band([[-8, -2], [-4, -0.9], [0, -0.7], [4, -0.9], [8, -2]], 1.1), "wood-700", 1, INKED),
    shape(f.band([[-7, -1.4], [-3, -0.6]], 0.3) + f.band([[-1, -0.6], [3, -0.7]], 0.3) + f.band([[4, -1.2], [7.4, -2.2]], 0.3), "wood-600", 0.9),
    shape(f.band([[-6, -3.9], [-2, -3.5]], 0.3) + f.band([[2, -3.6], [6, -3.9]], 0.3) + f.band([[-7.6, -2.6], [-6.5, -3.5]], 0.3), "wood-600", 0.9),
  ];
}

/** A small sailing ship on the anchor (its waterline), 20 wide and 18 tall at scale 1. */
function shipPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bel = between(rng, 0.6, 1.4);
  const flag = between(rng, 3.8, 4.8);
  return [
    shape(f.band([[0, -4.8], [0, -17.6]], 0.7), "wood-800", 1, INKED),
    shape(f.poly([[0.5, -16.2], [7.2 + bel, -13.8], [6.8, -7.4], [0.5, -6.6]]), "white", 1, INKED),
    shape(f.poly([[-0.5, -15], [-5.5 - bel * 0.5, -12.8], [-5.4, -8], [-0.5, -7.4]]), "white", 1, INKED),
    shape(f.poly([[3, -7.4], [6.8, -7.4], [7.2 + bel, -13.8], [3.5, -12]]) + f.poly([[-3, -7.7], [-5.4, -8], [-5.9, -12.9], [-3, -11]]), "slate-400", 0.3),
    shape(f.poly([[0.2, -18], [flag, -17.1], [0.2, -16.2]]), "red-600", 1),
    shape(f.poly([[-9.6, -5], [9.6, -5], [7.4, 0], [-7, 0]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-8.8, -3.6], [8.8, -3.6]], 0.45) + f.band([[-7.8, -1.7], [7, -1.7]], 0.4), "wood-800", 0.65),
    shape(f.band([[-9.2, -4.8], [9.2, -4.8]], 0.5), "white", 0.2),
    shape(f.circle(-3.8, -2.6, 0.5) + f.circle(0, -2.6, 0.5) + f.circle(3.8, -2.6, 0.5), "slate-900", 0.8),
  ].flat();
}

/** A wrapped gift box on the anchor, 4 wide at scale 1. */
function giftPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.3, 0.3);
  return [
    shape(eRect(f, -2, -3.3, 4, 3.3), "red-500", 1, MAIN_INKED),
    shape(eRect(f, -2.2, -4.2, 4.4, 1.1), "red-500", 1, MAIN_INKED),
    shape(eRect(f, 0.5, -3.3, 1.5, 3.3), "slate-900", 0.2),
    shape(eRect(f, -0.35, -4.2, 0.7, 4.2) + eRect(f, -2, -2.1, 1.65, 0.7) + eRect(f, 0.35, -2.1, 1.65, 0.7), "white", 1),
    shape(f.ellipse(-1 + tilt, -4.9, 0.95, 0.55) + f.ellipse(1 - tilt, -4.9, 0.95, 0.55), "white", 1, INKED),
    shape(f.circle(0, -4.6, 0.42), "white", 1, INKED),
  ];
}

/** A dripping honeypot on the anchor, 7 wide and 7 tall at scale 1. */
function honeypotPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drip = between(rng, 3.4, 4.4);
  return [
    shape(`M${f.p(-1.9, -5.5)}Q${f.p(-3.7, -3)} ${f.p(-3.1, -1)}Q${f.p(-2.6, 0)} ${f.p(0, 0)}Q${f.p(2.6, 0)} ${f.p(3.1, -1)}Q${f.p(3.7, -3)} ${f.p(1.9, -5.5)}Z`, "brass-600", 1, MAIN_INKED),
    shape(`M${f.p(1.3, -5)}Q${f.p(3.4, -3.6)} ${f.p(3, -1)}Q${f.p(2.6, 0)} ${f.p(1.4, 0)}Q${f.p(2.2, -2.5)} ${f.p(1.3, -5)}Z`, "slate-900", 0.25),
    shape(f.poly([[0.8, -6.3], [3.5, -6.9], [3.7, -6.2], [1, -5.6]]) + f.circle(2.2, -7.2, 0.5), "brass-500", 1, INKED),
    shape(f.ellipse(0, -5.5, 2.5, 0.6), "brass-500", 1, INKED),
    shape(f.ellipse(0, -5.6, 2.1, 0.5), "orange-400", 1),
    shape(`M${f.p(-1.7, -5.4)}L${f.p(-0.6, -5.4)}L${f.p(-0.6, -drip)}Q${f.p(-0.6, -drip + 1.2)} ${f.p(-1.15, -drip + 1.2)}Q${f.p(-1.7, -drip + 1.2)} ${f.p(-1.7, -drip)}Z`, "orange-400", 1),
    shape(`M${f.p(0.6, -5.4)}L${f.p(1.5, -5.4)}L${f.p(1.5, -3)}Q${f.p(1.5, -2.2)} ${f.p(1.05, -2.2)}Q${f.p(0.6, -2.2)} ${f.p(0.6, -3)}Z`, "orange-400", 1),
    shape(f.band([[-2.6, -4.4], [-2.4, -1.8]], 0.4), "white", 0.35),
  ];
}

/** A university photocopier standing on the anchor, 20 wide and 14 tall at scale 1. */
function copierPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const on = whole(rng, 0, 1);
  return [
    shape(f.ellipse(0, 0, 10.4, 0.7), "slate-900", 0.2),
    shape(eRect(f, -9, -10.6, 18, 10.6), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-9.8, -10.6], [9.8, -10.6], [8.8, -13.6], [-8.8, -13.6]]), "slate-400", 1, INKED),
    shape(f.poly([[-8, -12.2], [8, -12.2], [8.4, -11.2], [-8.4, -11.2]]), "white", 0.4),
    shape(eRect(f, 2.8, -9.4, 5.6, 3.2), "slate-700", 1, INKED),
    shape(f.circle(4, -7.6, 0.4) + f.circle(5.6, -7.6, 0.4), "lime-400", on === 1 ? 1 : 0.85),
    shape(f.circle(7.2, -7.6, 0.4), "red-500", 1),
    shape(f.band([[3.4, -8.7], [7.8, -8.7]], 0.5), "lime-400", 0.7),
    shape(f.band([[9, -5], [10.2, -5], [10.2, -2.6]], 0.7) + f.circle(10.2, -2.3, 0.5), "slate-700", 1),
    shape(f.poly([[-7.5, -3.2], [-0.5, -3.2], [0.5, -0.6], [-8.5, -0.6]]), "slate-400", 1, INKED),
    shape(f.poly([[-6.4, -3.6], [-1.6, -3.6], [-0.9, -1.2], [-7.1, -1.2]]), "white", 1, INKED),
    shape(f.band([[-7, -3.8], [-1, -3.8]], 0.5), "slate-900", 0.7),
  ];
}

/** A spiked crimson mask centred on the anchor, 10 wide at scale 1. */
function maskPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spikes: string[] = [];
  for (const a of [150, 180, 210, 240, 270, 300, 330, 0, 30]) {
    const tip = a * DEGREES;
    const l = (a - 7) * DEGREES;
    const r = (a + 7) * DEGREES;
    spikes.push(f.poly([[3.5 * Math.cos(l), 4.3 * Math.sin(l)], [5 * Math.cos(tip), 6.1 * Math.sin(tip)], [3.5 * Math.cos(r), 4.3 * Math.sin(r)]]));
  }
  const brow = between(rng, 0, 0.4);
  return [
    shape(spikes.join(""), "slate-100", 1, INKED),
    shape(`M${f.p(0, -4.5)}Q${f.p(3.9, -4.3)} ${f.p(3.7, 0)}Q${f.p(3.4, 3.2)} ${f.p(0, 5)}Q${f.p(-3.4, 3.2)} ${f.p(-3.7, 0)}Q${f.p(-3.9, -4.3)} ${f.p(0, -4.5)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.poly([[-3.2, -2 - brow], [-0.5, -0.9], [-3.2, -3.3]]) + f.poly([[3.2, -2 - brow], [0.5, -0.9], [3.2, -3.3]]), "red-900", 0.6),
    shape(f.poly([[-3, -1.5], [-0.6, -0.5], [-1.2, 1], [-2.9, 0]]) + f.poly([[3, -1.5], [0.6, -0.5], [1.2, 1], [2.9, 0]]), "black", 1),
    shape(f.band([[0, 0.8], [0, 2.4]], 0.4) + f.band([[-1.3, 3.4], [0, 3.8], [1.3, 3.4]], 0.4), "red-900", 0.8),
    shape(f.poly([[-3.3, -3.4], [-1.2, -4], [-2.4, -2.5]]), "white", 0.5),
  ];
}

/** An empty frying pan lying on the anchor, 10 long at scale 1, its handle to the facing side. */
function panPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hang = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(-0.6, 0.9, 5.2, 1.2), "slate-900", 0.25),
    shape(f.band([[2, 0], [5, hang]], 1.1), "slate-900", 1, INKED),
    shape(f.band([[2.6, -0.3], [4.8, -0.2 + hang]], 0.3), "slate-400", 0.7),
    shape(f.ellipse(-1.3, 0, 3.8, 2.5), "slate-400", 1, INKED),
    shape(f.ellipse(-1.3, 0.1, 3.2, 2), "slate-900", 1, MAIN_INKED),
    shape(`M${f.p(-3.8, -0.3)}Q${f.p(-3, -1.5)} ${f.p(-1.4, -1.8)}Q${f.p(-2.8, -0.9)} ${f.p(-3.8, -0.3)}Z`, "white", 0.2),
  ];
}

/** A rope snare lying flat on the anchor, 8 wide at scale 1. */
function snarePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.5, 0.5);
  return [
    shape(f.ellipse(0, -0.8, 4, 1.7), "slate-900", 0.18),
    shape(f.band([[3.4, -1.3], [3.8, -1.1], [3.6, -0.4]], 0.5), "wood-600", 1),
    shape(f.ellipse(0, -0.9, 3.7, 1.6) + f.ellipse(0, -0.9, 2.9, 1), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-3, -1.9], [-2.7, -0.3]], 0.3) + f.band([[-1, -2.3], [-0.7, -0.6]], 0.3) + f.band([[1.4, -2.3], [1.4, -0.6]], 0.3) + f.band([[3, -1.5], [2.9, -0.2]], 0.3), "wood-700", 0.9),
    shape(f.band([[-1.2, -0.3], [1.2 + lean, -2.8]], 0.5), "wood-700", 1, INKED),
    shape(f.band([[3.4, -3.2], [3.4, -0.2]], 0.7) + f.circle(3.4, -3.3, 0.45), "wood-800", 1, INKED),
  ];
}

/** A tall rolling library ladder, leaning a little, its wheels on the anchor, 6 wide and 34 tall at scale 1. */
function ladderPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0.05, 0.1);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => P(x, y)));
  const rungs: string[] = [];
  for (let i = 0; i < 8; i += 1) {
    const y = -4.2 - i * 4;
    rungs.push(poly([[-2.3, y], [2.3, y], [2.3, y - 0.8], [-2.3, y - 0.8]]));
  }
  const wheel = (dx: number, r: number): string => f.circle(...P(dx, -r), r);
  return [
    shape(poly([[-3, -1.8], [-2.3, -1.8], [-2.3, -34], [-3, -34]]) + poly([[2.3, -1.8], [3, -1.8], [3, -34], [2.3, -34]]), "wood-600", 1, MAIN_INKED),
    shape(rungs.join(""), "wood-600", 1, MAIN_INKED),
    shape(poly([[-3.2, -34.4], [3.2, -34.4], [3.2, -33.6], [-3.2, -33.6]]), "slate-700", 1),
    shape(wheel(-2.65, 0.95) + wheel(2.65, 0.95), "slate-900", 1),
    shape(f.circle(...P(-2.65, -0.95), 0.35) + f.circle(...P(2.65, -0.95), 0.35), "slate-400", 1),
    shape(poly([[-2.95, -2.2], [-2.7, -2.2], [-2.7, -33], [-2.95, -33]]), "white", 0.22),
  ];
}

/** A giant war hammer centred on the anchor, 24 long at scale 1, turned by the stream, its head to the facing side. */
function hammerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -60, -25);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const wrap: string[] = [];
  for (let i = 0; i < 4; i += 1) wrap.push(poly([[-11.6 + i * 1.3, -0.7], [-11.2 + i * 1.3, -0.7], [-11.2 + i * 1.3, 0.7], [-11.6 + i * 1.3, 0.7]]));
  return [
    shape(poly([[-12, -0.65], [6, -0.65], [6, 0.65], [-12, 0.65]]), "wood-700", 1, INKED),
    shape(wrap.join(""), "slate-900", 0.45),
    shape(poly([[5.5, -3.9], [11.7, -3.9], [11.7, 3.9], [5.5, 3.9]]), "slate-400", 1, MAIN_INKED),
    shape(poly([[5.5, -3.9], [11.7, -3.9], [11.7, -2.6], [5.5, -2.6]]), "white", 0.5),
    shape(poly([[5.5, 2.5], [11.7, 2.5], [11.7, 3.9], [5.5, 3.9]]), "slate-900", 0.3),
    shape(poly([[5.5, -3.9], [6.4, -3.9], [6.4, 3.9], [5.5, 3.9]]), "slate-700", 0.7),
    shape(poly([[-12.3, -1], [-11.5, -1], [-11.5, 1], [-12.3, 1]]), "slate-700", 1),
  ];
}

/** A hologram panel centred on the anchor, 30 wide and 20 tall at scale 1, its projector base below it. */
function hologramPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tile = (x: number, y: number): string => eRect(f, x, y, 13, 8.2) + eRect(f, x + 0.5, y + 0.5, 12, 7.2);
  const lines: string[] = [];
  const start = between(rng, 0, 1);
  for (let i = 0; i < 9; i += 1) lines.push(eRect(f, -14.5, -9 + i * 2 + start, 29, 0.3));
  return [
    shape(f.poly([[-3, 11], [3, 11], [14, 10], [-14, 10]]), "cyan-400", 0.12),
    shape(eRect(f, -15, -10, 30, 20), "cyan-400", 0.2, MAIN),
    shape(tile(-14, -9.4) + tile(1, -9.4) + tile(-14, 1.2) + tile(1, 1.2), "cyan-400", 0.85),
    shape(lines.join(""), "white", 0.12),
    shape(eRect(f, -15, -10, 30, 20) + eRect(f, -14.4, -9.4, 28.8, 18.8), "cyan-400", 0.35),
    shape(f.poly([[-5.5, 12.4], [5.5, 12.4], [3.6, 10.6], [-3.6, 10.6]]), "slate-700", 1, INKED),
    shape(f.band([[-2.5, 11.2], [2.5, 11.2]], 0.4), "cyan-400", 0.9),
  ];
}

/** A telescope on a tripod standing on the anchor, 14 tall at scale 1, its tube aimed up to the facing side. */
function telescopePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -58, -46);
  const T = (dx: number): Point => {
    const [x, y] = turned([dx, 0], ang);
    return [x, y - 6.2];
  };
  return [
    shape(f.band([[0, -6], [3.4, 0]], 0.7), "slate-700", 1, INKED),
    shape(f.band([[0, -6], [-3.8, 0]], 0.8), "slate-700", 1, INKED),
    shape(f.band([[0, -6], [-0.3, -0.1]], 0.6), "slate-900", 1),
    shape(f.band([T(-0.6), T(6.8)], 1.9), "slate-400", 1, MAIN_INKED),
    shape(f.band([T(6.8), T(8.4)], 2.5), "slate-700", 1, INKED),
    shape(f.band([T(-2.4), T(-0.6)], 1), "slate-900", 1),
    shape(f.band([T(0.5), T(6)].map(([x, y]) => [x - 0.2, y - 0.5] as const), 0.4), "white", 0.5),
    shape(f.circle(0, -6.2, 0.8), "slate-700", 1, INKED),
  ];
}

/** A rocket-powered cart standing on the anchor, 20 wide and 10 tall at scale 1, facing the side it faces, its flame behind. */
function rocketCartPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const fl = between(rng, 0, 1.4);
  return [
    shape(f.poly([[-7.3, -5.4], [-8.5 - fl * 0.3, -4.4], [-10 - fl, -3.7], [-8.4 - fl * 0.2, -3], [-7.3, -2.3], [-7.3, -3.8]]), "orange-400", 1),
    shape(f.poly([[-7.3, -4.7], [-8.4, -4.2], [-9 - fl * 0.6, -3.7], [-8.2, -3.2], [-7.3, -3]]), "white", 0.9),
    shape(f.poly([[-3.4, -6.4], [-3.4, -2.2], [8.6, -2.2], [8.6, -3.6], [6.8, -6.4]]), "red-600", 1, MAIN_INKED),
    shape(f.poly([[-3.4, -6.4], [6.8, -6.4], [7.2, -5.6], [-3.4, -5.6]]), "white", 0.3),
    shape(f.band([[-7.2, -3.9], [-3.2, -3.9]], 2.8), "slate-400", 1, INKED),
    shape(f.poly([[-7.2, -5.6], [-7.8, -6.4], [-7.8, -1.4], [-7.2, -2.2]]), "slate-700", 1),
    shape(f.band([[2.8, -6.3], [2.8, -8.6], [4.8, -8.6]], 0.5), "slate-900", 1),
    shape(f.circle(-0.8, -2, 2) + f.circle(6, -2, 2), "slate-900", 1, INKED),
    shape(f.circle(-0.8, -2, 0.8) + f.circle(6, -2, 0.8), "slate-400", 1),
  ];
}

/** A bronze bell hanging from the anchor, 8 wide and 8 tall at scale 1, swung by the stream. */
function bellPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -14, 14);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const q = (dx: number, dy: number): string => f.p(...T(dx, dy));
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  return [
    shape(f.band([T(0, 0), T(0, 1.8)], 0.8), "wood-700", 1, INKED),
    shape(`M${q(-1.4, 1.5)}L${q(1.4, 1.5)}Q${q(1.8, 4.6)} ${q(4, 5.9)}L${q(-4, 5.9)}Q${q(-1.8, 4.6)} ${q(-1.4, 1.5)}Z`, "brass-600", 1, MAIN_INKED),
    shape(poly([[-4.1, 5.7], [4.1, 5.7], [4, 6.9], [-4, 6.9]]), "brass-500", 1, INKED),
    shape(`M${q(-1, 2)}Q${q(-1.3, 4.6)} ${q(-3.2, 5.6)}L${q(-2.6, 5.6)}Q${q(-0.9, 4.4)} ${q(-0.5, 2)}Z`, "white", 0.4),
    shape(`M${q(1.1, 2)}Q${q(1.5, 4.6)} ${q(3.8, 5.7)}L${q(2.4, 5.7)}Q${q(1.1, 4.4)} ${q(0.8, 2)}Z`, "slate-900", 0.28),
    shape(f.circle(...T(0.3, 7.6), 0.85), "brass-800", 1, INKED),
  ];
}

/** Three soft rice cakes stacked on a white plate on the anchor, 12 wide at scale 1, a red sauce dab on top. */
function riceCakePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dab = between(rng, -0.6, 0.6);
  return [
    shape(f.ellipse(0, -0.6, 6, 1.1), "white", 1, INKED),
    shape(f.ellipse(0, -0.7, 4, 0.6), "slate-400", 0.3),
    shape(f.ellipse(0, -2.3, 3.5, 1.5), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0.2, -1.9, 3.2, 0.9), "slate-400", 0.3),
    shape(f.ellipse(0, -4, 3.2, 1.45), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0.2, -3.6, 2.9, 0.85), "slate-400", 0.3),
    shape(f.ellipse(0, -5.6, 2.9, 1.4), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(-1.2, -6, 1, 0.4) + f.ellipse(-1.8, -4.4, 0.8, 0.3) + f.ellipse(-2, -2.7, 0.9, 0.3), "white", 0.7),
    shape(`M${f.p(-1 + dab, -6.5)}Q${f.p(0 + dab, -7.3)} ${f.p(1.3 + dab, -6.5)}Q${f.p(1.1 + dab, -5.6)} ${f.p(0.6 + dab, -5.3)}Q${f.p(0.3 + dab, -5.9)} ${f.p(-1 + dab, -6.5)}Z`, "red-600", 1),
  ];
}

/** A sealed envelope centred on the anchor, 7 wide and 5 tall at scale 1. */
function envelopePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.25, 0.25);
  return [
    shape(eRect(f, -3.5, -2.5, 7, 5), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-3.5, -2.5], [3.5, -2.5], [0, 0.4 + tilt]]), "slate-400", 0.25),
    shape(f.band([[-3.5, -2.5], [0, 0.4 + tilt], [3.5, -2.5]], 0.25), "slate-400", 1),
    shape(f.band([[-3.5, 2.5], [-1, 0.3]], 0.25) + f.band([[3.5, 2.5], [1, 0.3]], 0.25), "slate-400", 1),
    shape(f.circle(0, 0.4 + tilt, 0.95), "red-600", 1, INKED),
    shape(f.circle(-0.2, 0.2 + tilt, 0.45), "red-300", 0.6),
  ];
}

/** A cracked alchemical crucible on the anchor, 10 wide and 9 tall at scale 1, molten and glowing. */
function cruciblePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = between(rng, -0.6, 0.6);
  const crack: Point[] = [[-1, -6.4], [-1.9 + c, -4.6], [-0.7, -3.2], [-1.6, -1.2]];
  return [
    shape(f.ellipse(0, -7.4, 5.4, 1.8), "red-300", 0.22),
    shape(`M${f.p(-4.6, -6.7)}Q${f.p(-5.1, -1)} ${f.p(-2.6, 0)}L${f.p(2.6, 0)}Q${f.p(5.1, -1)} ${f.p(4.6, -6.7)}Z`, "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(-4.3, -5.8)}Q${f.p(-4.7, -1.5)} ${f.p(-3.4, -0.6)}L${f.p(-2.7, -0.6)}Q${f.p(-3.7, -2)} ${f.p(-3.5, -5.8)}Z`, "slate-400", 0.5),
    shape(`M${f.p(3.6, -5.8)}Q${f.p(4.4, -1.5)} ${f.p(2.6, -0.2)}L${f.p(4, -0.8)}Q${f.p(4.8, -2)} ${f.p(4.6, -5.8)}Z`, "slate-900", 0.3),
    shape(f.ellipse(0, -6.7, 5, 1.1), "slate-400", 1, INKED),
    shape(f.ellipse(0, -6.75, 4.2, 0.78), "red-300", 1),
    shape(f.ellipse(-0.4, -6.8, 2.2, 0.4), "white", 0.9),
    shape(f.band(crack, 0.8), "red-300", 0.9),
    shape(f.band(crack, 0.3), "white", 0.95),
  ];
}

/** A silver crown centred on the anchor, 6 wide at scale 1. */
function crownPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hi = between(rng, 0, 0.4);
  return [
    shape(f.poly([[-3, 0.3], [-3, -2.1], [-1.5, -0.7], [0, -2.5 - hi], [1.5, -0.7], [3, -2.1], [3, 0.3]]), "slate-100", 1, MAIN_INKED),
    shape(eRect(f, -3, 0.2, 6, 1.7), "slate-100", 1, MAIN_INKED),
    shape(eRect(f, -3, 1.2, 6, 0.7), "slate-400", 0.9),
    shape(f.poly([[-3, 0.3], [-2.4, -1.2], [-1.9, 0.3]]) + f.poly([[-0.9, 0.3], [0, -1.7], [0.2, 0.3]]), "white", 0.5),
    shape(f.circle(0, 0.95, 0.5), "red-500", 1),
    shape(f.circle(-1.8, 0.95, 0.38) + f.circle(1.8, 0.95, 0.38), "sky-400", 1),
    shape(f.circle(-3, -2.2, 0.3) + f.circle(3, -2.2, 0.3) + f.circle(0, -2.7 - hi, 0.32), "red-500", 1),
  ];
}

/** A glass vial on the anchor, 3 wide and 6 tall at scale 1, its liquid the main colour. */
function vialPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const fill = between(rng, -0.4, 0.5);
  return [
    shape(`M${f.p(-0.6, -5.2)}L${f.p(-0.6, -3)}Q${f.p(-1.6, -2.4)} ${f.p(-1.6, -1.4)}Q${f.p(-1.6, 0)} ${f.p(0, 0)}Q${f.p(1.6, 0)} ${f.p(1.6, -1.4)}Q${f.p(1.6, -2.4)} ${f.p(0.6, -3)}L${f.p(0.6, -5.2)}Z`, "sky-300", 0.45, INKED),
    shape(`M${f.p(-1.45, -1.9 + fill)}L${f.p(1.45, -1.9 + fill)}Q${f.p(1.55, -0.3)} ${f.p(0, -0.15)}Q${f.p(-1.55, -0.3)} ${f.p(-1.45, -1.9 + fill)}Z`, "lime-400", 1, MAIN),
    shape(f.poly([[-0.85, -6.2], [0.85, -6.2], [0.7, -5.1], [-0.7, -5.1]]), "brass-500", 1, INKED),
    shape(f.band([[-1.05, -2.3], [-1.1, -0.9]], 0.35) + f.band([[-0.35, -5], [-0.35, -3.6]], 0.2), "white", 0.7),
    shape(f.circle(0.4, -1.1 + fill, 0.2), "white", 0.5),
  ];
}

/** An American football centred on the anchor, 5 long at scale 1, turned by the stream. */
function ballPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -45, 45);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const q = (dx: number, dy: number): string => f.p(...T(dx, dy));
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  return [
    shape(`M${q(-2.5, 0)}Q${q(0, -2.5)} ${q(2.5, 0)}Q${q(0, 2.5)} ${q(-2.5, 0)}Z`, "wood-700", 1, MAIN_INKED),
    shape(`M${q(-2, -0.3)}Q${q(0, -1.5)} ${q(2, -0.3)}Q${q(0, -0.8)} ${q(-2, -0.3)}Z`, "white", 0.28),
    shape(`M${q(-2, 0.4)}Q${q(0, 1.5)} ${q(2, 0.4)}Q${q(0, 1.1)} ${q(-2, 0.4)}Z`, "slate-900", 0.3),
    shape(poly([[-1.7, -0.6], [-1.45, -0.6], [-1.45, 0.6], [-1.7, 0.6]]) + poly([[1.45, -0.6], [1.7, -0.6], [1.7, 0.6], [1.45, 0.6]]), "white", 0.95),
    shape(poly([[-0.9, -0.08], [0.9, -0.08], [0.9, 0.08], [-0.9, 0.08]]), "white", 1),
    shape(poly([[-0.65, -0.5], [-0.5, -0.5], [-0.5, 0.5], [-0.65, 0.5]]) + poly([[-0.08, -0.55], [0.08, -0.55], [0.08, 0.55], [-0.08, 0.55]]) + poly([[0.5, -0.5], [0.65, -0.5], [0.65, 0.5], [0.5, 0.5]]), "white", 1),
  ];
}

/** A floor hatch sprung open on the anchor, 20 wide at scale 1: its lid swung up behind, a green glow below. */
function hatchPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -1, 1);
  return [
    shape(f.poly([[-7.4, -4.4], [7.4, -4.4], [8.2 + lean, -13.4], [-8.2 + lean, -13.4]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6, -6], [6, -6], [6.5 + lean, -12], [-6.5 + lean, -12]]), "slate-900", 0.22),
    shape(f.poly([[-7.4, -4.4], [7.4, -4.4], [7.6 + lean, -6.6], [-7.6 + lean, -6.6]]), "slate-900", 0.15),
    shape(f.poly([[-7.5 + lean, -13.4], [-4 + lean, -13.4], [-4.4, -4.6], [-7.2, -4.6]]), "white", 0.22),
    shape(eRect(f, -5.8, -5.1, 1.8, 1) + eRect(f, 4, -5.1, 1.8, 1), "slate-700", 1),
    shape(f.ellipse(0, -1.4, 9.8, 3.5), "slate-700", 1, INKED),
    shape(f.ellipse(0, -1.4, 8.2, 2.8), "black", 1),
    shape(f.ellipse(0, -1.3, 6.8, 2.1), "lime-400", 0.35),
    shape(f.poly([[-6.5, -1.5], [6.5, -1.5], [9, -12], [-9, -12]]), "lime-400", 0.1),
  ];
}

/** A robotic clamp arm reaching up from the anchor toward the side it faces, 22 long at scale 1. */
function clampPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -1, 1.5);
  const wx = 8.5 + bend;
  return [
    shape(f.poly([[-3.2, 0], [3.2, 0], [2.2, -2.2], [-2.2, -2.2]]), "slate-700", 1, INKED),
    shape(f.band([[0, -3], [4, -9.5]], 2.4), "slate-400", 1, MAIN_INKED),
    shape(f.band([[4, -9.5], [wx, -16]], 2.2), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-0.9, -3.4], [3.1, -9.8]], 0.4) + f.band([[3.1, -9.8], [wx - 0.9, -16.3]], 0.4), "white", 0.4),
    shape(f.circle(0, -3, 1.5) + f.circle(4, -9.5, 1.6) + f.circle(wx, -16, 1.5), "slate-700", 1, INKED),
    shape(f.poly([[wx - 2.2, -16.8], [wx + 2.2, -16.8], [wx + 1.8, -18.4], [wx - 1.8, -18.4]]), "slate-700", 1, INKED),
    shape(f.poly([[wx - 2, -18.2], [wx - 0.7, -18.2], [wx - 0.3, -21], [wx - 0.9, -22], [wx - 2.2, -20.4]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[wx + 0.7, -18.2], [wx + 2, -18.2], [wx + 2.2, -20.4], [wx + 0.9, -22], [wx + 0.3, -21]]), "slate-400", 1, MAIN_INKED),
  ];
}

/** A small velvet pocket bursting open, centred on the anchor, 8 wide at scale 1, white light bursting from its mouth. */
function pocketPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 0, 1);
  const rays: string[] = [];
  const tips = [-4.2, -2.2, 0, 2.2, 4.2];
  for (let i = 0; i < 5; i += 1) {
    const x0 = -2 + i;
    rays.push(f.poly([[x0 - 0.4, -0.6], [x0 + 0.4, -0.6], [tips[i] ?? 0, -5.8 - (i % 2) * 1.2 - reach]]));
  }
  return [
    shape(f.circle(0, -1.5, 4.2), "white", 0.16),
    shape(f.circle(0, -1.5, 2.8), "white", 0.25),
    shape(rays.join(""), "white", 0.55),
    shape(`M${f.p(-3.4, -0.4)}Q${f.p(-4.3, 3.8)} ${f.p(0, 4)}Q${f.p(4.3, 3.8)} ${f.p(3.4, -0.4)}Q${f.p(0, 0.9)} ${f.p(-3.4, -0.4)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.ellipse(0, -0.6, 3.3, 0.8), "white", 1),
    shape(`M${f.p(-3.2, 0.5)}Q${f.p(0, 1.6)} ${f.p(3.2, 0.5)}`.concat(`L${f.p(3.2, 0.9)}Q${f.p(0, 2)} ${f.p(-3.2, 0.9)}Z`), "brass-500", 1),
    shape(`M${f.p(-2.8, 0.9)}Q${f.p(-3.6, 3)} ${f.p(-2.2, 3.6)}Q${f.p(-2.9, 2.6)} ${f.p(-2.4, 0.9)}Z`, "white", 0.25),
    shape(`M${f.p(2.4, 1.2)}Q${f.p(3.8, 3.2)} ${f.p(0.4, 3.9)}Q${f.p(3.2, 3)} ${f.p(2, 1.2)}Z`, "slate-900", 0.25),
    shape(sparkle(f, 0, -3.4 - reach, 1.7), "white", 0.95),
  ];
}

/** A velvet sack on the anchor, 10 wide and 12 tall at scale 1, silver coins peeking from its neck. */
function sackPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flop = between(rng, -0.5, 0.5);
  return [
    shape(`M${f.p(-1.6, -8)}Q${f.p(-5.2, -4)} ${f.p(-4.8, -1)}Q${f.p(-4.5, 0)} ${f.p(0, 0)}Q${f.p(4.5, 0)} ${f.p(4.8, -1)}Q${f.p(5.2, -4)} ${f.p(1.6, -8)}Z`, "red-900", 1, MAIN_INKED),
    shape(f.poly([[-1.6, -8], [1.6, -8], [2.8 + flop, -11.2], [1.2, -10.2], [0, -11.6], [-1.2, -10.2], [-2.8 + flop, -11.2]]), "red-900", 1, MAIN_INKED),
    shape(f.circle(-1.5, -10.9, 0.9) + f.circle(0.5, -11.6, 0.9) + f.circle(2.2, -10.7, 0.8), "slate-100", 1, INKED),
    shape(f.band([[-1.8, -8], [1.8, -8]], 0.8) + f.circle(2.2, -7.6, 0.45), "brass-500", 1),
    shape(`M${f.p(-3.6, -5.6)}Q${f.p(-4.4, -3)} ${f.p(-3.2, -1)}Q${f.p(-4, -3)} ${f.p(-3.2, -5.6)}Z`, "white", 0.25),
    shape(`M${f.p(2.6, -6)}Q${f.p(5, -3)} ${f.p(3.6, -0.4)}L${f.p(4.8, -1)}Q${f.p(5.2, -4)} ${f.p(2.6, -6)}Z`, "slate-900", 0.3),
    shape(f.band([[-0.8, -7.6], [-1.6, -4], [-1.3, -1.2]], 0.3) + f.band([[0.9, -7.6], [1.4, -4]], 0.3), "slate-900", 0.22),
  ];
}

/** A Legendary's towering chaos wheel standing on the anchor, 34 wide and 40 tall at scale 1: cabinet, coloured wheel, reels window, lever. */
function chaosWheelPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cx = 0;
  const cy = -23;
  const turn = between(rng, 0, 36);
  const R = 10.8;
  const sector = (k: number): string => {
    const a0 = (turn + k * 36) * DEGREES;
    const a1 = (turn + k * 36 + 36) * DEGREES;
    return `M${f.p(cx, cy)}L${f.p(cx + R * Math.cos(a0), cy + R * Math.sin(a0))}A${fmt(f.r(R))} ${fmt(f.r(R))} 0 0 ${f.sweep(1)} ${f.p(cx + R * Math.cos(a1), cy + R * Math.sin(a1))}Z`;
  };
  const dots: string[] = [];
  for (let k = 0; k < 10; k += 1) {
    const a = (turn + k * 36) * DEGREES;
    dots.push(f.circle(cx + 11.7 * Math.cos(a), cy + 11.7 * Math.sin(a), 0.5));
  }
  const reels = [-5, -1.3, 2.4].map((x) => eRect(f, x, -9, 2.6, 4.4)).join("");
  const colours: PaletteName[] = ["red-500", "sky-400", "lime-400", "orange-400", "violet-300"];
  return [
    shape(`M${f.p(-14, 0)}L${f.p(-14, -30)}Q${f.p(-14, -38)} ${f.p(0, -40)}Q${f.p(14, -38)} ${f.p(14, -30)}L${f.p(14, 0)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.band([[14, -17], [15.6, -25.5]], 1), "slate-400", 1, INKED),
    shape(f.circle(15.6, -27, 1.5), "red-500", 1, INKED),
    shape(f.circle(cx, cy, 12.6), "slate-900", 1),
    ...colours.map((c, i) => shape(sector(i) + sector(i + 5), c, 1)),
    shape(dots.join("") + f.circle(cx, cy, 1.6) + f.poly([[-1.2, -36.3], [1.2, -36.3], [0, -33.9]]), "white", 0.95),
    shape(eRect(f, -8.5, -10, 17, 6.4), "slate-900", 1, INKED),
    shape(reels, "violet-300", 0.5),
  ];
}

/** A giant stone golem's fist centred on the anchor, 10 wide at scale 1, punching toward the side it faces. */
function fistPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-5, -3.2], [-1, -4.2], [3.5, -4.4], [5, -3], [5, 3], [3.5, 4.2], [-1, 4], [-5, 3.2]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-5, -3.2], [-1, -4.2], [3.5, -4.4], [5, -3], [3.5, -3.2], [-1, -3], [-5, -2.1]]), "white", 0.3),
    shape(f.poly([[-5, 2.4], [-1, 2.8], [3.5, 3], [5, 3], [3.5, 4.2], [-1, 4], [-5, 3.2]]), "slate-900", 0.25),
    shape(f.band([[1.6, -2.1 + j], [5, -2]], 0.4) + f.band([[1.6, -0.3], [5, -0.2]], 0.4) + f.band([[1.6, 1.6 - j], [5, 1.6]], 0.4), "slate-900", 0.4),
    shape(f.band([[1.7, -3.6], [1.7, 3.6]], 0.4), "slate-900", 0.3),
    shape(f.band([[-3.6, -3], [-3.6, 3]], 0.5) + f.band([[-0.8, -4], [-0.2, -2.6], [-1.3, -1.3], [-0.7, 0.3], [-1.6, 1.6]], 0.3), "slate-900", 0.55),
    shape(f.circle(4, -3.2, 0.5) + f.circle(4, -1.1, 0.5) + f.circle(4, 0.9, 0.5) + f.circle(4, 2.9, 0.5), "white", 0.3),
  ];
}

/** A healing fountain standing on the anchor, 16 wide and 14 tall at scale 1, in a green glow. */
function fountainPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const k = between(rng, 4.4, 5.4);
  const jet = (sign: number): Point[] => {
    const pts: Point[] = [];
    for (let i = 0; i <= 6; i += 1) {
      const t = i / 6;
      pts.push([sign * 6.5 * t, -9.2 + 4.8 * t - k * 4 * t * (1 - t)]);
    }
    return pts;
  };
  return [
    shape(f.circle(0, -6, 9), "green-400", 0.12),
    shape(f.circle(0, -6, 6), "green-400", 0.15),
    shape(`M${f.p(-8, -3.5)}Q${f.p(-7, 0)} ${f.p(-4, 0)}L${f.p(4, 0)}Q${f.p(7, 0)} ${f.p(8, -3.5)}Z`, "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-1.5, -3.4], [1.5, -3.4], [0.8, -8], [-0.8, -8]]), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0, -3.6, 8, 1.5), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0, -3.6, 6.8, 1.1), "sky-300", 0.85),
    shape(f.poly([[-2, -8], [2, -8], [1.2, -9.2], [-1.2, -9.2]]), "slate-100", 1, INKED),
    shape(f.band(jet(-1), 0.7), "sky-300", 0.8),
    shape(f.band(jet(1), 0.7), "sky-300", 0.8),
    shape(f.band([[0, -9.2], [0, -13]], 0.8), "sky-300", 0.8),
    shape(f.poly([[3.5, -0.4], [7.2, -0.6], [7.6, -2.4], [4.4, -2]]), "slate-900", 0.18),
  ];
}

/** A crystal chandelier hanging from the anchor, 16 wide and 10 tall at scale 1. */
function chandelierPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.3, 0.3);
  const drops: string[] = [];
  for (const x of [-7, -5, -2.5, 0, 2.5, 5, 7]) {
    const y0 = 3.3 + 3 * (x / 7.5) ** 2;
    const l = x === 0 ? 6.2 : 2.6 + sway * (x > 0 ? 1 : -1);
    drops.push(f.poly([[x, y0], [x + 0.65, y0 + l * 0.4], [x, y0 + l], [x - 0.65, y0 + l * 0.4]]));
  }
  return [
    shape(f.circle(0, 3, 6), "white", 0.1),
    shape(f.band([[0, 0], [0, 2]], 0.4), "slate-400", 1),
    shape(f.band([[-7.8, 6.3], [-4.6, 4.2], [0, 3.2], [4.6, 4.2], [7.8, 6.3]], 0.7), "slate-400", 1, INKED),
    shape(f.circle(0, 3, 1), "slate-400", 1, INKED),
    shape(drops.join(""), "violet-300", 1, MAIN_INKED),
    shape(f.circle(-7.6, 5.4, 0.5) + f.circle(-3.8, 3.5, 0.5) + f.circle(3.8, 3.5, 0.5) + f.circle(7.6, 5.4, 0.5) + f.circle(0, 1.6, 0.5), "white", 1),
    shape(f.band([[-0.3, 4.4], [-0.3, 7.6]], 0.25) + f.band([[-2.8, 4.1], [-2.8, 5.4]], 0.2) + f.band([[2.2, 4.1], [2.2, 5.4]], 0.2), "white", 0.6),
  ];
}

/** A Mythic's huge gloved hand reaching down from above, wrist at the anchor, 30 tall at scale 1, with puppet strings running 30 more below. */
function puppetHandPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -3.6, -2.4);
  const sway = between(rng, -1, 1);
  const fingers: Point[][] = [
    [[-6.5, 8], [-10, 11], [-13, 14]],
    [[-4.6, 12], [-5.2 + tilt * 0.4, 20], [-5 + tilt, 28.5]],
    [[-1.4, 12], [-1.4, 21], [-1.5, 30]],
    [[1.8, 12], [2.4, 21], [3.2, 28]],
    [[5, 12], [6, 18], [7.2, 24]],
  ];
  const tips = fingers.map((p) => p[p.length - 1] ?? [0, 0]);
  return [
    shape(f.poly([[-7, -7], [7, -7], [6.6, 0], [-6.6, 0]]), "slate-100", 1, INKED),
    shape(f.poly([[-7.2, -2], [7.2, -2], [7.1, -0.9], [-7.1, -0.9]]), "slate-400", 0.6),
    shape(f.poly([[-6.5, 0], [6.5, 0], [7.2, 13], [-7.2, 13]]), "slate-900", 1, MAIN_INKED),
    shape(fingers.map((p) => f.band(p, 2.7)).join(""), "slate-900", 1, MAIN_INKED),
    shape(f.band([[-6.6, 1], [-7.1, 12.5]], 0.8) + f.band([[-5.6, 13], [-6.2 + tilt * 0.4, 20], [-6 + tilt, 28.5]], 0.5) + f.band([[-2.4, 13], [-2.5, 30]], 0.5) + f.band([[0.9, 13], [1.4, 21], [2.2, 28]], 0.5) + f.band([[4.1, 13], [5.1, 18], [6.3, 24]], 0.5), "slate-700", 0.9),
    shape(tips.map(([x, y]) => f.band([[x, y], [x + sway * 0.4, y + 15], [x + sway, y + 30]], 0.28)).join(""), "slate-100", 0.5),
    shape(f.circle(-4, -3.8, 0.6) + f.circle(4, -3.8, 0.6), "slate-900", 0.6),
  ];
}

/** An anvil on a stump, standing on the anchor, 14 wide and 8 tall at scale 1, its horn to the facing side's back. */
function anvilPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const D = 1.2;
  const P = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => [x, y - D] as const));
  const g = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-5.4, 0], [5.4, 0], [4.8, -2.2], [-4.8, -2.2]]), "wood-700", 1, INKED),
    shape(f.band([[-3.6, -0.4], [-3.4 + g, -1.9]], 0.3) + f.band([[1.2, -0.3], [1.4 + g, -1.9]], 0.3) + f.band([[3.8, -0.5], [3.7, -1.8]], 0.3), "wood-800", 0.6),
    shape(P([[-7, -5.4], [-3.2, -6.6], [6.6, -6.6], [6.6, -5.2], [4.8, -4.6], [2.6, -3.6], [2.8, -2.4], [4.4, -1.8], [4.4, -1], [-4.4, -1], [-4.4, -1.8], [-2.6, -2.4], [-2.6, -3.6], [-3.4, -4.4], [-5.2, -4.6]]), "slate-700", 1, MAIN_INKED),
    shape(P([[-3.2, -6.6], [6.6, -6.6], [6.6, -5.8], [-3.8, -5.8]]) + P([[-7, -5.4], [-3.2, -6.6], [-3.4, -5.8], [-6.2, -5.4]]), "slate-400", 1),
    shape(P([[-2.6, -2.4], [2.8, -2.4], [4.4, -1.8], [4.4, -1], [-4.4, -1], [-4.4, -1.8]]), "slate-900", 0.3),
    shape(P([[-1.5, -6.3], [1.5, -6.3], [1.3, -6.1], [-1.7, -6.1]]), "white", 0.6),
  ];
}

/** Two glowing rails running from the box's left and right edges in to the anchor at its height (absolute box coordinates), in slight perspective. */
function railsPart(at: PartPlace, rng: Rng): KitShape[] {
  const drop = at.s * between(rng, 5, 7);
  const wide = at.s * 3.2;
  const narrow = at.s * 1.2;
  const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
  const sides = [
    { x0: EDGE, x1: at.x - 0.8 * at.s },
    { x0: FAR_EDGE, x1: at.x + 0.8 * at.s },
  ];
  const rail = (s: { x0: number; x1: number }, grow: number): string => {
    const y0 = at.y + drop;
    return `M${pt(s.x0, y0 - (wide * grow) / 2)}L${pt(s.x1, at.y - (narrow * grow) / 2)}L${pt(s.x1, at.y + (narrow * grow) / 2)}L${pt(s.x0, y0 + (wide * grow) / 2)}Z`;
  };
  const ties = (s: { x0: number; x1: number }): string => {
    let d = "";
    for (let i = 0; i < 9; i += 1) {
      const t = (i + 0.5) / 9;
      const cx = lerp(s.x0, s.x1, t);
      const cy = lerp(at.y + drop, at.y, t);
      const h = lerp(wide * 1.9, narrow * 1.9, t);
      const w = lerp(at.s * 2.2, at.s * 0.7, t);
      d += rectPath(cx - w / 2, cy - h / 2, w, h);
    }
    return d;
  };
  const [left, right] = sides as [(typeof sides)[number], (typeof sides)[number]];
  return [
    shape(rail(left, 3), "cyan-400", 0.14),
    shape(rail(right, 3), "cyan-400", 0.14),
    shape(ties(left) + ties(right), "slate-700", 1),
    shape(rail(left, 1) + rail(right, 1), "cyan-400", 1, MAIN),
    shape(rail(left, 0.4) + rail(right, 0.4), "white", 0.9),
  ];
}

/** A glowing green syringe centred on the anchor, 12 long at scale 1, turned by the stream, its needle to the facing side. */
function syringePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -45, 45);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const tick: string[] = [];
  for (let i = 0; i < 3; i += 1) tick.push(poly([[0 + i * 1.5, 0.9], [0.15 + i * 1.5, 0.9], [0.15 + i * 1.5, 1.3], [0 + i * 1.5, 1.3]]));
  return [
    shape(f.ellipse(...T(0, 0), 7.4, 3.2), "green-400", 0.14),
    shape(f.ellipse(...T(-0.5, 0), 4.6, 2.2), "green-400", 0.2),
    shape(poly([[-6.2, -1.6], [-5.6, -1.6], [-5.6, 1.6], [-6.2, 1.6]]) + poly([[-5.6, -0.3], [-2.2, -0.3], [-2.2, 0.3], [-5.6, 0.3]]), "slate-400", 1, INKED),
    shape(poly([[-3.4, -2.1], [-2.8, -2.1], [-2.8, 2.1], [-3.4, 2.1]]), "slate-400", 1, INKED),
    shape(poly([[-2.8, -1.3], [3, -1.3], [3, 1.3], [-2.8, 1.3]]), "sky-300", 0.45, INKED),
    shape(poly([[-1.6, -0.95], [2.8, -0.95], [2.8, 0.95], [-1.6, 0.95]]), "green-400", 1, MAIN),
    shape(poly([[-2.2, -0.95], [-1.6, -0.95], [-1.6, 0.95], [-2.2, 0.95]]), "slate-700", 1),
    shape(poly([[3, -0.75], [3.9, -0.75], [3.9, 0.75], [3, 0.75]]), "slate-700", 1, INKED),
    shape(poly([[3.9, -0.3], [6, -0.08], [6, 0.08], [3.9, 0.3]]), "slate-100", 1),
    shape(tick.join("") + poly([[-2, -1.05], [2.6, -1.05], [2.6, -0.8], [-2, -0.8]]), "white", 0.55),
  ];
}

/** A briefing podium standing on the anchor, 12 wide and 14 tall at scale 1, a microphone on its top. */
function podiumPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.4, 0.4);
  const star: Point[] = [];
  for (let i = 0; i < 10; i += 1) {
    const a = (-90 + i * 36) * DEGREES;
    const r = i % 2 === 0 ? 1.8 : 0.75;
    star.push([r * Math.cos(a), -5.2 + r * Math.sin(a)]);
  }
  return [
    shape(f.poly([[-5, 0], [5, 0], [5.6, -9.6], [-5.6, -9.6]]), "wood-700", 1, MAIN_INKED),
    shape(f.poly([[-5, 0], [-3.6, 0], [-4.1, -9.6], [-5.6, -9.6]]), "white", 0.14),
    shape(f.poly([[3.6, 0], [5, 0], [5.6, -9.6], [4.1, -9.6]]), "slate-900", 0.28),
    shape(f.poly([[-6.2, -9.6], [6.2, -9.6], [5.4, -11.2], [-5.4, -11.2]]), "wood-600", 1, INKED),
    shape(f.circle(0, -5.2, 2.9), "slate-100", 1, INKED),
    shape(f.poly(star), "blue-800", 1),
    shape(f.band([[3.2, -11], [3.6 + sway, -13], [2.4 + sway, -13.9]], 0.4), "slate-900", 1),
    shape(f.ellipse(2.2 + sway, -13.9, 0.85, 0.65), "slate-700", 1, INKED),
  ];
}

/** A streetlamp standing on the anchor, 30 tall at scale 1, its head a violet light in a halo. */
function streetlampPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glow = between(rng, 0.9, 1.1);
  return [
    shape(f.circle(0, -27.6, 8 * glow), "violet-300", 0.1),
    shape(f.circle(0, -27.6, 5.4 * glow), "violet-300", 0.18),
    shape(f.poly([[-1.8, 0], [1.8, 0], [1.1, -4], [-1.1, -4]]), "slate-900", 1, INKED),
    shape(f.band([[0, -4], [0, -25.8]], 1), "slate-900", 1, INKED),
    shape(f.band([[-0.3, -5], [-0.3, -25]], 0.3), "slate-700", 0.9),
    shape(f.poly([[-1.8, -25.8], [1.8, -25.8], [1.5, -26.6], [-1.5, -26.6]]), "slate-700", 1, INKED),
    shape(f.poly([[-1.5, -26.6], [1.5, -26.6], [2.1, -29.2], [-2.1, -29.2]]), "violet-300", 1, MAIN_INKED),
    shape(f.ellipse(0, -27.9, 0.7, 1.1), "white", 0.8),
    shape(f.poly([[-2.8, -29.2], [2.8, -29.2], [0, -31]]), "slate-900", 1, INKED),
  ];
}

/** A tidy steel scrap bin standing on the anchor, 16 wide and 12 tall at scale 1. */
function binPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ribs: string[] = [];
  for (let i = 0; i < 5; i += 1) {
    const x = -5 + i * 2.5;
    ribs.push(f.band([[x, -8.6], [x * 0.84, -1.8]], 0.7));
  }
  const dent = between(rng, 0, 0.5);
  return [
    shape(f.poly([[-7.6, -10.8], [7.6, -10.8], [6.2, 0], [-6.2, 0]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-7.4, -10.4], [7.4, -10.4], [7.2, -8.4 + dent], [-7.2, -8.4 + dent]]), "slate-900", 0.38),
    shape(ribs.join(""), "slate-900", 0.28),
    shape(f.poly([[-7.6, -10.8], [-5.8, -10.8], [-4.6, 0], [-6.2, 0]]), "white", 0.22),
    shape(f.poly([[-6.4, -1.4], [6.4, -1.4], [6.2, 0], [-6.2, 0]]), "slate-700", 0.8),
    shape(f.poly([[-8, -12], [8, -12], [8, -10.4], [-8, -10.4]]), "slate-100", 1, INKED),
    shape(f.poly([[-7.4, -11.8], [-1, -11.8], [-1, -11.3], [-7.4, -11.3]]), "white", 0.6),
  ];
}

/** A bed with a sleeper standing on the anchor, 26 wide and 10 tall at scale 1, the headboard on the far side. */
function bedPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hump = between(rng, 0, 0.5);
  return [
    shape(eRect(f, -12.6, -1.8, 1.2, 1.8) + eRect(f, 11.4, -1.8, 1.2, 1.8), "wood-800", 1),
    shape(f.poly([[-13, -9.6], [-11.3, -9.6], [-11.3, -4.2], [11.4, -4.2], [11.4, -6.6], [13, -6.6], [13, -1.8], [-13, -1.8]]), "wood-700", 1, INKED),
    shape(f.ellipse(-8.8, -5.4, 2.6, 1.4), "white", 1, INKED),
    shape(f.circle(-8.8, -6.9, 1.6), "red-300", 1, INKED),
    shape(`M${f.p(-10.5, -6.8)}Q${f.p(-10.3, -9)} ${f.p(-8.4, -8.7)}Q${f.p(-7.1, -8.4)} ${f.p(-7.1, -7.2)}Q${f.p(-8.5, -7.9)} ${f.p(-10.5, -6.8)}Z`, "wood-800", 1),
    shape(f.band([[-9.7, -6.8], [-9.2, -6.5], [-8.8, -6.8]], 0.22) + f.band([[-8.4, -6.8], [-7.9, -6.5], [-7.5, -6.8]], 0.22), "slate-900", 1),
    shape(`M${f.p(-6.4, -4.2)}Q${f.p(-6.5, -7.4 - hump)} ${f.p(-2, -7.2 - hump)}Q${f.p(4, -7.5 - hump)} ${f.p(11.4, -6.4)}L${f.p(11.4, -4.2)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.band([[-1, -7.2], [-0.4, -4.5]], 0.4) + f.band([[4, -7.2], [4.5, -4.6]], 0.4) + f.band([[8, -6.9], [8.3, -4.6]], 0.4), "slate-900", 0.22),
    shape(f.band([[-5, -7.1], [3, -7.1]], 0.4), "white", 0.25),
  ];
}

/** A string of bunting sagging between 25 left and 25 right of the anchor at its height, triangle flags grouped by colour. */
function buntingPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 4.5, 6.5);
  const yAt = (x: number): number => sag * (1 - (x / 25) ** 2);
  const line: Point[] = [];
  for (let i = 0; i <= 10; i += 1) {
    const x = -25 + i * 5;
    line.push([x, yAt(x)]);
  }
  const groups: string[] = ["", "", "", ""];
  for (let i = 0; i < 16; i += 1) {
    const x = -22.5 + i * 3;
    const y = yAt(x);
    groups[i % 4] += f.poly([[x - 1.1, y], [x + 1.1, y], [x, y + 3]]);
  }
  const colours: PaletteName[] = ["red-500", "sky-400", "white", "lime-400"];
  return [
    shape(f.band(line, 0.4), "slate-700", 1),
    ...colours.map((c, i) => shape(groups[i] ?? "", c, 1, i === 0 ? MAIN_INKED : INKED)),
  ];
}

/** A strip of five vanity bulbs centred on the anchor, 24 wide at scale 1, each in an orange glow. */
function bulbsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const xs = [-9.6, -4.8, 0, 4.8, 9.6];
  const lit = between(rng, 0.8, 1);
  return [
    shape(f.ellipse(0, 0, 13.6, 3.6), "orange-400", 0.12),
    shape(eRect(f, -12, -1.2, 24, 2.4), "slate-700", 1, INKED),
    shape(xs.map((x) => f.circle(x, 0, 2.35)).join(""), "orange-400", 0.3 * lit),
    shape(xs.map((x) => f.circle(x, 0, 1.55)).join(""), "white", 1, MAIN),
    shape(xs.map((x) => f.circle(x - 0.4, -0.4, 0.45)).join(""), "orange-400", 0.5),
  ];
}

/** A ball of gems and trinkets centred on the anchor, radius 6 at scale 1. */
function gemBallPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rot = whole(rng, 0, 3);
  const groups: string[] = ["", "", "", ""];
  for (let k = 0; k < 8; k += 1) {
    const a = (22.5 + k * 45) * DEGREES;
    const x = 4 * Math.cos(a);
    const y = 4 * Math.sin(a);
    groups[(k + rot) % 4] += f.poly([[x, y - 1.35], [x + 1.05, y], [x, y + 1.35], [x - 1.05, y]]);
  }
  const colours: PaletteName[] = ["green-500", "red-500", "sky-400", "violet-500"];
  return [
    shape(f.circle(0, 0, 6), "brass-600", 1, MAIN_INKED),
    shape(f.ellipse(2.3, 2.8, 3.2, 2.5), "slate-900", 0.18),
    ...colours.map((c, i) => shape(groups[i] ?? "", c, 1)),
    shape(f.circle(0, 0, 1.6) + f.circle(0, 0, 0.95), "slate-100", 1),
    shape(f.ellipse(-2.3, -3, 2.3, 1.3), "white", 0.18),
    shape(sparkle(f, -3.6, -3.5, 1.2) + sparkle(f, 3.7, 2.3, 0.9), "white", 0.95),
  ];
}

/** An unrolled map centred on the anchor, 8 wide and 6 tall at scale 1, its ends rolled. */
function mapPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -0.4, 0.4);
  const dots: string[] = [];
  const route: Point[] = [[-2.4, 2.3], [-1.6, 1.5], [-0.6, 1.2], [0.4, 0.4], [1.3, -0.2], [1.6, -1.2]];
  for (const [x, y] of route) dots.push(f.circle(x, y, 0.22));
  return [
    shape(eRect(f, -3.2, -3, 6.4, 6), "slate-100", 1, MAIN_INKED),
    shape(eRect(f, -4, -3.3, 1, 6.6) + eRect(f, 3, -3.3, 1, 6.6), "slate-400", 1, INKED),
    shape(eRect(f, -3.6, -3.2, 0.25, 6.4) + eRect(f, 3.35, -3.2, 0.25, 6.4), "white", 0.5),
    shape(f.band([[-2.4, -3], [-0.8 + bend, -1], [-1.5, 0.5], [0.2, 2], [0.6, 3]], 0.7), "blue-600", 1),
    shape(f.ellipse(-2, 1.6, 0.9, 0.7) + f.ellipse(1.9, -1.9, 1.1, 0.8) + f.ellipse(2, 1.8, 0.8, 0.6), "green-500", 1),
    shape(dots.join("") + f.circle(1.9, -2.1, 0.45), "red-600", 1),
  ];
}

/** A stitched rag doll standing on the anchor, 6 tall at scale 1, yarn hair. */
function dollPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hair = between(rng, -0.2, 0.2);
  return [
    shape(f.band([[-0.65, -1.6], [-0.7, 0]], 0.9) + f.band([[0.65, -1.6], [0.7, 0]], 0.9), "red-300", 1, MAIN_INKED),
    shape(f.poly([[-1.1, -3.7], [1.1, -3.7], [1.3, -1.5], [-1.3, -1.5]]), "red-300", 1, MAIN_INKED),
    shape(f.band([[-1, -3.2], [-2.4, -1.7]], 0.8) + f.band([[1, -3.2], [2.4, -1.7]], 0.8), "red-300", 1, MAIN_INKED),
    shape(f.circle(0, -4.7, 1.2), "red-300", 1, MAIN_INKED),
    shape(f.poly([[-1.3, -5], [-1.6, -5.9], [-0.8, -5.6 + hair], [-0.5, -6.4], [0.1, -5.8], [0.7, -6.4], [0.9, -5.6 - hair], [1.6, -5.9], [1.3, -5], [0.4, -5.5], [-0.4, -5.5]]), "orange-400", 1, INKED),
    shape(f.circle(-0.45, -4.9, 0.27) + f.circle(0.45, -4.9, 0.27), "slate-900", 1),
    shape(f.band([[-0.5, -4.2], [0.5, -4.2]], 0.15) + f.band([[0, -3.3], [0, -2.9]], 0.2) + f.band([[0, -2.6], [0, -2.2]], 0.2) + f.band([[0, -1.9], [0, -1.6]], 0.2) + f.band([[-1.9, -2.4], [-1.7, -2.1]], 0.15) + f.band([[0.65, -0.9], [0.7, -0.5]], 0.15) + f.band([[-0.7, -0.9], [-0.7, -0.5]], 0.15), "slate-900", 0.9),
  ];
}

/** An archer's bow centred on the anchor, 10 tall at scale 1, curving away from the facing side. */
function bowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const belly = between(rng, 2.2, 2.8);
  const stave: Point[] = [];
  for (let i = 0; i <= 8; i += 1) {
    const y = -5 + i * 1.25;
    stave.push([1 - belly * (1 - (y / 5) ** 2), y]);
  }
  return [
    shape(f.band([[1, -5], [1, 5]], 0.16), "white", 0.9),
    shape(f.band(stave, 0.8), "wood-600", 1, MAIN_INKED),
    shape(f.band(stave.map(([x, y]) => [x - 0.2, y] as const), 0.2), "white", 0.3),
    shape(f.band([[-belly + 1 - 0.1, -0.9], [-belly + 1 - 0.1, 0.9]], 1.2), "wood-800", 1, INKED),
    shape(f.circle(1, -5, 0.35) + f.circle(1, 5, 0.35), "wood-800", 1),
  ];
}

/** A glowing eye symbol centred on the anchor, 6 wide at scale 1. */
function eyePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const look = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(0, 0, 5, 3.6), "cyan-400", 0.12),
    shape(f.ellipse(0, 0, 3.9, 2.4), "cyan-400", 0.18),
    shape(`M${f.p(-3, 0)}Q${f.p(0, -3.4)} ${f.p(3, 0)}Q${f.p(0, 3.4)} ${f.p(-3, 0)}Z`, "white", 1, INKED),
    shape(f.circle(look, 0, 1.6), "cyan-400", 1, MAIN),
    shape(f.circle(look, 0, 0.7), "black", 1),
    shape(f.circle(look - 0.5, -0.5, 0.3), "white", 0.9),
  ];
}

/** A big plain shipping crate standing on the anchor, opened, 40 wide and 36 tall at scale 1: its front panel fallen open toward the viewer, straw spilling. */
function crateBigPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, -2, 2);
  return [
    shape(f.ellipse(0, 0.5, 21, 1.6), "slate-900", 0.25),
    shape(eRect(f, -14.5, -33.5, 29, 25.5), "wood-800", 1),
    shape(eRect(f, -14.5, -28, 29, 0.6) + eRect(f, -14.5, -21, 29, 0.6) + eRect(f, -14.5, -14, 29, 0.6), "wood-700", 0.9),
    shape(eRect(f, -17, -36, 34, 28) + eRect(f, -14.5, -33.5, 29, 25.5), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-15.7, -35], [-15.7, -9]], 0.5) + f.band([[15.7, -35], [15.7, -9]], 0.5) + f.band([[-16, -34.8], [16, -34.8]], 0.5), "wood-700", 0.9),
    shape(f.poly([[-17, -8], [17, -8], [20, 0], [-20, 0]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-9, -8], [-10.6, 0]], 0.8) + f.band([[0, -8], [0, 0]], 0.8) + f.band([[9, -8], [10.6, 0]], 0.8), "wood-700", 0.9),
    shape(f.poly([[-13, -8.5], [-12, -14], [-9, -13], [-8, -19 + h], [-5, -16], [-2, -23], [1, -17], [4, -21 - h], [6, -15], [9, -19], [11, -13], [13, -14], [13, -8.5]]), "brass-500", 1),
    shape(f.poly([[-9, -8.5], [-7, -4.4], [-4.5, -5.8], [-1, -2.6], [2, -5], [5, -3], [8, -6], [10, -8.5]]), "brass-500", 1),
    shape(f.band([[-6, -12], [-3.5, -16]], 0.35) + f.band([[3, -13], [6, -17]], 0.35) + f.band([[-2, -5.5], [1, -3.6]], 0.35), "brass-600", 0.8),
  ];
}
/** A ripe purple fig splitting open, centred on the anchor, 8 wide and 8 tall at scale 1: two halves, red flesh with seeds, a stem. */
function figPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gap = between(rng, 0.8, 1.4);
  return [
    shape(f.band([[0, -4.2], [0.4, -5.4], [1.4, -6]], 0.6), "green-700", 1),
    shape(`M${f.p(-gap, 3.6)}Q${f.p(-4.6, 2)} ${f.p(-3.6, -2)}Q${f.p(-2, -4.4)} ${f.p(-gap, -4)}Z`, "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(gap, 3.6)}Q${f.p(4.6, 2)} ${f.p(3.6, -2)}Q${f.p(2, -4.4)} ${f.p(gap, -4)}Z`, "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(-gap + 0.4, 3)}Q${f.p(-3.2, 1.6)} ${f.p(-2.6, -1.8)}Q${f.p(-1.8, -3.4)} ${f.p(-gap + 0.4, -3.2)}Z${f.poly([[gap - 0.4, -3.2], [gap - 0.4, 3], [2.6, 1.6], [2.2, -1.8]])}`, "red-400", 1),
    shape(f.circle(-1.8, -0.4, 0.28) + f.circle(-1.4, 1.2, 0.28) + f.circle(1.8, -0.8, 0.28) + f.circle(1.5, 0.9, 0.28) + f.circle(1.9, 2, 0.28), "white", 0.85),
    shape(f.ellipse(-3, -1.6, 0.5, 1), "white", 0.3),
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

/** A plus-shaped sparkle about a point, `size` its reach. */
function plusKitA(f: Local, cx: number, cy: number, size: number): string {
  const t = size * 0.28;
  return f.poly([[cx - t, cy - size], [cx + t, cy - size], [cx + t, cy - t], [cx + size, cy - t], [cx + size, cy + t], [cx + t, cy + t], [cx + t, cy + size], [cx - t, cy + size], [cx - t, cy + t], [cx - size, cy + t], [cx - size, cy - t], [cx - t, cy - t]]);
}

/** An arc band about the anchor from angle `from` to `to` (degrees, clockwise), `width` thick at radius `r`. */
function arcKitA(f: Local, r: number, width: number, from: number, to: number): string {
  const ro = r + width / 2;
  const ri = r - width / 2;
  const at = (radius: number, deg: number): string => f.p(radius * Math.cos(deg * DEGREES), radius * Math.sin(deg * DEGREES));
  const large = Math.abs(to - from) > 180 ? 1 : 0;
  return `M${at(ro, from)}A${fmt(f.r(ro))} ${fmt(f.r(ro))} 0 ${large} ${f.sweep(1)} ${at(ro, to)}L${at(ri, to)}A${fmt(f.r(ri))} ${fmt(f.r(ri))} 0 ${large} ${f.sweep(0)} ${at(ri, from)}Z`;
}

/** A ring about the anchor, an outer circle with a hole. */
function ringKitA(f: Local, r: number, width: number): string {
  return f.circle(0, 0, r) + f.circle(0, 0, r - width);
}

/** A flame outline `k` times the full one, base at (0, `base`) of the part's frame, tip leaning by `lean`. */
function flameKitA(f: Local, k: number, base: number, lean: number): string {
  const p = (x: number, y: number): string => f.p(x * k + lean * k * (y / 6) ** 2, base + y * k);
  return `M${p(-2.4, -0.8)}C${p(-3.2, -3)} ${p(-0.8, -4)} ${p(0.4, -6)}C${p(0.6, -4)} ${p(3, -3.2)} ${p(2.4, -0.8)}C${p(2, 0.6)} ${p(-2, 0.6)} ${p(-2.4, -0.8)}Z`;
}

/** A teardrop, tip up, centred on a point, `r` its round's radius. */
function dropKitA(f: Local, cx: number, cy: number, r: number): string {
  return (
    `M${f.p(cx, cy - 2 * r)}Q${f.p(cx + 0.9 * r, cy - 0.9 * r)} ${f.p(cx + r, cy + 0.4 * r)}` +
    `A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(cx - r, cy + 0.4 * r)}Q${f.p(cx - 0.9 * r, cy - 0.9 * r)} ${f.p(cx, cy - 2 * r)}Z`
  );
}

/** A huge cresting wave rising from its base at the anchor and curling toward the side it faces, 50 wide and 34 tall at scale 1. */
function wavePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = 34 * between(rng, 0.94, 1.04);
  const lip = between(rng, 21, 25);
  const body =
    `M${f.p(-25, 0)}C${f.p(-22, -h * 0.5)} ${f.p(-12, -h)} ${f.p(5, -h)}C${f.p(15, -h)} ${f.p(lip + 1, -h * 0.82)} ${f.p(lip, -h * 0.58)}` +
    `C${f.p(17, -h * 0.72)} ${f.p(9, -h * 0.62)} ${f.p(8, -h * 0.4)}C${f.p(7, -h * 0.2)} ${f.p(14, -4)} ${f.p(25, 0)}Z`;
  let spray = "";
  for (let i = 0; i < 5; i += 1) spray += f.circle(lip + 3 + (i % 3) * 3 + between(rng, -0.6, 0.6), -h * 0.9 + i * 3.6 + between(rng, -0.8, 0.8), between(rng, 0.5, 1.1));
  return [
    shape(body, "sky-400", 0.85, MAIN),
    shape(f.band([[lip - 2, -h * 0.6], [16, -h * 0.66], [11.4, -h * 0.55], [9.8, -h * 0.4], [10.6, -h * 0.25], [14, -h * 0.13], [19, -4]], 3), "blue-600", 0.85),
    shape(f.band([[-6, -h + 0.8], [2, -h + 0.5], [9, -h + 0.5], [16, -h * 0.92], [lip - 0.5, -h * 0.72]], 2.4), "white", 0.95),
    shape(f.band([[-24, -0.6], [-12, -1.4], [0, -0.6], [12, -1.4], [24, -0.6]], 1.6), "white", 0.6),
    shape(spray, "white", 0.9),
  ];
}

/** Droplets within 8 of the anchor, white and pale blue. */
function sprayPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 6;
  let white = "";
  let blue = "";
  const start = between(rng, 0, 360);
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 2.6, 7);
    const d = f.circle(radius * Math.cos(a), radius * Math.sin(a), between(rng, 0.5, 1));
    if (k % 2 === 0) white += d;
    else blue += d;
  }
  return [shape(white, "white", 0.95, MAIN), shape(blue, "sky-300", 0.9)];
}

/** Confetti within 20 by 14 of the anchor: small tilted rectangles, one shape per colour. */
function confettiPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const colours: readonly PaletteName[] = ["red-500", "sky-400", "lime-400", "violet-500", "white"];
  const paths = ["", "", "", "", ""];
  for (let r = 0; r < 3; r += 1) {
    for (let c = 0; c < 5; c += 1) {
      const index = (c + r * 2) % 5;
      const cx = -8 + c * 4 + between(rng, -0.6, 0.6);
      const cy = -4.7 + r * 4.7 + between(rng, -0.6, 0.6);
      const tilt = between(rng, 0, 180);
      const corners = ([[-0.8, -0.4], [0.8, -0.4], [0.8, 0.4], [-0.8, 0.4]] as const).map((corner) => {
        const [x, y] = turned(corner, tilt);
        return [cx + x, cy + y] as const;
      });
      paths[index] = (paths[index] ?? "") + f.poly(corners);
    }
  }
  return colours.map((colour, i) => shape(paths[i] ?? "", colour, 1, i === 0 ? MAIN : undefined));
}

/** Five to eight bubbles in a column 6 wide and 10 tall about the anchor: rings and glints. */
function bubblesPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 8);
  const slot = 10 / n;
  let rings = "";
  let fills = "";
  let glints = "";
  for (let i = 0; i < n; i += 1) {
    const r = slot * between(rng, 0.32, 0.42);
    const cx = between(rng, -(3 - r), 3 - r);
    const cy = 5 - (i + 0.5) * slot;
    rings += f.circle(cx, cy, r) + f.circle(cx, cy, r * 0.72);
    fills += f.circle(cx, cy, r * 0.72);
    glints += f.circle(cx - r * 0.3, cy - r * 0.32, r * 0.17);
  }
  return [shape(fills, "cyan-400", 0.14), shape(rings, "cyan-400", 0.95, MAIN), shape(glints, "white", 0.9)];
}

/** Spore dots within 10 of the anchor, each with a faint halo. */
function sporesPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 6, 7);
  const start = between(rng, 0, 360);
  let dots = f.circle(0, 0, 0.9);
  let halos = f.circle(0, 0, 2.1);
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 5.4, 8);
    const r = between(rng, 0.5, 0.8);
    dots += f.circle(radius * Math.cos(a), radius * Math.sin(a), r);
    halos += f.circle(radius * Math.cos(a), radius * Math.sin(a), r * 2.3);
  }
  return [shape(halos, "lime-400", 0.2), shape(dots, "green-400", 0.95, MAIN)];
}

/** A rainbow arc of radius 30 about the anchor (its ends on the anchor's line): five translucent bands. */
function rainbowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const colours: readonly PaletteName[] = ["red-500", "orange-400", "lime-400", "sky-400", "violet-500"];
  const r = 30 * between(rng, 0.96, 1.04);
  return colours.map((colour, i) => {
    const ro = r - i * 2.4;
    const d = `M${f.p(-ro, 0)}A${fmt(f.r(ro))} ${fmt(f.r(ro))} 0 0 ${f.sweep(1)} ${f.p(ro, 0)}L${f.p(ro - 2.4, 0)}A${fmt(f.r(ro - 2.4))} ${fmt(f.r(ro - 2.4))} 0 0 ${f.sweep(0)} ${f.p(-(ro - 2.4), 0)}Z`;
    return shape(d, colour, 0.6, i === 0 ? MAIN : undefined);
  });
}

/** A shockwave about the anchor: translucent rings out to radius 14 and short radial streaks. */
function shockwavePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 14 * between(rng, 0.92, 1);
  let streaks = "";
  const start = between(rng, 0, 45);
  for (let k = 0; k < 8; k += 1) {
    const a = (start + k * 45) * DEGREES;
    const from = r * 0.35;
    const to = r * between(rng, 0.6, 0.78);
    streaks += f.band([[from * Math.cos(a), from * Math.sin(a)], [to * Math.cos(a), to * Math.sin(a)]], 0.8);
  }
  return [
    shape(ringKitA(f, r, 1.3), "white", 0.6, MAIN),
    shape(ringKitA(f, r * 0.68, 1.7), "sky-300", 0.5),
    shape(ringKitA(f, r * 0.36, 1.2), "white", 0.4),
    shape(streaks, "white", 0.55),
  ];
}

/** Echo rings: three arcs either side of the anchor at radii 6, 10 and 14, fading outward. */
function echoRingsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spread = between(rng, 36, 44);
  const pair = (r: number, width: number): string => arcKitA(f, r, width, -spread, spread) + arcKitA(f, r, width, 180 - spread, 180 + spread);
  return [
    shape(pair(6, 1.4), "sky-300", 0.85, MAIN),
    shape(pair(10, 1.3), "blue-600", 0.6),
    shape(pair(14, 1.2), "sky-300", 0.35),
  ];
}

/** A crackling zigzag bolt from the anchor 30 toward the side it faces, with a glow, a white core and a branch. */
function zapPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bolt: Point[] = [[0, 0]];
  for (let k = 1; k <= 6; k += 1) bolt.push([k * 5, (k % 2 === 0 ? -1 : 1) * between(rng, 1.8, 3.4)]);
  const [bx, by] = bolt[3] ?? [15, 0];
  const branch: Point[] = [[bx, by], [bx + between(rng, 2, 3.5), by - 3], [bx + between(rng, 5, 7), by - 7]];
  return [
    shape(f.band(bolt, 3.6), "sky-300", 0.3),
    shape(f.band(bolt, 1.1), "white", 0.95, MAIN),
    shape(f.band(branch, 0.6), "white", 0.8),
    shape(f.circle(0, 0, 2), "sky-300", 0.4),
  ];
}

/** A healing glow of radius 8 about the anchor with three or four plus-shaped sparkles. */
function healGlowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let pluses = "";
  const n = whole(rng, 3, 4);
  const start = between(rng, 0, 90);
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 3.2, 5.4);
    pluses += plusKitA(f, radius * Math.cos(a), radius * Math.sin(a), between(rng, 1.2, 1.7));
  }
  return [
    shape(f.circle(0, 0, 8), "green-400", 0.14, MAIN),
    shape(f.circle(0, 0, 5.6), "green-400", 0.2, MAIN),
    shape(f.circle(0, 0, 3.2), "green-400", 0.3, MAIN),
    shape(pluses, "green-200", 0.95),
  ];
}

/** A flame standing on the anchor, 6 tall at scale 1: outer, inner and a white core. */
function flamePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.8, 0.8);
  return [
    shape(flameKitA(f, 1, 0, lean), "orange-400", 1, MAIN),
    shape(flameKitA(f, 0.66, 0, lean * 0.7), "ember-500", 1),
    shape(flameKitA(f, 0.3, 0, lean * 0.4), "white", 0.95),
  ];
}

/** Four to six falling teardrops within 8 of the anchor. */
function dropsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 4, 6);
  let drops = "";
  let glints = "";
  for (let i = 0; i < n; i += 1) {
    const r = between(rng, 0.85, 1.15);
    const cx = (i % 2 === 0 ? -1 : 1) * between(rng, 1.5, 5);
    const cy = -6 + (i + 0.5) * (12 / n);
    drops += dropKitA(f, cx, cy, r);
    glints += f.circle(cx - r * 0.35, cy + r * 0.1, r * 0.2);
  }
  return [shape(drops, "blue-600", 1, MAIN), shape(glints, "white", 0.6)];
}

/** Four to six horizontal motion streaks trailing 24 long from the anchor, away from the side it faces. */
function streaksPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 4, 6);
  let streaks = "";
  let bright = "";
  for (let i = 0; i < n; i += 1) {
    const y = (i - (n - 1) / 2) * 2.2;
    const length = i === Math.floor(n / 2) ? 24 : between(rng, 12, 22);
    const d = f.poly([[-length, y], [0, y - 0.45], [0, y + 0.45]]);
    streaks += d;
    if (i === Math.floor(n / 2)) bright += f.poly([[-length * 0.6, y], [0, y - 0.25], [0, y + 0.25]]);
  }
  return [shape(streaks, "white", 0.35, MAIN), shape(bright, "white", 0.6)];
}

/** A dust cloud about 12 wide about the anchor: translucent blobs that never overlap. */
function dustPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let low = "";
  for (const x of [-3.8, 0, 3.8]) low += blob(f, rng, x, 0, 1.5, 6);
  let high = "";
  for (const x of [-1.9, 1.9]) high += blob(f, rng, x, -3.2, 1.5, 6);
  return [shape(low, "brass-500", 0.5, MAIN), shape(high, "stone-500", 0.45)];
}

/** Two or three rising S-curved steam wisps above the anchor, 10 tall at scale 1. */
function steamPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 2, 3);
  let wisps = "";
  let haze = "";
  for (let i = 0; i < n; i += 1) {
    const ox = (i - (n - 1) / 2) * 3.2;
    const phase = between(rng, -0.4, 0.4);
    const line: Point[] = [];
    for (let k = 0; k <= 8; k += 1) {
      const t = k / 8;
      line.push([ox + 1.4 * Math.sin(t * Math.PI * 2 + phase), -t * 10]);
    }
    wisps += f.band(line, 0.9);
    haze += f.band(line, 2);
  }
  return [shape(wisps, "white", 0.3, MAIN), shape(haze, "white", 0.1)];
}

/** Billowing smoke 16 wide about the anchor: pale lobes with dark undersides. */
function smokePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(blob(f, rng, -4.7, -2, 3.4, 8), "slate-400", 0.55, MAIN),
    shape(blob(f, rng, 4.7, -2.5, 3.6, 8), "slate-400", 0.55, MAIN),
    shape(blob(f, rng, -0.5, -7, 3.2, 8), "slate-400", 0.55, MAIN),
    shape(blob(f, rng, -4.2, 0.6, 2.9, 7) + blob(f, rng, 4.2, 0.4, 3.1, 7) + blob(f, rng, 0, -9.6, 2.4, 6), "slate-700", 0.3),
  ];
}

/** Five to seven glass shards within 10 of the anchor, pale blue and white. */
function shardsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  const start = between(rng, 0, 360);
  const shard = (cx: number, cy: number, reach: number): string => {
    const turn = between(rng, 0, 360);
    const tri = ([[reach, 0], [-reach * 0.55, reach * 0.4], [-reach * 0.3, -reach * 0.35]] as const).map((p) => {
      const [x, y] = turned(p, turn);
      return [cx + x, cy + y] as const;
    });
    return f.poly(tri);
  };
  let pale = shard(0, 0, 2.7);
  let white = "";
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 5.6, 8);
    const d = shard(radius * Math.cos(a), radius * Math.sin(a), between(rng, 1.3, 2));
    if (k % 2 === 0) pale += d;
    else white += d;
  }
  return [shape(pale, "sky-300", 0.9, MAIN), shape(white, "white", 0.9)];
}

/** A spiral of six ever-larger square outlines unwinding from the anchor toward the side it faces, the largest about 30 across. */
function squaresPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sides = [3, 7, 12, 17, 23, 28];
  const turn = between(rng, 12, 17);
  return sides.map((side, k) => {
    const cx = k * 2.6;
    const cy = 0;
    const angle = 45 + k * turn;
    const outer = polygonPath(f.x(cx), f.y(cy), 4, f.r((side / 2) * Math.SQRT2), angle);
    const inner = polygonPath(f.x(cx), f.y(cy), 4, f.r((side / 2 - 0.5) * Math.SQRT2), angle);
    return shape(outer + inner, "white", 0.5 + 0.08 * k, k === sides.length - 1 ? MAIN : undefined);
  });
}

/** A starburst of radius 10 about the anchor: radiating spikes and a bright core. */
function burstPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const star = (spikes: number, outer: number, inner: number, turn: number): string => {
    const points: Point[] = [];
    for (let i = 0; i < spikes * 2; i += 1) {
      const a = (turn + (i * 180) / spikes) * DEGREES;
      const reach = i % 2 === 0 ? outer * between(rng, 0.85, 1) : inner;
      points.push([reach * Math.cos(a), reach * Math.sin(a)]);
    }
    return f.poly(points);
  };
  return [
    shape(star(12, 10, 3.2, between(rng, 0, 30)), "white", 0.6, MAIN),
    shape(star(8, 6, 2.4, between(rng, 0, 45)), "white", 0.5),
    shape(f.circle(0, 0, 2.2), "white", 0.95),
  ];
}

/** Three or four curling wind lines about 22 wide about the anchor. */
function gustPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 3, 4);
  let lines = "";
  for (let i = 0; i < n; i += 1) {
    const y0 = (i - (n - 1) / 2) * 4.6;
    const x0 = between(rng, -11, -8);
    const cx = between(rng, 3, 7);
    const r = 1.8;
    const line: Point[] = [];
    for (let k = 0; k <= 5; k += 1) {
      const x = x0 + ((cx - x0) * k) / 5;
      line.push([x, y0 + 0.6 * Math.sin(k + i)]);
    }
    for (let k = 1; k <= 6; k += 1) {
      const t = k / 6;
      const a = (90 - 270 * t) * DEGREES;
      const rr = r * (1 - 0.45 * t);
      line.push([cx + rr * Math.cos(a), y0 - r + rr * Math.sin(a)]);
    }
    lines += f.band(line, 0.8);
  }
  return [shape(lines, "white", 0.35, MAIN)];
}

/** Five to seven leaves blown within 14 of the anchor: green, orange and brass. */
function leavesPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  const start = between(rng, 0, 360);
  const paths = ["", "", ""];
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 6, 11);
    const cx = radius * Math.cos(a);
    const cy = radius * Math.sin(a);
    const turn = between(rng, 0, 180);
    const [ax, ay] = turned([-2, 0], turn);
    const [bx, by] = turned([2, 0], turn);
    const [ux, uy] = turned([0, -1.5], turn);
    const [lx, ly] = turned([0, 1.5], turn);
    const d = `M${f.p(cx + ax, cy + ay)}Q${f.p(cx + ux * 2, cy + uy * 2)} ${f.p(cx + bx, cy + by)}Q${f.p(cx + lx * 2, cy + ly * 2)} ${f.p(cx + ax, cy + ay)}Z`;
    paths[k % 3] = (paths[k % 3] ?? "") + d;
  }
  return [shape(paths[0] ?? "", "green-500", 1, MAIN), shape(paths[1] ?? "", "orange-400", 1), shape(paths[2] ?? "", "brass-600", 1)];
}

/** A lumpy round puff cloud of radius 10 about the anchor with a lighter top lobe. */
function puffPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(blob(f, rng, 0, 0, 9, 9), "red-300", 1, MAIN),
    shape(blob(f, rng, 0, 3.5, 6, 7), "red-400", 0.3),
    shape(blob(f, rng, -2.5, -3.8, 4.6, 7), "white", 0.35),
  ];
}

/** A bright swirling vortex of radius 14 about the anchor: four tapering arms and a bright core. */
function vortexPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = between(rng, 0, 90);
  const sweep = between(rng, 200, 240);
  const arm = (k: number): string => spiralArm(f, start + k * 90, sweep, 3, 14, 3);
  return [
    shape(arm(0) + arm(2), "white", 0.9, MAIN),
    shape(arm(1) + arm(3), "sky-300", 0.85),
    shape(f.circle(0, 0, 4), "sky-300", 0.3),
    shape(f.circle(0, 0, 2), "white", 0.95),
  ];
}

/** The sky folding along a glowing vertical seam through the anchor, spanning the box's height, with dark flaps at its edges. */
function foldSeamPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const x = f.x(0);
  const wobble = between(rng, -1.5, 1.5);
  const line: Point[] = [[x, EDGE], [x + wobble, 30], [x - wobble, 65], [x, FAR_EDGE]];
  const flap = 17 * at.s;
  return [
    shape(bandPath(line, 7 * at.s), "purple-400", 0.2),
    shape(bandPath(line, 3.2 * at.s), "purple-400", 0.28),
    shape(bandPath(line, 0.9 * at.s), "violet-300", 1, MAIN),
    shape(`M${pt(EDGE, EDGE)}L${pt(EDGE + flap, SCENE_BOX / 2)}L${pt(EDGE, FAR_EDGE)}Z` + `M${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE - flap, SCENE_BOX / 2)}L${pt(FAR_EDGE, FAR_EDGE)}Z`, "slate-900", 0.4),
  ];
}

/** A glowing bell curve over the anchor, feet 20 either side and its top 22 above. */
function bellCurvePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = 22 * between(rng, 0.95, 1.05);
  const curve: Point[] = [-20, -14, -10, -7, -4.5, -2.2, 0, 2.2, 4.5, 7, 10, 14, 20].map((x) => [x, -top * Math.exp(-(x * x) / (2 * 7 * 7))] as const);
  return [
    shape(f.band(curve, 4.5), "cyan-400", 0.28),
    shape(f.band(curve, 1.2), "white", 0.95, MAIN),
    shape(f.band([[-22, 0], [22, 0]], 0.5), "cyan-400", 0.45),
  ];
}

/** Five to seven thin taut puppet strings from the top of the box down to points spread across 50 about the anchor. */
function stringsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  let lines = "";
  let knots = "";
  for (let i = 0; i < n; i += 1) {
    const dx = -25 + (50 * (i + 0.5)) / n + between(rng, -1.5, 1.5);
    const dy = between(rng, -3, 3);
    lines += bandPath([[f.x(dx * 0.35), EDGE], [f.x(dx), f.y(dy)]], 0.35 * at.s);
    knots += f.circle(dx, dy, 0.7);
  }
  return [shape(lines, "slate-100", 0.7, MAIN), shape(knots, "slate-100", 0.9)];
}

/** A ring of ten to twelve small tilted cards whirling on an ellipse 30 by 10 about the anchor, the near half bigger. */
function cardRingPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 10, 12);
  const start = between(rng, 0, 360);
  let bodies = "";
  let borders = "";
  let glints = "";
  for (let k = 0; k < n; k += 1) {
    const theta = (start + (k * 360) / n) * DEGREES;
    const cx = 15 * Math.cos(theta);
    const cy = 5 * Math.sin(theta);
    const size = 0.65 + 0.35 * ((Math.sin(theta) + 1) / 2);
    const tangent = (Math.atan2(5 * Math.cos(theta), -15 * Math.sin(theta)) * 180) / Math.PI;
    const quad = (hw: number, hh: number): Point[] =>
      ([[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]] as const).map((corner) => {
        const [x, y] = turned(corner, tangent);
        return [cx + x, cy + y] as const;
      });
    const hw = 1.1 * size;
    const hh = 1.7 * size;
    bodies += f.poly(quad(hw, hh));
    borders += f.poly(quad(hw, hh)) + f.poly(quad(hw - 0.35 * size, hh - 0.35 * size));
    const g = quad(hw * 0.6, hh * 0.6).slice(0, 3).map(([x, y]) => [x * 0.5 + cx * 0.5, y * 0.5 + cy * 0.5] as const);
    glints += f.poly(g);
  }
  return [shape(bodies, "blue-800", 1, MAIN), shape(borders, "white", 0.95), shape(glints, "white", 0.5)];
}

/** One Mythic's own: a spinning cyclone funnel standing on the anchor, 40 tall, 32 wide at its top narrowing to 4, with sand streams at its foot. */
function cyclonePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.12, 0.12);
  const hw = (y: number): number => 2 + 14 * (-y / 40) ** 1.4;
  const P = (x: number, y: number): Point => leant([x, y], lean);
  const levels = [-40, -33, -26, -19, -12, -6, 0];
  const left = levels.map((y) => P(-hw(y), y));
  const right = levels.map((y) => P(hw(y), y)).reverse();
  const swirl = (yc: number, thick: number): string => {
    const yl = yc;
    const yr = yc - 2.4;
    const a = P(-hw(yl), yl);
    const b = P(hw(yr), yr);
    const c = P(hw(yr + thick), yr + thick);
    const d = P(-hw(yl + thick), yl + thick);
    const m = P(0, yl + 3.6);
    const m2 = P(0, yl + thick + 3.6);
    return `M${f.p(...a)}Q${f.p(...m)} ${f.p(...b)}L${f.p(...c)}Q${f.p(...m2)} ${f.p(...d)}Z`;
  };
  const rim = P(0, -40);
  return [
    shape(f.poly([...left, ...right]), "sky-300", 0.55, MAIN),
    shape(f.ellipse(rim[0], -40, 16, 3), "slate-100", 0.45),
    shape(swirl(-34, 2.6) + swirl(-20, 2.2), "slate-100", 0.7),
    shape(swirl(-28, 2.4) + swirl(-14, 1.8) + swirl(-7, 1.2), "white", 0.5),
    shape(f.band([[-0.5, -38], [-0.6, -20], [-0.2, -4]], 0.7), "slate-100", 0.3),
    shape(f.ellipse(0, 0.5, 9, 1.6), "brass-500", 0.4),
    shape(f.band([[-2, -3], [-7, -1.2], [-13, 0]], 1) + f.band([[2, -3], [7, -1.2], [13, 0]], 1) + f.band([[-1.5, -6], [-8, -4], [-11, -2.6]], 0.7) + f.band([[1.5, -6], [8, -4], [12, -2.8]], 0.7), "brass-500", 0.8),
  ];
}

/** The moon's shadow of radius 6 about the anchor: a dark disc and a bright corona a little larger, meant to lie over the moon. */
function eclipsePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wobble = between(rng, -0.2, 0.2);
  return [
    shape(f.circle(0, 0, 9.4 + wobble) + f.circle(0, 0, 7.4), "white", 0.14),
    shape(f.circle(0, 0, 7.4) + f.circle(0, 0, 5.8), "white", 0.4),
    shape(f.circle(0, 0, 6), "slate-900", 0.85, MAIN),
  ];
}

/** A bright sun of radius 5 about the anchor, in glow rings. */
function sunPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 5 * between(rng, 0.97, 1.03);
  return [
    shape(f.circle(0, 0, r * 1.9), "slate-100", 0.14),
    shape(f.circle(0, 0, r * 1.45), "white", 0.2),
    shape(f.circle(0, 0, r * 1.18), "white", 0.3),
    shape(f.circle(0, 0, r), "white", 1, MAIN),
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
  water: { category: "terrain", draw: waterPart },
  sand: { category: "terrain", draw: sandPart },
  snow: { category: "terrain", draw: snowPart },
  boards: { category: "terrain", draw: boardsPart },
  stage: { category: "terrain", draw: stagePart },
  road: { category: "terrain", draw: roadPart },
  "checker-floor": { category: "terrain", draw: checkerFloorPart },
  cobbles: { category: "terrain", draw: cobblesPart },
  lava: { category: "terrain", draw: lavaPart },
  "tile-wall": { category: "terrain", draw: tileWallPart },
  cavern: { category: "terrain", draw: cavernPart },
  aisle: { category: "terrain", draw: aislePart },
  amphitheatre: { category: "terrain", draw: amphitheatrePart },
  chessboard: { category: "terrain", draw: chessboardPart },
  mesa: { category: "terrain", draw: mesaPart },
  "candy-mountain": { category: "terrain", draw: candyMountainPart },
  "floating-island": { category: "terrain", draw: floatingIslandPart },
  fence: { category: "architecture", draw: fence },
  "fence-post": { category: "architecture", draw: fencePost },
  gate: { category: "architecture", draw: gate },
  "ruin-wall": { category: "architecture", draw: ruinWall },
  tower: { category: "architecture", draw: tower },
  column: { category: "architecture", draw: column },
  arch: { category: "architecture", draw: arch },
  tombstone: { category: "architecture", draw: tombstone },
  window: { category: "architecture", draw: windowPart },
  door: { category: "architecture", draw: doorPart },
  shelf: { category: "architecture", draw: shelfPart },
  counter: { category: "architecture", draw: counterPart },
  house: { category: "architecture", draw: housePart },
  dome: { category: "architecture", draw: domePart },
  "stone-wall": { category: "architecture", draw: stoneWallPart },
  battlement: { category: "architecture", draw: battlementPart },
  mirror: { category: "architecture", draw: mirrorPart },
  pedestal: { category: "architecture", draw: pedestalPart },
  bridge: { category: "architecture", draw: bridgePart },
  lighthouse: { category: "architecture", draw: lighthousePart },
  well: { category: "architecture", draw: wellPart },
  "glass-wall": { category: "architecture", draw: glassWallPart },
  skyline: { category: "architecture", draw: skylinePart },
  bookcase: { category: "architecture", draw: bookcasePart },
  greenhouse: { category: "architecture", draw: greenhousePart },
  "standing-stone": { category: "architecture", draw: standingStonePart },
  fortress: { category: "architecture", draw: fortressPart },
  barn: { category: "architecture", draw: barnPart },
  pavilion: { category: "architecture", draw: pavilionPart },
  treehouse: { category: "architecture", draw: treehousePart },
  ramp: { category: "architecture", draw: rampPart },
  forge: { category: "architecture", draw: forgePart },
  belfry: { category: "architecture", draw: belfryPart },
  rotunda: { category: "architecture", draw: rotundaPart },
  townhouse: { category: "architecture", draw: townhousePart },
  stadium: { category: "architecture", draw: stadiumPart },
  bastion: { category: "architecture", draw: bastionPart },
  statue: { category: "architecture", draw: statuePart },
  "paper-tower": { category: "architecture", draw: paperTowerPart },
  sprout: { category: "plants", draw: sprout },
  "grass-tuft": { category: "plants", draw: grassTuft },
  "dead-tree": { category: "plants", draw: deadTree },
  tree: { category: "plants", draw: tree },
  vine: { category: "plants", draw: vine },
  bush: { category: "plants", draw: bush },
  mushroom: { category: "plants", draw: mushroom },
  wheat: { category: "plants", draw: wheat },
  fern: { category: "plants", draw: fernPart },
  pine: { category: "plants", draw: pinePart },
  reed: { category: "plants", draw: reedPart },
  flower: { category: "plants", draw: flowerPart },
  "tall-grass": { category: "plants", draw: tallGrassPart },
  hedge: { category: "plants", draw: hedgePart },
  stump: { category: "plants", draw: stumpPart },
  crow: { category: "creatures", draw: crow },
  bat: { category: "creatures", draw: bat },
  "pale-hand": { category: "creatures", draw: paleHand },
  "bone-hand": { category: "creatures", draw: boneHand },
  wisp: { category: "creatures", draw: wisp },
  worm: { category: "creatures", draw: worm },
  moth: { category: "creatures", draw: moth },
  gull: { category: "creatures", draw: gullPart },
  cat: { category: "creatures", draw: catPart },
  sheep: { category: "creatures", draw: sheepPart },
  bee: { category: "creatures", draw: beePart },
  scarab: { category: "creatures", draw: scarabPart },
  cube: { category: "creatures", draw: cubePart },
  panther: { category: "creatures", draw: pantherPart },
  dragon: { category: "creatures", draw: dragonPart },
  hatchling: { category: "creatures", draw: hatchlingPart },
  "spiky-pillow": { category: "creatures", draw: spikyPillowPart },
  boulder: { category: "creatures", draw: boulderPart },
  loaf: { category: "creatures", draw: loafPart },
  "rush-token": { category: "creatures", draw: rushTokenPart },
  beast: { category: "creatures", draw: beastPart },
  goblin: { category: "creatures", draw: goblinPart },
  bunny: { category: "creatures", draw: bunnyPart },
  bear: { category: "creatures", draw: bearPart },
  crowd: { category: "creatures", draw: crowdPart },
  soldier: { category: "creatures", draw: soldierPart },
  "soldier-fallen": { category: "creatures", draw: soldierFallenPart },
  ghost: { category: "creatures", draw: ghostPart },
  "card-imp": { category: "creatures", draw: cardImpPart },
  "open-hand": { category: "creatures", draw: openHandPart },
  lantern: { category: "objects", draw: lantern },
  "bone-scarecrow": { category: "objects", draw: boneScarecrow },
  pitchfork: { category: "objects", draw: pitchfork },
  shovel: { category: "objects", draw: shovel },
  crate: { category: "objects", draw: crate },
  banner: { category: "objects", draw: banner },
  orb: { category: "objects", draw: orb },
  crystal: { category: "objects", draw: crystal },
  card: { category: "objects", draw: cardPart },
  "card-fan": { category: "objects", draw: cardFanPart },
  deck: { category: "objects", draw: deckPart },
  coin: { category: "objects", draw: coinPart },
  "coin-stack": { category: "objects", draw: coinStackPart },
  table: { category: "objects", draw: tablePart },
  chair: { category: "objects", draw: chairPart },
  tome: { category: "objects", draw: tomePart },
  jar: { category: "objects", draw: jarPart },
  cup: { category: "objects", draw: cupPart },
  dagger: { category: "objects", draw: daggerPart },
  blade: { category: "objects", draw: bladePart },
  arrow: { category: "objects", draw: arrowPart },
  target: { category: "objects", draw: targetPart },
  chest: { category: "objects", draw: chestPart },
  dice: { category: "objects", draw: dicePart },
  chain: { category: "objects", draw: chainPart },
  rune: { category: "objects", draw: runePart },
  portal: { category: "objects", draw: portalPart },
  spotlight: { category: "objects", draw: spotlightPart },
  pipe: { category: "objects", draw: pipePart },
  gear: { category: "objects", draw: gearPart },
  shield: { category: "objects", draw: shieldPart },
  dummy: { category: "objects", draw: dummyPart },
  toaster: { category: "objects", draw: toasterPart },
  "jelly-bean": { category: "objects", draw: jellyBeanPart },
  parcel: { category: "objects", draw: parcelPart },
  bicycle: { category: "objects", draw: bicyclePart },
  deckchair: { category: "objects", draw: deckchairPart },
  hat: { category: "objects", draw: hatPart },
  conveyor: { category: "objects", draw: conveyorPart },
  saw: { category: "objects", draw: sawPart },
  rifle: { category: "objects", draw: riflePart },
  helmet: { category: "objects", draw: helmetPart },
  blackboard: { category: "objects", draw: blackboardPart },
  pod: { category: "objects", draw: podPart },
  "wrecking-ball": { category: "objects", draw: wreckingBallPart },
  padlock: { category: "objects", draw: padlockPart },
  candle: { category: "objects", draw: candlePart },
  chute: { category: "objects", draw: chutePart },
  controller: { category: "objects", draw: controllerPart },
  "chess-piece": { category: "objects", draw: chessPiecePart },
  hourglass: { category: "objects", draw: hourglassPart },
  globe: { category: "objects", draw: globePart },
  notebook: { category: "objects", draw: notebookPart },
  bench: { category: "objects", draw: benchPart },
  nest: { category: "objects", draw: nestPart },
  ship: { category: "objects", draw: shipPart },
  gift: { category: "objects", draw: giftPart },
  honeypot: { category: "objects", draw: honeypotPart },
  copier: { category: "objects", draw: copierPart },
  mask: { category: "objects", draw: maskPart },
  pan: { category: "objects", draw: panPart },
  snare: { category: "objects", draw: snarePart },
  ladder: { category: "objects", draw: ladderPart },
  hammer: { category: "objects", draw: hammerPart },
  hologram: { category: "objects", draw: hologramPart },
  telescope: { category: "objects", draw: telescopePart },
  "rocket-cart": { category: "objects", draw: rocketCartPart },
  bell: { category: "objects", draw: bellPart },
  "rice-cake": { category: "objects", draw: riceCakePart },
  envelope: { category: "objects", draw: envelopePart },
  crucible: { category: "objects", draw: cruciblePart },
  crown: { category: "objects", draw: crownPart },
  vial: { category: "objects", draw: vialPart },
  ball: { category: "objects", draw: ballPart },
  hatch: { category: "objects", draw: hatchPart },
  clamp: { category: "objects", draw: clampPart },
  pocket: { category: "objects", draw: pocketPart },
  sack: { category: "objects", draw: sackPart },
  "chaos-wheel": { category: "objects", draw: chaosWheelPart },
  fist: { category: "objects", draw: fistPart },
  fountain: { category: "objects", draw: fountainPart },
  chandelier: { category: "objects", draw: chandelierPart },
  "puppet-hand": { category: "objects", draw: puppetHandPart },
  anvil: { category: "objects", draw: anvilPart },
  rails: { category: "objects", draw: railsPart },
  syringe: { category: "objects", draw: syringePart },
  podium: { category: "objects", draw: podiumPart },
  streetlamp: { category: "objects", draw: streetlampPart },
  bin: { category: "objects", draw: binPart },
  bed: { category: "objects", draw: bedPart },
  bunting: { category: "objects", draw: buntingPart },
  bulbs: { category: "objects", draw: bulbsPart },
  "gem-ball": { category: "objects", draw: gemBallPart },
  map: { category: "objects", draw: mapPart },
  doll: { category: "objects", draw: dollPart },
  bow: { category: "objects", draw: bowPart },
  eye: { category: "objects", draw: eyePart },
  "crate-big": { category: "objects", draw: crateBigPart },
  fig: { category: "objects", draw: figPart },
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
  wave: { category: "weather", draw: wavePart },
  spray: { category: "weather", draw: sprayPart },
  confetti: { category: "weather", draw: confettiPart },
  bubbles: { category: "weather", draw: bubblesPart },
  spores: { category: "weather", draw: sporesPart },
  rainbow: { category: "weather", draw: rainbowPart },
  shockwave: { category: "weather", draw: shockwavePart },
  "echo-rings": { category: "weather", draw: echoRingsPart },
  zap: { category: "weather", draw: zapPart },
  "heal-glow": { category: "weather", draw: healGlowPart },
  flame: { category: "weather", draw: flamePart },
  drops: { category: "weather", draw: dropsPart },
  streaks: { category: "weather", draw: streaksPart },
  dust: { category: "weather", draw: dustPart },
  steam: { category: "weather", draw: steamPart },
  smoke: { category: "weather", draw: smokePart },
  shards: { category: "weather", draw: shardsPart },
  squares: { category: "weather", draw: squaresPart },
  burst: { category: "weather", draw: burstPart },
  gust: { category: "weather", draw: gustPart },
  leaves: { category: "weather", draw: leavesPart },
  puff: { category: "weather", draw: puffPart },
  vortex: { category: "weather", draw: vortexPart },
  "fold-seam": { category: "weather", draw: foldSeamPart },
  "bell-curve": { category: "weather", draw: bellCurvePart },
  strings: { category: "weather", draw: stringsPart },
  "card-ring": { category: "weather", draw: cardRingPart },
  cyclone: { category: "weather", draw: cyclonePart },
  eclipse: { category: "weather", draw: eclipsePart },
  sun: { category: "weather", draw: sunPart },
} as const satisfies Readonly<Record<string, KitPart>>;

export type SceneryPartId = keyof typeof SCENE_KIT;
