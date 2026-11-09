// R3401000: the scene kit's machines (ART.md, "Scene recipes and kits"), which the Meditative scenes (#3401)
// added: two Units drawn whole as living machines (a towering clockwork sentry, a small round robot sitting
// cross-legged) and the machines of the cards' settings (a gacha capsule machine, an office copier, a glowing
// terminal, a coin press, a mountain railway carriage, a motorbike, a desk of monitors, a barbell), and R3404000's
// tower crane, which Meditative's Alt art (#3404) added. Each stands on its anchor, its bottom centre there, but
// for two: the blueprint table, the centre of whose front edge is the anchor, and the barbell, centred on it at
// its bar. sceneKit.ts spreads them into `SCENE_KIT`. Every
// colour is a palette.css name, none in Radiant's gold window; a glow (a lens, a screen, a lit window) is a
// stack of translucent shapes, never a gradient; shapes are filled even-odd, so a hole is a second sub-path
// and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, DEGREES, FULL_TURN, INKED, local, MAIN, MAIN_INKED, shape, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";

// Helpers

/** A point `r` from (`cx`, `cy`) at `degrees`, clockwise from the right on screen. */
function polar(r: number, degrees: number, cx = 0, cy = 0): Point {
  return [cx + r * Math.cos(degrees * DEGREES), cy + r * Math.sin(degrees * DEGREES)];
}

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** A rectangle with corners rounded by `r`, through `p` (a part's `f.p`, or a turned one). */
function roundRect(p: (dx: number, dy: number) => string, left: number, top: number, right: number, bottom: number, r: number): string {
  return (
    `M${p(left + r, top)}L${p(right - r, top)}Q${p(right, top)} ${p(right, top + r)}L${p(right, bottom - r)}Q${p(right, bottom)} ${p(right - r, bottom)}` +
    `L${p(left + r, bottom)}Q${p(left, bottom)} ${p(left, bottom - r)}L${p(left, top + r)}Q${p(left, top)} ${p(left + r, top)}Z`
  );
}

/** A ring about (`dx`, `dy`), from `inner` out to `outer`. */
function ring(f: Local, dx: number, dy: number, outer: number, inner: number): string {
  return f.circle(dx, dy, outer) + f.circle(dx, dy, inner);
}

/** The top half of a disc about (`dx`, `dy`), its flat side down. */
function halfDisc(f: Local, dx: number, dy: number, r: number): string {
  const radius = fmt(f.r(r));
  return `M${f.p(dx - r, dy)}A${radius} ${radius} 0 0 ${f.sweep(1)} ${f.p(dx + r, dy)}Z`;
}

/** A gear's toothed rim about (`cx`, `cy`): `teeth` teeth out to `outer` from a root at `root`, the first at `start` degrees. */
function gearRim(f: Local, cx: number, cy: number, teeth: number, outer: number, root: number, start: number): string {
  const arc = FULL_TURN / teeth;
  const points: Point[] = [];
  for (let i = 0; i < teeth; i += 1) {
    for (const [da, r] of [
      [-0.3, root],
      [-0.16, outer],
      [0.16, outer],
      [0.3, root],
    ] as const) {
      points.push(polar(r, start + (i + da) * arc, cx, cy));
    }
  }
  return f.poly(points);
}

// Living machines

/** A towering gunmetal clockwork sentry standing on the anchor, its one eye the way it faces: blocky legs, a barrel body and a periscope head (main), a brass gear on the barrel, a hopper over its back shoulder and a periscope neck ending in a round amber lens that glows; about 20 wide and 44 tall at scale 1. */
function clockworkSentry(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = -31;
  const neck = between(rng, -38.4, -37.2);
  const tilt = between(rng, -8, 5);
  const hx = 2.6;
  const head = (dx: number, dy: number): Point => {
    const [x, y] = turned([dx, dy], tilt);
    return [hx + x, neck + y];
  };
  const [ex, ey] = head(3, -2.4);
  const leg = (cx: number): string =>
    f.poly([
      [cx - 2.6, 0],
      [cx + 3.4, 0],
      [cx + 3.4, -1.2],
      [cx + 2.2, -2.4],
      [cx - 2.6, -2.4],
    ]) +
    f.poly([
      [cx - 1.7, -2.4],
      [cx + 1.7, -2.4],
      [cx + 1.7, -7.4],
      [cx + 2.3, -7.4],
      [cx + 2.3, -10.2],
      [cx + 1.7, -10.2],
      [cx + 1.7, -15],
      [cx - 1.7, -15],
      [cx - 1.7, -10.2],
      [cx - 2.3, -10.2],
      [cx - 2.3, -7.4],
      [cx - 1.7, -7.4],
    ]);
  const hopper =
    f.poly([
      [-4, -24],
      [-9.8, -24],
      [-12.6, -35.4],
      [-2.4, -35.4],
    ]) + rect(f, -13.2, -36.6, -1.8, -35.4);
  const neckTube = rect(f, hx - 0.9, neck + 0.4, hx + 0.9, top - 1.2) + rect(f, hx - 2, top - 1.2, hx + 2, top);
  const barrel = `M${f.p(-8.5, -14)}Q${f.p(-11.2, -22.5)} ${f.p(-8.5, top)}L${f.p(8.5, top)}Q${f.p(11.2, -22.5)} ${f.p(8.5, -14)}Z`;
  const periscope = f.poly([head(-2.4, 0), head(5.8, 0), head(6.6, -1.2), head(6.6, -4.6), head(-1.6, -4.6), head(-2.4, -3.8)]);
  const hoop = (y: number): string =>
    f.band(
      [
        [-9.1, y],
        [0, y + 0.8],
        [9.1, y],
      ],
      1.1,
    );
  const rivets = [-6.4, -3.4, 3.4, 6.4].map((x) => f.circle(x, -16.1, 0.35) + f.circle(x, -27.9, 0.35)).join("");
  return [
    shape(leg(-4.8) + leg(4.4) + hopper + neckTube, "slate-800", 1, INKED),
    shape(barrel + periscope, "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(5.4, -14.3)}Q${f.p(8.4, -22.5)} ${f.p(5.4, top + 0.3)}L${f.p(8.5, top)}Q${f.p(11.2, -22.5)} ${f.p(8.5, -14)}Z`, "slate-900", 0.3),
    shape(hoop(-16.6) + hoop(-28.4) + rect(f, hx - 1.1, -35.2, hx + 1.1, -34.5), "slate-900", 0.5),
    shape(rivets, "slate-400", 1),
    shape(
      f.band(
        [
          [-7.2, -17.6],
          [-8.6, -22.5],
          [-7.2, -27.4],
        ],
        1.2,
      ),
      "slate-100",
      0.3,
    ),
    shape(gearRim(f, 0.6, -22.4, 8, 5.2, 4.2, between(rng, 0, 45)) + f.circle(0.6, -22.4, 1.6), "brass-600", 1, INKED),
    shape(f.circle(ex, ey, 5.2), "orange-400", 0.16),
    shape(f.circle(ex, ey, 3.3), "orange-400", 0.32),
    shape(f.circle(ex, ey, 1.9), "orange-400", 1, INKED),
    shape(f.circle(ex - 0.4, ey - 0.4, 0.75), "white", 0.85),
  ];
}

/** A small round robot sitting cross-legged on the anchor: folded legs, a dome body and a round head (main, with its legs and mitts), two closed-eye curves glowing jade on a dark face plate, a chest light and an antenna; about 12 wide and 12 tall at scale 1 (its antenna 1.5 more). */
function roundBot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hx = between(rng, -0.4, 0.4);
  const hy = -8.9;
  const [ax, ay] = [hx + between(rng, -0.9, 0.9), hy - 4.6];
  const legs = `M${f.p(-6, -1.3)}Q${f.p(-6.2, -3.6)} ${f.p(-3.4, -3.6)}L${f.p(3.4, -3.6)}Q${f.p(6.2, -3.6)} ${f.p(6, -1.3)}Q${f.p(5.8, 0)} ${f.p(4, 0)}L${f.p(-4, 0)}Q${f.p(-5.8, 0)} ${f.p(-6, -1.3)}Z`;
  const body = `M${f.p(-4.6, -2.8)}C${f.p(-4.9, -8.2)} ${f.p(4.9, -8.2)} ${f.p(4.6, -2.8)}Q${f.p(0, -2)} ${f.p(-4.6, -2.8)}Z`;
  const eye = (x: number): string =>
    f.band(
      [
        [x - 0.8, hy - 0.1],
        [x, hy + 0.45],
        [x + 0.8, hy - 0.1],
      ],
      0.42,
    );
  return [
    shape(legs, "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(-2, -0.95, 1.5, 0.75) + f.ellipse(2, -0.95, 1.5, 0.75), "slate-700", 0.85),
    shape(body, "slate-400", 1, MAIN_INKED),
    shape(`M${f.p(2.2, -2.3)}C${f.p(3.6, -4.2)} ${f.p(3.4, -6.4)} ${f.p(1.6, -7.2)}C${f.p(4.6, -7)} ${f.p(4.9, -4.8)} ${f.p(4.6, -2.8)}Z`, "slate-900", 0.2),
    shape(f.ellipse(-4.8, -3.4, 1.2, 1.35) + f.ellipse(4.8, -3.4, 1.2, 1.35), "slate-400", 1, MAIN_INKED),
    shape(
      f.band(
        [
          [hx, hy - 3],
          [ax, ay + 0.6],
        ],
        0.4,
      ),
      "slate-700",
      1,
    ),
    shape(f.circle(hx, hy, 3.2), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(hx, hy + 0.1, 2.3, 1.6), "slate-800", 1),
    shape(eye(hx - 1.05) + eye(hx + 1.05) + f.circle(ax, ay, 0.65) + f.circle(0, -4.7, 0.55), "jade-400", 1),
    shape(f.circle(ax, ay, 1.4) + f.circle(0, -4.7, 1.3), "jade-400", 0.25),
    shape(
      f.band(
        [
          [hx - 2.4, hy - 0.9],
          [hx - 1.7, hy - 2.3],
          [hx - 0.5, hy - 2.8],
        ],
        0.55,
      ),
      "white",
      0.45,
    ),
  ];
}

// Machines

/** Where a capsule machine's capsules sit in its globe, packed so none touches another. */
const CAPSULE_SPOTS: readonly Point[] = [
  [-1.1, -10.1],
  [1.1, -10.1],
  [-2.2, -12],
  [0, -12],
  [2.2, -12],
  [-3.3, -13.9],
  [-1.1, -13.9],
  [1.1, -13.9],
  [3.3, -13.9],
  [-2.2, -15.8],
  [0, -15.8],
  [2.2, -15.8],
];
const CAPSULE_PAINTS: readonly PaletteName[] = ["red-400", "sky-400", "lime-400", "violet-300", "orange-400"];

/** A gacha capsule machine standing on the anchor: a glass globe full of two-tone capsules in three colours, a red base (main) with a crank and a chute; about 10 wide and 18 tall at scale 1. */
function capsuleMachine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 1.04;
  const first = whole(rng, 0, CAPSULE_PAINTS.length - 1);
  const fills = ["", "", ""];
  let halves = "";
  const missing = whole(rng, 0, 2);
  CAPSULE_SPOTS.forEach(([x, y], k) => {
    if (k >= CAPSULE_SPOTS.length - missing && k !== CAPSULE_SPOTS.length - 2) return;
    const pick = whole(rng, 0, 2);
    fills[pick] += f.circle(x, y, r);
    halves += halfDisc(f, x, y, r);
  });
  const turn = between(rng, -40, 40);
  const handle = [turned([-1.4, 0], turn), turned([1.4, 0], turn)].map(([x, y]): Point => [x, y - 5.2]);
  const out = [shape(f.circle(0, -13.2, 4.6), "slate-100", 0.55, INKED)];
  fills.forEach((d, k) => {
    if (d !== "") out.push(shape(d, CAPSULE_PAINTS[(first + k) % CAPSULE_PAINTS.length] ?? "red-400", 1));
  });
  out.push(
    shape(halves, "white", 0.85),
    shape(
      f.band(
        [
          [-3.5, -14.4],
          [-2.9, -16.1],
          [-1.4, -17.1],
        ],
        0.6,
      ),
      "white",
      0.75,
    ),
    shape(rect(f, -4.4, -9.2, 4.4, -8) + rect(f, -1.5, -18.5, 1.5, -17.5), "slate-700", 1, INKED),
    shape(
      f.poly([
        [-4.8, 0],
        [4.8, 0],
        [4.1, -8],
        [-4.1, -8],
      ]),
      "red-600",
      1,
      MAIN_INKED,
    ),
    shape(
      f.poly([
        [2.8, 0],
        [4.8, 0],
        [4.1, -8],
        [2.6, -8],
      ]),
      "slate-900",
      0.22,
    ),
    shape(f.circle(0, -5.2, 1.7), "slate-400", 1, INKED),
    shape(f.band(handle, 0.7) + roundRect(f.p, -1.7, -2.9, 1.7, -0.8, 0.5), "slate-900", 1),
  );
  return out;
}

/** An office copier standing on the anchor: a boxy body (main) on castors, its lid lifted a crack on a glowing jade scan line, paper in a tray at one side and a sheet coming out of the slot at the other, drawers and a lit panel; about 19 wide and 14 tall at scale 1. */
function printer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gap = between(rng, 0.35, 0.7);
  const lid = -12 - gap;
  const body = f.poly([
    [-6.4, -0.8],
    [6.4, -0.8],
    [6.4, -7],
    [7, -7],
    [7, -12],
    [-7, -12],
    [-7, -7],
    [-6.4, -7],
  ]);
  const trays =
    f.poly([
      [-7, -8.6],
      [-9.6, -10.2],
      [-9.6, -9.5],
      [-7, -7.9],
    ]) +
    f.poly([
      [7, -7.6],
      [9.6, -6.8],
      [9.6, -6.2],
      [7, -7],
    ]) +
    f.poly([
      [-7.2, lid],
      [6.6, lid],
      [6.2, lid - 1.2],
      [-6.8, lid - 1.2],
    ]);
  const paper =
    f.poly([
      [-7, -8.8],
      [-9.3, -10.2],
      [-9.1, -10.9],
      [-7, -9.6],
    ]) +
    f.poly([
      [7, -7.8],
      [9.2 + between(rng, -0.6, 0.3), -7.1],
      [9.1, -7.6],
      [7, -8.4],
    ]);
  const seam = (y: number): string =>
    f.band(
      [
        [-6.2, y],
        [6.2, y],
      ],
      0.3,
    );
  return [
    shape(trays, "slate-700", 1, INKED),
    shape(body, "slate-400", 1, MAIN_INKED),
    shape(paper, "white", 1, INKED),
    shape(seam(-7.15) + seam(-4.6) + seam(-2.6), "slate-700", 0.6),
    shape(
      rect(f, -1.6, -3.9, 1.6, -3.4) + rect(f, -1.6, -1.9, 1.6, -1.4) + rect(f, 2.6, -10.8, 6.2, -8.2) + rect(f, -6, -0.8, -4.8, 0) + rect(f, 4.8, -0.8, 6, 0),
      "slate-800",
      1,
    ),
    shape(
      f.band(
        [
          [-6.8, -12 - gap / 2],
          [6.6, -12 - gap / 2],
        ],
        gap + 0.15,
      ) +
        rect(f, 3, -10.4, 4.7, -9.3) +
        f.circle(5.4, -9.85, 0.35),
      "jade-400",
      1,
    ),
    shape(f.ellipse(0, -12.2, 9, 1.9), "jade-400", 0.22),
    shape(rect(f, 4.6, -0.8, 6.4, -7) + rect(f, 5.2, -7, 7, -12), "slate-900", 0.2),
    shape(
      f.band(
        [
          [-6.6, -11.6],
          [6.6, -11.6],
        ],
        0.4,
      ),
      "white",
      0.35,
    ),
  ];
}

