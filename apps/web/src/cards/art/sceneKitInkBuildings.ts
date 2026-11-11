// R3401000: the scene kit's buildings in ink (ART.md, "Scene recipes and kits"), which the Meditative scenes (#3401)
// added. Several are a token Unit's whole character (Jlockeed's Evil Blueprints raises buildings that are Units), so
// each reads as that building at a glance and keeps a little life: a lit window, a lamp or a beacon. A staked plot of
// fenced dirt lies back from its anchor, the anchor its front edge's middle; the buildings stand on their anchor, their
// bottom centre (a brick schoolhouse, a glass megachurch, a bunker in its mound, a stone university hall, a prison on
// a sea crag, a gunmetal corporate fortress, a soot-black factory, a mountain-road diner, a railway platform), and so
// does a desk; the Great Wall spans the box as terrain does, its anchor's height its mean, its anchor's x shifting its
// towers. sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css name, none in Radiant's gold
// window; a glow is a stack of translucent shapes, never a gradient; shapes are filled even-odd, so a hole is a second
// sub-path and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, EDGE, FAR_EDGE, INKED, local, MAIN, MAIN_INKED, pt, shape, sparkle, whole, type Local, type Point } from "./kitDraw.ts";
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

/** An opening with a round head about `cx`, its sides `half` out, standing on `base` and springing at `spring`. */
function arched(f: Local, cx: number, half: number, base: number, spring: number): string {
  const r = fmt(f.r(half));
  return `M${f.p(cx - half, base)}L${f.p(cx - half, spring)}A${r} ${r} 0 0 ${f.sweep(1)} ${f.p(cx + half, spring)}L${f.p(cx + half, base)}Z`;
}

/** A tuft of grass standing at (`x`, `y`), one zigzag outline about 2.4 wide and 2 tall. */
function tuft(f: Local, x: number, y: number): string {
  return f.poly([
    [x - 1.2, y],
    [x - 0.9, y - 1.5],
    [x - 0.35, y - 0.35],
    [x, y - 2],
    [x + 0.35, y - 0.35],
    [x + 0.9, y - 1.4],
    [x + 1.2, y],
  ]);
}

/** The point `t` along the quadratic curve from `a` through control `c` to `b`. */
function onQuad(a: Point, c: Point, b: Point, t: number): Point {
  const u = 1 - t;
  return [u * u * a[0] + 2 * u * t * c[0] + t * t * b[0], u * u * a[1] + 2 * u * t * c[1] + t * t * b[1]];
}

/** `a` to `b`, `t` of the way. */
function mix(a: Point, b: Point, t: number): Point {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
}

// Plots and buildings

/** A vacant hillside lot of bare dirt seen from a little above, its front edge's middle on the anchor: a low post-and-wire fence round it open at the front, stakes and white string marking out a foundation, a tall post at its left front corner (its top 11.5 up) for a crow; about 40 wide and 14 deep at scale 1. MAIN is the dirt. */
function stakePlot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -1.6, 1.6);
  const FL: Point = [-20, tilt * 0.5];
  const FR: Point = [20, -tilt * 0.5];
  const BL: Point = [-16.5, -13 + tilt];
  const BR: Point = [16.5, -13 - tilt];
  const q = (u: number, v: number): Point => mix(mix(FL, FR, (u + 1) / 2), mix(BL, BR, (u + 1) / 2), v);
  const up = ([x, y]: Point, h: number): Point => [x, y - h];
  const tall = (v: number): number => 3 - 0.9 * v;
  const shift = between(rng, -0.15, 0.15);
  const [u0, u1, v0, v1] = [-0.55 + shift, 0.35 + shift, 0.3, 0.78];
  const [du, dv] = [0.35 / 18, 0.35 / 13];
  const quad = (a: number, b: number, c: number, d: number): string => f.poly([q(a, c), q(b, c), q(b, d), q(a, d)]);
  const corners = [q(u0, v0), q(u1, v0), q(u1, v1), q(u0, v1)];
  const posts: [number, number][] = [
    [-0.6, 0],
    [-0.2, 0],
    [0.2, 0],
    [0.6, 0],
    [1, 0],
    [-1, 0.5],
    [1, 0.5],
    [-1, 1],
    [-0.5, 1],
    [0, 1],
    [0.5, 1],
    [1, 1],
  ];
  const clods: Point[] = [
    [-0.78, 0.2],
    [-0.12, 0.12],
    [0.62, 0.22],
    [0.72, 0.68],
    [-0.74, 0.82],
    [0.12, 0.92],
  ];
  const lean = between(rng, -0.6, 0.6);
  const crowFoot = q(-1, 0);
  const wireTop = (u: number, v: number): Point => up(q(u, v), tall(v) * 0.9);
  const wireLow = (u: number, v: number): Point => up(q(u, v), tall(v) * 0.45);
  return [
    shape(tuft(f, 22, 0.4) + tuft(f, -20.5, -6) + tuft(f, 18.5, -11) + tuft(f, -6 + lean * 4, -14.4), "green-700", 0.7),
    shape(f.poly([FL, FR, BR, BL]), "wood-700", 1, MAIN_INKED),
    shape(f.ellipse(...q(0.2, 0.55), 9, 2.8), "wood-600", 0.45),
    shape(clods.map(([u, v]) => f.ellipse(...q(u + between(rng, -0.05, 0.05), v), between(rng, 0.7, 1.1), 0.35)).join(""), "wood-800", 0.55),
    shape(quad(u0, u1, v0, v1) + quad(u0 + du, u1 - du, v0 + dv, v1 - dv), "white", 0.85),
    shape(corners.map((c) => f.band([up(c, -0.3), up(c, 2.2)], 0.45)).join(""), "brass-500", 1),
    shape(corners.map((c) => f.band([up(c, 1.6), up(c, 2.25)], 0.6)).join(""), "red-600", 1),
    shape(posts.map(([u, v]) => f.band([up(q(u, v), -0.2), up(q(u, v), tall(v))], v === 0 ? 0.6 : 0.5)).join(""), "wood-800", 1),
    shape(
      f.band([wireTop(-0.2, 0), wireTop(-1, 0), wireTop(-1, 1), wireTop(1, 1), wireTop(1, 0), wireTop(0.2, 0)], 0.22) +
        f.band([wireLow(-0.2, 0), wireLow(-1, 0)], 0.22) +
        f.band([wireLow(1, 0), wireLow(0.2, 0)], 0.22) +
        f.band([wireLow(-1, 1), wireLow(1, 1)], 0.22),
      "slate-700",
      0.9,
    ),
    shape(f.band([up(crowFoot, -0.4), [crowFoot[0] + lean, crowFoot[1] - 11.5]], 1.5), "wood-800", 1, INKED),
  ];
}

