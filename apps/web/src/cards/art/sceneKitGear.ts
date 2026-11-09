// R3403000: the scene kit's gear in space (ART.md, "Scene recipes and kits"), which Stinkyscape's later cards
// (#3403) added: the devices and props of its stations and ships (a console, a diagram of light, a cable, ore,
// wreckage, a credit chip, a roulette table, a capsule, a kicking sack, loose pages, a blueprint, a reset lever, a
// pickaxe and lunchbox, a mining drill, a chaos egg, a game board, a cracker, a price chart, an AI's light ring) and
// the energies between them (a beam of light, magic bolts, a laser, an energy wall, a tractor beam, an engine's
// plume, a jump flash, a thread of shadow). sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css
// name, none in Radiant's gold window; a glow is a stack of translucent shapes, never a gradient; shapes are filled
// even-odd, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { between, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** What a part draws with: its own frame (`local`), or that frame turned about its anchor (`tilted`). */
type Pen = Pick<Local, "p" | "poly" | "circle" | "band">;

/** A part's frame turned `degrees` about its anchor, clockwise on screen as drawn facing right. */
function tilted(f: Local, degrees: number): Pen {
  const T = (dx: number, dy: number): Point => turned([dx, dy], degrees);
  return {
    p: (dx, dy) => f.p(...T(dx, dy)),
    poly: (points) => f.poly(points.map(([dx, dy]) => T(dx, dy))),
    circle: (dx, dy, r) => f.circle(...T(dx, dy), r),
    band: (points, width) => f.band(points.map(([dx, dy]) => T(dx, dy)), width),
  };
}

/** `a` plus `k` times `v`. */
function plus(a: Point, v: Point, k = 1): Point {
  return [a[0] + v[0] * k, a[1] + v[1] * k];
}

/** The unit vector at `degrees`, clockwise from +x on screen as drawn facing right. */
function heading(degrees: number): Point {
  return [Math.cos(degrees * DEGREES), Math.sin(degrees * DEGREES)];
}

/** A point `r` from (`cx`, `cy`) at `degrees`, clockwise from +x on screen as drawn facing right. */
function polar(r: number, degrees: number, cx = 0, cy = 0): Point {
  return plus([cx, cy], heading(degrees), r);
}

/** An upright rectangle from (x0, y0) to (x1, y1). */
function box(f: Pen, x0: number, y0: number, x1: number, y1: number): string {
  return f.poly([
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ]);
}

/** A rectangle from (x0, y0) to (x1, y1), its corners rounded `r`. */
function rounded(f: Pen, x0: number, y0: number, x1: number, y1: number, r: number): string {
  return (
    `M${f.p(x0 + r, y0)}L${f.p(x1 - r, y0)}Q${f.p(x1, y0)} ${f.p(x1, y0 + r)}L${f.p(x1, y1 - r)}Q${f.p(x1, y1)} ${f.p(x1 - r, y1)}` +
    `L${f.p(x0 + r, y1)}Q${f.p(x0, y1)} ${f.p(x0, y1 - r)}L${f.p(x0, y0 + r)}Q${f.p(x0, y0)} ${f.p(x0 + r, y0)}Z`
  );
}

/** An upright ellipse about (cx, cy), drawn as four curves so it turns with its frame. */
function oval(f: Pen, cx: number, cy: number, rx: number, ry: number): string {
  const k = 0.5523;
  const q = (a: number, b: number): string => f.p(cx + rx * a, cy + ry * b);
  return `M${q(1, 0)}C${q(1, k)} ${q(k, 1)} ${q(0, 1)}C${q(-k, 1)} ${q(-1, k)} ${q(-1, 0)}C${q(-1, -k)} ${q(-k, -1)} ${q(0, -1)}C${q(k, -1)} ${q(1, -k)} ${q(1, 0)}Z`;
}

/** Points along the quadratic curve from `a` to `b` drawn toward `c`, in `steps` steps. */
function quad(a: Point, c: Point, b: Point, steps: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const u = 1 - t;
    out.push([u * u * a[0] + 2 * u * t * c[0] + t * t * b[0], u * u * a[1] + 2 * u * t * c[1] + t * t * b[1]]);
  }
  return out;
}

/** The points of a path shifted `d` across it, to the right of its travel on screen (below a path running toward +x). */
function beside(points: readonly Point[], d: number): Point[] {
  return points.map((p, i) => {
    const a = points[Math.max(0, i - 1)] ?? p;
    const b = points[Math.min(points.length - 1, i + 1)] ?? p;
    const length = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
    return [p[0] - ((b[1] - a[1]) / length) * d, p[1] + ((b[0] - a[0]) / length) * d];
  });
}

/** A star of `points` points about `c`, its tips `size` out and its waist `waist` of that, turned `turn` degrees: four points make a sparkle. */
function star(f: Pen, c: Point, size: number, points = 4, turn = 0, waist = 0.28): string {
  const rim: Point[] = [];
  for (let i = 0; i < points * 2; i += 1) rim.push(polar(i % 2 === 0 ? size : size * waist, turn - 90 + (i * 180) / points, ...c));
  return f.poly(rim);
}

/** A gear's toothed rim about `c`: `teeth` teeth reaching `r`, their roots at three quarters of it, turned `spin` degrees. */
function cog(f: Pen, c: Point, r: number, teeth: number, spin: number): string {
  const step = 360 / teeth;
  const rim: Point[] = [];
  for (let i = 0; i < teeth; i += 1) {
    const a = spin + i * step;
    rim.push(polar(r * 0.76, a - step * 0.3, ...c), polar(r, a - step * 0.16, ...c), polar(r, a + step * 0.16, ...c), polar(r * 0.76, a + step * 0.3, ...c));
  }
  return f.poly(rim);
}

/** A band along `points` whose width at each point is `width(t)`, `t` running from 0 at its first point to 1 at its last: a trail, a flame or a tendril that tapers. */
function tapered(f: Pen, points: readonly Point[], width: (t: number) => number): string {
  const last = Math.max(1, points.length - 1);
  const left: Point[] = [];
  const right: Point[] = [];
  points.forEach((p, i) => {
    const a = points[Math.max(0, i - 1)] ?? p;
    const b = points[Math.min(last, i + 1)] ?? p;
    const length = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
    const half = width(i / last) / 2;
    const n: Point = [(-(b[1] - a[1]) / length) * half, ((b[0] - a[0]) / length) * half];
    left.push(plus(p, n));
    right.push(plus(p, n, -1));
  });
  return f.poly([...left, ...right.reverse()]);
}

// Devices and props

/** A control console standing on the anchor: a sloped desk on a pedestal, two or three lit screens, rows of buttons and a joystick; about 20 wide and 14 tall at scale 1. */
function consoleDesk(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const screens = whole(rng, 2, 3);
  const gap = 0.7;
  const w = (15 - gap * (screens - 1)) / screens;
  let glass = "";
  let lines = "";
  for (let k = 0; k < screens; k += 1) {
    const x0 = -7.5 + k * (w + gap);
    glass += box(f, x0, -13.4, x0 + w, -10.8);
    lines += f.band([[x0 + 0.6, -12.5], [x0 + w * between(rng, 0.45, 0.85), -12.5]], 0.35) + f.band([[x0 + 0.6, -11.6], [x0 + w * between(rng, 0.3, 0.7), -11.6]], 0.35);
  }
  const keys: Point[] = [[-8.1, -8.2], [-6.7, -8.2], [-5.3, -8.2], [-3.9, -8.2], [-2.5, -8.2], [-7.6, -9.4], [-6.2, -9.4], [-4.8, -9.4], [-3.4, -9.4], [-0.6, -8.8], [1, -8.8]];
  const offset = whole(rng, 0, 2);
  let red = "";
  let green = "";
  keys.forEach(([x, y], i) => {
    if ((i + offset) % 3 === 0) red += f.circle(x, y, 0.45);
    else green += box(f, x - 0.5, y - 0.3, x + 0.5, y + 0.3);
  });
  const base: Point = [5.6, -8.8];
  const top = plus(base, heading(-90 + between(rng, -30, 30)), 3);
  return [
    shape(f.poly([[-9.8, -9.6], [9.8, -9.6], [9.2, -15.2], [-9.2, -15.2]]), "cyan-400", 0.14),
    shape(
      f.poly([[-6, 0], [6, 0], [6, -0.9], [3.4, -1.4], [3.4, -5.2], [-3.4, -5.2], [-3.4, -1.4], [-6, -0.9]]) + f.ellipse(...base, 1.3, 0.5),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.poly([[-10, -5], [10, -5], [10, -7.4], [8.4, -10.2], [-8.4, -10.2], [-10, -7.4]]), "slate-400", 1, MAIN_INKED),
    shape(box(f, -10, -7.4, 10, -5), "slate-700", 0.45),
    shape(f.poly([[-8.4, -10.2], [8.4, -10.2], [8, -14], [-8, -14]]), "slate-800", 1, INKED),
    shape(glass, "cyan-400", 0.9),
    shape(lines, "white", 0.5),
    shape(red, "red-400", 1),
    shape(green, "green-400", 0.9),
    shape(f.band([base, top], 0.6), "slate-800", 1, INKED),
    shape(f.circle(...top, 0.9), "red-500", 1, INKED),
  ];
}

