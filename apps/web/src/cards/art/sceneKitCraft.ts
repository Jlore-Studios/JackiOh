// R3403000: the scene kit's craft and crew in space (ART.md, "Scene recipes and kits"), which Stinkyscape's later
// cards (#3403) added: the creatures, drones and robots (a space wasp, drones, a corvette, an escort disc, robots, a
// mech, a spirit, a crowd, an alien, a dog, a jelly blob, a scrap golem, a gloved hand, a mining arm, an AI's eye)
// and the ships and machines (a starship, a fighter, a freighter, a dropship, a shuttle, a rocket, a tank, probes,
// satellites, a rover, a buoy, a ship's module, a missile). sceneKit.ts spreads them into `SCENE_KIT`. Every colour
// is a palette.css name, none in Radiant's gold window; a glow is a stack of translucent shapes, never a gradient;
// shapes are filled even-odd, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { between, DEGREES, INKED, leant, local, MAIN, MAIN_INKED, shape, sparkle, turned, whole, type Local, type Point } from "./kitDraw.ts";
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

/** `point` turned `degrees` about `pivot`. */
function about(pivot: Point, point: Point, degrees: number): Point {
  return plus(pivot, turned([point[0] - pivot[0], point[1] - pivot[1]], degrees));
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

/** A rectangle from (x0, y0) to (x1, y1), its corners cut `c` deep: a plate. */
function cut(f: Pen, x0: number, y0: number, x1: number, y1: number, c: number): string {
  return f.poly([
    [x0 + c, y0],
    [x1 - c, y0],
    [x1, y0 + c],
    [x1, y1 - c],
    [x1 - c, y1],
    [x0 + c, y1],
    [x0, y1 - c],
    [x0, y0 + c],
  ]);
}

/** A rectangle from (x0, y0) to (x1, y1), its corners rounded `r`. */
function rounded(f: Pen, x0: number, y0: number, x1: number, y1: number, r: number): string {
  return (
    `M${f.p(x0 + r, y0)}L${f.p(x1 - r, y0)}Q${f.p(x1, y0)} ${f.p(x1, y0 + r)}L${f.p(x1, y1 - r)}Q${f.p(x1, y1)} ${f.p(x1 - r, y1)}` +
    `L${f.p(x0 + r, y1)}Q${f.p(x0, y1)} ${f.p(x0, y1 - r)}L${f.p(x0, y0 + r)}Q${f.p(x0, y0)} ${f.p(x0 + r, y0)}Z`
  );
}

/** An ellipse about `c` whose semi-axes are the vectors `u` and `v`, drawn as four curves so it turns with its frame. */
function skewOval(f: Pen, c: Point, u: Point, v: Point): string {
  const k = 0.5523;
  const q = (a: number, b: number): string => f.p(c[0] + u[0] * a + v[0] * b, c[1] + u[1] * a + v[1] * b);
  return `M${q(1, 0)}C${q(1, k)} ${q(k, 1)} ${q(0, 1)}C${q(-k, 1)} ${q(-1, k)} ${q(-1, 0)}C${q(-1, -k)} ${q(-k, -1)} ${q(0, -1)}C${q(k, -1)} ${q(1, -k)} ${q(1, 0)}Z`;
}

/** An upright ellipse about (cx, cy). */
function oval(f: Pen, cx: number, cy: number, rx: number, ry: number): string {
  return skewOval(f, [cx, cy], [rx, 0], [0, ry]);
}

/** A ring about (cx, cy), from `outer` in to `inner`. */
function ring(f: Pen, cx: number, cy: number, outer: number, inner: number): string {
  return f.circle(cx, cy, outer) + f.circle(cx, cy, inner);
}

/** A tongue of flame or glow from `base`, `half` to either side of it, tapering `length` along `degrees` (0 is +x, 90 straight down). */
function tongue(f: Pen, base: Point, degrees: number, length: number, half: number): string {
  const u = heading(degrees);
  const n: Point = [-u[1], u[0]];
  const mid = plus(base, u, length * 0.45);
  return `M${f.p(...plus(base, n, half))}Q${f.p(...plus(mid, n, half))} ${f.p(...plus(base, u, length))}Q${f.p(...plus(mid, n, -half))} ${f.p(...plus(base, n, -half))}Z`;
}

// Creatures, drones and robots

/** A chitin-plated space wasp in flight, centred on the anchor and facing +x: a big-eyed head, a thorax, a striped jet-thruster abdomen burning behind, four translucent wings, legs and a stinger; about 22 long and 14 tall at scale 1. */
function spaceWasp(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -8, 8));
  const flap = between(rng, -1.2, 1.2);
  const c = between(rng, 0, 1.4);
  const burn = between(rng, 4, 7);
  const wing = (root: Point, tip: Point, w: number): string => {
    const u: Point = [tip[0] - root[0], tip[1] - root[1]];
    const length = Math.hypot(u[0], u[1]) || 1;
    const n: Point = [(-u[1] / length) * w, (u[0] / length) * w];
    const mid = plus(root, u, 0.5);
    return `M${f.p(...root)}Q${f.p(...plus(mid, n))} ${f.p(...tip)}Q${f.p(...plus(mid, n, -1))} ${f.p(...root)}Z`;
  };
  const pair = (dx: number, dy: number): string =>
    wing([2.8 + dx, -2.4 + dy], [-5 + dx, -9 + flap + dy], 3.2) + wing([0.8 + dx, -2.2 + dy], [-7.5 + dx, -5.5 + flap * 0.6 + dy], 2.2);
  const nozzle: Point = [-12.6, 0.5 + c];
  return [
    shape(pair(0, 0), "sky-300", 0.22),
    shape(
      f.band([[1.2, 2.2], [0, 4.6], [-1.4, 6.4]], 0.6) +
        f.band([[3, 2.4], [2.8, 5], [1.6, 6.8]], 0.6) +
        f.band([[4.6, 2], [5.6, 4.4], [5.2, 6.6]], 0.6) +
        f.band([[7.8, -3.4], [9.4, -6.4], [11.4, -6.9]], 0.45) +
        f.band([[6.6, -3.4], [7.4, -6.2], [9.2, -7.6]], 0.45) +
        f.poly([[8.4, 1], [10.4, 1.8], [8.8, 2]]) +
        f.poly([[-9.4, 2.4 + c], [-13.4, 5 + c], [-10.4, 1.6 + c]]),
      "slate-900",
      1,
    ),
    shape(tongue(f, nozzle, 180, burn * 1.3, 2.6), "cyan-400", 0.22),
    shape(tongue(f, nozzle, 180, burn * 0.6, 1.1), "white", 0.8),
    shape(f.poly([[-10.2, -1 + c], [-12.8, -1.8 + c], [-12.8, 2.8 + c], [-10.2, 2 + c]]), "slate-700", 1, INKED),
    shape(
      `M${f.p(-0.8, -1.2)}C${f.p(-3, -3.4)} ${f.p(-8, -3 + c)} ${f.p(-10.6, -1.4 + c)}L${f.p(-10.6, 2.4 + c)}C${f.p(-8, 4 + c)} ${f.p(-3, 3.4)} ${f.p(-0.8, 1.4)}Z`,
      "lime-400",
      1,
      MAIN_INKED,
    ),
    shape(
      f.band([[-3.4, -2 + 0.2 * c], [-3.4, 2.3 + 0.2 * c]], 1.1) +
        f.band([[-6.1, -2.2 + 0.57 * c], [-6.1, 2.8 + 0.57 * c]], 1.1) +
        f.band([[-8.8, -1.7 + 0.9 * c], [-8.8, 2.6 + 0.9 * c]], 1.1),
      "slate-900",
      0.8,
    ),
    shape(oval(f, 2.6, -0.1, 3.5, 2.7), "lime-400", 1, MAIN_INKED),
    shape(f.circle(7.4, -1, 2.7), "lime-400", 1, MAIN_INKED),
    shape(oval(f, 8.5, -1.5, 1.5, 2), "red-500", 1, INKED),
    shape(f.circle(9, -2.4, 0.45), "white", 0.85),
    shape(pair(1.6, 0.6), "slate-100", 0.3, INKED),
  ];
}

/** A small round maintenance drone, centred on the anchor: a disc body, a cyan sensor eye, two thruster pods burning below, an antenna with a tip light; about 10 wide at scale 1. */
function drone(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -10, 10));
  const look = between(rng, -0.4, 0.9);
  const sway = between(rng, -0.8, 0.8);
  const burn = between(rng, 2.2, 3.4);
  return [
    shape(tongue(f, [-4.2, 1.8], 90, burn, 0.8) + tongue(f, [4.2, 1.8], 90, burn, 0.8), "cyan-400", 0.28),
    shape(f.band([[0.4, -3], [0.6 + sway * 0.5, -4.4], [0.8 + sway, -5.4]], 0.45), "slate-700", 1),
    shape(rounded(f, -5.2, -1.1, -3.2, 1.8, 0.6) + rounded(f, 3.2, -1.1, 5.2, 1.8, 0.6), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 3.6), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-3.3, 1], [0, 1.7], [3.3, 1]], 0.55), "slate-700", 0.7),
    shape(f.band([[1.4, -3.1], [2.8, -2], [3.3, -0.2]], 0.55), "white", 0.35),
    shape(f.circle(look, -0.4, 2.3), "cyan-400", 0.2),
    shape(f.circle(look, -0.4, 1.4), "slate-900", 1, INKED),
    shape(f.circle(look, -0.4, 0.85), "cyan-400", 1),
    shape(f.circle(look + 0.3, -0.7, 0.3), "white", 0.9),
    shape(f.circle(0.8 + sway, -5.6, 0.6), "red-400", 1),
  ];
}

