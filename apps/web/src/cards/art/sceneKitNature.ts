// R3390000: the scene kit's living things, food, and weather and light (ART.md, "Scene recipes and kits"),
// which the Classic+ scenes (#3390) added: plants (a fern, flowers, a fruit tree, a toadstool the size of a
// shrine), creatures (a rattlesnake, a book worm, a boar-like beast, a cat, a monkey, a bee, a ghost, a
// bird), food (an apple, a pear, grapes, a papaya, a basket of fruit, a stack of pancakes) and weather and
// light (a shockwave, a tornado, flames, a fireball, a ribbon of light, a thought bubble, spores, snow,
// static, bubbles, smoke, rain, dust, a sun, falling petals). R3396000: the Classic+ Alt art scenes (#3396)
// added a clump of bamboo, a cactus and a fox. R3397000: the Meditative+ and Training Grounds Alt art scenes
// (#3397) added a mountain goat, a deer, a paddling duck and a heap of walnuts, and R3398000's Core Alt art scenes
// (#3398) a palm and a camel. sceneKit.ts spreads the table into `SCENE_KIT`. Each part keeps the kit's rules: one
// to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream alone, every colour a
// palette name out of Radiant's gold window (food's golds are brass, butter is slate), a glow a stack of
// translucent shapes, and no two sub-paths of one shape overlapping but for a hole.

import { fmt } from "./hash.ts";
import { between, blob, DEGREES, FULL_TURN, INKED, local, MAIN, MAIN_INKED, shape, sparkle, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** A point `degrees` round an ellipse about (cx, cy), clockwise on screen from its right. */
function onEllipse(cx: number, cy: number, rx: number, ry: number, degrees: number): Point {
  return [cx + rx * Math.cos(degrees * DEGREES), cy + ry * Math.sin(degrees * DEGREES)];
}

/** A lobed outline round an ellipse (a cloud, a crown): `lobes` bulges, each curve's control `bulge` times the radius out. */
function lobed(f: Local, rng: Rng, cx: number, cy: number, rx: number, ry: number, lobes: number, bulge: number): string {
  const start = rng() * (FULL_TURN / lobes);
  const at = (i: number, reach: number): Point => onEllipse(cx, cy, rx * reach, ry * reach, start + (i * FULL_TURN) / lobes);
  let d = `M${f.p(...at(0, 1))}`;
  for (let i = 0; i < lobes; i += 1) d += `Q${f.p(...at(i + 0.5, bulge * between(rng, 0.94, 1.06)))} ${f.p(...at(i + 1, 1))}`;
  return `${d}Z`;
}

/** A flower's head of `petals` round petals about (cx, cy), `r` their reach. */
function petalRing(f: Local, cx: number, cy: number, r: number, petals: number, start: number): string {
  const at = (i: number, reach: number): Point => onEllipse(cx, cy, r * reach, r * reach, start + (i * FULL_TURN) / petals);
  let d = `M${f.p(...at(0, 0.3))}`;
  for (let i = 0; i < petals; i += 1) d += `Q${f.p(...at(i + 0.5, 1.8))} ${f.p(...at(i + 1, 0.3))}`;
  return `${d}Z`;
}

/** A band along the points whose width changes, `widths[i]` wide at `points[i]`. */
function taper(f: Local, points: readonly Point[], widths: readonly number[]): string {
  const left: Point[] = [];
  const right: Point[] = [];
  points.forEach(([x, y], i) => {
    const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
    const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [x, y];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (widths[i] ?? 0) / 2;
    const ox = (-(ny - py) / length) * half;
    const oy = ((nx - px) / length) * half;
    left.push([x + ox, y + oy]);
    right.push([x - ox, y - oy]);
  });
  return f.poly([...left, ...right.reverse()]);
}

type Dot = readonly [number, number, number];

/** Up to `count` dots over the ellipse `w` by `h` about (cx, cy), radii from `low` to `high`, none within `gap` of another. */
function scatter(rng: Rng, count: number, cx: number, cy: number, w: number, h: number, low: number, high: number, gap: number): Dot[] {
  const dots: Dot[] = [];
  for (let tries = 0; dots.length < count && tries < count * 8; tries += 1) {
    const angle = rng() * FULL_TURN * DEGREES;
    const reach = Math.sqrt(rng());
    const dot: Dot = [cx + w * reach * Math.cos(angle), cy + h * reach * Math.sin(angle), between(rng, low, high)];
    if (dots.every(([x, y, r]) => Math.hypot(x - dot[0], y - dot[1]) > r + dot[2] + gap)) dots.push(dot);
  }
  return dots;
}

/** The lower rim of an ellipse as a crescent, `inner` of its depth hollowed at the middle: a coil's or a lip's shade. */
function underside(f: Local, cx: number, cy: number, rx: number, ry: number, inner: number): string {
  const r = fmt(f.r(rx));
  return `M${f.p(cx - rx, cy)}A${r} ${fmt(f.r(ry))} 0 0 ${f.sweep(0)} ${f.p(cx + rx, cy)}A${r} ${fmt(f.r(ry * inner))} 0 0 ${f.sweep(1)} ${f.p(cx - rx, cy)}Z`;
}

/** An apple's outline about (cx, cy), `r` its half-width, turned `tilt` degrees. */
function applePath(f: Local, cx: number, cy: number, r: number, tilt: number): string {
  const P = (dx: number, dy: number): string => {
    const [x, y] = turned([dx * r, dy * r], tilt);
    return f.p(cx + x, cy + y);
  };
  return (
    `M${P(0, -0.72)}C${P(0.5, -1.12)} ${P(1.05, -0.88)} ${P(1.02, -0.1)}C${P(1, 0.62)} ${P(0.55, 0.98)} ${P(0.22, 0.96)}` +
    `Q${P(0, 0.88)} ${P(-0.22, 0.96)}C${P(-0.55, 0.98)} ${P(-1, 0.62)} ${P(-1.02, -0.1)}C${P(-1.05, -0.88)} ${P(-0.5, -1.12)} ${P(0, -0.72)}Z`
  );
}

/** A pear's outline, its bulb about (cx, cy), `r` its scale (the bulb about 1.7 r wide, the pear 2.5 r tall), turned `tilt` degrees. */
function pearPath(f: Local, cx: number, cy: number, r: number, tilt: number): string {
  const P = (dx: number, dy: number): string => {
    const [x, y] = turned([dx * r, dy * r], tilt);
    return f.p(cx + x, cy + y);
  };
  return (
    `M${P(0, -1.55)}C${P(0.4, -1.55)} ${P(0.4, -0.95)} ${P(0.62, -0.6)}C${P(1.18, -0.2)} ${P(1.12, 0.96)} ${P(0, 0.98)}` +
    `C${P(-1.12, 0.96)} ${P(-1.18, -0.2)} ${P(-0.62, -0.6)}C${P(-0.4, -0.95)} ${P(-0.4, -1.55)} ${P(0, -1.55)}Z`
  );
}

/** A pod's outline about (cx, cy): `a` its half-length, `b` its half-height at its fat end (the right), `narrow` its thin end's share of it. */
function podPath(f: Local, cx: number, cy: number, a: number, b: number, narrow: number): string {
  const n = b * narrow;
  return (
    `M${f.p(cx - a, cy)}C${f.p(cx - a, cy - n * 0.8)} ${f.p(cx - a * 0.4, cy - b)} ${f.p(cx + a * 0.2, cy - b)}` +
    `C${f.p(cx + a * 0.78, cy - b)} ${f.p(cx + a, cy - b * 0.55)} ${f.p(cx + a, cy)}` +
    `C${f.p(cx + a, cy + b * 0.55)} ${f.p(cx + a * 0.78, cy + b)} ${f.p(cx + a * 0.2, cy + b)}` +
    `C${f.p(cx - a * 0.4, cy + b)} ${f.p(cx - a, cy + n * 0.8)} ${f.p(cx - a, cy)}Z`
  );
}

/** Tongues of flame from a base `half` wide each side of the anchor up to each tip and down to each valley, back along the base. */
function tongues(f: Local, tips: readonly Point[], valleys: readonly Point[], half: number): string {
  let d = `M${f.p(-half, 0)}`;
  let from: Point = [-half, 0];
  tips.forEach((tip, i) => {
    const to: Point = valleys[i] ?? [half, 0];
    d += `Q${f.p(from[0] + (tip[0] - from[0]) * 0.2, (from[1] + tip[1]) * 0.5)} ${f.p(...tip)}`;
    d += `Q${f.p(to[0] - (to[0] - tip[0]) * 0.2, (to[1] + tip[1]) * 0.5)} ${f.p(...to)}`;
    from = to;
  });
  return `${d}Q${f.p(0, 0.35)} ${f.p(-half, 0)}Z`;
}

// Plants

/** A fern of five to seven arching fronds standing on the anchor, about 11 wide and 7.5 tall at scale 1. */
function fern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 5, 7);
  const fronds: { readonly d: string; readonly t: number }[] = [];
  const ribs: string[] = [];
  for (let k = 0; k < count; k += 1) {
    const t = (2 * k) / (count - 1) - 1;
    const angle = t * 66 + between(rng, -5, 5);
    const curl = Math.abs(angle) > 4 ? Math.sign(angle) : rng() < 0.5 ? -1 : 1;
    const length = (7.6 - Math.abs(t) * 2.2) * between(rng, 0.9, 1.04);
    const droop = Math.min(90, Math.abs(angle) * 1.3 + between(rng, 8, 14)) * curl;
    const base: Point = [t * 0.5, 0];
    const ctrl: Point = [base[0] + length * 0.85 * Math.sin(angle * 0.72 * DEGREES), -length * 0.85 * Math.cos(angle * 0.72 * DEGREES)];
    const tip: Point = [base[0] + length * Math.sin(droop * DEGREES), -length * Math.cos(droop * DEGREES)];
    const on = (s: number): Point => [
      (1 - s) ** 2 * base[0] + 2 * s * (1 - s) * ctrl[0] + s ** 2 * tip[0],
      (1 - s) ** 2 * base[1] + 2 * s * (1 - s) * ctrl[1] + s ** 2 * tip[1],
    ];
    const width = (1.3 * length) / 7;
    const left: Point[] = [];
    const right: Point[] = [];
    for (let i = 0; i < 6; i += 1) {
      const s = i / 6;
      const [x, y] = on(s);
      const dx = 2 * (1 - s) * (ctrl[0] - base[0]) + 2 * s * (tip[0] - ctrl[0]);
      const dy = 2 * (1 - s) * (ctrl[1] - base[1]) + 2 * s * (tip[1] - ctrl[1]);
      const n = Math.hypot(dx, dy) || 1;
      const [ux, uy] = [dx / n, dy / n];
      const half = i === 0 ? 0.22 : i % 2 === 1 ? width * (1 - s * 0.75) : width * 0.3 * (1 - s);
      const ahead = i % 2 === 1 ? 0.45 * width : 0;
      left.push([x - uy * half + ux * ahead, y + ux * half + uy * ahead]);
      right.push([x + uy * half + ux * ahead, y - ux * half + uy * ahead]);
    }
    fronds.push({ d: f.poly([...left, tip, ...right.reverse()]), t });
    ribs.push(f.band([on(0.3), on(0.6), on(0.88)], 0.2));
  }
  const order = [...fronds].sort((a, b) => Math.abs(b.t) - Math.abs(a.t));
  const back = order.slice(0, 2);
  return [
    ...back.map((frond) => shape(frond.d, "green-700", 1, MAIN)),
    shape(back.map((frond) => frond.d).join(""), "black", 0.25),
    ...order.slice(2).map((frond) => shape(frond.d, "green-700", 1, MAIN)),
    shape(ribs.join(""), "green-500", 0.55),
  ];
}

/** A clump of three to five flowers on stems with leaves, standing on the anchor, about 6 wide and 6 tall at scale 1. */
function flowers(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const stems: string[] = [];
  const centres: string[] = [];
  const heads: KitShape[] = [];
  for (let k = 0; k < count; k += 1) {
    const t = (2 * k) / (count - 1) - 1;
    const root = t * 0.5;
    const top: Point = [t * 2.1 + between(rng, -0.25, 0.25), -(5.2 - Math.abs(t) * 1.4) * between(rng, 0.88, 1.04)];
    stems.push(f.band([[root, 0], [(root + top[0]) / 2 + between(rng, -0.3, 0.3), top[1] * 0.5], top], 0.26));
    const r = between(rng, 0.78, 1);
    heads.push(shape(petalRing(f, top[0], top[1], r, 5, rng() * 72), "red-300", 1, MAIN));
    centres.push(f.circle(top[0], top[1], r * 0.3));
  }
  const leaf = (side: number): string => {
    const length = between(rng, 2.2, 2.8);
    const lift = between(rng, 0.9, 1.7);
    return `M${f.p(side * 0.45, -0.1)}Q${f.p(side * length * 0.5, -lift - 1.1)} ${f.p(side * length, -lift)}Q${f.p(side * length * 0.6, -lift + 0.4)} ${f.p(side * 0.45, -0.1)}Z`;
  };
  return [shape(stems.join(""), "green-700", 1), shape(leaf(-1) + leaf(1), "green-500", 1), ...heads, shape(centres.join(""), "white", 0.95)];
}

