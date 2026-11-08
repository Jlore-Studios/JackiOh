// R3401000: the scene kit's ink-wash plants and weather (ART.md, "Scene recipes and kits"), which the Meditative
// scenes (#3401) added: Gatling Pea's whole character (a faceless pea vine in pod armour coiled up a trellis, its
// ringed pods firing peas the way it faces), a lotus on its pads and a branch of plum blossom reaching in from its
// anchor, a colossal glowing tree, a bamboo trellis, and the weather of a scroll: a sumi cloud bank with curled
// edges, a shimmering tear in the air and thought made visible as shapes orbiting a point. The vine, the tree and the
// trellis stand on their anchor (it is their bottom centre); the lotus, the cloud, the tear and the shapes centre on
// it; the branch grows out from it. sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css name,
// none in Radiant's gold window; a glow or a wash is a stack of translucent shapes, never a gradient; shapes are
// filled even-odd, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, sparkle, whole, type Local, type Point } from "./kitDraw.ts";
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

/** A band along the points that narrows from `w0` wide at the first to `w1` at the last: a stem, a twig, a tail. */
function taper(f: Local, points: readonly Point[], w0: number, w1: number): string {
  const left: Point[] = [];
  const right: Point[] = [];
  const n = points.length;
  for (let i = 0; i < n; i += 1) {
    const [px, py] = points[Math.max(0, i - 1)] ?? [0, 0];
    const [nx, ny] = points[Math.min(n - 1, i + 1)] ?? [0, 0];
    const [x, y] = points[i] ?? [0, 0];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (w0 + ((w1 - w0) * i) / Math.max(1, n - 1)) / 2;
    const ox = (-(ny - py) / length) * half;
    const oy = ((nx - px) / length) * half;
    left.push([x + ox, y + oy]);
    right.push([x - ox, y - oy]);
  }
  return f.poly([...left, ...right.reverse()]);
}

/** A pointed lens from `a` to `b`, `half` wide at its middle, bowed `bow` across (a leaf, a petal, a tear). */
function lens(f: Local, a: Point, b: Point, half: number, bow = 0): string {
  const mx = (a[0] + b[0]) / 2;
  const my = (a[1] + b[1]) / 2;
  const length = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  const nx = -(b[1] - a[1]) / length;
  const ny = (b[0] - a[0]) / length;
  const side = (k: number): string => f.p(mx + nx * (k * 2 * half + bow), my + ny * (k * 2 * half + bow));
  return `M${f.p(...a)}Q${side(1)} ${f.p(...b)}Q${side(-1)} ${f.p(...a)}Z`;
}

/** A five-petalled blossom about (cx, cy), `r` its reach, turned `turn` degrees. */
function flower(f: Local, cx: number, cy: number, r: number, turn: number): string {
  const at = (k: number, reach: number): string => {
    const angle = (turn + k * 72) * DEGREES;
    return f.p(cx + reach * Math.cos(angle), cy + reach * Math.sin(angle));
  };
  let d = `M${at(-0.5, r * 0.35)}`;
  for (let k = 0; k < 5; k += 1) d += `Q${at(k - 0.4, r * 1.2)} ${at(k, r * 1.05)}Q${at(k + 0.4, r * 1.2)} ${at(k + 0.5, r * 0.35)}`;
  return `${d}Z`;
}

/** A tapering curl about (cx, cy), from `r0` out to `r1`, `sweep` degrees round from `start`: a cloud's scroll. */
function curl(f: Local, cx: number, cy: number, start: number, sweep: number, r0: number, r1: number, width: number): string {
  const steps = 12;
  const outer: Point[] = [];
  const inner: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const r = r0 + (r1 - r0) * t;
    const half = (width * (0.35 + 0.65 * t)) / 2;
    const angle = (start + sweep * t) * DEGREES;
    outer.push([cx + (r + half) * Math.cos(angle), cy + (r + half) * Math.sin(angle)]);
    inner.push([cx + Math.max(0.2, r - half) * Math.cos(angle), cy + Math.max(0.2, r - half) * Math.sin(angle)]);
  }
  return f.poly([...outer, ...inner.reverse()]);
}

