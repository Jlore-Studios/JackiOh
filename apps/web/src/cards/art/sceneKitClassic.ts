// R3352000: the scene kit's Classic parts (ART.md, "Scene recipes and kits"), which Classic's scenes (#3352) added
// for the things their briefs name and the kit lacked. sceneKit.ts spreads them into `SCENE_KIT`. Each part keeps
// the kit's rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream alone,
// every colour a palette.css name none in Radiant's gold window, a glow or a wash a stack of translucent shapes,
// never a gradient, and every shape filled even-odd, so a hole is a second sub-path and no two sub-paths of one
// shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, DEGREES, INKED, local, MAIN_INKED, shape, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";
import { CLASSIC_PROPS_KIT } from "./sceneKitClassicProps.ts";

// Helpers

/** A closed curve through the midpoints of the ring's sides, each corner their control: a smooth outline inside the points. */
function smooth(f: Local, ring: readonly Point[]): string {
  const n = ring.length;
  const at = (i: number): Point => ring[i % n] ?? [0, 0];
  const mid = (i: number): string => f.p((at(i)[0] + at(i + 1)[0]) / 2, (at(i)[1] + at(i + 1)[1]) / 2);
  let d = `M${mid(0)}`;
  for (let i = 1; i <= n; i += 1) d += `Q${f.p(...at(i))} ${mid(i)}`;
  return `${d}Z`;
}

/** A band along the points whose width at each is its entry of `widths`: a limb, a tail or a body tapering. */
function taper(f: Local, points: readonly Point[], widths: readonly number[]): string {
  const left: string[] = [];
  const right: string[] = [];
  points.forEach(([x, y], i) => {
    const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
    const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [x, y];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (widths[i] ?? 1) / 2;
    left.push(f.p(x - ((ny - py) / length) * half, y + ((nx - px) / length) * half));
    right.push(f.p(x + ((ny - py) / length) * half, y - ((nx - px) / length) * half));
  });
  return `M${left.join("L")}L${right.reverse().join("L")}Z`;
}

/** The two sides of `taper`'s band at its `i`th point, `reach` of the way out from its middle: a stripe across it. */
function edges(points: readonly Point[], widths: readonly number[], i: number, reach: number): Point[] {
  const [x, y] = points[i] ?? [0, 0];
  const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
  const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [x, y];
  const length = Math.hypot(nx - px, ny - py) || 1;
  const half = ((widths[i] ?? 1) / 2) * reach;
  const ox = (-(ny - py) / length) * half;
  const oy = ((nx - px) / length) * half;
  return [[x + ox, y + oy], [x - ox, y - oy]];
}

/** A sawtooth from `a` to `b` of `teeth` points, each `depth` off the line on its left as drawn (up, going right), closed back along it. */
function sawtooth(f: Local, a: Point, b: Point, teeth: number, depth: number, lean: number): string {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const length = Math.hypot(dx, dy) || 1;
  const points: Point[] = [a];
  for (let i = 0; i < teeth; i += 1) {
    const t = (i + 0.5 + lean) / teeth;
    points.push([a[0] + dx * t + (dy / length) * depth, a[1] + dy * t - (dx / length) * depth]);
    points.push([a[0] + (dx * (i + 1)) / teeth, a[1] + (dy * (i + 1)) / teeth]);
  }
  return f.poly(points);
}

/** An ellipse about (cx, cy) turned `degrees` (clockwise as drawn facing right), its arcs mirrored with the part. */
function tilted(f: Local, at: PartPlace, cx: number, cy: number, rx: number, ry: number, degrees: number): string {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  const a = f.p(cx - rx * c, cy - rx * s);
  const b = f.p(cx + rx * c, cy + rx * s);
  const arc = `${fmt(f.r(rx))} ${fmt(f.r(ry))} ${fmt(at.flip * degrees)} 1 0 `;
  return `M${a}A${arc}${b}A${arc}${a}Z`;
}

// Creatures