/** A stern brick schoolhouse standing on the anchor: two rows of tall windows (one or two lit), a central door up two steps under a small gable, a bell in a cupola on the roof; about 36 wide and 31 tall at scale 1. MAIN is the brick. */
function schoolhouse(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cap = between(rng, -31.6, -30.6);
  const cols = [-14.5, -10.5, -6.5, 6.5, 10.5, 14.5];
  const rows: readonly (readonly [number, number])[] = [
    [-8.8, -3.4],
    [-17.4, -11.8],
  ];
  const windows: (readonly [number, number, number, number])[] = [];
  for (const [top, bottom] of rows) for (const x of cols) windows.push([x, 1.1, top, bottom]);
  windows.push([0, 1.3, -17.4, -11.8]);
  const lefts = [0, 1, 2, 6, 7, 8];
  const rights = [3, 4, 5, 9, 10, 11];
  const lit = [lefts[whole(rng, 0, 5)] ?? 0];
  if (rng() < 0.6) lit.push(rights[whole(rng, 0, 5)] ?? 3);
  const litWindows = lit.map((k) => windows[k] ?? windows[0] ?? ([0, 1, -8.8, -3.4] as const));
  const panes = ([x, h, top, bottom]: readonly [number, number, number, number]): string => {
    const mid = top + (bottom - top) * 0.45;
    return (
      rect(f, x - h + 0.2, top + 0.2, x - 0.15, mid - 0.15) +
      rect(f, x + 0.15, top + 0.2, x + h - 0.2, mid - 0.15) +
      rect(f, x - h + 0.2, mid + 0.15, x - 0.15, bottom - 0.2) +
      rect(f, x + 0.15, mid + 0.15, x + h - 0.2, bottom - 0.2)
    );
  };
  const walls: Point[] = [
    [-17, 0],
    [17, 0],
    [17, -20],
    [4.2, -20],
    [4.2, -21],
    [0, -24.2],
    [-4.2, -21],
    [-4.2, -20],
    [-17, -20],
  ];
  const steps: Point[] = [
    [-4.5, 0],
    [4.5, 0],
    [4.5, -0.9],
    [3.6, -0.9],
    [3.6, -1.8],
    [-3.6, -1.8],
    [-3.6, -0.9],
    [-4.5, -0.9],
  ];
  const lintels = windows.filter(([, , top]) => top === -8.8).map(([x]) => rect(f, x - 1.5, -9.6, x + 1.5, -8.9));
  const bell = `M${f.p(-1, -25.6)}L${f.p(-0.7, -25.9)}Q${f.p(-0.7, -27.5)} ${f.p(0, -27.6)}Q${f.p(0.7, -27.5)} ${f.p(0.7, -25.9)}L${f.p(1, -25.6)}Z`;
  return [
    shape(f.poly([[-18, -20], [18, -20], [15, -24.5], [-15, -24.5]]) + f.poly([[-3.1, -29], [3.1, -29], [0, cap]]), "slate-700", 1, INKED),
    shape(f.poly(walls), "red-900", 1, MAIN_INKED),
    shape(rect(f, 12.6, -20, 17, 0), "slate-900", 0.2),
    shape(rect(f, -1.45, -28.3, 1.45, -25) + arched(f, 0, 1.9, -1.8, -6.2), "wood-900", 1),
    shape(rect(f, -2.4, -29, 2.4, -24.5) + arched(f, 0, 1.3, -25.1, -26.8) + f.poly(steps), "stone-500", 1, INKED),
    shape(bell, "brass-600", 1),
    shape(windows.map(([x, h, top, bottom]) => rect(f, x - h, top, x + h, bottom)).join("") + f.circle(0, -21.7, 0.8), "slate-800", 1),
    shape(litWindows.map(([x, , top, bottom]) => f.ellipse(x, (top + bottom) / 2, 2.8, 4)).join(""), "orange-400", 0.22),
    shape(litWindows.map(panes).join(""), "orange-400", 0.95),
    shape(
      rect(f, -17, -10.75, 17, -10.05) + rect(f, -17, -20, 17, -19.3) + lintels.join("") + arched(f, 0, 2.9, -1.8, -6.2) + arched(f, 0, 1.9, -1.8, -6.2),
      "slate-100",
      0.7,
    ),
  ];
}