/** A lily pad about (cx, cy): an ellipse with a narrow wedge cut from its rim at `notch` degrees (0 the way it faces). */
function pad(f: Local, cx: number, cy: number, rx: number, ry: number, notch: number): string {
  const rim = (degrees: number): string => f.p(cx + rx * Math.cos(degrees * DEGREES), cy + ry * Math.sin(degrees * DEGREES));
  return `M${f.p(cx, cy)}L${rim(notch + 16)}A${fmt(f.r(rx))} ${fmt(f.r(ry))} 0 1 ${f.sweep(1)} ${rim(notch + 344)}Z`;
}

/** A thin ring about (cx, cy): an ellipse less a smaller one (a hole), `width` its band. */
function ring(f: Local, cx: number, cy: number, rx: number, ry: number, width: number): string {
  return f.ellipse(cx, cy, rx, ry) + f.ellipse(cx, cy, Math.max(0.1, rx - width), Math.max(0.1, ry - width));
}

/** A thin outline of a regular polygon about (cx, cy): the polygon less a smaller one (a hole). */
function outline(f: Local, cx: number, cy: number, sides: number, r: number, turn: number, width: number): string {
  const corner = (k: number, reach: number): Point => [cx + reach * Math.cos((turn + (k * 360) / sides) * DEGREES), cy + reach * Math.sin((turn + (k * 360) / sides) * DEGREES)];
  const inner = r - width / Math.cos(Math.PI / sides);
  const points = (reach: number): Point[] => Array.from({ length: sides }, (_, k) => corner(k, reach));
  return f.poly(points(r)) + f.poly(points(Math.max(0.1, inner)));
}

// Plants

/** How the pea vine's stem winds: its point at `t` (0 at the ground, 1 at its crown) round a trellis behind it. */
function vineAt(t: number): Point {
  return [3.2 * Math.sin(t * 2.6 * Math.PI) * (1 - 0.3 * t), -31 * t];
}

/** Gatling Pea: a thick, faceless pea vine (main, green-700) coiled up a small trellis on the anchor, three pods ringed like gun barrels pointing the way it faces and firing hard peas; about 26 wide and 34 tall at scale 1. */
function peaVine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stem: Point[] = Array.from({ length: 13 }, (_, i) => vineAt(i / 12));
  const leaves = [0.12, 0.45, 0.78].map((t) => {
    const [x, y] = vineAt(t);
    return lens(f, [x - 0.8, y], [x - 6.5, y - between(rng, 1.5, 3)], 1.5, -0.6);
  });
  const pods = [-9, -18, -26].map((y, k) => {
    const [sx] = vineAt(-y / 31);
    const x0 = sx + 1.2;
    const x1 = x0 + between(rng, 8, 9.5) - k * 0.6;
    return { y, x0, x1 };
  });
  const capsule = ({ y, x0, x1 }: { y: number; x0: number; x1: number }): string =>
    `M${f.p(x0, y - 1.6)}L${f.p(x1, y - 1.6)}Q${f.p(x1 + 1.6, y)} ${f.p(x1, y + 1.6)}L${f.p(x0, y + 1.6)}Q${f.p(x0 - 1, y)} ${f.p(x0, y - 1.6)}Z`;
  const rings = pods.flatMap(({ y, x0 }) => [2.4, 4.4, 6.4].map((dx) => f.poly([[x0 + dx, y - 1.75], [x0 + dx + 0.55, y - 1.75], [x0 + dx + 0.55, y + 1.75], [x0 + dx, y + 1.75]])));
  const peas: Point[] = pods.flatMap(({ y, x1 }) => [
    [x1 + between(rng, 3.4, 4.4), y + between(rng, -0.6, 0.6)],
    [x1 + between(rng, 6.6, 8.2), y + between(rng, -1.2, 1.2)],
  ]);
  const [vx, vy] = vineAt(1);
  return [
    shape(f.band([[-6, 0], [-6, -30]], 0.7) + f.band([[2.2, 0], [2.2, -30]], 0.7), "wood-700", 0.9),
    shape([-7.5, -15.5, -23.5].map((y) => f.band([[-8.4, y], [-6.6, y]], 0.6) + f.band([[-5.4, y], [1.4, y]], 0.6) + f.band([[3, y], [4.4, y]], 0.6)).join(""), "wood-700", 0.9),
    shape(leaves.join(""), "green-500", 1, INKED),
    shape(taper(f, stem, 3.8, 1.8), "green-700", 1, MAIN_INKED),
    shape(f.ellipse(vx + 0.6, vy - 2.6, 2.4, 3.2), "lime-400", 1, INKED),
    shape(pods.map(capsule).join(""), "lime-400", 1, INKED),
    shape(rings.join(""), "green-900", 0.9),
    shape(pods.map(({ y, x1 }) => f.circle(x1 + 0.5, y, 0.85)).join(""), "slate-900", 1),
    shape(pods.map(({ y, x1 }) => sparkle(f, x1 + 2.1, y, 1.3)).join(""), "lime-200", 0.85),
    shape(peas.map(([x, y]) => f.circle(x, y, 1.4)).join(""), "green-400", 1),
    shape(peas.map(([x, y]) => f.circle(x - 0.45, y - 0.5, 0.5)).join(""), "white", 0.75),
    shape(peas.map(([x, y]) => f.band([[x - 4, y + 0.1], [x - 1.5, y]], 0.35)).join(""), "white", 0.5),
  ];
}