/** R3352000: a hulking armoured tortoise standing on the anchor, facing the way it faces: a high domed shell (main) with plate seams, a riveted iron turret bolted on top, its stubby cannon aimed forward and a little up, stumpy scaly legs and a wrinkled head with a steady eye; about 30 wide and 22 tall at scale 1. */
function tortoise(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const aim = between(rng, -1, 1);
  const bob = between(rng, -0.5, 0.5);
  const hy = -9.2 + bob;
  const stump = (x: number, w: number): string => f.poly([[x - w, -5], [x + w, -5], [x + w + 0.5, 0], [x - w - 0.5, 0]]);
  const nails = (x: number, w: number): string => f.poly([[x + w - 0.2, -0.9], [x + w + 1.1, 0], [x + w - 0.6, 0]]);
  return [
    shape(stump(-3.8, 1.5) + stump(3.6, 1.5), "slate-700", 1),
    shape(smooth(f, [[6, -10], [11, -10.4 + bob], [14.2, -11.4 + bob], [16.6, -9.4 + bob], [15.8, -6.6 + bob], [12, -6.6], [8, -5.6], [6, -6.6]]) + stump(-9.6, 1.9) + stump(7.2, 1.9), "stone-500", 1, INKED),
    shape(`M${f.p(-13.5, -4)}C${f.p(-13.5, -12)} ${f.p(-7, -15.6)} ${f.p(-1.5, -15.6)}C${f.p(4.5, -15.6)} ${f.p(9.6, -11.5)} ${f.p(10.2, -4)}Z`, "green-700", 1, MAIN_INKED),
    shape(`M${f.p(-13.2, -5.6)}Q${f.p(-1.5, -9.8)} ${f.p(9.9, -5.6)}L${f.p(10.2, -4)}L${f.p(-13.5, -4)}Z`, "black", 0.25),
    shape(
      f.band([[-12.4, -8.4], [-6, -9.6], [1, -9.8], [8.8, -8.4]], 0.5) + f.band([[-6.4, -4.6], [-6, -9.1]], 0.5) + f.band([[2.2, -4.6], [1.6, -9.3]], 0.5) +
        f.band([[-3, -10.3], [-3.8, -14.8]], 0.5) + f.band([[4, -10.3], [5, -13.6]], 0.5) + f.band([[11.2, -9.9 + bob], [11.6, -7]], 0.35),
      "black",
      0.35,
    ),
    shape(`M${f.p(-11, -9.4)}Q${f.p(-9, -14)} ${f.p(-3.4, -14.8)}Q${f.p(-8, -12.8)} ${f.p(-11, -9.4)}Z`, "white", 0.22),
    shape(f.band([[-14, -4.6], [-8, -3.4], [-1.5, -3], [5, -3.4], [10.8, -4.6]], 2.2), "wood-700", 1, INKED),
    shape(f.band([[1.4, -17.2], [12.4, -20.2 + aim]], 2), "slate-800", 1, INKED),
    shape(`M${f.p(-3.6, -14.8)}L${f.p(-3, -18)}Q${f.p(0.6, -20.6)} ${f.p(4.2, -18)}L${f.p(4.8, -14.8)}Z` + f.band([[11.4, -19.9 + aim * 0.9], [12.8, -20.3 + aim]], 2.8), "slate-700", 1, INKED),
    shape(f.circle(-2.4, -15.6, 0.4) + f.circle(0.6, -15.8, 0.4) + f.circle(3.6, -15.6, 0.4) + f.band([[-2.2, -17.6], [0.4, -19.2]], 0.5), "slate-400", 1),
    shape(f.circle(14, hy, 0.55) + f.band([[13.2, hy - 0.8], [14.8, hy - 0.9]], 0.3) + f.band([[14.2, hy + 1.8], [16, hy + 1.6]], 0.3) + nails(-9.6, 1.9) + nails(7.2, 1.9), "slate-900", 1),
  ];
}

