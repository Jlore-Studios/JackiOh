// R3401000: the scene kit's ink-wash creatures and people (ART.md, "Scene recipes and kits"), which the
// Meditative scenes (#3401) added: the animals (a red-crowned crane in flight and a pair of koi seen from
// above in water, both centred on their anchor; a beetle, a sheep and a two-dancer lion-dance costume standing
// on it), a small distant person in a few brush strokes standing (or walking, or seated) on it, two near hands
// that reach into a picture (an open palm, its centre on the anchor, and a pinch, holding whatever stands on
// the anchor just above its fingertips) and a glitching drop of ink scampering on it. sceneKit.ts spreads
// them into `SCENE_KIT`. Each part keeps the kit's rules: one to twelve filled shapes drawn in its own frame
// (`local`), varied by the scene's stream alone, every colour a palette.css name none in Radiant's gold
// window, a glow or a wash a stack of translucent shapes, never a gradient, and every shape filled even-odd,
// so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { between, DEGREES, INKED, leant, local, MAIN, MAIN_INKED, shape, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

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

/** A lobed outline round an ellipse about (cx, cy): `lobes` bulges, each curve's control `bulge` times the radius out (wool, a cloud). */
function lobed(f: Local, rng: Rng, cx: number, cy: number, rx: number, ry: number, lobes: number, bulge: number): string {
  const start = rng() * (360 / lobes);
  const at = (i: number, reach: number): string => {
    const angle = (start + (i * 360) / lobes) * DEGREES;
    return f.p(cx + rx * reach * Math.cos(angle), cy + ry * reach * Math.sin(angle));
  };
  let d = `M${at(0, 1)}`;
  for (let i = 0; i < lobes; i += 1) d += `Q${at(i + 0.5, bulge * between(rng, 0.94, 1.06))} ${at(i + 1, 1)}`;
  return `${d}Z`;
}

/** A sawtooth of `teeth` tufts from `a` to `b`, each `depth` off the line on its left as drawn (up, going right), closed back along it. */
function fringe(f: Local, a: Point, b: Point, teeth: number, depth: number): string {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const length = Math.hypot(dx, dy) || 1;
  const points: Point[] = [a];
  for (let i = 0; i < teeth; i += 1) {
    const t = (i + 0.5) / teeth;
    points.push([a[0] + dx * t + (dy / length) * depth, a[1] + dy * t - (dx / length) * depth]);
    points.push([a[0] + (dx * (i + 1)) / teeth, a[1] + (dy * (i + 1)) / teeth]);
  }
  return f.poly(points);
}

/**
 * The outline of a finger from `a` (where the pen is, one side of its root) round its tip at `tip` to `b` (the
 * root's other side), narrowing to `narrow` of its root's width, bowed `bend` across it (up, as drawn pointing right).
 */
function digit(f: Local, a: Point, b: Point, tip: Point, bend: number, narrow: number): string {
  const mx = (a[0] + b[0]) / 2;
  const my = (a[1] + b[1]) / 2;
  const length = Math.hypot(tip[0] - mx, tip[1] - my) || 1;
  const ux = (tip[0] - mx) / length;
  const uy = (tip[1] - my) / length;
  const side = (a[0] - mx) * -uy + (a[1] - my) * ux >= 0 ? 1 : -1;
  const vx = -uy * side;
  const vy = ux * side;
  const root = Math.hypot(a[0] - b[0], a[1] - b[1]) / 2;
  const half = root * narrow;
  const end = length - half;
  const at = (along: number, across: number, bow: number): string => f.p(mx + ux * along + vx * across + uy * bow, my + uy * along + vy * across - ux * bow);
  const round = half * 1.33;
  return (
    `Q${at(end * 0.5, (root + half) / 2, bend)} ${at(end, half, 0)}` +
    `C${at(end + round, half, 0)} ${at(end + round, -half, 0)} ${at(end, -half, 0)}` +
    `Q${at(end * 0.5, -(root + half) / 2, bend)} ${f.p(...b)}`
  );
}

/** A wing from its root, `length` long at `angle` degrees (0 the way it faces, -90 straight up), its trailing edge `chord` behind: the wing and its black tip. */
function craneWing(f: Local, root: Point, angle: number, length: number, chord: number): { wing: string; tip: string } {
  const ux = Math.cos(angle * DEGREES);
  const uy = Math.sin(angle * DEGREES);
  const W = (along: number, back: number): string => f.p(root[0] + ux * along + uy * back, root[1] + uy * along - ux * back);
  const L = length;
  const fingers = `L${W(L - 0.9, 0.8)}L${W(L - 0.6, 1.05)}L${W(L - 1.6, 1.5)}L${W(L - 1.3, 1.8)}L${W(L - 2.4, 2.1)}`;
  return {
    wing: `M${W(0, 0)}Q${W(L * 0.45, -0.7)} ${W(L, 0)}${fingers}Q${W(L * 0.35, chord + 0.5)} ${W(0, chord)}Z`,
    tip: `M${W(L * 0.6, -0.33)}Q${W(L * 0.8, -0.25)} ${W(L, 0)}${fingers}L${W(L * 0.6, 1.9)}Z`,
  };
}