/** A small triangular interceptor drone, centred on the anchor and facing +x: a dart hull, one red sensor eye, a blade below, a gun barrel above and a shield plate ahead, one of the three thrust out; about 14 wide at scale 1. */
function interceptor(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -10, 10));
  const out = whole(rng, 0, 2);
  const blade = out === 0 ? 4.2 : 0.6;
  const gun = out === 1 ? 3.6 : 0.4;
  const guard = out === 2 ? 2 : 0.3;
  const burn = between(rng, 4, 6);
  return [
    shape(tongue(f, [-3.2, 0], 180, burn, 1.5), "cyan-400", 0.25),
    shape(tongue(f, [-3.2, 0], 180, burn * 0.5, 0.7), "white", 0.75),
    shape(f.poly([[-1.6, 2.6], [4.6 + blade, 3.3], [-1.6, 4.1]]), "slate-100", 1, INKED),
    shape(
      f.poly([[-1, -3], [4 + gun, -3], [4 + gun, -3.4], [5.6 + gun, -3.4], [5.6 + gun, -1.8], [4 + gun, -1.8], [4 + gun, -2.2], [-1, -2.2]]),
      "slate-800",
      1,
      INKED,
    ),
    shape(f.poly([[7.2, 0], [-4.6, -4.8], [-2.8, 0], [-4.6, 4.8]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[7.2, 0], [-4.6, -4.8], [-2.8, 0]]), "slate-400", 0.45),
    shape(f.poly([[7.6 + guard, -1.6], [8.4 + guard, 0], [7.6 + guard, 1.6], [8.4 + guard, 1.8], [9.6 + guard, 0], [8.4 + guard, -1.8]]), "slate-400", 1, INKED),
    shape(f.circle(2, 0, 2.2), "red-500", 0.25),
    shape(f.circle(2, 0, 1.1), "red-500", 1),
    shape(f.circle(2.35, -0.35, 0.35), "white", 0.85),
  ];
}

/** A chunky corvette-sized drone, centred on the anchor and facing +x: a blocky hull, three small turrets, missile-rack pods, a row of lit ports, a sensor eye sweeping a translucent cone ahead, engines burning behind; about 30 long at scale 1. */
function corvette(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -5, 5));
  const sweep = between(rng, -10, 10);
  const burn = between(rng, 5, 8);
  const eye: Point = [13.4, -0.4];
  const cone = (reach: number, spread: number): string => {
    const T = (d: number, s: number): Point => plus(eye, heading(sweep + s), d);
    return f.poly([eye, T(reach, -spread), T(reach * 1.03, -spread / 2), T(reach * 1.04, 0), T(reach * 1.03, spread / 2), T(reach, spread)]);
  };
  const turrets = [-7, 0, 7];
  const barrels = turrets.map((x) => f.band([[x, -6.2], plus([x, -6.2], heading(between(rng, -14, 4)), 4)], 0.6)).join("");
  const domes = turrets.map((x) => `M${f.p(x - 1.9, -4.6)}Q${f.p(x - 1.9, -7.2)} ${f.p(x, -7.2)}Q${f.p(x + 1.9, -7.2)} ${f.p(x + 1.9, -4.6)}Z`).join("");
  const missiles = [-3, 7]
    .flatMap((x) =>
      [4.8, 5.9].map((y) =>
        f.poly([
          [x, y - 0.4],
          [x + 1.3, y],
          [x, y + 0.4],
        ]),
      ),
    )
    .join("");
  const windows = [-11.5, -9.5, -7.5, -1.8, 0.2, 2.2, 6, 8, 10]
    .filter((_, i) => rng() < 0.75 || i === 0)
    .map((x) => box(f, x - 0.55, -1.3, x + 0.55, -0.5))
    .join("");
  const nozzles: Point[] = [
    [-14, -1.8],
    [-14, 1.4],
  ];
  return [
    shape(cone(12, 22), "cyan-400", 0.12),
    shape(cone(8, 12), "cyan-400", 0.2),
    shape(nozzles.map((n) => tongue(f, n, 180, burn, 1.3)).join(""), "cyan-400", 0.25),
    shape(barrels + box(f, -9, 4, -3, 6.6) + box(f, 1, 4, 7, 6.6), "slate-800", 1, INKED),
    shape(missiles, "red-500", 1),
    shape(f.poly([[-14, -3.6], [-11, -4.8], [9, -4.8], [13, -3], [15.5, -0.5], [15.5, 1.4], [12.5, 4.2], [-11, 4.2], [-14, 2.8]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-14, -3.6], [-11, -4.8], [9, -4.8], [13, -3], [14.2, -1.8], [-14, -1.8]]), "slate-400", 0.5),
    shape(
      f.band([[-4, -1.6], [-4, 4]], 0.35) +
        f.band([[4, -1.6], [4, 4]], 0.35) +
        f.band([[-13.6, 1.6], [-4.4, 1.6]], 0.35) +
        f.band([[-3.6, 1.6], [3.6, 1.6]], 0.35) +
        f.band([[4.4, 1.6], [12.8, 1.6]], 0.35),
      "slate-900",
      0.55,
    ),
    shape(windows, "cyan-400", 1),
    shape(domes, "slate-400", 1, INKED),
    shape(f.circle(...eye, 1.3), "cyan-400", 1, INKED),
    shape(nozzles.map((n) => tongue(f, n, 180, burn * 0.5, 0.6)).join("") + f.circle(...eye, 0.5), "white", 0.8),
  ];
}

/** A polite hovering AI, centred on the anchor: a white rounded disc, a soft blue face-light (two eyes and a smile), two thin courtesy arms with light tips (one sometimes raised), a hover glow below; about 14 wide at scale 1. */
function escortDisc(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -8, 8));
  const wave = rng() < 0.5;
  const lift = between(rng, -0.6, 0.6);
  const hand: Point = wave ? [8.6, -2.6 + lift] : [7.4, 5.4 + lift];
  const elbow: Point = wave ? [7.6, 0.6] : [7.4, 3];
  const rest: Point = [-6.8, 5.6];
  return [
    shape(oval(f, 0, 8, 4.6, 1.5), "sky-300", 0.12),
    shape(oval(f, 0, 7.4, 2.6, 0.9), "sky-300", 0.25),
    shape(f.band([[-5.4, 1.6], [-7, 3.4], rest], 0.5) + f.band([[5.4, 1.6], elbow, hand], 0.5), "slate-400", 1),
    shape(oval(f, 0, 0, 6, 5.2), "white", 1, MAIN_INKED),
    shape(`M${f.p(-5.7, 1.6)}Q${f.p(0, 6.8)} ${f.p(5.7, 1.6)}Q${f.p(0, 4.2)} ${f.p(-5.7, 1.6)}Z`, "slate-400", 0.35),
    shape(rounded(f, -1.6, 4.6, 1.6, 6, 0.6), "slate-400", 1, INKED),
    shape(oval(f, 0.7, -0.6, 3.8, 2.6), "slate-900", 0.92, INKED),
    shape(oval(f, 0.7, -0.6, 3.2, 2.1), "sky-300", 0.2),
    shape(oval(f, -0.6, -1.2, 0.5, 0.75) + oval(f, 2, -1.2, 0.5, 0.75) + f.band([[-0.8, 0.4], [0.7, 1.3], [2.2, 0.4]], 0.45), "sky-300", 1),
    shape(f.circle(...rest, 1.2) + f.circle(...hand, 1.2), "sky-300", 0.25),
    shape(f.circle(...rest, 0.6) + f.circle(...hand, 0.6), "sky-300", 1),
  ];
}

/** A small boxy robot standing on the anchor: a box head with a visor, a torso with a chest panel, two arms (one sometimes waving), legs and an antenna; about 10 wide and 18 tall at scale 1. */
function robot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wave = rng() < 0.4;
  const swing = between(rng, -0.6, 0.6);
  const front = wave
    ? f.band([[3.4, -11.6], [5.4, -12.8], [5.6, -16.2]], 1.2)
    : f.band([[3.4, -11.6], [4.9, -8.8 - swing], [4.6, -6]], 1.2);
  return [
    shape(f.band([[-3.4, -11.6], [-4.9, -8.8 + swing], [-4.6, -6]], 1.2) + f.band([[0.4, -16.4], [0.8, -18.6]], 0.5), "slate-700", 1, INKED),
    shape(
      f.poly([[-2.6, -5.8], [-0.8, -5.8], [-0.8, -1.1], [-0.2, -1.1], [-0.2, 0], [-3, 0], [-3, -1.1], [-2.6, -1.1]]) +
        f.poly([[0.8, -5.8], [2.6, -5.8], [2.6, -1.1], [3.4, -1.1], [3.4, 0], [0.6, 0], [0.6, -1.1], [0.8, -1.1]]) +
        box(f, -0.8, -13.2, 0.8, -12.2),
      "slate-700",
      1,
      INKED,
    ),
    shape(rounded(f, -3.6, -12.4, 3.6, -5.4, 0.8), "slate-400", 1, MAIN_INKED),
    shape(f.band([[3, -11.4], [3, -6.4]], 0.5), "white", 0.3),
    shape(rounded(f, -2.2, -11.2, 2.2, -7.4, 0.4), "slate-700", 0.85),
    shape(f.circle(0.9, -9.3, 0.6) + box(f, -1.6, -10.3, -0.4, -9.7) + box(f, -1.6, -8.9, -0.4, -8.3), "cyan-400", 1),
    shape(front, "slate-700", 1, INKED),
    shape(rounded(f, -2.8, -16.6, 2.8, -12.6, 0.7), "slate-400", 1, MAIN_INKED),
    shape(rounded(f, -1.8, -15.7, 2.4, -13.6, 0.5), "slate-900", 1, INKED),
    shape(box(f, -0.9, -15.1, 0, -14.3) + box(f, 0.9, -15.1, 1.8, -14.3), "cyan-400", 1),
    shape(f.circle(0.8, -18.9, 0.55), "red-400", 1),
  ];
}