/** R3352000: a bubbling plague slime sitting on the anchor: a gooey dome (main) with a drippy skirt, a shine, two eyes and a grin, weeping pustules and a bubble on top; about 19 wide and 14 tall at scale 1 (its bubble included). */
function slime(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const lean = j(0.8);
  const top = -12.6 + j(0.5);
  let skirt = "";
  for (const x of [6, 3, 0, -3, -6, -9.6]) skirt += `Q${f.p(x + 1.6, 1.5)} ${f.p(x, 0)}`;
  type Spot = readonly [number, number, number];
  const low: Spot = [-5.6, -4 + j(0.4), 1.3];
  const side: Spot = [6.2, -6.2 + j(0.4), 1.1];
  const spots: Spot[] = [low, [-3.4, -9.4, 1], side, [3, -2.2, 0.8]];
  const drip = ([x, y, r]: Spot): string => `M${f.p(x - 0.4, y + r + 0.15)}Q${f.p(x, y + r + 2.6)} ${f.p(x + 0.4, y + r + 0.15)}Z`;
  const ex = 0.8 + lean * 0.5;
  const bx = -1.6 + lean;
  return [
    shape(f.ellipse(0, 0.4, 11, 1.6), "black", 0.3),
    shape(`M${f.p(-9.6, 0)}C${f.p(-9.8, -6)} ${f.p(-5.6 + lean, top)} ${f.p(lean, top)}C${f.p(5.6 + lean, top)} ${f.p(9.8, -6)} ${f.p(9, 0)}${skirt}Z`, "green-700", 1, MAIN_INKED),
    shape(`M${f.p(3 + lean, top + 0.4)}C${f.p(8.6, -10)} ${f.p(9.6, -5)} ${f.p(8.8, -0.6)}Q${f.p(0, 0.6)} ${f.p(-8.8, -0.6)}Q${f.p(0, -1.8)} ${f.p(6.4, -2.6)}Q${f.p(7.6, -7)} ${f.p(3 + lean, top + 0.4)}Z`, "black", 0.25),
    shape(f.circle(bx, top + 0.2, 1.6), "green-400", 1, INKED),
    shape(`M${f.p(-6.6, -5.6)}Q${f.p(-6, -10.2)} ${f.p(-3.6 + lean, top + 1.4)}Q${f.p(-5, -9.4)} ${f.p(-6.6, -5.6)}Z` + f.circle(-7, -3.4, 0.5) + f.circle(bx - 0.5, top - 0.3, 0.5), "white", 0.6),
    shape(spots.map(([x, y, r]) => f.circle(x, y, r)).join("") + drip(low) + drip(side) + f.circle(2.4 + lean, top - 1.2, 0.4), "lime-200", 1),
    shape(spots.map(([x, y, r]) => f.circle(x, y, r * 0.34)).join(""), "green-900", 1),
    shape(f.circle(ex - 2, -7.4, 1.3) + f.circle(ex + 2, -7.6, 1.3), "white", 1),
    shape(f.circle(ex - 1.6, -7.2, 0.6) + f.circle(ex + 2.4, -7.4, 0.6) + `M${f.p(ex - 2, -4.6)}Q${f.p(ex, -2.6)} ${f.p(ex + 2.2, -4.8)}Q${f.p(ex, -4)} ${f.p(ex - 2, -4.6)}Z`, "slate-900", 1),
  ];
}

/** R3352000: a tiny shrimp mercenary standing upright on its curled tail and fan on the anchor, facing the way it faces, chest puffed: a segmented body (main), little legs, antennae, an eye on a stalk, a dented iron helmet, a short sword brandished forward and a coin pouch held out in its back claw; about 19 wide (pouch to sword) and 19 tall at scale 1. */
function shrimp(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -0.6, 0.6);
  const spine: Point[] = [[1, -0.9], [-2.2, -1.2], [-4, -3.6], [-4.3, -7], [-3, -10], [-0.8, -12.2], [0.8, -14], [1.2, -15.2]];
  const widths = [1, 2.4, 3.2, 4, 4.8, 5.4, 4.4, 2.8];
  const bx = 5.8;
  const by = -13 + swing * 0.3;
  const tx = 8.6 + swing;
  const ty = -18.2;
  const ux = (tx - bx) / Math.hypot(tx - bx, ty - by);
  const uy = (ty - by) / Math.hypot(tx - bx, ty - by);
  const W = 0.45;
  return [
    shape(
      f.band([[-2.6, -4.6], [-1.4, -3.6], [-0.8, -2.2]], 0.4) + f.band([[-2.4, -6.8], [-0.8, -5.8], [0, -4.2]], 0.4) + f.band([[-1.6, -8.8], [0.4, -8], [1.2, -6.2]], 0.4) +
        f.band([[-4.4, -11], [-6.6, -10], [-7.4, -8.4]], 0.8),
      "red-400",
      1,
    ),
    shape(`M${f.p(-8.1, -8)}Q${f.p(-9.8, -4.8)} ${f.p(-7.5, -4.5)}Q${f.p(-5.4, -4.8)} ${f.p(-6.9, -8)}Z`, "wood-600", 1, INKED),
    shape(f.poly([[0.4, -1.5], [2.4, -2], [3.8, -1], [3.6, 0], [2.4, -0.4], [1.6, 0.1], [0.2, -0.3]]), "red-300", 1, MAIN_INKED),
    shape(taper(f, spine, widths), "red-300", 1, MAIN_INKED),
    shape([1, 2, 3, 4].map((i) => f.band(edges(spine, widths, i, 0.9), 0.35)).join(""), "red-600", 0.6),
    shape(f.band([[-5.2, -3.4], [-5.6, -7], [-4.6, -10.2], [-2.8, -12.6]], 0.6), "white", 0.35),
    shape(f.band([[3, -14.9], [4, -15.6]], 0.5) + f.band([[3.4, -14.2], [5.6, -16], [5.4, -18.4], [3.4, -19.4]], 0.25) + f.band([[3.6, -13.8], [6.4, -14.6], [8, -13.4]], 0.25), "red-600", 1),
    shape(f.poly([[bx - W * uy, by + W * ux], [tx - 0.8 * ux - W * uy, ty - 0.8 * uy + W * ux], [tx, ty], [tx - 0.8 * ux + W * uy, ty - 0.8 * uy - W * ux], [bx + W * uy, by - W * ux]]), "slate-100", 1, INKED),
    shape(
      f.poly([[0.8, -11], [3, -10.6], [4.2, -11.8], [4.4, -13], [5.2, -12.8], [5.8, -11.4], [5, -10.2], [3.4, -9], [1, -9.6]]) + f.poly([[2.4, -14.6], [5, -15.2], [2.6, -13.8]]),
      "red-300",
      1,
      MAIN_INKED,
    ),
    shape(f.band([[bx + 1.2 * uy, by - 1.2 * ux], [bx - 1.2 * uy, by + 1.2 * ux]], 0.5) + f.circle(-7.5, -8.3, 0.5), "brass-500", 1),
    shape(`M${f.p(-1, -14.8)}Q${f.p(-1.1, -17.6)} ${f.p(1, -17.8)}Q${f.p(1.5, -17.1)} ${f.p(2.2, -17.6)}Q${f.p(3.6, -17.3)} ${f.p(3.6, -15)}L${f.p(4.2, -14.7)}L${f.p(-1.6, -14.5)}Z`, "slate-400", 1, INKED),
    shape(f.circle(4.3, -16, 0.6) + f.band([[-8, -7.7], [-7, -7.7]], 0.3), "slate-900", 1),
  ];
}