// Creatures

/** A red-crowned crane in flight centred on the anchor, flying the way it faces: white body and wings (main) with black tips, a black neck, a red crown, legs trailing; about 16 wide and 8 tall at scale 1. */
function crane(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sweep = between(rng, -132, -88);
  const far = craneWing(f, [0.6, 0.9], sweep - between(rng, 16, 28), 5.2, 2.2);
  const near = craneWing(f, [1.0, 1.4], sweep, 6, 2.6);
  const trail = between(rng, -0.3, 0.3);
  const body = `M${f.p(-3.4, 2.4)}Q${f.p(-1.4, 0.6)} ${f.p(1.8, 1.2)}Q${f.p(2.8, 1.5)} ${f.p(2.6, 2.2)}Q${f.p(0.4, 3.4)} ${f.p(-3.4, 2.4)}Z`;
  const neck = f.band([[2.2, 1.7], [4.4, 1.1], [6.2, 0.95]], 0.7) + f.ellipse(6.6, 0.95, 0.75, 0.5);
  const tuft = `M${f.p(-2.2, 2.0)}Q${f.p(-3.9, 1.6)} ${f.p(-4.8, 2.7)}Q${f.p(-3.6, 2.5)} ${f.p(-2.4, 2.9)}Z`;
  return [
    shape(far.wing, "white", 1, MAIN_INKED),
    shape(far.wing, "slate-400", 0.3),
    shape(far.tip, "slate-900", 1),
    shape(f.band([[-2.6, 2.6], [-5.2, 2.9 + trail * 0.5], [-7.9, 3.1 + trail]], 0.28) + f.band([[-2.2, 2.9], [-5, 3.4 + trail * 0.5], [-7.6, 3.8 + trail]], 0.28), "slate-900", 1),
    shape(body, "white", 1, MAIN_INKED),
    shape(`M${f.p(-3, 2.6)}Q${f.p(0, 2.2)} ${f.p(2.4, 2.1)}Q${f.p(0.4, 3.1)} ${f.p(-3, 2.6)}Z`, "slate-400", 0.35),
    shape(neck + tuft, "slate-900", 1),
    shape(f.poly([[7.2, 0.7], [8.4, 1.1], [7.2, 1.3]]), "stone-500", 1),
    shape(f.circle(6.5, 0.58, 0.38), "red-600", 1),
    shape(near.wing, "white", 1, MAIN_INKED),
    shape(near.tip, "slate-900", 1),
  ];
}

type Koi = { body: string; fins: string; patches: string; eyes: string };

/** One koi seen from above, its middle at (cx, cy) heading `heading` degrees, its spine bowed `bend`: about 8 long and 2.4 wide. */
function koiFish(f: Local, rng: Rng, cx: number, cy: number, heading: number, bend: number): Koi {
  const xs = [3.1, 1.8, 0.3, -1.1, -2.4];
  const halves = [0.95, 1.2, 1.05, 0.65, 0.3];
  const spineY = (x: number): number => bend * (x / 4) ** 2;
  const T = (x: number, y: number): Point => {
    const [rx, ry] = turned([x, y], heading);
    return [cx + rx, cy + ry];
  };
  const left: Point[] = [];
  const right: Point[] = [];
  xs.forEach((x, i) => {
    const half = halves[i] ?? 0.3;
    const slope = (bend * x) / 8;
    const n = Math.hypot(1, slope);
    left.push(T(x + (slope / n) * half, spineY(x) - half / n));
    right.push(T(x - (slope / n) * half, spineY(x) + half / n));
  });
  const nose = T(4.4, spineY(4.4));
  const body = smooth(f, [nose, ...left, T(-3, spineY(-3)), ...right.reverse()]);
  const yb = spineY(-2.6);
  const sway = between(rng, -0.5, 0.5);
  const tail =
    `M${f.p(...T(-2.3, yb - 0.3))}Q${f.p(...T(-3.4, yb - 1.7 + sway))} ${f.p(...T(-4.7, yb - 1.5 + sway))}Q${f.p(...T(-4, yb - 0.5 + sway))} ${f.p(...T(-3.8, yb + sway * 0.6))}` +
    `Q${f.p(...T(-4, yb + 0.5 + sway))} ${f.p(...T(-4.7, yb + 1.5 + sway))}Q${f.p(...T(-3.4, yb + 1.7 + sway))} ${f.p(...T(-2.3, yb + 0.3))}Z`;
  const pectoral = (s: number): string => {
    const y = spineY(1.8) + s * 1.0;
    return `M${f.p(...T(2.1, y))}Q${f.p(...T(1.4, y + s * 1.5))} ${f.p(...T(0.4, y + s * 1.4))}Q${f.p(...T(0.9, y + s * 0.5))} ${f.p(...T(1.2, y - s * 0.1))}Z`;
  };
  const patch = (x: number, across: number, r: number): string => {
    const y = spineY(x) + across;
    return smooth(f, [T(x + r, y), T(x + r * 0.3, y - r * 0.8), T(x - r * 0.8, y - r * 0.6), T(x - r, y + r * 0.3), T(x - r * 0.1, y + r * 0.8), T(x + r * 0.8, y + r * 0.5)]);
  };
  const spots = [patch(2.9, between(rng, -0.25, 0.25), 0.55)];
  if (rng() < 0.75) spots.push(patch(0.9, between(rng, -0.35, 0.35), between(rng, 0.5, 0.75)));
  if (rng() < 0.6) spots.push(patch(-1.1, between(rng, -0.2, 0.2), 0.45));
  return {
    body,
    fins: tail + pectoral(-1) + pectoral(1),
    patches: spots.join(""),
    eyes: f.circle(...T(3.5, spineY(3.5) - 0.65), 0.2) + f.circle(...T(3.5, spineY(3.5) + 0.65), 0.2),
  };
}

