// R3401000: the scene kit's curios (ART.md, "Scene recipes and kits"), which the Meditative scenes (#3401) added:
// the small marvels a Meditative card's whole picture can be of, each with a glint or a glow of its own. Jade (a
// river boulder sawn open on a glowing face, a polished bi disc, a many-sided die), a bottle with a lit town inside,
// a huge horned skull, food (a split melon and its slices on a board, a split dragon fruit with a card standing in
// it), a cracked hand mirror, a heart-shaped bomb and a torn sack spilling booster packs. The boulder, the skull,
// the melon, the dragon fruit and the sack stand on their anchor (it is their bottom centre); the disc, the die, the
// bottle and the bomb centre on it; the mirror lies with its handle's end on it. sceneKit.ts spreads the table into
// `SCENE_KIT`. Every colour is a palette.css name, none in Radiant's gold window; a glow is a stack of translucent
// shapes, never a gradient; shapes are filled even-odd, so a hole is a second sub-path and no two sub-paths of one
// shape overlap otherwise.

import { between, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, sparkle, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";

// Helpers

/** A point `r` from (`cx`, `cy`) at `degrees`, clockwise from the right on screen. */
function polar(r: number, degrees: number, cx = 0, cy = 0): Point {
  return [cx + r * Math.cos(degrees * DEGREES), cy + r * Math.sin(degrees * DEGREES)];
}

/** `p` moved by `k` times (`dx`, `dy`). */
function plus(p: Point, dx: number, dy: number, k = 1): Point {
  return [p[0] + dx * k, p[1] + dy * k];
}

/** A band along the points, `w0` wide at the first and narrowing (or widening) evenly to `w1` at the last. */
function taper(f: Local, points: readonly Point[], w0: number, w1: number): string {
  const left: Point[] = [];
  const right: Point[] = [];
  const last = points.length - 1;
  points.forEach(([x, y], i) => {
    const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
    const [nx, ny] = points[Math.min(last, i + 1)] ?? [x, y];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (w0 + ((w1 - w0) * i) / Math.max(1, last)) / 2;
    left.push([x - ((ny - py) / length) * half, y + ((nx - px) / length) * half]);
    right.push([x + ((ny - py) / length) * half, y - ((nx - px) / length) * half]);
  });
  return f.poly([...left, ...right.reverse()]);
}

/** A sliver hugging the inside of a circle of `r` about (`cx`, `cy`) from `from` to `to` degrees, `thick` at its middle: a rim's highlight. */
function sliver(f: Local, cx: number, cy: number, r: number, from: number, to: number, thick: number): string {
  const steps = 6;
  const outer: Point[] = [];
  const inner: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const angle = from + (to - from) * t;
    outer.push(polar(r, angle, cx, cy));
    if (i > 0 && i < steps) inner.push(polar(r - thick * Math.sin(Math.PI * t), angle, cx, cy));
  }
  return f.poly([...outer, ...inner.reverse()]);
}

/** A straight band from `a` to `b`, `w` wide, stopping `gap` short of each end, so bands meeting at a point never overlap. */
function trimmed(f: Local, a: Point, b: Point, w: number, gap: number): string {
  const length = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  const ux = (b[0] - a[0]) / length;
  const uy = (b[1] - a[1]) / length;
  return f.band(
    [
      [a[0] + ux * gap, a[1] + uy * gap],
      [b[0] - ux * gap, b[1] - uy * gap],
    ],
    w,
  );
}

/** A frame whose points are turned `degrees` about the anchor before they are placed. */
function turnedFrame(f: Local, degrees: number): { p(dx: number, dy: number): string; poly(points: readonly Point[]): string; circle(dx: number, dy: number, r: number): string } {
  const t = (dx: number, dy: number): Point => turned([dx, dy], degrees);
  return {
    p: (dx, dy) => f.p(...t(dx, dy)),
    poly: (points) => f.poly(points.map(([dx, dy]) => t(dx, dy))),
    circle: (dx, dy, r) => f.circle(...t(dx, dy), r),
  };
}

// Jade