/** A sleek terminal standing on the anchor: a thin stand, a screen glowing cyan (main) with a few abstract lines, a halo of its light, and a small emitter on top casting a faint beam up; about 12 wide and 14.5 tall at scale 1. */
function terminal(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let lines = "";
  for (let k = 0; k < 4; k += 1) {
    const y = -10 + k * 1.5;
    if (k === 0 || rng() < 0.85)
      lines += f.band(
        [
          [-4.6, y],
          [-4.6 + between(rng, 2.6, 6.4), y],
        ],
        0.45,
      );
  }
  const icon = rng() < 0.5 ? ring(f, 3.4, -9, 1.2, 0.7) + rect(f, 2.4, -6.4, 4.4, -5) : rect(f, 2.4, -10.2, 4.4, -8.6) + ring(f, 3.4, -5.7, 1, 0.55);
  return [
    shape(roundRect(f.p, -7.8, -13, 7.8, -2, 2.4), "cyan-400", 0.12),
    shape(roundRect(f.p, -6.9, -12.1, 6.9, -2.9, 1.6), "cyan-400", 0.2),
    shape(
      f.poly([
        [-3.4, 0],
        [3.4, 0],
        [2.6, -0.9],
        [-2.6, -0.9],
      ]) + rect(f, -0.5, -3.8, 0.5, -0.9),
      "slate-800",
      1,
      INKED,
    ),
    shape(roundRect(f.p, -6, -11.2, 6, -3.8, 1), "slate-900", 1, INKED),
    shape(roundRect(f.p, -5.4, -10.6, 5.4, -4.4, 0.5), "cyan-400", 0.95, MAIN),
    shape(lines + icon, "white", 0.8),
    shape(
      f.poly([
        [-5.4, -10.6],
        [-1.6, -10.6],
        [-4.4, -4.4],
        [-5.4, -4.4],
      ]),
      "white",
      0.15,
    ),
    shape(halfDisc(f, 0, -11.2, 1.1), "slate-700", 1, INKED),
    shape(
      f.poly([
        [-0.35, -12.2],
        [0.35, -12.2],
        [2.4, -14.6],
        [-2.4, -14.6],
      ]),
      "cyan-400",
      0.28,
    ),
    shape(f.circle(0, -11.75, 0.4), "cyan-400", 1),
  ];
}