/** An old fruit tree on the anchor: a gnarled trunk, a broad crown and its apples and pears, a windfall at its foot; about 26 wide and 30 tall at scale 1. */
function fruitTree(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.06, 0.06);
  const j = (n: number): number => between(rng, -n, n);
  const trunk: Point[] = [
    [-4.8, 0],
    [-2.6, -1],
    [-2, -3.5],
    [-2.5 + j(0.3), -6.5],
    [-1.8, -9.5],
    [-3.6, -12],
    [-6.6 + j(0.5), -14.6],
    [-5.6, -15.6],
    [-2.2, -13.2],
    [-0.6, -12.6],
    [0.2 + j(0.5), -16.6],
    [1.4, -16.4],
    [1.2, -12.8],
    [3.6, -14.6],
    [6.4 + j(0.5), -15.8],
    [6.9, -14.8],
    [2.6, -11.4],
    [1.8, -8.8],
    [2.4 + j(0.3), -5.5],
    [2, -2.6],
    [2.8, -1],
    [5, 0],
  ];
  const P = (points: readonly Point[]): Point[] => points.map(([dx, dy]) => [dx - lean * dy, dy]);
  const cy = -20.5;
  const fruit = scatter(rng, whole(rng, 7, 10), 0, cy + 2, 9.5, 5.2, 0.85, 1.1, 0.5);
  const pears: Dot[] = [];
  for (const dot of fruit) if (pears.length < 2 && pears.every(([x, y]) => Math.hypot(x - dot[0], y - dot[1]) > 4)) pears.push(dot);
  const apples = fruit.filter((dot) => !pears.includes(dot));
  const side = rng() < 0.5 ? -1 : 1;
  const fallen: Dot = [side * between(rng, 6.5, 9), -0.9, 0.9];
  const shine = [...fruit, fallen].map(([x, y, r]) => f.circle(x - r * 0.35, y - r * 0.4, r * 0.26)).join("");
  return [
    shape(f.poly(P(trunk)), "wood-700", 1, INKED),
    shape(f.band(P([[2, -1.4], [1.3, -5.4], [1, -9], [2.2, -12]]), 1.2), "slate-900", 0.35),
    shape(f.band(P([[-0.9, -2.2], [-0.5, -6], [-1, -10]]), 0.35) + f.ellipse(...P([[0.7, -4.6]])[0]!, 0.35, 0.55), "wood-900", 0.6),
    shape(lobed(f, rng, 0, cy, 11.2, 8, 9, 1.32), "green-700", 1, MAIN_INKED),
    shape(lobed(f, rng, 3, cy + 2.6, 7, 4.6, 6, 1.25), "black", 0.18),
    shape(f.circle(-4.6, cy - 3.6, 2.6) + f.circle(-0.4, cy - 5.6, 1.8), "green-500", 0.4),
    shape([...apples.map(([x, y, r]) => f.circle(x, y, r)), f.circle(fallen[0], fallen[1], fallen[2])].join(""), "red-600", 1, INKED),
    shape(pears.map(([x, y, r]) => pearPath(f, x, y + 0.2, r * 0.85, j(15))).join(""), "lime-400", 1, INKED),
    shape(shine, "white", 0.65),
  ];
}

/** A charcoal toadstool the size of a shrine on the anchor, its cap split open at the top by a crack of violet glow; about 27 wide and 26 tall at scale 1. */
function giantMushroom(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 13.5 * between(rng, 0.94, 1.04);
  const top = -26 * between(rng, 0.96, 1.02);
  const rim = -14;
  const lens = (x: number): number => rim + 10 * ((x + w) / (2 * w)) * (1 - (x + w) / (2 * w));
  const gills = [-0.82, -0.6, -0.38, -0.15, 0.15, 0.38, 0.6, 0.82].map((u) => f.band([[u * w * 0.34, rim + 0.6], [u * w * 0.96, lens(u * w * 0.96) - 0.35]], 0.35));
  const crack: Point[] = [
    [-1.6, top + 0.1],
    [-0.6, top + 2.6],
    [-0.1, top + 5],
    [0.6 + between(rng, -0.3, 0.3), top + 6.4],
    [0.3, top + 8.4],
    [0.9, top + 6.6],
    [0.4, top + 4.8],
    [0.6, top + 2.8],
    [1.4, top + 0.2],
  ];
  const spots: Point[] = [
    [-8.5, -18.5],
    [5, -21.8],
    [9, -17.6],
    [-4.6, -22.2],
    [-10.6, -15.6],
    [3.6, -17],
    [-4.4, -17.2],
  ];
  const spotPaths = spots.filter((_, k) => k < 3 || rng() < 0.6).map(([x, y], k) => f.ellipse(x * (w / 13.5), y, 1.5 - k * 0.1, 1.0 - k * 0.06));
  return [
    shape(`M${f.p(-w, rim)}Q${f.p(0, rim + 5)} ${f.p(w, rim)}Q${f.p(0, rim - 2)} ${f.p(-w, rim)}Z`, "slate-900", 1),
    shape(gills.join(""), "red-300", 0.4),
    shape(`M${f.p(-4.6, 0)}Q${f.p(-3, -0.6)} ${f.p(-3.2, -3)}L${f.p(-2.7, rim + 1.6)}L${f.p(2.7, rim + 1.6)}L${f.p(3.2, -3)}Q${f.p(3, -0.6)} ${f.p(4.6, 0)}Z`, "slate-400", 1, INKED),
    shape(f.poly([[1.2, -0.3], [1.5, rim + 1.6], [2.7, rim + 1.6], [3.2, -3], [3.7, -0.3]]), "slate-900", 0.3),
    shape(
      `M${f.p(-3, -10.2)}L${f.p(3, -10.2)}Q${f.p(4.2, -9.2)} ${f.p(4, -7.8)}Q${f.p(2, -8.6)} ${f.p(0, -7.9)}Q${f.p(-2, -8.6)} ${f.p(-4, -7.8)}Q${f.p(-4.2, -9.2)} ${f.p(-3, -10.2)}Z`,
      "slate-100",
      1,
      INKED,
    ),
    shape(
      `M${f.p(-w, rim)}C${f.p(-w * 1.02, top + 5)} ${f.p(-w * 0.6, top + 0.4)} ${f.p(-1.6, top + 0.1)}L${f.p(-0.2, top + 3.4)}L${f.p(1.4, top + 0.2)}` +
        `C${f.p(w * 0.6, top + 0.4)} ${f.p(w * 1.02, top + 5)} ${f.p(w, rim)}Q${f.p(0, rim - 1.6)} ${f.p(-w, rim)}Z`,
      "slate-700",
      1,
      MAIN_INKED,
    ),
    shape(`M${f.p(-w + 1, rim - 1)}C${f.p(-w + 0.8, top + 6)} ${f.p(-w * 0.58, top + 1.6)} ${f.p(-2.4, top + 1)}C${f.p(-w * 0.52, top + 2.6)} ${f.p(-w + 2.8, top + 6)} ${f.p(-w + 1, rim - 1)}Z`, "slate-400", 0.3),
    shape(`M${f.p(1.4, top + 0.2)}C${f.p(w * 0.6, top + 0.4)} ${f.p(w * 1.02, top + 5)} ${f.p(w, rim)}Q${f.p(w * 0.7, rim - 0.8)} ${f.p(w * 0.45, rim - 1)}C${f.p(w * 0.7, top + 6)} ${f.p(w * 0.45, top + 1.6)} ${f.p(1.4, top + 0.2)}Z`, "black", 0.25),
    shape(spotPaths.join(""), "slate-100", 0.55),
    shape(f.ellipse(0, top + 0.6, 4.6, 3.2), "violet-300", 0.2),
    shape(f.poly(crack), "violet-300", 0.95),
    shape(f.poly([[-0.8, top + 0.6], [-0.1, top + 3], [0.4, top + 5.6], [0.3, top + 3], [0.6, top + 0.6]]), "white", 0.85),
  ];
}

/** R3396000: a clump of three to five jointed bamboo stalks standing on the anchor, swaying together, each with a spray of leaves near its top; about 10 wide and 34 tall at scale 1. */
function bamboo(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const sway = between(rng, -1.2, 1.2);
  let stalks = "";
  let joints = "";
  let shine = "";
  const sprays: KitShape[] = [];
  for (let k = 0; k < count; k += 1) {
    const x = -4.5 + (9 * k) / (count - 1) + between(rng, -0.2, 0.2);
    const height = between(rng, 26, 34);
    const lean = sway + between(rng, -0.2, 0.2);
    const on = (t: number): Point => [x + lean * t, -height * t];
    stalks += f.band([on(0), on(1)], 0.9);
    shine += f.band([[x - 0.22, -0.6], [x + lean - 0.22, -height + 0.6]], 0.2);
    const nodes = whole(rng, 5, 6);
    for (let j = 1; j < nodes; j += 1) {
      const [nx, ny] = on(j / nodes);
      joints += f.band([[nx - 0.62, ny], [nx + 0.62, ny]], 0.3);
    }
    const [sx, sy] = on(between(rng, 0.72, 0.9));
    const side = k % 2 === 0 ? -1 : 1;
    let leaves = "";
    for (const angle of [-32, 6, 44]) {
      const length = between(rng, 5.6, 7.2);
      const u: Point = [side * Math.cos(angle * DEGREES), Math.sin(angle * DEGREES)];
      const n: Point = [-u[1] * 1.05, u[0] * 1.05];
      const base: Point = [sx + u[0] * 0.6, sy + u[1] * 0.6];
      const mid: Point = [sx + u[0] * length * 0.5, sy + u[1] * length * 0.5];
      const tip: Point = [sx + u[0] * length, sy + u[1] * length];
      leaves += `M${f.p(...base)}Q${f.p(mid[0] + n[0], mid[1] + n[1])} ${f.p(...tip)}Q${f.p(mid[0] - n[0], mid[1] - n[1])} ${f.p(...base)}Z`;
    }
    sprays.push(shape(leaves, k % 2 === 0 ? "green-700" : "green-400", 1));
  }
  return [shape(stalks, "green-500", 1, MAIN_INKED), shape(shine, "lime-200", 0.35), shape(joints, "green-900", 0.7), ...sprays];
}

/** R3396000: a ribbed cactus standing on the anchor: a round-topped trunk (main) with one to three arms (main), its ribs, spines, a shine and often a flower; about 9 wide and 18 tall at scale 1. */
function cactus(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = between(rng, -18.6, -16.8);
  const arm = (side: number, y: number, reach: number): string => {
    const s = (dx: number, dy: number): string => f.p(side * dx, dy);
    return (
      `M${s(2, y + 1.2)}L${s(reach - 1.2, y + 1.2)}Q${s(reach, y + 1.2)} ${s(reach, y)}L${s(reach, y - 4)}` +
      `Q${s(reach, y - 5.4)} ${s(reach - 1.1, y - 5.4)}Q${s(reach - 2.2, y - 5.4)} ${s(reach - 2.2, y - 4)}L${s(reach - 2.2, y - 1)}L${s(2, y - 1)}Z`
    );
  };
  const arms = whole(rng, 1, 3);
  const out = [shape(f.ellipse(0, 0.3, 4.2, 0.6), "black", 0.2)];
  out.push(shape(arm(-1, between(rng, -11, -7), between(rng, 4.8, 5.8)), "green-700", 1, MAIN_INKED));
  if (arms >= 2) out.push(shape(arm(1, between(rng, -13, -9), between(rng, 4.6, 5.6)), "green-700", 1, MAIN_INKED));
  if (arms >= 3) out.push(shape(arm(1, between(rng, -6, -4.6), between(rng, 3.8, 4.4)), "green-700", 1, MAIN_INKED));
  out.push(
    shape(`M${f.p(-2.2, 0)}L${f.p(-2.2, top + 2.2)}Q${f.p(-2.2, top)} ${f.p(0, top)}Q${f.p(2.2, top)} ${f.p(2.2, top + 2.2)}L${f.p(2.2, 0)}Z`, "green-700", 1, MAIN_INKED),
    shape(f.band([[-1.1, -0.4], [-1.1, top + 1.4]], 0.26) + f.band([[0, -0.4], [0, top + 0.6]], 0.26) + f.band([[1.1, -0.4], [1.1, top + 1.4]], 0.26), "green-900", 0.5),
    shape(f.band([[-1.65, -1], [-1.65, top + 2.4]], 0.32), "lime-200", 0.3),
  );
  let spines = "";
  for (let y = -2; y > top + 2; y -= 2.4) spines += f.band([[-2.2, y], [-2.9, y - 0.5]], 0.14) + f.band([[2.2, y - 1.2], [2.9, y - 1.7]], 0.14);
  out.push(shape(spines, "white", 0.55));
  if (rng() < 0.5) out.push(shape(f.circle(between(rng, -0.6, 0.6), top - 0.3, 0.9), rng() < 0.5 ? "red-400" : "purple-400", 1, INKED));
  return out;
}