/** A diagram of light spiralling about the anchor: glowing lines and nodes in a spiral, two or three outlined gears turning on it; about 40 across at scale 1. */
function lightDiagram(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = between(rng, 0, 360);
  const arm = (offset: number): Point[] => {
    const out: Point[] = [];
    for (let i = 0; i <= 20; i += 1) out.push(polar(3 + 0.8 * i, start + offset + 20 * i));
    return out;
  };
  const arms = [arm(0), arm(180)];
  const nodes = arms.flatMap((points) => [7, 13, 20].map((i): Point => points[i] ?? [0, 0]));
  const phase = between(rng, 0, 30);
  let dashes = "";
  for (let a = 0; a < 360; a += 30) dashes += f.band([polar(12.5, phase + a), polar(12.5, phase + a + 9), polar(12.5, phase + a + 18)], 0.4);
  const slots: readonly (readonly [number, number, number, number])[] = [
    [-8.5, -6, 5, 10],
    [9.5, 5, 4, 8],
    [5, -12.5, 3.2, 7],
  ];
  const gears = slots.slice(0, whole(rng, 2, 3)).map(([x, y, r, teeth]) => [[x + between(rng, -0.6, 0.6), y + between(rng, -0.6, 0.6)] as Point, r, teeth, between(rng, 0, 360)] as const);
  return [
    shape(f.circle(0, 0, 21), "cyan-400", 0.08),
    shape(f.circle(0, 0, 11), "cyan-400", 0.1),
    shape(dashes, "sky-300", 0.35),
    shape(arms.map((points) => f.band(points, 0.6)).join(""), "cyan-400", 0.6, MAIN),
    shape(gears.map(([c, r]) => f.circle(...c, r + 1.4)).join(""), "cyan-400", 0.14, MAIN),
    shape(gears.map(([c, r, teeth, spin]) => cog(f, c, r, teeth, spin) + f.circle(...c, r * 0.76 - 0.9)).join(""), "cyan-400", 0.85, MAIN),
    shape(gears.map(([c, r]) => f.circle(...c, r * 0.22)).join(""), "white", 0.8),
    shape(nodes.map((c) => f.circle(...c, 1.5)).join(""), "white", 0.25),
    shape(nodes.map((c) => f.circle(...c, 0.7)).join("") + f.circle(0, 0, 1.2), "white", 0.95),
  ];
}

/** A thick power cable from the anchor curving about 24 toward its facing: a dark sheathed body with bands, a glowing core showing at its torn or plugged end, sparks there. */
function cable(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const end: Point = [24, between(rng, -3, 3)];
  const path = quad([0, 0], [11, between(rng, -8, -3)], end, 10);
  const prev = path[path.length - 2] ?? [0, 0];
  const length = Math.hypot(end[0] - prev[0], end[1] - prev[1]) || 1;
  const u: Point = [(end[0] - prev[0]) / length, (end[1] - prev[1]) / length];
  const n: Point = [-u[1], u[0]];
  const torn = rng() < 0.6;
  const upper = beside(path, -1.6);
  const lower = beside(path, 1.6);
  const glow = plus(end, u, torn ? 0.8 : 2.4);
  const tip = torn
    ? [-1, 0, 1].map((k) => f.band([plus(end, n, k * 0.75), plus(plus(end, u, k === 0 ? 2.8 : 2.2), n, k * 1.6 + between(rng, -0.3, 0.3))], 0.4)).join("")
    : [-1, 1].map((k) => f.band([plus(end, n, k * 0.8), plus(plus(end, u, 2), n, k * 0.8)], 0.55)).join("");
  const out = [
    shape(f.circle(...glow, 4.2), "cyan-400", 0.14),
    shape(f.band(path, 2.6), "slate-800", 1, MAIN_INKED),
    shape(f.band(beside(path, -0.65).slice(1, -1), 0.45), "slate-400", 0.4),
    shape([3, 5, 7].map((i) => f.band([upper[i] ?? end, lower[i] ?? end], 0.7)).join(""), "slate-400", 1, INKED),
  ];
  if (!torn) out.push(shape(f.band([plus(end, u, -2.2), end], 3.4), "slate-700", 1, INKED));
  out.push(
    shape(tip, "brass-500", 1, INKED),
    shape(f.circle(...glow, 2.2), "cyan-400", 0.3),
    shape(torn ? f.circle(...end, 1) : f.circle(...plus(end, u, 2.6), 0.6), "cyan-400", 1),
    shape(star(f, plus(plus(glow, u, 2.2), n, -1.8), 1.7) + star(f, plus(plus(glow, u, 1.4), n, 2), 1.1), "white", 0.9),
  );
  return out;
}

/** A glowing ore chunk centred on the anchor: a faceted rock (slate) laced with a crystal vein (main) that glows, a glint; about 8 across at scale 1. */
function oreChunk(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -40, 40));
  const rim: Point[] = [];
  for (let i = 0; i < 7; i += 1) rim.push(polar(4 * between(rng, 0.82, 1.05), i * (360 / 7) - 90 + between(rng, -8, 8)));
  const corner = (i: number): Point => rim[i % 7] ?? [0, 0];
  const vein = ([[-2.8, 0.5], [-1.3, -0.5], [-0.1, 0.8], [1.3, 0], [2.7, 0.9]] as const).map(([x, y]): Point => [x, y + between(rng, -0.3, 0.3)]);
  const crystals: readonly (readonly [Point, number, number, number])[] = [
    [[-0.9, -2.4], -112 + between(rng, -8, 8), between(rng, 3.2, 4), 0.75],
    [[1.1, -2.2], -62 + between(rng, -8, 8), between(rng, 2.4, 3), 0.6],
  ];
  let prisms = "";
  let facets = "";
  let glint: Point = [0, -5];
  crystals.forEach(([base, degrees, length, half], k) => {
    const u = heading(degrees);
    const n: Point = [-u[1], u[0]];
    const shoulder = plus(base, u, length * 0.7);
    const point = plus(base, u, length);
    prisms += f.poly([plus(base, n, half), plus(shoulder, n, half), point, plus(shoulder, n, -half), plus(base, n, -half)]);
    facets += f.poly([base, plus(base, n, half), plus(shoulder, n, half), point]);
    if (k === 0) glint = plus(point, u, 0.4);
  });
  return [
    shape(f.circle(0, -0.6, 6), "sky-400", 0.14, MAIN),
    shape(f.poly(rim), "slate-700", 1, INKED),
    shape(f.poly([[-0.5, -0.7], corner(5), corner(6), corner(0)]), "slate-400", 0.5),
    shape(f.poly([[0.5, 0.5], corner(2), corner(3), corner(4)]), "slate-900", 0.35),
    shape(f.band(vein, 2), "sky-400", 0.3, MAIN),
    shape(f.band(vein, 0.75), "sky-400", 1, MAIN),
    shape(prisms, "sky-400", 1, MAIN_INKED),
    shape(facets, "white", 0.4),
    shape(star(f, glint, 1.6), "white", 0.9),
  ];
}