/** One or two koi (body main, orange-400) seen from above in still water, circling about the anchor, white-patched, a ripple by them; each about 8 long and 2.4 wide at scale 1. */
function koi(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pair = rng() < 0.55;
  const fish = pair
    ? [koiFish(f, rng, -0.8, -2.6, between(rng, -8, 8), between(rng, 0.4, 1)), koiFish(f, rng, 0.8, 2.6, 180 + between(rng, -8, 8), between(rng, 0.4, 1))]
    : [koiFish(f, rng, 0, 0, between(rng, -20, 20), between(rng, -1.2, 1.2))];
  const rx = between(rng, 2.2, 2.8);
  const ripple = (cx: number, cy: number, r: number): string => f.ellipse(cx, cy, r, r * 0.55) + f.ellipse(cx, cy, r - 0.3, r * 0.55 - 0.25);
  const nose = pair ? [5.6, -2.2] : [6, 0];
  return [
    shape(f.ellipse(0, 0, 7.5, pair ? 5.6 : 3.6), "jade-400", 0.12),
    shape(ripple(nose[0] ?? 6, nose[1] ?? 0, rx) + ripple(nose[0] ?? 6, nose[1] ?? 0, rx * 0.5), "slate-400", 0.4),
    shape(fish.map((k) => k.fins).join(""), "white", 0.6),
    shape(fish.map((k) => k.body).join(""), "orange-400", 1, MAIN_INKED),
    shape(fish.map((k) => k.patches).join(""), "white", 0.95),
    shape(fish.map((k) => k.eyes).join(""), "slate-900", 1),
  ];
}

/** A beetle standing on the anchor, facing the way it faces: a domed shell (main, green-900) split by its midline, a dark head, six legs, two antennae and a shine; about 5 wide and 4 tall at scale 1. */
function beetle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const X = -0.3;
  const leg = (points: readonly Point[]): string => f.band(points.map(([x, y]) => [x + X, y] as const), 0.3);
  const shell = `M${f.p(X - 2.5, -1)}C${f.p(X - 2.6, -3.1)} ${f.p(X + 0.6, -3.7)} ${f.p(X + 1.5, -1.6)}Q${f.p(X + 1.6, -0.9)} ${f.p(X + 1, -0.8)}Q${f.p(X - 1, -0.5)} ${f.p(X - 2.5, -1)}Z`;
  const head = `M${f.p(X + 1, -0.9)}Q${f.p(X + 1.2, -2.4)} ${f.p(X + 2.2, -2)}Q${f.p(X + 2.7, -1.7)} ${f.p(X + 2.6, -1.2)}Q${f.p(X + 3.1, -1.1)} ${f.p(X + 2.9, -0.7)}Q${f.p(X + 2, -0.45)} ${f.p(X + 1, -0.9)}Z`;
  const stride = j(0.25);
  return [
    shape(leg([[0.8, -1.2], [1.6, -1.5], [2.3, -0.3 + stride]]) + leg([[-0.2, -1.2], [-0.1, -1.6], [0.4, -0.35]]) + leg([[-1.4, -1.2], [-2.1, -1.5], [-2.7, -0.35 - stride]]), "slate-700", 1),
    shape(leg([[1, -0.9], [1.8, -0.8], [2.4 + stride, 0]]) + leg([[-0.2, -0.8], [0.3, -0.55], [0.1, 0]]) + leg([[-1.4, -0.9], [-2.1, -0.7], [-2.6 - stride, 0]]), "slate-900", 1),
    shape(
      f.band([[X + 2.8, -1.3], [X + 3.4, -2 + j(0.2)], [X + 3.2, -2.9 + j(0.3)]], 0.16) + f.band([[X + 2.5, -1.75], [X + 2.8, -2.5 + j(0.2)], [X + 2.4, -3.3 + j(0.3)]], 0.16),
      "slate-900",
      1,
    ),
    shape(shell, "green-900", 1, MAIN_INKED),
    shape(`M${f.p(X - 1.9, -1.9)}Q${f.p(X - 1.2, -3.2)} ${f.p(X + 0.6, -2.6)}Q${f.p(X - 0.6, -2.4)} ${f.p(X - 1.9, -1.9)}Z`, "jade-400", 0.45),
    shape(f.band([[X + 1.1, -1.9], [X - 0.3, -2.25], [X - 1.6, -1.8], [X - 2.3, -1.15]], 0.18), "slate-900", 0.85),
    shape(head, "slate-900", 1),
    shape(f.band([[X - 1.5, -2.35], [X - 0.9, -2.9], [X - 0.1, -3]], 0.32), "white", 0.7),
  ];
}