/** A clanking coin press standing on the anchor: a brass frame (main) with a screw and an iron hand wheel on top, a ram over a die block with a blank between, and a spout at one side dropping coins onto a little pile; about 19 wide and 19 tall at scale 1. */
function coinPress(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spin = between(rng, 0, 90);
  const [wy, wr, wd] = [-17.6, 4.6, 1.05];
  let threads = "";
  for (let y = -16.9; y < -9.6; y += 0.9) {
    if (y < -15.3 || y > -12.4)
      threads += f.band(
        [
          [-0.8, y + 0.3],
          [0.8, y - 0.3],
        ],
        0.3,
      );
  }
  let spokes = "";
  for (let k = 0; k < 4; k += 1) {
    const a = (spin + k * 90) * DEGREES;
    const [c, s] = [Math.cos(a), Math.sin(a)];
    spokes += f.band(
      [
        [1 * c, wy + 0.45 * s],
        [0.96 * wr * c, wy + 0.96 * wd * s],
      ],
      0.45,
    );
  }
  const frame =
    f.poly([
      [-8, 0],
      [8, 0],
      [8, -1.8],
      [6.4, -1.8],
      [6.4, -12.6],
      [7.2, -12.6],
      [7.2, -15],
      [-7.2, -15],
      [-7.2, -12.6],
      [-6.4, -12.6],
      [-6.4, -1.8],
      [-8, -1.8],
    ]) + rect(f, -4.4, -12.6, 4.4, -1.8);
  const out = [
    shape(frame, "brass-600", 1, MAIN_INKED),
    shape(rect(f, 5, -12.6, 6.4, -1.8) + rect(f, -7.2, -13.4, 7.2, -12.6) + rect(f, -8, -0.6, 8, 0), "brass-800", 0.45),
    shape(
      f.band(
        [
          [-5.9, -12.2],
          [-5.9, -2.2],
        ],
        0.5,
      ) +
        f.band(
          [
            [-6.9, -14.6],
            [6.9, -14.6],
          ],
          0.4,
        ),
      "white",
      0.25,
    ),
    shape(rect(f, -0.8, wy, 0.8, -9.4), "slate-400", 1, INKED),
    shape(threads, "slate-800", 0.6),
    shape(
      rect(f, -2.6, -6, 2.6, -1.8) +
        rect(f, -2.1, -9.4, 2.1, -7.6) +
        f.poly([
          [2.6, -5.6],
          [9.4, -4.2],
          [9.4, -3.2],
          [2.6, -4.4],
        ]),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.ellipse(0, wy, 5.4, 1.6) + f.ellipse(0, wy, wr, wd) + spokes + f.ellipse(0, wy, 0.9, 0.4), "slate-800", 1, INKED),
    shape(f.ellipse(0, -6.35, 1.6, 0.34) + f.ellipse(10.4, -2.5, 0.55, 0.8) + f.ellipse(10, -0.4, 1.4, 0.4) + f.ellipse(10.3, -1.22, 1.2, 0.38), "brass-500", 1),
  ];
  if (rng() < 0.7) {
    const mark = (side: 1 | -1, dy: number): string =>
      f.band(
        [
          [side * 2.5, -6.8 + dy],
          [side * 3.6, -7.3 + dy * 2],
        ],
        0.3,
      );
    out.push(shape(mark(1, 0) + mark(-1, 0) + mark(1, 0.9) + mark(-1, 0.9), "slate-900", 0.6));
  }
  return out;
}

