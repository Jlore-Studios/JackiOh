// R3390000: the scene kit's objects (ART.md, "Scene recipes and kits"), which the Classic+ scenes (#3390) added:
// paper and books (a scroll, a tome, a playing card, dice, a quill in its inkwell), treasure (coins, a chest, gems,
// a gift box), tools and machines (a gear, pipes, an hourglass, an astrolabe, a spatula, bellows, a slate board),
// weapons and training (an axe, a ring of blades, an arrow, a weapon rack, weights, knuckles), potions (a flask, a
// cauldron), locks and chains (a padlock, a chain, a key), the dead and the humble (a candle, bones, pebbles,
// bread), a drum, a lyre, a pair of scales and a bamboo chute; R3397000: the Meditative+ and Training Grounds Alt art
// scenes (#3397) added a cork board, a washing line, a picnic blanket and a hanging rope; R3398000's Core Alt art
// scenes (#3398) added a hay bale, a gingham blanket (`gingham-blanket`, apart from R3397000's `picnic-blanket`), a
// rug, a hammock (`beach-hammock`, apart from the Legacy touches' `hammock`), a sleeping bag and a teapot pouring
// (`pouring-teapot`, apart from the Meditative ink kit's `teapot`). sceneKit.ts spreads the table into `SCENE_KIT`.
// Each part keeps the kit's rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the
// scene's stream alone, every colour a palette name out of Radiant's gold window (brass and orange stand for gold), a
// glow a stack of translucent shapes, and no two sub-paths of one shape overlapping but for a hole.

import { fmt } from "./hash.ts";
import { between, blob, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, sparkle, spiralArm, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** An upright rectangle from corner (x0, y0) to (x1, y1). */
function box(f: Local, x0: number, y0: number, x1: number, y1: number): string {
  return f.poly([
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ]);
}

/** A straight band from `a` to `b`, `w` wide. */
function seg(f: Local, a: Point, b: Point, w: number): string {
  return f.band([a, b], w);
}

/** `point` plus `along` of the unit vector from `a` to `b` and `across` of its left normal. */
function offset(a: Point, b: Point, along: number, across: number): Point {
  const length = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  const ux = (b[0] - a[0]) / length;
  const uy = (b[1] - a[1]) / length;
  return [a[0] + ux * along - uy * across, a[1] + uy * along + ux * across];
}

/** A bone from `a` to `b`, `w` thick, a knob at each end. */
function bonePoly(a: Point, b: Point, w: number): Point[] {
  const h = w / 2;
  const k = w * 0.85;
  const j = w * 0.4;
  return [
    offset(a, b, -j, k),
    offset(a, b, j, k),
    offset(a, b, 3 * j, h),
    offset(b, a, 3 * j, -h),
    offset(b, a, j, -k),
    offset(b, a, -j, -k),
    offset(b, a, -j, k),
    offset(b, a, j, k),
    offset(b, a, 3 * j, h),
    offset(a, b, 3 * j, -h),
    offset(a, b, j, -k),
    offset(a, b, -j, -k),
  ];
}

/** A rounded capsule about the origin, `a` its half-length and `b` its half-width, turned `degrees` and moved to (cx, cy). */
function capsule(a: number, b: number, degrees: number, cx: number, cy: number): Point[] {
  const raw: Point[] = [
    [-a, -b * 0.5],
    [-a + b * 0.4, -b],
    [a - b * 0.4, -b],
    [a, -b * 0.5],
    [a, b * 0.5],
    [a - b * 0.4, b],
    [-a + b * 0.4, b],
    [-a, b * 0.5],
  ];
  return raw.map((p) => {
    const [x, y] = turned(p, degrees);
    return [cx + x, cy + y] as const;
  });
}

// Paper and books

/** A scroll lying half unrolled on the anchor, 16 wide and 6 tall at scale 1. */
function scroll(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bow = between(rng, 0.3, 1.1);
  const lift = between(rng, -0.4, 0.4);
  return [
    shape(
      `M${f.p(-6.8, -4.6 + lift)}Q${f.p(0, -4.6 - bow - 1)} ${f.p(6.8, -4.4 - lift)}L${f.p(6.8, -1.2)}Q${f.p(0, -1.2 + bow)} ${f.p(-6.8, -1.2)}Z`,
      "slate-100",
      1,
      MAIN_INKED,
    ),
    shape(`${seg(f, [-5, -3.5], [4.6, -3.4], 0.35)}${seg(f, [-5, -2.2], [2.4, -2.1], 0.35)}`, "slate-400", 0.6),
    shape(`${f.ellipse(-6.8, -2.9, 1.5, 2.9)}${f.ellipse(6.8, -2.9, 1.5, 2.9)}`, "slate-700", 1, INKED),
    shape(`${f.ellipse(-6.9, -2.9, 0.8, 2)}${f.ellipse(6.9, -2.9, 0.8, 2)}`, "slate-100", 1),
  ];
}

/** A heavy open book on the anchor, its leather cover, fanned pages and a ribbon, 16 wide and 7 tall at scale 1. */
function tome(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.4, 0.4);
  const page = (top: number): string =>
    `M${f.p(-7.4, -1.6)}L${f.p(-7.4, top)}Q${f.p(-3.8, top - 1.8 + lift)} ${f.p(-0.2, top + 1.2)}L${f.p(-0.2, -1.6)}Z` +
    `M${f.p(7.4, -1.6)}L${f.p(7.4, top)}Q${f.p(3.8, top - 1.8 - lift)} ${f.p(0.2, top + 1.2)}L${f.p(0.2, -1.6)}Z`;
  return [
    shape(
      f.poly([
        [-8.4, -1.8],
        [8.4, -1.8],
        [7.8, 0],
        [-7.8, 0],
      ]),
      "red-900",
      1,
      MAIN_INKED,
    ),
    shape(page(-4.8), "slate-400", 1, INKED),
    shape(page(-3.6), "slate-100", 1, INKED),
    shape(seg(f, [0, -4.4], [0, -1.6], 0.5), "slate-400", 0.8),
    shape(seg(f, [1, -3.2], [1.5, 0.8], 0.8), "red-500", 1),
  ];
}

/** One blank playing card standing tilted by up to 15 degrees, bottom at the anchor, 7 wide and 10 tall at scale 1. */
function card(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -15, 15);
  const T = (x: number, y: number): Point => turned([x, y], tilt);
  const rect = (x0: number, y0: number, x1: number, y1: number): string =>
    f.poly([T(x0, y0), T(x1, y0), T(x1, y1), T(x0, y1)]);
  return [
    shape(rect(-3.5, -10, 3.5, 0), "white", 1, INKED),
    shape(rect(-2.8, -9.3, 2.8, -0.7), "blue-800", 1, MAIN),
    shape(f.poly([T(0, -7.6), T(1.5, -5), T(0, -2.4), T(-1.5, -5)]), "slate-100", 0.85),
  ];
}

/** Two dice with pips, on the anchor, 7 wide and 3 tall at scale 1. */
function dice(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const faces: readonly (readonly Point[])[] = [
    [[0, 0]],
    [
      [-0.8, -0.8],
      [0.8, 0.8],
    ],
    [
      [-0.8, -0.8],
      [0, 0],
      [0.8, 0.8],
    ],
    [
      [-0.8, -0.8],
      [0.8, -0.8],
      [-0.8, 0.8],
      [0.8, 0.8],
    ],
    [
      [-0.8, -0.8],
      [0.8, -0.8],
      [0, 0],
      [-0.8, 0.8],
      [0.8, 0.8],
    ],
    [
      [-0.8, -0.9],
      [0.8, -0.9],
      [-0.8, 0],
      [0.8, 0],
      [-0.8, 0.9],
      [0.8, 0.9],
    ],
  ];
  const one = (cx: number, cy: number, tilt: number, face: number): { body: string; pips: string } => {
    const at2 = (x: number, y: number): Point => {
      const [tx, ty] = turned([x, y], tilt);
      return [cx + tx, cy + ty];
    };
    const half = 1.5;
    return {
      body: f.poly([at2(-half, -half), at2(half, -half), at2(half, half), at2(-half, half)]),
      pips: (faces[face] ?? []).map(([x, y]) => f.circle(...at2(x, y), 0.3)).join(""),
    };
  };
  const a = one(-2, -1.5, between(rng, -6, 6), whole(rng, 0, 5));
  const b = one(2.2, -1.5, between(rng, 10, 25), whole(rng, 0, 5));
  return [shape(a.body, "slate-100", 1, MAIN_INKED), shape(b.body, "slate-100", 1, MAIN_INKED), shape(`${a.pips}${b.pips}`, "slate-900", 1)];
}