/** A toppled robot lying on its side, its side on the anchor's ground: the box body tipped over, its legs and arms splayed, its head's visor cracked and flickering, sparks at its neck, a wisp of smoke; about 20 long and 11 tall at scale 1. */
function robotWreck(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const kick = between(rng, -0.8, 0.8);
  const reach = between(rng, -0.8, 0.8);
  return [
    shape(oval(f, 4.8 + kick * 0.5, -10.4, 2, 1.3) + oval(f, 6.6 + kick, -13.6, 1.4, 1), "slate-400", 0.18),
    shape(
      f.band([[-4, -1.4], [-7.2, -1 + kick * 0.3], [-9.6, -0.9]], 1.5) +
        f.band([[-4, -5.6], [-6.8, -7.8 - kick], [-9.2, -6.6 - kick]], 1.5) +
        f.band([[1.6, -7.2], [2.4, -10 + reach], [4.8, -11.2 + reach]], 1.2) +
        f.band([[-1.4, -7.2], [-2.6, -9.6], [-4.8, -10.4 - reach * 0.5]], 1.2) +
        f.band([[7.8, -3], [9.2, -2.8], [9.9, -1.4]], 0.5),
      "slate-700",
      1,
      INKED,
    ),
    shape(rounded(f, -11, -1.8, -9.2, 0, 0.4) + rounded(f, -10.4, -7.6 - kick, -8.6, -5.6 - kick, 0.4) + box(f, 3, -3.6, 3.8, -2), "slate-800", 1, INKED),
    shape(rounded(f, -4, -7.2, 3, 0, 0.8), "slate-400", 1, MAIN_INKED),
    shape(rounded(f, -2.6, -5.8, 1.6, -1.6, 0.4), "slate-700", 0.85),
    shape(rounded(f, 3.6, -5.6, 7.8, 0, 0.7), "slate-400", 1, MAIN_INKED),
    shape(rounded(f, 5.2, -4.9, 7.1, -0.7, 0.5), "slate-900", 1, INKED),
    shape(box(f, 5.6, -4.4, 6.6, -3.6), "cyan-400", 0.55),
    shape(f.band([[5.3, -3.8], [6.2, -3.1], [5.6, -2.3], [6.6, -1.6], [7, -1]], 0.28), "white", 0.75),
    shape(f.circle(3.4, -6.4, 2), "orange-400", 0.25),
    shape(sparkle(f, 3.4, -6.4, 1.6) + sparkle(f, 8.8, -5.8, 1), "orange-400", 1),
  ];
}

/** A boxy combat robot standing on the anchor: a square torso, a red visor, four mismatched weapon arms (a raised saw, a cannon, a claw, a hammer), thick legs; about 20 wide (its weapons a little past) and 28 tall at scale 1. */
function combatRobot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spin = between(rng, 0, 30);
  const swing = between(rng, -1, 1);
  const firing = rng() < 0.5;
  const saw: Point = [-9.4, -25.6];
  const teeth: Point[] = [];
  for (let i = 0; i < 12; i += 1) teeth.push(polar(3.1, spin + i * 30, ...saw), polar(2.3, spin + i * 30 + 8, ...saw));
  const out = [
    shape(
      f.poly([[-5.4, -10], [-1.2, -10], [-1.4, -2.2], [-0.6, -2.2], [-0.6, 0], [-6.6, 0], [-6.6, -2.2], [-5.6, -2.2]]) +
        f.poly([[1.2, -10], [5.4, -10], [5.6, -2.2], [6.6, -2.2], [6.6, 0], [0.6, 0], [0.6, -2.2], [1.4, -2.2]]),
      "slate-700",
      1,
      INKED,
    ),
    shape(
      f.band([[-5, -20.4], [-8.4, -21.4], [-9.4, -24]], 1.8) +
        f.band([[-5, -14], [-7.8, -12.4 + swing * 0.5], [-8.4, -9.6]], 1.8) +
        f.band([[5, -20.4], [8.6, -20.1]], 1.8) +
        f.band([[5, -14], [8.2, -12.6], [9.4, -10.6]], 1.8),
      "slate-700",
      1,
      INKED,
    ),
    shape(cut(f, -5.4, -22, 5.4, -9.4, 1.6), "slate-400", 1, MAIN_INKED),
    shape(cut(f, -8, -23.4, -4.4, -18.8, 0.8) + cut(f, 4.4, -23.4, 8, -18.8, 0.8) + cut(f, -3.4, -15.8, 3.4, -11.6, 0.6), "slate-700", 1, INKED),
    shape(f.poly(teeth) + f.circle(...saw, 0.8), "slate-100", 1, INKED),
    shape(
      f.poly([[8.2, -21.8], [10.8, -21.8], [10.8, -21.1], [13.2, -21.1], [13.2, -19.5], [10.8, -19.5], [10.8, -18.4], [8.2, -18.4]]) +
        f.band([[9.8, -10.8], [11.6, -9.6], [11.8, -7.6]], 0.7) +
        f.band([[9, -10], [9.4, -8.2], [10.6, -7]], 0.7) +
        f.band([[-8.4, -9.6], [-8.6, -5.6]], 0.7),
      "slate-800",
      1,
      INKED,
    ),
    shape(cut(f, -10.8, -5.8, -6.4, -2.6, 0.5), "brass-600", 1, INKED),
    shape(rounded(f, -3, -27, 3, -21.6, 0.8), "slate-400", 1, MAIN_INKED),
    shape(oval(f, 0.4, -24.7, 4.2, 2.4), "red-500", 0.25),
    shape(rounded(f, -2.2, -25.6, 2.8, -23.8, 0.5), "red-500", 1),
    shape(f.band([[-1.4, -25.1], [0.4, -25.1]], 0.35), "white", 0.7),
  ];
  if (firing) out.push(shape(sparkle(f, 14.4, -20.3, 2.2), "orange-400", 0.9));
  return out;
}

/** A towering mech standing on the anchor: a cockpit head with a canopy, an armoured torso with a glowing core, big shoulders, an arm cannon aimed toward its facing, the other arm's fist, two legs; about 22 wide (its cannon past that) and 44 tall at scale 1. */
function mech(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const aim = between(rng, -14, 6);
  const stride = between(rng, -0.6, 0.6);
  const pivot: Point = [9.2, -29];
  const A = (x: number, y: number): Point => about(pivot, [x, y], aim);
  return [
    shape(
      f.poly([[-5.4, -21], [-1.4, -21], [-2.4 + stride, -12.5], [-2.4, -3], [-5.2, -3], [-6.4 + stride, -12.5]]) +
        f.poly([[1.4, -21], [5.4, -21], [6.4 - stride, -12.5], [5.2, -3], [2.4, -3], [2.4 - stride, -12.5]]) +
        f.band([[-2, -41], [-2.6, -44.4]], 0.5),
      "slate-700",
      1,
      INKED,
    ),
    shape(
      f.poly([[-10.2, -33], [-7, -33], [-7.6, -22.4], [-10.8, -22.4]]) +
        rounded(f, -11.2, -22.4, -7.4, -18.6, 0.8) +
        box(f, -4, -22.4, 4, -19) +
        box(f, 7.8, -31.6, 10.8, -27),
      "slate-700",
      1,
      INKED,
    ),
    shape(
      f.poly([[-6.6, 0], [-6.4, -3.2], [-2, -3.2], [-0.8, 0]]) +
        f.poly([[1.4, 0], [1.8, -3.2], [5.8, -3.2], [7.4, 0]]) +
        cut(f, -7, -15, -1.8, -11, 0.8) +
        cut(f, 1.8, -15, 7, -11, 0.8),
      "slate-400",
      1,
      MAIN_INKED,
    ),
    shape(f.poly([[-7.5, -35], [7.5, -35], [8.5, -30], [6, -24], [3.5, -21], [-3.5, -21], [-6, -24], [-8.5, -30]]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-5.6, -31.4], [5.6, -31.4]], 0.5) + f.band([[-4.6, -27], [-3, -22.6]], 0.5) + f.band([[4.6, -27], [3, -22.6]], 0.5), "slate-700", 0.6),
    shape(f.circle(0, -27.4, 3.4), "cyan-400", 0.25),
    shape(f.poly([A(8.4, -27.2), A(13.6, -27.2), A(13.6, -26.2), A(17.4, -26.2), A(17.4, -24), A(13.6, -24), A(13.6, -23), A(8.4, -23)]), "slate-800", 1, INKED),
    shape(cut(f, -11.6, -37.4, -6, -30, 1.5) + cut(f, 6, -37.4, 11.6, -30, 1.5), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-3.4, -34.6], [-3.8, -39], [-1.5, -41.6], [3, -41.6], [5, -38.6], [4.2, -34.6]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[0.2, -40.4], [2.8, -40.4], [4.1, -38.5], [3.5, -36.4], [0.2, -36.4]]) + f.circle(0, -27.4, 1.8), "cyan-400", 1, INKED),
    shape(f.band([[1, -39.6], [2.6, -39.6]], 0.4) + f.circle(0.4, -27.8, 0.6), "white", 0.8),
  ];
}

/** A translucent glowing humanoid standing on the anchor, the figure kit's height: a head and a robed body as stacked translucent silhouettes in a glow, arms at its sides, pointing toward its facing or palms together, leaning a little, motes about it; about 56 tall at scale 1. */
function spirit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.06, 0.14);
  const pose = whole(rng, 0, 2);
  const L = (x: number, y: number): Point => leant([x, y], lean);
  const silhouette = (kx: number, ky: number): string => {
    const S = (x: number, y: number): string => f.p(...L(x * kx, -28 + (y + 28) * ky));
    return (
      `M${S(-2, -44.6)}Q${S(-6.6, -44.4)} ${S(-7, -40)}L${S(-5.4, -26)}Q${S(-6.9, -12)} ${S(-6.4, 0)}` +
      `L${S(6.4, 0)}Q${S(6.9, -12)} ${S(5.4, -26)}L${S(7, -40)}Q${S(6.6, -44.4)} ${S(2, -44.6)}Z` +
      f.circle(...L(0, -28 - 21.6 * ky), 4.4 * kx)
    );
  };
  const down = (side: number): Point[] => [L(side * 6.2, -41.6), L(side * 8.2, -33), L(side * 7.8, -25)];
  const arms: Point[][] =
    pose === 0
      ? [down(-1), down(1)]
      : pose === 1
        ? [down(-1), [L(6.2, -41.6), L(11.4, -40.4), L(17.4, -41.8)]]
        : [
            [L(-6.2, -41.6), L(-5.2, -34), L(-1.3, -36.4)],
            [L(6.2, -41.6), L(5.2, -34), L(1.3, -36.4)],
          ];
  const limbs = (width: number): string => arms.map((arm) => f.band(arm, width)).join("");
  const mote = (x: number, y: number, size: number): string => sparkle(f, ...L(x, y), size);
  return [
    shape(skewOval(f, L(0, -27), [15, 0], [-lean * 31, 31]), "sky-300", 0.1, MAIN),
    shape(silhouette(1.18, 1.18), "sky-300", 0.16, MAIN),
    shape(limbs(4.4), "sky-300", 0.14, MAIN),
    shape(silhouette(1, 1), "sky-300", 0.35, MAIN),
    shape(limbs(2.4), "sky-300", 0.35, MAIN),
    shape(silhouette(0.55, 0.92), "white", 0.28),
    shape(f.circle(...L(-0.6, -50), 0.5) + f.circle(...L(2, -50), 0.5), "white", 0.8),
    shape(
      mote(-10.5 + between(rng, -1, 1), -30, 1.4) + mote(10 + between(rng, -1, 1), -48, 1.1) + mote(-8.6, -54 + between(rng, -1, 1), 0.9),
      "sky-300",
      0.6,
      MAIN,
    ),
  ];
}