/** A rough grey river boulder sawn flat across its upper right on a glowing face, standing on the anchor, about 14.5 wide and 10 tall at scale 1; `main` is the cut face and its glow (jade, red-600 for red jade, stone-500 for a dud). */
function jadeBoulder(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const jitter = (): number => between(rng, -0.4, 0.4);
  const a: Point = [1.8, -9.9 + jitter()];
  const b: Point = [6.6, -4.6 + jitter()];
  const body =
    `M${f.p(-6.4, 0)}Q${f.p(-7.6, -0.4)} ${f.p(-7.2, -3.4)}Q${f.p(-6.8, -7.4 + jitter())} ${f.p(-3.6, -9 + jitter())}` +
    `Q${f.p(-1, -10.4)} ${f.p(...a)}L${f.p(...b)}Q${f.p(7.6, -3)} ${f.p(7, -1)}Q${f.p(6.6, 0)} ${f.p(4.6, 0)}Z`;
  const length = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  const [ux, uy] = [(b[0] - a[0]) / length, (b[1] - a[1]) / length];
  const [nx, ny] = [-uy, ux];
  const mid: Point = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  const face = `M${f.p(...a)}L${f.p(...b)}Q${f.p(...plus(mid, nx, ny, 5.2))} ${f.p(...a)}Z`;
  const glow = plus(mid, nx, ny, -1.4);
  const spot = plus(mid, nx, ny, 1.1);
  const sheen = f.band([plus(plus(a, ux, uy, length * 0.2), nx, ny, 0.65), plus(plus(a, ux, uy, length * 0.78), nx, ny, 0.65)], 0.5);
  const spots: Point[] = [
    [-4.6, -5.4],
    [-2.2, -2.8],
    [-5.4, -2.2],
    [-1.4, -6.8],
  ];
  const pits = spots
    .slice(0, whole(rng, 3, 4))
    .map(([x, y]) => f.ellipse(x + jitter(), y + jitter(), between(rng, 0.4, 0.6), 0.28))
    .join("");
  return [
    shape(f.circle(...glow, 5.4), "jade-400", 0.12, MAIN),
    shape(f.circle(...glow, 3.6), "jade-400", 0.2, MAIN),
    shape(body, "slate-400", 1, INKED),
    shape(f.ellipse(-0.6, -1.6, 5.6, 1.2), "slate-700", 0.28),
    shape(pits, "slate-700", 0.5),
    shape(
      taper(
        f,
        [
          [-5.9, -5.6],
          [-4.8, -7.6],
          [-2.6, -8.8],
        ],
        1,
        0.3,
      ),
      "slate-100",
      0.55,
    ),
    shape(face, "jade-400", 1, MAIN_INKED),
    shape(sheen, "white", 0.45),
    shape(sparkle(f, ...spot, 1.4), "white", 0.9),
  ];
}

/** A polished jade bi disc (a ring) centred on the anchor, a faint wisp rising from it, about 9 across (16 tall with the wisp) at scale 1; `main` is the disc, its wisp and its glow. */
function jadeDisc(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  const dots = whole(rng, 7, 9);
  const start = between(rng, 0, 360);
  let grain = "";
  for (let i = 0; i < dots; i += 1) grain += f.circle(...polar(2.95, start + (i * 360) / dots), 0.26);
  const wisp: Point[] = [
    [0.3, -3.6],
    [1.4, -5.8],
    [-0.4, -8],
    [0.8 + sway, -10.6],
  ];
  return [
    shape(f.circle(0, 0, 7), "jade-400", 0.1, MAIN),
    shape(f.circle(0, 0, 5.6), "jade-400", 0.16, MAIN),
    shape(taper(f, wisp, 1.1, 0.15), "jade-400", 0.35, MAIN),
    shape(f.circle(0, 0, 4.5) + f.circle(0, 0, 1.5), "jade-400", 1, MAIN_INKED),
    shape(f.circle(0, 0, 4) + f.circle(0, 0, 3.65) + f.circle(0, 0, 2.25) + f.circle(0, 0, 1.95), "green-900", 0.3),
    shape(grain, "green-900", 0.3),
    shape(sliver(f, 0, 0, 4.15, 195, 265, 0.75), "white", 0.6),
    shape(sparkle(f, ...polar(3, 232), 1.4), "white", 0.9),
  ];
}

/** A many-sided jade die, blank faced, centred on the anchor, about 9 wide and 10.5 tall at scale 1; `main` is its body and its glow. */
function jadeDie(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -9, 9);
  const corner = (i: number, reach: number): Point => turned(polar(reach, -90 + 60 * i), tilt);
  const rim = [0, 1, 2, 3, 4, 5].map((i) => corner(i, 5.2));
  const core = [0, 2, 4].map((i) => corner(i, 2.6));
  const h = (i: number): Point => rim[i % 6] ?? [0, 0];
  const c = (i: number): Point => core[i % 3] ?? [0, 0];
  const edges: readonly (readonly [Point, Point])[] = [
    [c(0), c(1)],
    [c(1), c(2)],
    [c(2), c(0)],
    [c(0), h(5)],
    [c(0), h(0)],
    [c(0), h(1)],
    [c(1), h(1)],
    [c(1), h(2)],
    [c(1), h(3)],
    [c(2), h(3)],
    [c(2), h(4)],
    [c(2), h(5)],
  ];
  const lines = edges.map(([p, q]) => trimmed(f, p, q, 0.38, 0.36)).join("") + core.map(([x, y]) => f.circle(x, y, 0.3)).join("");
  const glint = turned([-2.4, -2.9], tilt);
  return [
    shape(f.circle(0, 0, 7.4), "jade-400", 0.1, MAIN),
    shape(f.circle(0, 0, 5.9), "jade-400", 0.16, MAIN),
    shape(f.poly(rim), "jade-400", 1, MAIN_INKED),
    shape(f.poly([h(4), h(5), h(0), c(0), c(2)]), "white", 0.32),
    shape(f.poly([h(1), h(2), h(3), c(2), c(1)]), "slate-900", 0.2),
    shape(lines, "slate-900", 0.55),
    shape(sparkle(f, ...glint, 1.4), "white", 0.9),
  ];
}

// Things of glass and bone