// Creatures

/** A coiled rattlesnake on the anchor, its head raised and looking the way it faces, its rattle up behind, blotched with plague sores; about 18 wide and 14 tall at scale 1. */
function snake(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const coils: readonly (readonly [number, number, number, number])[] = [
    [0, -2.2, 7.6, 2.2],
    [0.5, -5.1, 6, 1.9],
    [0.9, -7.6, 4.4, 1.6],
  ];
  const sway = between(rng, -0.5, 0.5);
  const lift = between(rng, -0.6, 0.4);
  const neck: Point[] = [
    [0.4, -8.4],
    [-0.5 + sway, -10.2],
    [0.4 + sway, -11.8 + lift],
    [2.4, -12.4 + lift],
  ];
  const hx = 2.2;
  const hy = -12.6 + lift;
  const tail: Point[] = [
    [-6.4, -3.4],
    [-7.7, -4.9],
    [-8.3, -7],
    [-8 + sway * 0.5, -8.8],
  ];
  const sores: Point[] = [
    [-4.2, -2.6],
    [-0.4, -1.6],
    [3, -2.4],
    [5.8, -1.8],
    [-3, -5.4],
    [1, -4.8],
    [4.4, -5.4],
    [-1.4, -7.9],
    [2.6, -7.8],
    [-0.4 + sway, -10.4],
  ];
  const sorePaths = sores.filter((_, k) => k % 3 === 0 || rng() < 0.65).map(([x, y]) => f.ellipse(x, y, between(rng, 0.5, 0.75), 0.42));
  const tongue: Point[] = [
    [hx + 4.1, hy + 0.85],
    [hx + 5.6, hy + 0.85],
    [hx + 6.5, hy + 0.3],
    [hx + 5.9, hy + 1],
    [hx + 6.5, hy + 1.7],
    [hx + 5.6, hy + 1.15],
    [hx + 4.1, hy + 1.15],
  ];
  return [
    shape(taper(f, tail, [1.4, 1.2, 0.9, 0.6]), "wood-600", 1, MAIN_INKED),
    shape([0, 1, 2].map((k) => f.ellipse(-8 + sway * 0.5 + k * 0.08, -9.5 - k * 0.78, 0.5, 0.34)).join(""), "stone-500", 1, INKED),
    ...coils.map(([x, y, rx, ry]) => shape(f.ellipse(x, y, rx, ry), "wood-600", 1, MAIN_INKED)),
    shape(coils.map(([x, y, rx, ry]) => underside(f, x, y, rx * 0.96, ry * 0.92, 0.45)).join(""), "black", 0.28),
    shape(taper(f, neck, [2, 1.8, 1.6, 1.4]), "wood-600", 1, MAIN_INKED),
    shape(
      `M${f.p(hx - 0.2, hy - 1)}Q${f.p(hx + 1.6, hy - 1.7)} ${f.p(hx + 3.4, hy - 0.8)}Q${f.p(hx + 4.4, hy - 0.2)} ${f.p(hx + 4.2, hy + 0.5)}Q${f.p(hx + 3.2, hy + 1.4)} ${f.p(hx + 1.4, hy + 1.2)}Q${f.p(hx - 0.1, hy + 1)} ${f.p(hx - 0.2, hy - 1)}Z`,
      "wood-600",
      1,
      MAIN_INKED,
    ),
    shape(sorePaths.join(""), "green-400", 0.85),
    shape(f.circle(hx + 2, hy - 0.35, 0.34), "black", 1),
    shape(f.poly(tongue), "red-600", 1),
  ];
}

/** A fat book worm arching up out of a mound on the anchor, spectacles on its face, the way it faces; about 17 long and 9 tall at scale 1. */
function grub(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const arch = between(rng, 0.9, 1.08);
  const spine: Point[] = [
    [-7.6, 0.4],
    [-7, -2.6 * arch],
    [-5.4, -5.4 * arch],
    [-3, -7 * arch],
    [-0.4, -7.2 * arch],
    [1.8, -6.2 * arch],
    [3.4, -5],
  ];
  const widths = [2.8, 3, 3.2, 3.3, 3.2, 3, 2.8];
  const rings: string[] = [];
  for (let i = 1; i < 6; i += 1) {
    const [x, y] = spine[i] ?? [0, 0];
    const [px, py] = spine[i - 1] ?? [0, 0];
    const [nx, ny] = spine[i + 1] ?? [0, 0];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (widths[i] ?? 3) * 0.44;
    const [ox, oy] = [(-(ny - py) / length) * half, ((nx - px) / length) * half];
    rings.push(f.band([[x + ox, y + oy], [x - ox, y - oy]], 0.24));
  }
  const spots = [1.5, 2.5, 3.5, 4.5].filter(() => rng() < 0.75).map((s) => {
    const [x, y] = spine[Math.floor(s)] ?? [0, 0];
    const [nx, ny] = spine[Math.ceil(s)] ?? [0, 0];
    return f.circle((x + nx) / 2 + between(rng, -0.3, 0.3), (y + ny) / 2 - 0.5, between(rng, 0.38, 0.55));
  });
  const hx = 5.2;
  const hy = -5 - between(rng, 0, 0.6);
  const ring = (x: number): string => f.circle(x, hy - 0.6, 0.78) + f.circle(x, hy - 0.6, 0.54);
  return [
    shape(`M${f.p(-10.4, 0.4)}Q${f.p(-7.6, -1.8)} ${f.p(-4.8, 0.4)}Z`, "wood-800", 1),
    shape(taper(f, spine, widths), "red-300", 1, MAIN_INKED),
    shape(rings.join(""), "black", 0.22),
    shape(spots.length > 0 ? spots.join("") : f.circle(-3, -7.2 * arch, 0.45), "green-400", 0.8),
    shape(f.circle(-4.4, -6.6 * arch, 0.9), "white", 0.3),
    shape(f.circle(hx, hy, 2.3), "red-300", 1, MAIN_INKED),
    shape(f.circle(hx - 0.85, hy - 0.6, 0.54) + f.circle(hx + 0.95, hy - 0.6, 0.54), "white", 0.4),
    shape(ring(hx - 0.85) + ring(hx + 0.95) + f.band([[hx - 0.07, hy - 0.7], [hx + 0.17, hy - 0.7]], 0.22) + f.band([[hx - 1.63, hy - 0.6], [hx - 2.25, hy - 0.2]], 0.2), "slate-100", 1),
    shape(f.circle(hx - 0.7, hy - 0.6, 0.24) + f.circle(hx + 1.1, hy - 0.6, 0.24) + f.band([[hx - 0.2, hy + 0.9], [hx + 0.6, hy + 1.15], [hx + 1.3, hy + 0.8]], 0.22), "black", 0.85),
  ];
}

/** A hulking boar-like beast on four legs on the anchor, a ridge of bristles, tusks and a mouth of jagged teeth, facing the way it faces; about 25 long and 15 tall at scale 1. */
function boar(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sx = between(rng, 0.94, 1.06);
  const p = (dx: number, dy: number): string => f.p(dx * sx, dy);
  const B = (points: readonly Point[]): Point[] => points.map(([dx, dy]) => [dx * sx, dy]);
  const gape = between(rng, 0, 0.5);
  const body =
    `M${p(-11, -6.4)}Q${p(-11.6, -10)} ${p(-8, -11.2)}Q${p(-4, -12.8)} ${p(-0.5, -12.6)}Q${p(3, -12.4)} ${p(4.6, -10.8)}L${p(5.2, -12.6)}L${p(6.6, -10.4)}` +
    `Q${p(9, -9.6)} ${p(11, -8)}L${p(12.2, -7.4)}L${p(12.5, -5.6 + gape)}L${p(11.6, -5.2 + gape)}Q${p(10.4, -3.8 + gape)} ${p(8.6, -4.2)}Q${p(6.6, -3.4)} ${p(4.8, -4.2)}` +
    `L${p(-8.4, -4.4)}Q${p(-11, -4.4)} ${p(-11, -6.4)}Z`;
  const back: Point[] = [[-9, -10.8], [-6.5, -11.8], [-4, -12.4], [-1.5, -12.6], [1, -12.5], [3.2, -11.8], [4.5, -10.9]];
  const ridge: Point[] = [];
  back.forEach(([x, y], i) => {
    ridge.push([x, y + 0.2]);
    const next = back[i + 1];
    if (next !== undefined) ridge.push([(x + next[0]) / 2 - 0.6, Math.min(y, next[1]) - between(rng, 1.4, 2.2)]);
  });
  const ridgePath = f.poly(B([...ridge, ...back.map(([x, y]) => [x, y + 0.9] as Point).reverse()]));
  const upper: Point[] = [[8.6, -6.3], [12, -6.4]];
  for (const x of [11.6, 10.7, 9.8, 9]) upper.push([x, -5.7], [x - 0.42, -6.3]);
  const lower: Point[] = [[8.9, -5 + gape], [11.4, -5 + gape]];
  for (const x of [11.15, 10.25, 9.4]) lower.push([x, -5.6 + gape * 0.5], [x - 0.42, -5 + gape]);
  const tusk = between(rng, 2.6, 3.4);
  return [
    shape(f.poly(B([[0.8, -4.6], [2.8, -4.6], [2.6, -0.2], [1.2, -0.2]])) + f.poly(B([[-5.4, -4.6], [-3.4, -4.6], [-3.8, -0.2], [-5, -0.2]])), "wood-800", 1, INKED),
    shape(body, "wood-700", 1, MAIN_INKED),
    shape(f.poly(B([[3.2, -4.8], [5.4, -4.8], [5, 0], [3.4, 0]])) + f.poly(B([[-8.6, -5.2], [-5.6, -4.8], [-6.2, 0], [-7.8, 0]])), "wood-700", 1, MAIN_INKED),
    shape(f.poly(B([[3.4, 0], [5, 0], [5.06, -0.7], [3.36, -0.7]])) + f.poly(B([[-7.8, 0], [-6.2, 0], [-6.15, -0.7], [-7.85, -0.7]])), "slate-900", 0.9),
    shape(`M${p(-10.4, -6)}Q${p(-3, -7.4)} ${p(5, -6.2)}L${p(4.8, -4.3)}L${p(-8.4, -4.5)}Q${p(-10.6, -4.6)} ${p(-10.4, -6)}Z`, "black", 0.28),
    shape(`M${p(-9.4, -9.4)}Q${p(-5, -12.2)} ${p(0, -11.6)}Q${p(-5, -11)} ${p(-9.4, -9.4)}Z`, "white", 0.12),
    shape(ridgePath + f.band(B([[-11, -8.6], [-12.4, -9.2], [-12.6, -10.6], [-11.8, -11]]), 0.5) + f.band(B([[6.9, -9.7], [8.9, -9.4]]), 0.5), "wood-900", 1),
    shape(f.poly(B([[12.4, -6.5], [8.2, -6.3], [8.6, -4.9 + gape], [11.7, -4.9 + gape]])), "red-900", 1),
    shape(f.poly(B(upper)) + f.poly(B(lower)), "white", 0.95),
    shape(`M${p(10.3, -4.9 + gape)}Q${p(12.9, -5.2 + gape)} ${p(12.6, -5 - tusk)}Q${p(12, -5.8)} ${p(10.1, -5.6 + gape * 0.5)}Z`, "slate-100", 1, INKED),
    shape(f.circle(7.9 * sx, -8.7, 0.45), "red-500", 1),
  ];
}