/** A feather quill standing in an inkwell, on the anchor, 4 wide and 13 tall at scale 1. */
function quill(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 10);
  const T = (x: number, y: number): string => {
    const [tx, ty] = turned([x, y + 3], tilt);
    return f.p(tx, ty - 3);
  };
  return [
    shape(`M${T(0, -5.5)}C${T(-3, -7.5)} ${T(-2.6, -11)} ${T(0, -13.2)}C${T(1.8, -11)} ${T(2.3, -8)} ${T(0, -5.5)}Z`, "slate-100", 1, MAIN_INKED),
    shape(
      `M${T(0, -2.8)}L${T(0.18, -2.8)}L${T(0.18, -12.4)}L${T(-0.18, -12.4)}L${T(-0.18, -2.8)}Z`,
      "slate-400",
      0.9,
    ),
    shape(
      f.poly([
        [-2, 0],
        [2, 0],
        [2, -2.4],
        [1.1, -3],
        [-1.1, -3],
        [-2, -2.4],
      ]),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.ellipse(0, -3, 1.2, 0.35), "slate-900", 1),
  ];
}

// Treasure

/** A short stack of silver coins and two or three loose ones, on the anchor, 12 wide and 6 tall at scale 1. */
function coins(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const height = 1.1;
  const stack = whole(rng, 3, 4);
  const sides: string[] = [];
  const tops: string[] = [];
  for (let i = 0; i < stack; i += 1) {
    const cx = -3 + between(rng, -0.3, 0.3);
    const base = -i * height;
    sides.push(
      `M${f.p(cx - 2.6, base)}L${f.p(cx - 2.6, base - height)}L${f.p(cx + 2.6, base - height)}L${f.p(cx + 2.6, base)}Q${f.p(cx, base + 1.8)} ${f.p(cx - 2.6, base)}Z`,
    );
    tops.push(f.ellipse(cx, base - height, 2.6, 0.9));
  }
  const out: KitShape[] = [];
  sides.forEach((side, i) => {
    out.push(shape(side, "slate-400", 1, i === 0 ? MAIN_INKED : INKED));
    out.push(shape(tops[i] ?? "", "slate-100", 1, INKED));
  });
  out.push(shape(`${f.ellipse(2.2, -0.8, 2.3, 0.8)}`, "slate-400", 1, MAIN_INKED));
  out.push(shape(`${f.ellipse(5.2, -1.5, 2, 0.7)}`, "slate-400", 1, MAIN_INKED));
  return out.slice(0, 12);
}

/** A wooden chest on the anchor, its lid thrown open, iron bands and a glint inside, 16 wide and 13 tall at scale 1. */
function chest(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glint = between(rng, -2.5, 2.5);
  return [
    shape(
      f.poly([
        [-8, -6],
        [-7, -13],
        [7, -13],
        [8, -6],
      ]),
      "wood-800",
      1,
      INKED,
    ),
    shape(`${box(f, -5.6, -12.6, -4, -6.4)}${box(f, 4, -12.6, 5.6, -6.4)}`, "slate-700", 1),
    shape(
      f.poly([
        [-7.4, -6],
        [-6.6, -8.4],
        [6.6, -8.4],
        [7.4, -6],
      ]),
      "black",
      0.75,
    ),
    shape(f.circle(glint, -8.6, 4.2), "orange-400", 0.2),
    shape(f.circle(glint, -8.6, 2.6), "orange-400", 0.3),
    shape(sparkle(f, glint, -8.8, 2.8), "white", 0.95),
    shape(box(f, -8, -6, 8, 0), "wood-700", 1, MAIN_INKED),
    shape(`${box(f, -5.6, -6, -4, 0)}${box(f, 4, -6, 5.6, 0)}${box(f, -8, -3.4, 8, -2.6)}`, "slate-700", 1),
    shape(box(f, -1, -4.8, 1, -2.4), "brass-600", 1, INKED),
  ];
}

/** A scatter of four to six faceted gems about the anchor, 14 wide and 4 tall at scale 1; the biggest is `main`. */
function gems(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 4, 6);
  const colours = ["cyan-400", "red-500", "green-400", "violet-500", "orange-400"] as const;
  const out: KitShape[] = [];
  for (let i = 0; i < count; i += 1) {
    const cx = -6 + (12 * i) / (count - 1) + between(rng, -0.4, 0.4);
    const r = i === 0 ? 2.1 : between(rng, 1.1, 1.8);
    const cy = i === 0 ? -r * 0.7 : -1 + between(rng, -0.6, 0.6);
    const x = i === 0 ? 0 : cx;
    const body = f.poly([
      [x - r, cy - r * 0.3],
      [x - r * 0.5, cy - r],
      [x + r * 0.5, cy - r],
      [x + r, cy - r * 0.3],
      [x, cy + r * 0.8],
    ]);
    out.push(shape(body, colours[i % colours.length] ?? "cyan-400", 1, i === 0 ? MAIN_INKED : INKED));
    if (i < 3) {
      out.push(
        shape(
          f.poly([
            [x - r * 0.5, cy - r],
            [x + r * 0.5, cy - r],
            [x + r * 0.1, cy - r * 0.3],
            [x - r * 0.6, cy - r * 0.3],
          ]),
          "white",
          0.45,
        ),
      );
    }
  }
  return out;
}

/** A ribboned gift box with a bow, on the anchor, 8 wide and 9 tall at scale 1. */
function giftBox(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const loop = between(rng, 0.8, 1.1);
  return [
    shape(box(f, -4, -6.4, 4, 0), "red-600", 1, MAIN_INKED),
    shape(box(f, -4.6, -8, 4.6, -6.2), "red-600", 1, MAIN_INKED),
    shape(box(f, -0.9, -8, 0.9, 0), "slate-100", 1),
    shape(`${f.ellipse(-1.9, -8.7, 1.9, loop)}`, "slate-100", 1, INKED),
    shape(`${f.ellipse(1.9, -8.7, 1.9, loop)}`, "slate-100", 1, INKED),
    shape(f.circle(0, -8.2, 0.8), "slate-400", 1, INKED),
  ];
}

// Tools and machines

/** An hourglass in a wooden frame, sand running, on the anchor, 7 wide and 12 tall at scale 1. */
function hourglass(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const run = between(rng, 0.8, 1.2);
  return [
    shape(
      f.poly([
        [-2.5, -10.8],
        [2.5, -10.8],
        [0.4, -6],
        [2.5, -1.2],
        [-2.5, -1.2],
        [-0.4, -6],
      ]),
      "sky-300",
      0.35,
    ),
    shape(
      `${f.poly([
        [-1.9, -10.3],
        [1.9, -10.3],
        [0.3, -6.6 + run * 0.3],
        [-0.3, -6.6 + run * 0.3],
      ])}${f.poly([
        [-2.2, -1.2],
        [2.2, -1.2],
        [0, -4 + run * 0.4],
      ])}`,
      "brass-500",
      1,
      MAIN,
    ),
    shape(seg(f, [0, -6.2], [0, -3.8], 0.3), "brass-500", 1),
    shape(`${seg(f, [-3, -11], [-3, -1], 0.8)}${seg(f, [3, -11], [3, -1], 0.8)}`, "wood-600", 1, INKED),
    shape(`${box(f, -3.5, -12, 3.5, -10.8)}${box(f, -3.5, -1.2, 3.5, 0)}`, "wood-700", 1, INKED),
  ];
}