/** A lotus centred on the anchor: a pink flower (main, red-300) open on the water among two or three notched pads, a ripple and a soft glow round it; about 14 wide and 7 tall at scale 1. */
function lotus(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const third = rng() < 0.6;
  const pads =
    pad(f, -5, 0.8, 3, 1.05, between(rng, 60, 120)) +
    pad(f, 4.6, 1.1, 2.7, 0.95, between(rng, 200, 260)) +
    (third ? pad(f, 0.6, 2.9, 2.1, 0.7, between(rng, 280, 320)) : "");
  const back = [-62, -90, -118].map((angle) => {
    const tip: Point = [4.8 * Math.cos(angle * DEGREES), -0.6 + 4.8 * Math.sin(angle * DEGREES)];
    return lens(f, [0, -0.4], tip, 1.25);
  });
  const front = [-30, -150].map((angle) => lens(f, [0, -0.2], [4.4 * Math.cos(angle * DEGREES), 0.2 + 2.6 * Math.sin(angle * DEGREES)], 1.1, 0));
  return [
    shape(ring(f, 0, 0.9, 7.4, 2.1, 0.3), "white", 0.45),
    shape(pads, "green-700", 1, INKED),
    shape(f.ellipse(-5.6, 0.5, 1.4, 0.35) + f.ellipse(4.1, 0.8, 1.2, 0.3), "green-400", 0.55),
    shape(f.circle(0, -2.2, 5.2), "white", 0.2),
    shape(back.join(""), "red-300", 1, MAIN_INKED),
    shape(front.join(""), "white", 1, INKED),
    shape(f.ellipse(0, -1.3, 1.1, 0.6), "lime-200", 1),
  ];
}

/** A branch of plum blossom growing out from the anchor the way it faces: an ink twig with side twigs, pink (main, red-300) and white blossoms with red hearts, buds and two falling petals; about 30 long and 12 tall at scale 1. */
function blossomBranch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const kink = between(rng, -1, 1);
  const branch: Point[] = [[0, 0], [7, -2 + kink], [13, -1.2], [19.5, -4.6 + kink], [25.5, -4], [30, -6.8]];
  const twigs = [
    taper(f, [[7, -2 + kink], [9.5, -5.5], [10.8, -8.6]], 1.1, 0.3),
    taper(f, [[19.5, -4.6 + kink], [21, -1.6], [23.8, 0.2]], 0.9, 0.25),
  ];
  const pinks: Point[] = [[9.6, -7.2], [16, -2.6 + kink], [24.2, -5.6], [29.4, -8.6]];
  const whites: Point[] = [[4.4, -2.6], [12.2, 1.2], [21.6, -1.2]];
  const flowers = (spots: readonly Point[], r: number): string => spots.map(([x, y]) => flower(f, x, y, r * between(rng, 0.85, 1.1), between(rng, 0, 72))).join("");
  const buds: Point[] = [[11.2, -9.6], [25, 0.8], [18.2, -6.4], [27.6, -3.2]];
  return [
    shape(taper(f, branch, 2.2, 0.4), "slate-900", 1),
    shape(twigs.join(""), "slate-900", 1),
    shape(flowers(pinks, 1.7), "red-300", 1, MAIN_INKED),
    shape(flowers(whites, 1.5), "white", 1, INKED),
    shape([...pinks, ...whites].map(([x, y]) => f.circle(x, y, 0.42)).join(""), "red-600", 1),
    shape(buds.map(([x, y]) => f.circle(x, y, 0.55)).join(""), "red-600", 0.9),
    shape(f.ellipse(14 + between(rng, -2, 2), 5, 0.7, 0.35) + f.ellipse(22 + between(rng, -2, 2), 3.6, 0.6, 0.3), "red-300", 0.8),
  ];
}