/** A cat sitting upright on the anchor, its tail curled up behind it and its ears up, facing the way it faces; about 7 wide and 8.5 tall at scale 1. */
function cat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.5, 0.5);
  const tilt = between(rng, -0.25, 0.25);
  const hx = 0.9;
  const hy = -6 + tilt * 0.4;
  return [
    shape(f.band([[-1.9, -0.5], [-3.2, -1], [-3.7, -2.6], [-3.2 + curl * 0.3, -3.9], [-2.4 + curl, -3.8]], 0.75), "orange-400", 1, MAIN_INKED),
    shape(`M${f.p(-2.4, 0)}C${f.p(-2.9, -2.4)} ${f.p(-2, -4.6)} ${f.p(-0.2, -5)}L${f.p(1, -4.9)}Q${f.p(1.9, -3.6)} ${f.p(1.7, -1.8)}L${f.p(2, 0)}Z`, "orange-400", 1, MAIN_INKED),
    shape(
      `M${f.p(hx - 1.4, hy - 0.3)}L${f.p(hx - 1.3, hy - 2 + tilt)}L${f.p(hx - 0.4, hy - 1.4)}Q${f.p(hx + 0.1, hy - 1.6)} ${f.p(hx + 0.6, hy - 1.4)}L${f.p(hx + 1.5, hy - 2 - tilt)}` +
        `L${f.p(hx + 1.5, hy - 0.2)}Q${f.p(hx + 1.6, hy + 1.3)} ${f.p(hx, hy + 1.4)}Q${f.p(hx - 1.6, hy + 1.3)} ${f.p(hx - 1.4, hy - 0.3)}Z`,
      "orange-400",
      1,
      MAIN_INKED,
    ),
    shape(f.band([[-1.8, -3.4], [-1, -3.9]], 0.3) + f.band([[-2.3, -2.2], [-1.3, -2.7]], 0.3) + f.band([[-2.4, -1], [-1.5, -1.4]], 0.3) + f.band([[0.9, -0.2], [1, -2.2]], 0.22), "black", 0.22),
    shape(f.ellipse(1.25, -3, 0.5, 1.1) + f.ellipse(hx + 0.4, hy + 0.8, 0.7, 0.4), "white", 0.5),
    shape(f.ellipse(hx - 0.45, hy - 0.1, 0.34, 0.27) + f.ellipse(hx + 0.75, hy - 0.1, 0.34, 0.27), "lime-400", 1),
    shape(f.band([[hx - 0.45, hy - 0.32], [hx - 0.45, hy + 0.12]], 0.12) + f.band([[hx + 0.75, hy - 0.32], [hx + 0.75, hy + 0.12]], 0.12), "black", 0.9),
    shape(f.poly([[hx + 0.05, hy + 0.45], [hx + 0.45, hy + 0.45], [hx + 0.25, hy + 0.7]]), "red-300", 1),
  ];
}

/** A monkey hanging by one long arm and reaching with the other, its tail curled behind, centred on the anchor; about 8 wide and 9.5 tall at scale 1. */
function monkey(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 10);
  const reach = between(rng, -0.6, 0.6);
  const T = (points: readonly Point[]): Point[] => points.map((point) => turned(point, tilt));
  const c = (dx: number, dy: number): Point => turned([dx, dy], tilt);
  const curl = between(rng, -0.4, 0.4);
  return [
    shape(f.band(T([[-0.8, 1.6], [-2.2, 2.8], [-3.6, 2.2], [-3.8, 0.8], [-3 + curl, 0.2], [-2.4, 0.8]]), 0.5), "wood-600", 1, MAIN_INKED),
    shape(f.band(T([[0.6, 1.5], [1.8, 2.4], [1.4, 3.8]]), 0.85) + f.band(T([[-0.5, 1.7], [-1.2, 3], [-0.6, 4.2]]), 0.85) + f.circle(...c(-1.3, -2.5), 0.5), "wood-600", 1, MAIN_INKED),
    shape(f.band(T([[0.6, -1.4], [0.9, -3], [0.7, -4.4]]), 0.75) + f.band(T([[0.9, -0.6], [2.4, reach * 0.5], [3.6, 0.9 + reach]]), 0.7), "wood-600", 1, MAIN_INKED),
    shape(f.poly(T([[0, -1.8], [1.1, -1.4], [1.5, 0], [1.2, 1.4], [0.4, 2.2], [-0.6, 2.1], [-1.4, 1.2], [-1.5, 0], [-1, -1.4]])), "wood-600", 1, MAIN_INKED),
    shape(f.circle(...c(-0.1, -2.6), 1.25), "wood-600", 1, MAIN_INKED),
    shape(f.ellipse(...c(0.45, -2.45), 0.82, 0.72) + f.ellipse(...c(-0.1, 0.4), 0.8, 1.1), "brass-500", 0.9),
    shape(f.circle(...c(0.2, -2.75), 0.16) + f.circle(...c(0.8, -2.75), 0.16) + f.band(T([[0.3, -2], [0.75, -2.05]]), 0.14), "black", 0.9),
  ];
}

/** A bee in flight centred on the anchor: an orange body striped black, two translucent wings; about 3.3 wide and 3 tall at scale 1. */
function bee(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flap = between(rng, -0.4, 0.4);
  const tilt = between(rng, -15, 15);
  const T = (dx: number, dy: number): string => f.p(...turned([dx, dy], tilt));
  const c = (dx: number, dy: number): Point => turned([dx, dy], tilt);
  const wing = (rootX: number, tipX: number, tipY: number): string =>
    `M${T(rootX, -0.45)}Q${T(tipX - 0.9, tipY + 0.2)} ${T(tipX, tipY)}Q${T(tipX + 0.6, tipY + 0.7)} ${T(rootX + 0.25, -0.4)}Z`;
  const stripe = (x: number): string => {
    const h = 0.7 * Math.sqrt(1 - (x / 1.05) ** 2) - 0.03;
    return `M${T(x - 0.15, -h)}L${T(x + 0.15, -h)}L${T(x + 0.15, h)}L${T(x - 0.15, h)}Z`;
  };
  return [
    shape(wing(-0.3, -0.9, -1.9 + flap), "white", 0.45),
    shape(f.ellipse(...c(0, 0), 1.05, 0.72), "orange-400", 1, MAIN),
    shape(stripe(-0.45) + stripe(0.2) + f.circle(...c(1.25, -0.05), 0.48) + `M${T(-1.02, -0.18)}L${T(-1.6, 0.05)}L${T(-1.02, 0.25)}Z`, "black", 1),
    shape(wing(0.05, 0.55, -1.8 - flap), "white", 0.6),
    shape(f.circle(...c(-0.2, -0.32), 0.18), "white", 0.6),
  ];
}

/** A hooded ghost floating centred on the anchor, translucent, tapering to a ragged tail behind it, with two dark eyes; about 8 wide and 12 tall at scale 1. */
function ghost(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const sway = j(0.6);
  const body =
    `M${f.p(-0.5, -6.2)}C${f.p(2.2, -6.3)} ${f.p(3.9, -4.2)} ${f.p(3.8, -1.6)}C${f.p(3.6, 1.4)} ${f.p(1.8, 3.4)} ${f.p(0.4 + sway * 0.5, 5)}` +
    `L${f.p(-1.6 + sway, 6 + j(0.3))}L${f.p(-1, 4.5)}L${f.p(-2.3 + sway * 0.6, 4.5 + j(0.4))}L${f.p(-1.7, 3.4)}L${f.p(-3 + sway * 0.3, 2.7 + j(0.3))}` +
    `C${f.p(-3.8, 1.2)} ${f.p(-3.9, -0.4)} ${f.p(-3.8, -1.6)}C${f.p(-3.8, -4)} ${f.p(-2.6, -5.6)} ${f.p(-0.5, -6.2)}Z`;
  return [
    shape(blob(f, rng, 0, -0.4, 5.6, 9), "slate-100", 0.1, MAIN),
    shape(body, "slate-100", 0.55, MAIN),
    shape(`M${f.p(-0.6, -5.4)}C${f.p(-2.6, -5)} ${f.p(-3.1, -2.4)} ${f.p(-2.6, 0.2)}C${f.p(-1.6, -1.4)} ${f.p(-1.2, -3.6)} ${f.p(-0.6, -5.4)}Z`, "white", 0.35),
    shape(f.ellipse(0.5, -2.6, 2, 2.2), "slate-700", 0.4),
    shape(f.ellipse(-0.3, -2.8, 0.42, 0.68) + f.ellipse(1.3, -2.8, 0.42, 0.68), "black", 0.9),
  ];
}

/** A white bird (or a winged spirit) in flight seen from below, wings spread, centred on the anchor; about 12 wide and 6 tall at scale 1. */
function bird(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flap = between(rng, -1, 1);
  const span = between(rng, 5.4, 6.2);
  const wing = (side: number): string => {
    const s = (dx: number, dy: number): string => f.p(side * dx, dy);
    return (
      `M${s(0.5, -0.7)}Q${s(span * 0.45, -2.2 + flap * 0.5)} ${s(span, -1.6 + flap)}Q${s(span * 0.88, -0.6 + flap * 0.6)} ${s(span * 0.8, -0.1 + flap * 0.4)}` +
      `Q${s(span * 0.7, 0)} ${s(span * 0.58, 0.6 + flap * 0.2)}Q${s(span * 0.46, 0.7)} ${s(span * 0.34, 1.1)}Q${s(span * 0.2, 1.1)} ${s(0.5, 0.8)}Z`
    );
  };
  const shade = (side: number): string => `M${f.p(side * 0.6, -0.3)}Q${f.p(side * span * 0.4, -0.9 + flap * 0.3)} ${f.p(side * span * 0.7, -0.3 + flap * 0.5)}Q${f.p(side * span * 0.4, 0.6)} ${f.p(side * 0.6, 0.6)}Z`;
  return [
    shape(wing(-1) + wing(1), "white", 1, MAIN),
    shape(shade(-1) + shade(1), "slate-400", 0.35),
    shape(
      `M${f.p(0, -2.8)}C${f.p(0.7, -2.8)} ${f.p(0.8, -1.6)} ${f.p(0.75, -0.6)}C${f.p(0.8, 0.8)} ${f.p(0.5, 1.6)} ${f.p(0.4, 2)}L${f.p(1.4, 3)}L${f.p(0.5, 3.3)}L${f.p(0, 2.9)}` +
        `L${f.p(-0.5, 3.3)}L${f.p(-1.4, 3)}L${f.p(-0.4, 2)}C${f.p(-0.5, 1.6)} ${f.p(-0.8, 0.8)} ${f.p(-0.75, -0.6)}C${f.p(-0.8, -1.6)} ${f.p(-0.7, -2.8)} ${f.p(0, -2.8)}Z`,
      "white",
      1,
      MAIN,
    ),
    shape(f.band([[0.2, 0.1], [0.3, 1.8]], 0.3), "slate-400", 0.3),
    shape(f.poly([[-0.25, -2.75], [0, -3.4], [0.25, -2.75]]), "orange-400", 1),
  ];
}

/** R3396000: a fox standing on the anchor, nosing the way it faces: a bushy white-tipped tail, dark socks, a pointed snout and ears, a white chest and muzzle; about 15 long and 7.5 tall at scale 1. */
function fox(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.8, 0.8);
  const dip = between(rng, 0, 1.2);
  const tip: Point = [-7.1, -3.2 + lift];
  return [
    shape(f.ellipse(0, 0.2, 5.4, 0.5), "black", 0.2),
    shape(`M${f.p(-3.4, -4)}Q${f.p(-6.4, -5.8 + lift)} ${f.p(-7.8, -3.4 + lift)}Q${f.p(-7, -1.4 + lift * 0.5)} ${f.p(-4.6, -2)}Q${f.p(-4, -2.6)} ${f.p(-3.4, -3)}Z`, "orange-400", 1, MAIN_INKED),
    shape(f.circle(tip[0], tip[1], 0.8), "white", 1),
    shape(f.band([[-2.4, -2.6], [-2.6, 0]], 0.55) + f.band([[-1.2, -2.6], [-1, 0]], 0.55) + f.band([[2.1, -2.6], [1.9, 0]], 0.55) + f.band([[3.2, -2.6], [3.4, 0]], 0.55), "wood-800", 1, INKED),
    shape(`M${f.p(-3.6, -3.6)}Q${f.p(-3.2, -5.2)} ${f.p(0, -5)}Q${f.p(3, -5.2)} ${f.p(3.8, -3.8)}Q${f.p(3.6, -2.2)} ${f.p(2.4, -2.2)}L${f.p(-2.6, -2.2)}Q${f.p(-3.8, -2.4)} ${f.p(-3.6, -3.6)}Z`, "orange-400", 1, MAIN_INKED),
    shape(
      `M${f.p(3.4, -4 + dip * 0.3)}L${f.p(3.6, -6 + dip)}L${f.p(3.9, -7.4 + dip)}L${f.p(4.6, -6.4 + dip)}L${f.p(5.4, -7.4 + dip)}L${f.p(5.6, -6 + dip)}` +
        `Q${f.p(6.6, -5.4 + dip)} ${f.p(7.6, -4.9 + dip)}Q${f.p(6.4, -4.2 + dip)} ${f.p(4.6, -3.6 + dip * 0.5)}Q${f.p(3.6, -3.4)} ${f.p(3.4, -4 + dip * 0.3)}Z`,
      "orange-400",
      1,
      MAIN_INKED,
    ),
    shape(f.poly([[3.95, -6.9 + dip], [4.3, -6.3 + dip], [3.8, -6.2 + dip]]) + f.poly([[5.35, -6.9 + dip], [5.25, -6.2 + dip], [4.9, -6.3 + dip]]), "wood-800", 0.8),
    shape(f.poly([[5.7, -4.75 + dip], [7.3, -4.9 + dip], [5.9, -4.2 + dip]]) + f.ellipse(3.4, -3.1, 0.7, 0.55), "white", 0.9),
    shape(f.circle(5.2, -5.7 + dip, 0.22) + f.circle(7.55, -4.9 + dip, 0.26), "black", 0.9),
  ];
}