/** A gleaming glass megachurch standing on the anchor: a glass front sweeping up from low walls to a tall glass spire, mullion lines, a broad four-leaved door under a canopy with a warm light in it; about 36 wide and 34 tall at scale 1. MAIN is the glass. */
function glassChurch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = between(rng, -35, -33);
  const base = 1.8;
  const roof = (x: number): number => -8 - 14 * (1 - Math.abs(x) / 18) ** 2;
  const spring = roof(base);
  const outline = (side: 1 | -1): string => `Q${f.p(side * 9.9, -8)} ${f.p(side * 18, -8)}`;
  const body =
    `M${f.p(-18, 0)}L${f.p(-18, -8)}Q${f.p(-9.9, -8)} ${f.p(-base, spring)}L${f.p(0, top)}L${f.p(base, spring)}` + `${outline(1)}L${f.p(18, 0)}Z`;
  const facet = `M${f.p(0, 0)}L${f.p(18, 0)}L${f.p(18, -8)}Q${f.p(9.9, -8)} ${f.p(base, spring)}L${f.p(0, top)}Z`;
  const reach = (y: number): number => 18 * (1 - Math.sqrt(Math.max(0, -(y + 7.6)) / 14)) - 0.3;
  const verticals = [-14.4, -10.8, -7.2, -3.6, 3.6, 7.2, 10.8, 14.4].map((x) => f.band([[x, -0.2], [x, roof(x) + 0.4]], 0.3)).join("");
  const horizontals = [-4, -10, -15].map((y) => f.band([[-reach(y), y], [reach(y), y]], 0.3)).join("");
  const leaves = [-5.4, -2.6, 0.1, 2.8].map((x) => rect(f, x, -5.5, x + 2.5, 0)).join("");
  const gleamAt = rng() < 0.5;
  return [
    shape(body, "sky-300", 1, MAIN_INKED),
    shape(facet, "blue-600", 0.2),
    shape(verticals + f.band([[0, -6.7], [0, top + 3.5]], 0.3), "slate-700", 0.45),
    shape(horizontals, "slate-700", 0.45),
    shape(
      f.band([[-15, -2], [-9.5, -10.6]], 1) + f.band([[-12.5, -1.2], [-7.5, -9]], 0.5) + f.band([[-1.1, -20], [-0.18, top + 3.5]], 0.4),
      "white",
      0.5,
    ),
    shape(leaves, "slate-800", 0.95),
    shape(f.ellipse(0, -2, 4.2, 2.4), "orange-400", 0.35),
    shape(rect(f, -7, -6.7, 7, -5.5), "white", 0.9),
    shape(gleamAt ? sparkle(f, 0, top + 1, 2.4) : sparkle(f, -11, -7, 2.2), "white", 0.95),
  ];
}

/** A squat concrete bunker dug into a slope, standing on the anchor: an earth mound over and round it, two or three gun slits glaring orange, a row of sandbags at its foot, a strand of barbed wire on posts along the mound; about 40 wide and 16 tall at scale 1 (the concrete 28 wide and 10.5 tall). MAIN is the concrete. */
function bunker(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, 0, 1.6);
  const L: Point = [-20, 0];
  const L1: Point = [-18.5, -10.5 - rise];
  const A: Point = [-11, -13.2 - rise];
  const M: Point = [0, -15];
  const B: Point = [11, -12.6];
  const R1: Point = [18.5, -9];
  const R: Point = [20, 0];
  const mound = `M${f.p(...L)}Q${f.p(...L1)} ${f.p(...A)}Q${f.p(...M)} ${f.p(...B)}Q${f.p(...R1)} ${f.p(...R)}Z`;
  const face: Point[] = [
    [-12.5, 0],
    [12.5, 0],
    [11.4, -7.8],
    [14, -7.8],
    [14, -10.4],
    [-14, -10.4],
    [-14, -7.8],
    [-11.4, -7.8],
  ];
  const slits = whole(rng, 2, 3) === 2 ? [-5.5, 5.5] : [-7.5, 0, 7.5];
  const bags: string[] = [];
  for (let k = 0; k < 8; k += 1) {
    const x = -13.65 + 3.9 * k;
    const h = between(rng, 2.2, 2.6);
    bags.push(`M${f.p(x - 1.8, 0)}Q${f.p(x - 2, -h * 0.45)} ${f.p(x - 1.55, -h * 0.82)}Q${f.p(x, -h * 1.05)} ${f.p(x + 1.55, -h * 0.82)}Q${f.p(x + 2, -h * 0.45)} ${f.p(x + 1.8, 0)}Z`);
  }
  const feet = [onQuad(L, L1, A, 0.55), onQuad(A, M, B, 0.25), onQuad(A, M, B, 0.75), onQuad(B, R1, R, 0.45)];
  const tops = feet.map(([x, y]): Point => [x, y - 2.6]);
  const wire: Point[] = [];
  const barbs: string[] = [];
  tops.forEach((p, k) => {
    wire.push(p);
    const next = tops[k + 1];
    if (next === undefined) return;
    const sag: Point = [(p[0] + next[0]) / 2, (p[1] + next[1]) / 2 + 0.7];
    wire.push(sag);
    for (const t of [0.3, 0.7]) {
      const [x, y] = t < 0.5 ? mix(p, sag, t * 2) : mix(sag, next, t * 2 - 1);
      barbs.push(f.band([[x - 0.4, y - 0.4], [x + 0.4, y + 0.4]], 0.18));
    }
  });
  const posts = feet.map(([x, y]) => f.band([[x, y + 0.6], [x, y - 2.45]], 0.4)).join("");
  return [
    shape(mound, "wood-700", 1, INKED),
    shape(tuft(f, -16.5, -8.4) + tuft(f, -4, -13.7) + tuft(f, 8.5, -12.8) + tuft(f, 17.6, -5.6), "green-700", 0.6),
    shape(f.poly(face), "slate-400", 1, MAIN_INKED),
    shape(rect(f, -11.4, -7.8, 11.4, -7) + rect(f, 9.2, -7, 11.8, 0), "slate-900", 0.28),
    shape(slits.map((x) => f.ellipse(x, -5.15, 3.6, 2.4)).join(""), "red-500", 0.22),
    shape(slits.map((x) => f.ellipse(x, -5.15, 2.9, 1.3)).join(""), "ember-500", 0.35),
    shape(slits.map((x) => rect(f, x - 2.6, -5.9, x + 2.6, -4.4)).join(""), "slate-900", 1),
    shape(slits.map((x) => rect(f, x - 2.1, -5.4, x + 2.1, -4.9)).join(""), "orange-400", 1),
    shape(bags.join(""), "stone-500", 1, INKED),
    shape(posts + f.band(wire, 0.22), "slate-800", 1),
    shape(barbs.join(""), "slate-800", 1),
  ];
}