/** R3352000: a many-legged plague crawler on the anchor, facing the way it faces, its front reared up: a segmented body (main), pustules on its back, tiny round spectacles, one leg scratching and its front legs holding a little open book; about 23 long and 14 tall at scale 1 (antennae included). */
function crawler(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const kick = j(0.3);
  const spine: Point[] = [[-11.4, -2.8], [-8, -3.1], [-4.4, -3.2], [-0.8, -3.3], [2.4, -4.6], [4.4, -7.2], [5.2, -9.4]];
  const widths = [1.4, 3, 3.6, 3.8, 3.8, 3.4, 3];
  let legs = "";
  for (let i = 0; i < 7; i += 1) {
    const x = -9.8 + i * 1.85;
    legs += f.poly([[x - 0.35, -1.8], [x + 0.35, -1.8], [x - 0.5 + (i % 2 === 0 ? kick : -kick), 0]]);
  }
  return [
    shape(legs, "slate-900", 1),
    shape(taper(f, spine, widths), "wood-600", 1, MAIN_INKED),
    shape([1, 2, 3, 4].map((i) => f.band(edges(spine, widths, i, 0.9), 0.35)).join(""), "black", 0.3),
    shape(f.circle(5.9, -10.3, 2.2), "wood-600", 1, MAIN_INKED),
    shape(f.circle(-6.4, -4.8, 0.9) + f.circle(-2.6, -5.1, 1) + f.circle(1.8, -6.4 + j(0.2), 0.8), "lime-200", 1),
    shape(f.poly([[7.2, -8.8], [9.1, -7.8], [11.2, -9.2], [11.2, -6], [9.1, -4.8], [7.2, -6]]), "red-600", 1, INKED),
    shape(f.poly([[7.6, -8.2], [9.1, -7.4], [10.8, -8.5], [10.8, -6.3], [9.1, -5.3], [7.6, -6.2]]) + f.band([[-1.4, -6.8], [-0.4, -7.6]], 0.3), "slate-100", 1),
    shape(f.circle(7.6, -10.2, 0.65) + f.circle(5.8, -10.4, 0.65), "slate-100", 0.75, INKED),
    shape(
      f.circle(7.7, -10.1, 0.28) + f.circle(5.9, -10.3, 0.28) + f.band([[6.45, -10.4], [6.95, -10.3]], 0.2) + f.band([[5.15, -10.4], [3.9, -10.8]], 0.2) +
        f.band([[7.8, -13.8], [5.6, -12.3], [3.2, -13.4]], 0.3) +
        f.band([[4.6, -6.8], [7.6, -7.4]], 0.4) + f.band([[3.8, -5.6], [7.8, -5.6]], 0.4) + f.band([[-3.6, -1.8], [-4.6, -4.2], [-3, -5.6]], 0.4),
      "slate-900",
      1,
    ),
  ];
}