/** A fluffy sheep standing on the anchor, facing the way it faces: a cloud of wool (main, white), a dark face and ears, four thin legs; about 12 wide and 9 tall at scale 1. */
function sheep(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.6, 1.4);
  const stance = between(rng, -0.3, 0.3);
  const tilt = between(rng, -12, 8);
  const H = (x: number, y: number): Point => {
    const [rx, ry] = turned([x - 3.6, y + 6.2], tilt);
    return [rx + 3.6, ry - 6.2 + lift];
  };
  const hp = (x: number, y: number): string => f.p(...H(x, y));
  const head = `M${hp(3.4, -6.9)}Q${hp(5.2, -7.9)} ${hp(6.1, -6.2)}Q${hp(6.7, -4.9)} ${hp(5.8, -4.4)}Q${hp(4.6, -4.1)} ${hp(3.7, -5.1)}Z`;
  const ears = `M${hp(4.2, -6.9)}Q${hp(3.3, -7.6)} ${hp(2.5, -7.1)}Q${hp(3.3, -6.6)} ${hp(4.1, -6.4)}Z`;
  const curl = (x: number, y: number): string => f.band([[x - 0.6, y - 0.1], [x - 0.1, y + 0.4], [x + 0.5, y]], 0.26);
  const curls: Point[] = [[-3.6, -5.6], [-1.4, -6.6], [0.6, -4.3], [-2.4, -3.8], [1.8, -6.2]];
  return [
    shape(f.band([[1.3, -2.8], [1.2 + stance, 0]], 0.5) + f.band([[-2.1, -2.8], [-2 - stance, 0]], 0.5), "slate-700", 1),
    shape(f.band([[2.4, -2.8], [2.6 - stance, 0]], 0.55) + f.band([[-3.3, -2.8], [-3.5 + stance, 0]], 0.55), "slate-900", 1),
    shape(lobed(f, rng, -0.8, -5.2, 4.8, 2.6, whole(rng, 8, 10), 1.34), "white", 1, MAIN_INKED),
    shape(`M${f.p(-5, -4.4)}Q${f.p(-0.8, -1.4)} ${f.p(3.4, -4.4)}Q${f.p(-0.8, -3)} ${f.p(-5, -4.4)}Z`, "slate-400", 0.4),
    shape(curls.slice(0, whole(rng, 3, 5)).map(([x, y]) => curl(x, y)).join(""), "slate-400", 0.6),
    shape(ears, "slate-900", 1),
    shape(head, "slate-900", 1),
    shape(lobed(f, rng, ...H(4.1, -7), 0.9, 0.55, 5, 1.4), "white", 1, MAIN),
    shape(f.circle(...H(5.1, -6.2), 0.24), "white", 0.9),
  ];
}