/** A grey stone university hall standing on the anchor: two wings of tall arched windows (one or two lit) under slate roofs, a central tower with a pinnacled top, a belfry, a rose window and a spire, an arched door up its steps; about 42 wide and 30 tall at scale 1. MAIN is the stone. */
function campusHall(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const eave = -15;
  const head = -23.5;
  const spire = between(rng, -31, -29.4);
  const xs = [-16.5, -12.5, -8.5, 8.5, 12.5, 16.5];
  const lit = [whole(rng, 0, 2)];
  if (rng() < 0.6) lit.push(whole(rng, 3, 5));
  const walls: Point[] = [
    [-20, 0],
    [20, 0],
    [20, eave],
    [4.6, eave],
    [4.6, head],
    [4.1, head - 3],
    [3.6, head],
    [-3.6, head],
    [-4.1, head - 3],
    [-4.6, head],
    [-4.6, eave],
    [-20, eave],
  ];
  const steps: Point[] = [
    [-7, 0],
    [7, 0],
    [7, -1],
    [6, -1],
    [6, -2],
    [-6, -2],
    [-6, -1],
    [-7, -1],
  ];
  const buttresses = [-19.3, -14.5, -10.5, -6.1, 6.1, 10.5, 14.5, 19.3].map((x) => rect(f, x - 0.35, -14.1, x + 0.35, 0)).join("");
  const roofs =
    f.poly([[-21, eave], [-4.6, eave], [-4.6, -19.2], [-19, -19.2]]) + f.poly([[21, eave], [4.6, eave], [4.6, -19.2], [19, -19.2]]) + f.poly([[-3.4, head], [3.4, head], [0, spire]]);
  return [
    shape(roofs, "slate-700", 1, INKED),
    shape(f.poly(walls), "slate-400", 1, MAIN_INKED),
    shape(rect(f, 2.5, head, 4.6, -2) + rect(f, 17.8, eave, 20, 0), "slate-900", 0.18),
    shape(buttresses + rect(f, -20, eave, -4.6, -14.2) + rect(f, 4.6, eave, 20, -14.2) + rect(f, -4.6, head + 0.9, 4.6, head + 0.1) + rect(f, -4.6, -10.3, 4.6, -9.6), "slate-700", 0.45),
    shape(xs.map((x) => arched(f, x, 1.2, -2.8, -10.2)).join("") + f.circle(0, -12.4, 1.6) + arched(f, 0, 1.4, -16.2, -19.8), "slate-800", 1),
    shape(lit.map((k) => f.ellipse(xs[k] ?? 0, -7, 2.4, 5.4)).join(""), "orange-400", 0.2),
    shape(lit.map((k) => arched(f, xs[k] ?? 0, 0.8, -3.2, -10.2)).join(""), "orange-400", 0.95),
    shape(arched(f, 0, 2.1, -2, -6.5), "wood-800", 1),
    shape(f.poly(steps), "stone-500", 1, INKED),
    shape(f.poly([[-3.4, head], [0, spire], [-1.2, head]]), "slate-100", 0.3),
  ];
}