/** A crowd of three to eight head-and-shoulder silhouettes of varied heights standing on the anchor in two ranks, one or two arms raised; about 30 wide and 7 to 11 tall at scale 1. */
function crowd(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 3, 8);
  const step = 30 / n;
  const raised = [whole(rng, 0, n - 1), rng() < 0.5 ? whole(rng, 0, n - 1) : -1];
  let back = "";
  let front = "";
  let arms = "";
  for (let i = 0; i < n; i += 1) {
    const x = -15 + step * (i + 0.5) + between(rng, -0.09, 0.09) * step;
    const h = between(rng, 7, 10.5);
    const half = Math.min(between(rng, 2.2, 2.8), step * 0.85);
    const top = -h + 3.6;
    const person =
      `M${f.p(x - half, 0)}L${f.p(x - half, top + 1.4)}Q${f.p(x - half, top)} ${f.p(x - half + 1.4, top)}L${f.p(x - 0.6, top)}` +
      `L${f.p(x - 0.6, top - 0.35)}L${f.p(x + 0.6, top - 0.35)}L${f.p(x + 0.6, top)}L${f.p(x + half - 1.4, top)}` +
      `Q${f.p(x + half, top)} ${f.p(x + half, top + 1.4)}L${f.p(x + half, 0)}Z` +
      f.circle(x, top - 2, 1.6);
    if (i % 2 === 0) back += person;
    else front += person;
    if (raised.includes(i)) arms += f.band([[x + half - 0.7, top + 0.8], [x + half + 0.5, top - 2.2], [x + half + 0.2, top - 5]], 1);
  }
  return [shape(back, "slate-800", 0.85, MAIN_INKED), shape(arms, "slate-800", 1, MAIN_INKED), shape(front, "slate-800", 1, MAIN_INKED)];
}

/** An alien trooper standing on the anchor: a bulbous head with three eyes, tentacle arms, armour plates on its chest and shoulders, a belt and boots; about 10 wide and 22 tall at scale 1. */
function alien(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = (): number => between(rng, -0.6, 0.6);
  const [a, b, c, d] = [w(), w(), w(), w()];
  return [
    shape(
      f.band([[-1.6, -7], [-1.9, -3.6], [-1.8, -1.2]], 1.6) +
        f.band([[1.6, -7], [1.9, -3.6], [2, -1.2]], 1.6) +
        f.band([[-3.6, -12.4], [-5 + a, -9.6], [-4.2, -7.2 + b], [-5.4 + a, -5]], 1.3) +
        f.band([[3.6, -12.4], [5.2 + c, -10], [4.6, -7.6 + d], [5.8 + c, -6]], 1.3),
      "green-500",
      1,
      MAIN_INKED,
    ),
    shape(f.poly([[-3.4, -13.6], [3.4, -13.6], [2.8, -6.4], [-2.8, -6.4]]), "green-500", 1, MAIN_INKED),
    shape(
      cut(f, -2.4, -13.2, 2.4, -9.2, 0.8) +
        oval(f, -4.4, -13.2, 1.7, 1.1) +
        oval(f, 4.4, -13.2, 1.7, 1.1) +
        box(f, -2.9, -7.6, 2.9, -6.4) +
        rounded(f, -3, -1.8, -0.5, 0, 0.4) +
        rounded(f, 0.7, -1.8, 3.4, 0, 0.4),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.band([[-1.2, -12.4], [1.4, -12.4]], 0.5) + f.band([[4, -13.8], [5.2, -13.4]], 0.4), "slate-400", 0.6),
    shape(`M${f.p(-1.4, -13.4)}C${f.p(-5.8, -14)} ${f.p(-5.2, -21.8)} ${f.p(0, -21.8)}C${f.p(5.2, -21.8)} ${f.p(5.8, -14)} ${f.p(1.4, -13.4)}Z`, "green-500", 1, MAIN_INKED),
    shape(oval(f, -1.7, -17.4, 0.9, 1.2) + oval(f, 1.7, -17.4, 0.9, 1.2) + oval(f, 0, -19.8, 0.7, 0.9) + f.band([[-0.7, -14.9], [0.7, -14.9]], 0.35), "slate-900", 1),
    shape(f.circle(-1.4, -17.9, 0.3) + f.circle(2, -17.9, 0.3) + f.circle(0.25, -20.2, 0.25), "white", 0.85),
    shape(f.band([[1.2, -20.9], [2.6, -20.1], [3.4, -18.6]], 0.8), "lime-200", 0.35),
  ];
}

/** A dog sitting on the anchor, facing +x: a haunch and chest, front legs, a head with a floppy ear, a wagging tail, a red collar with a tag; about 10 wide and 8 tall at scale 1. */
function dog(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wag = between(rng, -1, 1);
  const flop = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-4, -1.4], [-5.4, -2.6 + wag * 0.4], [-5.6 + wag, -4.4]], 0.9), "wood-600", 1, MAIN_INKED),
    shape(f.band([[2.9, -3], [3, -0.4]], 1.1), "wood-700", 1, INKED),
    shape(`M${f.p(-4.4, 0)}Q${f.p(-5, -3.4)} ${f.p(-2.6, -4.2)}Q${f.p(0, -4.8)} ${f.p(1.6, -6.2)}L${f.p(3.8, -5.4)}Q${f.p(3.8, -2.8)} ${f.p(2.8, -1.6)}L${f.p(2.2, 0)}Z`, "wood-600", 1, MAIN_INKED),
    shape(f.band([[-3.6, -0.4], [-3.4, -2.6], [-1.6, -3.2], [0, -1.6], [0.4, 0]], 0.4), "wood-700", 0.5),
    shape(f.band([[2, -2.6], [1.9, -0.3]], 1.2), "wood-600", 1, MAIN_INKED),
    shape(
      `M${f.p(1.8, -6)}Q${f.p(1.6, -8.6)} ${f.p(3.6, -8.6)}Q${f.p(4.8, -8.6)} ${f.p(5.1, -7.6)}L${f.p(6.2, -7.1)}Q${f.p(6.8, -6.3)} ${f.p(6.1, -5.8)}L${f.p(4.6, -5.4)}Q${f.p(2.6, -4.8)} ${f.p(1.8, -6)}Z`,
      "wood-600",
      1,
      MAIN_INKED,
    ),
    shape(`M${f.p(2.6, -8.2)}Q${f.p(1.2, -7.9)} ${f.p(1.4 + flop * 0.5, -5.8 + flop)}Q${f.p(2.2, -5.4 + flop)} ${f.p(2.9, -6.8)}Z`, "wood-700", 1, INKED),
    shape(f.band([[1.7, -5.3], [3.8, -4.9]], 0.6), "red-500", 1, INKED),
    shape(f.circle(2.9, -4.4, 0.4), "brass-500", 1),
    shape(f.circle(4.2, -7.4, 0.35) + f.circle(6.3, -6.5, 0.45), "slate-900", 1),
  ];
}

/** A wobbling jelly-blob AI standing on the anchor: a translucent scalloped dome in a glow, a denser inner dome, a glowing core, two eyes and a highlight, a puddle at its base; about 16 wide and 12 tall at scale 1. */
function jellyBlob(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -1.4, 1.4);
  const squash = between(rng, 0.9, 1.1);
  const J = (x: number, y: number): Point => [x / squash + lean * (-y / 12), y * squash];
  const dome = (k: number): string => {
    const P = (x: number, y: number): string => f.p(...J(x * k, y * k));
    return (
      `M${P(-8, 0)}C${P(-8.4, -6)} ${P(-4, -12)} ${P(0, -12)}C${P(4, -12)} ${P(8.4, -6)} ${P(8, 0)}` +
      `Q${P(6, 1)} ${P(4, 0)}Q${P(2, 1)} ${P(0, 0)}Q${P(-2, 1)} ${P(-4, 0)}Q${P(-6, 1)} ${P(-8, 0)}Z`
    );
  };
  const core = J(0, -4.6);
  return [
    shape(oval(f, 0, 0.2, 9, 1), "cyan-400", 0.22, MAIN),
    shape(dome(1.18), "cyan-400", 0.14, MAIN),
    shape(dome(1), "cyan-400", 0.5, MAIN_INKED),
    shape(dome(0.66), "cyan-400", 0.3, MAIN),
    shape(f.circle(...core, 3.1), "white", 0.2),
    shape(f.circle(...core, 1.5), "white", 0.8),
    shape(oval(f, ...J(-1.9, -7.8), 0.8, 1.2) + oval(f, ...J(2.7, -7.8), 0.8, 1.2), "slate-900", 1),
    shape(f.circle(...J(-1.6, -8.3), 0.3) + f.circle(...J(3, -8.3), 0.3), "white", 0.9),
    shape(f.band([J(2.2, -10.6), J(4, -9.4), J(5.2, -7.2)], 0.9), "white", 0.5),
  ];
}