/** A mountain railway carriage standing on its rail at the anchor: a short track of sleepers, four wheels, a long rounded carriage (main) under a roof, a row of six or seven lit windows and a door at each end; about 44 wide and 14.5 tall at scale 1. */
function train(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 6, 7);
  const gap = 1.3;
  const w = (31 - (count - 1) * gap) / count;
  let panes = "";
  let glows = "";
  for (let k = 0; k < count; k += 1) {
    const x = -15.5 + k * (w + gap);
    panes += rect(f, x, -11.6, x + w, -8.2);
    glows += rect(f, x - 0.5, -12.1, x + w + 0.5, -7.7);
  }
  let sleepers = "";
  for (let x = -21; x <= 21; x += 3) sleepers += rect(f, x - 0.6, -0.7, x + 0.6, 0);
  const wheels = [-14.6, -10.4, 10.4, 14.6];
  const body = `M${f.p(-19, -4.4)}L${f.p(19, -4.4)}L${f.p(19, -11.6)}Q${f.p(18.8, -13.2)} ${f.p(17, -13.4)}L${f.p(-17, -13.4)}Q${f.p(-18.8, -13.2)} ${f.p(-19, -11.6)}Z`;
  return [
    shape(sleepers, "wood-800", 0.9),
    shape(
      rect(f, -22, -1.2, 22, -0.4) +
        f.poly([
          [-17.4, -13.4],
          [17.4, -13.4],
          [16.4, -14.4],
          [-16.4, -14.4],
        ]),
      "slate-700",
      1,
      INKED,
    ),
    shape(wheels.map((x) => f.circle(x, -3.05, 1.85)).join(""), "slate-900", 1, INKED),
    shape(wheels.map((x) => f.circle(x, -3.05, 0.55)).join(""), "slate-400", 1),
    shape(rect(f, -18, -4.6, 18, -3.9), "slate-900", 0.9),
    shape(body, "blue-800", 1, MAIN_INKED),
    shape(rect(f, -19, -6, 19, -4.4), "slate-900", 0.25),
    shape(rect(f, -18.3, -11.6, -16.4, -5.1) + rect(f, 16.4, -11.6, 18.3, -5.1), "slate-900", 0.35),
    shape(
      f.band(
        [
          [-19, -6.8],
          [19, -6.8],
        ],
        0.7,
      ),
      "slate-100",
      0.55,
    ),
    shape(glows, "orange-400", 0.25),
    shape(panes, "orange-400", 1),
  ];
}