/** The Great Wall across the whole box (x -2 to 102), as terrain: a crenellated ribbon rising and falling over ridges within about 8 of the anchor's height (its mean; the relief, its thickness and its merlons scale with the placement), a ridge wash below it down past the bottom, one or two watchtowers shifted by the anchor's x, one with a bricked-up gate. MAIN is the wall. */
function longWall(at: PartPlace, rng: Rng): KitShape[] {
  const s = at.s;
  const tau = 2 * Math.PI;
  const [p1, p2] = [rng() * tau, rng() * tau];
  const amp = 7.5 * s;
  const crest = (x: number): number => {
    const u = at.flip === 1 ? x : 100 - x;
    return at.y + amp * (0.68 * Math.sin((u / 72) * tau + p1) + 0.32 * Math.sin((u / 38) * tau + p2));
  };
  const n = 10;
  const step = (FAR_EDGE - EDGE) / n;
  const xs = Array.from({ length: n + 1 }, (_, i) => EDGE + i * step);
  const ys = xs.map(crest);
  const yAt = (i: number): number => ys[Math.max(0, Math.min(n, i))] ?? at.y;
  const midY = (i: number): number => (yAt(i) + yAt(i + 1)) / 2;
  const top = (x: number): number => {
    const i = Math.max(1, Math.min(n - 1, Math.round((x - EDGE) / step)));
    const x0 = (xs[i] ?? 0) - step / 2;
    if (x < EDGE + step / 2) return yAt(0) + ((midY(0) - yAt(0)) * (x - EDGE)) / (step / 2);
    if (x > FAR_EDGE - step / 2) return midY(n - 1) + ((yAt(n) - midY(n - 1)) * (x - FAR_EDGE + step / 2)) / (step / 2);
    const t = (x - x0) / step;
    return (1 - t) ** 2 * midY(i - 1) + 2 * t * (1 - t) * yAt(i) + t * t * midY(i);
  };
  const chain = (dy: number, back: boolean): string => {
    const at = (x: number, y: number): string => pt(x, y + dy);
    let d = "";
    if (!back) {
      d += `M${at(EDGE, yAt(0))}L${at(EDGE + step / 2, midY(0))}`;
      for (let i = 1; i < n; i += 1) d += `Q${at(xs[i] ?? 0, yAt(i))} ${at((xs[i] ?? 0) + step / 2, midY(i))}`;
      return `${d}L${at(FAR_EDGE, yAt(n))}`;
    }
    d += `L${at(FAR_EDGE, yAt(n))}L${at(FAR_EDGE - step / 2, midY(n - 1))}`;
    for (let i = n - 1; i >= 1; i -= 1) d += `Q${at(xs[i] ?? 0, yAt(i))} ${at((xs[i] ?? 0) - step / 2, midY(i - 1))}`;
    return `${d}L${at(EDGE, yAt(0))}Z`;
  };
  const ribbon = (from: number, to: number): string => chain(from, false) + chain(to, true);
  const h = 4.2 * s;
  const ridge = `M${pt(EDGE, FAR_EDGE)}L${chain(h, false).slice(1)}L${pt(FAR_EDGE, FAR_EDGE)}Z`;
  const spacing = Math.max(2.8, 3.4 * s);
  const half = 0.75 * s;
  let merlons = "";
  for (let x = EDGE + spacing / 2; x < FAR_EDGE; x += spacing) {
    const [l, r] = [top(x - half), top(x + half)];
    merlons += `M${pt(x - half, l + 0.4 * s)}L${pt(x - half, l - 1.4 * s)}L${pt(x + half, r - 1.4 * s)}L${pt(x + half, r + 0.4 * s)}Z`;
  }
  const shiftX = (dx: number): number => Math.max(8, Math.min(92, at.x + at.flip * dx * Math.min(1, s)));
  const towerXs = [shiftX(-20 + between(rng, -3, 3))];
  if (rng() < 0.65) towerXs.push(shiftX(21 + between(rng, -3, 3)));
  const gated = whole(rng, 0, towerXs.length - 1);
  const towers: string[] = [];
  const windows: string[] = [];
  const shades: string[] = [];
  let gate = "";
  let mortar = "";
  towerXs.forEach((tx, k) => {
    const t = local({ x: tx, y: top(tx) + h, s, flip: at.flip });
    const T = -4.2 - 8;
    const outline: Point[] = [
      [-4.5, 1.5],
      [4.5, 1.5],
      [4.5, T],
      [3, T],
      [3, T - 1.5],
      [2, T - 1.5],
      [2, T],
      [0.5, T],
      [0.5, T - 1.5],
      [-0.5, T - 1.5],
      [-0.5, T],
      [-2, T],
      [-2, T - 1.5],
      [-3, T - 1.5],
      [-3, T],
      [-4.5, T],
    ];
    towers.push(t.poly(outline));
    shades.push(rect(t, 2.6, T, 4.5, 1.5));
    windows.push(arched(t, -2, 0.75, T + 4.4, T + 3) + arched(t, 2, 0.75, T + 4.4, T + 3));
    if (k === gated) {
      windows.push(arched(t, 0, 2.2, 1.5, -2.4) + arched(t, 0, 1.5, 1.5, -2.4));
      gate = arched(t, 0, 1.5, 1.5, -2.4);
      for (const y of [0.3, -0.7, -1.7, -2.7]) mortar += t.band([[-1.4, y], [1.4, y]], 0.22);
      for (const [x, y0, y1] of [
        [0, 0.41, 1.4],
        [-0.7, -0.59, 0.19],
        [0.7, -0.59, 0.19],
        [0, -1.59, -0.81],
        [-0.7, -2.59, -1.81],
        [0.7, -2.59, -1.81],
      ] as const)
        mortar += t.band([[x, y0], [x, y1]], 0.22);
    }
  });
  return [
    shape(ridge, "slate-400", 0.4),
    shape(merlons, "stone-500", 1, MAIN),
    shape(ribbon(0, h), "stone-500", 1, MAIN_INKED),
    shape(ribbon(0.5 * s, 1.3 * s), "slate-100", 0.35),
    shape(ribbon(0.6 * h, h), "slate-900", 0.2),
    shape(towers.join(""), "stone-500", 1, MAIN_INKED),
    shape(shades.join(""), "slate-900", 0.2),
    shape(gate, "slate-700", 0.4),
    shape(mortar, "slate-900", 0.35),
    shape(windows.join(""), "slate-900", 0.75),
  ];
}