/** R3352000: a gaping worm-maw bursting up through a stone lane, its base on the anchor: a thick ringed body (main) rising to a wide round mouth seen at an angle, rings of hooked teeth round a dark throat, and broken flagstones thrown out round it; about 22 wide and 26 tall at scale 1. */
function maw(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const tilt = -12 + j(4);
  const cx = 1.6;
  const cy = -18.2;
  const c = Math.cos(tilt * DEGREES);
  const s = Math.sin(tilt * DEGREES);
  const on = (rx: number, ry: number, a: number): Point => {
    const ex = rx * Math.cos(a * DEGREES);
    const ey = ry * Math.sin(a * DEGREES);
    return [cx + ex * c - ey * s, cy + 0.3 + ex * s + ey * c];
  };
  const teeth = (n: number, base: number, tip: number): string => {
    let d = "";
    for (let i = 0; i < n; i += 1) {
      const a = (i * 360) / n;
      const half = 360 / n / 4.4;
      d += f.poly([on(7.6 * base, 3.7 * base, a - half), on(7.6 * tip, 3.7 * tip, a + half * 1.8), on(7.6 * base, 3.7 * base, a + half)]);
    }
    return d;
  };
  const spine: Point[] = [[0, 1], [-1.2, -2.4], [-1.8, -5.6], [-1.2, -8.8], [0, -11.8], [1.5, -15.4]];
  const widths = [10.4, 9.6, 8.8, 8.4, 8.8, 11.6];
  const ring = (i: number): string => {
    const [a, b] = edges(spine, widths, i, 0.96) as [Point, Point];
    return f.band([a, [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2 + 1.1], b], 0.6);
  };
  return [
    shape(f.ellipse(0, -0.2, 10, 2.3), "slate-900", 1),
    shape(f.poly([[-10.2, -7 + j(1)], [-7.4, -8.4], [-6.6, -6.2], [-9.4, -5]]) + f.poly([[7.4, -9 + j(1)], [10.2, -7.6], [9.6, -5.4], [7, -6.2]]), "stone-500", 1, INKED),
    shape(taper(f, spine, widths), "red-400", 1, MAIN_INKED),
    shape(ring(1) + ring(2) + ring(3) + ring(4), "red-900", 0.45),
    shape(f.band([[3.6, 0], [2.2, -5.6], [3, -12.4]], 1.8), "black", 0.2),
    shape(f.band([[-4, -1.6], [-4.6, -6], [-3.2, -12]], 0.9), "white", 0.25),
    shape(tilted(f, at, cx, cy, 9.8, 5.2, tilt), "red-400", 1, MAIN_INKED),
    shape(tilted(f, at, cx, cy + 0.3, 7.6, 3.7, tilt), "red-900", 1),
    shape(tilted(f, at, cx + 0.4, cy + 0.6, 3.6, 1.6, tilt), "black", 0.9),
    shape(teeth(10, 1.02, 0.66), "slate-100", 1),
    shape(`M${f.p(...on(9, 4.4, 175))}Q${f.p(...on(10.4, 5.6, 222))} ${f.p(...on(9, 4.4, 265))}Q${f.p(...on(8.6, 4, 222))} ${f.p(...on(9, 4.4, 175))}Z`, "white", 0.3),
    shape(f.poly([[-11, 0.6], [-9.4, -2.8], [-5.6, -2], [-6.2, 1.2]]) + f.poly([[5, 1], [6.2, -2.4], [9.8, -1.4], [10.2, 1.4]]) + f.poly([[-3.2, 1.8], [-2.6, -0.2], [1.8, 0], [2.4, 2]]), "stone-500", 1, INKED),
  ];
}