/** R3397000: a shaggy white mountain goat standing on the anchor, facing the way it faces: a stocky body (main), a bearded head held up, two swept-back horns, dark hooves and a stub tail; about 14 long and 12 tall at scale 1. */
function goat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.8, 0.8);
  const horn = between(rng, 2.6, 3.6);
  const hx = 5.4;
  const hy = -8.6 + lift;
  const hornPath = (dx: number): Point[] => [
    [hx - 0.3 + dx, hy - 1],
    [hx - 0.9 + dx, hy - 1 - horn * 0.6],
    [hx - 2 + dx, hy - 1 - horn],
    [hx - 2.8 + dx, hy - 1 - horn * 0.8],
  ];
  return [
    shape(f.ellipse(0, 0.2, 6.4, 0.6), "black", 0.2),
    shape(f.band([[-3.6, -3.2], [-3.8, -0.4]], 0.9) + f.band([[-2, -3.2], [-1.8, -0.4]], 0.9) + f.band([[2.4, -3.2], [2.2, -0.4]], 0.9) + f.band([[3.9, -3.2], [4.1, -0.4]], 0.9), "slate-100", 1, INKED),
    shape(f.band([[-4.25, -0.25], [-3.35, -0.25]], 0.6) + f.band([[-2.25, -0.25], [-1.35, -0.25]], 0.6) + f.band([[1.75, -0.25], [2.65, -0.25]], 0.6) + f.band([[3.65, -0.25], [4.55, -0.25]], 0.6), "slate-900", 1),
    shape(f.poly([[-5.6, -4], [-4.4, -4.2], [-5.2, -5.6]]), "white", 1, INKED),
    shape(`M${f.p(-5.2, -4.6)}Q${f.p(-5.8, -7.8)} ${f.p(-2, -7.9)}L${f.p(2.6, -7.9)}Q${f.p(5.4, -7.7)} ${f.p(5.2, -4.8)}Q${f.p(5, -2.6)} ${f.p(3, -2.6)}L${f.p(-3.4, -2.6)}Q${f.p(-5.4, -2.8)} ${f.p(-5.2, -4.6)}Z`, "white", 1, MAIN_INKED),
    shape(f.band([[3.4, -6.4], [hx - 0.2, hy + 0.8]], 2.4), "white", 1, INKED),
    shape(`M${f.p(hx - 1.2, hy - 1.1)}Q${f.p(hx + 0.8, hy - 1.8)} ${f.p(hx + 2.8, hy - 0.2)}Q${f.p(hx + 3, hy + 0.8)} ${f.p(hx + 2.2, hy + 0.9)}L${f.p(hx - 0.6, hy + 1.2)}Z`, "white", 1, INKED),
    shape(f.poly([[hx + 1.5, hy + 0.8], [hx + 1.3, hy + 3], [hx + 2.4, hy + 0.8]]), "slate-400", 0.9),
    shape(f.band(hornPath(-0.9), 0.55), "stone-500", 0.85),
    shape(f.band(hornPath(0), 0.6), "wood-800", 1, INKED),
    shape(f.circle(hx + 0.9, hy - 0.4, 0.26), "black", 0.9),
  ];
}

/** R3397000: a deer walking on the anchor the way it faces, one foreleg lifted: a slim body (main), a long neck and pointed head, its ear back, branched antlers and a white rump; about 16 long and 15 tall at scale 1. */
function deer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const step = between(rng, 0.6, 1.6);
  const tines = whole(rng, 2, 3);
  const hx = 6.6;
  const hy = -10.6 + between(rng, -0.5, 0.5);
  const beam = (dx: number): Point[] => [
    [hx - 0.6 + dx, hy - 1],
    [hx - 1.2 + dx, hy - 3.2],
    [hx - 0.6 + dx, hy - 5.4],
    [hx + 0.6 + dx, hy - 6.6],
  ];
  let prongs = "";
  for (let k = 0; k < tines; k += 1) {
    const [bx, by] = beam(0)[k + 1] ?? [hx, hy];
    prongs += f.band([[bx, by], [bx + 1.6, by - 1.2]], 0.42);
  }
  return [
    shape(f.ellipse(0, 0.2, 7, 0.55), "black", 0.2),
    shape(f.band([[-4.2, -5], [-4.6, 0]], 0.75) + f.band([[-2.6, -5], [-2.2, 0]], 0.75) + f.band([[3, -5], [3.4, 0]], 0.75) + f.band([[4.8, -5], [5.4 + step * 0.6, -2.4 + step * 0.4], [5.2 + step, -step]], 0.75), "wood-700", 1, INKED),
    shape(`M${f.p(-5.6, -7)}Q${f.p(-5.8, -9.4)} ${f.p(-2.8, -9.4)}L${f.p(3, -9.2)}Q${f.p(5.8, -9.2)} ${f.p(5.6, -6.6)}Q${f.p(5.2, -4.6)} ${f.p(3, -4.6)}L${f.p(-3.6, -4.6)}Q${f.p(-5.6, -4.8)} ${f.p(-5.6, -7)}Z`, "wood-600", 1, MAIN_INKED),
    shape(f.ellipse(-5, -6.8, 1.2, 1.6), "white", 0.85),
    shape(f.band([[3.8, -8.4], [hx - 0.4, hy + 0.6]], 2.2), "wood-600", 1, INKED),
    shape(`M${f.p(hx - 1.2, hy - 1)}Q${f.p(hx + 0.6, hy - 1.6)} ${f.p(hx + 3, hy + 0.4)}Q${f.p(hx + 3, hy + 1.2)} ${f.p(hx + 2.2, hy + 1.2)}L${f.p(hx - 0.8, hy + 1.3)}Z`, "wood-600", 1, INKED),
    shape(f.poly([[hx - 0.8, hy - 0.6], [hx - 3, hy - 1.8], [hx - 1.4, hy + 0.2]]), "wood-700", 1, INKED),
    shape(f.band(beam(-0.8), 0.45), "wood-800", 0.75),
    shape(f.band(beam(0), 0.5), "wood-800", 1, INKED),
    shape(prongs, "wood-800", 1),
    shape(f.circle(hx + 0.7, hy - 0.2, 0.24) + f.circle(hx + 2.9, hy + 0.6, 0.3), "black", 0.9),
  ];
}

/** R3397000: a mallard paddling, its waterline on the anchor, facing the way it faces: a grey body (main), a brown chest and wing with its blue flash, a green head on a white-ringed neck, an orange bill and a ripple ring; about 9 long and 7 tall at scale 1. */
function duck(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bob = between(rng, -0.4, 0.5);
  const hx = 3.1;
  const hy = -5.1 + bob;
  return [
    shape(f.ellipse(0, 0.2, 5.6, 0.9) + f.ellipse(0, 0.2, 4.4, 0.55), "white", 0.4),
    shape(f.poly([[-3.4, -2.4], [-5.2, -3.6], [-4.6, -2.2], [-3.6, -1.4]]), "slate-900", 1, INKED),
    shape(`M${f.p(-3.8, -1.4)}Q${f.p(-4.4, -3.4)} ${f.p(-2, -3.4)}L${f.p(1.6, -3.2)}Q${f.p(3.4, -3)} ${f.p(3.3, -1.4)}Q${f.p(2.8, 0)} ${f.p(0.4, 0)}L${f.p(-2.6, 0)}Q${f.p(-3.8, -0.2)} ${f.p(-3.8, -1.4)}Z`, "stone-500", 1, MAIN_INKED),
    shape(f.ellipse(2.3, -1.7, 1.3, 1.2), "wood-700", 1),
    shape(f.ellipse(-0.6, -2.1, 2.4, 0.85), "wood-600", 0.9, INKED),
    shape(f.band([[-1.6, -1.95], [0.2, -1.95]], 0.4), "blue-600", 1),
    shape(f.band([[2.4, -2.6], [hx - 0.2, hy + 0.8]], 1.3), "green-700", 1, INKED),
    shape(f.band([[2.25, -2.85], [3.05, -2.75]], 0.35), "white", 1),
    shape(f.circle(hx, hy, 1.25), "green-700", 1, INKED),
    shape(f.poly([[hx + 0.9, hy - 0.1], [hx + 2.6, hy + 0.3], [hx + 0.9, hy + 0.6]]), "orange-400", 1, INKED),
    shape(f.circle(hx + 0.3, hy - 0.4, 0.22), "black", 0.9),
  ];
}

// R3398000: the parts the Core Alt art scenes added.

/** R3398000: a palm standing on the anchor: a ringed trunk curving up and over, five to seven fronds drooping from its crown (main, every other one in a second shape so that no two touching fronds share one) over darker ones behind, their midribs, and a cluster of coconuts spaced clear of each other; about 16 wide and 30 tall at scale 1. */
function palm(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const height = between(rng, 26, 29);
  const lean = between(rng, 2.5, 5.5);
  const crown: Point = [lean, -height];
  const on = (t: number): Point => [lean * t * t, -height * t];
  let trunk = `M${f.p(-1.3, 0)}`;
  for (const t of [0.35, 0.7, 1]) trunk += `L${f.p(on(t)[0] - 1.3 + 0.6 * t, on(t)[1])}`;
  for (const t of [1, 0.7, 0.35]) trunk += `L${f.p(on(t)[0] + 1.3 - 0.6 * t, on(t)[1])}`;
  trunk += `L${f.p(1.3, 0)}Z`;
  let rings = "";
  for (let t = 0.1; t < 0.96; t += 0.11) {
    const [x, y] = on(t);
    rings += f.band([[x - 1.2 + 0.5 * t, y], [x + 1.2 - 0.5 * t, y - 0.3]], 0.28);
  }
  const fronds = whole(rng, 5, 7);
  const front = ["", ""];
  let back = "";
  let ribs = "";
  for (let k = 0; k < fronds + 2; k += 1) {
    const behind = k >= fronds;
    const angle = behind ? -90 + (k === fronds ? -40 : 40) : -172 + (164 * k) / (fronds - 1) + between(rng, -6, 6);
    const length = between(rng, 8, 10) * (behind ? 0.8 : 1);
    const u: Point = [Math.cos(angle * DEGREES), Math.sin(angle * DEGREES)];
    const droop = 3.2 * Math.abs(u[0]);
    const base: Point = [crown[0] + u[0] * 0.9, crown[1] + u[1] * 0.9];
    const tip: Point = [crown[0] + u[0] * length, crown[1] + u[1] * length + droop];
    const mid: Point = [(base[0] + tip[0]) / 2, (base[1] + tip[1]) / 2 - 1.6 * Math.abs(u[0])];
    const n: Point = [-u[1] * 1.5, u[0] * 1.5];
    const leaf = `M${f.p(...base)}Q${f.p(mid[0] + n[0], mid[1] + n[1])} ${f.p(...tip)}Q${f.p(mid[0] - n[0], mid[1] - n[1])} ${f.p(...base)}Z`;
    if (behind) back += leaf;
    else {
      front[k % 2] += leaf;
      ribs += `M${f.p(...base)}Q${f.p(mid[0] + n[0] * 0.12, mid[1] + n[1] * 0.12)} ${f.p(...tip)}Q${f.p(mid[0] - n[0] * 0.12, mid[1] - n[1] * 0.12)} ${f.p(...base)}Z`;
    }
  }
  const nuts = whole(rng, 2, 3);
  let coconuts = "";
  for (let k = 0; k < nuts; k += 1) coconuts += f.circle(crown[0] - 1.8 + 1.8 * k, crown[1] + 1.5 + (k % 2) * 0.5, 0.85);
  return [
    shape(f.ellipse(lean * 0.2, 0.3, 4.2, 0.6), "black", 0.2),
    shape(trunk, "wood-600", 1, INKED),
    shape(rings, "wood-800", 0.6),
    shape(back, "green-700", 1, INKED),
    shape(coconuts, "wood-700", 1, INKED),
    shape(front[0] ?? "", "green-500", 1, MAIN_INKED),
    shape(front[1] ?? "", "green-500", 1, MAIN_INKED),
    shape(ribs, "green-900", 0.55),
  ];
}