/** A colossal gnarled tree standing on the anchor: roots spreading wide, a dark trunk and boughs, a huge canopy of leaf masses (main, green-700) glowing jade over a jade glow at its roots, lit leaves and motes; about 66 wide and 64 tall at scale 1. */
function greatTree(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -1, 1);
  const trunk: Point[] = [
    [-31, 0.4],
    [-21, -1.4],
    [-13, -4],
    [-8.5, -10],
    [-7.5, -20],
    [-10, -28],
    [-20, -37 + j()],
    [-17, -39],
    [-6, -33],
    [-3, -42],
    [0, -45],
    [3, -42],
    [6, -34],
    [18, -40 + j()],
    [21, -38],
    [8.5, -28],
    [7.5, -19],
    [9.5, -10],
    [14, -4],
    [22, -1.4],
    [31, 0.4],
    [18, 1.2],
    [8, -0.4],
    [0, 1.2],
    [-8, -0.2],
    [-18, 1.2],
  ];
  const mass = (cx: number, cy: number, rx: number, ry: number, lobes: number): string => {
    const points: Point[] = [];
    for (let i = 0; i < lobes * 2; i += 1) {
      const angle = (i * 180) / lobes + between(rng, -6, 6);
      const reach = i % 2 === 0 ? between(rng, 1.02, 1.16) : between(rng, 0.8, 0.88);
      points.push([cx + rx * reach * Math.cos(angle * DEGREES), cy + ry * reach * Math.sin(angle * DEGREES)]);
    }
    return smooth(f, points);
  };
  const lit: Point[] = [[-20, -48], [-9, -40], [6, -58], [17, -50], [25, -42], [-3, -54]];
  const motes = Array.from({ length: 5 }, (_, k) => sparkle(f, -26 + k * 13 + between(rng, -3, 3), -24 - between(rng, 0, 34), between(rng, 0.9, 1.6)));
  return [
    shape(f.ellipse(0, -44, 36, 23), "jade-400", 0.2),
    shape(mass(0, -45, 31, 16, 9), "green-900", 1, INKED),
    shape(smooth(f, trunk), "wood-800", 1, INKED),
    shape(f.band([[-4, -2], [-3, -14], [-5, -26]], 0.7) + f.band([[3.5, -4], [2.6, -17], [4, -27]], 0.6), "wood-950", 0.6),
    shape(mass(-17, -40, 13.5, 8.5, 7), "green-700", 1, MAIN_INKED),
    shape(mass(16, -42, 14, 8.5, 7), "green-700", 1, MAIN_INKED),
    shape(mass(1, -53, 14.5, 8.5, 7), "green-700", 1, MAIN_INKED),
    shape(lit.map(([x, y]) => f.ellipse(x + j(), y + j(), 2.6, 1.3)).join(""), "green-400", 0.75),
    shape(f.ellipse(0, 0.4, 27, 2.2), "jade-400", 0.3),
    shape(motes.join(""), "lime-200", 0.9),
  ];
}

// Architecture