/** Tumbling hull wreckage centred on the anchor: three or four torn hull plates with rivets and scorched edges, a bent girder, loose bolts drifting; about 24 across at scale 1. */
function wreckage(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slots: readonly (readonly [number, number, number, number])[] = [
    [-6.5, -1.5, 4, 2.6],
    [6, -3.5, 3.4, 2.2],
    [2.5, 5, 3, 2],
    [-6, 6.5, 2.2, 1.5],
  ];
  let plates = "";
  let seams = "";
  let scorch = "";
  let rivets = "";
  let hot = "";
  slots.slice(0, whole(rng, 3, 4)).forEach(([cx, cy, hw, hh], k) => {
    const turn = between(rng, -35, 35);
    const P = ([x, y]: Point): Point => plus([cx, cy], turned([x, y], turn));
    const edge = ([[hw, -hh], [hw * 0.62, -hh * 0.35], [hw * 1.04, 0.1], [hw * 0.6, hh * 0.5], [hw * 0.92, hh]] as const).map(([x, y]): Point => [x + between(rng, -0.25, 0.25), y]);
    const torn = edge.slice(1, 4);
    plates += f.poly([P([-hw, -hh]), ...edge.map(P), P([-hw, hh])]);
    seams += f.band([P([-hw + 0.9, -hh * 0.1]), P([hw * 0.45, -hh * 0.1])], 0.35);
    scorch += f.band(torn.map(([x, y]) => P([x - 0.45, y])), 0.7);
    rivets += [-hh + 0.6, 0, hh - 0.6].map((y) => f.circle(...P([-hw + 0.55, y]), 0.28)).join("");
    if (k === 0) {
      const [ex, ey] = edge[2] ?? [hw, 0];
      hot = f.band(torn.map(([x, y]) => P([x - 0.12, y])), 0.4) + f.circle(...P([ex + 1.3, ey - 0.6]), 0.3) + f.circle(...P([ex + 2.1, ey + 0.5]), 0.22);
    }
  });
  const bend = between(rng, -15, 15);
  const G = (x: number, y: number): Point => plus([-3, -8.6], turned([x, y], bend));
  const bolts = ([[10, 4.5], [-1.4, 10], [-0.2, -4.6]] as const)
    .map(([x, y]) => {
      const c: Point = [x + between(rng, -0.5, 0.5), y + between(rng, -0.5, 0.5)];
      const spin = between(rng, 0, 60);
      return f.poly([0, 1, 2, 3, 4, 5].map((i) => polar(0.6, spin + i * 60, ...c)));
    })
    .join("");
  return [
    shape(f.band([G(-8, 0.4), G(-1, -0.8), G(4.5, 1.6)], 1.5), "slate-700", 1, INKED),
    shape(f.circle(...G(-5.9, 0.04), 0.38) + f.circle(...G(-3.1, -0.44), 0.38) + f.circle(...G(1.75, 0.4), 0.38), "slate-900", 0.75),
    shape(plates, "slate-400", 1, MAIN_INKED),
    shape(seams, "slate-700", 0.6),
    shape(scorch, "slate-900", 0.45),
    shape(hot, "orange-400", 0.9),
    shape(rivets, "slate-700", 1),
    shape(bolts, "slate-400", 1, INKED),
  ];
}

/** One credit chip centred on the anchor, tilted: a rounded green card (main) with a stripe, a contact square and a glint; about 5 wide at scale 1 (count and spread shower them). */
function creditChip(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -35, 35));
  const digits = between(rng, 1.1, 1.8);
  return [
    shape(rounded(f, -3.3, -2.4, 3.3, 2.4, 1), "green-400", 0.18, MAIN),
    shape(rounded(f, -2.5, -1.6, 2.5, 1.6, 0.5), "green-400", 1, MAIN_INKED),
    shape(f.poly([[-2.5, 0.6], [2.5, 0.6], [2.5, 1.1], [2, 1.6], [-2, 1.6], [-2.5, 1.1]]), "green-700", 0.35),
    shape(box(f, -2.5, -1.05, 2.5, -0.5), "green-900", 0.75),
    shape(rounded(f, -1.9, -0.2, -0.7, 0.8, 0.15), "brass-500", 1, INKED),
    shape(f.band([[-1.9, 0.3], [-0.7, 0.3]], 0.15) + f.band([[-1.3, -0.2], [-1.3, 0.15]], 0.15) + f.band([[-1.3, 0.45], [-1.3, 0.8]], 0.15), "brass-600", 0.9),
    shape(f.band([[0.2, 0.05], [1.9, 0.05]], 0.3) + f.band([[0.2, 0.75], [0.2 + digits, 0.75]], 0.3), "white", 0.55),
    shape(star(f, [1.9, -1.3], 1.1), "white", 0.9),
  ];
}

/** A casino roulette table standing on the anchor in three-quarter view: a felt table (main) on legs, a blank wheel in a wooden bowl with spokes and pockets, a ball rattling; about 36 wide and 12 tall at scale 1. */
function roulette(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spin = between(rng, 0, 360);
  const c: Point = [-7.5, -9.6];
  const on = (rx: number, ry: number, degrees: number): Point => [c[0] + rx * Math.cos(degrees * DEGREES), c[1] + ry * Math.sin(degrees * DEGREES)];
  let pockets = "";
  for (let k = 0; k < 18; k += 2) {
    const a = spin + k * 20;
    pockets += f.poly([on(5.8, 2.15, a), on(5.8, 2.15, a + 10), on(5.8, 2.15, a + 20), on(4.5, 1.65, a + 20), on(4.5, 1.65, a + 10), on(4.5, 1.65, a)]);
  }
  let spokes = f.ellipse(...c, 0.7, 0.45);
  for (let k = 0; k < 4; k += 1) spokes += f.band([on(1.1, 0.4, spin + 45 + k * 90), on(3.6, 1.3, spin + 45 + k * 90)], 0.4);
  // The felt narrows toward the back: its front half-width 16.6 at y -6.9, its back one 14 at y -10.1.
  const felt = (x: number, y: number): Point => [x * (1 - ((-6.9 - y) / 3.2) * 0.157), y];
  let cells = "";
  for (let i = 0; i < 4; i += 1) {
    for (const [y0, y1] of [[-7.5, -8.5], [-8.8, -9.7]] as const) {
      const x0 = 2 + i * 3;
      cells += f.poly([felt(x0, y0), felt(x0 + 2.5, y0), felt(x0 + 2.5, y1), felt(x0, y1)]);
    }
  }
  return [
    shape(f.band([[-15, -5.2], [-15, 0]], 1.2) + f.band([[15, -5.2], [15, 0]], 1.2) + f.band([[-12, -6], [-12, -1.6]], 1) + f.band([[12, -6], [12, -1.6]], 1), "wood-800", 1, INKED),
    shape(f.poly([[-18, -6.4], [18, -6.4], [15, -10.6], [-15, -10.6]]) + box(f, -18, -6.4, 18, -5), "wood-700", 1, INKED),
    shape(f.poly([[-16.6, -6.9], [16.6, -6.9], [14, -10.1], [-14, -10.1]]), "green-700", 1, MAIN_INKED),
    shape(cells, "white", 0.22),
    shape(f.ellipse(c[0], c[1] + 0.6, 7, 2.6), "wood-800", 1, INKED),
    shape(f.ellipse(...c, 7, 2.6), "wood-600", 1, INKED),
    shape(f.ellipse(...c, 5.8, 2.15) + f.ellipse(...c, 4.5, 1.65), "slate-900", 1),
    shape(pockets, "red-500", 1),
    shape(f.ellipse(...c, 4.5, 1.65), "wood-700", 1, INKED),
    shape(spokes, "brass-500", 1),
    shape(f.circle(...on(5.15, 1.9, between(rng, 0, 360)), 0.5), "white", 1),
  ];
}