/** A grim stone prison on a sea crag standing on the anchor: a jagged rock with foam at its foot, a block of barred windows (one cell lit), a guard tower whose lamp room burns at its top; about 36 wide and 31 tall at scale 1. MAIN is the stone of the block and tower. */
function seaPrison(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.6, 0.6);
  const crag: Point[] = [
    [-17.5, 0],
    [-15.5 + j(), -3.5],
    [-16.2 + j(), -6.4],
    [-13, -9.2],
    [-6 + j(), -10],
    [3, -9.6],
    [9 + j(), -10.2],
    [13.8, -9.4],
    [15.6 + j(), -6],
    [14.6, -3.2],
    [18, 0],
  ];
  const walls: Point[] = [
    [-12, -9.2],
    [13, -9.2],
    [13, -24.6],
    [13.8, -24.6],
    [13.8, -25.8],
    [7.2, -25.8],
    [7.2, -24.6],
    [8, -24.6],
    [8, -21.4],
    [-12, -21.4],
  ];
  const cols = [-9.6, -5.4, -1.2, 3];
  const windows: Point[] = [];
  for (const y of [-12.2, -16.4]) for (const x of cols) windows.push([x, y]);
  const cell = windows[whole(rng, 0, windows.length - 1)] ?? [-9.6, -12.2];
  const lamp: Point = [10.5, -27.1];
  return [
    shape(f.poly(crag) + f.poly([[7.6, -28.4], [13.4, -28.4], [10.5, -31.2]]), "slate-800", 1, INKED),
    shape(f.poly([[-15.2, -3.4], [-12.5, -8.6], [-9.5, -6], [-12, -1.5]]) + f.poly([[0, -9.2], [6, -9.6], [3.4, -5.4]]), "slate-400", 0.3),
    shape(f.ellipse(-15.5, -0.2, 3.4, 0.8) + f.ellipse(-7, 0.3, 2.6, 0.6) + f.ellipse(5, 0.2, 2.8, 0.6) + f.ellipse(15.5, -0.2, 3.2, 0.8) + f.circle(-18.5, -1.8, 0.5) + f.circle(17.8, -2.2, 0.45), "white", 0.75),
    shape(f.poly(walls), "slate-700", 1, MAIN_INKED),
    shape(rect(f, -12, -21.4, 8, -20.6) + rect(f, 4.6, -20.6, 8, -9.2) + rect(f, 11.6, -24.6, 13, -9.2), "slate-900", 0.28),
    shape(
      windows.map(([x, y]) => rect(f, x - 0.9, y, x + 0.9, y - 2.6)).join("") +
        rect(f, 10.1, -14, 10.9, -18) +
        [8.4, 10.5, 12.6].map((x) => rect(f, x - 0.3, -28.4, x + 0.3, -25.8)).join(""),
      "slate-900",
      1,
    ),
    shape(rect(f, cell[0] - 0.7, cell[1] - 0.2, cell[0] + 0.7, cell[1] - 2.4) + rect(f, 8.7, -28.2, 10.2, -26) + rect(f, 10.8, -28.2, 12.3, -26), "orange-400", 0.95),
    shape(windows.map(([x, y]) => rect(f, x - 0.42, y, x - 0.17, y - 2.6) + rect(f, x + 0.17, y, x + 0.42, y - 2.6)).join(""), "slate-400", 0.95),
    shape(f.circle(...lamp, 5), "orange-400", 0.14),
    shape(f.circle(...lamp, 2.9), "orange-400", 0.26),
  ];
}

/** A towering gunmetal corporate fortress standing on the anchor: three stepped blocky tiers banded with dark window strips (a few cells lit), racks of pointed missiles on the ledges at its corners, a blast door, a mast, red beacons glowing at its corners and tip; about 30 wide and 44 tall at scale 1. MAIN is the gunmetal. */
function fortress(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const mast = between(rng, -44.4, -43.2);
  const body: Point[] = [
    [-15, 0],
    [15, 0],
    [15, -14],
    [11, -14],
    [11, -28],
    [7, -28],
    [7, -39.5],
    [-7, -39.5],
    [-7, -28],
    [-11, -28],
    [-11, -14],
    [-15, -14],
  ];
  const strips: [number, number][] = [
    [-5.8, 13],
    [-10.4, 13],
    [-18.6, 9.5],
    [-23.4, 9.5],
    [-32.2, 5.5],
    [-36.2, 5.5],
  ];
  const lit: string[] = [];
  for (let k = 0; k < 4; k += 1) {
    const [y, half] = strips[whole(rng, 0, strips.length - 1)] ?? [-5.8, 13];
    const x = Math.round(between(rng, -half + 1, half - 2) / 1.5) * 1.5 + 0.3 * k;
    if (k < 2 || rng() < 0.5) lit.push(rect(f, x, y - 0.5, x + 0.9, y + 0.1));
  }
  const racks: [number, number][] = [
    [-13, -14],
    [13, -14],
    [-9, -28],
    [9, -28],
  ];
  const missile = (x: number, y: number): string =>
    f.poly([
      [x - 0.38, y],
      [x - 0.38, y - 2.8],
      [x, y - 3.7],
      [x + 0.38, y - 2.8],
      [x + 0.38, y],
    ]);
  const beacons: Point[] = [
    [0, mast - 0.4],
    [-6.3, -40.1],
    [6.3, -40.1],
    [-11, -28.6],
    [11, -28.6],
  ];
  const dark =
    racks.map(([x, y]) => rect(f, x - 1.6, y - 1.2, x + 1.6, y)).join("") +
    rect(f, -3.2, -5.2, 3.2, 0) +
    f.band([[0, -39.5], [0, mast + 0.3]], 0.6) +
    f.band([[-1.5, -41.8], [-0.3, -41.8]], 0.35) +
    f.band([[0.3, -41.8], [1.5, -41.8]], 0.35);
  return [
    shape(f.poly(body), "slate-700", 1, MAIN_INKED),
    shape(rect(f, 10, -14, 15, 0) + rect(f, 7.3, -28, 11, -14) + rect(f, 4.6, -39.5, 7, -28), "slate-900", 0.3),
    shape(f.band([[-15, -13.75], [-11, -13.75]], 0.4) + f.band([[-11, -27.75], [-7, -27.75]], 0.4) + f.band([[-7, -39.25], [7, -39.25]], 0.4) + rect(f, -14.8, -13.5, -14.2, 0), "slate-400", 0.45),
    shape(strips.map(([y, half]) => rect(f, -half, y - 0.7, half, y + 0.3)).join("") + rect(f, -2.6, -4.6, 2.6, -0.1), "slate-900", 0.75),
    shape(lit.join(""), "sky-300", 0.85),
    shape(dark, "slate-900", 1),
    shape(racks.map(([x, y]) => [-1, 0, 1].map((dx) => missile(x + dx, y - 1.2)).join("")).join(""), "slate-400", 1),
    shape(beacons.map(([x, y]) => f.circle(x, y, 3.3)).join(""), "red-500", 0.1),
    shape(beacons.map(([x, y]) => f.circle(x, y, 1.9)).join(""), "red-500", 0.22),
    shape(beacons.map(([x, y]) => f.circle(x, y, between(rng, 0.55, 0.75))).join(""), "red-500", 1),
  ];
}