/** A toothed gear with a hub hole, centred on the anchor, radius 8 at scale 1. */
function gear(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const teeth = whole(rng, 8, 10);
  const start = between(rng, 0, 360 / teeth);
  const pts: Point[] = [];
  for (let i = 0; i < teeth; i += 1) {
    const a = start + (i * 360) / teeth;
    const arc = 360 / teeth;
    for (const [da, r] of [
      [-0.3 * arc, 6.2],
      [-0.17 * arc, 8],
      [0.17 * arc, 8],
      [0.3 * arc, 6.2],
    ] as const) {
      pts.push([r * Math.cos((a + da) * DEGREES), r * Math.sin((a + da) * DEGREES)]);
    }
  }
  return [
    shape(f.circle(0, 0, 2.4), "slate-900", 0.8),
    shape(`${f.poly(pts)}${f.circle(0, 0, 2.4)}`, "slate-400", 1, MAIN_INKED),
    shape(`${f.circle(0, 0, 4.7)}${f.circle(0, 0, 3.7)}`, "slate-100", 0.3),
  ];
}

/** Copper pipes with two elbows, a flange and a valve wheel, bottom centre at the anchor, 30 wide and 16 tall at scale 1. */
function pipes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wheel = between(rng, 2.6, 3.2);
  return [
    shape(seg(f, [-1, -13.5], [-1, -15.4], 0.9), "slate-700", 1),
    shape(f.ellipse(-1, -15.6, wheel, 0.9), "red-600", 1, INKED),
    shape(
      f.poly([
        [-12.5, 0],
        [-12.5, -13.5],
        [9.5, -13.5],
        [9.5, -5.5],
        [15, -5.5],
        [15, -2.5],
        [6.5, -2.5],
        [6.5, -10.5],
        [-9.5, -10.5],
        [-9.5, 0],
      ]),
      "brass-600",
      1,
      MAIN_INKED,
    ),
    shape(seg(f, [-8.6, -12.4], [8, -12.4], 0.6), "orange-400", 0.5),
    shape(`${box(f, -13.5, -6, -8.5, -4.6)}${box(f, 12, -7, 13.6, -1.2)}`, "brass-800", 1, INKED),
    shape(`${box(f, -13.2, -14.2, -8.8, -9.8)}${box(f, 5.8, -14.2, 10.2, -9.8)}`, "brass-800", 1, INKED),
  ];
}

/** An iron spatula planted blade-down, handle up, its foot at the anchor, 20 tall at scale 1. */
function spatula(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -7, 7);
  const T = (x: number, y: number): Point => turned([x, y], tilt);
  const slot = (x: number): string => f.poly([T(x, -4.8), T(x + 0.7, -4.8), T(x + 0.7, -1.4), T(x, -1.4)]);
  return [
    shape(f.poly([T(-2.5, 0), T(2.5, 0), T(2.5, -6.2), T(-2.5, -6.2)]) + slot(-1.7) + slot(1), "slate-400", 1, MAIN_INKED),
    shape(f.band([T(0, -6), T(0, -9.5)], 0.9), "slate-700", 1),
    shape(f.band([T(0, -9), T(0, -19.4)], 1.7), "wood-700", 1, INKED),
    shape(f.band([T(-0.3, -10), T(-0.3, -18)], 0.4), "wood-600", 0.7),
    shape(f.circle(...T(0, -18.2), 0.4), "slate-900", 0.8),
  ];
}

/** Leather bellows, nozzle pointing the way it faces, handles behind, centred on the anchor, 14 wide and 8 tall at scale 1. */
function bellows(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gape = between(rng, 3, 3.8);
  return [
    shape(`${seg(f, [-5, -gape], [-7.4, -gape - 0.7], 1)}`, "wood-600", 1, INKED),
    shape(`${seg(f, [-5, gape], [-7.4, gape + 0.7], 1)}`, "wood-600", 1, INKED),
    shape(
      f.poly([
        [3, -1.2],
        [7.2, -0.5],
        [7.2, 0.5],
        [3, 1.2],
      ]),
      "slate-700",
      1,
      INKED,
    ),
    shape(
      f.poly([
        [-4.5, -gape + 0.4],
        [2.8, -1.1],
        [2.8, 1.1],
        [-4.5, gape - 0.4],
      ]),
      "wood-700",
      1,
      MAIN_INKED,
    ),
    shape(`${seg(f, [-1.5, -gape * 0.55], [-1.5, gape * 0.55], 0.4)}${seg(f, [0.8, -1.6], [0.8, 1.6], 0.4)}`, "slate-900", 0.4),
    shape(`${seg(f, [-5, -gape - 0.3], [3, -1.4], 1.2)}${seg(f, [-5, gape + 0.3], [3, 1.4], 1.2)}`, "wood-800", 1, INKED),
  ];
}

/** A wooden-framed dark slate on short legs with chalk shapes on it, on the anchor, 18 wide and 14 tall at scale 1. */
function slateBoard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, 0, 1.2);
  return [
    shape(`${seg(f, [-6, -4.5], [-6.6, 0], 1.3)}${seg(f, [6, -4.5], [6.6, 0], 1.3)}`, "wood-700", 1, INKED),
    shape(box(f, -9, -14, 9, -4), "wood-700", 1, INKED),
    shape(box(f, -8, -13, 8, -5), "slate-800", 1, MAIN),
    shape(
      `${f.poly([
        [-6.5, -6.2],
        [-1.5, -6.2],
        [-4, -11.2],
      ])}${f.poly([
        [-5.5, -6.87],
        [-2.5, -6.87],
        [-4, -9.87],
      ])}`,
      "slate-100",
      0.8,
    ),
    shape(f.band([[0.5, -6.8], [2.5, -9.5 - bend], [4.6, -11.2], [6.6, -9.4]], 0.55), "slate-100", 0.8),
    shape(
      f.poly([
        [1.5, -6.2],
        [3.7, -6.2],
        [2.6, -8],
      ]),
      "slate-100",
      0.6,
    ),
  ];
}

/** A brass armillary: rings on a stand, on the anchor, 12 wide and 16 tall at scale 1. */
function astrolabe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slant = between(rng, -1, 1);
  return [
    shape(f.ellipse(0, -0.9, 4.5, 0.9), "brass-600", 1, INKED),
    shape(seg(f, [0, -1], [0, -5.5], 1.3), "brass-600", 1, INKED),
    shape(seg(f, [-4.5, -14 + slant], [4.5, -7 - slant], 0.8), "brass-800", 1),
    shape(`${f.ellipse(0, -10.5, 6, 2)}${f.ellipse(0, -10.5, 5.1, 1.2)}`, "brass-600", 1, INKED),
    shape(`${f.ellipse(0, -10.5, 2.5, 6)}${f.ellipse(0, -10.5, 1.7, 5.2)}`, "brass-600", 1, INKED),
    shape(`${f.circle(0, -10.5, 6)}${f.circle(0, -10.5, 5.2)}`, "brass-500", 1, MAIN_INKED),
    shape(f.circle(0, -10.5, 1.5), "orange-400", 1, INKED),
  ];
}

// Weapons and training

/** A broad executioner's axe, its haft rising from the anchor leant 20 to 35 degrees to the right, 22 long at scale 1. */
function axe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 20, 35);
  const P = (x: number, y: number): Point => turned([x, y], lean);
  const T = (x: number, y: number): string => f.p(...P(x, y));
  return [
    shape(f.band([P(0, 0), P(0, -20)], 1.4), "wood-700", 1, INKED),
    shape(
      `M${T(0.6, -21)}L${T(5.5, -24)}C${T(12, -20)} ${T(12, -15)} ${T(5.5, -11.5)}L${T(0.6, -14.5)}Z`,
      "slate-400",
      1,
      MAIN_INKED,
    ),
    shape(`M${T(5.5, -23.4)}C${T(11.2, -20)} ${T(11.2, -15.4)} ${T(5.5, -12.1)}L${T(5.8, -12.6)}C${T(11.8, -15.6)} ${T(11.8, -19.8)} ${T(5.8, -23)}Z`, "slate-100", 0.8),
    shape(f.band([P(0, -14.6), P(0, -21.2)], 1.8), "slate-700", 1, INKED),
    shape(seg(f, P(-0.9, -8), P(0.9, -8), 0.8), "slate-700", 1),
  ];
}