/** A corked glass bottle lying on its side, a lit town glowing inside, centred on the anchor, about 14 wide and 6 tall at scale 1; `main` is the glow inside and its lantern dots. */
function bottle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const t = turnedFrame(f, between(rng, -5, 5));
  const peak = between(rng, -1.7, -1.3);
  const glass =
    `M${t.p(-4, -3)}L${t.p(1.8, -3)}C${t.p(3.4, -3)} ${t.p(3.6, -1.1)} ${t.p(4.6, -1.1)}L${t.p(5.7, -1.1)}L${t.p(5.7, 1.1)}L${t.p(4.6, 1.1)}` +
    `C${t.p(3.6, 1.1)} ${t.p(3.4, 3)} ${t.p(1.8, 3)}L${t.p(-4, 3)}C${t.p(-7.6, 3)} ${t.p(-7.6, -3)} ${t.p(-4, -3)}Z`;
  const town =
    `M${t.p(-5.8, 2)}Q${t.p(-4.6, 1.3)} ${t.p(-3.2, 1.5)}L${t.p(-3.2, 0.2)}L${t.p(-3.9, 0.1)}Q${t.p(-2.6, -0.2)} ${t.p(-1.6, peak)}` +
    `Q${t.p(-0.6, -0.2)} ${t.p(0.7, 0.1)}L${t.p(0, 0.2)}L${t.p(0, 1.5)}Q${t.p(1.8, 1.5)} ${t.p(3, 2.2)}Q${t.p(-1.4, 2.8)} ${t.p(-5.8, 2)}Z`;
  const lights: Point[] = [
    [-2.4, 0.9],
    [-0.8, 0.9],
    [1.6, -0.9],
    [2.8, -0.4],
  ];
  const lanterns = lights
    .slice(0, whole(rng, 3, 4))
    .map(([x, y]) => t.circle(x, y + between(rng, -0.15, 0.15), 0.38))
    .join("");
  return [
    shape(f.ellipse(-0.6, 0, 8.6, 5.4), "orange-400", 0.12, MAIN),
    shape(glass, "slate-100", 0.3, INKED),
    shape(f.ellipse(-1.4, 0.3, 4.6, 2.3), "orange-400", 0.45, MAIN),
    shape(town, "slate-800", 0.85),
    shape(lanterns, "orange-400", 1, MAIN),
    shape(
      t.poly([
        [-4.8, -2],
        [-3.8, -2.45],
        [1.6, -2.45],
        [1.9, -2.15],
        [-3.6, -1.95],
      ]),
      "white",
      0.6,
    ),
    shape(t.poly([
      [5.3, -1.45],
      [5.9, -1.45],
      [5.9, 1.45],
      [5.3, 1.45],
    ]), "slate-400", 0.8),
    shape(t.poly([
      [5.9, -0.95],
      [7.3, -0.85],
      [7.3, 0.85],
      [5.9, 0.95],
    ]), "brass-600", 1, INKED),
    shape(sparkle(f, ...turned([-4.4, -1.6], 0), 1.3), "white", 0.95),
  ];
}

/** A huge horned skull, its jaw on the anchor, two horns curving out and up, about 16 wide and 12.5 tall at scale 1; `main` is the bone (the horns stay slate). */
function hornedSkull(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.4, 0.4);
  const horn: Point[] = [
    [3.6, -8.4],
    [5.8, -8.7],
    [7.4, -9.8],
    [8, -11.3],
    [7.3 - curl, -12.6],
  ];
  const mirror = (points: readonly Point[]): Point[] => points.map(([x, y]) => [-x, y]);
  const hornWidth = (i: number): number => 2.6 + ((0.15 - 2.6) * i) / (horn.length - 1);
  let ridges = "";
  for (const i of [1, 2, 3]) {
    const [x, y] = horn[i] ?? [0, 0];
    const [px, py] = horn[i - 1] ?? [x, y];
    const [nx, ny] = horn[i + 1] ?? [x, y];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const reach = hornWidth(i) * 0.42;
    const across: Point = [(-(ny - py) / length) * reach, ((nx - px) / length) * reach];
    for (const side of [1, -1]) {
      ridges += f.band(
        [
          [side * (x - across[0]), y - across[1]],
          [side * (x + across[0]), y + across[1]],
        ],
        0.3,
      );
    }
  }
  const skull =
    `M${f.p(0, 0)}C${f.p(1.4, 0)} ${f.p(2.6, -0.3)} ${f.p(2.9, -1.3)}L${f.p(3, -2.6)}C${f.p(3.6, -3)} ${f.p(4.4, -3.6)} ${f.p(4.5, -4.8)}` +
    `C${f.p(4.6, -5.6)} ${f.p(4.9, -6.6)} ${f.p(4.9, -7.6)}C${f.p(4.9, -10.2)} ${f.p(2.8, -11.4)} ${f.p(0, -11.4)}` +
    `C${f.p(-2.8, -11.4)} ${f.p(-4.9, -10.2)} ${f.p(-4.9, -7.6)}C${f.p(-4.9, -6.6)} ${f.p(-4.6, -5.6)} ${f.p(-4.5, -4.8)}` +
    `C${f.p(-4.4, -3.6)} ${f.p(-3.6, -3)} ${f.p(-3, -2.6)}L${f.p(-2.9, -1.3)}C${f.p(-2.6, -0.3)} ${f.p(-1.4, 0)} ${f.p(0, 0)}Z`;
  const socket = (s: number): string =>
    `M${f.p(s * 0.7, -6.3)}Q${f.p(s * 2, -7.7)} ${f.p(s * 3.6, -7.1)}Q${f.p(s * 3.7, -5)} ${f.p(s * 2, -4.9)}Q${f.p(s * 0.6, -5)} ${f.p(s * 0.7, -6.3)}Z`;
  const nose = `M${f.p(0, -4.5)}L${f.p(0.8, -3.1)}Q${f.p(0.4, -2.8)} ${f.p(0, -3.2)}Q${f.p(-0.4, -2.8)} ${f.p(-0.8, -3.1)}Z`;
  const hollows = (s: number): string => `M${f.p(s * 4.3, -4.6)}Q${f.p(s * 3.2, -6.3)} ${f.p(s * 4.4, -8.6)}Q${f.p(s * 4.9, -6.6)} ${f.p(s * 4.3, -4.6)}Z`;
  const gaps = [-1.65, -0.55, 0.55, 1.65].map((x) => f.band([[x, -2.5], [x * 0.92, -0.7]], 0.28)).join("");
  const crack: Point[] = [
    [1.3, -11.3],
    [2 + curl * 0.5, -10.2],
    [1.3, -9.4],
    [2, -8.5],
  ];
  return [
    shape(taper(f, horn, 2.6, 0.15) + taper(f, mirror(horn), 2.6, 0.15), "slate-700", 1, INKED),
    shape(ridges, "slate-400", 0.6),
    shape(skull, "slate-100", 1, MAIN_INKED),
    shape(hollows(1) + hollows(-1) + f.ellipse(0, -0.9, 1.8, 0.45), "slate-400", 0.45),
    shape(
      taper(
        f,
        [
          [-3.6, -9.2],
          [-2.2, -10.4],
          [-0.4, -10.8],
        ],
        0.9,
        0.3,
      ),
      "white",
      0.65,
    ),
    shape(socket(1) + socket(-1) + nose, "slate-900", 1),
    shape(f.band([[-2.5, -1.55], [0, -1.35], [2.5, -1.55]], 0.35), "slate-900", 1),
    shape(gaps, "slate-900", 1),
    shape(f.band(crack, 0.3), "slate-700", 0.8),
    shape(sparkle(f, -7.6, -11.4, 1.1), "white", 0.9),
  ];
}