/** A lion-dance costume standing on the anchor, facing the way it faces: a great lion head with a mane, round eyes, a horn and an open mouth, its body cloth (main, red-600) draped over the dancer behind, two pairs of legs below; about 18 wide and 14 tall at scale 1. */
function lionDancer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -7, 6);
  const hump = between(rng, -0.8, 0.6);
  const wide = between(rng, -0.3, 0.3);
  const H = (x: number, y: number): Point => {
    const [rx, ry] = turned([x - 3.2, y + 7], tilt);
    return [rx + 3.2, ry - 7];
  };
  const hp = (x: number, y: number): string => f.p(...H(x, y));
  const legs: [Point, Point, Point][] = [
    [[2.5, -5], [1.4 - wide, -2.7], [1.9, -0.6]],
    [[4.7, -5], [5.8 + wide, -2.7], [5.3, -0.6]],
    [[-6.4, -4.6], [-7.3 - wide, -2.5], [-6.8, -0.6]],
    [[-4.2, -4.8], [-3.3 + wide, -2.5], [-3.8, -0.6]],
  ];
  const cloth =
    `M${f.p(3.4, -11.4)}Q${f.p(-0.8, -12.6 + hump)} ${f.p(-4.6, -10.6 + hump)}Q${f.p(-7.9, -8.8 + hump * 0.5)} ${f.p(-8.6, -6.2)}` +
    `Q${f.p(-9.6, -4.6)} ${f.p(-8.4, -4)}L${f.p(-7.3, -4.7)}Q${f.p(-2.2, -5.4)} ${f.p(3.4, -5.6)}Z`;
  const head =
    `M${hp(2.6, -7)}C${hp(1.8, -10.4)} ${hp(3, -13.2)} ${hp(6, -13)}C${hp(8.4, -12.8)} ${hp(9.4, -11.4)} ${hp(9.2, -9.8)}` +
    `L${hp(9, -8.3)}Q${hp(9.3, -7.2)} ${hp(8.7, -6.6)}Q${hp(6.2, -5.4)} ${hp(3.4, -6.1)}Z`;
  const mane = f.poly([H(2.9, -7.6), H(1.6, -8.3), H(2.6, -8.9), H(1.3, -9.9), H(2.5, -10.4), H(1.6, -11.6), H(3, -11.6), H(2.6, -13), H(4, -12.6), H(4.2, -14), H(5.3, -13.1), H(6.2, -13), H(4, -11.5)]);
  const beard = f.poly([H(4.4, -6), H(4.9, -4.6), H(5.6, -5.6), H(6.3, -4.4), H(6.9, -5.6), H(7.8, -4.6), H(8.4, -6.2), H(6.4, -5.7)]);
  const hem = fringe(f, [2, -5.65], [-7.3, -4.7], 9, -1.1);
  const cuffs = legs.map(([, , ankle]) => f.band([[ankle[0] - 0.65, ankle[1] - 0.55], [ankle[0] + 0.65, ankle[1] - 0.55]], 0.45)).join("");
  const scale = (x: number, y: number): string => f.band([[x - 0.7, y - 0.25], [x, y + 0.45], [x + 0.7, y - 0.25]], 0.24);
  const eye = (x: number, y: number, r: number): string => f.circle(...H(x, y), r);
  const brow = (x: number): string => f.band([H(x - 1.1, -11.7), H(x, -12.5), H(x + 1, -11.9)], 0.45);
  const teeth = f.poly([H(8.9, -8.25), H(6.8, -7.9), H(7.2, -7.5), H(7.6, -7.85), H(8, -7.4), H(8.4, -7.9), H(8.8, -7.5)]);
  return [
    shape(legs.map((leg) => f.band(leg, 1.15)).join(""), "red-600", 1, MAIN_INKED),
    shape(legs.map(([, , [x, y]]) => f.ellipse(x + 0.45, y + 0.25, 0.95, 0.38)).join(""), "slate-900", 1),
    shape(cloth, "red-600", 1, MAIN_INKED),
    shape(`M${f.p(-8, -5.2)}Q${f.p(-2.4, -6.6)} ${f.p(3, -6.4)}Q${f.p(-2.4, -5.4)} ${f.p(-8, -5.2)}Z`, "slate-900", 0.22),
    shape(mane + beard + hem + cuffs, "white", 0.95),
    shape(head, "red-600", 1, MAIN_INKED),
    shape(`M${hp(9, -8.3)}L${hp(6.6, -7.9)}L${hp(8.7, -6.8)}Z`, "slate-900", 1),
    shape([scale(-1.5, -10.6 + hump * 0.8), scale(-3.6, -9.8 + hump * 0.8), scale(-5.7, -8.4 + hump * 0.5), scale(0.7, -10.8 + hump * 0.6)].join("") + brow(6.2) + brow(8.4), "brass-500", 0.9),
    shape(eye(6.3, -10.7, 1) + eye(8.4, -10.8, 0.9) + teeth, "white", 1),
    shape(eye(6.6, -10.6, 0.48) + eye(8.65, -10.7, 0.44), "slate-900", 1),
    shape(f.poly([H(6.6, -12.85), H(7.6, -14.1), H(7.8, -12.8)]), "brass-500", 1, INKED),
  ];
}