/** A sealed capsule centred on the anchor, lying along its facing: a pill-shaped body (main) with bands and a seal light, small fins, a soft glow; about 10 long at scale 1. */
function capsule(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -12, 12));
  const lamp = rng() < 0.6 ? "green-400" : "red-400";
  const fin = between(rng, 0.9, 1.5);
  return [
    shape(oval(f, 0, 0, 7.6, 4.4), "cyan-400", 0.1),
    shape(oval(f, 0, 0, 6.2, 3.2), "cyan-400", 0.16),
    shape(f.poly([[-1.8, -2], [-3.4, -2.2 - fin], [-5, -2.2 - fin], [-4.4, -1.2]]) + f.poly([[-1.8, 2], [-3.4, 2.2 + fin], [-5, 2.2 + fin], [-4.4, 1.2]]), "slate-700", 1, INKED),
    shape(rounded(f, -5, -2.2, 5, 2.2, 2.2), "slate-100", 1, MAIN_INKED),
    shape(f.band([[-4, 1.3], [4, 1.3]], 1.1), "slate-400", 0.4),
    shape(f.band([[-2, -2.1], [-2, 2.1]], 0.7) + f.band([[2.4, -2.1], [2.4, 2.1]], 0.7), "slate-700", 1, INKED),
    shape(f.circle(0.2, -0.3, 1.4), lamp, 0.3),
    shape(f.circle(0.2, -0.3, 0.65), lamp, 1),
    shape(f.band([[-4, -1.2], [-3.2, -1.6]], 0.45) + f.band([[-1, -1.6], [1.4, -1.7]], 0.4) + f.band([[3.4, -1.4], [4.2, -0.8]], 0.4), "white", 0.6),
  ];
}

/** A lumpy burlap sack standing on the anchor: a bulging body (main) with stitched seams, a drawstring cinched at its neck, a clawed foot or a boot kicking out of it, dust puffs; about 12 wide and 14 tall at scale 1. */
function kickingSack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bulge = between(rng, -0.8, 0.8);
  const lean = between(rng, -0.06, 0.06);
  const claw = rng() < 0.5;
  const kick = between(rng, -10, 30);
  const L = (x: number, y: number): string => f.p(x - lean * y, y);
  const body =
    `M${L(-4.6, 0)}Q${L(-6.8, -0.6)} ${L(-6.2, -4.4 + bulge)}Q${L(-5.8, -9)} ${L(-1.7, -10.6)}L${L(1.7, -10.6)}Q${L(5.4, -9.2)} ${L(6, -5.2 - bulge)}Q${L(6.8, -0.8)} ${L(4.6, 0)}Z` +
    `M${L(-1.7, -10.6)}Q${L(-3.6, -11.6)} ${L(-3.2, -13.4)}L${L(-1.4, -12.2)}L${L(-0.4, -14)}L${L(0.6, -12.3)}L${L(2.6, -13.6)}Q${L(3.4, -11.6)} ${L(1.7, -10.6)}Z`;
  const shade = `M${L(-4.4, -0.3)}Q${L(-6.2, -0.8)} ${L(-5.8, -4.4 + bulge)}Q${L(-5.4, -8)} ${L(-2.4, -9.6)}Q${L(-3.8, -5)} ${L(-2, -0.3)}Z`;
  let stitches = "";
  for (let i = 0; i < 5; i += 1) {
    const [x, y] = [1.4 + 0.2 * i - lean * (-9 + 1.9 * i), -9 + 1.9 * i];
    stitches += f.band([[x - 0.1, y - 0.5], [x + 0.1, y + 0.5]], 0.35);
  }
  const root: Point = [5.4 - lean * -4.4, -4.4];
  const u = heading(-kick);
  const n: Point = [-u[1], u[0]];
  const knee = plus(root, u, 4.2);
  const foot = claw
    ? f.circle(...plus(knee, u, 0.5), 1.3)
    : f.poly([plus(plus(knee, u, -0.3), n, 1.1), plus(plus(knee, u, 1.3), n, 1.1), plus(plus(knee, u, 1.3), n, -1.4), plus(plus(knee, u, 0.2), n, -2.6), plus(plus(knee, u, -0.3), n, -1.1)]);
  const tips = claw
    ? [-38, 0, 38]
        .map((a) => {
          const d = heading(-kick + a);
          const base = plus(plus(knee, u, 0.5), d, 1.1);
          const side: Point = [-d[1] * 0.35, d[0] * 0.35];
          return f.poly([plus(base, side), plus(base, d, 1.5), plus(base, side, -1)]);
        })
        .join("")
    : f.band([plus(plus(knee, u, 1.45), n, 0.9), plus(plus(knee, u, 1.45), n, -1.2)], 0.4);
  return [
    shape(f.circle(-6.6 + between(rng, -0.2, 0.2), -0.5, 1.2) + f.circle(-8.4, -1.6, 0.7) + f.circle(7, -0.6, 1.1), "stone-500", 0.3),
    shape(body, "brass-500", 1, MAIN_INKED),
    shape(shade, "wood-700", 0.3),
    shape(stitches, "wood-700", 0.8),
    shape(f.band([[-2.1, -10.4], [0, -10.1], [2.1, -10.4]], 0.9) + f.band([[1.5, -9.4], [2.7, -8.5], [2.3, -7.2]], 0.45), "wood-800", 1, INKED),
    shape(f.band([root, knee], 2), claw ? "green-500" : "slate-700", 1, INKED),
    shape(foot, claw ? "green-500" : "slate-900", 1, INKED),
    shape(star(f, root, 1.6, 6, between(rng, 0, 60), 0.7), "brass-500", 1, MAIN_INKED),
    shape(tips, claw ? "white" : "slate-700", 0.9),
    shape(f.band([[-7.4, -9.2], [-8.2, -8.3], [-8, -7.2]], 0.4) + f.band([[-8.6, -6.2], [-9.4, -5.2], [-9.1, -4.1]], 0.4), "white", 0.5),
  ];
}