/** R3352000: a skeleton warrior standing on the anchor, facing the way it faces: a skull with dark sockets and a jaw, a ribcage, bony limbs, a round wooden shield (main) on its near arm and a notched sword raised in the other; about 9 wide and 16 tall at scale 1 (the sword's tip a little higher). */
function skeleton(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gape = between(rng, 0, 0.5);
  const angle = between(rng, 5, 35) * DEGREES;
  const hx = -1.8;
  const hy = -12.6 + between(rng, -0.4, 0.4);
  const ux = Math.sin(angle);
  const uy = -Math.cos(angle);
  const A = (along: number, across: number): Point => [hx + ux * along - uy * across, hy + uy * along + ux * across];
  const L = 4.6;
  const w = 0.45;
  return [
    shape(f.poly([A(0.3, w), A(L * 0.4, w), A(L * 0.46, 0.05), A(L * 0.54, w), A(L * 0.86, w), A(L, 0), A(L * 0.86, -w), A(L * 0.7, -w), A(L * 0.75, -0.1), A(L * 0.8, -w), A(0.3, -w)]), "slate-400", 1, INKED),
    shape(f.band([A(-1.2, 0), A(-0.35, 0)], 0.4) + f.band([A(0, -0.9), A(0, 0.9)], 0.45), "wood-800", 1),
    shape(f.band([[-0.4, -10.4], [-2.4, -10.8], [hx, hy + 0.3]], 0.6) + f.band([[-0.4, -5.8], [-1.2, -3], [-1.8, -0.4], [-0.6, -0.3]], 0.75), "slate-400", 1),
    shape(f.band([[0.2, -7.8], [0.1, -6.4]], 0.7) + f.band([[0.6, -5.8], [1.3, -3.1], [1, -0.4], [2.4, -0.3]], 0.8), "slate-100", 1),
    shape(f.poly([[-1.2, -6.9], [1.4, -6.9], [1.1, -5.5], [-0.9, -5.5]]), "slate-100", 1, INKED),
    shape(
      smooth(f, [[-1.4, -10.8], [0.4, -11.4], [1.8, -10.4], [1.4, -8], [0, -7.4], [-1.4, -8.2]]) +
        [-10, -8.8].map((y) => f.poly([[-0.9, y], [0.2, y - 0.15], [1, y], [1, y + 0.3], [0.2, y + 0.15], [-0.9, y + 0.3]])).join(""),
      "slate-100",
      1,
      INKED,
    ),
    shape(
      `M${f.p(-1.1, -12.5)}C${f.p(-1.4, -15.6)} ${f.p(3.2, -15.8)} ${f.p(2.9, -13)}L${f.p(2.7, -11.5)}L${f.p(0.3, -11.5)}Q${f.p(-0.8, -11.8)} ${f.p(-1.1, -12.5)}Z` +
        f.poly([[0.2, -11.1 + gape], [2.6, -11 + gape], [2.3, -10.1 + gape], [0.6, -10.2 + gape]]),
      "slate-100",
      1,
      INKED,
    ),
    shape(f.circle(1.7, -13.1, 0.65) + f.poly([[2.6, -12.4], [2.9, -11.9], [2.4, -11.9]]), "slate-900", 1),
    shape(f.circle(3.6, -7.4, 2.5), "wood-600", 1, MAIN_INKED),
    shape(f.band([[2.8, -9.6], [2.8, -5.2]], 0.3) + f.band([[4.4, -9.6], [4.4, -5.2]], 0.3), "black", 0.3),
    shape(f.circle(3.6, -7.4, 2.5) + f.circle(3.6, -7.4, 2.1) + f.circle(3.6, -7.4, 0.7), "slate-400", 1),
  ];
}