// Food

/** A split melon on a cutting board standing on the anchor: a cut half and five or six glistening slices fanned out smallest to largest, about 22 wide and 8 tall at scale 1; `main` is the flesh. */
function melon(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = -2.6;
  const board =
    `M${f.p(-10.4, top)}L${f.p(10.4, top)}Q${f.p(11, top)} ${f.p(11, top + 0.6)}L${f.p(11, -0.6)}Q${f.p(11, 0)} ${f.p(10.4, 0)}` +
    `L${f.p(-10.4, 0)}Q${f.p(-11, 0)} ${f.p(-11, -0.6)}L${f.p(-11, top + 0.6)}Q${f.p(-11, top)} ${f.p(-10.4, top)}Z`;
  const cx = -5.2;
  const rind = `M${f.p(cx - 5.2, -5.4)}C${f.p(cx - 5.2, -1.6)} ${f.p(cx + 5.2, -1.6)} ${f.p(cx + 5.2, -5.4)}C${f.p(cx + 5.2, -8.4)} ${f.p(cx - 5.2, -8.4)} ${f.p(cx - 5.2, -5.4)}Z`;
  let stripes = "";
  for (const k of [-0.62, -0.22, 0.22, 0.62]) {
    const x = cx + 5 * k;
    const y = -5.4 + 2.1 * Math.sqrt(1 - k * k) + 0.35;
    stripes += taper(
      f,
      [
        [x, y],
        [cx + 4.4 * k, -3.6],
        [cx + 3 * k, -2.85],
      ],
      0.45,
      0.15,
    );
  }
  const count = whole(rng, 5, 6);
  let x = 0.6;
  let skins = "";
  let flesh = "";
  let dabs = "";
  let last: Point = [0, 0];
  for (let i = 0; i < count; i += 1) {
    const t = i / (count - 1);
    const h = 2.6 + 2.8 * t;
    const b = 0.38 * h;
    const lean = -12 + 24 * t + between(rng, -1.5, 1.5);
    const at2 = (dx: number, dy: number): Point => plus(turned([dx, dy], lean), x, top);
    const q = (dx: number, dy: number): string => f.p(...at2(dx, dy));
    skins += `M${q(0, 0)}Q${q(2 * b, -h / 2)} ${q(0, -h)}Q${q(0.5 * b, -h / 2)} ${q(0, 0)}Z`;
    flesh += `M${q(0, -0.15)}Q${q(1.45 * b, -h / 2)} ${q(0, -h + 0.15)}Q${q(0.5 * b, -h / 2)} ${q(0, -0.15)}Z`;
    dabs += f.circle(...at2(0.5 * b, -0.64 * h), 0.2);
    last = at2(0.55 * b, -0.82 * h);
    x += b * 0.95 + 0.3;
  }
  return [
    shape(board, "wood-600", 1, INKED),
    shape(f.band([[-10.6, -0.55], [10.6, -0.55]], 0.8) + f.band([[-9.6, -1.75], [-6, -1.8]], 0.25) + f.band([[3.6, -1.85], [9.4, -1.8]], 0.25), "wood-800", 0.5),
    shape(rind, "green-700", 1, INKED),
    shape(f.ellipse(cx, -5.4, 5, 2.1) + stripes, "lime-200", 1),
    shape(f.ellipse(cx, -5.35, 4.4, 1.75), "orange-400", 1, MAIN),
    shape(f.ellipse(cx, -5.3, 2.2, 0.85), "brass-600", 0.55),
    shape([-1.3, -0.5, 0.3, 1.1].map((dx) => f.ellipse(cx + dx, -5.3 + between(rng, -0.25, 0.25), 0.28, 0.17)).join(""), "lime-200", 0.9),
    shape(skins, "green-700", 1),
    shape(flesh, "orange-400", 1, MAIN),
    shape(dabs, "white", 0.7),
    shape(sparkle(f, ...last, 1.1) + sparkle(f, cx - 2.4, -6.1, 1), "white", 0.95),
  ];
}