/** Six to eight swords in a spinning ring, points outward, a faint swirl, centred on the anchor, radius 12 at scale 1. */
function bladeRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 6, 8);
  const start = between(rng, 0, 360 / count);
  const out: KitShape[] = [
    shape(spiralArm(f, start, 150, 4, 11, 3), "sky-300", 0.3),
    shape(spiralArm(f, start + 180, 150, 4, 11, 3), "sky-300", 0.25),
  ];
  const sword: readonly Point[] = [
    [3.2, -0.5],
    [5, -0.5],
    [5, -2],
    [5.8, -2],
    [5.8, -0.9],
    [10.8, -0.9],
    [12, 0],
    [10.8, 0.9],
    [5.8, 0.9],
    [5.8, 2],
    [5, 2],
    [5, 0.5],
    [3.2, 0.5],
  ];
  for (let i = 0; i < count; i += 1) {
    out.push(shape(f.poly(sword.map((p) => turned(p, start + (i * 360) / count))), "slate-100", 1, MAIN_INKED));
  }
  return out;
}

/** An arrow flying the way it faces, shaft, head, fletching and a faint speed streak, centred on the anchor, 18 long at scale 1. */
function arrow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -8, 8);
  const P = (x: number, y: number): Point => turned([x, y], tilt);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => P(x, y)));
  return [
    shape(f.band([P(-10, -1.2), P(-15, -1.2)], 0.4), "white", 0.25),
    shape(f.band([P(-10, 1.2), P(-14, 1.2)], 0.4), "white", 0.2),
    shape(f.band([P(-9, 0), P(6.5, 0)], 0.8), "wood-600", 1, INKED),
    shape(
      poly([
        [5.8, -1.7],
        [9, 0],
        [5.8, 1.7],
        [6.6, 0],
      ]),
      "slate-400",
      1,
      INKED,
    ),
    shape(
      `${poly([
        [-9, 0],
        [-6.6, -2.4],
        [-4.2, -2.4],
        [-5.8, 0],
      ])}${poly([
        [-9, 0],
        [-6.6, 2.4],
        [-4.2, 2.4],
        [-5.8, 0],
      ])}`,
      "red-600",
      1,
      MAIN_INKED,
    ),
  ];
}

/** Two stone dumbbells lying on the ground, one cracked, on the anchor, 18 wide and 6 tall at scale 1. */
function weights(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const jig = between(rng, -0.5, 0.5);
  const dumbbell = (x0: number, x1: number, y: number): [string, string] => [
    seg(f, [x0, y - 1.5], [x1, y - 1.5], 1.1),
    `${box(f, x0 - 0.4, y - 3, x0 + 2.4, y)}${box(f, x1 - 2.4, y - 3, x1 + 0.4, y)}`,
  ];
  const [barA, headsA] = dumbbell(-9 + jig, 5 + jig, -3);
  const [barB, headsB] = dumbbell(-5, 9, 0);
  return [
    shape(barA, "stone-500", 1, INKED),
    shape(headsA, "stone-500", 1, MAIN_INKED),
    shape(
      f.band(
        [
          [-8.4, -5.8],
          [-7.4, -4.4],
          [-8.1, -3.8],
          [-7.2, -3],
        ],
        0.35,
      ),
      "slate-900",
      0.75,
    ),
    shape(barB, "stone-500", 1, INKED),
    shape(headsB, "stone-500", 1, MAIN_INKED),
    shape(`${seg(f, [-4.4, -2.7], [-2.4, -2.7], 0.4)}${seg(f, [7.4, -2.7], [9, -2.7], 0.4)}`, "white", 0.25),
  ];
}

/** A wooden rack holding three or four spears and a sword upright, on the anchor, 18 wide and 20 tall at scale 1. */
function weaponRack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spears = whole(rng, 3, 4);
  const xs = [-5.5, -2.3, 0.9, 4.1].slice(0, spears);
  const sword = xs.length < 4 ? 5.6 : 5.8;
  const lengths = xs.map(() => between(rng, 16.5, 19.4));
  const shafts = xs.map((x, i) => seg(f, [x, -1], [x, -(lengths[i] ?? 18)], 0.8)).join("");
  const heads = xs
    .map((x, i) => {
      const top = -(lengths[i] ?? 18);
      return f.poly([
        [x - 1, top],
        [x, top - 3],
        [x + 1, top],
      ]);
    })
    .join("");
  return [
    shape(shafts, "wood-600", 1, INKED),
    shape(heads, "slate-400", 1, INKED),
    shape(
      f.poly([
        [sword - 0.9, -11],
        [sword - 0.9, -18],
        [sword, -19.6],
        [sword + 0.9, -18],
        [sword + 0.9, -11],
      ]),
      "slate-100",
      1,
      INKED,
    ),
    shape(`${box(f, sword - 2, -11, sword + 2, -10)}${seg(f, [sword, -10], [sword, -7.6], 0.9)}`, "brass-600", 1, INKED),
    shape(`${box(f, -9, -6, 9, -4.8)}${box(f, -9, -12.4, 9, -11.2)}${box(f, -9, -1.2, 9, 0)}`, "wood-700", 1, MAIN_INKED),
    shape(`${seg(f, [-8, 0], [-8, -14], 1.6)}${seg(f, [8, 0], [8, -14], 1.6)}`, "wood-700", 1, MAIN_INKED),
  ];
}

/** A pair of iron brawler's knuckles, centred on the anchor, 8 wide and 4 tall at scale 1. */
function knuckles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 10);
  const bar = (cy: number): string => {
    const P = (x: number, y: number): Point => {
      const [tx, ty] = turned([x, y], tilt);
      return [tx, cy + ty];
    };
    let holes = "";
    for (let i = 0; i < 4; i += 1) holes += f.circle(...P(-2.7 + i * 1.8, 0), 0.5);
    return (
      f.poly([P(-3.9, -0.5), P(-3.2, -0.95), P(3.2, -0.95), P(3.9, -0.5), P(3.9, 0.5), P(3.2, 0.95), P(-3.2, 0.95), P(-3.9, 0.5)]) +
      holes
    );
  };
  return [
    shape(bar(-1.1), "slate-400", 1, MAIN_INKED),
    shape(bar(1.1), "slate-400", 1, MAIN_INKED),
    shape(seg(f, [-3.4, -1.9], [3.4, -1.9], 0.3), "white", 0.4),
  ];
}

// Potions

/** A round-bottomed glass flask with glowing liquid, on the anchor, 8 wide and 12 tall at scale 1. */
function flask(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const level = between(rng, -0.6, 0.6);
  return [
    shape(f.circle(0, -4.3, 5.8), "green-400", 0.14),
    shape(f.circle(0, -4.3, 4.6), "green-400", 0.2),
    shape(
      `M${f.p(-1.3, -12)}L${f.p(1.3, -12)}L${f.p(1.3, -8.6)}C${f.p(5.4, -7.6)} ${f.p(5.2, 0)} ${f.p(0, 0)}C${f.p(-5.2, 0)} ${f.p(-5.4, -7.6)} ${f.p(-1.3, -8.6)}Z`,
      "slate-100",
      0.3,
      INKED,
    ),
    shape(
      `M${f.p(-3.6, -4.6 + level)}Q${f.p(0, -5.8 + level)} ${f.p(3.6, -4.6 + level)}C${f.p(4, -1.6)} ${f.p(2.6, -0.3)} ${f.p(0, -0.3)}C${f.p(-2.6, -0.3)} ${f.p(-4, -1.6)} ${f.p(-3.6, -4.6 + level)}Z`,
      "green-400",
      0.95,
      MAIN,
    ),
    shape(box(f, -1.6, -13, 1.6, -11.6), "brass-600", 1, INKED),
    shape(seg(f, [-2.6, -6.4], [-2.9, -2.6], 0.7), "white", 0.6),
  ];
}