/** R3352000: a horned rock lizard standing on four splayed legs on the anchor, facing the way it faces, head raised and jaws open (its flame a part of its own placed before it): horns, a row of spines down its back, a long tail and rocky scales (main); about 26 long and 13 tall at scale 1 (horns included). */
function lizard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const sweep = j(0.8);
  const spike = 1.5 + j(0.3);
  const spine: Point[] = [[-13, -1.2 + sweep], [-9.6, -2.2], [-6, -3.4], [-2, -4.2], [2, -4.2], [4.8, -5], [6.6, -6.4]];
  const leg = (points: readonly Point[]): string => taper(f, points, [1.8, 1.1, 0.9, 0.7]);
  return [
    shape(f.band([[2, -3.4], [3.6, -1.8], [3.2, 0]], 0.9) + f.band([[-2.2, -3.4], [-3.4, -1.8], [-2.8, 0]], 0.9), "slate-700", 1),
    shape(sawtooth(f, [-9.6, -2.8], [-2, -5.7], 4, spike, 0) + sawtooth(f, [-2, -5.7], [5, -6.4], 3, spike, 0), "slate-800", 1),
    shape(f.poly([[7.4, -8], [12.8, -10.8], [12.2, -6.6]]), "red-900", 1),
    shape(taper(f, spine, [0.4, 1.6, 3, 3.9, 4.1, 3.4, 2.8]), "stone-500", 1, MAIN_INKED),
    shape(
      `M${f.p(5.8, -6.6)}Q${f.p(5.6, -9.8)} ${f.p(8.4, -10.4)}Q${f.p(10.8, -11.4)} ${f.p(13.2, -11.2)}L${f.p(12.6, -10.2)}L${f.p(8.8, -8.4)}L${f.p(12.4, -7.4)}Q${f.p(12.2, -6.2)} ${f.p(10.6, -5.8)}Q${f.p(7.8, -4.8)} ${f.p(5.8, -6.6)}Z`,
      "stone-500",
      1,
      MAIN_INKED,
    ),
    shape(leg([[3.6, -3.4], [5.8, -2.6], [6, -0.6], [7.2, -0.2]]) + leg([[-3.4, -3.2], [-5.8, -2.4], [-6.2, -0.6], [-4.8, -0.2]]), "stone-500", 1, MAIN_INKED),
    shape(f.band([[-9, -1.6], [-2, -2.2], [4, -2.4]], 0.8) + f.circle(-5.4, -3.2, 0.55) + f.circle(-1.4, -3.4, 0.65) + f.circle(2.4, -3.4, 0.5), "black", 0.25),
    shape(f.circle(-3.4, -4.8, 0.5) + f.circle(0.6, -5.2, 0.55) + f.circle(4, -5.8, 0.4), "white", 0.3),
    shape(`M${f.p(6.4, -9.4)}Q${f.p(4.6, -10.2)} ${f.p(3.4, -12)}Q${f.p(5.6, -11.2)} ${f.p(7.4, -10)}Z` + `M${f.p(7.8, -10.1)}Q${f.p(7.4, -11.6)} ${f.p(6.8, -12.8)}Q${f.p(8.2, -11.8)} ${f.p(8.8, -10.3)}Z`, "slate-100", 1, INKED),
    shape(f.poly([[10.2, -9.1], [10.6, -8.4], [11, -9.3]]) + f.poly([[10.8, -7.6], [11.2, -8.3], [11.5, -7.5]]), "white", 1),
    shape(f.circle(8.2, -8.9, 0.5), "orange-400", 1),
    shape(f.band([[8.2, -9.3], [8.2, -8.5]], 0.18), "slate-900", 1),
  ];
}

/** R3352000: a deer leaping, facing the way it faces, its hind hooves on the anchor: antlers up, legs stretched fore and aft, a white tail flash, its coat main; about 17 long and 18 tall at scale 1 (antlers included). */
function leapingDeer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.6, 0.6);
  return [
    shape(f.band([[8.4, -8.6], [10.6, -8.8], [12.6, -8 + reach]], 0.8) + f.band([[1.6, -7.6], [0.2, -5.2], [-1.2, -3], [-1.6, -0.6]], 0.8), "wood-800", 1),
    shape(smooth(f, [[0.4, -9.6], [3, -11.2], [7, -11.4], [9.2, -14.6], [11, -13.4], [10.4, -10], [9.4, -7.4], [5, -6.6], [1.6, -6.2]]), "wood-600", 1, MAIN_INKED),
    shape(smooth(f, [[9.6, -14.4], [11.4, -15.4], [13.8, -14], [14.4, -12.8], [13.4, -12.2], [11, -12.6]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[9, -8.2], [11.4, -7.4], [13.6, -6.6 + reach]], 0.85) + f.band([[2.4, -7.4], [1.4, -4.6], [0.2, -2.4], [0.4, -0.4]], 0.85) + f.poly([[10.6, -14.6], [9.4, -16.2], [11.2, -15.2]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[3, -7], [6, -7], [9, -8.6]], 0.9) + f.ellipse(10.6, -11.6, 0.8, 1.2), "white", 0.45),
    shape(f.band([[2.6, -10.6], [6, -10.9], [8.6, -12.2]], 0.6), "white", 0.18),
    shape(smooth(f, [[1.2, -8.6], [-0.8, -10.8], [0.4, -11.6], [2, -9.6]]), "white", 1, INKED),
    shape(
      f.band([[11, -15], [10.6, -16.8], [9.2, -18]], 0.45) + f.band([[11, -16.45], [11.9, -17.6]], 0.35) + f.band([[11.8, -14.9], [12.6, -16.6], [12, -18.2]], 0.45) + f.band([[12.8, -16.35], [13.7, -17.2]], 0.35),
      "brass-500",
      1,
    ),
    shape(f.circle(12.4, -13.8, 0.38) + f.circle(14.2, -13, 0.32) + f.band([[13.6, -6.8 + reach], [14.2, -6.2 + reach]], 0.85) + f.band([[0.4, -0.4], [0.5, 0.1]], 0.85) + f.band([[-1.6, -0.6], [-1.7, -0.1]], 0.8), "slate-900", 1),
  ];
}