/** A scale frill: a pointed flap from `base` reaching `length` at `degrees`, `half` wide at its root, and its tip's last part. */
function frill(base: Point, degrees: number, length: number, half: number): { flap: Point[]; tip: Point[] } {
  const [ux, uy] = polar(1, degrees);
  const [nx, ny] = [-uy, ux];
  const tip = plus(plus(base, ux, uy, length), 0, -0.5);
  const near = (along: number, across: number): Point => plus(plus(base, ux, uy, along), nx, ny, across);
  return {
    flap: [near(0, half), near(length * 0.55, half * 0.55), tip, near(length * 0.55, -half * 0.55), near(0, -half)],
    tip: [near(length * 0.62, half * 0.42), tip, near(length * 0.62, -half * 0.42)],
  };
}

/** A split dragon fruit, its cut face up, a small glowing card standing in its flesh, on the anchor, about 13 wide and 10 tall at scale 1; `main` is the skin and its frills (red-300 or purple-400). */
function dragonFruit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = (): number => between(rng, 2.2, 2.8);
  const back = [
    frill([-5.4, -4.4], 190, reach(), 0.9),
    frill([-4.8, -5.6], 225, reach(), 0.9),
    frill([-1.6, -6.3], 255, reach() * 0.8, 0.8),
    frill([2, -6.3], 290, reach() * 0.8, 0.8),
    frill([4.8, -5.6], 315, reach(), 0.9),
    frill([5.4, -4.4], 350, reach(), 0.9),
  ];
  const front = [frill([-3, -0.9], 200, 2.3, 0.8), frill([0.2, -0.5], 245, 2, 0.8), frill([3.2, -0.9], 330, 2.3, 0.8)];
  const body = `M${f.p(-6, -4)}C${f.p(-6, 1.33)} ${f.p(6, 1.33)} ${f.p(6, -4)}C${f.p(6, -7.2)} ${f.p(-6, -7.2)} ${f.p(-6, -4)}Z`;
  let seeds = "";
  for (const [row, y] of [-5.2, -4.5, -3.8, -3.1].entries()) {
    for (let x = -4.2 + (row % 2) * 0.55; x < 4.4; x += 1.1) {
      if ((x / 4.5) ** 2 + ((y + 4.1) / 1.6) ** 2 >= 0.85) continue;
      const sx = x + between(rng, -0.2, 0.2);
      const sy = y + between(rng, -0.12, 0.12);
      seeds += f.poly([
        [sx, sy - 0.24],
        [sx + 0.17, sy],
        [sx, sy + 0.24],
        [sx - 0.17, sy],
      ]);
    }
  }
  const card = turnedFrame(f, between(rng, 4, 12));
  const cardAt = (dx: number, dy: number): Point => plus(turned([dx, dy], 0), 0.4, -4.1);
  const cardBody = card.poly([cardAt(-1.5, 0), cardAt(-1.5, -4.3), cardAt(1.5, -4.3), cardAt(1.5, 0)].map((p) => turned(p, 0)));
  const mark = card.poly([cardAt(0, -3.05), cardAt(0.65, -2.15), cardAt(0, -1.25), cardAt(-0.65, -2.15)]);
  return [
    shape(back.map((one) => f.poly(one.flap)).join(""), "red-300", 1, MAIN),
    shape(body, "red-300", 1, MAIN_INKED),
    shape(front.map((one) => f.poly(one.flap)).join(""), "white", 0.32),
    shape([...back, ...front].map((one) => f.poly(one.tip)).join(""), "green-400", 1),
    shape(f.ellipse(0, -4.1, 5, 1.9), "white", 1),
    shape(seeds, "slate-900", 0.85),
    shape(f.ellipse(0.5, -6.2, 4, 3.6), "jade-400", 0.16),
    shape(f.ellipse(0.5, -6.3, 2.7, 2.8), "jade-400", 0.24),
    shape(cardBody, "white", 1, INKED),
    shape(mark, "red-600", 0.9),
    shape(sparkle(f, ...turned([2.2, -8.6], 0), 1.2), "white", 0.95),
  ];
}

// Keepsakes and mischief