/** A classic motorbike standing on the anchor, its front the way it faces: two spoked wheels, a frame, an engine, chrome pipes and fork, a seat, a red tank and mudguards (main) and a headlamp; about 20 wide and 11 tall at scale 1. */
function motorbike(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const R = 3.3;
  const [rx, fx, wy] = [-6.6, 6.8, -R];
  const turn = between(rng, 0, 45);
  const bar = between(rng, -0.4, 0.4);
  const spokes = (cx: number): string => {
    let d = ring(f, cx, wy, 2.5, 2.15);
    for (let k = 0; k < 8; k += 1) d += f.band([polar(0.6, turn + k * 45, cx, wy), polar(2.1, turn + k * 45, cx, wy)], 0.22);
    return d;
  };
  const arc = (cx: number, from: number, to: number): string => {
    const points: Point[] = [];
    for (let k = 0; k <= 5; k += 1) points.push(polar(3.9, from + ((to - from) * k) / 5, cx, wy));
    return f.band(points, 0.9);
  };
  const tank = `M${f.p(-1.4, -7.1)}Q${f.p(-1.6, -9.7)} ${f.p(1.8, -9.9)}Q${f.p(4.6, -9.8)} ${f.p(4.9, -8.3)}Q${f.p(4, -7)} ${f.p(1.6, -6.9)}Z`;
  const seat = `M${f.p(-6.6, -7.4)}Q${f.p(-6.8, -8.7)} ${f.p(-5, -8.5)}L${f.p(-1.4, -8.1)}L${f.p(-1, -7.2)}L${f.p(-6, -7)}Z`;
  const frame =
    f.band(
      [
        [rx, wy],
        [-0.6, -4.4],
      ],
      0.9,
    ) +
    f.band(
      [
        [rx, wy],
        [-3.6, -7.2],
      ],
      0.8,
    ) +
    f.band(
      [
        [4.2, -8.2],
        [1.6, -4.6],
      ],
      1,
    ) +
    f.band(
      [
        [4.7, -9.4],
        [3.6, -10.5 + bar],
        [2.4, -10.6 + bar],
      ],
      0.6,
    );
  const chrome =
    f.band(
      [
        [fx, wy],
        [4.7, -9.4],
      ],
      0.9,
    ) +
    f.band(
      [
        [2.4, -3.8],
        [1.4, -2.2],
        [-2.4, -1.9],
        [-5.6, -2.3],
      ],
      1.1,
    ) +
    f.poly([
      [-5.75, -2.95],
      [-10, -3.9],
      [-10.1, -2.7],
      [-5.75, -1.65],
    ]);
  const fin = (y: number): string =>
    f.band(
      [
        [-1.2, y],
        [3, y],
      ],
      0.3,
    );
  return [
    shape(ring(f, rx, wy, R, 2.5) + ring(f, fx, wy, R, 2.5), "slate-900", 1, INKED),
    shape(spokes(rx) + spokes(fx), "slate-400", 1),
    shape(frame, "slate-800", 1),
    shape(
      f.poly([
        [-1.8, -7.1],
        [3.2, -7.1],
        [3.4, -4.4],
        [2, -3.4],
        [-1.4, -3.6],
      ]),
      "slate-700",
      1,
      INKED,
    ),
    shape(fin(-6.1) + fin(-5.2) + f.circle(rx, wy, 0.6) + f.circle(fx, wy, 0.6), "slate-900", 0.8),
    shape(chrome, "slate-400", 1),
    shape(
      f.band(
        [
          [1.6, -2.55],
          [-2.4, -2.3],
          [-5.4, -2.7],
        ],
        0.3,
      ) +
        f.band(
          [
            [-6, -2.75],
            [-9.8, -3.6],
          ],
          0.3,
        ),
      "white",
      0.5,
    ),
    shape(seat, "slate-900", 1, INKED),
    shape(tank + arc(rx, 190, 300) + arc(fx, 220, 335), "red-600", 1, MAIN_INKED),
    shape(
      f.band(
        [
          [-0.4, -8.6],
          [1.4, -9.3],
          [3, -9.2],
        ],
        0.45,
      ),
      "white",
      0.4,
    ),
    shape(f.circle(5.8, -8.5, 0.85), "slate-100", 1, INKED),
  ];
}