/** A soot-black factory standing on the anchor: a sawtooth roof with lit glazing, two or three tall chimneys behind it (their smoke is the recipe's), a row of sickly green windows, a loading opening at its right with a conveyor belt carrying lumps out of it on legs; about 43 wide and 28 tall at scale 1. MAIN is the walls. */
function factory(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const chimneys = whole(rng, 2, 3) === 2 ? [-11, -2.5] : [-13.5, -6.5, 0.5];
  const stacks = chimneys.map((x) => [x, between(rng, -28, -25.5)] as const);
  const shed: Point[] = [
    [-18, 0],
    [18, 0],
    [18, -12],
  ];
  for (let k = 0; k < 5; k += 1) shed.push([18 - 7.2 * k, -16.2], [18 - 7.2 * (k + 1), -12]);
  const windowXs = [-15.5, -11.5, -7.5, -3.5, 0.5, 4.5];
  const dark = whole(rng, 0, 5);
  const panes = windowXs
    .filter((_, k) => k !== dark)
    .map((x) => rect(f, x - 1.1, -9.2, x - 0.15, -5.2) + rect(f, x + 0.15, -9.2, x + 1.1, -5.2))
    .join("");
  const darkX = windowXs[dark] ?? 0;
  const belt: [Point, Point] = [
    [10, -2.8],
    [24.5, -6.6],
  ];
  const [b0, b1] = belt;
  const length = Math.hypot(b1[0] - b0[0], b1[1] - b0[1]);
  const [nx, ny] = [(b1[1] - b0[1]) / length, -(b1[0] - b0[0]) / length];
  const onBelt = (t: number, lift: number): Point => [b0[0] + (b1[0] - b0[0]) * t + nx * lift, b0[1] + (b1[1] - b0[1]) * t + ny * lift];
  const legs = [17, 22.5].map((x) => {
    const y = b0[1] + ((b1[1] - b0[1]) * (x - b0[0])) / (b1[0] - b0[0]);
    return f.band([[x, y], [x, 0]], 0.5);
  });
  return [
    shape(stacks.map(([x, y]) => f.poly([[x - 1.5, -12], [x + 1.5, -12], [x + 1.15, y], [x - 1.15, y]])).join(""), "slate-900", 1, INKED),
    shape(stacks.map(([x, y]) => rect(f, x - 1.45, y, x + 1.45, y + 0.9) + rect(f, x - 1.35, y + 4, x + 1.35, y + 4.7)).join("") + legs.join(""), "slate-700", 0.9),
    shape(f.poly(shed), "slate-800", 1, MAIN_INKED),
    shape([0, 1, 2, 3, 4].map((k) => rect(f, 18 - 7.2 * k - 1.05, -15.3, 18 - 7.2 * k - 0.3, -12.4)).join(""), "green-400", 0.6),
    shape(panes, "green-400", 0.85),
    shape(f.ellipse(-5.5, -7.2, 13.5, 4.4), "green-400", 0.15),
    shape(rect(f, 8.5, -7, 15, 0) + rect(f, darkX - 1.1, -9.2, darkX + 1.1, -5.2), "slate-900", 1),
    shape(f.ellipse(11.75, -2, 2.8, 1.8), "green-400", 0.3),
    shape(f.band(belt, 1.1), "slate-900", 1, INKED),
    shape([0.45, 0.68, 0.9].map((t) => f.circle(...onBelt(t + between(rng, -0.04, 0.04), 1.2), 0.7)).join(""), "slate-400", 1),
    shape([-13.5, -5.5, 2.5, 16.5].map((x) => f.poly([[x - 0.5, -12], [x + 0.5, -12], [x + 0.1, -12 + between(rng, 2, 4)]])).join(""), "slate-900", 0.35),
  ];
}

/** A long low mountain-road diner standing on the anchor: pale walls with a red stripe, a row of lit windows and a lit glass door spilling light, a red fascia under its flat roof, a blank red sign board edged with bulbs on two posts above it; about 32 wide and 16 tall at scale 1. MAIN is the red trim and sign. */
function diner(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const panes: string[] = [];
  for (let k = 0; k < 4; k += 1) {
    const x = -13.4 + 4.9 * k;
    panes.push(rect(f, x, -7.6, x + 1.95, -3.4) + rect(f, x + 2.25, -7.6, x + 4.2, -3.4));
  }
  const bulbs = Array.from({ length: 7 }, (_, k) => f.circle(-6 + 2 * k, -15.85, 0.28)).join("");
  const vent = rng() < 0.6 ? rect(f, 10, -11.8, 12.6, -10.6) : rect(f, -13, -11.6, -10.8, -10.6);
  return [
    shape(rect(f, -15, -9, 15, 0), "white", 1, INKED),
    shape(rect(f, -15, -9, 15, -8.2) + rect(f, 12.6, -8.2, 15, 0), "slate-400", 0.35),
    shape(rect(f, -15, -2.4, 15, -1.4), "red-600", 1, MAIN),
    shape(rect(f, 8.5, -7.4, 11.5, 0), "slate-800", 1, INKED),
    shape(panes.join("") + rect(f, 9.1, -6.8, 10.9, -4), "orange-400", 0.9),
    shape(f.ellipse(-4, -5.5, 12.5, 4.1) + f.ellipse(-2, 0.7, 13, 1.2), "orange-400", 0.17),
    shape(rect(f, -15.8, -10.6, 15.8, -9) + rect(f, -7, -16.2, 7, -12), "red-600", 1, MAIN_INKED),
    shape(f.band([[-4.5, -10.6], [-4.5, -12]], 0.5) + f.band([[4.5, -10.6], [4.5, -12]], 0.5) + vent, "slate-700", 1),
    shape(rect(f, -6, -15.3, 6, -12.8) + bulbs, "white", 0.95),
  ];
}