/** A hulking golem of scrap stone and hull plates, centred on the anchor: a blocky stone body with cracked seams, bolted plates, a cockpit-visor head, a reactor at its chest, wing fins behind its shoulders, thruster boots burning below; about 30 wide and 40 tall at scale 1. */
function scrapGolem(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const burn = between(rng, 5, 7.5);
  const crack = between(rng, -0.8, 0.8);
  const boots: Point[] = [
    [-4, 14],
    [4, 14],
  ];
  const plate = (x0: number, y0: number, x1: number, y1: number): string =>
    cut(f, x0, y0, x1, y1, 0.6) + [x0 + 0.7, x1 - 0.7].flatMap((x) => [y0 + 0.7, y1 - 0.7].map((y) => f.circle(x, y, 0.3))).join("");
  return [
    shape(f.poly([[-7, -12], [-12, -20], [-15.5, -20], [-12, -11]]) + f.poly([[7, -12], [12, -20], [15.5, -20], [12, -11]]), "slate-700", 1, INKED),
    shape(boots.map((b) => tongue(f, b, 90, burn, 2.3)).join(""), "cyan-400", 0.28),
    shape(
      f.poly([[-14, -13.2], [-8, -13.6], [-7.6, -7], [-9.6, -6.8], [-9.8, 0], [-14.2, 0], [-14.4, -7.4]]) +
        f.poly([[14, -13.2], [8, -13.6], [7.6, -7], [9.6, -6.8], [9.8, 0], [14.2, 0], [14.4, -7.4]]) +
        box(f, -6.2, 3.4, -1.6, 9.4) +
        box(f, 1.6, 3.4, 6.2, 9.4),
      "stone-500",
      1,
      MAIN_INKED,
    ),
    shape(f.poly([[-7, -12.6], [7, -12.6], [9, -8], [8, 2], [5, 4], [-5, 4], [-8, 2], [-9, -8]]), "stone-500", 1, MAIN_INKED),
    shape(
      f.band([[-3, -12.2], [-2.2 + crack, -6], [-3.4, 3.6]], 0.45) +
        f.band([[3.6, -12.2], [4.2, -3], [3, 3.6]], 0.45) +
        f.band([[-8.4, -4.2], [-3.4, -4.3]], 0.45) +
        f.band([[-1.4, -4.3], [3, -4.4]], 0.45) +
        f.band([[5, -4.4], [8.4, -4.5]], 0.45),
      "slate-800",
      0.6,
    ),
    shape(plate(-7.4, -11, -4, -6.2) + plate(4.8, -2.8, 7.6, 1.6), "slate-400", 1, INKED),
    shape(
      rounded(f, -15, -0.4, -9, 4, 1) +
        rounded(f, 9, -0.4, 15, 4, 1) +
        f.poly([[-7, 9.2], [-1, 9.2], [-1.4, 12.6], [-2.2, 12.6], [-1.6, 14], [-6.4, 14], [-5.8, 12.6], [-6.6, 12.6]]) +
        f.poly([[7, 9.2], [1, 9.2], [1.4, 12.6], [2.2, 12.6], [1.6, 14], [6.4, 14], [5.8, 12.6], [6.6, 12.6]]),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.poly([[-4.2, -12], [-4.6, -17], [-2.4, -19.6], [2.4, -19.6], [4.6, -17], [4.2, -12]]), "stone-500", 1, MAIN_INKED),
    shape(f.circle(0, -7, 3.8), "cyan-400", 0.25),
    shape(f.poly([[-3, -17], [3, -17], [3.4, -14.6], [2.4, -13.6], [-2.4, -13.6], [-3.4, -14.6]]) + f.circle(0, -7, 2.2), "cyan-400", 1, INKED),
    shape(boots.map((b) => tongue(f, b, 90, burn * 0.5, 1)).join("") + f.band([[-1.8, -16.2], [1, -16.2]], 0.4) + f.circle(-0.4, -7.4, 0.6), "white", 0.8),
  ];
}

/** A big gloved hand reaching in from the foreground, its wrist at the anchor: an armoured cuff running back with a light, the back of the hand, knuckle plates and fingers spread and reaching about 16 toward its facing; about 25 long and 12 wide at scale 1. */
function glovedHand(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -1, 1.2);
  const spread = between(rng, 1, 1.35);
  const knuckles = [-2.7, -0.9, 0.9, 2.7];
  const lengths = [6.2, 7.6, 7.4, 6.2];
  const fingers = knuckles
    .map((y, k) => {
      const length = lengths[k] ?? 7;
      return f.band([[7.4, y], [7.4 + length * 0.55, y * (1 + (spread - 1) * 0.6) + curl * 0.3], [7.4 + length, y * spread + curl]], 1.6);
    })
    .join("");
  return [
    shape(fingers + f.band([[3, -3.2], [5.6, -5.6], [8.6, -6.4 + curl * 0.3]], 1.9), "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(0, -3.4)}L${f.p(6.4, -3.8)}Q${f.p(8.4, -3.7)} ${f.p(8.6, -1)}L${f.p(8.6, 2.4)}Q${f.p(8.2, 4.1)} ${f.p(6, 4)}L${f.p(0, 3.4)}Z`, "slate-700", 1, MAIN_INKED),
    shape(knuckles.map((y) => rounded(f, 7.6, y - 0.7, 9.4, y + 0.7, 0.3)).join(""), "slate-400", 1, INKED),
    shape(f.poly([[-9, -4.8], [0.6, -3.8], [0.6, 3.8], [-9, 4.8]]), "slate-800", 1, INKED),
    shape(f.band([[-2, -4], [-2, 4]], 1.4) + f.band([[-6, -4.5], [-6, 4.5]], 1.2), "slate-400", 1, INKED),
    shape(f.circle(-4, 0, 1.6), "cyan-400", 0.25),
    shape(f.circle(-4, 0, 0.75), "cyan-400", 1),
    shape(f.band([[1.6, -2.5], [6.2, -2.8]], 0.8), "white", 0.2),
  ];
}

/** An autonomous mining arm standing on the anchor: a base and turntable, two jointed segments with hazard stripes and a piston reaching about 30 toward its facing and up, a chisel head, lit joints; about 30 wide and 22 tall at scale 1. */
function robotArm(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j0: Point = [0, -4.6];
  const d1 = heading(between(rng, -72, -50));
  const n1: Point = [-d1[1], d1[0]];
  const j1 = plus(j0, d1, 16);
  const a2 = between(rng, -14, 14);
  const j2 = plus(j1, heading(a2), 15);
  const u = heading(a2 + between(rng, 25, 55));
  const n: Point = [-u[1], u[0]];
  const blade = plus(j2, u, 4.4);
  const stripes = [0.4, 0.55, 0.7]
    .map((t) => {
      const c = plus(j0, d1, 16 * t);
      return f.band([plus(c, n1, -1.3), plus(plus(c, n1, 1.3), d1, 0.9)], 0.55);
    })
    .join("");
  const joints: (readonly [Point, number])[] = [
    [j0, 2.3],
    [j1, 2],
    [j2, 1.6],
  ];
  return [
    shape(f.poly([[-5, 0], [-4, -2.6], [4, -2.6], [5, 0]]) + box(f, -2.6, -4.4, 2.6, -2.6), "slate-700", 1, INKED),
    shape(f.band([[2.8, -3.2], plus(plus(j0, d1, 9), n1, 2)], 1), "slate-400", 1, INKED),
    shape(f.band([j0, j1], 3.2), "orange-400", 1, MAIN_INKED),
    shape(stripes, "slate-900", 0.7),
    shape(f.band([j1, j2], 2.6), "orange-400", 1, MAIN_INKED),
    shape(f.poly([plus(j2, n, 1.7), plus(blade, n, 0.6), plus(j2, u, 5.8), plus(blade, n, -0.6), plus(j2, n, -1.7)]), "slate-400", 1, INKED),
    shape(f.poly([plus(blade, n, 0.6), plus(j2, u, 5.8), plus(blade, n, -0.6)]), "slate-100", 1),
    shape(joints.map(([j, r]) => f.circle(...j, r)).join(""), "slate-700", 1, INKED),
    shape(joints.map(([j]) => f.circle(...j, 1.3)).join(""), "cyan-400", 0.22),
    shape(joints.map(([j]) => f.circle(...j, 0.6)).join(""), "cyan-400", 1),
  ];
}

/** A floating AI eye, centred on the anchor: a dark lens ring, a glowing iris and pupil, a ring of light segments round it (most of them lit), all in a glow; about 10 across (its glow 13) at scale 1. */
function aiEye(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = whole(rng, 0, 11);
  const lit = whole(rng, 7, 11);
  const look: Point = [between(rng, -0.6, 0.8), between(rng, -0.6, 0.6)];
  let on = "";
  let off = "";
  for (let i = 0; i < 12; i += 1) {
    const a = i * 30;
    const piece = f.poly([polar(4.5, a - 10), polar(5.3, a - 10), polar(5.3, a), polar(5.3, a + 10), polar(4.5, a + 10), polar(4.5, a)]);
    if ((i - start + 12) % 12 < lit) on += piece;
    else off += piece;
  }
  return [
    shape(f.circle(0, 0, 6.4), "red-500", 0.12, MAIN),
    shape(f.circle(0, 0, 5), "red-500", 0.2, MAIN),
    shape(off, "slate-700", 0.8),
    shape(on, "red-500", 0.9, MAIN),
    shape(ring(f, 0, 0, 4.1, 3.1), "slate-800", 1, INKED),
    shape(f.circle(0, 0, 3.1), "red-500", 1, MAIN_INKED),
    shape(ring(f, ...look, 2.3, 1.8), "red-300", 0.55),
    shape(f.circle(...look, 1.1), "slate-900", 1),
    shape(f.circle(look[0] + 0.9, look[1] - 0.9, 0.45) + f.circle(look[0] - 0.4, look[1] + 0.4, 0.2), "white", 0.85),
  ];
}

// Ships and machines

/** A big starship, centred on the anchor and facing +x: a long hull with a bow stripe, a bridge tower, a dorsal and a ventral fin, an engine block with two nozzles burning behind, a turret, gun ports and rows of lit windows; about 50 long and 18 tall at scale 1. */
function starship(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -4, 4));
  const burn = between(rng, 8, 12);
  const nozzles: Point[] = [
    [-27.4, -3.3],
    [-27.4, 2.6],
  ];
  const flames = (length: number, half: number): string => nozzles.map((n) => tongue(f, n, 180, length, half)).join("");
  let windows = "";
  for (let i = 0; i < 15; i += 1) {
    const x = -18 + i * 2.3;
    if (rng() < 0.7 || i === 2) windows += box(f, x - 0.45, -1.9, x + 0.45, -1.1);
  }
  for (const x of [-6, -4, -2, 0]) windows += box(f, x - 0.6, -7.6, x + 0.6, -6.8);
  return [
    shape(flames(burn * 1.3, 2.8), "cyan-400", 0.15),
    shape(flames(burn * 0.8, 1.7), "cyan-400", 0.3),
    shape(flames(burn * 0.45, 0.9), "white", 0.8),
    shape(f.poly([[-21.5, -4.2], [-23, -9], [-19.6, -9], [-14.6, -4.2]]) + f.poly([[-21, 4.6], [-22.6, 9.2], [-19.2, 9.2], [-14, 4.6]]), "slate-700", 1, INKED),
    shape(
      f.poly([[-21.5, -5.6], [-25, -5.6], [-25, -4.6], [-27.4, -5.4], [-27.4, -1.2], [-25, -1.9], [-25, 1], [-27.4, 0.4], [-27.4, 4.8], [-25, 4.1], [-25, 5.8], [-21.5, 5.8]]),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.poly([[-22, -4.2], [8, -4.2], [18, -2.2], [26.5, 0.6], [18, 3], [6, 4.6], [-22, 4.6]]), "slate-400", 1, MAIN_INKED),
    shape(
      f.poly([[-22, 1.4], [23.6, 1.4], [18, 3], [6, 4.6], [-22, 4.6]]) +
        f.band([[-9.95, -3.9], [-9.95, 1.1]], 0.35) +
        f.band([[1.55, -3.9], [1.55, 1.1]], 0.35) +
        f.band([[13.05, -3], [13.05, 1.1]], 0.35),
      "slate-700",
      0.45,
    ),
    shape(f.band([[-21, -3.6], [7.8, -3.6], [17.4, -1.7]], 0.5), "white", 0.25),
    shape(f.poly([[19.4, -1.6], [20.8, -1.2], [20.8, 2.2], [19.4, 2.5]]), "red-500", 0.9),
    shape(f.poly([[-10.4, -4], [-9.2, -7.2], [-6.6, -8.8], [1.4, -8.8], [3.4, -6.4], [4.6, -4]]), "slate-400", 1, MAIN_INKED),
    shape(
      `M${f.p(10, -3.6)}Q${f.p(10, -6)} ${f.p(12.4, -6)}Q${f.p(14.6, -6)} ${f.p(14.6, -3.6)}Z` +
        box(f, 14.4, -5.4, 20, -4.6) +
        [-17, -12, -7, -2, 3].map((x) => box(f, x - 0.8, 2.6, x + 0.8, 3.4)).join(""),
      "slate-800",
      1,
      INKED,
    ),
    shape(windows, "cyan-400", 1),
  ];
}