/** R3398000: a one-humped camel standing on the anchor and facing the way it faces: four thin knobbly legs, a tufted tail, its hump and belly, a long curved neck and a drooping head with an ear and an eye, and a saddle blanket over the hump (main) with a fringed hem; about 20 long and 18 tall at scale 1. */
function camel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hump = between(rng, -16.4, -15);
  const nod = between(rng, -0.8, 0.8);
  const legs = (xs: readonly number[]): string => xs.map((x) => f.band([[x, -7], [x + 0.3, -3.6], [x - 0.1, 0]], 0.75)).join("");
  return [
    shape(f.ellipse(0, 0.3, 8.6, 0.7), "black", 0.2),
    shape(legs([-4.6, 4.2]), "wood-700", 1, INKED),
    shape(f.band([[-7.2, -9.6], [-8.2, -7.6], [-8, -5.6]], 0.45), "wood-700", 1),
    shape(
      `M${f.p(-7.6, -8.6)}Q${f.p(-7.8, -11.4)} ${f.p(-5, -11.8)}Q${f.p(-1.6, hump)} ${f.p(1.6, -11.8)}Q${f.p(5.4, -11.6)} ${f.p(6, -9.4)}Q${f.p(5.4, -6.4)} ${f.p(0, -6.6)}Q${f.p(-7, -6.4)} ${f.p(-7.6, -8.6)}Z`,
      "brass-600",
      1,
      INKED,
    ),
    shape(legs([-6, 2.8]), "brass-600", 1, INKED),
    shape(f.band([[4.8, -10], [7, -12.2], [8, -15.2 + nod]], 1.6), "brass-600", 1, INKED),
    shape(f.ellipse(9.2, -15.6 + nod, 2, 1.05) + f.poly([[7.6, -16.4 + nod], [7.8, -17.6 + nod], [8.4, -16.6 + nod]]), "brass-600", 1, INKED),
    shape(f.poly([[-4.4, -12.2], [-1.6, hump + 0.9], [1.4, -12.4], [2, -8.4], [-4.6, -8.2]]), "red-600", 1, MAIN_INKED),
    shape(f.band([[-4.6, -8.2], [2, -8.4]], 0.5), "blue-600", 1),
    shape(f.circle(9.4, -16 + nod, 0.25) + f.circle(11, -15.3 + nod, 0.18), "black", 0.9),
  ];
}

// Food

/** An apple standing on the anchor, with a stem, a leaf and a shine; about 3.7 across and 4.3 tall (leaf included) at scale 1. */
function apple(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 1.8 * between(rng, 0.92, 1.06);
  const tilt = between(rng, -12, 12);
  const cy = -0.96 * r;
  const [sx, sy] = turned([0, -0.72 * r], tilt);
  const [tx, ty] = turned([0.25 * r, -1.25 * r], tilt);
  return [
    shape(applePath(f, 0, cy, r, tilt), "red-600", 1, MAIN_INKED),
    shape(underside(f, 0.15 * r, cy + 0.1 * r, 0.88 * r, 0.82 * r, 0.2), "black", 0.25),
    shape(f.band([[sx, cy + sy], [tx, cy + ty]], 0.26), "wood-700", 1),
    shape(`M${f.p(tx - 0.05, cy + ty + 0.15)}Q${f.p(tx + 0.5, cy + ty - 0.9)} ${f.p(tx + 1.4, cy + ty - 0.6)}Q${f.p(tx + 0.8, cy + ty + 0.2)} ${f.p(tx - 0.05, cy + ty + 0.15)}Z`, "green-700", 1),
    shape(f.ellipse(-0.45 * r, cy - 0.2 * r, 0.18 * r, 0.32 * r), "white", 0.6),
  ];
}

/** A pear standing on the anchor, with a stem and a shine; about 3.4 wide and 4.4 tall (stem besides) at scale 1. */
function pear(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 1.76 * between(rng, 0.94, 1.06);
  const tilt = between(rng, -10, 10);
  const cy = -0.98 * r;
  const [sx, sy] = turned([0, -1.55 * r], tilt);
  const [tx, ty] = turned([0.3, -1.55 * r - 0.9], tilt);
  return [
    shape(pearPath(f, 0, cy, r, tilt), "lime-400", 1, MAIN_INKED),
    shape(underside(f, 0.12 * r, cy + 0.05 * r, 0.9 * r, 0.9 * r, 0.25), "black", 0.2),
    shape(f.ellipse(0.4 * r, cy + 0.2 * r, 0.3 * r, 0.22 * r), "red-600", 0.25),
    shape(f.band([[sx, cy + sy + 0.1], [(sx + tx) / 2 + 0.1, cy + (sy + ty) / 2], [tx, cy + ty]], 0.25), "wood-700", 1),
    shape(f.ellipse(-0.45 * r, cy - 0.1 * r, 0.16 * r, 0.3 * r), "white", 0.6),
  ];
}

/** A bunch of eight to twelve grapes hanging from the anchor (their stem's top), with a leaf; about 5.5 wide and 7 tall at scale 1. */
function grapes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const layouts: readonly (readonly number[])[] = [
    [2, 3, 2, 1],
    [3, 2, 3, 2],
    [4, 3, 2, 1],
    [3, 2, 3, 2, 1],
    [3, 4, 3, 2],
  ];
  const rows = layouts[whole(rng, 0, layouts.length - 1)] ?? [4, 3, 2, 1];
  const r = between(rng, 0.58, 0.62);
  const lean = between(rng, -0.12, 0.12);
  const grapesAt: Point[] = [];
  rows.forEach((count, i) => {
    const y = 1.5 + i * 1.1;
    for (let k = 0; k < count; k += 1) grapesAt.push([(k - (count - 1) / 2) * 1.3 + lean * y, y]);
  });
  const side = rng() < 0.5 ? -1 : 1;
  return [
    shape(f.band([[0, 0], [lean * 0.6, 0.6], [lean * 1.2, 1.2]], 0.3) + f.band([[side * 0.3, 0.3], [side * 0.9, 0.1]], 0.2), "wood-700", 1),
    shape(grapesAt.map(([x, y]) => f.circle(x, y, r)).join(""), "violet-500", 1, MAIN_INKED),
    shape(grapesAt.map(([x, y]) => f.circle(x + r * 0.2, y + r * 0.2, r * 0.68)).join(""), "black", 0.28),
    shape(
      grapesAt
        .slice(0, 4)
        .map(([x, y]) => f.circle(x - r * 0.38, y - r * 0.38, r * 0.2))
        .join(""),
      "white",
      0.65,
    ),
    shape(
      `M${f.p(side * 0.7, 0.2)}Q${f.p(side * 1.6, -1.2)} ${f.p(side * 2.8, -0.6)}Q${f.p(side * 2.4, 0.1)} ${f.p(side * 2.9, 0.7)}Q${f.p(side * 2, 1.2)} ${f.p(side * 0.7, 0.2)}Z`,
      "green-700",
      1,
      INKED,
    ),
  ];
}

/** A papaya cut in half lying on the anchor, its flesh up, black seeds in its hollow, its fat end the way it faces; about 12 wide and 6 tall at scale 1. */
function papaya(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = 6 * between(rng, 0.95, 1.05);
  const seeds: string[] = [];
  for (const y of [-4.15, -3.45, -2.75]) {
    for (let x = -1.6; x <= 3.7; x += 0.85) {
      const sx = x + between(rng, -0.08, 0.08) + (y === -3.45 ? 0.42 : 0);
      if (((sx - 1.1) / 2.9) ** 2 + ((y + 3.45) / 1.05) ** 2 < 0.8 && rng() < 0.85) seeds.push(f.circle(sx, y + between(rng, -0.06, 0.06), 0.32));
    }
  }
  return [
    shape(podPath(f, 0, -3, a, 3, 0.72), "green-700", 1, INKED),
    shape(podPath(f, 0.15, -3.4, a - 0.6, 2.45, 0.7), "orange-400", 1, MAIN),
    shape(podPath(f, 1.1, -3.45, 3.3, 1.35, 0.6), "ember-500", 0.55),
    shape(seeds.length > 0 ? seeds.join("") : f.circle(1, -3.45, 0.32), "black", 0.95),
    shape(`M${f.p(-a + 1, -3.6)}Q${f.p(-a + 1.6, -5.4)} ${f.p(-0.5, -5.75)}Q${f.p(-a + 2.2, -5)} ${f.p(-a + 1, -3.6)}Z`, "white", 0.3),
    shape(`M${f.p(-a + 0.4, -2)}Q${f.p(0, 0.4)} ${f.p(a - 0.4, -2)}Q${f.p(0, -0.8)} ${f.p(-a + 0.4, -2)}Z`, "black", 0.22),
  ];
}

/** A wicker basket on the anchor heaped with fruit: red apples, a pear, violet grapes and a leaf; about 13 wide and 9.5 tall at scale 1. */
function basket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const half = (y: number): number => 4.2 + 1.8 * (-y / 5.2);
  const swap = rng() < 0.5 ? -1 : 1;
  const grape = [[0, 0], [1.24, 0], [-1.24, 0], [0.62, -1.07], [-0.62, -1.07], [0, -2.14]] as const;
  const gx = -0.6 * swap + j(0.3);
  return [
    shape(grape.map(([x, y]) => f.circle(gx + x, -7.4 + y, 0.62)).join(""), "violet-500", 1, INKED),
    shape(f.poly([[-0.8, -7.2], [-2, -9.4], [-3, -8.8]]), "green-700", 1),
    shape(applePath(f, -3.3 * swap, -6.1, 1.6, j(12)) + applePath(f, 1 * swap, -6.5, 1.55, j(12)), "red-600", 1, INKED),
    shape(pearPath(f, 4 * swap, -5.9, 1.05, swap * between(rng, 8, 20)), "lime-400", 1, INKED),
    shape(f.circle(-3.9 * swap, -6.7, 0.35) + f.circle(0.45 * swap, -7.1, 0.33), "white", 0.6),
    shape(`M${f.p(-6, -5.2)}L${f.p(6, -5.2)}Q${f.p(5.6, -1.2)} ${f.p(4.2, 0)}L${f.p(-4.2, 0)}Q${f.p(-5.6, -1.2)} ${f.p(-6, -5.2)}Z`, "wood-600", 1, MAIN_INKED),
    shape([-1.4, -2.8, -4.1].map((y) => f.band([[-half(y) + 0.35, y], [0, y + 0.25], [half(y) - 0.35, y]], 0.4)).join(""), "wood-800", 0.5),
    shape([-3.3, -1.65, 0, 1.65, 3.3].map((x) => f.band([[x * 0.78, -0.4], [x, -4.8]], 0.3)).join(""), "wood-800", 0.3),
    shape(f.poly([[2.6, -0.1], [3.9, -5], [6, -5], [5.6, -1.2], [4.2, -0.1]]), "black", 0.25),
    shape(f.band([[-6.3, -5.25], [0, -5.6], [6.3, -5.25]], 1.1), "wood-700", 1, INKED),
  ];
}

/** A stack of three or four pancakes on a plate on the anchor, a pat of butter on top and syrup running down; about 9 wide and 6 tall at scale 1. */
function pancakes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 4);
  const thick = 1.02;
  const cakes: string[] = [];
  const lows: string[] = [];
  const highs: string[] = [];
  let top = -0.75;
  for (let k = 0; k < count; k += 1) {
    const x = between(rng, -0.3, 0.3);
    const w = 3.6 * between(rng, 0.94, 1.04) - k * 0.08;
    const y0 = top - 0.04;
    const y1 = y0 - thick;
    top = y1;
    cakes.push(`M${f.p(x - w + 0.5, y1)}L${f.p(x + w - 0.5, y1)}Q${f.p(x + w, y1)} ${f.p(x + w, (y0 + y1) / 2)}Q${f.p(x + w, y0)} ${f.p(x + w - 0.5, y0)}L${f.p(x - w + 0.5, y0)}Q${f.p(x - w, y0)} ${f.p(x - w, (y0 + y1) / 2)}Q${f.p(x - w, y1)} ${f.p(x - w + 0.5, y1)}Z`);
    lows.push(f.band([[x - w + 0.6, y0 - 0.2], [x + w - 0.6, y0 - 0.2]], 0.3));
    highs.push(f.band([[x - w + 0.7, y1 + 0.22], [x + w - 1.2, y1 + 0.22]], 0.22));
  }
  const ty = top;
  let syrup = `M${f.p(-2.9, ty + 0.1)}Q${f.p(0, ty - 0.9)} ${f.p(2.9, ty + 0.1)}`;
  const drips = [2.3, 0.9, -0.6, -2].map((x) => [x + between(rng, -0.2, 0.2), between(rng, 0.5, 2.6)] as const);
  for (const [x, length] of drips) syrup += `L${f.p(x + 0.3, ty + 0.35)}L${f.p(x + 0.3, ty + length)}Q${f.p(x, ty + length + 0.5)} ${f.p(x - 0.3, ty + length)}L${f.p(x - 0.3, ty + 0.4)}`;
  syrup += `Z`;
  const bx = between(rng, -0.6, 0.6);
  return [
    shape(f.ellipse(0, -0.4, 4.6, 0.75), "white", 0.9, INKED),
    shape(underside(f, 0, -0.4, 4.4, 0.7, 0.3), "slate-400", 0.6),
    shape(cakes.join(""), "brass-600", 1, MAIN_INKED),
    shape(lows.join(""), "wood-700", 0.45),
    shape(highs.join(""), "brass-500", 0.6),
    shape(syrup, "wood-700", 0.95),
    shape(f.band([[-1.8, ty - 0.2], [0.4, ty - 0.45]], 0.18), "white", 0.3),
    shape(f.poly([[bx - 0.8, ty - 0.35], [bx + 0.6, ty - 0.35], [bx + 0.85, ty - 1.05], [bx - 0.55, ty - 1.05]]), "slate-100", 1, INKED),
  ];
}