/** A small distant person in a few brush strokes standing on the anchor, facing the way it faces, by the stream standing, walking or seated (about 8 tall, sitting lower): a robe (main, slate-700), a head dot, arms in its sleeves; about 4 wide and 11 tall at scale 1. */
function inkPerson(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pose = rng();
  const bob = between(rng, -0.25, 0.25);
  if (pose < 0.32) {
    const lean = between(rng, -0.03, 0.03);
    const P = (x: number, y: number): string => f.p(...leant([x, y], lean));
    const robe =
      `M${P(-0.9, -8.6)}Q${P(0.2, -9.1)} ${P(1, -8.6)}Q${P(1.6, -8)} ${P(1.6, -6.8)}L${P(2.1, -5)}Q${P(2.1, -4.2)} ${P(1.4, -4.1)}` +
      `L${P(1.5, -2)}Q${P(1.7, -0.6)} ${P(2, 0)}L${P(-1.9, 0)}Q${P(-1.5, -3)} ${P(-1.6, -5)}Q${P(-1.7, -7.6)} ${P(-0.9, -8.6)}Z`;
    return [
      shape(robe, "slate-700", 1, MAIN_INKED),
      shape(f.band([leant([0.2, -7.6], lean), leant([0.9, -5.6], lean), leant([1.9, -4.7], lean)], 0.28) + f.band([leant([0.4, -3.6], lean), leant([0.6, -0.6], lean)], 0.2), "slate-900", 0.6),
      shape(f.band([leant([-1.2, -7.6], lean), leant([-1.3, -3.4], lean)], 0.4), "white", 0.22),
      shape(f.circle(...leant([0.25 + bob * 0.3, -9.75], lean), 0.9), "slate-900", 1),
      shape(f.ellipse(1.6, -0.15, 0.65, 0.25), "slate-900", 1),
    ];
  }
  if (pose < 0.7) {
    const lean = between(rng, 0.06, 0.11);
    const step = between(rng, -0.25, 0.25);
    const P = (x: number, y: number): string => f.p(...leant([x, y], lean));
    const robe =
      `M${P(-0.9, -8.6)}Q${P(0.2, -9.1)} ${P(1, -8.6)}Q${P(1.6, -8)} ${P(1.7, -7)}Q${P(2.5, -5.6)} ${P(2.2, -4.6)}Q${P(1.7, -4.4)} ${P(1.4, -4.6)}` +
      `L${P(1.6, -1.2)}L${P(-0.2, -0.8)}L${P(-2.5 - step, -0.2)}Q${P(-1.8, -2.4)} ${P(-1.8, -3.8)}Q${P(-2.6, -5)} ${P(-2.2, -5.6)}Q${P(-1.6, -7.4)} ${P(-0.9, -8.6)}Z`;
    return [
      shape(f.band([[0.7, -1.4], [1.6 + step, -0.7], [2.4 + step, -0.15]], 0.5) + f.band([[-0.6, -0.6], [-1.1, -0.1]], 0.4), "slate-900", 1),
      shape(robe, "slate-700", 1, MAIN_INKED),
      shape(f.band([leant([0.3, -7.6], lean), leant([1.3, -6], lean), leant([1.9, -4.8], lean)], 0.28) + f.band([leant([-0.8, -7.2], lean), leant([-1.6, -5.4], lean)], 0.22), "slate-900", 0.6),
      shape(f.band([leant([-0.8, -3.8], lean), leant([-1.4, -1.2], lean)], 0.4), "white", 0.22),
      shape(f.circle(...leant([0.35 + bob * 0.3, -9.75], lean), 0.9), "slate-900", 1),
    ];
  }
  const robe =
    `M${f.p(-0.8, -5.9)}Q${f.p(0.2, -6.4)} ${f.p(0.9, -5.9)}Q${f.p(1.5, -5.2)} ${f.p(1.5, -4.2)}L${f.p(1.7, -2.6)}Q${f.p(2.5, -2.5)} ${f.p(2.8, -1.4)}` +
    `Q${f.p(3.1, -0.2)} ${f.p(2.5, 0)}L${f.p(-2, 0)}Q${f.p(-2.5, -0.9)} ${f.p(-1.8, -1.8)}Q${f.p(-1.4, -3.6)} ${f.p(-1.6, -4.6)}Q${f.p(-1.6, -5.5)} ${f.p(-0.8, -5.9)}Z`;
  return [
    shape(robe, "slate-700", 1, MAIN_INKED),
    shape(f.band([[0.2, -5.2], [0.9, -3.6], [1.8, -2.7]], 0.28) + f.band([[-1.2, -1.3], [0.8, -1.5], [2.4, -1.2]], 0.2), "slate-900", 0.6),
    shape(f.band([[-1.2, -4.8], [-1.1, -2.2]], 0.4), "white", 0.22),
    shape(f.circle(0.15 + bob * 0.3, -7.05, 0.9), "slate-900", 1),
  ];
}

/** The sleeve a near hand reaches out of: a cuff from (x1, y1) to (x2, y2) across its wrist, running `reach` back along the arm, flaring `flare`; its pale rim first, then the sleeve. */
function sleeve(f: Local, a: Point, b: Point, reach: number, flare: number): { rim: string; cloth: string } {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const length = Math.hypot(dx, dy) || 1;
  const ex = dx / length;
  const ey = dy / length;
  const back: Point = [dy / length, -dx / length];
  const far = (p: Point, s: number): Point => [p[0] + back[0] * reach + ex * flare * s, p[1] + back[1] * reach + ey * flare * s];
  const inset = (p: Point, t: number): Point => [p[0] + back[0] * t, p[1] + back[1] * t];
  return {
    rim: f.poly([a, b, inset(b, 0.55), inset(a, 0.55)]),
    cloth: f.poly([inset(a, 0.55), inset(b, 0.55), far(b, 1), far(a, -1)]),
  };
}