/** A row of three to five glowing computer screens on a desk, standing on the anchor: dark bezels on little stands, faces glowing cyan (main) and violet by turns with abstract lines on them, their light spilling on the desk; about 30 wide and 10 tall at scale 1. */
function monitors(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 5);
  const gap = 1;
  const w = (28 - (count - 1) * gap) / count;
  const cyanFirst = rng() < 0.5;
  const bottom = -3.2;
  let bezels = "";
  let stands = "";
  let lines = "";
  const faces = { cyan: "", violet: "" };
  const glows = { cyan: f.ellipse(0, -1.4, 13, 1.05), violet: "" };
  for (let k = 0; k < count; k += 1) {
    const x0 = -14 + k * (w + gap);
    const x1 = x0 + w;
    const cx = (x0 + x1) / 2;
    const top = bottom - 6 - (k === Math.floor(count / 2) ? 0.6 : 0) - between(rng, -0.3, 0.3);
    const tone = (k % 2 === 0) === cyanFirst ? "cyan" : "violet";
    bezels += rect(f, x0, top, x1, bottom);
    faces[tone] += rect(f, x0 + 0.5, top + 0.5, x1 - 0.5, bottom - 0.6);
    glows[tone] += rect(f, x0 - 0.45, top - 0.45, x1 + 0.45, bottom + 0.45);
    stands += rect(f, cx - 0.35, bottom, cx + 0.35, -1.9) + rect(f, cx - 1.3, -1.9, cx + 1.3, -1.4);
    const rows = whole(rng, 2, 3);
    for (let r = 0; r < rows; r += 1) {
      const y = top + 1.4 + r * 1.3;
      lines += f.band(
        [
          [x0 + 0.9, y],
          [x0 + 0.9 + between(rng, 0.35, 0.85) * (w - 1.8), y],
        ],
        0.35,
      );
    }
  }
  return [
    shape(glows.cyan, "cyan-400", 0.18),
    shape(glows.violet, "violet-500", 0.18),
    shape(stands, "slate-700", 1),
    shape(rect(f, -15, -1.4, 15, 0), "slate-800", 1, INKED),
    shape(bezels, "slate-900", 1, INKED),
    shape(faces.cyan, "cyan-400", 1, MAIN),
    shape(faces.violet, "violet-500", 1),
    shape(lines, "white", 0.7),
    shape(
      f.band(
        [
          [-14.6, -1.15],
          [14.6, -1.15],
        ],
        0.3,
      ),
      "white",
      0.25,
    ),
  ];
}