/** A bamboo trellis standing on the anchor: three poles (main, wood-600) with dark nodes, top and foot rails, a diamond lattice and a green tendril climbing it; about 14 wide and 22 tall at scale 1. */
function trellis(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const posts = [-6.4, 0, 6.4];
  const rise: Point[] = [];
  const fall: Point[] = [];
  const diagonals = (dir: 1 | -1): string =>
    [-12, -6, 0, 6, 12]
      .map((offset) => {
        const a: Point = [offset - 6 * dir, -2.5];
        const b: Point = [offset + 6 * dir, -19.5];
        const clip = (p: Point, q: Point): Point => {
          const t = p[0] < -6.4 ? (-6.4 - p[0]) / (q[0] - p[0]) : p[0] > 6.4 ? (6.4 - p[0]) / (q[0] - p[0]) : 0;
          return [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
        };
        const from = clip(a, b);
        const to = clip(b, a);
        return Math.abs(from[1] - to[1]) > 1 ? f.band([from, to], 0.45) : "";
      })
      .join("");
  rise.push([0, 0]);
  fall.push([0, 0]);
  const leafAt = (x: number, y: number, k: number): string => lens(f, [x, y], [x + 2.4 * k, y - 1.4], 0.8);
  const sway = between(rng, -0.8, 0.8);
  return [
    shape(posts.map((x) => f.band([[x, 0.4], [x + sway * 0.2, -22]], 1)).join(""), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-7.6, -20.6], [7.6, -20.6]], 0.8) + f.band([[-7.6, -2], [7.6, -2]], 0.8), "wood-700", 1),
    shape(diagonals(1), "wood-800", 0.9),
    shape(diagonals(-1), "wood-800", 0.9),
    shape(posts.flatMap((x) => [-7, -14].map((y) => f.band([[x - 0.6, y], [x + 0.6, y]], 0.4))).join(""), "slate-900", 0.8),
    shape(f.band([[-3, 0], [-2 + sway, -5], [-4, -10], [-1.6 + sway, -15], [-3.4, -19]], 0.45), "green-700", 1),
    shape(leafAt(-2 + sway, -5, 1) + leafAt(-4, -10, -1) + leafAt(-1.6 + sway, -15, 1), "green-500", 1, INKED),
  ];
}

// Weather

/** A sumi-ink cloud bank centred on the anchor: a soft wash (main, slate-400) with a flat foot, a darker under-wash, curled scroll edges, pale tops and trailing wisps; about 36 wide and 12 tall at scale 1 (44 with its wisps). */
function inkCloud(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = (): number => between(rng, -1, 1);
  const body =
    `M${f.p(-18, 2.5)}Q${f.p(-19.5, -1)} ${f.p(-15, -2 + h())}Q${f.p(-14, -6)} ${f.p(-9, -5.5 + h())}Q${f.p(-7, -10)} ${f.p(-1, -8.5 + h())}` +
    `Q${f.p(3, -11)} ${f.p(7, -7 + h())}Q${f.p(11, -8.5)} ${f.p(13, -4.5 + h())}Q${f.p(18, -4)} ${f.p(18.5, 0)}Q${f.p(19.5, 2.5)} ${f.p(16, 2.5)}` +
    `Q${f.p(0, 3.8)} ${f.p(-18, 2.5)}Z`;
  const turn = between(rng, 0, 90);
  return [
    shape(body, "slate-400", 0.5, MAIN),
    shape(`M${f.p(-15, 2.4)}Q${f.p(0, -1.6)} ${f.p(15, 2.4)}Q${f.p(0, 4.4)} ${f.p(-15, 2.4)}Z`, "slate-700", 0.28),
    shape(curl(f, -12, -2.6, turn, 400, 0.4, 2.9, 1) + curl(f, 2, -5.6, turn + 120, 380, 0.4, 3.1, 1) + curl(f, 13.6, -1.4, turn + 240, 360, 0.4, 2.5, 0.9), "slate-700", 0.5),
    shape(f.ellipse(-8.6, -4.6, 2.8, 0.7) + f.ellipse(-0.8, -7.6, 3.2, 0.8) + f.ellipse(7.4, -6, 2.4, 0.6), "white", 0.35),
    shape(taper(f, [[-17, 1.6], [-20, 2.2], [-23, 1.8]], 1.2, 0.2) + taper(f, [[17.4, 1.6], [20.4, 0.8], [22.6, 1.2]], 1, 0.2), "slate-400", 0.4),
  ];
}