/** A near hand palm up, open and a little curled, its palm's centre on the anchor, its fingers the way it faces and its wrist reaching in from below behind (a skin main, red-300; its sleeve stays slate-700); about 12 wide and 8 tall at scale 1. */
function handOpen(at: PartPlace, rng: Rng): KitShape[] {
  const tilt = between(rng, -6, 6);
  const curl = between(rng, 0.2, 0.55);
  const f = local(at);
  const R = (x: number, y: number): Point => turned([x, y], tilt);
  const wristTop = R(-3.5, -0.3);
  const wristBottom = R(-2.3, 2.6);
  const outline =
    `M${f.p(...wristTop)}Q${f.p(...R(-2.6, -1.3))} ${f.p(...R(-1.3, -1.8))}` +
    digit(f, R(-1.3, -1.8), R(0.1, -1.7), R(0.1, -4), 0.35, 0.8) +
    `Q${f.p(...R(0.8, -1.6))} ${f.p(...R(1.4, -1.9))}` +
    digit(f, R(1.4, -1.9), R(1.85, -0.85), R(5.2, -2.7), curl, 0.8) +
    digit(f, R(1.85, -0.85), R(2.05, 0.15), R(5.9, -1), curl, 0.82) +
    digit(f, R(2.05, 0.15), R(1.85, 1.1), R(5.6, 0.6), curl, 0.82) +
    digit(f, R(1.85, 1.1), R(1.35, 1.95), R(4.4, 2), curl * 0.8, 0.8) +
    `Q${f.p(...R(-0.4, 3))} ${f.p(...wristBottom)}Z`;
  const cuff = sleeve(f, R(-3.35, -0.75), R(-2, 3.1), 2.8, 0.45);
  const crease = (x: number, y: number): string => f.band([R(x - 0.05, y - 0.4), R(x + 0.05, y + 0.4)], 0.16);
  return [
    shape(outline, "red-300", 1, MAIN_INKED),
    shape(f.ellipse(...R(0.3, 0.5), 1.9, 1.25), "red-900", 0.14),
    shape(f.ellipse(...R(-1.3, -0.6), 0.95, 0.6) + f.band([R(2.6, -1.9), R(4.4, -2.3)], 0.3), "white", 0.32),
    shape(
      f.band([R(0.9, -1.4), R(-0.6, -0.5), R(-1.5, 1.6)], 0.18) + f.band([R(1.6, -0.6), R(0.4, 0), R(-0.4, -0.2)], 0.16) + crease(3.9, -1.2) + crease(4.1, 0.2) + crease(3.5, 1.5),
      "slate-900",
      0.45,
    ),
    shape(cuff.cloth, "slate-700", 1, INKED),
    shape(cuff.rim, "slate-100", 1, INKED),
  ];
}

/** A near hand pinching between thumb and forefinger whatever stands on the anchor just above its fingertips, its wrist rising from below (a skin main, red-300; its sleeve stays slate-700); about 10 wide and 12 tall at scale 1. */
function handHold(at: PartPlace, rng: Rng): KitShape[] {
  const tilt = between(rng, -7, 7);
  const f = local(at);
  const R = (x: number, y: number): Point => turned([x, y], tilt);
  const q = (cx: number, cy: number, x: number, y: number): string => `Q${f.p(...R(cx, cy))} ${f.p(...R(x, y))}`;
  const outline =
    `M${f.p(...R(-2.3, 10))}` +
    q(-3.1, 7.8, -3.1, 6.4) +
    q(-2.9, 4, -2, 2.6) +
    q(-1.4, 1.6, -1.1, 0.9) +
    q(-0.85, 0.15, -0.1, 0.35) +
    `L${f.p(...R(0.4, 0.3))}` +
    q(1.3, -0.1, 2, 0.7) +
    q(2.9, 1.6, 2.8, 2.8) +
    q(2.8, 3.8, 3.2, 4.4) +
    q(4.1, 4.9, 3.6, 5.8) +
    q(4.2, 6.4, 3.5, 7.2) +
    q(3.9, 7.9, 3.1, 8.4) +
    q(2.6, 9, 2.2, 10) +
    "Z";
  const hole = smooth(f, [R(0.3, 1.1), R(1.3, 1.5), R(1.4, 3), R(0.7, 3.8), R(-0.4, 2.9), R(-0.5, 1.6)]);
  const line = (points: readonly Point[], width: number): string => f.band(points.map(([x, y]) => R(x, y)), width);
  const cuff = sleeve(f, R(3.1, 9.6), R(-3.1, 9.6), 2.6, 0.7);
  return [
    shape(outline + hole, "red-300", 1, MAIN_INKED),
    shape(f.ellipse(...R(1.8, 6.6), 1.5, 2), "red-900", 0.13),
    shape(f.ellipse(...R(-1.85, 7), 0.5, 1.6) + f.ellipse(...R(-1.15, 1.2), 0.3, 0.45) + f.ellipse(...R(1.35, 0.45), 0.4, 0.25), "white", 0.4),
    shape(
      line([[-2.3, 2.9], [-1.4, 3.2]], 0.18) + line([[3.5, 5.85], [2.4, 5.7]], 0.18) + line([[3.4, 7.25], [2.4, 7.1]], 0.18) + line([[3.1, 4.45], [2.2, 4.6]], 0.16),
      "slate-900",
      0.5,
    ),
    shape(cuff.cloth, "slate-700", 1, INKED),
    shape(cuff.rim, "slate-100", 1, INKED),
  ];
}