/** An iron pot on three short legs, its brew at the rim and a few bubbles, on the anchor, 18 wide and 12 tall at scale 1. */
function cauldron(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bubble = between(rng, 0.7, 1.1);
  return [
    shape(
      `${f.poly([
        [-7, -2.5],
        [-4.8, -2.5],
        [-5.2, 0],
        [-7.4, 0],
      ])}${f.poly([
        [4.8, -2.5],
        [7, -2.5],
        [7.4, 0],
        [5.2, 0],
      ])}${f.poly([
        [-1, -2],
        [1, -2],
        [1.2, 0],
        [-1.2, 0],
      ])}`,
      "slate-800",
      1,
      INKED,
    ),
    shape(`M${f.p(-8, -9)}L${f.p(8, -9)}C${f.p(10.4, -6.6)} ${f.p(8, -1)} ${f.p(0, -1)}C${f.p(-8, -1)} ${f.p(-10.4, -6.6)} ${f.p(-8, -9)}Z`, "slate-700", 1, INKED),
    shape(f.band([[-6.4, -6.6], [-5.4, -3.8], [-3, -2.2]], 0.7), "slate-400", 0.4),
    shape(f.ellipse(0, -9, 8.4, 1.7), "slate-800", 1, INKED),
    shape(f.ellipse(0, -9, 7.3, 1.1), "green-400", 1, MAIN),
    shape(f.circle(-3, -10.6, bubble) + f.circle(1.2, -11.4, bubble * 0.8) + f.circle(4, -10.4, bubble * 0.65), "green-200", 0.85),
  ];
}

// Locks and chains

/** An iron padlock, shackle and keyhole, bottom at the anchor, 5 wide and 7 tall at scale 1. */
function padlock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 0.9, 1);
  const s = fmt(f.r(1.9 * w));
  const s2 = fmt(f.r(1.1 * w));
  return [
    shape(
      `M${f.p(-1.9, -3.4)}L${f.p(-1.9, -5)}A${s} ${s} 0 0 ${f.sweep(1)} ${f.p(1.9, -5)}L${f.p(1.9, -3.4)}L${f.p(1.1, -3.4)}L${f.p(1.1, -5)}A${s2} ${s2} 0 0 ${f.sweep(0)} ${f.p(-1.1, -5)}L${f.p(-1.1, -3.4)}Z`,
      "slate-400",
      1,
      INKED,
    ),
    shape(box(f, -2.5, -3.6, 2.5, 0), "slate-700", 1, MAIN_INKED),
    shape(f.circle(0, -2.5, 0.55), "slate-900", 1),
    shape(
      f.poly([
        [-0.3, -2],
        [0.3, -2],
        [0.5, -0.7],
        [-0.5, -0.7],
      ]),
      "slate-900",
      1,
    ),
    shape(seg(f, [-1.9, -3.1], [1.9, -3.1], 0.35), "white", 0.2),
  ];
}

/** A sagging chain of six to nine links from the anchor to 22 to the right, sagging about 5, at scale 1. */
function chain(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const links = whole(rng, 6, 9);
  const sag = between(rng, 4.5, 5.5);
  const out: KitShape[] = [];
  for (let i = 0; i < links; i += 1) {
    const t = i / (links - 1);
    const cx = 22 * t;
    const cy = 4 * sag * t * (1 - t);
    const angle = Math.atan2(4 * sag * (1 - 2 * t), 22) / DEGREES;
    const flat = i % 2 === 0;
    const body = f.poly(capsule(1.5, flat ? 1 : 0.5, angle, cx, cy));
    const hole = flat ? f.poly(capsule(0.7, 0.4, angle, cx, cy)) : "";
    out.push(shape(body + hole, "slate-400", 1, MAIN_INKED));
  }
  return out;
}

/** A large old key lying across, bow ring and bit, centred on the anchor, 12 long at scale 1. */
function key(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -15, 15);
  const P = (x: number, y: number): Point => turned([x, y], tilt);
  const ring = f.circle(...P(-4, 0), 2.2) + f.circle(...P(-4, 0), 1.2);
  return [
    shape(ring, "slate-400", 1, MAIN_INKED),
    shape(
      f.poly([
        P(-2, -0.5),
        P(6, -0.5),
        P(6, 2.6),
        P(4.8, 2.6),
        P(4.8, 1.4),
        P(3.8, 1.4),
        P(3.8, 2.6),
        P(2.6, 2.6),
        P(2.6, 0.5),
        P(-2, 0.5),
      ]),
      "slate-400",
      1,
      MAIN_INKED,
    ),
    shape(f.band([P(-1, -0.15), P(2, -0.15)], 0.3), "white", 0.45),
  ];
}

// The dead and the humble

/** A wax candle with a flame and a small glow, on the anchor, 2 wide and 6 tall plus the flame at scale 1. */
function candle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flick = between(rng, -0.4, 0.4);
  return [
    shape(f.circle(flick * 0.5, -8.2, 4.8), "orange-400", 0.14),
    shape(f.circle(flick * 0.5, -8.2, 3), "orange-400", 0.22),
    shape(box(f, -1, -6, 1, 0), "slate-100", 1, MAIN_INKED),
    shape(seg(f, [0, -6], [0, -6.9], 0.3), "slate-900", 1),
    shape(`M${f.p(0, -6.4)}C${f.p(-1.3, -7.4)} ${f.p(-0.9, -8.8)} ${f.p(flick, -10.2)}C${f.p(0.9, -8.8)} ${f.p(1.3, -7.4)} ${f.p(0, -6.4)}Z`, "orange-400", 1),
    shape(`M${f.p(0, -6.8)}C${f.p(-0.6, -7.4)} ${f.p(-0.4, -8.2)} ${f.p(flick * 0.4, -8.8)}C${f.p(0.4, -8.2)} ${f.p(0.6, -7.4)} ${f.p(0, -6.8)}Z`, "white", 0.95),
  ];
}

/** A small heap of bones with one skull, on the anchor, 12 wide and 5 tall at scale 1. */
function bones(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, 1.8, 3);
  const sx = between(rng, -2, 0);
  return [
    shape(f.poly(bonePoly([-6, -0.9], [5.5, -0.8], 1)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(bonePoly([-4, -0.5], [6, -rise], 1)), "slate-100", 1, INKED),
    shape(f.circle(sx, -3.1, 2.1), "slate-100", 1, INKED),
    shape(box(f, sx - 1.2, -1.2, sx + 1.2, -0.1), "slate-100", 1, INKED),
    shape(f.circle(sx - 0.8, -3.3, 0.5) + f.circle(sx + 0.8, -3.3, 0.5), "slate-900", 1),
  ];
}

/** Five to eight small scattered stones about the anchor, 16 wide and 3 tall at scale 1. */
function pebbles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 5, 8);
  const out: KitShape[] = [];
  for (let i = 0; i < count; i += 1) {
    const cx = -7 + (14 * i) / (count - 1) + between(rng, -0.3, 0.3);
    const r = between(rng, 0.6, 1.2);
    const cy = -r * 0.75 - between(rng, 0, 1);
    out.push(shape(blob(f, rng, cx, cy, r, 6), i % 3 === 2 ? "grey-500" : "stone-500", 1, i % 3 === 2 ? INKED : MAIN_INKED));
  }
  return out;
}

/** A crusty loaf with a scored top, on the anchor, 8 wide and 4 tall at scale 1. */
function bread(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const score = between(rng, -0.2, 0.2);
  return [
    shape(`M${f.p(-4, 0)}C${f.p(-4.7, -2.8)} ${f.p(-2.5, -4)} ${f.p(0, -4)}C${f.p(2.5, -4)} ${f.p(4.7, -2.8)} ${f.p(4, 0)}Z`, "wood-600", 1, MAIN_INKED),
    shape(`M${f.p(-3, -2.4)}C${f.p(-2.6, -3.6)} ${f.p(2.6, -3.6)} ${f.p(3, -2.4)}C${f.p(1.6, -3.2)} ${f.p(-1.6, -3.2)} ${f.p(-3, -2.4)}Z`, "brass-500", 0.5),
    shape(
      seg(f, [-2.4, -3.3], [-1.6 + score, -1.5], 0.55) + seg(f, [-0.4, -3.7], [0.4 + score, -1.8], 0.55) + seg(f, [1.6, -3.4], [2.4 + score, -1.6], 0.55),
      "orange-400",
      0.7,
    ),
    shape(seg(f, [-3.6, -0.3], [3.6, -0.3], 0.6), "black", 0.2),
  ];
}

// Music and measure