/** R3352000: a mangy mule galloping on the anchor, facing the way it faces, mid-sneeze: long ears, a scruffy mane, a patchy coat (main), a spray from its nose, a saddle with a stolen keepsake dangling; its saddle seat's top is at (-1, -13), where a recipe sets a rider; about 28 long and 21 tall at scale 1 (ears included, its sneeze 6 further). */
function mule(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const swing = j(0.6);
  const leg = (points: readonly Point[]): string => taper(f, points, [2.4, 1.4, 1.1]);
  return [
    shape(leg([[5.2, -8.6], [7, -5.4], [5.4, -3.8]]) + leg([[-6.8, -8.8], [-5.8, -4.4], [-4.4, -0.6]]), "slate-700", 1),
    shape(smooth(f, [[-10.6, -10.6], [-6, -13], [-1, -12.4], [4, -13], [9.6, -17.6], [12, -14], [9, -10.4], [7, -6.6], [-1, -6.6], [-9.6, -7.2]]), "stone-500", 1, MAIN_INKED),
    shape(smooth(f, [[9.4, -16.6], [11.8, -17], [14, -14.4], [15.2, -12], [14.4, -10.8], [12.8, -11.2], [10.4, -13.4]]), "stone-500", 1, MAIN_INKED),
    shape(
      leg([[6, -8.6], [8.6, -5.6], [11.2, -4.4 + swing]]) + leg([[-7.8, -8.6], [-10.2, -5], [-12.2, -2.6 - swing]]) + f.poly([[10, -16.6], [8.2, -20.6], [10.9, -17.1]]) + f.poly([[11.3, -17.2], [11.8, -21], [12.4, -16.8]]),
      "stone-500",
      1,
      MAIN_INKED,
    ),
    shape(f.ellipse(-5.2, -9.8, 2.2, 1.4) + f.ellipse(4.4, -9, 1.6, 1.1), "black", 0.25),
    shape(sawtooth(f, [10, -17], [4.8, -12.8], 4, 1.6, 0.2) + f.poly([[-10.2, -11], [-12.8, -10.4], [-14, -7.4], [-12.8, -8.6], [-12.2, -9.8], [-10.4, -10]]), "slate-800", 1),
    shape(f.poly([[-6, -12.4], [3.4, -12.6], [3, -8.8], [-5.6, -8.6]]), "red-600", 1, INKED),
    shape(`M${f.p(-4.6, -11.6)}L${f.p(-4.6, -14.6)}Q${f.p(-3, -12.8)} ${f.p(-1, -13)}Q${f.p(1, -12.8)} ${f.p(2, -14.4)}L${f.p(2.4, -11.6)}Z`, "wood-700", 1, INKED),
    shape(f.circle(3.5, -7.6, 0.8), "brass-500", 1, INKED),
    shape(f.poly([[15.2, -12.4], [18.6, -14.8], [19.4, -13.6], [20.8, -13.4], [20, -11.8], [21, -10.4], [19.6, -10], [18.8, -8.8], [15.2, -11.4]]), "green-400", 0.6),
    shape(
      f.band([[11.3, -15.2], [12.6, -15.5]], 0.25) + f.circle(14.8, -11.9, 0.3) + f.band([[2.8, -11.6], [3.4, -8.5]], 0.2) +
        f.band([[11, -4.6 + swing], [11.8, -4.1 + swing]], 1.1) + f.band([[-12.1, -2.8 - swing], [-12.6, -2.2 - swing]], 1.1),
      "slate-900",
      1,
    ),
  ];
}

export const CLASSIC_KIT = {
  // Creatures
  tortoise: { category: "creatures", draw: tortoise },
  slime: { category: "creatures", draw: slime },
  shrimp: { category: "creatures", draw: shrimp },
  crawler: { category: "creatures", draw: crawler },
  maw: { category: "creatures", draw: maw },
  skeleton: { category: "creatures", draw: skeleton },
  lizard: { category: "creatures", draw: lizard },
  "leaping-deer": { category: "creatures", draw: leapingDeer },
  mule: { category: "creatures", draw: mule },
  // The props (sceneKitClassicProps.ts), after the creatures.
  ...CLASSIC_PROPS_KIT,
} as const satisfies Readonly<Record<string, KitPart>>;
