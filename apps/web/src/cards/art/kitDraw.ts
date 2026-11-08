// R3331000, R3390000: the scene kit's drawing helpers (ART.md, "Scene recipes and kits"), shared by every module
// of scenery parts (sceneKit.ts, and the parts #3390 added in sceneKitPlaces.ts, sceneKitObjects.ts and
// sceneKitNature.ts) and by the figure kit: a part's frame (`local`), the shapes it draws them with (bands for
// lines, circles, blobs, crescents, spiral arms, a ground band) and its flags. Pure: every draw is the stream's.

import { fmt } from "./hash.ts";
import type { KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";

/** The scene's box (scene.ts re-exports it, and `scene.test.ts` holds it to the painter's `ART_BOX`). */
export const SCENE_BOX = 100;
/** Where a terrain band starts and ends: a little past the box, so no seam shows at its edges. */
export const EDGE = -2;
export const FAR_EDGE = SCENE_BOX + 2;
export const FULL_TURN = 360;
export const DEGREES = Math.PI / 180;

export type Point = readonly [number, number];

export function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

export function whole(rng: Rng, low: number, high: number): number {
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

export type Flags = { readonly inked?: boolean; readonly main?: boolean };
export const MAIN: Flags = { main: true };
export const INKED: Flags = { inked: true };
export const MAIN_INKED: Flags = { main: true, inked: true };

export function shape(d: string, paint: PaletteName, opacity: number, flags?: Flags): KitShape {
  return flags === undefined ? { d, paint, opacity } : { d, paint, opacity, ...flags };
}

/** `point` turned `degrees` about the part's anchor (clockwise on screen). */
export function turned([dx, dy]: Point, degrees: number): Point {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  return [dx * c - dy * s, dx * s + dy * c];
}

/** `point` leant by `lean` (a shear): its top moves sideways by `lean` per unit of height. */
export function leant([dx, dy]: Point, lean: number): Point {
  return [dx - lean * dy, dy];
}

/** A four-pointed sparkle about a point, `size` its reach. */
export function sparkle(f: Local, cx: number, cy: number, size: number): string {
  const points: Point[] = [];
  for (let i = 0; i < 8; i += 1) {
    const angle = (i * FULL_TURN) / 8 - 90;
    const reach = i % 2 === 0 ? size : size * 0.28;
    points.push([cx + reach * Math.cos(angle * DEGREES), cy + reach * Math.sin(angle * DEGREES)]);
  }
  return f.poly(points);
}

/** A wobbly round blob, smoothed through the midpoints of its jittered rim. */
export function blob(f: Local, rng: Rng, cx: number, cy: number, r: number, points: number): string {
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
export function groundBand(rng: Rng, top: number, amplitude: number, bumps: number): string {
  const ys: number[] = [];
  for (let i = 0; i <= bumps; i += 1) ys.push(top + between(rng, -amplitude, amplitude));
  const xAt = (i: number): number => EDGE + ((FAR_EDGE - EDGE) * i) / bumps;
  const yAt = (i: number): number => ys[i] ?? top;
  let d = `M${pt(EDGE, FAR_EDGE)}L${pt(EDGE, yAt(0))}`;
  for (let i = 1; i < bumps; i += 1) d += `Q${pt(xAt(i), yAt(i))} ${pt((xAt(i) + xAt(i + 1)) / 2, (yAt(i) + yAt(i + 1)) / 2)}`;
  return `${d}L${pt(FAR_EDGE, yAt(bumps))}L${pt(FAR_EDGE, FAR_EDGE)}Z`;
}

/** A crescent along the side the part faces: an arc of `r`, hollowed by a narrower one. */
export function crescent(f: Local, r: number, inner: number): string {
  return (
    `M${f.p(0, -r)}A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(0, r)}` +
    `A${fmt(f.r(r * inner))} ${fmt(f.r(r))} 0 0 ${f.sweep(0)} ${f.p(0, -r)}Z`
  );
}

/** A tapering spiral arm about the anchor, from `r0` out to `r1`, `sweep` degrees round. */
export function spiralArm(f: Local, start: number, sweep: number, r0: number, r1: number, width: number): string {
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