/** A drum: a round war drum with laced hide and two straps, on the anchor, 12 wide and 12 tall at scale 1. */
function drum(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const zig = between(rng, 0, 0.6);
  return [
    shape(`${f.band([[-5, -10], [-6.2, -6], [-5.2, -2]], 0.9)}${f.band([[5, -10], [6.2, -6], [5.2, -2]], 0.9)}`, "wood-700", 1, INKED),
    shape(`M${f.p(-5.5, -10)}L${f.p(-5.5, -2)}Q${f.p(0, 1.6)} ${f.p(5.5, -2)}L${f.p(5.5, -10)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -10, 5.5, 1.7), "slate-100", 1, INKED),
    shape(
      f.band(
        [
          [-4.4, -8.4],
          [-3.2, -3.4 - zig],
          [-1.8, -8],
          [-0.6, -3 - zig],
          [0.6, -8],
          [1.8, -3 - zig],
          [3.2, -8.2],
          [4.4, -3.6],
        ],
        0.5,
      ),
      "slate-100",
      0.9,
    ),
    shape(seg(f, [-5.4, -8.6], [5.4, -8.6], 0.6), "wood-700", 0.9),
  ];
}

/** A wooden lyre with strings, on the anchor, 8 wide and 12 tall at scale 1. */
function lyre(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out = between(rng, 0, 0.6);
  return [
    shape(
      f.poly([
        [-2.8, -3.5],
        [2.8, -3.5],
        [2, 0],
        [-2, 0],
      ]),
      "wood-600",
      1,
      MAIN_INKED,
    ),
    shape(
      `${f.band([[-2.5, -3.2], [-4.2, -6], [-3.8, -9.5], [-2.6 - out, -11.8]], 0.9)}${f.band([[2.5, -3.2], [4.2, -6], [3.8, -9.5], [2.6 + out, -11.8]], 0.9)}`,
      "wood-600",
      1,
      MAIN_INKED,
    ),
    shape(seg(f, [-3.9, -10.3], [3.9, -10.3], 0.8), "wood-700", 1, INKED),
    shape(`${seg(f, [-1.6, -10], [-1.6, -3.6], 0.25)}${seg(f, [-0.55, -10], [-0.55, -3.6], 0.25)}${seg(f, [0.55, -10], [0.55, -3.6], 0.25)}${seg(f, [1.6, -10], [1.6, -3.6], 0.25)}`, "slate-100", 0.9),
  ];
}

/** A stone balance: a post, a beam tipped by a random angle and a pan hung from each end, on the anchor, 22 wide and 16 tall at scale 1. */
function scales(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 10);
  const pivot: Point = [0, -12.5];
  const end = (side: -1 | 1): Point => {
    const [x, y] = turned([side * 8, 0], tilt);
    return [pivot[0] + x, pivot[1] + y];
  };
  const left = end(-1);
  const right = end(1);
  const hang = (e: Point): [string, string] => [
    f.band([[e[0] - 3, e[1] + 4.5], e, [e[0] + 3, e[1] + 4.5]], 0.3),
    `M${f.p(e[0] - 3.6, e[1] + 4.5)}Q${f.p(e[0], e[1] + 7.5)} ${f.p(e[0] + 3.6, e[1] + 4.5)}Z`,
  ];
  const [chainL, panL] = hang(left);
  const [chainR, panR] = hang(right);
  return [
    shape(
      f.poly([
        [-3, 0],
        [3, 0],
        [2, -1.5],
        [-2, -1.5],
      ]),
      "stone-500",
      1,
      MAIN_INKED,
    ),
    shape(seg(f, [0, -1.5], [0, -12], 1.8), "stone-500", 1, MAIN_INKED),
    shape(chainL, "slate-400", 1),
    shape(chainR, "slate-400", 1),
    shape(panL, "brass-600", 1, INKED),
    shape(panR, "brass-600", 1, INKED),
    shape(f.band([left, right], 1), "stone-500", 1, MAIN_INKED),
    shape(f.circle(pivot[0], pivot[1] - 1, 1.2), "stone-500", 1, MAIN_INKED),
  ];
}

/** A bamboo chute sloping down to the right from (-14, -18) to (14, 0), on two stilts, its joints banded, 28 wide and 18 tall at scale 1. */
function chute(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, -0.6, 0.6);
  const top: Point = [-14, -18];
  const foot: Point = [14, 0 + sag * 0.3];
  const joints = [0.25, 0.5, 0.75].map((t): Point => [top[0] + (foot[0] - top[0]) * t, top[1] + (foot[1] - top[1]) * t]);
  return [
    shape(`${seg(f, [-9, -14.6], [-9, 0], 1.1)}${seg(f, [3, -6.9 + sag * 0.2], [3, 0], 1.1)}`, "wood-700", 1, INKED),
    shape(seg(f, top, foot, 3.2), "lime-400", 1, MAIN_INKED),
    shape(seg(f, offset(top, foot, 0, 0.9), offset(foot, top, 0, -0.9), 0.6), "green-700", 0.8),
    shape(joints.map((j) => seg(f, offset(j, foot, 0, -1.7), offset(j, foot, 0, 1.7), 0.5)).join(""), "green-700", 1),
  ];
}

// R3397000: the objects the Meditative+ and Training Grounds Alt art scenes added.

/** R3397000: a cork board hung flat, centred on the anchor: a timber frame round its cork (main), the cork's speckle and four to six round pins, red and blue; about 24 wide and 16 tall at scale 1. */
function corkboard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let specks = "";
  for (let i = 0; i < 5; i += 1) {
    for (let j = 0; j < 3; j += 1) specks += f.circle(-8.6 + i * 4.3 + between(rng, -1, 1), -4.4 + j * 4.4 + between(rng, -1, 1), 0.3);
  }
  const pins = whole(rng, 4, 6);
  let red = "";
  let blue = "";
  let glints = "";
  for (let k = 0; k < pins; k += 1) {
    const x = -9 + (18 * (k + 0.5)) / pins + between(rng, -0.6, 0.6);
    const y = k % 2 === 0 ? between(rng, -5.6, -4.4) : between(rng, 3.4, 5);
    if (k % 3 === 1) blue += f.circle(x, y, 0.55);
    else red += f.circle(x, y, 0.55);
    glints += f.circle(x - 0.18, y - 0.18, 0.16);
  }
  return [
    shape(box(f, -11.4, -7.2, 12.6, 8.8), "black", 0.2),
    shape(box(f, -12, -8, 12, 8) + box(f, -10.8, -6.8, 10.8, 6.8), "wood-700", 1, INKED),
    shape(box(f, -10.8, -6.8, 10.8, 6.8), "brass-600", 1, MAIN_INKED),
    shape(specks, "wood-600", 0.6),
    shape(red, "red-500", 1, INKED),
    shape(blue, "blue-600", 1, INKED),
    shape(glints, "white", 0.7),
  ];
}

/** R3397000: a washing line strung between two T-topped posts 28 apart about the anchor, sagging between them, two or three pegs clipped on it; about 30 wide and 16 tall at scale 1. */
function washingLine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 2, 4);
  const on = (t: number): Point => [-14 + 28 * t, -15 + 4 * sag * t * (1 - t)];
  const line: Point[] = [];
  for (let i = 0; i <= 12; i += 1) line.push(on(i / 12));
  const count = whole(rng, 2, 3);
  let pegs = "";
  for (let k = 0; k < count; k += 1) {
    const [px, py] = on((k + 1) / (count + 1) + between(rng, -0.04, 0.04));
    pegs += box(f, px - 0.3, py - 0.9, px + 0.3, py + 0.8);
  }
  return [
    shape(f.ellipse(-14, 0.2, 1.8, 0.4) + f.ellipse(14, 0.2, 1.8, 0.4), "black", 0.2),
    shape(seg(f, [-14, 0], [-14, -15.2], 0.9) + seg(f, [14, 0], [14, -15.2], 0.9), "wood-700", 1, MAIN_INKED),
    shape(seg(f, [-15.4, -15.8], [-12.6, -15.8], 0.6) + seg(f, [12.6, -15.8], [15.4, -15.8], 0.6), "wood-800", 1, INKED),
    shape(f.band(line, 0.28), "slate-700", 0.9),
    shape(pegs, "wood-600", 1, INKED),
  ];
}

/** R3397000: a checked picnic blanket spread on the ground in perspective, its near edge centred on the anchor: the cloth (main), its red checks, a hem and one corner folded back; about 30 wide and 9 deep at scale 1. */
function picnicBlanket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const skew = between(rng, -1.5, 1.5);
  const tilt = between(rng, -0.6, 0.6);
  const corners = { nl: [-15, tilt] as Point, nr: [15, -tilt] as Point, fl: [-11 + skew, -9 + tilt] as Point, fr: [11 + skew, -9 - tilt] as Point };
  const at2 = (u: number, v: number): Point => {
    const near: Point = [corners.nl[0] + (corners.nr[0] - corners.nl[0]) * u, corners.nl[1] + (corners.nr[1] - corners.nl[1]) * u];
    const far: Point = [corners.fl[0] + (corners.fr[0] - corners.fl[0]) * u, corners.fl[1] + (corners.fr[1] - corners.fl[1]) * u];
    return [near[0] + (far[0] - near[0]) * v, near[1] + (far[1] - near[1]) * v];
  };
  const cols = 6;
  const rows = 4;
  let checks = "";
  for (let i = 0; i < cols; i += 1) {
    for (let j = 0; j < rows; j += 1) {
      if ((i + j) % 2 !== 0) continue;
      checks += f.poly([at2(i / cols, j / rows), at2((i + 1) / cols, j / rows), at2((i + 1) / cols, (j + 1) / rows), at2(i / cols, (j + 1) / rows)]);
    }
  }
  const { nl, nr, fl, fr } = corners;
  return [
    shape(f.poly([[nl[0] + 0.4, nl[1] + 0.8], [nr[0] + 0.4, nr[1] + 0.8], [fr[0], fr[1] + 0.4], [fl[0], fl[1] + 0.4]]), "black", 0.2),
    shape(f.poly([nl, nr, fr, fl]), "white", 1, MAIN_INKED),
    shape(checks, "red-500", 0.9),
    shape(f.band([[nl[0] + 0.6, nl[1] - 0.5], [nr[0] - 0.6, nr[1] - 0.5]], 0.35), "red-600", 0.7),
    shape(f.poly([[nr[0], nr[1]], [nr[0] - 4.2, nr[1] - 0.2], [nr[0] - 1.6, nr[1] - 2.6]]), "white", 1, INKED),
  ];
}

/** R3397000: a rope hanging from the box's top down to the anchor, bowing a little as it takes the weight: its strand (main), a lit side and twists, and a knot with two loose ends at the anchor; about 3 wide at scale 1. */
function rope(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = Math.min(-2, (-2 - at.y) / at.s);
  const bow = between(rng, -1.2, 1.2);
  const along = (t: number): Point => [bow * Math.sin(Math.PI * t), top + (-1.4 - top) * t];
  const strand: Point[] = [];
  const lit: Point[] = [];
  for (let i = 0; i <= 10; i += 1) {
    const [x, y] = along(i / 10);
    strand.push([x, y]);
    lit.push([x - 0.3, y]);
  }
  const length = -1.4 - top;
  const marks = Math.max(1, Math.floor(length / 2.4));
  let twists = "";
  for (let k = 0; k < marks; k += 1) {
    const [x, y] = along((k + 0.5) / marks);
    twists += seg(f, [x - 0.5, y - 0.4], [x + 0.5, y + 0.4], 0.22);
  }
  return [
    shape(f.band(strand, 1.1), "wood-600", 1, MAIN_INKED),
    shape(f.band(lit, 0.25), "brass-500", 0.45),
    shape(twists, "wood-800", 0.7),
    shape(seg(f, [-0.3, 0.4], [-0.6, 2.6], 0.6) + seg(f, [0.4, 0.4], [0.8, 2.2], 0.6), "wood-600", 1, INKED),
    shape(f.ellipse(0, -0.6, 1.2, 1), "wood-700", 1, INKED),
  ];
}

// R3398000: the parts the Core Alt art scenes added.

/** R3398000: a round hay bale lying on the anchor, its curved side (main) and its rolled end facing the way it faces: straw strokes, the end's spiral and two twine bands; about 14 wide and 10 tall at scale 1. */
function hayBale(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = between(rng, 4.7, 5.1);
  let straw = "";
  for (let k = 0; k < 6; k += 1) {
    const x = -6 + k * 1.9 + between(rng, -0.05, 0.05);
    const y = -2 * r + 1.6 + between(rng, 0, 2 * r - 3.4);
    straw += seg(f, [x, y], [x + between(rng, 1, 1.6), y + between(rng, -0.4, 0.4)], 0.22);
  }
  let spiral = "";
  for (const k of [0.32, 0.58, 0.82]) spiral += `M${f.p(4.5 - r * k, -r)}A${fmt(f.r(r * k * 0.62))} ${fmt(f.r(r * k))} 0 0 ${f.sweep(1)} ${f.p(4.5, -r - r * k)}L${f.p(4.5, -r - r * k + 0.2)}A${fmt(f.r(r * k * 0.62 - 0.2))} ${fmt(f.r(r * k - 0.2))} 0 0 ${f.sweep(0)} ${f.p(4.5 - r * k + 0.2, -r)}Z`;
  return [
    shape(f.ellipse(-0.6, 0.2, 7.4, 0.7), "black", 0.22),
    shape(`M${f.p(-6.6, 0)}Q${f.p(-7.6, -r)} ${f.p(-6.6, -2 * r)}L${f.p(4.5, -2 * r)}L${f.p(4.5, 0)}Z`, "brass-500", 1, MAIN_INKED),
    shape(straw, "brass-600", 0.7),
    shape(f.ellipse(4.5, -r, r * 0.62, r), "brass-600", 1, INKED),
    shape(spiral, "wood-700", 0.45),
    shape(seg(f, [-3.4, -2 * r + 0.2], [-3.6, -0.2], 0.4) + seg(f, [0.8, -2 * r + 0.2], [0.6, -0.2], 0.4), "wood-800", 0.75),
  ];
}

/** R3398000: a gingham picnic blanket laid flat about the anchor and seen in perspective (main), its far edge narrower than its near one: check bands both ways, darker where they cross, and its near corner folded back; about 30 wide and 8 deep at scale 1. */
function ginghamBlanket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const skew = between(rng, -1, 1);
  const bands = whole(rng, 3, 4);
  const edge = (t: number, y: number): Point => {
    const half = 12 + (3 * (y + 4)) / 8;
    return [-half + 2 * half * t + skew * (4 - y) * 0.12, y];
  };
  let along = "";
  for (let k = 1; k <= bands; k += 1) along += f.band([edge(k / (bands + 1), -4), edge(k / (bands + 1), 4)], 1.3);
  let across = "";
  for (const y of [-1.4, 1.6]) across += f.band([edge(0, y), edge(1, y)], 1.2);
  const corner = edge(1, 4);
  return [
    shape(f.poly([edge(0, -4), edge(1, -4), edge(1, 4), edge(0, 4)]), "red-600", 1, MAIN_INKED),
    shape(along, "white", 0.45),
    shape(across, "white", 0.45),
    shape(f.poly([[corner[0] - 4.2, 4], [corner[0] - 1.1, 0.6], [corner[0] - 4.8, 0.9]]), "white", 0.5, INKED),
    shape(f.band([[corner[0] - 4.2, 4.2], [corner[0] + 0.4, 4.2]], 0.5), "black", 0.2),
  ];
}

/** R3398000: a rug laid flat about the anchor and seen in perspective (main), its far edge narrower than its near one: three to five stripes across it in two colours, a border and fringes at its two short ends; about 28 wide and 9 deep at scale 1. */
function rug(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stripes = whole(rng, 3, 5);
  const halfAt = (y: number): number => 12 + (2 * (y + 4.5)) / 9;
  const strip = (y0: number, y1: number): string => f.poly([[-halfAt(y0), y0], [halfAt(y0), y0], [halfAt(y1), y1], [-halfAt(y1), y1]]);
  let light = "";
  let dark = "";
  for (let k = 0; k < stripes; k += 1) {
    const y = -3.2 + (6.4 * (k + 0.5)) / stripes;
    const half = 0.32 + between(rng, 0, 0.18);
    if (k % 2 === 0) light += strip(y - half, y + half);
    else dark += strip(y - half, y + half);
  }
  let fringe = "";
  for (let y = -4.2; y <= 4.3; y += 1.05) fringe += seg(f, [-halfAt(y), y], [-halfAt(y) - 1.1, y + 0.15], 0.22) + seg(f, [halfAt(y), y], [halfAt(y) + 1.1, y + 0.15], 0.22);
  return [
    shape(strip(-4.5, 4.5), "blue-800", 1, MAIN_INKED),
    shape(strip(-4.5, -3.8) + strip(3.8, 4.5), "slate-900", 0.35),
    shape(light, "orange-400", 0.85),
    shape(dark, "white", 0.6),
    shape(fringe, "slate-100", 0.85),
  ];
}

/** R3398000: a hammock slung between two posts standing on the ground at 13 either side of the anchor: its posts, its ropes, its sagging cloth (main) with stripes along it, the shade of its underside and its shadow; about 26 wide and 10 tall at scale 1. */
function hammock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 3, 3.8);
  const top = -10 + between(rng, -0.6, 0.6);
  const hang = top + 2;
  const cloth = (lift: number, deep: number): string =>
    `M${f.p(-9, hang + lift)}Q${f.p(0, hang + lift + sag * 2)} ${f.p(9, hang + lift)}L${f.p(9, hang + lift + 0.6)}Q${f.p(0, hang + lift + sag * 2 + deep)} ${f.p(-9, hang + lift + 0.6)}Z`;
  return [
    shape(f.ellipse(0, 0.2, 10, 0.7), "black", 0.2),
    shape(seg(f, [-13, 0], [-13, top - 0.6], 0.9) + seg(f, [13, 0], [13, top - 0.6], 0.9), "wood-700", 1, INKED),
    shape(seg(f, [-13, top], [-9, hang], 0.25) + seg(f, [-13, top + 0.5], [-9, hang + 0.6], 0.25) + seg(f, [13, top], [9, hang], 0.25) + seg(f, [13, top + 0.5], [9, hang + 0.6], 0.25), "wood-800", 0.9),
    shape(cloth(0, 2.4), "red-600", 1, MAIN_INKED),
    shape(cloth(0.9, 0.3) + cloth(2, 0.3), "white", 0.5),
    shape(cloth(1.8, 0.8), "black", 0.18),
  ];
}

/** R3398000: a sleeping bag lying on the anchor, its padded length (main) quilted across, a zip along its top with its pull and a rolled hood at the end it faces away from, a dark mouth in the hood; about 22 long and 7 tall at scale 1. */
function sleepingBag(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const height = between(rng, 5.6, 6.6);
  let quilt = "";
  for (let x = -5; x < 9; x += 2.8) quilt += seg(f, [x, -height + 0.5], [x + 0.3, -0.4], 0.3);
  return [
    shape(f.ellipse(0, 0.2, 11.4, 0.7), "black", 0.22),
    shape(`M${f.p(-8, 0)}L${f.p(-8, -height)}L${f.p(9, -height + 0.6)}Q${f.p(11.4, -height * 0.5)} ${f.p(9, 0)}Z`, "blue-700", 1, MAIN_INKED),
    shape(quilt, "slate-900", 0.35),
    shape(seg(f, [-7.6, -height + 0.6], [8.6, -height + 1.1], 0.35), "slate-400", 0.9),
    shape(f.ellipse(between(rng, 3, 7), -height + 1.4, 0.45, 0.7), "slate-400", 0.9),
    shape(f.ellipse(-8.4, -height * 0.5, 3, height * 0.55), "blue-800", 1, INKED),
    shape(f.ellipse(-8.2, -height * 0.5, 1.6, height * 0.32), "slate-900", 0.85),
  ];
}

/** R3398000: a round teapot standing on the anchor, spout the way it faces: its body (main) and foot, a lid and its knob, a curved spout, a looped handle, a band round its belly, a shine and a translucent stream of tea pouring from the spout; about 12 wide and 10 tall at scale 1. */
function teapot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pour = between(rng, 0.8, 1.6);
  return [
    shape(f.ellipse(0, 0.2, 4.6, 0.5), "black", 0.2),
    shape(`M${f.p(-6.2, -6.2)}Q${f.p(-7.4, -4.2)} ${f.p(-3.4, -2.4)}L${f.p(-3.4, -3.4)}Q${f.p(-6, -4.4)} ${f.p(-5.2, -5.8)}Z`, "slate-400", 1, INKED),
    shape(`M${f.p(3.4, -3.6)}Q${f.p(5.6, -4)} ${f.p(7.2, -7.4)}L${f.p(7.8, -7.2)}Q${f.p(6.6, -2.8)} ${f.p(3.6, -1.8)}Z`, "slate-400", 1, INKED),
    shape(`M${f.p(-4.4, -4.2)}Q${f.p(-4.4, -8)} ${f.p(0, -8)}Q${f.p(4.4, -8)} ${f.p(4.4, -4.2)}Q${f.p(4.4, -0.6)} ${f.p(0, -0.6)}Q${f.p(-4.4, -0.6)} ${f.p(-4.4, -4.2)}Z`, "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-2.4, -0.8], [2.4, -0.8], [2, 0], [-2, 0]]), "slate-400", 1, INKED),
    shape(f.band([[-4.3, -4.4], [0, -3.4], [4.3, -4.4]], 0.6), "blue-600", 0.8),
    shape(`M${f.p(-2.6, -7.6)}Q${f.p(0, -9.4)} ${f.p(2.6, -7.6)}Z` + f.circle(0, -9.2, 0.6), "slate-400", 1, INKED),
    shape(f.ellipse(-2, -5.6, 0.9, 1.4), "white", 0.4),
    shape(f.band([[7.6, -7.2], [8.4 + pour * 0.5, -5], [8.6 + pour, 0]], 0.55), "orange-400", 0.5),
  ];
}

export const OBJECTS_KIT = {
  scroll: { category: "objects", draw: scroll },
  tome: { category: "objects", draw: tome },
  card: { category: "objects", draw: card },
  coins: { category: "objects", draw: coins },
  chest: { category: "objects", draw: chest },
  gems: { category: "objects", draw: gems },
  hourglass: { category: "objects", draw: hourglass },
  gear: { category: "objects", draw: gear },
  flask: { category: "objects", draw: flask },
  cauldron: { category: "objects", draw: cauldron },
  pipes: { category: "objects", draw: pipes },
  axe: { category: "objects", draw: axe },
  "blade-ring": { category: "objects", draw: bladeRing },
  arrow: { category: "objects", draw: arrow },
  spatula: { category: "objects", draw: spatula },
  bellows: { category: "objects", draw: bellows },
  weights: { category: "objects", draw: weights },
  "weapon-rack": { category: "objects", draw: weaponRack },
  "gift-box": { category: "objects", draw: giftBox },
  drum: { category: "objects", draw: drum },
  astrolabe: { category: "objects", draw: astrolabe },
  "slate-board": { category: "objects", draw: slateBoard },
  padlock: { category: "objects", draw: padlock },
  chain: { category: "objects", draw: chain },
  key: { category: "objects", draw: key },
  candle: { category: "objects", draw: candle },
  bones: { category: "objects", draw: bones },
  pebbles: { category: "objects", draw: pebbles },
  knuckles: { category: "objects", draw: knuckles },
  lyre: { category: "objects", draw: lyre },
  scales: { category: "objects", draw: scales },
  dice: { category: "objects", draw: dice },
  bread: { category: "objects", draw: bread },
  quill: { category: "objects", draw: quill },
  chute: { category: "objects", draw: chute },
  // R3397000: the Meditative+ and Training Grounds Alt art scenes' cork board, washing line, picnic blanket and rope.
  corkboard: { category: "objects", draw: corkboard },
  "washing-line": { category: "objects", draw: washingLine },
  "picnic-blanket": { category: "objects", draw: picnicBlanket },
  rope: { category: "objects", draw: rope },
  // R3398000: the Core Alt art scenes' hay bale, gingham blanket, rug, hammock, sleeping bag and pouring teapot.
  "hay-bale": { category: "objects", draw: hayBale },
  "gingham-blanket": { category: "objects", draw: ginghamBlanket },
  rug: { category: "objects", draw: rug },
  "beach-hammock": { category: "objects", draw: hammock },
  "sleeping-bag": { category: "objects", draw: sleepingBag },
  "pouring-teapot": { category: "objects", draw: teapot },
} as const satisfies Readonly<Record<string, KitPart>>;