/** A vast blueprint table seen from above and in front, the centre of its front edge on the anchor: a wooden top in perspective, a blue sheet (main) pinned at its corners with a cyan grid and cyan plan outlines (a building, a round works, a small block, a dimension line), often a set square lying on it; about 60 wide and 18 deep at scale 1 (its edge 1.8 more below). */
function blueprint(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const b = 0.25;
  const skew = between(rng, -0.03, 0.03);
  const side = rng() < 0.5 ? 1 : -1;
  const plane = (u: number, z: number): Point => [(u * 30) / (1 + b * z), (-18 * z * (1 + b)) / (1 + b * z)];
  const S = (u: number, z: number): Point => plane(u + skew * (z - 0.5) * 2, z);
  const quad = (u0: number, z0: number, u1: number, z1: number): string => f.poly([S(u0, z0), S(u1, z0), S(u1, z1), S(u0, z1)]);
  const [eu, ez] = [0.008, 0.014];
  const outline = (u0: number, z0: number, u1: number, z1: number): string => quad(u0 - eu, z0 - ez, u1 + eu, z1 + ez) + quad(u0 + eu, z0 + ez, u1 - eu, z1 - ez);
  const [u0, z0, u1, z1] = [-0.9, 0.07, 0.9, 0.93];
  let across = "";
  for (let i = 1; i < 10; i += 1) across += f.band([S(u0 + ((u1 - u0) * i) / 10, z0), S(u0 + ((u1 - u0) * i) / 10, z1)], 0.2);
  let deep = "";
  for (let j = 1; j < 6; j += 1) deep += f.band([S(u0, z0 + ((z1 - z0) * j) / 6), S(u1, z0 + ((z1 - z0) * j) / 6)], 0.2);
  const [cu, cz, ru, rz] = [0.42 * side, 0.52, 0.2, 0.24];
  const circle = (scale: number): string => {
    const points: Point[] = [];
    for (let k = 0; k < 16; k += 1) points.push(S(cu + ru * scale * Math.cos((k * FULL_TURN * DEGREES) / 16), cz + rz * scale * Math.sin((k * FULL_TURN * DEGREES) / 16)));
    return f.poly(points);
  };
  const [ha, hb] = side === 1 ? [-0.72, -0.2] : [0.2, 0.72];
  const wall = -0.45 * side;
  const plans =
    outline(ha, 0.25, hb, 0.78) +
    f.band([S(wall, 0.25 + ez), S(wall, 0.58)], 0.45) +
    circle(1.06) +
    circle(0.94) +
    f.poly([S(cu - 0.035, cz - 0.05), S(cu + 0.035, cz - 0.05), S(cu + 0.035, cz + 0.05), S(cu - 0.035, cz + 0.05)]) +
    outline(-0.08 * side - 0.12, 0.12, -0.08 * side + 0.12, 0.3) +
    f.band([S(ha, 0.14), S(hb, 0.14)], 0.4);
  const pins = [S(u0 + 0.04, z0 + 0.03), S(u1 - 0.04, z0 + 0.03), S(u0 + 0.04, z1 - 0.03), S(u1 - 0.04, z1 - 0.03)].map(([x, y]) => f.circle(x, y, 0.65)).join("");
  const out = [
    shape(
      f.poly([
        [-30, 0],
        [30, 0],
        [30, 1.8],
        [-30, 1.8],
      ]),
      "wood-800",
      1,
      INKED,
    ),
    shape(f.poly([plane(-1, 0), plane(1, 0), plane(1, 1), plane(-1, 1)]), "wood-700", 1, INKED),
    shape(quad(u0, z0, u1, z1), "blue-800", 1, MAIN_INKED),
    shape(across, "cyan-400", 0.3),
    shape(deep, "cyan-400", 0.3),
    shape(plans, "cyan-400", 0.9),
    shape(pins, "red-600", 1),
    shape(
      f.band(
        [
          [-29.6, 0.35],
          [29.6, 0.35],
        ],
        0.4,
      ),
      "white",
      0.25,
    ),
  ];
  if (rng() < 0.6) {
    const [su, sz] = [between(rng, -0.25, 0.05) * side, between(rng, 0.62, 0.7)];
    const corners: Point[] = [
      [su, sz],
      [su + 0.26 * side, sz],
      [su, sz + 0.24],
    ];
    const [mu, mz] = [su + (0.26 * side) / 3, sz + 0.08];
    const inner = corners.map(([u, z]): Point => [mu + (u - mu) * 0.45, mz + (z - mz) * 0.45]);
    out.push(shape(f.poly(corners.map(([u, z]) => S(u, z))) + f.poly(inner.map(([u, z]) => S(u, z))), "white", 0.35));
  }
  return out;
}