/** A small fighter, centred on the anchor and facing +x: a dart fuselage with a red nose band, swept wings near and far, a tail fin, a canopy, an engine burning behind; about 14 long at scale 1. */
function fighter(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -9, 9));
  const burn = between(rng, 4, 6);
  return [
    shape(tongue(f, [-7.8, 0], 180, burn, 1.3), "cyan-400", 0.28),
    shape(tongue(f, [-7.8, 0], 180, burn * 0.5, 0.6), "white", 0.8),
    shape(f.poly([[1, -0.8], [-1.8, -3.6], [-3.4, -3.6], [-3, -0.8]]), "slate-700", 1, INKED),
    shape(f.poly([[-4, -1], [-6, -4.4], [-7.2, -4.4], [-6.8, -1]]), "slate-400", 1, MAIN_INKED),
    shape(box(f, -7.8, -0.9, -6.6, 0.9), "slate-800", 1, INKED),
    shape(f.poly([[7.4, 0.2], [4, -1.2], [-1, -1.4], [-6.8, -1.2], [-6.8, 1.2], [-1, 1.5], [4, 1.2]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[7.4, 0.2], [-6.8, 0.2], [-6.8, 1.2], [-1, 1.5], [4, 1.2]]), "slate-700", 0.4),
    shape(f.poly([[4.6, -0.9], [5.6, -0.5], [5.6, 0.7], [4.6, 1]]), "red-500", 0.9),
    shape(f.poly([[2, 0.6], [-2.6, 4.6], [-4.6, 4.6], [-3.8, 0.8]]), "slate-400", 1, MAIN_INKED),
    shape(`M${f.p(-0.8, -1.3)}Q${f.p(0.8, -3.2)} ${f.p(3.6, -1.15)}Z`, "cyan-400", 1, INKED),
    shape(f.band([[0, -1.7], [1.4, -1.95]], 0.3), "white", 0.8),
  ];
}

/** A cargo freighter, centred on the anchor and facing +x: a boxy hull with cargo pods on top and below, an open hold in its side (a dark opening, lit within, its door swung down), a cockpit, two engines burning behind; about 40 long at scale 1. */
function freighter(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -4, 4));
  const burn = between(rng, 6, 9);
  const swing = between(rng, 0, 1.4);
  const nozzles: Point[] = [
    [-20, -2.6],
    [-20, 2.6],
  ];
  return [
    shape(nozzles.map((n) => tongue(f, n, 180, burn, 1.9)).join(""), "cyan-400", 0.26),
    shape(nozzles.map((n) => tongue(f, n, 180, burn * 0.5, 0.8)).join(""), "white", 0.75),
    shape(box(f, -14, -8.6, -6.6, -5) + box(f, 2, -8, 8.4, -5), "brass-600", 1, INKED),
    shape(box(f, -6.2, -8.6, 1.6, -5) + box(f, -16, 5, -8.4, 7.4), "blue-800", 1, INKED),
    shape(f.poly([[-17, -4], [-20, -4.6], [-20, -0.6], [-17, -1]]) + f.poly([[-17, 1], [-20, 0.6], [-20, 4.6], [-17, 4]]), "slate-800", 1, INKED),
    shape(cut(f, -17.4, -5.2, 14.4, 5.2, 1.4), "slate-700", 1, MAIN_INKED),
    shape(
      f.band([[-10, -4.8], [-10, 4.8]], 0.4) +
        f.band([[8, -4.8], [8, 4.8]], 0.4) +
        f.band([[-10.3, -8.4], [-10.3, -5.2]], 0.35) +
        f.band([[-2.3, -8.4], [-2.3, -5.2]], 0.35) +
        f.band([[5.2, -7.8], [5.2, -5.2]], 0.35) +
        f.band([[-12.2, 5.2], [-12.2, 7.2]], 0.35),
      "slate-800",
      0.6,
    ),
    shape(f.poly([[13.8, -4.4], [17.6, -3.8], [20.8, -1.2], [20.8, 1.6], [17.6, 3.8], [13.8, 4.4]]), "slate-400", 1, INKED),
    shape(
      f.poly([[17, -2.8], [19.6, -1.3], [19.6, 0], [17, 0]]) + [-14.6, -12.4, 6.2, 10.2, 12].map((x) => f.circle(x, -2.2, 0.5)).join(""),
      "cyan-400",
      1,
    ),
    shape(box(f, -6, -3, 4, 3.2), "slate-900", 1),
    shape(oval(f, -1, 1.6, 4, 1.3), "orange-400", 0.25),
    shape(f.poly([[-6, 3.2], [4, 3.2], [4.4 + swing * 0.4, 6.6 + swing], [-5.6 + swing * 0.4, 6.6 + swing]]), "slate-400", 1, INKED),
  ];
}

/** A landed dropship standing on the anchor, facing +x: a hull on two landing struts, an engine pod idling on its back, a cockpit window, an open hold with its ramp lowered to the ground ahead and light spilling out, landing lights; about 30 wide and 16 tall at scale 1. */
function dropship(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const idle = between(rng, 2, 3.6);
  const spill = between(rng, -0.6, 0.6);
  return [
    shape(oval(f, 0.6, 0, 15.4, 1), "slate-900", 0.35),
    shape(f.band([[-10, -5.6], [-12, -1.2]], 1.2) + f.band([[3, -5.6], [4.6, -1.2]], 1.2) + box(f, -13.8, -0.9, -10.4, 0) + box(f, 3, -0.9, 6.4, 0), "slate-800", 1, INKED),
    shape(tongue(f, [-15, -14.6], 180, idle, 1.2), "cyan-400", 0.22),
    shape(f.poly([[-15, -9], [-13, -13.6], [-4, -15.4], [5, -15.4], [10.6, -12.4], [14, -8.6], [12.6, -6.2], [8, -5.4], [-14, -5.4]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-13, -13.6], [-4, -15.4], [5, -15.4], [8.6, -13.4], [-13.2, -12.8]]), "slate-400", 0.5),
    shape(rounded(f, -15, -16.4, -6, -12.8, 1.2), "slate-400", 1, INKED),
    shape(f.poly([[6.4, -13.8], [10, -11.8], [12.6, -8.8], [8.4, -9.6]]), "cyan-400", 1, INKED),
    shape(f.poly([[3, -8.4], [12.2, -8.4], [12.6, -6.6], [9.6, -5.7], [3, -5.7]]), "slate-900", 1),
    shape(oval(f, 7.6, -6.8, 3.4, 1.3) + oval(f, 16.6 + spill, -0.2, 3.2, 0.6), "orange-400", 0.22),
    shape(f.poly([[8.4, -5.7], [10.2, -5.9], [17, -0.7], [17, 0], [15.4, 0]]), "slate-400", 1, INKED),
    shape(f.circle(-11, -3.2, 0.45) + f.circle(3.8, -3.2, 0.45) + f.circle(0, -15.8, 0.5), "red-400", 1),
  ];
}

/** A small shuttle, centred on the anchor and facing +x: a rounded white hull with a blue stripe, a row of windows, a windshield, a stubby wing and a tail fin, an engine burning behind; about 16 long at scale 1. */
function shuttle(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -7, 7));
  const burn = between(rng, 3.4, 5);
  return [
    shape(tongue(f, [-8.6, 0.4], 180, burn, 1.3), "cyan-400", 0.28),
    shape(tongue(f, [-8.6, 0.4], 180, burn * 0.5, 0.6), "white", 0.8),
    shape(f.poly([[-4.4, -2.4], [-6.8, -5.6], [-8.2, -5.6], [-7.8, -2.2]]), "slate-400", 1, INKED),
    shape(box(f, -8.6, -0.9, -7.2, 1.7), "slate-700", 1, INKED),
    shape(`M${f.p(-7.6, -2.2)}Q${f.p(-2, -3.6)} ${f.p(4, -3)}Q${f.p(8.4, -2.2)} ${f.p(8.4, 0.6)}Q${f.p(8, 3)} ${f.p(3, 3)}L${f.p(-7.6, 3)}Z`, "white", 1, MAIN_INKED),
    shape(`M${f.p(-7.6, 1.6)}L${f.p(7.9, 1.6)}Q${f.p(7.6, 3)} ${f.p(3, 3)}L${f.p(-7.6, 3)}Z`, "slate-400", 0.35),
    shape(f.band([[-7.4, 0.7], [7.9, 0.7]], 0.6), "blue-600", 1),
    shape(f.circle(-4, -0.9, 0.65) + f.circle(-1.8, -1, 0.65) + f.circle(0.4, -1.1, 0.65), "cyan-400", 1),
    shape(`M${f.p(4, -2.6)}Q${f.p(7, -2.3)} ${f.p(7.9, -0.6)}L${f.p(4.6, -0.6)}Z`, "slate-800", 1, INKED),
    shape(f.band([[4.8, -2.1], [6.2, -1.8]], 0.3), "white", 0.7),
    shape(f.poly([[1, 2.6], [-2.4, 5.2], [-4.8, 5.2], [-4.2, 2.6]]), "slate-400", 1, INKED),
  ];
}