/** A cracked hand mirror lying with its handle's end on the anchor and its round glass up, about 8 wide and 14 long at scale 1; `main` is the frame and the handle (wood-700). */
function handMirror(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const t = turnedFrame(f, between(rng, 6, 14));
  const lean = (dx: number, dy: number): Point => turned([dx, dy], 0);
  const centre: Point = [0, -10.4];
  const radius = 3.25;
  const hub: Point = [between(rng, 0.2, 1), centre[1] + between(rng, -0.2, 0.8)];
  const rays = whole(rng, 4, 5);
  const start = between(rng, 0, 360);
  const star: Point[] = [];
  const tips: Point[] = [];
  const kinks: Point[] = [];
  for (let k = 0; k < rays; k += 1) {
    const angle = start + (k * 360) / rays + between(rng, -16, 16);
    const [ux, uy] = polar(1, angle);
    const [dx, dy] = [hub[0] - centre[0], hub[1] - centre[1]];
    const along = dx * ux + dy * uy;
    const reach = -along + Math.sqrt(Math.max(0, along * along - (dx * dx + dy * dy - (radius * 0.9) ** 2)));
    const kink = plus(plus(hub, ux, uy, reach * between(rng, 0.4, 0.6)), -uy, ux, between(rng, -0.35, 0.35));
    const tip = plus(hub, ux, uy, reach);
    star.push(polar(0.4, start + ((k - 0.5) * 360) / rays, ...hub), plus(kink, uy, -ux, 0.17), tip, plus(kink, -uy, ux, 0.17));
    tips.push(tip);
    kinks.push(kink);
  }
  const shard = [hub, kinks[0] ?? hub, tips[0] ?? hub, tips[1] ?? hub, kinks[1] ?? hub];
  const streak = (offset: number, half: number): Point[] => {
    const [ux, uy] = [0.64, -0.77];
    const run = Math.sqrt(Math.max(0, radius * radius - offset * offset)) * 0.72;
    const middle = plus(centre, -uy, ux, offset);
    return [plus(plus(middle, ux, uy, -run), -uy, ux, -half), plus(plus(middle, ux, uy, run), -uy, ux, -half), plus(plus(middle, ux, uy, run), -uy, ux, half), plus(plus(middle, ux, uy, -run), -uy, ux, half)];
  };
  const handle = t.poly([
    lean(-0.6, -6.6),
    lean(0.6, -6.6),
    lean(0.75, -1.5),
    lean(1.15, -0.8),
    lean(0.9, -0.15),
    lean(0, 0),
    lean(-0.9, -0.15),
    lean(-1.15, -0.8),
    lean(-0.75, -1.5),
  ]);
  return [
    shape(handle, "wood-700", 1, MAIN_INKED),
    shape(t.poly([lean(-0.95, -6.9), lean(0.95, -6.9), lean(0.85, -6.1), lean(-0.85, -6.1)]), "brass-600", 1),
    shape(t.circle(...centre, radius), "blue-200", 1),
    shape(t.poly(streak(-1.3, 0.45)) + t.poly(streak(0.15, 0.22)), "white", 0.5),
    shape(t.poly(shard), "slate-700", 0.35),
    shape(t.poly(star), "slate-900", 0.9),
    shape(t.circle(...centre, 3.95) + t.circle(...centre, 3.2), "wood-700", 1, MAIN_INKED),
    shape(sliverTurned(t, centre, 3.85, 195, 255, 0.4), "white", 0.35),
    shape(sparkle(f, ...turned(plus(centre, -1.6, -1.2), between(rng, 6, 14) * 0), 1.2), "white", 0.95),
  ];
}

/** `sliver` drawn through a turned frame. */
function sliverTurned(t: { poly(points: readonly Point[]): string }, centre: Point, r: number, from: number, to: number, thick: number): string {
  const steps = 6;
  const outer: Point[] = [];
  const inner: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const s = i / steps;
    const angle = from + (to - from) * s;
    outer.push(polar(r, angle, ...centre));
    if (i > 0 && i < steps) inner.push(polar(r - thick * Math.sin(Math.PI * s), angle, ...centre));
  }
  return t.poly([...outer, ...inner.reverse()]);
}