/** A mountain railway platform standing on the anchor: a raised stone edge with a white line, a canopy roof with a fringe of dagger boards on slim braced posts, a lamp hanging lit from it and a bench; about 44 wide and 18 tall at scale 1. MAIN is the canopy. */
function station(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lampX = [-9, 0, 9][whole(rng, 0, 2)] ?? 0;
  const benchX = lampX <= 0 ? 9 : -9;
  const posts = [-13.5, -4.5, 4.5, 13.5];
  const canopy: Point[] = [
    [-18.5, -16.2],
    [0, -18],
    [18.5, -16.2],
    [18.5, -15.2],
  ];
  const teeth = 12;
  const w = 37 / teeth;
  for (let k = 0; k < teeth; k += 1) canopy.push([18.5 - w * (k + 0.5), -14.2], [18.5 - w * (k + 1), -15.2]);
  const braces = posts.flatMap((x) => [f.band([[x - 0.3, -12.8], [x - 2, -14.5]], 0.35), f.band([[x + 0.3, -12.8], [x + 2, -14.5]], 0.35)]).join("");
  const b = benchX;
  const bench =
    rect(f, b - 2.6, -7.6, b + 2.6, -6.8) +
    rect(f, b - 2.3, -6.8, b - 1.9, -5.9) +
    rect(f, b + 1.9, -6.8, b + 2.3, -5.9) +
    rect(f, b - 2.8, -5.9, b + 2.8, -5.3) +
    rect(f, b - 2.4, -5.3, b - 1.9, -4.2) +
    rect(f, b + 1.9, -5.3, b + 2.4, -4.2);
  const glow: Point = [lampX, -10.6];
  return [
    shape(rect(f, -22, -3, 22, 0) + f.poly([[-22, -3], [22, -3], [20.6, -5], [-20.6, -5]]), "stone-500", 1, INKED),
    shape(rect(f, -22, -3, 22, 0), "slate-900", 0.22),
    shape([-16.5, -11, -5.5, 0, 5.5, 11, 16.5].map((x) => f.band([[x, -0.3], [x, -2.7]], 0.3)).join(""), "slate-700", 0.45),
    shape(f.band([[-21.6, -3.55], [21.6, -3.55]], 0.4), "white", 0.85),
    shape(posts.map((x) => f.band([[x, -4.2], [x, -14.6]], 0.55)).join(""), "slate-800", 1),
    shape(f.circle(...glow, 3) + f.ellipse(lampX, -4.1, 4.5, 0.75), "orange-400", 0.22),
    shape(bench, "wood-700", 1),
    shape(f.poly(canopy), "slate-700", 1, MAIN_INKED),
    shape(f.band([[-18.3, -16.5], [0, -17.6], [18.3, -16.5]], 0.5), "slate-400", 0.4),
    shape(
      braces + f.band([[lampX, -14.6], [lampX, -12.4]], 0.25) + f.poly([[lampX - 1.1, -11.4], [lampX + 1.1, -11.4], [lampX + 0.5, -12.4], [lampX - 0.5, -12.4]]),
      "slate-900",
      1,
    ),
    shape(f.circle(...glow, 0.85), "orange-400", 1),
  ];
}

/** A school or scholar's desk standing on the anchor: its top in slight perspective over an apron and four legs, a dark modesty panel behind, sometimes a brass drawer pull; about 12 wide and 8 tall at scale 1. MAIN is the wood. */
function desk(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const g = between(rng, -0.8, 0.8);
  const out = [
    shape(
      f.poly([
        [-4.9, -1],
        [-4.3, -1],
        [-4.3, -2.2],
        [4.3, -2.2],
        [4.3, -1],
        [4.9, -1],
        [4.9, -5.2],
        [-4.9, -5.2],
      ]),
      "wood-800",
      1,
      INKED,
    ),
    shape(
      f.poly([
        [-5.8, 0],
        [-5, 0],
        [-5, -5.2],
        [5, -5.2],
        [5, 0],
        [5.8, 0],
        [5.8, -6],
        [6.1, -6],
        [6.1, -6.9],
        [5.2, -8.2],
        [-5.2, -8.2],
        [-6.1, -6.9],
        [-6.1, -6],
        [-5.8, -6],
      ]),
      "wood-700",
      1,
      MAIN_INKED,
    ),
    shape(f.poly([[-5.7, -7.05], [5.7, -7.05], [5.05, -8], [-5.05, -8]]), "slate-100", 0.3),
    shape(rect(f, -6.1, -6.9, 6.1, -6), "slate-900", 0.25),
    shape(f.band([[-4 + g, -7.4], [2.5 + g, -7.4]], 0.2) + f.band([[-1.5 + g, -7.75], [4 + g, -7.75]], 0.2), "wood-900", 0.4),
  ];
  if (rng() < 0.6) out.push(shape(f.circle(0, -5.6, 0.32), "brass-500", 1));
  return out;
}

/** R3401000: the buildings the Meditative scenes added, by id, each with its kind. */
export const INK_BUILDINGS_KIT = {
  "stake-plot": { category: "architecture", draw: stakePlot },
  schoolhouse: { category: "architecture", draw: schoolhouse },
  "glass-church": { category: "architecture", draw: glassChurch },
  bunker: { category: "architecture", draw: bunker },
  "campus-hall": { category: "architecture", draw: campusHall },
  "long-wall": { category: "architecture", draw: longWall },
  "sea-prison": { category: "architecture", draw: seaPrison },
  "peak-fortress": { category: "architecture", draw: fortress },
  factory: { category: "architecture", draw: factory },
  diner: { category: "architecture", draw: diner },
  station: { category: "architecture", draw: station },
  desk: { category: "architecture", draw: desk },
} as const satisfies Readonly<Record<string, KitPart>>;