/** A barbell centred on the anchor at its bar, tilted a hair: a thin bar, three or four plates (main) at each end, bigger nearer the middle, held by collars; about 32 wide and 13 tall at scale 1. */
function barbell(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -2, 2);
  const T = (dx: number, dy: number): string => f.p(...turned([dx, dy], tilt));
  const poly = (points: readonly Point[]): string => `M${points.map(([x, y]) => T(x, y)).join("L")}Z`;
  const band = (points: readonly Point[], width: number): string =>
    f.band(
      points.map((point) => turned(point, tilt)),
      width,
    );
  const count = whole(rng, 3, 4);
  const sizes: readonly Point[] = [
    [1.7, 6.4],
    [1.4, 5.2],
    [1.2, 4.1],
    [1, 3.2],
  ];
  let plates = "";
  let lights = "";
  let x = 8.4;
  for (let k = 0; k < count; k += 1) {
    const [w, h] = sizes[k] ?? [1, 3];
    for (const s of [1, -1]) {
      const [l, r] = s === 1 ? [x, x + w] : [-x - w, -x];
      plates += roundRect(T, l, -h, r, h, 0.5);
      lights += band(
        [
          [l + 0.45, -h + 0.8],
          [l + 0.45, h - 0.8],
        ],
        0.35,
      );
    }
    x += w + 0.25;
  }
  const collar = (from: number, w: number, h: number): string => roundRect(T, from, -h, from + w, h, 0.25) + roundRect(T, -from - w, -h, -from, h, 0.25);
  return [
    shape(
      band(
        [
          [-7.2, 0],
          [7.2, 0],
        ],
        0.9,
      ) +
        poly([
          [7.2, -0.7],
          [16, -0.7],
          [16, 0.7],
          [7.2, 0.7],
        ]) +
        poly([
          [-16, -0.7],
          [-7.2, -0.7],
          [-7.2, 0.7],
          [-16, 0.7],
        ]),
      "slate-400",
      1,
    ),
    shape(
      band(
        [
          [-15.8, 0.45],
          [15.8, 0.45],
        ],
        0.3,
      ),
      "slate-900",
      0.4,
    ),
    shape(
      band(
        [
          [-15.6, -0.3],
          [15.6, -0.3],
        ],
        0.25,
      ),
      "white",
      0.5,
    ),
    shape(plates, "slate-700", 1, MAIN_INKED),
    shape(lights, "slate-400", 0.45),
    shape(collar(7.2, 1, 1.5) + collar(x - 0.1, 0.9, 1.3), "slate-400", 1, INKED),
  ];
}

/** R3404000: a tower crane standing on the anchor: a lattice mast (main, orange-400) on a concrete slab, its operator's cab at the top with a lit window, a long jib out the way it faces and a short counter-jib behind with its block of weights, tie bars from a peak above the mast to both ends, a trolley partway along the jib and a hook on its line hanging below; about 38 wide and 44 tall at scale 1. */
function towerCrane(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const [top, jib] = [-37.6, 28];
  const trolley = between(rng, 13, 24);
  const hook = between(rng, -28, -14);
  let lattice = "";
  for (let y = -1.2; y > top + 2; y -= 3.2) lattice += f.band([[-1.1, y], [1.1, y - 3.2]], 0.28) + f.band([[1.1, y], [-1.1, y - 3.2]], 0.28);
  for (let x = 3; x < jib - 1; x += 3) lattice += f.band([[x, top + 1.3], [x + 1.5, top + 0.15]], 0.24) + f.band([[x + 1.5, top + 0.15], [x + 3, top + 1.3]], 0.24);
  const mast = rect(f, -1.3, top, 1.3, -1) + rect(f, -9, top - 1.4, jib, top + 1.4);
  return [
    shape(rect(f, -4, -1, 4, 0), "slate-400", 1, INKED),
    shape(mast, "orange-400", 1, MAIN_INKED),
    shape(lattice, "slate-900", 0.5),
    shape(f.band([[-9, top - 1.4], [0, top - 7], [jib, top - 1.4]], 0.3) + f.band([[0, top - 7], [0, top - 1.4]], 0.4), "slate-800", 1),
    shape(rect(f, -8.6, top + 1.4, -4.8, top + 5), "slate-800", 1, INKED),
    shape(rect(f, 1.3, top + 1.4, 4.6, top + 4.8), "slate-700", 1, INKED),
    shape(rect(f, 2, top + 2, 4, top + 3.6), "blue-200", 0.7),
    shape(rect(f, trolley - 1.2, top + 1.4, trolley + 1.2, top + 2.4) + f.band([[trolley, top + 2.4], [trolley, hook - 1]], 0.2), "slate-800", 1),
    shape(rect(f, trolley - 0.9, hook - 1.4, trolley + 0.9, hook) + f.band([[trolley, hook], [trolley, hook + 1.4], [trolley - 0.8, hook + 1.9], [trolley - 1.2, hook + 1.1]], 0.36), "slate-900", 1),
  ];
}

/** R3401000: the machines the Meditative scenes added, by id, each with its kind. R3404000: the tower crane the Alt art briefs added after them. */
export const INK_MACHINES_KIT = {
  "clockwork-sentry": { category: "objects", draw: clockworkSentry },
  "round-bot": { category: "objects", draw: roundBot },
  "capsule-machine": { category: "objects", draw: capsuleMachine },
  printer: { category: "objects", draw: printer },
  terminal: { category: "objects", draw: terminal },
  "coin-press": { category: "objects", draw: coinPress },
  train: { category: "objects", draw: train },
  motorbike: { category: "objects", draw: motorbike },
  monitors: { category: "objects", draw: monitors },
  blueprint: { category: "objects", draw: blueprint },
  barbell: { category: "objects", draw: barbell },
  "tower-crane": { category: "objects", draw: towerCrane },
} as const satisfies Readonly<Record<string, KitPart>>;