/** A pink heart-shaped bomb centred on the anchor, a lipstick kiss on it and a short fuse sparking at its tip, about 10.5 wide and 9 tall (13 with the spark) at scale 1; `main` is its body (red-300). */
function heartBomb(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const heart =
    `M${f.p(0, -2.6)}C${f.p(-0.6, -4.8)} ${f.p(-5.2, -5.4)} ${f.p(-5.2, -1.6)}C${f.p(-5.2, 1.2)} ${f.p(-2.2, 2.8)} ${f.p(0, 4.8)}` +
    `C${f.p(2.2, 2.8)} ${f.p(5.2, 1.2)} ${f.p(5.2, -1.6)}C${f.p(5.2, -5.4)} ${f.p(0.6, -4.8)} ${f.p(0, -2.6)}Z`;
  const kissTurn = between(rng, -22, -8);
  const kissAt = (dx: number, dy: number): string => f.p(...plus(turned([dx, dy], kissTurn), 1.9, 1));
  const kiss =
    `M${kissAt(-1.4, 0)}C${kissAt(-1, -1)} ${kissAt(-0.4, -0.9)} ${kissAt(0, -0.45)}C${kissAt(0.4, -0.9)} ${kissAt(1, -1)} ${kissAt(1.4, 0)}Q${kissAt(0, -0.2)} ${kissAt(-1.4, 0)}Z` +
    `M${kissAt(-1.3, 0.2)}Q${kissAt(0, 0.05)} ${kissAt(1.3, 0.2)}C${kissAt(0.9, 1.15)} ${kissAt(-0.9, 1.15)} ${kissAt(-1.3, 0.2)}Z`;
  const capAt = (dx: number, dy: number): Point => plus(turned([dx, dy], 28), 2.7, -4.1);
  const spark: Point = [4.9 + between(rng, -0.3, 0.3), -7.8];
  const fuse: Point[] = [capAt(0, -0.9), [3.8, -6.1], [4.7, -6.7], spark];
  let flying = "";
  for (let k = 0; k < 3; k += 1) flying += sparkle(f, ...polar(between(rng, 1.8, 2.6), -150 + k * 75 + between(rng, -15, 15), ...spark), 0.55);
  return [
    shape(heart, "red-300", 1, MAIN_INKED),
    shape(
      taper(
        f,
        [
          [4.3, -1.2],
          [3.2, 1.2],
          [0.9, 3.2],
        ],
        1.2,
        0.3,
      ),
      "red-500",
      0.25,
    ),
    shape(
      taper(
        f,
        [
          [-4.2, -1.4],
          [-3.6, -3.1],
          [-2.2, -3.7],
        ],
        0.95,
        0.35,
      ) + f.circle(-4.2, 0, 0.35),
      "white",
      0.6,
    ),
    shape(kiss, "red-600", 0.9),
    shape(taper(f, fuse, 0.55, 0.4), "wood-700", 1),
    shape(f.poly([capAt(-0.8, -0.95), capAt(0.8, -0.95), capAt(0.9, 0.3), capAt(-0.9, 0.3)]), "slate-700", 1, INKED),
    shape(f.circle(...spark, 2.1), "orange-400", 0.25),
    shape(f.circle(...spark, 1.1), "orange-400", 0.5),
    shape(sparkle(f, ...spark, 1.6), "white", 1),
    shape(flying, "ember-500", 0.9),
  ];
}

/** A booster pack's body and its crimp and badge, about (`cx`, `cy`), 2.4 wide and 3.2 tall, turned `degrees`. */
function pack(f: Local, cx: number, cy: number, degrees: number): { body: string; trim: string } {
  const t = (dx: number, dy: number): Point => plus(turned([dx, dy], degrees), cx, cy);
  const q = (dx: number, dy: number): string => f.p(...t(dx, dy));
  const [w, h, c] = [1.2, 1.6, 0.4];
  const body =
    `M${q(-w + c, -h)}L${q(w - c, -h)}Q${q(w, -h)} ${q(w, -h + c)}L${q(w, h - c)}Q${q(w, h)} ${q(w - c, h)}` +
    `L${q(-w + c, h)}Q${q(-w, h)} ${q(-w, h - c)}L${q(-w, -h + c)}Q${q(-w, -h)} ${q(-w + c, -h)}Z`;
  const trim = f.poly([t(-0.7, -1.1), t(0.7, -1.1), t(0.7, -0.7), t(-0.7, -0.7)]) + f.circle(...t(0, 0.35), 0.45);
  return { body, trim };
}

/** A torn burlap sack standing on the anchor, booster packs spilling from a rip near its mouth, about 14.5 wide and 14 tall at scale 1; `main` is the sack (wood-600 or brass-800). */
function sack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tuft = (): number => between(rng, -0.3, 0.3);
  const body =
    `M${f.p(-5, 0)}L${f.p(2.4, 0)}C${f.p(4.6, 0)} ${f.p(5, -2.6)} ${f.p(4.4, -5.4)}C${f.p(3.9, -7.8)} ${f.p(1.8, -9.4)} ${f.p(0.8, -10.6)}` +
    `L${f.p(1, -11.2)}L${f.p(2.4, -13.6 + tuft())}L${f.p(1.2, -12.9)}L${f.p(0.2, -14 + tuft())}L${f.p(-0.9, -13)}L${f.p(-2.2, -13.8 + tuft())}` +
    `L${f.p(-2.6, -12.9)}L${f.p(-2, -11.2)}L${f.p(-2.2, -10.6)}C${f.p(-3.4, -9.4)} ${f.p(-7, -8.2)} ${f.p(-7.2, -5)}C${f.p(-7.4, -2)} ${f.p(-7, 0)} ${f.p(-5, 0)}Z`;
  const ripAt: Point = [1.7, -7.3];
  const rip: Point[] = [];
  for (let i = 0; i < 10; i += 1) {
    const reach = i % 2 === 0 ? 1 : between(rng, 0.62, 0.78);
    const [x, y] = turned([1.45 * reach * Math.cos(i * 36 * DEGREES), 0.95 * reach * Math.sin(i * 36 * DEGREES)], -38);
    rip.push([ripAt[0] + x, ripAt[1] + y]);
  }
  const colours: readonly PaletteName[] = ["red-600", "sky-400", "violet-500", "lime-400"];
  const first = whole(rng, 0, 3);
  const spots: readonly (readonly [number, number, number])[] = [
    [2.8, -8.6, 40 + between(rng, -6, 6)],
    [5.4, -4.6, -18 + between(rng, -6, 6)],
    [6.4, -1.25, 76 + between(rng, -4, 4)],
    [2.4, -1.7, 6 + between(rng, -5, 5)],
  ];
  const packs = spots.slice(0, whole(rng, 3, 4)).map(([x, y, turn]) => pack(f, x, y, turn));
  const folds =
    taper(
      f,
      [
        [-1.2, -10.2],
        [-2.6, -7.6],
        [-3.6, -4.2],
      ],
      0.45,
      0.15,
    ) +
    taper(
      f,
      [
        [-0.2, -10],
        [-0.4, -7.4],
      ],
      0.35,
      0.12,
    ) +
    taper(
      f,
      [
        [-5.6, -1.8],
        [-3.4, -0.6],
      ],
      0.35,
      0.12,
    );
  return [
    shape(body, "wood-600", 1, MAIN_INKED),
    shape(
      taper(
        f,
        [
          [-6.3, -1.2],
          [-6.5, -4.6],
          [-5.2, -7.6],
        ],
        1.6,
        0.4,
      ),
      "wood-950",
      0.22,
    ),
    shape(folds, "wood-950", 0.35),
    shape(f.poly(rip), "wood-950", 1),
    shape(f.band([[-2.2, -11], [-0.5, -11.15], [1.1, -11]], 0.9) + f.band([[0.5, -10.45], [1.1, -9.3]], 0.4), "brass-600", 1),
    ...packs.map(({ body: one }, k) => shape(one, colours[(first + k) % colours.length] ?? "red-600", 1, INKED)),
    shape(packs.map(({ trim }) => trim).join(""), "white", 0.55),
    shape(sparkle(f, 5.9, -5.7, 1.1), "white", 0.95),
  ];
}