/** An upright rocket standing on the anchor on its fins: a red nose cone, a white body banded twice with a round window, three red fins, an engine bell; about 10 wide and 36 tall at scale 1. */
function rocket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, -0.4, 0.6);
  const port = between(rng, -1.2, 1.2);
  return [
    shape(f.poly([[-3.2, -10], [-5.2 - flare, -3.4], [-5.2 - flare, 0], [-3.2, -3]]) + f.poly([[3.2, -10], [5.2 + flare, -3.4], [5.2 + flare, 0], [3.2, -3]]), "red-500", 1, INKED),
    shape(f.poly([[-1.8, -6.2], [1.8, -6.2], [2.6, -1.4], [-2.6, -1.4]]), "slate-700", 1, INKED),
    shape(box(f, -3.3, -27, 3.3, -5.8), "slate-100", 1, MAIN_INKED),
    shape(box(f, -3.3, -27, -1.6, -5.8), "slate-400", 0.45),
    shape(box(f, -3.3, -11.6, 3.3, -10.4) + box(f, -3.3, -25, 3.3, -24), "slate-400", 1, INKED),
    shape(`M${f.p(-3.3, -27)}Q${f.p(-3.2, -32.4)} ${f.p(0, -36)}Q${f.p(3.2, -32.4)} ${f.p(3.3, -27)}Z`, "red-500", 1, INKED),
    shape(f.poly([[-0.6, -10.2], [0.6, -10.2], [0.9, -1], [-0.9, -1]]), "red-600", 1, INKED),
    shape(ring(f, 0, -18.6 + port, 2, 1.35), "slate-700", 1, INKED),
    shape(f.circle(0, -18.6 + port, 1.35), "cyan-400", 1),
    shape(f.circle(0.55, -19.2 + port, 0.4), "white", 0.85),
  ];
}

/** A treaded tank standing on the anchor, facing +x: treads over five wheels, a sloped hull, a domed turret with a cupola and an antenna, a long barrel raised a little toward its facing; about 24 wide and 12 tall at scale 1. */
function tank(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const raise = between(rng, -8, 2);
  const pivot: Point = [4, -9.6];
  const B = (x: number, y: number): Point => about(pivot, [x, y], raise);
  return [
    shape(rounded(f, -11.6, -4.6, 11.2, 0, 2.2), "slate-800", 1, INKED),
    shape([-8.6, -4.3, 0, 4.3, 8.6].map((x) => ring(f, x, -2.3, 1.45, 0.5)).join(""), "slate-400", 1),
    shape(f.poly([B(4, -10.2), B(12.4, -10.2), B(12.4, -10.6), B(14, -10.6), B(14, -8.6), B(12.4, -8.6), B(12.4, -9), B(4, -9)]), "slate-700", 1, INKED),
    shape(f.poly([[-10.8, -4.4], [-10.2, -7.6], [6, -7.6], [11.4, -4.8], [11.4, -4.4]]), "blue-800", 1, MAIN_INKED),
    shape(f.band([[-10.2, -5.6], [9.6, -5.6]], 0.5), "blue-900", 0.6),
    shape(`M${f.p(-5.6, -7.4)}L${f.p(-5, -10.4)}Q${f.p(-4.4, -11.6)} ${f.p(-2, -11.6)}L${f.p(1.6, -11.6)}Q${f.p(3.8, -11.4)} ${f.p(4.6, -9.4)}L${f.p(5, -7.4)}Z`, "blue-800", 1, MAIN_INKED),
    shape(rounded(f, -3.4, -12.6, -0.6, -11.2, 0.5) + f.band([[-4.4, -10.8], [-5.8, -15.6]], 0.4), "slate-700", 1, INKED),
    shape(f.band([[-9.6, -7.2], [5.6, -7.2]], 0.5) + f.band([[-0.2, -11.1], [2, -11]], 0.45), "white", 0.22),
  ];
}

/** A small spindly probe, centred on the anchor and tumbling a little: a hexagonal body half wrapped in foil, two solar panels on a strut, one long antenna up with a red tip light; about 12 across and 10 tall at scale 1. */
function probe(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -16, 16));
  return [
    shape(f.band([[-3, 0], [3, 0]], 0.5), "slate-700", 1),
    shape(box(f, -6, -1.8, -2.8, 1.8) + box(f, 2.8, -1.8, 6, 1.8), "blue-800", 1, INKED),
    shape([-4.95, -3.85, 3.85, 4.95].map((x) => f.band([[x, -1.6], [x, 1.6]], 0.22)).join(""), "blue-600", 0.8),
    shape(f.band([[0, -1.8], [0.3, -7.4]], 0.4), "slate-400", 1),
    shape(f.poly([0, 60, 120, 180, 240, 300].map((a) => polar(2.2, a))), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-1.9, 0.3], [1.9, 0.3], [1, 1.75], [-1, 1.75]]), "brass-600", 0.9),
    shape(f.circle(0.9, -0.8, 0.45), "cyan-400", 1),
    shape(f.circle(0.33, -7.7, 1.2), "red-400", 0.25),
    shape(f.circle(0.33, -7.7, 0.5), "red-400", 1),
  ];
}

/** A relay satellite, centred on the anchor and turned a little: a foil-banded body, two solar wings, a dish on a boom facing +x sending arcs of signal, an antenna up with a red light; about 20 across and 18 tall at scale 1. */
function satellite(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -12, 12));
  const pulse = between(rng, 0.85, 1.15);
  const arc = (r: number): string => f.band([-30, -15, 0, 15, 30].map((a) => polar(r, a * pulse, 3.4, 6.6)), 0.4);
  return [
    shape(f.band([[-4.8, 0], [-2.8, 0]], 0.6) + f.band([[2.8, 0], [4.8, 0]], 0.6) + f.band([[1.6, 3], [3, 4.6]], 0.6), "slate-700", 1),
    shape(box(f, -10.4, -2.2, -4.4, 2.2) + box(f, 4.4, -2.2, 10.4, 2.2), "blue-800", 1, INKED),
    shape([-8.9, -7.4, -5.9, 5.9, 7.4, 8.9].map((x) => f.band([[x, -2], [x, 2]], 0.22)).join(""), "blue-600", 0.8),
    shape(cut(f, -3, -3.4, 3, 3.4, 0.8), "slate-400", 1, MAIN_INKED),
    shape(box(f, -3, 0.8, 3, 2.4), "brass-600", 0.85),
    shape(`M${f.p(3.6, 3.4)}Q${f.p(0.8, 6.6)} ${f.p(3.6, 9.8)}Q${f.p(2.6, 6.6)} ${f.p(3.6, 3.4)}Z`, "slate-100", 1, INKED),
    shape(f.band([[3.2, 6.6], [6.2, 6.6]], 0.4) + f.poly([[6.2, 6], [7.2, 5.8], [7.2, 7.4], [6.2, 7.2]]) + f.band([[0, -3.4], [0, -7]], 0.4), "slate-400", 1),
    shape(f.circle(1.4, -1.6, 0.5), "cyan-400", 1),
    shape(f.circle(0, -7.3, 0.55), "red-400", 1),
    shape(arc(5.6) + arc(7.8), "cyan-400", 0.3),
  ];
}

/** A giant ground uplink dish standing on the anchor: a base, a braced support tower, a dish bowl tilted up toward its facing, a feed horn on three struts at its focus with a red warning light; about 26 wide and 30 tall at scale 1. */
function satelliteDish(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const d = heading(between(rng, -58, -36));
  const n: Point = [-d[1], d[0]];
  const c = plus([0, -15], d, 3);
  const R = 15;
  const a = 4.4;
  const u: Point = [n[0] * R, n[1] * R];
  const v: Point = [d[0] * a, d[1] * a];
  const k = 0.5523;
  const q = (s: number, t: number): Point => [c[0] + u[0] * s + v[0] * t, c[1] + u[1] * s + v[1] * t];
  const Q = (s: number, t: number): string => f.p(...q(s, t));
  const focus = plus(c, d, 9.5);
  const strut = (s: number, t: number): string => {
    const start = q(s, t);
    return f.band([start, plus(start, [focus[0] - start[0], focus[1] - start[1]], 0.9)], 0.5);
  };
  const H = (along: number, across: number): Point => plus(plus(focus, d, along), n, across);
  return [
    shape(f.poly([[-9, 0], [-8, -3.2], [8, -3.2], [9, 0]]), "slate-700", 1, INKED),
    shape(f.band([[-5.6, -3], [-1.2, -14]], 1.3) + f.band([[5.6, -3], [1.2, -14]], 1.3), "slate-700", 1, INKED),
    shape(f.band([[-4.6, -5.2], [3.7, -7.6]], 0.6) + f.band([[3.4, -8.6], [-2.9, -10.8]], 0.6) + f.circle(0, -15, 1.8), "slate-800", 1, INKED),
    shape(`M${Q(1, 0)}Q${Q(0, -14 / a)} ${Q(-1, 0)}C${Q(-1, -k)} ${Q(-k, -1)} ${Q(0, -1)}C${Q(k, -1)} ${Q(1, -k)} ${Q(1, 0)}Z`, "slate-400", 1, INKED),
    shape(skewOval(f, c, u, v), "slate-100", 1, MAIN_INKED),
    shape(skewOval(f, q(0, -0.15), [u[0] * 0.78, u[1] * 0.78], [v[0] * 0.62, v[1] * 0.62]), "slate-400", 0.35),
    shape(strut(0.92, 0) + strut(-0.92, 0) + strut(0, 0.95), "slate-700", 0.95),
    shape(f.poly([H(-1.2, 0.9), H(1, 0.9), H(1.6, 0.5), H(1.6, -0.5), H(1, -0.9), H(-1.2, -0.9)]), "slate-700", 1, INKED),
    shape(f.circle(...H(2.2, 0), 1.4), "red-400", 0.25),
    shape(f.circle(...H(2.2, 0), 0.6), "red-400", 1),
  ];
}