/** R3397000: a heap of walnuts on the anchor, two rows of round wrinkled shells (main) with their seams, and cracked half-shells and kernels on either side; about 18 wide and 5 tall at scale 1. */
function walnuts(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = whole(rng, 2, 3);
  let low = "";
  let high = "";
  let seams = "";
  for (const x of [-4.5, -1.5, 1.5, 4.5]) {
    const y = -1.35 + between(rng, -0.1, 0.1);
    low += f.ellipse(x, y, 1.42, 1.3);
    seams += f.band([[x + 0.1, y - 1], [x - 0.1, y + 0.9]], 0.25);
  }
  for (let k = 0; k < top; k += 1) {
    const x = (k - (top - 1) / 2) * 3;
    const y = -3.75 + between(rng, -0.1, 0.1);
    high += f.ellipse(x, y, 1.42, 1.3);
    seams += f.band([[x + 0.1, y - 1], [x - 0.1, y + 0.9]], 0.25);
  }
  const cup = (cx: number): string => `M${f.p(cx - 1.6, -1.2)}Q${f.p(cx - 1.6, 0.3)} ${f.p(cx, 0.3)}Q${f.p(cx + 1.6, 0.3)} ${f.p(cx + 1.6, -1.2)}Z`;
  return [
    shape(f.ellipse(0, 0.25, 9, 0.6), "black", 0.2),
    shape(low, "wood-600", 1, MAIN_INKED),
    shape(high, "wood-600", 1, MAIN_INKED),
    shape(seams, "wood-900", 0.6),
    shape(cup(-7.6) + cup(7.4), "wood-700", 1, INKED),
    shape(f.ellipse(-7.6, -1.2, 1.6, 0.4) + f.ellipse(7.4, -1.2, 1.6, 0.4), "wood-900", 0.8),
    shape(blob(f, rng, -7.6, -1.3, 0.8, 6) + blob(f, rng, 7.4 + between(rng, -0.3, 0.3), -1.2, 0.7, 6), "brass-500", 1),
    shape(f.circle(-4.9, -1.9, 0.35) + f.circle(-0.4, -4.3, 0.35), "white", 0.3),
  ];
}

// Weather and light

/** An expanding shockwave centred on the anchor: three flat rings fading outward, the outer about 44 wide and 12 tall at scale 1. */
function shockwave(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spread = between(rng, 0.94, 1.04);
  const out = [shape(f.ellipse(0, 0, 21 * spread, 5.6 * spread), "violet-300", 0.07, MAIN)];
  [0.75, 0.5, 0.28].forEach((opacity, k) => {
    const rx = (7.4 + k * 7.3) * spread;
    const ry = (2 + k * 2) * spread;
    const t = 1.1 + k * 0.35;
    out.push(shape(f.ellipse(0, 0, rx, ry) + f.ellipse(0, -t * 0.25, rx - t, ry - t * 0.4), "violet-300", opacity, MAIN));
  });
  out.push(shape(f.ellipse(0, 0, 7.2 * spread, 1.95 * spread) + f.ellipse(0, -0.1, 6.9 * spread, 1.78 * spread), "white", 0.6));
  out.push(shape(f.ellipse(0, -0.3, 3.6, 1.1), "white", 0.3));
  return out;
}

/** A tornado's funnel of wind, its tip on the anchor, widening upward to about 18 wide at 30 above it at scale 1, with debris. */
function tornado(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const height = 30;
  const sway = between(rng, -3, 3);
  const bend = between(rng, -2, 2);
  const axis = (t: number): number => sway * t * t + bend * Math.sin(Math.PI * t);
  const half = (t: number): number => 0.5 + 8.5 * t ** 1.4;
  const left: Point[] = [];
  const right: Point[] = [];
  for (let i = 0; i <= 6; i += 1) {
    const t = i / 6;
    left.push([axis(t) - half(t), -height * t]);
    right.push([axis(t) + half(t), -height * t]);
  }
  const out = [shape(f.poly([...left, ...right.reverse()]), "slate-400", 0.22, MAIN)];
  const bands = whole(rng, 5, 6);
  for (let k = 0; k < bands; k += 1) {
    const t = 0.16 + (0.84 * k) / (bands - 1);
    const y = -height * t;
    const cx = axis(t);
    const w = half(t);
    const arc: Point[] = [-1.05, -0.6, 0, 0.6, 1.05].map((u) => [cx + w * u, y + w * 0.24 * (1 - u * u) - (Math.abs(u) > 1 ? 0.3 : 0)]);
    out.push(shape(f.band(arc, 0.6 + 1.4 * t), "slate-400", 0.62 - k * 0.05, MAIN));
  }
  const debris = scatter(rng, whole(rng, 6, 9), 0, -height * 0.5, 13, 14, 0.25, 0.55, 0.3)
    .filter(([x, y]) => Math.abs(x - axis(-y / height)) > half(-y / height) * 0.8)
    .map(([x, y, r]) => f.poly([turned([r, 0], x * 40), turned([0, r * 0.7], x * 40), turned([-r, 0], x * 40), turned([0, -r * 0.7], x * 40)].map(([dx, dy]) => [x + dx, y + dy] as Point)));
  out.push(shape(debris.length > 0 ? debris.join("") : f.circle(half(0.5) + 1.5, -height * 0.5, 0.35), "wood-800", 0.85));
  return out;
}

/** A fire of three to five tongues of flame standing on the anchor, ember outside, orange inside, a white core; about 8 wide and 10 tall at scale 1. */
function flames(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const lean = between(rng, -0.12, 0.12);
  const width = 3;
  const tips: Point[] = [];
  for (let k = 0; k < count; k += 1) {
    const x = -width + (2 * width * (k + 0.5)) / count + between(rng, -0.25, 0.25);
    const h = 10 * (1 - Math.abs(x / width) * 0.5) * between(rng, 0.78, 1);
    tips.push([x + lean * h, -h]);
  }
  const valleys = (scale: number): Point[] =>
    tips.slice(0, -1).map(([x, y], k) => {
      const [nx, ny] = tips[k + 1] ?? [x, y];
      return [((x + nx) / 2) * scale, Math.max(y, ny) * scale * 0.4];
    });
  const inner = tips.map(([x, y]): Point => [x * 0.55, y * 0.66]);
  const sparks = [0, 1].map(() => {
    const x = between(rng, -3, 3);
    const y = -between(rng, 10.5, 12.5);
    return f.poly([[x, y - 0.6], [x + 0.25, y], [x, y + 0.35], [x - 0.25, y]]);
  });
  return [
    shape(f.ellipse(0, -4.4, 6, 6.4), "orange-400", 0.14),
    shape(f.ellipse(0, -0.1, 5, 1), "ember-500", 0.2),
    shape(tongues(f, tips, valleys(1), 3.6), "ember-500", 0.92, MAIN),
    shape(tongues(f, inner, valleys(0.6).map(([x, y]): Point => [x * 0.92, y * 1.1]), 2.2), "orange-400", 1, MAIN),
    shape(tongues(f, [[lean * 3.4, -3.4]], [], 1), "white", 0.9),
    shape(sparks.join(""), "orange-400", 0.85),
  ];
}

/** A fireball flying the way it faces, centred on its core (about 6.4 across), its flame tail about 10 long behind it at scale 1. */
function fireball(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (n: number): number => between(rng, -n, n);
  const flicker = [j(0.6), j(0.6), j(0.6)] as const;
  const fire = (s: number, ox: number): string => {
    const p = (dx: number, dy: number): string => f.p(ox + dx * s, dy * s);
    const R = 3.2;
    return (
      `M${p(R, 0)}C${p(R, -R * 0.6)} ${p(R * 0.55, -R)} ${p(0, -R)}Q${p(-4.4, -R)} ${p(-8.4, -2.8 + flicker[0])}Q${p(-7, -1.8)} ${p(-7.6, -1.2)}` +
      `Q${p(-9.6, -1.2)} ${p(-13, -0.2 + flicker[1])}Q${p(-9.6, 0.9)} ${p(-8, 1)}Q${p(-8.4, 1.6)} ${p(-9.6, 2.5 + flicker[2])}` +
      `Q${p(-4.4, R)} ${p(0, R)}C${p(R * 0.55, R)} ${p(R, R * 0.6)} ${p(R, 0)}Z`
    );
  };
  const sparks = [0, 1, 2].map(() => {
    const x = -between(rng, 9, 15);
    const y = j(3.4);
    return f.poly([[x - 0.6, y], [x, y - 0.25], [x + 0.35, y], [x, y + 0.25]]);
  });
  return [
    shape(f.circle(0, 0, 5.6), "ember-500", 0.16),
    shape(f.ellipse(-5, 0, 8.4, 3.8), "orange-400", 0.1),
    shape(fire(1, 0), "ember-500", 0.95, MAIN),
    shape(fire(0.62, 0.9), "orange-400", 1, MAIN),
    shape(f.circle(1.1, -0.2, 1.5), "white", 0.9),
    shape(sparks.join(""), "orange-400", 0.8),
  ];
}

/** A ribbon of light looping in a figure-eight centred on the anchor, a wide translucent band and a bright thin one; about 40 wide and 16 tall at scale 1. */
function lightRibbon(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = 19 * between(rng, 0.94, 1.04);
  const b = 7.2 * between(rng, 0.9, 1.08);
  const tilt = between(rng, -9, 9);
  const steps = 12;
  const piece = (from: number, width: number): string => {
    const points: Point[] = [];
    const widths: number[] = [];
    for (let i = 0; i <= steps; i += 1) {
      const t = from + (Math.PI * i) / steps;
      points.push(turned([a * Math.sin(t), b * Math.sin(2 * t)], tilt));
      widths.push(width * (0.3 + 0.7 * Math.abs(Math.cos(t))));
    }
    return taper(f, points, widths);
  };
  const back = Math.PI / 2;
  const front = -Math.PI / 2;
  const glint = (t: number, size: number): string => {
    const [x, y] = turned([a * Math.sin(t), b * Math.sin(2 * t)], tilt);
    return sparkle(f, x, y, size);
  };
  return [
    shape(piece(back, 4.2), "violet-300", 0.26, MAIN),
    shape(piece(back, 1), "violet-300", 0.85, MAIN),
    shape(piece(front, 4.2), "violet-300", 0.3, MAIN),
    shape(piece(front, 1), "violet-300", 0.95, MAIN),
    shape(glint(between(rng, 0.5, 1.1), 1.8) + glint(between(rng, 3.6, 4.2), 1.3), "white", 0.85),
  ];
}

/** A cloud-shaped thought bubble centred on the anchor, about 10 wide and 7 tall at scale 1, two small bubbles trailing to its lower left (to about 6 below the anchor). */
function thoughtBubble(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const trail = between(rng, -0.3, 0.3);
  return [
    shape(lobed(f, rng, 0, 0, 4.4, 2.9, 8, 1.32) + f.circle(-3.9 + trail, 4.3, 0.85) + f.circle(-5.3 + trail * 1.5, 5.8, 0.5), "slate-100", 0.8, MAIN_INKED),
    shape(f.ellipse(0.4, 1.4, 3.4, 1.1), "slate-400", 0.25),
    shape(f.ellipse(-1.6, -1.5, 1.7, 0.7), "white", 0.45),
  ];
}

/** A puff of glowing spores over a field about 16 across, centred on the anchor: dots in a faint cloud. */
function spores(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dots = scatter(rng, whole(rng, 10, 15), 0, 0, 8, 7.5, 0.2, 0.6, 0.25);
  const big = dots.filter(([, , r]) => r > 0.42);
  const small = dots.filter(([, , r]) => r <= 0.42);
  const circles = (list: readonly Dot[], scale: number): string => list.map(([x, y, r]) => f.circle(x, y, r * scale)).join("");
  const out = [shape(blob(f, rng, 0, 0, 7.2, 9), "green-400", 0.1, MAIN), shape(blob(f, rng, between(rng, -1, 1), between(rng, -1, 1), 4, 8), "green-400", 0.12, MAIN)];
  if (small.length > 0) out.push(shape(circles(small, 1), "green-400", 0.55, MAIN));
  if (big.length > 0) out.push(shape(circles(big, 1), "green-400", 0.9, MAIN), shape(circles(big, 0.45), "white", 0.8));
  return out;
}