/** R3401000: the curios the Meditative scenes added, by id, each with its kind. */
/** A few small pink hearts drifting up and away from the anchor the way it faces, each with a pale highlight; about 13 wide and 9 tall at scale 1, `main` their pink (red-300). */
function inkHearts(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const heart = (cx: number, cy: number, r: number): string =>
    `M${f.p(cx, cy - r * 0.5)}C${f.p(cx - r * 0.2, cy - r * 1.3)} ${f.p(cx - r * 1.4, cy - r * 1.4)} ${f.p(cx - r * 1.4, cy - r * 0.3)}` +
    `C${f.p(cx - r * 1.4, cy + r * 0.4)} ${f.p(cx - r * 0.6, cy + r * 0.9)} ${f.p(cx, cy + r * 1.4)}` +
    `C${f.p(cx + r * 0.6, cy + r * 0.9)} ${f.p(cx + r * 1.4, cy + r * 0.4)} ${f.p(cx + r * 1.4, cy - r * 0.3)}` +
    `C${f.p(cx + r * 1.4, cy - r * 1.4)} ${f.p(cx + r * 0.2, cy - r * 1.3)} ${f.p(cx, cy - r * 0.5)}Z`;
  const spots = Array.from({ length: whole(rng, 3, 4) }, (_, k) => ({ x: -5 + k * 3.6 + between(rng, -0.5, 0.5), y: 2 - k * 2.6 + between(rng, -0.6, 0.6), r: 1.25 - k * 0.12 }));
  return [
    shape(spots.map(({ x, y, r }) => heart(x, y, r)).join(""), "red-300", 1, MAIN_INKED),
    shape(spots.map(({ x, y, r }) => f.circle(x - r * 0.65, y - r * 0.55, r * 0.28)).join(""), "white", 0.8),
  ];
}

/** A torn ticket stub centred on the anchor, tilted by the seed: a paper slip (main, slate-100) with a red printed band, two printed lines and a dotted tear line before its ragged torn end; about 9 wide and 4.6 tall at scale 1. */
function ticketStub(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -14, 14);
  const T = (p: Point): Point => turned(p, tilt);
  const torn: Point[] = Array.from({ length: 6 }, (_, k): Point => [4.4 + (k % 2 === 0 ? 0 : between(rng, 0.4, 0.9)), -2.3 + k * 0.92]);
  const slip: Point[] = [[-4.4, -2.3], ...torn, [-4.4, 2.3]];
  const band: Point[] = [[-3.6, -1.6], [2.4, -1.6], [2.4, -0.5], [-3.6, -0.5]];
  const dots = Array.from({ length: 4 }, (_, k) => f.circle(...T([3.4, -1.6 + k * 1.05]), 0.2)).join("");
  return [
    shape(f.poly(slip.map(T)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(band.map(T)), "red-600", 0.9),
    shape(f.band([T([-3.6, 0.5]), T([2, 0.5])], 0.3) + f.band([T([-3.6, 1.4]), T([0.8, 1.4])], 0.3), "slate-700", 0.7),
    shape(dots, "slate-400", 0.9),
  ];
}

export const INK_CURIOS_KIT = {
  "jade-boulder": { category: "objects", draw: jadeBoulder },
  "jade-disc": { category: "objects", draw: jadeDisc },
  "jade-die": { category: "objects", draw: jadeDie },
  bottle: { category: "objects", draw: bottle },
  "horned-skull": { category: "objects", draw: hornedSkull },
  melon: { category: "objects", draw: melon },
  "dragon-fruit": { category: "objects", draw: dragonFruit },
  "hand-mirror": { category: "objects", draw: handMirror },
  "heart-bomb": { category: "objects", draw: heartBomb },
  "booster-sack": { category: "objects", draw: sack },
  "ink-hearts": { category: "weather", draw: inkHearts },
  "ticket-stub": { category: "objects", draw: ticketStub },
} as const satisfies Readonly<Record<string, KitPart>>;