/** A prospector rover standing on the anchor, facing +x: six wheels (three near, three far) on a rocker suspension, an orange body and cab with a window, a solar rack, a scoop arm ahead, an antenna, a headlight and its beam; about 18 wide and 10 tall at scale 1. */
function rover(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -18, 2);
  const pivot: Point = [9, -4];
  const S = (x: number, y: number): Point => about(pivot, [x, y], lift);
  const xs = [-5.2, -0.6, 4];
  return [
    shape(xs.map((x) => f.circle(x + 0.8, -2.1, 1.6)).join(""), "slate-900", 1, INKED),
    shape(f.poly([[9.4, -4.9], [15.4, -6.6], [15.4, -2.2], [9.4, -4.3]]), "white", 0.12),
    shape(f.band([[-6.6, -6.6], [-7.4, -10.4]], 0.35), "slate-700", 1),
    shape(f.poly([[-8, -2.6], [-8, -5.8], [4.4, -5.8], [4.4, -7.8], [7.8, -7.8], [9.4, -5], [9.4, -2.6]]), "orange-400", 1, MAIN_INKED),
    shape(box(f, -7.4, -6.8, 3.4, -5.8), "blue-800", 1, INKED),
    shape(f.poly([[5.2, -7.2], [7.4, -7.2], [8.6, -5.4], [5.2, -5.4]]), "cyan-400", 1, INKED),
    shape(f.band([[-5.2, -1.7], [-2.9, -3.2], [-0.6, -1.7], [1.7, -3.2], [4, -1.7]], 0.6), "slate-700", 1),
    shape(f.poly([S(8.8, -4.4), S(11.8, -2.8), S(13.8, -2.4), S(13.4, -0.2), S(11.6, -0.2), S(11.2, -1.9), S(8.8, -3.6)]), "slate-700", 1, INKED),
    shape(xs.map((x) => f.circle(x, -1.7, 1.7)).join(""), "slate-800", 1, INKED),
    shape(xs.map((x) => f.circle(x, -1.7, 0.6)).join(""), "slate-400", 1),
    shape(f.circle(-7.45, -10.6, 0.45), "red-400", 1),
    shape(f.circle(9.1, -4.6, 0.45), "white", 0.9),
  ];
}

/** A warp-disruptor buoy, centred on the anchor and tilted a little: a spindle body with a collar and a lit slit, emitter fins, light at both tips, a pulsing translucent ring of light round its middle; about 10 wide (its ring up to 13) and 16 tall at scale 1. */
function buoy(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -12, 12));
  const pulse = between(rng, 0.85, 1.15);
  const rx = 5 * pulse;
  const ry = 1.8 * pulse;
  const front = [30, 55, 90, 125, 150].map((t): Point => [(rx - 0.35) * Math.cos(t * DEGREES), (ry - 0.22) * Math.sin(t * DEGREES)]);
  return [
    shape(oval(f, 0, 0, rx + 1.6, ry + 1.2), "cyan-400", 0.14),
    shape(
      f.poly([[-1.9, -2.6], [-5, -4.6], [-5, -3.4], [-2.3, -0.9]]) +
        f.poly([[1.9, -2.6], [5, -4.6], [5, -3.4], [2.3, -0.9]]) +
        f.poly([[-1.6, 3.2], [-3.8, 6.6], [-2.6, 6.8], [-1.1, 4.6]]) +
        f.poly([[1.6, 3.2], [3.8, 6.6], [2.6, 6.8], [1.1, 4.6]]),
      "slate-700",
      1,
      INKED,
    ),
    shape(`M${f.p(0, -8)}C${f.p(2, -6)} ${f.p(2.8, -2)} ${f.p(2.6, 0)}C${f.p(2.8, 2)} ${f.p(2, 6)} ${f.p(0, 8)}C${f.p(-2, 6)} ${f.p(-2.8, 2)} ${f.p(-2.6, 0)}C${f.p(-2.8, -2)} ${f.p(-2, -6)} ${f.p(0, -8)}Z`, "slate-400", 1, MAIN_INKED),
    shape(f.band([[0.9, -5.6], [1.9, -2.6]], 0.4) + f.band([[1.9, 2.6], [0.9, 5.6]], 0.4), "white", 0.3),
    shape(box(f, -2.9, -0.9, 2.9, 0.9), "slate-700", 1, INKED),
    shape(box(f, -2.2, -0.32, 2.2, 0.32) + f.circle(0, -8.3, 0.6) + f.circle(0, 8.3, 0.6), "cyan-400", 1),
    shape(oval(f, 0, 0, rx, ry) + oval(f, 0, 0, rx - 0.7, ry - 0.45), "cyan-400", 0.35),
    shape(f.band(front, 0.3), "white", 0.35),
  ];
}

/** A detached starship module, centred on the anchor and tumbling: a plated block torn open at its back with cables trailing and sparks, a bolted hatch ring, a docking collar toward +x, a red light on top; about 14 wide and 10 tall at scale 1. */
function shipModule(at: PartPlace, rng: Rng): KitShape[] {
  const tilt = between(rng, -20, 20);
  const L = local(at);
  const f = tilted(L, tilt);
  const j = (): number => between(rng, -0.5, 0.5);
  const torn: Point[] = [
    [-6.6, 5],
    [-5.8 + j(), 3.4],
    [-7 + j(), 1.8],
    [-6 + j(), 0.2],
    [-7.2 + j(), -1.6],
    [-6 + j(), -3.2],
    [-6.6, -5],
  ];
  const spark = (x: number, y: number, size: number): string => sparkle(L, ...turned([x, y], tilt), size);
  return [
    shape(f.band([[-6.2, 1.2], [-8.2, 2.6], [-8.8, 4.8]], 0.45) + f.band([[-6.2, -1.8], [-8.6, -2], [-9.4, -0.6]], 0.45), "slate-800", 1),
    shape(f.poly([[4, -5], [4, 5], ...torn]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-3.6, -4.8], [-3.6, 4.8]], 0.4) + f.band([[2.6, -4.8], [2.6, 4.8]], 0.4), "slate-700", 0.7),
    shape(f.band([[-5.4, -4.4], [3.6, -4.4]], 0.5), "white", 0.25),
    shape(ring(f, -0.5, 0.2, 2.2, 1.5), "slate-700", 1, INKED),
    shape([0, 45, 90, 135, 180, 225, 270, 315].map((a) => f.circle(...polar(1.85, a, -0.5, 0.2), 0.25)).join(""), "slate-100", 1),
    shape(box(f, 4, -3.6, 5.6, 3.6) + box(f, 5.6, -4.2, 6.8, 4.2), "slate-700", 1, INKED),
    shape(f.circle(2.4, -5.6, 1.3), "red-400", 0.25),
    shape(f.circle(2.4, -5.6, 0.55), "red-400", 1),
    shape(spark(-7.2, 1.8, 1.3) + spark(-6.8, -2.6, 0.9), "orange-400", 1),
  ];
}

/** A missile flying toward its facing, centred on its body: a white body with a dark band, a red nose and red fins, a flame, a trail of smoke puffs about 14 behind it; about 10 long at scale 1. */
function missile(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -10, 10));
  const burn = between(rng, 3.5, 5.5);
  const drift = (): number => between(rng, -0.8, 0.8);
  const puffs: (readonly [number, number, number])[] = [
    [-7.4, 0, 1.1],
    [-10.2, drift(), 1.6],
    [-13.2, drift(), 2],
    [-16.3, drift(), 2.4],
    [-19.4, drift(), 2.8],
  ];
  const smoke = (odd: number): string =>
    puffs
      .filter((_, i) => i % 2 === odd)
      .map(([x, y, r]) => oval(f, x, y, r * 1.1, r))
      .join("");
  return [
    shape(smoke(0), "slate-400", 0.2),
    shape(smoke(1), "slate-400", 0.2),
    shape(tongue(f, [-5.4, 0], 180, burn, 1.5), "ember-500", 0.28),
    shape(tongue(f, [-5.4, 0], 180, burn * 0.7, 0.9), "orange-400", 0.9),
    shape(tongue(f, [-5.4, 0], 180, burn * 0.35, 0.45), "white", 0.85),
    shape(f.poly([[-2.6, -1], [-5.2, -3], [-5.6, -1]]) + f.poly([[-2.6, 1], [-5.2, 3], [-5.6, 1]]), "red-600", 1, INKED),
    shape(box(f, -5.6, -1.1, 3.2, 1.1), "slate-100", 1, MAIN_INKED),
    shape(box(f, -5.6, 0.25, 3.2, 1.1), "slate-400", 0.4),
    shape(box(f, -1.6, -1.1, -0.8, 1.1), "slate-700", 0.85),
    shape(`M${f.p(3.2, -1.1)}Q${f.p(5.4, -0.9)} ${f.p(6, 0)}Q${f.p(5.4, 0.9)} ${f.p(3.2, 1.1)}Z`, "red-500", 1, INKED),
  ];
}

export const CRAFT_KIT = {
  "space-wasp": { category: "creatures", draw: spaceWasp },
  drone: { category: "creatures", draw: drone },
  interceptor: { category: "creatures", draw: interceptor },
  corvette: { category: "creatures", draw: corvette },
  "escort-disc": { category: "creatures", draw: escortDisc },
  robot: { category: "creatures", draw: robot },
  "robot-wreck": { category: "creatures", draw: robotWreck },
  "combat-robot": { category: "creatures", draw: combatRobot },
  mech: { category: "creatures", draw: mech },
  spirit: { category: "creatures", draw: spirit },
  "crew-crowd": { category: "creatures", draw: crowd },
  alien: { category: "creatures", draw: alien },
  dog: { category: "creatures", draw: dog },
  "jelly-blob": { category: "creatures", draw: jellyBlob },
  "scrap-golem": { category: "creatures", draw: scrapGolem },
  "gloved-hand": { category: "creatures", draw: glovedHand },
  "robot-arm": { category: "creatures", draw: robotArm },
  "ai-eye": { category: "creatures", draw: aiEye },
  starship: { category: "objects", draw: starship },
  fighter: { category: "objects", draw: fighter },
  freighter: { category: "objects", draw: freighter },
  dropship: { category: "objects", draw: dropship },
  shuttle: { category: "objects", draw: shuttle },
  rocket: { category: "objects", draw: rocket },
  tank: { category: "objects", draw: tank },
  probe: { category: "objects", draw: probe },
  satellite: { category: "objects", draw: satellite },
  "satellite-dish": { category: "objects", draw: satelliteDish },
  rover: { category: "objects", draw: rover },
  buoy: { category: "objects", draw: buoy },
  "ship-module": { category: "objects", draw: shipModule },
  missile: { category: "objects", draw: missile },
} as const satisfies Readonly<Record<string, KitPart>>;