/** Falling snow over a field about 40 wide and 30 tall centred on the anchor: round flakes and a few sparkling ones. */
function snow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flakes: Dot[] = [];
  for (let tries = 0; flakes.length < 20 && tries < 120; tries += 1) {
    const flake: Dot = [between(rng, -20, 20), between(rng, -15, 15), between(rng, 0.3, 0.8)];
    if (flakes.every(([x, y, r]) => Math.hypot(x - flake[0], y - flake[1]) > (r + flake[2]) * 2.4 + 0.3)) flakes.push(flake);
  }
  const sparkling = whole(rng, 2, 4);
  const groups: string[][] = [[], [], []];
  flakes.forEach(([x, y, r], k) => {
    const d = k < sparkling ? sparkle(f, x, y, r * 2.4) : f.circle(x, y, r);
    groups[r > 0.62 ? 0 : r > 0.45 ? 1 : 2]?.push(d);
  });
  const opacity = [0.95, 0.75, 0.5];
  return groups.flatMap((group, k) => (group.length > 0 ? [shape(group.join(""), "white", opacity[k] ?? 0.5, MAIN)] : []));
}

/** Digital static over a field about 22 wide and 14 tall centred on the anchor: offset bars and blocks in cyan, purple and white. */
function glitch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rowHeight = 1.27;
  const rows = 11;
  const rowY = (row: number): number => -7 + row * rowHeight;
  const shuffled = Array.from({ length: rows }, (_, k) => k);
  for (let k = rows - 1; k > 0; k -= 1) {
    const swap = whole(rng, 0, k);
    [shuffled[k], shuffled[swap]] = [shuffled[swap] ?? 0, shuffled[k] ?? 0];
  }
  const bar = (row: number, x: number, length: number, h: number): string => {
    const y = rowY(row) + (rowHeight - h) / 2;
    return f.poly([[x, y], [x + length, y], [x + length, y + h], [x, y + h]]);
  };
  const cyanRows = shuffled.slice(0, 4);
  const cyan = cyanRows.map((row) => [row, between(rng, -11, 3), between(rng, 3, 8), between(rng, 0.45, 1)] as const);
  const purple = [...cyan.slice(0, 2).map(([row, x, length, h]) => [row, x + 0.9, length, h] as const), ...shuffled.slice(4, 6).map((row) => [row, between(rng, -11, 4), between(rng, 2.5, 7), between(rng, 0.4, 0.9)] as const)];
  const white = shuffled.slice(6, 9).map((row) => [row, between(rng, -11, 6), between(rng, 1.5, 5), between(rng, 0.3, 0.7)] as const);
  const blocks: string[] = [];
  const used = new Set<number>();
  for (let k = 0; k < 5; k += 1) {
    const cell = whole(rng, 0, rows * 10 - 1);
    if (used.has(cell)) continue;
    used.add(cell);
    const size = between(rng, 0.7, 1.2);
    blocks.push(bar(cell % rows, -11 + Math.floor(cell / rows) * 2.2, size, size));
  }
  const draw = (list: readonly (readonly [number, number, number, number])[]): string => list.map(([row, x, length, h]) => bar(row, x, Math.min(length, 11 - x), h)).join("");
  return [
    shape(draw(cyan), "cyan-400", 0.6, MAIN),
    shape(draw(purple), "purple-400", 0.55),
    shape(draw(white), "white", 0.7),
    shape(blocks.length > 0 ? blocks.join("") : bar(0, 0, 1, 1), "white", 0.35),
  ];
}

/** Bubbles rising over a field about 10 wide and 16 tall centred on the anchor, growing as they rise: rings and glints. */
function bubbles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const found: Dot[] = [];
  const phase = rng() * Math.PI * 2;
  for (let tries = 0; found.length < whole(rng, 5, 8) && tries < 60; tries += 1) {
    const y = between(rng, -7, 7);
    const r = (0.45 + ((7 - y) / 14) * 1.05) * between(rng, 0.8, 1.15);
    const x = Math.sin(y * 0.45 + phase) * 2 + between(rng, -2.4, 2.4);
    const bubble: Dot = [Math.max(-5 + r, Math.min(5 - r, x)), y, r];
    if (found.every(([bx, by, br]) => Math.hypot(bx - bubble[0], by - bubble[1]) > br + bubble[2] + 0.25)) found.push(bubble);
  }
  return [
    shape(found.map(([x, y, r]) => f.circle(x, y, r * 0.82)).join(""), "white", 0.1),
    shape(found.map(([x, y, r]) => f.circle(x, y, r) + f.circle(x, y, r * 0.82)).join(""), "sky-300", 0.8, MAIN),
    shape(found.map(([x, y, r]) => f.circle(x - r * 0.4, y - r * 0.4, r * 0.18)).join(""), "white", 0.9),
  ];
}

/** Billowing smoke rising from the anchor: three to five puffs growing and fading upward to about 18 tall and 8 wide at scale 1. */
function smoke(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const drift = between(rng, -0.8, 0.8);
  const out: KitShape[] = [];
  const lights: string[] = [];
  for (let k = 0; k < count; k += 1) {
    const t = k / (count - 1);
    const r = 1.5 + 1.9 * t;
    const x = drift * k * 1.2 + between(rng, -0.5, 0.5);
    const y = -(1.6 + 12.7 * t);
    out.push(shape(blob(f, rng, x, y, r, 8), "slate-400", 0.62 - 0.36 * t, MAIN));
    if (k > 0 && k % 2 === 1) lights.push(f.circle(x - r * 0.4, y - r * 0.35, r * 0.4));
  }
  if (lights.length > 0) out.push(shape(lights.join(""), "white", 0.12));
  return out;
}

/** Falling rain over a field about 40 wide and 30 tall centred on the anchor: slanted thin streaks. */
function rain(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slant = between(rng, 0.18, 0.38);
  const near: string[] = [];
  const far: string[] = [];
  for (let col = 0; col < 8; col += 1) {
    for (let row = 0; row < 4; row += 1) {
      if (rng() < 0.25) continue;
      const x = -17.5 + col * 5 + between(rng, -1.5, 1.5);
      const y = -15 + row * 7.5 + between(rng, 0, 2);
      const length = between(rng, 3, 5);
      const w = 0.28;
      const streak = f.poly([[x, y], [x + w, y], [x + w - slant * length, y + length], [x - slant * length, y + length]]);
      ((col + row) % 2 === 0 ? near : far).push(streak);
    }
  }
  const out: KitShape[] = [];
  if (near.length > 0) out.push(shape(near.join(""), "blue-200", 0.6, MAIN));
  if (far.length > 0) out.push(shape(far.join(""), "blue-200", 0.35, MAIN));
  return out.length > 0 ? out : [shape(f.poly([[0, 0], [0.28, 0], [0.28 - slant * 4, 4], [-slant * 4, 4]]), "blue-200", 0.6, MAIN)];
}

/** A low cloud of dust on the ground about the anchor, about 22 wide and 8 tall at scale 1: translucent puffs and specks. */
function dust(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 4, 6);
  const out: KitShape[] = [];
  for (let k = 0; k < count; k += 1) {
    const t = (2 * k) / (count - 1) - 1;
    const r = (4 - Math.abs(t) * 1.4) * between(rng, 0.85, 1.1);
    out.push(shape(blob(f, rng, t * 8 + between(rng, -0.6, 0.6), -r * 0.72, r, 8), "stone-500", between(rng, 0.2, 0.32), MAIN));
  }
  const specks = scatter(rng, whole(rng, 6, 10), 0, -3.6, 11, 3.8, 0.15, 0.32, 0.3).map(([x, y, r]) => f.poly([[x - r, y], [x, y - r], [x + r, y], [x, y + r]]));
  out.push(shape(f.ellipse(-2, -4.6, 5, 1.6), "white", 0.1));
  out.push(shape(specks.join(""), "wood-700", 0.7));
  return out;
}

/** A sun centred on the anchor: a disc about 10 across, a white core, a ring of short rays (to about 8.6 out) and a translucent halo (to 12 out) at scale 1. */
function sun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rays = whole(rng, 10, 14);
  const start = rng() * (FULL_TURN / rays);
  const spread = (FULL_TURN / rays) * 0.22;
  const ray: string[] = [];
  for (let k = 0; k < rays; k += 1) {
    const angle = start + (k * FULL_TURN) / rays;
    const reach = k % 2 === 0 ? 8.6 : 7.4;
    ray.push(f.poly([onEllipse(0, 0, 5.8, 5.8, angle - spread), onEllipse(0, 0, reach, reach, angle), onEllipse(0, 0, 5.8, 5.8, angle + spread)]));
  }
  return [
    shape(f.circle(0, 0, 12), "orange-400", 0.1, MAIN),
    shape(f.circle(0, 0, 8.4), "orange-400", 0.16, MAIN),
    shape(ray.join(""), "orange-400", 0.75, MAIN),
    shape(f.circle(0, 0, 5), "orange-400", 1, MAIN),
    shape(f.circle(-0.6, -0.6, 3.2), "white", 0.45),
    shape(f.circle(-0.8, -0.8, 1.7), "white", 0.8),
  ];
}

/** Petals or leaves falling over a field about 30 wide and 20 tall centred on the anchor: six to nine small curled petals. */
function petals(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spots = scatter(rng, whole(rng, 6, 9), 0, 0, 15, 10, 1, 1.1, 0.4);
  const near: string[] = [];
  const far: string[] = [];
  spots.forEach(([x, y], k) => {
    const angle = rng() * FULL_TURN;
    const length = between(rng, 1.4, 2);
    const width = between(rng, 0.5, 0.75) * between(rng, 0.45, 1);
    const [ux, uy] = turned([length / 2, 0], angle);
    const [nx, ny] = turned([0, width], angle);
    const d = `M${f.p(x - ux, y - uy)}Q${f.p(x + nx * 1.6, y + ny * 1.6)} ${f.p(x + ux, y + uy)}Q${f.p(x + nx * 0.3, y + ny * 0.3)} ${f.p(x - ux, y - uy)}Z`;
    (k % 2 === 0 ? near : far).push(d);
  });
  const out = [shape(near.join(""), "red-300", 0.95, MAIN)];
  if (far.length > 0) out.push(shape(far.join(""), "red-300", 0.6, MAIN));
  return out;
}

/** R3390000: the nature parts the Classic+ scenes added, by id, each with its kind. */
export const NATURE_KIT = {
  fern: { category: "plants", draw: fern },
  flowers: { category: "plants", draw: flowers },
  "fruit-tree": { category: "plants", draw: fruitTree },
  "giant-mushroom": { category: "plants", draw: giantMushroom },
  // R3396000: the Alt art scenes' bamboo, cactus and fox.
  bamboo: { category: "plants", draw: bamboo },
  cactus: { category: "plants", draw: cactus },
  fox: { category: "creatures", draw: fox },
  // R3397000: the Meditative+ and Training Grounds Alt art scenes' goat, deer and duck.
  goat: { category: "creatures", draw: goat },
  deer: { category: "creatures", draw: deer },
  duck: { category: "creatures", draw: duck },
  // R3398000: the Core Alt art scenes' palm and camel.
  palm: { category: "plants", draw: palm },
  camel: { category: "creatures", draw: camel },
  snake: { category: "creatures", draw: snake },
  grub: { category: "creatures", draw: grub },
  boar: { category: "creatures", draw: boar },
  cat: { category: "creatures", draw: cat },
  monkey: { category: "creatures", draw: monkey },
  bee: { category: "creatures", draw: bee },
  ghost: { category: "creatures", draw: ghost },
  bird: { category: "creatures", draw: bird },
  apple: { category: "objects", draw: apple },
  pear: { category: "objects", draw: pear },
  grapes: { category: "objects", draw: grapes },
  papaya: { category: "objects", draw: papaya },
  basket: { category: "objects", draw: basket },
  pancakes: { category: "objects", draw: pancakes },
  // R3397000: the Meditative+ and Training Grounds Alt art scenes' walnuts.
  walnuts: { category: "objects", draw: walnuts },
  shockwave: { category: "weather", draw: shockwave },
  tornado: { category: "weather", draw: tornado },
  flames: { category: "weather", draw: flames },
  fireball: { category: "weather", draw: fireball },
  "light-ribbon": { category: "weather", draw: lightRibbon },
  "thought-bubble": { category: "weather", draw: thoughtBubble },
  spores: { category: "weather", draw: spores },
  snow: { category: "weather", draw: snow },
  glitch: { category: "weather", draw: glitch },
  bubbles: { category: "weather", draw: bubbles },
  smoke: { category: "weather", draw: smoke },
  rain: { category: "weather", draw: rain },
  dust: { category: "weather", draw: dust },
  sun: { category: "weather", draw: sun },
  petals: { category: "weather", draw: petals },
} as const satisfies Readonly<Record<string, KitPart>>;