/** A glitching drop of ink scampering on the anchor the way it faces: a teardrop body (main, slate-900) on two tiny legs mid-step, two pale eyes, its colours split and pixel bits flicking off it; about 6 wide and 7 tall at scale 1. */
function inkImp(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tip = between(rng, -2.1, -1.2);
  const step = between(rng, -0.3, 0.3);
  const drop = (dx: number, dy: number): string =>
    `M${f.p(tip + dx, -7 + dy)}C${f.p(0.6 + dx, -5.7 + dy)} ${f.p(2.7 + dx, -4.5 + dy)} ${f.p(2.6 + dx, -2.9 + dy)}C${f.p(2.6 + dx, -1.5 + dy)} ${f.p(1.4 + dx, -1.1 + dy)} ${f.p(0 + dx, -1.1 + dy)}` +
    `C${f.p(-1.4 + dx, -1.1 + dy)} ${f.p(-2.6 + dx, -1.7 + dy)} ${f.p(-2.5 + dx, -3.1 + dy)}C${f.p(-2.4 + dx, -4.5 + dy)} ${f.p(-1.6 + dx, -5.2 + dy)} ${f.p(tip + dx, -7 + dy)}Z`;
  const square = (x: number, y: number, s: number): string => f.poly([[x, y], [x + s, y], [x + s, y + s], [x, y + s]]);
  const cyan: string[] = [];
  const red: string[] = [];
  const spots: Point[] = [[-3.6, -4.6], [-4.4, -3], [-3.3, -6.3], [-4.9, -5.6], [-3.9, -1.8]];
  const bits = whole(rng, 3, 5);
  spots.slice(0, bits).forEach(([x, y], k) => {
    (k % 2 === 0 ? cyan : red).push(square(x + between(rng, -0.25, 0.25), y + between(rng, -0.2, 0.2), between(rng, 0.45, 0.8)));
  });
  const slice = between(rng, -4.2, -2.6);
  return [
    shape(f.ellipse(0.2, 0, 2.6, 0.4), "slate-900", 0.2),
    shape(drop(-0.6, 0), "cyan-400", 0.75),
    shape(drop(0.6, 0.1), "red-600", 0.7),
    shape(
      f.band([[0.6, -1.4], [1.3 + step, -0.7], [2 + step, -0.05]], 0.45) + f.band([[-0.6, -1.4], [-1.2 - step, -0.9], [-1.9 - step, -0.6]], 0.45) + f.poly([[-3.7, slice], [-2.1, slice], [-2.1, slice + 0.45], [-3.7, slice + 0.45]]),
      "slate-900",
      1,
    ),
    shape(drop(0, 0), "slate-900", 1, MAIN),
    shape(`M${f.p(-1.8, -3.4)}Q${f.p(-1.6, -4.6)} ${f.p(-0.6, -5.2)}Q${f.p(-1.2, -4.2)} ${f.p(-1.8, -3.4)}Z`, "white", 0.35),
    shape(f.ellipse(0.7, -3.5, 0.36, 0.44) + f.ellipse(1.75, -3.45, 0.33, 0.42), "slate-100", 1),
    shape(cyan.join(""), "cyan-400", 1),
    shape(red.join("") || square(2.9, -6, 0.5), "red-600", 1),
  ];
}

export const INK_CREATURES_KIT = {
  crane: { category: "creatures", draw: crane },
  koi: { category: "creatures", draw: koi },
  beetle: { category: "creatures", draw: beetle },
  "ink-sheep": { category: "creatures", draw: sheep },
  "lion-dancer": { category: "creatures", draw: lionDancer },
  "ink-person": { category: "creatures", draw: inkPerson },
  "hand-open": { category: "creatures", draw: handOpen },
  "hand-hold": { category: "creatures", draw: handHold },
  "ink-imp": { category: "creatures", draw: inkImp },
} as const satisfies Readonly<Record<string, KitPart>>;