/** A tall shimmering tear folded open in the air, centred on the anchor: violet glows (main, violet-500), a deep violet opening with bright curled edges and a white seam of light, sparkles and motes; about 14 wide and 36 tall at scale 1. */
function rift(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top: Point = [0, -18];
  const foot: Point = [0, 18];
  const seam: Point[] = [top, [between(rng, 0.3, 0.8), -10], [between(rng, -0.8, -0.3), -3], [between(rng, 0.3, 0.8), 4], [between(rng, -0.7, -0.2), 11], foot];
  const sparkles = Array.from({ length: 4 }, (_, k) => sparkle(f, (k % 2 === 0 ? -1 : 1) * between(rng, 4.6, 6.8), -14 + k * 9 + between(rng, -2, 2), between(rng, 0.9, 1.6)));
  const motes = Array.from({ length: 5 }, (_, k) => f.circle((k % 2 === 0 ? 1 : -1) * between(rng, 2.6, 6), -16 + k * 8 + between(rng, -1.5, 1.5), between(rng, 0.25, 0.45)));
  return [
    shape(lens(f, [0, -20], [0, 20], 7), "violet-500", 0.22, MAIN),
    shape(lens(f, top, foot, 4.2), "violet-300", 0.4),
    shape(lens(f, [0, -17], [0, 17], 2.6), "violet-700", 0.85),
    shape(lens(f, [-0.6, -14], [-2.6, 7], 1, -1.6) + lens(f, [0.6, -6], [2.4, 14], 1, 1.6), "violet-300", 0.65),
    shape(lens(f, [0, -15], [0, 15], 1.1), "violet-300", 0.6),
    shape(f.band(seam, 0.45), "white", 0.9),
    shape(sparkles.join(""), "white", 0.9),
    shape(motes.join(""), "violet-300", 0.75),
  ];
}

/** Thought made visible, orbiting the anchor: a faint elliptical orbit and on it glowing outlines (cyan-400 circles, white triangles, jade-400 squares), each in a soft glow, and a few sparkles; about 34 wide and 20 tall at scale 1. */
function thoughtShapes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 7, 8);
  const start = between(rng, 0, 360 / count);
  const spots = Array.from({ length: count }, (_, k) => {
    const angle = (start + (k * 360) / count) * DEGREES;
    const depth = (Math.sin(angle) + 1) / 2;
    return { x: 15 * Math.cos(angle), y: 7.5 * Math.sin(angle), r: 1.5 + depth * 0.9, turn: between(rng, 0, 90) };
  });
  const kind = (k: number): readonly (typeof spots)[number][] => spots.filter((_, i) => i % 3 === k);
  return [
    shape(ring(f, 0, 0, 15, 7.5, 0.22), "white", 0.3),
    shape(spots.map(({ x, y, r }) => f.circle(x, y, r + 0.7)).join(""), "cyan-400", 0.18),
    shape(kind(0).map(({ x, y, r }) => ring(f, x, y, r, r, 0.38)).join(""), "cyan-400", 0.9),
    shape(kind(1).map(({ x, y, r, turn }) => outline(f, x, y, 3, r * 1.1, turn - 90, 0.38)).join(""), "white", 0.9),
    shape(kind(2).map(({ x, y, r, turn }) => outline(f, x, y, 4, r * 1.05, turn, 0.38)).join(""), "jade-400", 0.9),
    shape(sparkle(f, between(rng, -6, -3), -1.5, 1) + sparkle(f, between(rng, 3, 6), 1.8, 0.9), "white", 0.8),
  ];
}

export const INK_NATURE_KIT = {
  "pea-vine": { category: "creatures", draw: peaVine },
  lotus: { category: "plants", draw: lotus },
  "blossom-branch": { category: "plants", draw: blossomBranch },
  "great-tree": { category: "plants", draw: greatTree },
  trellis: { category: "architecture", draw: trellis },
  "ink-cloud": { category: "weather", draw: inkCloud },
  rift: { category: "weather", draw: rift },
  "thought-shapes": { category: "weather", draw: thoughtShapes },
} as const satisfies Readonly<Record<string, KitPart>>;