/** Loose blank pages fluttering about the anchor: four to six sheets (main) tilted every way, a curl on some, faint ruled lines and one stamped seal; about 20 across at scale 1. */
function pages(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cells: Point[] = [[-6.6, -3.4], [0, -3.8], [6.6, -3.2], [-6.2, 3.6], [0.4, 3.2], [6.8, 3.8]];
  const drop = whole(rng, 0, 5);
  const gone = [drop, (drop + 3) % 6].slice(0, 6 - whole(rng, 4, 6));
  // Neighbouring sheets go to different groups, so a group's sheets never overlap.
  const groups = [
    { sheets: "", curls: "", lines: "" },
    { sheets: "", curls: "", lines: "" },
  ];
  let seal = "";
  const [w, h, c] = [1.9, 2.5, 1.1];
  cells.forEach((cell, i) => {
    if (gone.includes(i)) return;
    const centre = plus(cell, [between(rng, -0.4, 0.4), between(rng, -0.4, 0.4)]);
    const turn = between(rng, -50, 50);
    const squash = between(rng, 0.5, 1);
    const S = (x: number, y: number): Point => plus(centre, turned([x * squash, y], turn));
    const group = groups[i % 2] ?? { sheets: "", curls: "", lines: "" };
    if (rng() < 0.5) {
      group.sheets += f.poly([S(-w, -h), S(w, -h), S(w, h - c), S(w - c, h), S(-w, h)]);
      group.curls += f.poly([S(w, h - c), S(w - c, h), S(w - c * 1.1, h - c * 1.1)]);
    } else group.sheets += f.poly([S(-w, -h), S(w, -h), S(w, h), S(-w, h)]);
    for (const y of [-1.4, -0.5, 0.4]) group.lines += f.band([S(-w + 0.5, y), S(w - 0.5, y)], 0.22);
    if (seal === "") {
      const ring = (r: number): string => f.poly([0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((k) => S(...polar(r, k * 36, -0.4, 1.2))));
      seal = ring(0.95) + ring(0.7) + f.band([S(-0.8, 1.2), S(-0.5, 1.5), S(0, 0.9)], 0.22);
    }
  });
  return [
    shape(groups[0]?.sheets ?? "", "white", 1, MAIN_INKED),
    shape(groups[0]?.curls ?? "", "slate-400", 0.8, INKED),
    shape(groups[0]?.lines ?? "", "sky-300", 0.5),
    shape(groups[1]?.sheets ?? "", "slate-100", 1, MAIN_INKED),
    shape(groups[1]?.curls ?? "", "slate-400", 0.8, INKED),
    shape(groups[1]?.lines ?? "", "sky-300", 0.5),
    shape(seal, "red-500", 0.85),
  ].filter((s) => s.d !== "");
}

/** A glowing blueprint lying flat in perspective, centred on the anchor: a blue sheet (main) with a grid, a drawing's outline in light lines, a curled corner, a soft glow; about 20 wide and 8 deep at scale 1. */
function blueprint(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const skew = between(rng, -1.2, 1.2);
  const back = between(rng, 7.2, 8.2);
  /** A point on the sheet: `u` from -1 (left) to 1 (right), `v` from -1 (its far edge) to 1 (its near one). */
  const P = (u: number, v: number): Point => {
    const t = (v + 1) / 2;
    return [u * (back + (10 - back) * t) + skew * (1 - t), v * 4];
  };
  const sheet = (k: number, kv: number): string => f.poly([P(-k, -kv), P(k, -kv), P(k, kv), P(-k, kv)]);
  /** An outline of light: the points as a ring, hollowed toward their centre. */
  const outline = (points: readonly Point[], cu: number, cv: number, k: number): string =>
    f.poly(points.map(([u, v]) => P(u, v))) + f.poly(points.map(([u, v]) => P(cu + (u - cu) * k, cv + (v - cv) * k)));
  const round = rng() < 0.5;
  const part: Point[] = [];
  for (let i = 0; i < (round ? 12 : 4); i += 1) {
    const a = (round ? i * 30 : 45 + i * 90) * DEGREES;
    part.push([-0.5 + 0.3 * Math.cos(a), -0.05 + 0.55 * Math.sin(a)]);
  }
  const nose = 0.8 + between(rng, -0.08, 0.08);
  const hull: Point[] = [[0.05, -0.5], [0.5, -0.5], [nose, 0], [0.5, 0.5], [0.05, 0.5]];
  let across = "";
  let down = "";
  for (const v of [-0.5, 0, 0.5]) across += f.band([P(-0.97, v), P(0.95, v)], 0.18);
  for (const u of [-0.6, -0.2, 0.2, 0.6]) down += f.band([P(u, -0.97), P(u, 0.97)], 0.18);
  return [
    shape(sheet(1.12, 1.35), "cyan-400", 0.1),
    shape(sheet(1.05, 1.15), "cyan-400", 0.18),
    shape(f.poly([P(1, 0.45), P(1, 1), P(0.78, 1)]), "blue-900", 0.35),
    shape(f.poly([P(-1, -1), P(1, -1), P(1, 0.45), P(0.78, 1), P(-1, 1)]), "blue-600", 1, MAIN_INKED),
    shape(across, "sky-300", 0.35),
    shape(down, "sky-300", 0.35),
    shape(outline(part, -0.5, -0.05, round ? 0.72 : 0.6) + outline(hull, 0.38, 0, 0.62) + f.band([P(0.05, 0.75), P(0.5, 0.75)], 0.22), "white", 0.85),
    shape(f.poly([P(1, 0.45), P(0.78, 1), P(0.74, 0.38)]), "sky-300", 1, INKED),
    shape(star(f, P(nose, 0), 1.3), "white", 0.9),
  ];
}

/** A big reset lever standing on the anchor: a base box banded with hazard stripes, a slot, a long handle (main) thrown toward its facing, a red knob, a status lamp; about 10 wide and 18 tall at scale 1. */
function resetLever(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const thrown = between(rng, 18, 48);
  const pivot: Point = [0, -7.6];
  const grip = plus(pivot, heading(thrown - 90), 9.6);
  const lamp = rng() < 0.5 ? "green-400" : "red-400";
  let marks = box(f, -3.2, -7.9, 3.2, -7.3) + box(f, 0.6, -5.4, 4, -3.8);
  for (let k = 0; k < 5; k += 1) {
    const x = -4.7 + k * 1.9;
    marks += f.poly([[x, -0.6], [x + 0.9, -0.6], [x + 1.6, -2.4], [x + 0.7, -2.4]]);
  }
  const arc = (r: number, from: number, to: number): string => f.band([0, 1, 2, 3, 4, 5].map((i) => polar(r, from + ((to - from) * i) / 5, ...pivot)), 0.45);
  return [
    shape(box(f, -5, -7, 5, 0), "slate-700", 1, INKED),
    shape(f.poly([[-5, -7], [5, -7], [4, -8.2], [-4, -8.2]]), "slate-400", 1, INKED),
    shape(box(f, -5, -2.4, 5, -0.6), "brass-500", 1),
    shape(marks, "slate-900", 0.9),
    shape(arc(8.4, -108, thrown - 100) + arc(6.6, -100, thrown - 104), "white", 0.4),
    shape(f.band([pivot, grip], 1.3), "slate-400", 1, MAIN_INKED),
    shape(f.circle(...pivot, 1.4), "slate-800", 1, INKED),
    shape(f.circle(...grip, 1.8), "red-500", 1, INKED),
    shape(f.circle(grip[0] - 0.6, grip[1] - 0.6, 0.55), "white", 0.6),
    shape(f.circle(-3, -4.6, 1.6), lamp, 0.3),
    shape(f.circle(-3, -4.6, 0.8), lamp, 1),
  ];
}

/** A miner's pickaxe and lunchbox resting on the anchor: a lunchbox (main) with a handle and latch, the pick's head and haft leaning on it; about 14 wide and 6 tall at scale 1. */
function pickaxe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const left = -6 + between(rng, -0.4, 0.4);
  const foot: Point = [7.4 + between(rng, -0.4, 0.4), 0];
  const top: Point = [0.4, -4.2 + between(rng, -0.4, 0.3)];
  const length = Math.hypot(top[0] - foot[0], top[1] - foot[1]) || 1;
  const u: Point = [(top[0] - foot[0]) / length, (top[1] - foot[1]) / length];
  const n: Point = [-u[1], u[0]];
  const H = (a: number, b: number): Point => plus(plus(top, n, a), u, b);
  const wrap = (t: number): string => f.band([plus(plus(foot, u, t), n, -0.55), plus(plus(foot, u, t), n, 0.55)], 0.35);
  return [
    shape(f.ellipse(0.6, 0.2, 7.6, 0.8), "slate-900", 0.3),
    shape(`M${f.p(left, 0)}L${f.p(left, -3.6)}Q${f.p(left, -5.4)} ${f.p(-2.5, -5.4)}Q${f.p(1, -5.4)} ${f.p(1, -3.6)}L${f.p(1, 0)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.band([[left, -3.5], [1, -3.5]], 0.35) + box(f, left, -1.2, 1, 0), "slate-900", 0.4),
    shape(f.band([[-4.4, -5.1], [-4.2, -6.5], [-0.8, -6.5], [-0.6, -5.1]], 0.5), "slate-700", 1, INKED),
    shape(box(f, -3.1, -4.1, -1.9, -2.9), "brass-500", 1, INKED),
    shape(f.band([[left + 0.7, -4.2], [-4.4, -5], [-2.8, -5.2]], 0.4), "white", 0.35),
    shape(f.band([foot, plus(top, u, 0.4)], 0.8), "wood-600", 1, INKED),
    shape(wrap(1) + wrap(1.7), "wood-800", 0.8),
    shape(f.poly([H(3.4, -1), H(1.3, 0.75), H(0, 0.9), H(-1.3, 0.75), H(-3.4, -1), H(-1.1, -0.55), H(0, -0.45), H(1.1, -0.55)]), "slate-400", 1, INKED),
    shape(f.band([H(0.6, 0.6), H(2.4, 0.1)], 0.3), "slate-100", 0.8),
  ];
}

/** A huge mining drill from the anchor toward its facing: a housing with vents and lights, a conical spiralled bit (main) about 18 long, sparks and dust at its tip; about 30 long and 10 tall at scale 1. */
function miningDrill(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 28, 31);
  const spin = between(rng, 0, 2.6);
  const tip: Point = [reach, 0];
  const half = (x: number): number => (4 * (reach - x)) / (reach - 13);
  let flutes = "";
  for (let x = 14 + spin; x + 1.8 < reach - 2; x += 3.2) flutes += f.band([[x, -half(x) * 0.9], [x + 1.8, half(x + 1.8) * 0.9]], 0.55);
  const streaks = [-140, -60, 70, 140]
    .map((a) => {
      const d = heading(a + between(rng, -15, 15));
      return f.band([plus(tip, d, 1.6), plus(tip, d, between(rng, 3.6, 5.2))], 0.35);
    })
    .join("");
  return [
    shape(f.circle(reach + 1.5, -3.2, 1.6) + f.circle(reach + 2.2, 2.8, 1.3) + f.circle(reach + 3.6, -0.6, 1), "stone-500", 0.3),
    shape(f.poly([[1.4, -5], [10.6, -5], [12, -3.6], [12, 3.6], [10.6, 5], [1.4, 5], [0, 3.6], [0, -3.6]]), "slate-700", 1, INKED),
    shape(f.poly([[1.4, -5], [10.6, -5], [12, -3.6], [12, -2.6], [0, -2.6], [0, -3.6]]), "slate-400", 0.35),
    shape(f.band([[1.6, -1], [5.6, -1]], 0.6) + f.band([[1.6, 0.6], [5.6, 0.6]], 0.6) + f.band([[1.6, 2.2], [5.6, 2.2]], 0.6), "slate-900", 0.8),
    shape(f.circle(8.4, -1.2, 0.5) + f.circle(10, -1.2, 0.5) + f.circle(9.2, 2, 0.5), "cyan-400", 1),
    shape(box(f, 11.4, -4.4, 13.2, 4.4), "slate-400", 1, INKED),
    shape(`M${f.p(13, -4)}Q${f.p(20, -3.2)} ${f.p(...tip)}Q${f.p(20, 3.2)} ${f.p(13, 4)}Z`, "slate-400", 1, MAIN_INKED),
    shape(`M${f.p(13, 0.4)}L${f.p(...tip)}Q${f.p(20, 3.2)} ${f.p(13, 4)}Z`, "slate-900", 0.3),
    shape(flutes, "slate-800", 0.7),
    shape(f.circle(...tip, 3.6), "orange-400", 0.28),
    shape(streaks, "orange-400", 1),
    shape(star(f, tip, 2.4), "white", 0.9),
  ];
}

/** A cracked chaos egg centred on the anchor: a speckled shell (main) split by a jagged crack, a glow pouring out of it, shell pieces flying off; about 12 wide and 16 tall at scale 1. */
function chaosEgg(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const crack = between(rng, -2.4, -0.8);
  const lift = between(rng, 2.6, 3.6);
  const tip = between(rng, -18, 18);
  const side = between(rng, -0.6, 0.6);
  const rim = (t: number): Point => [6.2 * Math.sin(t) * (1 - 0.18 * Math.cos(t)), -8 * Math.cos(t)];
  const tc = Math.acos(-crack / 8);
  const edge = rim(tc)[0];
  const zigzag: Point[] = [];
  for (let i = 0; i <= 6; i += 1) zigzag.push([-edge + (2 * edge * i) / 6, i === 0 || i === 6 ? crack : crack + (i % 2 === 0 ? 1 : -1) * between(rng, 0.9, 1.4)]);
  const arc = (from: number, to: number, steps: number): Point[] => {
    const out: Point[] = [];
    for (let i = 1; i < steps; i += 1) out.push(rim(from + ((to - from) * i) / steps));
    return out;
  };
  const C = (p: Point): Point => plus([side, crack - lift], turned([p[0], p[1] - crack], tip));
  const cap = [...arc(-tc, tc, 10), ...[...zigzag].reverse()].map(C);
  const shade = arc(tc + 0.15, Math.PI + 0.5, 6);
  const speckles: Point[] = [[-3, 2.4], [1.5, 4.6], [3.6, 1.2], [-1.6, 5.6], [-4.2, 4.8]];
  return [
    shape(f.circle(0, crack - 2, 9), "green-400", 0.16),
    shape(f.poly([[-edge * 0.8, crack], [edge * 0.8, crack], [edge * 1.3, crack - lift - 6], [-edge * 1.3, crack - lift - 6]]), "lime-400", 0.22),
    shape(f.ellipse(0, crack - 0.2, edge * 0.85, 1.5), "lime-400", 0.9),
    shape(f.poly([...zigzag, ...arc(tc, 2 * Math.PI - tc, 12)]), "violet-500", 1, MAIN_INKED),
    shape(f.poly([...shade, ...shade.map(([x, y]): Point => [-1 + (x + 1) * 0.7, 2 + (y - 2) * 0.7]).reverse()]), "violet-700", 0.45),
    shape(f.poly(cap), "violet-500", 1, MAIN_INKED),
    shape(speckles.map((p) => f.circle(...p, 0.4)).join("") + [C([1.8, -5]), C([-1.4, -6.2])].map((p) => f.circle(...p, 0.4)).join(""), "violet-300", 0.8),
    shape(f.band(zigzag, 0.5) + f.band(zigzag.map(C), 0.5), "lime-200", 0.9),
    shape(
      ([[-7.4, -2.4], [7.6, -3.6], [-5, -8]] as const)
        .map(([x, y]) => {
          const c: Point = [x + between(rng, -0.6, 0.6), crack + y];
          const spin = between(rng, 0, 360);
          return f.poly([polar(1.2, spin, ...c), polar(0.9, spin + 130, ...c), polar(1, spin + 240, ...c)]);
        })
        .join(""),
      "violet-500",
      1,
      MAIN_INKED,
    ),
    shape(star(f, [-1.6, crack - 4.5], 1.4) + star(f, [2.4, crack - 6.5], 1), "white", 0.85),
  ];
}

/** A game board lying flat in perspective, centred on the anchor: a checkered board (main) with a rim, five or six pieces standing on it; about 20 wide and 8 deep at scale 1. */
function gameBoard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  // A point of the board's top, `u` across it and `v` from its far edge to its near one, the far edge narrower.
  const P = (u: number, v: number): Point => [(-10 + 20 * u) * (0.8 + 0.2 * v), -4 + 8 * v];
  const cell = (c: number, r: number): string => f.poly([P(c / 6, r / 3), P((c + 1) / 6, r / 3), P((c + 1) / 6, (r + 1) / 3), P(c / 6, (r + 1) / 3)]);
  let dark = "";
  for (let r = 0; r < 3; r += 1) for (let c = r % 2; c < 6; c += 2) dark += cell(c, r);
  const piece = (c: number, r: number): string => {
    const [x, y] = P((c + 0.5) / 6, (r + 0.5) / 3);
    return f.poly([[x - 0.8, y], [x + 0.8, y], [x + 0.45, y - 1.6], [x - 0.45, y - 1.6]]) + f.circle(x, y - 2.4, 0.75);
  };
  const shift = whole(rng, 0, 5);
  // A row's pieces, two columns apart so none touches another.
  const row = (r: number, n: number): string => {
    let d = "";
    for (let k = 0; k < n; k += 1) d += piece((shift + r + 2 * k) % 6, r);
    return d;
  };
  return [
    shape(f.ellipse(0, 4.8, 11, 1.4), "slate-900", 0.3),
    shape(f.poly([[-10, 4], [10, 4], [10, 5.4], [-10, 5.4]]), "wood-800", 1, INKED),
    shape(f.poly([P(0, 0), P(1, 0), P(1, 1), P(0, 1)]), "wood-600", 1, MAIN_INKED),
    shape(dark, "wood-900", 0.75),
    shape(row(0, 2), "red-500", 1, INKED),
    shape(row(1, 2), "sky-300", 1, INKED),
    shape(row(2, whole(rng, 1, 2)), "red-500", 1, INKED),
    shape(f.band([P(0.08, 0.06), P(0.5, 0.06)], 0.3), "white", 0.35),
  ];
}

/** A square wheat cracker centred on the anchor, tilted: a golden-brown biscuit (main, brass-500) docked with holes, rosemary sprigs and salt flecks on top, a bite taken from a corner; about 6 across at scale 1. */
function cracker(at: PartPlace, rng: Rng): KitShape[] {
  const t = tilted(local(at), between(rng, -24, 24));
  const rim: Point[] = [[-3, -3], [1, -3], [1.3, -2.3], [1.9, -2], [2.2, -1.4], [3, -1], [3, 3], [-3, 3]];
  const holes: Point[] = [[-1.6, -1.6], [0, -1.6], [-1.6, 0], [0, 0], [1.6, 0.2], [-1.6, 1.6], [0, 1.6], [1.6, 1.6]];
  const sprig = (a: Point, b: Point): string => {
    let d = t.band([a, b], 0.25);
    for (const s of [0.25, 0.5, 0.75]) {
      const p: Point = [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
      d += t.band([p, [p[0] - 0.2, p[1] - 0.7]], 0.2) + t.band([p, [p[0] + 0.6, p[1] + 0.3]], 0.2);
    }
    return d;
  };
  return [
    shape(t.poly(rim.map(([x, y]): Point => [x + 0.4, y + 0.5])), "wood-700", 1, INKED),
    shape(t.poly(rim), "brass-500", 1, MAIN_INKED),
    shape(t.band([[-2.4, -2.4], [0.6, -2.4]], 0.3) + t.band([[-2.4, 2.4], [2.4, 2.4]], 0.3), "wood-600", 0.45),
    shape(holes.map(([x, y]) => t.circle(x, y, 0.28)).join(""), "wood-800", 0.8),
    shape(sprig([-2.4, 1], [0.8, -0.9]) + sprig([0.2, 2.5], [2.5, 0.6]), "green-700", 1),
    shape(t.circle(-0.8, -2.2, 0.18) + t.circle(2.1, 1, 0.18) + t.circle(-2.2, 2.2, 0.18) + t.circle(0.9, 0.9, 0.18), "white", 0.85),
  ];
}

/** A holographic price chart standing on the anchor: four or five translucent bars (main) rising left to right, a frame, an arrow climbing over them; about 14 wide and 14 tall at scale 1. */
function priceBars(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 4, 5);
  const step = 12 / n;
  let bars = "";
  let tops = "";
  const peaks: Point[] = [];
  for (let i = 0; i < n; i += 1) {
    const x0 = -6 + i * step + step * 0.18;
    const x1 = x0 + step * 0.64;
    const h = 2.6 + (8.4 * i) / (n - 1) + between(rng, -0.6, 0.6);
    bars += box(f, x0, -h, x1, -0.4);
    tops += box(f, x0, -h, x1, -h + 0.5);
    peaks.push([(x0 + x1) / 2, -h - 1.4]);
  }
  const first = peaks[0] ?? [-5, -4];
  const end = peaks[peaks.length - 1] ?? [5, -12];
  const head: Point = [end[0] + 1.4, end[1] - 1.4];
  return [
    shape(box(f, -7, -14, 7, 0), "sky-300", 0.12),
    shape(f.band([[-7, -14], [-7, 0], [7, 0]], 0.4) + f.band([[-6.6, -4.7], [6.6, -4.7]], 0.15) + f.band([[-6.6, -9.4], [6.6, -9.4]], 0.15), "white", 0.4),
    shape(bars, "sky-300", 0.55, MAIN),
    shape(tops, "white", 0.7),
    shape(f.band([first, ...peaks.slice(1, -1), end], 0.6), "white", 0.9),
    shape(f.poly([[head[0] + 0.4, head[1] - 0.4], [head[0] - 2.4, head[1] + 0.2], [head[0] - 0.2, head[1] + 2.4]]), "white", 0.9),
    shape(box(f, -5, 0, 5, 1), "slate-700", 1, INKED),
  ];
}

/** An AI's light ring centred on the anchor: a dark ring housing, ten or twelve lit segments (main) round it with a few dark, a glow and spin streaks; about 14 across at scale 1. */
function lightRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 10, 12);
  const spin = between(rng, 0, 360);
  const dark = new Set([whole(rng, 0, n - 1), whole(rng, 0, n - 1)]);
  let lit = "";
  let off = "";
  for (let i = 0; i < n; i += 1) {
    const a = spin + (i * 360) / n;
    const w = (360 / n) * 0.34;
    const segment = f.poly([polar(5, a - w), polar(6.6, a - w), polar(6.6, a + w), polar(5, a + w)]);
    if (dark.has(i)) off += segment;
    else lit += segment;
  }
  const streak = (from: number): string => f.band([0, 1, 2, 3, 4].map((k) => polar(8.4, from + k * 12)), 0.4);
  return [
    shape(f.circle(0, 0, 9.4), "sky-300", 0.16, MAIN),
    shape(f.circle(0, 0, 7.2) + f.circle(0, 0, 4.4), "slate-800", 1, INKED),
    shape(lit, "sky-300", 1, MAIN),
    shape(off, "slate-700", 0.9),
    shape(f.circle(0, 0, 3.6), "slate-900", 1, INKED),
    shape(f.circle(0, 0, 1.4), "sky-300", 0.8, MAIN),
    shape(streak(spin + 20) + streak(spin + 200), "white", 0.4),
  ];
}

/** A beam of light falling straight down from the anchor: a wide translucent column (main) about 8 wide and 60 long, a brighter core, rings of light down it and a splash of light where it lands at its foot. */
function energyBeam(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const first = between(rng, 6, 12);
  let rings = "";
  for (let y = first; y < 54; y += 12) rings += f.ellipse(0, y, 5.4, 1.3) + f.ellipse(0, y, 4.4, 0.8);
  return [
    shape(f.poly([[-5.6, 0], [5.6, 0], [6.6, 60], [-6.6, 60]]), "sky-300", 0.16, MAIN),
    shape(f.poly([[-4, 0], [4, 0], [4.4, 60], [-4.4, 60]]), "sky-300", 0.4, MAIN),
    shape(f.poly([[-1.4, 0], [1.4, 0], [1.6, 60], [-1.6, 60]]), "white", 0.75),
    shape(rings, "white", 0.45),
    shape(f.ellipse(0, 60, 11, 2.6), "sky-300", 0.35, MAIN),
    shape(f.ellipse(0, 60, 5, 1.1) + f.circle(0, 0, 2.6), "white", 0.6),
  ];
}

/** Three wobbly bolts of magic from the anchor curving off toward its facing at different angles: each a glowing head (main) with a tapering wavering trail; about 30 long and 16 tall at scale 1. */
function magicBolt(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const trails: Point[][] = [];
  const heads: Point[] = [];
  for (const angle of [-22, 0, 22]) {
    const a = angle + between(rng, -6, 6);
    const reach = between(rng, 26, 30);
    const end = polar(reach, a);
    const bend = polar(reach * 0.55, a + between(rng, -28, 28));
    const wobble = between(rng, 0, 6);
    const curve = quad([0, 0], bend, end, 10);
    trails.push(beside(curve, 0).map((p, i) => plus(p, heading(a + 90), Math.sin(wobble + i * 1.3) * 0.7 * (i / 10))));
    heads.push(end);
  }
  const trail = (scale: number): string => trails.map((points) => tapered(f, points, (t) => scale * t)).join("");
  return [
    shape(trail(3.6), "violet-300", 0.25, MAIN),
    shape(trail(1.6), "violet-300", 0.85, MAIN),
    shape(trail(0.5), "white", 0.7),
    shape(heads.map((p) => f.circle(...p, 3.6)).join(""), "violet-300", 0.3, MAIN),
    shape(heads.map((p) => f.circle(...p, 2)).join(""), "violet-300", 1, MAIN),
    shape(heads.map((p) => star(f, p, 2.6)).join(""), "white", 0.85),
  ];
}

/** A laser from the anchor toward its facing: a thin bright line (main) with a white core about 40 long, a glow about it, a flare at the muzzle and an impact flare at its far end. */
function laser(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 38, 42);
  const end: Point = [reach, 0];
  const sparks = [150, 200, 120, 240].map((a) => f.band([plus(end, heading(a + between(rng, -12, 12)), 1.8), plus(end, heading(a), between(rng, 3.6, 5.4))], 0.35)).join("");
  return [
    shape(f.band([[0, 0], end], 3.2), "red-500", 0.25, MAIN),
    shape(f.band([[0, 0], end], 1.2), "red-500", 0.9, MAIN),
    shape(f.band([[0, 0], end], 0.4), "white", 0.95),
    shape(f.circle(0, 0, 2.6) + f.circle(...end, 4), "red-500", 0.35, MAIN),
    shape(star(f, [0, 0], 3.2) + star(f, end, 4.6, 6, between(rng, 0, 30)), "white", 0.85),
    shape(sparks, "red-500", 0.9, MAIN),
  ];
}

/** A shimmering wall of energy standing on the anchor: a translucent panel (main) about 30 wide and 30 tall, a hexagon grid of light on it, bright edges, crackles along its top and an emitter rail at its foot. */
function energyWall(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hex = (cx: number, cy: number, r: number): string => f.poly([0, 1, 2, 3, 4, 5].map((k) => polar(r, 30 + k * 60, cx, cy)));
  let grid = "";
  for (const y of [-24, -15, -6]) {
    const zig: Point[] = [];
    for (let x = -14; x <= 14; x += 3.5) zig.push([x, y + ((x + 14) % 7 === 0 ? -1.5 : 1.5)]);
    grid += f.band(zig, 0.3);
  }
  let cells = "";
  for (let k = 0; k < 6; k += 1) cells += hex(-12 + k * 4.8 + between(rng, -0.6, 0.6), -27 + ((k * 7) % 5) * 5.4, 1.8);
  const crackle = (x0: number): string => {
    const points: Point[] = [];
    for (let i = 0; i <= 5; i += 1) points.push([x0 + i * 1.8, -30 - (i % 2 === 0 ? 0.4 : between(rng, 1.4, 2.6))]);
    return f.band(points, 0.4);
  };
  return [
    shape(f.ellipse(0, 0, 17, 2.2), "sky-300", 0.3, MAIN),
    shape(box(f, -15, -30, 15, 0), "sky-300", 0.28, MAIN),
    shape(grid, "white", 0.3),
    shape(cells, "white", 0.25),
    shape(f.band([[-15, 0], [-15, -30], [15, -30], [15, 0]], 0.6), "white", 0.7),
    shape(crackle(-12 + between(rng, -1, 1)) + crackle(3 + between(rng, -1, 1)), "sky-300", 0.9, MAIN),
    shape(box(f, -16.5, 0, 16.5, 2), "slate-700", 1, INKED),
    shape(f.circle(-12, 1, 0.5) + f.circle(0, 1, 0.5) + f.circle(12, 1, 0.5), "sky-300", 1, MAIN),
  ];
}

/** A tractor beam spreading down from the anchor: a translucent cone of light (main) about 20 wide at its foot and 24 long, bands of light down it and a net of light lines at its foot. */
function tractorBeam(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const width = (y: number): number => 1.6 + (8.4 * y) / 24;
  let bands = "";
  for (let y = between(rng, 4, 7); y < 20; y += 5) bands += f.band([[-width(y), y], [0, y + 0.2 * width(y)], [width(y), y]], 0.5);
  let net = "";
  for (let k = -2; k <= 2; k += 1) net += f.band([[k * 3.4 - 2, 19.6], [k * 3.4 + 2, 24]], 0.3);
  return [
    shape(f.poly([[-2.6, 0], [2.6, 0], [12, 24], [-12, 24]]), "cyan-400", 0.16, MAIN),
    shape(f.poly([[-1.6, 0], [1.6, 0], [10, 24], [-10, 24]]), "cyan-400", 0.32, MAIN),
    shape(bands, "white", 0.45),
    shape(net, "white", 0.35),
    shape(f.ellipse(0, 24, 10, 2) + f.ellipse(0, 24, 9, 1.4), "white", 0.55),
    shape(f.ellipse(0, 0, 3, 1.2), "slate-700", 1, INKED),
    shape(f.circle(0, 0.4, 0.7), "white", 0.9),
  ];
}

/** An engine's plume behind the anchor, away from its facing: an outer flame (main), an inner one and a white-hot core tapering about 18 long, shock diamonds in it and smoke puffs at its tail. */
function thrusterFlame(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 16, 20);
  const plume = (half: number, length: number): string => `M${f.p(0, -half)}Q${f.p(-length * 0.45, -half * 1.2)} ${f.p(-length, 0)}Q${f.p(-length * 0.45, half * 1.2)} ${f.p(0, half)}Z`;
  const diamond = (x: number, r: number): string => f.poly([[x + r, 0], [x, -r * 0.6], [x - r, 0], [x, r * 0.6]]);
  const puffs = [0, 1, 2].map((k) => f.circle(-reach - 1 - k * 1.6 + between(rng, -0.5, 0.5), between(rng, -2.4, 2.4), 1.1 + k * 0.3)).join("");
  return [
    shape(f.ellipse(-reach * 0.45, 0, reach * 0.62, 4.6), "orange-400", 0.2, MAIN),
    shape(plume(3, reach), "orange-400", 0.85, MAIN),
    shape(plume(1.9, reach * 0.7), "white", 0.55),
    shape(plume(1, reach * 0.35), "white", 0.95),
    shape(diamond(-reach * 0.42, 1.1) + diamond(-reach * 0.62, 0.8) + diamond(-reach * 0.8, 0.5), "white", 0.6),
    shape(puffs, "slate-400", 0.3),
  ];
}

/** A jump flash centred on the anchor: three rippling rings of light (main) widening out, a four-pointed starburst and a bright white core; about 30 across at scale 1. */
function warpFlash(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, 0, 6);
  // Six ripples round each ring, four points to a ripple.
  const wavy = (r: number): Point[] => Array.from({ length: 24 }, (_, k) => polar(r + 0.5 * Math.sin(phase + (k * Math.PI) / 2), k * 15));
  const ring = (r: number, w: number): string => f.poly(wavy(r + w / 2)) + f.poly(wavy(r - w / 2));
  const turn = between(rng, -12, 12);
  return [
    shape(f.circle(0, 0, 15), "sky-300", 0.15, MAIN),
    shape(ring(6, 1) + ring(10, 0.8) + ring(13.6, 0.6), "sky-300", 0.6, MAIN),
    shape(star(f, [0, 0], 13, 4, turn, 0.12), "white", 0.55),
    shape(star(f, [0, 0], 7, 4, turn + 45, 0.2), "white", 0.5),
    shape(f.circle(0, 0, 4.6), "sky-300", 0.5, MAIN),
    shape(f.circle(0, 0, 2.6), "white", 0.95),
  ];
}

/** A thread of black shadow from the anchor curling about 30 toward its facing: a thin dark band thickening and thinning as it winds, wisps peeling off it, a violet glow along it (main) and a sharp point at its end. */
function shadowTendril(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, 0, 6);
  const sway = between(rng, 3, 5);
  const path: Point[] = Array.from({ length: 13 }, (_, i) => {
    const t = i / 12;
    return [30 * t, sway * Math.sin(phase + t * 7.5) * (1 - 0.4 * t)];
  });
  const width = (t: number): number => (0.6 + 1.6 * Math.abs(Math.sin(t * 4.7))) * (1 - t);
  const wisp = (i: number, side: number): string => {
    const base = path[i] ?? [0, 0];
    return tapered(f, [base, plus(base, [1.6, side * 2.4]), plus(base, [3.8, side * 3])], (t) => 0.9 * (1 - t));
  };
  return [
    shape(f.band(path, 3.2), "violet-500", 0.3, MAIN),
    shape(tapered(f, path, width), "black", 0.95),
    shape(wisp(3, -1) + wisp(7, 1) + wisp(10, -1), "black", 0.6),
    shape(f.band(beside(path.slice(1, 10), -0.9), 0.25), "violet-300", 0.5),
    shape(star(f, path[12] ?? [30, 0], 1.6), "violet-300", 0.8),
  ];
}

/** R3403000: the gear in space Stinkyscape's later cards added, by id, each with its kind. */
export const GEAR_KIT = {
  console: { category: "objects", draw: consoleDesk },
  "light-diagram": { category: "objects", draw: lightDiagram },
  cable: { category: "objects", draw: cable },
  "ore-chunk": { category: "objects", draw: oreChunk },
  wreckage: { category: "objects", draw: wreckage },
  "credit-chip": { category: "objects", draw: creditChip },
  roulette: { category: "objects", draw: roulette },
  capsule: { category: "objects", draw: capsule },
  "kicking-sack": { category: "objects", draw: kickingSack },
  pages: { category: "objects", draw: pages },
  "holo-blueprint": { category: "objects", draw: blueprint },
  "reset-lever": { category: "objects", draw: resetLever },
  pickaxe: { category: "objects", draw: pickaxe },
  "mining-drill": { category: "objects", draw: miningDrill },
  "chaos-egg": { category: "objects", draw: chaosEgg },
  "game-board": { category: "objects", draw: gameBoard },
  cracker: { category: "objects", draw: cracker },
  "price-bars": { category: "objects", draw: priceBars },
  "light-ring": { category: "objects", draw: lightRing },
  "energy-beam": { category: "weather", draw: energyBeam },
  "magic-bolt": { category: "weather", draw: magicBolt },
  laser: { category: "weather", draw: laser },
  "energy-wall": { category: "weather", draw: energyWall },
  "tractor-beam": { category: "weather", draw: tractorBeam },
  "thruster-flame": { category: "weather", draw: thrusterFlame },
  "warp-flash": { category: "weather", draw: warpFlash },
  "shadow-tendril": { category: "weather", draw: shadowTendril },
} as const satisfies Readonly<Record<string, KitPart>>;
