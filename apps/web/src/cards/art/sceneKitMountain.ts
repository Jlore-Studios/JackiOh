// R3393000: the scene kit's parts that the Meditative+ scenes (#3393) added, drawn in the Meditative family's ink
// wash (ART.md's art bible): layered `peaks` in mist across the box and the one red `seal` every Meditative+ scene
// stamps in its near layer; the mountain places (a round window, a pavilion, an amphitheatre's stands, terraces, a
// tall waterfall `cascade` and a glowing `camp-tent`, apart from #3396's `waterfall` and `tent`); its plants (an
// ink pine, a plum branch, a crimson bloom, a giant pumpkin, a beanstalk, a cart-sized elderberry, a mossy
// boulder); and the briefs' props (a red-ink brush stroke, an ink whip, a summit
// flag, a vending machine, folded towels, a star map, a giant stapler, spice sacks, rising steam, a burst pouch, a
// mud-caked glove, a stone chime, a ladder, a snowplow, sound ripples, a block of line-work, a cursor of light, a
// card's ink frame and a floating sheet of paper). sceneKit.ts spreads the table into `SCENE_KIT`. Each part keeps
// the kit's rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream
// alone, every colour a palette name out of Radiant's gold window, a glow a stack of translucent shapes, and no
// two sub-paths of one shape overlapping but for a hole.

import { fmt } from "./hash.ts";
import { between, EDGE, FAR_EDGE, INKED, local, MAIN, MAIN_INKED, pt, shape, sparkle, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** A closed outline smoothed through the midpoints of `points`, its ends joined straight: a soft ink edge. */
function smooth(f: Local, points: readonly Point[]): string {
  const at = (i: number): Point => points[i] ?? [0, 0];
  const mid = (a: Point, b: Point): string => f.p((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
  let d = `M${f.p(...at(0))}L${mid(at(0), at(1))}`;
  for (let i = 1; i < points.length - 1; i += 1) d += `Q${f.p(...at(i))} ${mid(at(i), at(i + 1))}`;
  return `${d}L${f.p(...at(points.length - 1))}Z`;
}

/** The box as a frame of its own, for terrain drawn in the box's coordinates. */
const BOX_FRAME = local({ x: 0, y: 0, s: 1, flip: 1 });

// Terrain

/**
 * An ink-wash range across the box: three ridges of steep rounded peaks, each fainter the farther it stands, mist
 * lying between them; the front ridge's peaks rise up to about 22 above the placement's y (the back one's to 40),
 * every body running down past the box's bottom. The front ridge is `main`.
 */
function peaks(at: PartPlace, rng: Rng): KitShape[] {
  const ridge = (base: number, high: number, count: number): Point[] => {
    const step = (FAR_EDGE - EDGE) / count;
    const points: Point[] = [[EDGE, FAR_EDGE], [EDGE, base - high * between(rng, 0.1, 0.3) * at.s]];
    for (let k = 0; k < count; k += 1) {
      const x = EDGE + step * (k + between(rng, 0.35, 0.65));
      points.push([x - step * 0.22, base - high * between(rng, 0.4, 0.6) * at.s]);
      points.push([x, base - high * between(rng, 0.75, 1) * at.s]);
      points.push([x + step * 0.26, base - high * between(rng, 0.1, 0.35) * at.s]);
    }
    points.push([FAR_EDGE, base - high * between(rng, 0.1, 0.3) * at.s], [FAR_EDGE, FAR_EDGE]);
    return points;
  };
  const mistBand = (y: number, thick: number): string =>
    `M${pt(EDGE, y)}Q${pt(25, y - thick * between(rng, 0.6, 1.4))} ${pt(50, y - thick * 0.3)}Q${pt(75, y - thick * between(rng, 0.6, 1.4))} ${pt(FAR_EDGE, y)}` +
    `L${pt(FAR_EDGE, y + thick)}Q${pt(50, y + thick * 1.6)} ${pt(EDGE, y + thick)}Z`;
  const back = ridge(at.y - 12 * at.s, 40, whole(rng, 4, 5));
  const middle = ridge(at.y - 6 * at.s, 31, whole(rng, 3, 4));
  const front = ridge(at.y, 22, whole(rng, 2, 3));
  // Each front peak (every third point from the third) is lit on its right slope and brushed down its left, both
  // kept under the smoothed outline: its crown stands a quarter of the way down from the peak's control point.
  let lit = "";
  let strokes = "";
  for (let i = 3; i < front.length - 2; i += 3) {
    const [px, py] = front[i] ?? [0, 0];
    const [vx, vy] = front[i + 1] ?? [px, py];
    const [sx, sy] = front[i - 1] ?? [px, py];
    const ax = 0.75 * px + 0.125 * (sx + vx);
    const ay = 0.75 * py + 0.125 * (sy + vy);
    const [mx, my] = [(px + vx) / 2, (py + vy) / 2];
    const [nx, ny] = [(px + sx) / 2, (py + sy) / 2];
    lit += BOX_FRAME.poly([[ax, ay + 0.8], [mx - 0.5, my + 0.3], [ax + (mx - ax) * 0.3, vy]]);
    strokes += BOX_FRAME.band([[ax - 0.4, ay + 1.6], [nx + 0.8, ny + 1.2], [nx + 1.6, ny + 6 * at.s]], 0.7 * at.s);
  }
  return [
    shape(smooth(BOX_FRAME, back), "slate-400", 0.35),
    shape(mistBand(at.y - 10 * at.s, 3 * at.s), "white", 0.45),
    shape(smooth(BOX_FRAME, middle), "slate-700", 0.5),
    shape(mistBand(at.y - 4 * at.s, 2.6 * at.s), "white", 0.4),
    shape(smooth(BOX_FRAME, front), "slate-800", 0.92, MAIN),
    shape(lit, "slate-100", 0.16),
    shape(strokes, "slate-900", 0.4),
    shape(mistBand(at.y + 2 * at.s, 3 * at.s), "white", 0.35),
  ];
}

// Objects

/** The red seal of an ink painting, centred on the anchor: a square chop with a carved border and three to five carved bars, never a letter; about 5.2 across at scale 1. */
function seal(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -4, 4);
  const square = (left: number, top: number, right: number, bottom: number): string =>
    f.poly([turned([left, top], tilt), turned([right, top], tilt), turned([right, bottom], tilt), turned([left, bottom], tilt)]);
  let marks = "";
  for (const y of [-1.3, -0.1, 1.1]) {
    if (rng() < 0.85) marks += square(-1.7, y - 0.2, -1.7 + between(rng, 0.7, 1.4), y + 0.2);
  }
  if (rng() < 0.5) {
    marks += square(0.35, -1.6, 0.75, 1.6);
    marks += square(1.05, -0.4 + between(rng, -0.8, 0.8), 1.7, 0.1 + between(rng, -0.8, 0.8));
  } else {
    marks += square(0.35, -1.6, 1.7, 0.1) + square(0.75, -1.2, 1.3, -0.3);
    marks += square(0.35, 0.5, 1.7, 0.9);
  }
  return [
    shape(square(-2.6, -2.6, 2.6, 2.6), "red-600", 0.92, MAIN),
    shape(square(-2.25, -2.25, 2.25, 2.25) + square(-1.95, -1.95, 1.95, 1.95), "slate-100", 0.55),
    shape(marks, "slate-100", 0.6),
  ];
}

/** A calligraphic red-ink stroke rising from the anchor to an arrowhead about 30 above it (a plan's route), swelling and drying as a brush does, a splatter beside it. */
function brushStroke(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -4, 4);
  const tip: Point = [between(rng, 1, 4), -30];
  const along = (t: number): Point => [2 * (1 - t) * t * bend + t * t * tip[0], -30 * t * t - 30 * (1 - t) * t];
  const left: Point[] = [];
  const right: Point[] = [];
  const core: Point[] = [];
  for (let i = 0; i <= 8; i += 1) {
    const t = i / 8;
    const [x, y] = along(t);
    const half = 0.35 + 0.75 * Math.max(0, 1 - Math.abs(t - 0.35) * 1.3);
    left.push([x - half, y]);
    right.push([x + half, y]);
    core.push([x + half * 0.2, y]);
  }
  const [tx, ty] = tip;
  return [
    shape(f.band(core, 3.4), "red-600", 0.18, MAIN),
    shape(f.poly([...left, ...right.reverse()]), "red-600", 0.95, MAIN),
    shape(f.poly([[tx - 2.6, ty + 3.4], [tx, ty - 1.8], [tx + 2.6, ty + 3.4], [tx, ty + 2.2]]), "red-600", 0.95, MAIN),
    shape(f.band(core.slice(2, 6), 0.35), "white", 0.3),
    shape(f.circle(bend * 0.4 + 2.6, -9, 0.45) + f.circle(bend * 0.6 - 2.2, -16, 0.35) + f.circle(tx + 1.6, ty + 7, 0.3), "red-600", 0.7),
  ];
}

/** A ghostly ink whip cracking across the anchor, from a short handle 24 to its left to its cracking tip 24 to its right: a tapering S of ink in a pale glow, a spark where it cracks. */
function inkWhip(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wave = between(rng, 3, 6);
  const core: Point[] = [];
  const left: Point[] = [];
  const right: Point[] = [];
  for (let i = 0; i <= 10; i += 1) {
    const t = i / 10;
    const x = -20 + 44 * t;
    const y = -Math.sin(t * Math.PI * 1.6) * wave * (0.4 + t);
    const half = 0.9 * (1 - t) + 0.12;
    core.push([x, y]);
    left.push([x, y - half]);
    right.push([x, y + half]);
  }
  const [ex, ey] = core[core.length - 1] ?? [24, 0];
  return [
    shape(f.band(core, 4), "violet-300", 0.22),
    shape(f.band(core, 1.8), "violet-300", 0.35),
    shape(f.poly([...left, ...right.reverse()]), "slate-900", 0.9, MAIN),
    shape(f.band([[-25, 1.6], [-20.4, 0]], 1.4), "wood-800", 1, INKED),
    shape(sparkle(f, ex + 1.4, ey, 3.2), "white", 0.9),
    shape(f.circle(ex + 1.4, ey, 2.6), "violet-300", 0.3),
  ];
}

/** A summit flag on a pole planted in a little cairn on the anchor, its pennant streaming the way the part faces; about 7 wide and 12 tall at scale 1. */
function flag(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -1, 1);
  return [
    shape(f.ellipse(-1.2, -0.5, 1.4, 0.8) + f.ellipse(1.3, -0.4, 1.2, 0.7) + f.ellipse(0.1, -1.5, 1, 0.6), "slate-400", 1, INKED),
    shape(f.band([[0, -1.8], [0, -12]], 0.5), "wood-800", 1, INKED),
    shape(`M${f.p(0.3, -11.8)}Q${f.p(3, -12.6 + sway)} ${f.p(6.4, -10.6 + sway)}Q${f.p(3.2, -9.6)} ${f.p(0.3, -8.4)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.circle(0, -12.3, 0.45), "slate-100", 1),
  ];
}

/** A camp tent glowing from inside, its door flap tied back, guy lines pegged out and snow banked at its foot, on the anchor; about 12 wide and 9 tall at scale 1. */
function campTent(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 5.4, 6.6);
  const h = between(rng, 8, 9.4);
  return [
    shape(f.circle(0, -h * 0.45, h * 0.95), "orange-400", 0.16),
    shape(f.poly([[0, -h], [w, 0], [w + 3.4, -0.6], [3.2, -h - 0.3]]), "ember-500", 0.9, INKED),
    shape(f.poly([[-w, 0], [0, -h], [w, 0]]), "orange-400", 1, MAIN_INKED),
    shape(f.poly([[-1.9, 0], [0, -h * 0.58], [1.9, 0]]), "white", 0.85),
    shape(f.poly([[0, -h * 0.58], [1.9, 0], [3.2, 0], [0.5, -h * 0.66]]), "red-300", 0.9),
    shape(f.band([[-0.2, -h + 0.4], [-w - 2.6, 0.2]], 0.18) + f.band([[3, -h], [w + 5.2, 0]], 0.18), "slate-900", 0.6),
    shape(f.ellipse(0.8, 0.3, w + 3, 0.9), "white", 0.8),
  ];
}

/** A lonely vending machine standing on the anchor, its glass front lit over rows of snacks, a coin slot and a tray; about 8 wide and 15 tall at scale 1. */
function vendingMachine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rows: [string, string, string] = ["", "", ""];
  for (let row = 0; row < 4; row += 1) {
    for (let col = 0; col < 3; col += 1) {
      const x = -2.9 + col * 1.4;
      const y = -12.8 + row * 2.2;
      const k = row === 0 ? col : whole(rng, 0, 2);
      rows[k] += rect(f, x, y, x + 1, y + 1.3);
    }
  }
  return [
    shape(f.circle(0, -8, 10), "sky-300", 0.12),
    shape(rect(f, -4, -15, 4, 0), "blue-700", 1, MAIN_INKED),
    shape(rect(f, -3.4, -14, 1.3, -3.8), "sky-300", 0.4),
    shape(rows[0], "red-500", 0.95),
    shape(rows[1], "orange-400", 0.95),
    shape(rows[2], "green-400", 0.95),
    shape(rect(f, 1.9, -13.4, 3.4, -5.6), "slate-900", 0.9),
    shape(rect(f, 2.4, -11, 2.9, -9.2) + rect(f, 2.2, -7.8, 3.1, -7), "white", 0.8),
    shape(rect(f, -3.2, -2.8, 1.1, -1.2), "black", 0.75),
  ];
}

/** A stack of three folded towels on the anchor, something round and ticking tucked under the bottom one at its side; about 12 wide and 6 tall at scale 1. */
function towels(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const shift = between(rng, -0.5, 0.5);
  const fold = (left: number, top: number, right: number, bottom: number): string =>
    `M${f.p(left + 0.6, top)}L${f.p(right - 0.6, top)}Q${f.p(right, top)} ${f.p(right, top + 0.6)}L${f.p(right, bottom)}L${f.p(left, bottom)}L${f.p(left, top + 0.6)}Q${f.p(left, top)} ${f.p(left + 0.6, top)}Z`;
  return [
    shape(f.circle(5.6, -1.2, 1.4), "slate-400", 1, INKED),
    shape(f.circle(5.6, -1.2, 1), "white", 0.95),
    shape(f.band([[5.6, -1.9], [5.6, -1.2], [6.2, -0.9]], 0.25), "black", 0.8),
    shape(fold(-5, -2, 5, 0) + fold(-4.2 + shift, -6, 4.2 + shift, -4), "white", 1, MAIN_INKED),
    shape(fold(-4.6 - shift, -4, 4.4 - shift, -2), "jade-400", 1, INKED),
    shape(f.band([[-4.4, -1], [4.4, -1]], 0.25) + f.band([[-3.6 + shift, -5], [3.6 + shift, -5]], 0.25), "slate-400", 0.5),
    shape(f.band([[7.5, -2.6], [8.1, -1.2], [7.5, 0.2]], 0.2) + f.band([[8.6, -3.2], [9.4, -1.2], [8.6, 0.8]], 0.2), "slate-900", 0.45),
  ];
}

/** A hand-drawn star map lying unrolled on the anchor: stars, a route of nodes across it and one edge circled twice in red; about 25 wide and 11 deep at scale 1. */
function starMap(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const corner: Point = [-12, 0];
  const across: Point = [23, -1.2];
  const up: Point = [1.4, -10];
  const on = (u: number, v: number): Point => [corner[0] + across[0] * u + up[0] * v, corner[1] + across[1] * u + up[1] * v];
  let stars = "";
  for (let k = 0; k < 9; k += 1) stars += f.circle(...on(between(rng, 0.08, 0.92), between(rng, 0.1, 0.9)), between(rng, 0.15, 0.3));
  const route: Point[] = [];
  for (let k = 0; k < 5; k += 1) route.push(on(0.12 + k * 0.17, 0.3 + 0.4 * rng()));
  let nodes = "";
  for (const [x, y] of route) nodes += f.circle(x, y, 0.6);
  const [rx, ry] = on(0.88, 0.62);
  return [
    shape(f.poly([on(0, 0), on(1, 0), on(1, 1), on(0, 1)]), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(...on(0, 0.5), 1.1, 5.2) + f.ellipse(...on(1, 0.5), 1.1, 5.2), "stone-500", 1, INKED),
    shape(stars, "slate-700", 0.8),
    shape(f.band(route, 0.3), "blue-800", 0.75),
    shape(nodes, "blue-800", 0.95),
    shape(f.circle(rx, ry, 2.2) + f.circle(rx, ry, 1.9) + f.circle(rx, ry, 1.5) + f.circle(rx, ry, 1.2), "red-600", 0.9),
  ];
}

/** A giant desk stapler on the anchor, its arm hinged up at the back and its jaw over the plate, a staple bright where it bites; about 26 wide and 12 tall at scale 1. */
function stapler(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, 8, 18);
  const hinge: Point = [-11, -3];
  const arm = (dx: number, dy: number): Point => {
    const [x, y] = turned([dx, dy], -lift);
    return [hinge[0] + x, hinge[1] + y];
  };
  const [jx, jy] = arm(23, 0.6);
  return [
    shape(`M${f.p(-12.4, 0)}L${f.p(12, 0)}Q${f.p(13, 0)} ${f.p(13, -1.2)}L${f.p(13, -2.4)}L${f.p(-12.4, -2.4)}Z`, "slate-700", 1, INKED),
    shape(rect(f, 6, -3.2, 12.4, -2.4), "slate-400", 1),
    shape(f.poly([arm(-1.4, 1), arm(23.4, 1.2), arm(24, -1.4), arm(21, -3.4), arm(-1.4, -3.6)]), "red-600", 1, MAIN_INKED),
    shape(f.band([arm(1, -2.6), arm(19, -2.6)], 0.5), "white", 0.3),
    shape(f.circle(...hinge, 1.5), "slate-400", 1, INKED),
    shape(sparkle(f, jx - 0.6, (jy - 2.4) / 2, 2.6), "white", 0.85),
    shape(f.band([[jx - 2.2, -2.8], [jx - 2.2, -3.6], [jx + 0.4, -3.6], [jx + 0.4, -2.8]], 0.3), "slate-100", 0.9),
  ];
}

/** A round window in a wall, centred on the anchor: a wooden frame and sill, its muntins, and snowy peaks in the pale sky beyond; about 18 across at scale 1. */
function roundWindow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 7.8;
  const edge = Math.sqrt(r * r - 4);
  const tops: Point[] = [[-edge, 2]];
  const count = whole(rng, 2, 3);
  for (let k = 0; k < count; k += 1) {
    const x = -edge + ((2 * edge) / count) * (k + 0.5) + between(rng, -1, 1);
    tops.push([x - 1.6, between(rng, -1, 0.5)], [x, between(rng, -5.6, -3.4)], [x + 1.8, between(rng, 0, 1.4)]);
  }
  tops.push([edge, 2]);
  let caps = "";
  for (let i = 2; i < tops.length - 1; i += 3) {
    const [px, py] = tops[i] ?? [0, 0];
    caps += f.poly([[px, py], [px + 1, py + 1.6], [px, py + 1.1], [px - 0.9, py + 1.5]]);
  }
  return [
    shape(f.circle(0, 0, r), "blue-200", 1),
    shape(`M${tops.map(([x, y]) => f.p(x, y)).join("L")}A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(-edge, 2)}Z`, "slate-400", 0.9),
    shape(caps, "white", 0.9),
    shape(rect(f, -0.45, -r, 0.45, r) + rect(f, -r, -0.45, -0.45, 0.45) + rect(f, 0.45, -0.45, r, 0.45), "wood-800", 0.9),
    shape(f.circle(0, 0, 9) + f.circle(0, 0, r), "wood-700", 1, MAIN_INKED),
    shape(rect(f, -6.6, 8.8, 6.6, 10.2), "wood-800", 1, INKED),
  ];
}

// Plants

/** A gnarled ink pine standing on the anchor, its crooked trunk holding four flat pads of needles out over the drop; about 16 wide and 22 tall at scale 1. */
function pine(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.6, 0.6);
  const trunk: Point[] = [[0, 0], [1.2 + j(), -5], [-0.8 + j(), -10], [1.4 + j(), -15], [0.4, -19.5]];
  const pads: [number, number, number][] = [
    [-6 + j(), -9 + j(), between(rng, 3.8, 4.8)],
    [5.4 + j(), -13 + j(), between(rng, 3.6, 4.6)],
    [-3.6 + j(), -16.6 + j(), between(rng, 3.2, 4)],
    [2.2 + j(), -21 + j(), between(rng, 2.8, 3.6)],
  ];
  const at0 = (i: number): Point => trunk[i] ?? [0, 0];
  let boughs = "";
  let padPaths = "";
  let lit = "";
  let needles = "";
  pads.forEach(([x, y, rx], k) => {
    const [bx, by] = at0(k + 1);
    boughs += f.band([[bx, by], [(bx + x) / 2, by - 0.6], [x, y + 0.4]], 0.7);
    padPaths += f.ellipse(x, y, rx, 1.5);
    lit += f.ellipse(x - rx * 0.2, y - 0.6, rx * 0.6, 0.55);
    needles += f.band([[x - rx * 0.6, y + 1.3], [x + rx * 0.5, y + 1.2]], 0.3);
  });
  return [
    shape(f.band(trunk, 1.5), "wood-800", 1, INKED),
    shape(boughs, "wood-800", 1),
    shape(padPaths, "green-900", 1, MAIN_INKED),
    shape(lit, "jade-400", 0.5),
    shape(needles, "slate-900", 0.4),
  ];
}

/** A wobbly lump about (cx, cy), `rx` by `ry`, smoothed through the midpoints of its jittered rim: a stone, a moss cap. */
function lump(f: Local, rng: Rng, cx: number, cy: number, rx: number, ry: number, points: number): string {
  const rim: Point[] = [];
  for (let i = 0; i < points; i += 1) {
    const angle = (i * 2 * Math.PI) / points;
    const reach = between(rng, 0.85, 1.1);
    rim.push([cx + rx * reach * Math.cos(angle), cy + ry * reach * Math.sin(angle)]);
  }
  const at = (i: number): Point => rim[i % points] ?? [cx, cy];
  const mid = (a: Point, b: Point): string => f.p((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
  let d = `M${mid(at(0), at(1))}`;
  for (let i = 1; i <= points; i += 1) d += `Q${f.p(...at(i))} ${mid(at(i), at(i + 1))}`;
  return `${d}Z`;
}

/** A crimson flower burst open on the snow about 1.6 above the anchor, the drop of blood it grew from splashed round its foot, a red glow about it; about 9 across at scale 1. */
function bloom(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const petals = whole(rng, 5, 7);
  const turn = between(rng, 0, 1);
  const flower = (r: number, offset: number): string => {
    let d = "";
    for (let i = 0; i <= petals; i += 1) {
      const a = ((i + offset + turn) * 2 * Math.PI) / petals;
      const c = ((i + offset + turn + 0.5) * 2 * Math.PI) / petals;
      const valley = f.p(r * 0.4 * Math.cos(a), -1.6 + r * 0.4 * Math.sin(a));
      if (i === 0) d += `M${valley}`;
      else d += `Q${f.p(r * 1.45 * Math.cos(c - (2 * Math.PI) / petals), -1.6 + r * 1.45 * Math.sin(c - (2 * Math.PI) / petals))} ${valley}`;
    }
    return `${d}Z`;
  };
  const r = between(rng, 3.4, 4.2);
  return [
    shape(f.circle(0, -1.6, r * 1.9), "red-400", 0.18),
    shape(f.ellipse(-4.4, 1, 0.9, 0.35) + f.ellipse(3.8, 1.3, 0.7, 0.3) + f.ellipse(5.2, 0.3, 0.4, 0.2), "red-900", 0.75),
    shape(flower(r, 0), "red-600", 1, MAIN_INKED),
    shape(flower(r * 0.55, 0.5), "red-500", 1),
    shape(f.circle(0, -1.6, 0.9), "red-900", 1),
    shape(f.circle(0.5, -2.4, 0.25) + f.circle(-0.6, -2.2, 0.22) + f.circle(0.2, -0.9, 0.22), "white", 0.85),
  ];
}

/** A bare plum branch reaching from the anchor 26 the way it faces, its crooked twigs dotted with pale blossoms and red buds. */
function plumBranch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.8, 0.8);
  const bough: Point[] = [[0, 0], [6, -2 + j()], [12, -1 + j()], [19, -4 + j()], [26, -3 + j()]];
  const node = (i: number): Point => bough[i] ?? [0, 0];
  const twigs = f.band([node(1), [8.4, -7.2]], 0.5) + f.band([node(2), [15.4, 2.6]], 0.45) + f.band([node(3), [22.4, -9]], 0.45);
  const slots: Point[] = [[8.4, -7.2], [13, -1.8 + node(2)[1] + 1], [15.4, 2.6], [22.4, -9], [20, -5.6], [26.4, node(4)[1] - 0.2], [4, -2]];
  let blossoms = "";
  let hearts = "";
  let buds = "";
  slots.forEach(([x, y], k) => {
    const bx = x + j() * 0.35;
    const by = y + j() * 0.35;
    if (k % 3 === 2 && rng() < 0.5) buds += f.circle(bx, by, 0.45);
    else {
      blossoms += f.circle(bx, by, 0.95);
      hearts += f.circle(bx, by, 0.32);
    }
  });
  return [
    shape(f.band(bough, 1.2), "slate-900", 1, INKED),
    shape(twigs, "slate-900", 0.95),
    shape(blossoms, "red-300", 1, MAIN),
    shape(hearts, "white", 0.85),
    shape(buds === "" ? f.circle(node(4)[0] - 2, node(4)[1] + 0.9, 0.4) : buds, "red-600", 0.9),
  ];
}

/** A pavilion standing on the anchor: a stone platform, four red lacquered pillars, a beam and an upswept tiled roof; about 32 wide and 22 tall at scale 1. */
function pavilion(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, -1, 1);
  const flare = between(rng, -0.6, 0.8);
  let pillars = "";
  for (const x of [-10, -4, 4, 10]) pillars += rect(f, x - 0.6, -13, x + 0.6, -2);
  return [
    shape(rect(f, -9.4, -13, 9.4, -2), "slate-900", 0.35),
    shape(rect(f, -12.4, -2, 12.4, 0), "stone-500", 1, INKED),
    shape(pillars, "red-600", 1, MAIN_INKED),
    shape(rect(f, -12, -14.4, 12, -13), "wood-800", 1, INKED),
    shape(
      `M${f.p(-16, -16.6 - flare)}Q${f.p(-13, -14.2)} ${f.p(-10, -14.2)}L${f.p(10, -14.2)}Q${f.p(13, -14.2)} ${f.p(16, -16.6 - flare)}` +
        `Q${f.p(10, -17.4)} ${f.p(7, -21 + rise)}L${f.p(-7, -21 + rise)}Q${f.p(-10, -17.4)} ${f.p(-16, -16.6 - flare)}Z`,
      "slate-800",
      1,
      INKED,
    ),
    shape(f.band([[-13, -14.9], [13, -14.9]], 0.5), "slate-400", 0.4),
    shape(f.band([[-7.4, -21.2 + rise], [7.4, -21.2 + rise]], 0.8) + f.circle(0, -22.6 + rise, 0.8), "slate-900", 1),
    shape(f.band([[-7, -14.2], [-7, -11.8]], 0.2) + f.band([[7, -14.2], [7, -11.8]], 0.2), "slate-900", 0.7),
    shape(f.ellipse(-7, -10.8, 0.9, 1.2) + f.ellipse(7, -10.8, 0.9, 1.2), "orange-400", 0.95),
  ];
}

/** A tiered grandstand standing on the anchor, its four rows of stone seats stepping up toward the back (its left as drawn facing right), heads along each row; about 22 wide and 11 tall at scale 1. */
function stands(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const step = 2.6;
  const outline: Point[] = [[-12, 0], [10, 0]];
  for (let k = 0; k < 4; k += 1) outline.push([10 - k * step, -(k + 1) * step], [10 - (k + 1) * step, -(k + 1) * step]);
  outline.pop();
  outline.push([-12, -4 * step]);
  let tops = "";
  let heads: [string, string] = ["", ""];
  for (let k = 0; k < 4; k += 1) {
    const y = -(k + 1) * step;
    const right = 10 - k * step;
    tops += rect(f, -12, y, right, y + 0.7);
    for (let x = -11; x < right - 0.8; x += between(rng, 1.6, 2.4)) {
      if (rng() < 0.2) continue;
      const which = rng() < 0.5 ? 0 : 1;
      heads[which] += f.circle(x, y - 0.75, 0.55);
    }
  }
  heads = [heads[0] === "" ? f.circle(-10, -1.4, 0.5) : heads[0], heads[1] === "" ? f.circle(-8, -1.4, 0.5) : heads[1]];
  return [
    shape(f.poly(outline), "slate-700", 1, INKED),
    shape(tops, "stone-500", 1, MAIN),
    shape(heads[0], "red-300", 0.95),
    shape(heads[1], "slate-900", 0.85),
  ];
}

/** Sound rippling out both ways from the anchor: three flat pairs of arcs fading as they widen, the outer about 40 wide, a hum at their heart. */
function ripples(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const arc = (r: number, from: number, to: number): Point[] => {
    const points: Point[] = [];
    for (let i = 0; i <= 6; i += 1) {
      const a = ((from + ((to - from) * i) / 6) * Math.PI) / 180;
      points.push([r * Math.cos(a), r * 0.45 * Math.sin(a)]);
    }
    return points;
  };
  const out: KitShape[] = [shape(f.circle(0, 0, 3), "violet-300", 0.3, MAIN)];
  for (let k = 1; k <= 3; k += 1) {
    const r = 5 + k * 5 + between(rng, -0.8, 0.8);
    out.push(shape(f.band(arc(r, -40, 40), 0.7) + f.band(arc(r, 140, 220), 0.7), "violet-300", 0.75 - k * 0.17, MAIN));
  }
  return out;
}

/** Three sacks of spice on the anchor, their necks rolled down over heaps of red, orange and green; about 17 wide and 9 tall at scale 1. */
function sacks(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sack = (cx: number, w: number, h: number): string =>
    `M${f.p(cx - w * 0.8, 0)}Q${f.p(cx - w * 1.15, -h * 0.5)} ${f.p(cx - w, -h)}L${f.p(cx + w, -h)}Q${f.p(cx + w * 1.15, -h * 0.5)} ${f.p(cx + w * 0.8, 0)}Z`;
  const heap = (cx: number, w: number, h: number): string => `M${f.p(cx - w * 0.9, -h)}Q${f.p(cx, -h - between(rng, 1.6, 2.6))} ${f.p(cx + w * 0.9, -h)}Z`;
  const rim = (cx: number, w: number, h: number): string => f.band([[cx - w * 1.02, -h + 0.4], [cx + w * 1.02, -h + 0.4]], 0.9);
  return [
    shape(sack(0, 3, 8.2), "wood-600", 1, INKED),
    shape(heap(0, 3, 8.2), "orange-400", 1),
    shape(sack(-5.2, 3, 6) + sack(5.4, 2.8, 5.6), "stone-500", 1, MAIN_INKED),
    shape(heap(-5.2, 3, 6), "red-600", 1),
    shape(heap(5.4, 2.8, 5.6), "green-700", 1),
    shape(rim(0, 3, 8.2), "wood-700", 1),
    shape(rim(-5.2, 3, 6) + rim(5.4, 2.8, 5.6), "wood-700", 1),
  ];
}

/** Steam curling up from the anchor in three wisps to about 18 above it, each fading as it rises. */
function steam(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  for (let k = 0; k < 3; k += 1) {
    const x0 = (k - 1) * 3.4 + between(rng, -0.6, 0.6);
    const sway = between(rng, 1.4, 2.6) * (k % 2 === 0 ? 1 : -1);
    const points: Point[] = [];
    for (let i = 0; i <= 6; i += 1) points.push([x0 + Math.sin(i * 1.1 + k) * sway, -i * 3 - between(rng, 0, 0.6)]);
    out.push(shape(f.band(points, 1.6 - k * 0.2), "white", 0.32 - k * 0.04, MAIN));
  }
  out.push(shape(f.ellipse(0, -1, 6, 1.6), "white", 0.18, MAIN));
  return out;
}

/** A leather pouch burst open on its side on the anchor, its mouth gaping the way it faces and its drawstring loose; about 10 wide and 7 tall at scale 1. */
function pouch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-5, -1)}Q${f.p(-5.6, -6.4)} ${f.p(0, -6.2 + sag)}Q${f.p(3.6, -6)} ${f.p(4, -4.6)}L${f.p(4.2, -1.4)}Q${f.p(0, 0.6)} ${f.p(-5, -1)}Z`, "wood-600", 1, MAIN_INKED),
    shape(f.ellipse(4.2, -3, 1.3, 2.1), "wood-900", 1, INKED),
    shape(f.band([[3.4, -5], [5.4, -6.4], [7.2, -5.6], [8, -3.4 + sag]], 0.35), "wood-800", 1),
    shape(f.band([[-3.4, -4.6], [0.6, -5.4]], 0.6), "orange-400", 0.3),
    shape(f.circle(8, -3.2 + sag, 0.5), "brass-600", 1),
  ];
}

/** A waterfall pouring from the anchor down about 44 between dark rocks into spray and foam, its curtain widening a little as it falls; about 18 wide at scale 1. */
function cascade(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drop = 44;
  let streaks = "";
  for (let k = 0; k < 5; k += 1) {
    const x = -5.6 + k * 2.8 + between(rng, -0.4, 0.4);
    streaks += f.band([[x, 1], [x * 1.15, drop * between(rng, 0.6, 0.95)]], 0.45);
  }
  let spray = "";
  for (let k = 0; k < 5; k += 1) spray += f.circle(-6.4 + k * 3.2, drop - between(rng, 0, 2.4), between(rng, 1.4, 2.2));
  return [
    shape(f.poly([[-11, -2], [-6.4, 0], [-7.8, drop], [-12, drop]]) + f.poly([[11, -2], [6.4, 0], [7.8, drop], [12, drop]]), "slate-800", 1, INKED),
    shape(f.poly([[-6.6, 0], [6.6, 0], [7.6, drop], [-7.6, drop]]), "blue-200", 0.85, MAIN),
    shape(streaks, "white", 0.55),
    shape(f.band([[-6.6, 0.4], [6.6, 0.4]], 1), "white", 0.6),
    shape(spray, "white", 0.4),
    shape(f.ellipse(0, drop + 1.6, 10, 1.6), "slate-100", 0.55),
  ];
}

/** A mud-caked work glove reaching the way it faces from its cuff behind the anchor, fingers out to about 9 ahead; about 11 long and 6 tall at scale 1. */
function glove(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.6, 0.8);
  let fingers = "";
  for (let k = 0; k < 4; k += 1) {
    const y = -4.4 + k * 1.15;
    fingers += f.band([[2.6, y], [6.4 - Math.abs(k - 1.4) * 0.5, y + curl * 0.3], [8.4 - Math.abs(k - 1.4) * 0.7, y + curl]], 0.95);
  }
  return [
    shape(rect(f, -2.6, -5.4, 0.2, 0), "wood-800", 1, INKED),
    shape(`M${f.p(0, -5)}Q${f.p(2.4, -5.6)} ${f.p(3.4, -4.6)}L${f.p(3.4, -0.6)}Q${f.p(1.6, 0.6)} ${f.p(0, -0.4)}Z`, "wood-600", 1, MAIN_INKED),
    shape(fingers, "wood-600", 1, MAIN_INKED),
    shape(f.band([[1.8, -0.8], [3.4, 0.6], [5, 0.8]], 0.95), "wood-600", 1, INKED),
    shape(f.circle(1.4, -3.2, 0.7) + f.circle(5.6, -3.2 + curl * 0.3, 0.5) + f.circle(-1.2, -1.6, 0.6), "wood-900", 0.6),
  ];
}

/** A stone chime hung in a wooden frame on the anchor: two posts, a beam, a bent jade plate on two cords and the note it rings; about 14 wide and 14 tall at scale 1. */
function chime(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -4, 4);
  const plate = (dx: number, dy: number): Point => {
    const [x, y] = turned([dx, dy], swing);
    return [x, y - 7];
  };
  return [
    shape(rect(f, -6.8, -13, -5.8, 0) + rect(f, 5.8, -13, 6.8, 0), "wood-800", 1, INKED),
    shape(rect(f, -7.6, -14.4, 7.6, -13), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-3, -13], plate(-3, 0.2)], 0.2) + f.band([[2, -13], plate(2.4, -0.6)], 0.2), "slate-900", 0.8),
    shape(f.poly([plate(-5, 1.6), plate(-3, -0.2), plate(4.6, -1.6), plate(5, 0), plate(-2.6, 1.6), plate(-4.4, 3)]), "jade-400", 1, INKED),
    shape(f.band([plate(-2.6, 0.8), plate(3.6, -0.6)], 0.3), "white", 0.4),
    shape(f.circle(-6.3, 0.1, 0.9) + f.circle(6.3, 0.1, 0.9), "stone-500", 1),
  ];
}

/** A mossy boulder resting on the anchor, a crack across it and lichen on its flank; about 13 wide and 7 tall at scale 1. */
function boulder(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(lump(f, rng, 0, -3.4, 6.4, 3.6, 8), "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(-6, -2)}Q${f.p(0, 1.4)} ${f.p(6, -2)}L${f.p(5.4, -0.4)}Q${f.p(0, 0.6)} ${f.p(-5.4, -0.4)}Z`, "slate-700", 0.45),
    shape(lump(f, rng, -0.6, -6.4, 4.2, 1.2, 7), "green-700", 0.95),
    shape(f.circle(3.4, -3.4, 0.5) + f.circle(4.4, -2.2, 0.35) + f.circle(-3.6, -2.6, 0.4), "jade-400", 0.6),
    shape(f.band([[-1.6, -5.2], [-0.6, -3.4], [-1.2, -1.6]], 0.3), "slate-900", 0.45),
  ];
}

// Terrain

/** A terraced hillside from the placement's y down across the box: three or four curved green terraces, each held up by its stone wall, a water channel running along its lip. */
function terrace(at: PartPlace, rng: Rng): KitShape[] {
  const tiers = whole(rng, 3, 4);
  const out: KitShape[] = [];
  let walls = "";
  let channels = "";
  for (let k = 0; k < tiers; k += 1) {
    const y = at.y + (k === 0 ? 0 : (6 + 5 * k) * k * 0.55 * at.s);
    const bow = between(rng, 1.4, 3) * at.s;
    const left = y + between(rng, -1, 1);
    const right = y + between(rng, -1, 1);
    const curve = (dy: number): string => `${pt(EDGE, left + dy)}Q${pt(50, y - bow + dy)} ${pt(FAR_EDGE, right + dy)}`;
    out.push(shape(`M${pt(EDGE, FAR_EDGE)}L${curve(0)}L${pt(FAR_EDGE, FAR_EDGE)}Z`, k % 2 === 0 ? "green-700" : "green-500", 1, MAIN));
    if (k > 0) walls += `M${curve(0)}L${pt(FAR_EDGE, right + 2.4 * at.s)}Q${pt(50, y - bow + 2.4 * at.s)} ${pt(EDGE, left + 2.4 * at.s)}Z`;
    channels += `M${curve(2.8 * at.s)}L${pt(FAR_EDGE, right + 3.4 * at.s)}Q${pt(50, y - bow + 3.4 * at.s)} ${pt(EDGE, left + 3.4 * at.s)}Z`;
  }
  out.push(shape(walls === "" ? `M${pt(EDGE, at.y)}L${pt(FAR_EDGE, at.y)}L${pt(FAR_EDGE, at.y + 1)}L${pt(EDGE, at.y + 1)}Z` : walls, "stone-500", 1, INKED));
  out.push(shape(channels, "blue-200", 0.7));
  return out;
}

// Plants

/** A giant pumpkin sitting on the anchor, ribbed, with a stalk, a leaf and a curl of vine; about 12 wide and 9 tall at scale 1. */
function pumpkin(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 5.6, 6.4);
  const h = between(rng, 3.8, 4.4);
  return [
    shape(f.ellipse(-w * 0.55, -h, w * 0.5, h) + f.ellipse(w * 0.55, -h, w * 0.5, h), "ember-500", 1, INKED),
    shape(f.ellipse(0, -h, w * 0.55, h * 1.02), "orange-400", 1, MAIN_INKED),
    shape(f.band([[-w * 0.2, -h * 1.7], [-w * 0.25, -h * 0.4]], 0.4) + f.band([[w * 0.65, -h * 1.6], [w * 0.75, -h * 0.5]], 0.4), "white", 0.25),
    shape(f.band([[0, -h * 1.9], [0.6, -h * 2.4], [1.4, -h * 2.5]], 0.9), "green-900", 1, INKED),
    shape(f.ellipse(-2.4, -h * 2.1, 2, 0.9), "green-700", 1, INKED),
    shape(f.band([[1.4, -h * 2.5], [3, -h * 2.8], [3.4, -h * 2.3], [2.6, -h * 2.2]], 0.3), "green-700", 1),
  ];
}

/** A towering beanstalk rising from the anchor to about 44 above it, two stalks twisting round each other, broad leaves out both ways and a pod split open on boulder-sized seeds. */
function beanstalk(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, 0, Math.PI);
  const stalk = (offset: number): Point[] => {
    const points: Point[] = [];
    for (let i = 0; i <= 10; i += 1) points.push([Math.sin(i * 0.9 + phase + offset) * 1.6, -i * 4.4]);
    return points;
  };
  let leaves = "";
  let veins = "";
  for (let k = 0; k < 5; k += 1) {
    const y = -6 - k * 8 + between(rng, -1, 1);
    const side = k % 2 === 0 ? 1 : -1;
    leaves += f.ellipse(side * 4.2, y, 3.4, 1.5);
    veins += f.band([[side * 1.2, y + 0.3], [side * 6.6, y - 0.3]], 0.25);
  }
  return [
    shape(f.band(stalk(0), 1.5), "green-700", 1, INKED),
    shape(f.band(stalk(Math.PI), 1.2), "green-900", 1, INKED),
    shape(leaves, "green-500", 1, MAIN_INKED),
    shape(veins, "green-900", 0.6),
    shape(`M${f.p(-1, -30)}Q${f.p(-9, -30)} ${f.p(-8.6, -25)}Q${f.p(-4, -27.4)} ${f.p(-1, -30)}Z`, "green-700", 1, INKED),
    shape(f.circle(-6.8, -26.8, 1.1) + f.circle(-4.4, -27.6, 1.05), "stone-500", 1, INKED),
  ];
}

/** A wooden ladder leant from its feet on the anchor up to about 24 above it, its top tipped a little the way it faces. */
function ladder(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 2, 5);
  const h = 24;
  let rungs = "";
  for (let k = 1; k < 8; k += 1) {
    const t = k / 8;
    rungs += f.band([[-2 + lean * t, -h * t], [2 + lean * t, -h * t]], 0.5);
  }
  return [
    shape(f.band([[-2.2, 0], [-2.2 + lean, -h]], 0.7) + f.band([[2.2, 0], [2.2 + lean, -h]], 0.7), "wood-700", 1, MAIN_INKED),
    shape(rungs, "wood-800", 1),
  ];
}

/** A dark elderberry the size of a cart resting on the anchor: a purple sphere with a sheen, a dusty bloom, its crown and a stub of stalk; about 11 across at scale 1. */
function berry(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = between(rng, 5, 5.8);
  let dust = "";
  for (let k = 0; k < 5; k += 1) dust += f.circle(between(rng, -r * 0.6, r * 0.6), -r + between(rng, -r * 0.6, r * 0.6), between(rng, 0.25, 0.45));
  const crown: Point[] = [];
  for (let k = 0; k < 10; k += 1) {
    const a = (k * Math.PI) / 5;
    const reach = k % 2 === 0 ? 1.5 : 0.6;
    crown.push([reach * Math.cos(a), -2 * r + 0.2 + reach * 0.5 * Math.sin(a)]);
  }
  return [
    shape(f.circle(0, -r, r), "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(r * 0.2, -2 * r + 0.3)}A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(r * 0.2, -0.05)}Q${f.p(r * 0.75, -r)} ${f.p(r * 0.2, -2 * r + 0.3)}Z`, "slate-900", 0.35),
    shape(f.ellipse(-r * 0.4, -r * 1.4, r * 0.28, r * 0.18), "white", 0.4),
    shape(dust, "violet-300", 0.3),
    shape(f.poly(crown), "slate-900", 0.9),
    shape(f.band([[0, -2 * r + 0.2], [0.6, -2 * r - 1.8], [1.6, -2 * r - 2.4]], 0.5), "wood-800", 1),
  ];
}

// Objects

/** A full-size snowplow, its wheels on the anchor and its curved blade the way it faces: a boxy cab, windows, a light bar and snow caked on the blade; about 30 wide and 14 tall at scale 1. */
function snowplow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-11, -2], [10, -2], [10, -6], [8, -6], [8, -12.5], [-1, -12.5], [-3, -6 + tilt], [-11, -6 + tilt]]), "orange-400", 1, MAIN_INKED),
    shape(f.poly([[-0.4, -11.6], [3, -11.6], [3, -7], [-2, -7]]) + rect(f, 4, -11.6, 7.2, -7), "sky-300", 0.85),
    shape(f.band([[10, -4], [12.4, -4]], 1.2), "slate-700", 1),
    shape(`M${f.p(11.4, -0.4)}Q${f.p(15.6, -4)} ${f.p(13.2, -10)}L${f.p(15.4, -10.6)}Q${f.p(18.8, -4)} ${f.p(16.2, 0.4)}Z`, "slate-400", 1, INKED),
    shape(f.ellipse(15.6, -9.6, 1.6, 0.8) + f.ellipse(16.4, -1, 1.6, 0.7), "white", 0.9),
    shape(f.circle(-6.5, -1.8, 2.6) + f.circle(5, -1.8, 2.6), "black", 1, INKED),
    shape(f.circle(-6.5, -1.8, 1) + f.circle(5, -1.8, 1), "slate-400", 1),
    shape(rect(f, 1, -13.6, 4.6, -12.5), "red-500", 1),
    shape(rect(f, -10.4, -5.2, -4, -4.6), "slate-900", 0.5),
  ];
}

/** A glowing block of indented line-work hanging centred on the anchor, wordless bars of light at two or three depths of indent; about 12 wide and 9 tall at scale 1. */
function codeBlock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let lines = "";
  let indent = 0;
  for (let k = 0; k < 5; k += 1) {
    const y = -3.2 + k * 1.5;
    const x0 = -4.8 + indent * 1.4;
    lines += rect(f, x0, y - 0.3, x0 + between(rng, 2.4, 9.4 - indent * 1.4), y + 0.3);
    indent = Math.max(0, Math.min(2, indent + whole(rng, -1, 1)));
  }
  return [
    shape(f.ellipse(0, 0, 9, 7), "cyan-400", 0.14),
    shape(rect(f, -6, -4.6, 6, 4.6), "slate-900", 0.55, MAIN),
    shape(rect(f, -6, -4.6, 6, 4.6) + rect(f, -5.6, -4.2, 5.6, 4.2), "cyan-400", 0.8),
    shape(lines, "cyan-400", 0.95),
  ];
}

/** A blinking bar of cyan light standing on the anchor, about 1.2 wide and 10 tall at scale 1, in its glow. */
function cursor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 9.4, 10.6);
  return [
    shape(f.ellipse(0, -h / 2, 4.4, h * 0.8), "cyan-400", 0.12),
    shape(f.ellipse(0, -h / 2, 2.2, h * 0.62), "cyan-400", 0.2),
    shape(rect(f, -0.6, -h, 0.6, 0), "cyan-400", 0.95, MAIN),
    shape(rect(f, -0.2, -h + 0.6, 0.2, -0.6), "white", 0.8),
  ];
}

/** A card's frame rising in ink like a scaffold from the anchor: its sides and foot drawn, one side still climbing, its art window begun, braces crossing it and ink drips under it; about 16 wide and 22 tall at scale 1. */
function cardFrame(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 12, 18);
  let braces = "";
  let counter = "";
  for (let k = 0; k < 3; k += 1) {
    const y0 = -2 - k * 6;
    braces += f.band([[-7.6, y0], [7.6, y0 - 5]], 0.3);
    if (k < 2) counter += f.band([[7.6, y0 - 1], [-7.6, y0 - 6]], 0.3);
  }
  return [
    shape(f.band([[-8, -22], [-8, 0], [8, 0], [8, -reach]], 0.9), "slate-900", 1, MAIN),
    shape(f.band([[-6, -9], [6, -9], [6, -reach + 2]], 0.5) + f.band([[-6, -11], [-6, -19.6]], 0.5), "slate-900", 0.85),
    shape(braces, "slate-700", 0.45),
    shape(counter, "slate-700", 0.35),
    shape(f.band([[-6, -22], [-2.4, -22]], 0.5) + f.band([[0.6, -22], [3.4, -22]], 0.5), "slate-900", 0.25),
    shape(f.ellipse(-8, 1.6, 0.4, 0.9) + f.ellipse(8, -reach + 1.4, 0.35, 0.8) + f.ellipse(2.4, 1.8, 0.3, 0.7), "slate-900", 0.8),
  ];
}

/** A blank sheet of paper floating level, centred on the anchor and seen from above, a corner curling and its shadow below; about 30 wide and 8 deep at scale 1. */
function paperSheet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, 1.4, 2.4);
  return [
    shape(f.ellipse(0, 6, 15, 1.6), "slate-400", 0.25),
    shape(f.poly([[-15, 2], [13, 3], [15 - curl, -4], [-12, -5]]), "white", 1, MAIN_INKED),
    shape(f.poly([[15 - curl, -4], [15, -4 + curl * 0.4], [13.6, -3.2]]), "slate-100", 1, INKED),
    shape(f.band([[-13.4, -1.6], [13.6, -0.4]], 0.2), "slate-400", 0.35),
  ];
}

// Terrain

/** A lone karst spire standing on the anchor, its peak about 32 above it at scale 1: a smoothed ink body lit on its right, snow on its crown, brush strokes down its left and mist about its foot. */
function summit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.6, 0.6);
  const tall = between(rng, 0.92, 1.08);
  const outline: Point[] = [
    [-10, 0.5],
    [-7.6 + j(), -8],
    [-5 + j(), -12 * tall],
    [-4.6 + j(), -20 * tall],
    [-2.2, -27 * tall],
    [-0.6, -35 * tall],
    [1.6, -29 * tall],
    [3.6 + j(), -22 * tall],
    [4.4 + j(), -14 * tall],
    [7 + j(), -9],
    [10, 0.5],
  ];
  return [
    shape(smooth(f, outline), "slate-700", 0.95, MAIN),
    shape(f.poly([[-0.4, -31 * tall], [2.6, -25 * tall], [4, -14 * tall], [7, -6], [8.6, 0], [3.4, 0], [1.6, -16 * tall]]), "slate-400", 0.3),
    shape(f.poly([[-2, -28.6 * tall], [-0.7, -31.6 * tall], [1, -29.6 * tall], [1.3, -27.6 * tall], [0.4, -28.4 * tall], [-0.4, -27 * tall], [-1.2, -28 * tall]]), "white", 0.9),
    shape(f.band([[-2.2, -24 * tall], [-3.6, -18 * tall], [-3, -12 * tall]], 0.6) + f.band([[-5.6, -9], [-6.4, -4]], 0.5), "slate-900", 0.4),
    shape(f.circle(-4.6, -12.6 * tall, 0.9) + f.circle(3.8, -16 * tall, 0.8), "green-900", 0.85),
    shape(f.ellipse(0, -1, 13, 2.4), "white", 0.4),
  ];
}

/** A hut's plaster wall across the box from its top down to the anchor's y, its floor there, a round moon window open in it about 14 across centred 26 above the anchor (scaled), so what stands behind shows through; timber posts at its edges and a lintel. */
function moonWall(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const r = 14 * at.s;
  const cx = at.x;
  const cy = floor - 26 * at.s;
  const ring = (outer: number, inner: number): string => ellipsePathAt(cx, cy, outer) + ellipsePathAt(cx, cy, inner);
  const posts = between(rng, 5, 8);
  return [
    shape(`M${pt(EDGE, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, floor)}L${pt(EDGE, floor)}Z${ellipsePathAt(cx, cy, r)}`, "stone-500", 1, MAIN),
    shape(`M${pt(EDGE, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, floor * 0.16)}L${pt(EDGE, floor * 0.16)}Z`, "slate-900", 0.22),
    shape(
      `M${pt(EDGE, EDGE)}L${pt(posts, EDGE)}L${pt(posts, floor)}L${pt(EDGE, floor)}Z` +
        `M${pt(FAR_EDGE - posts, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, floor)}L${pt(FAR_EDGE - posts, floor)}Z`,
      "wood-800",
      1,
      INKED,
    ),
    shape(ring(r + 1.4 * at.s, r), "wood-700", 1, INKED),
    shape(`M${pt(EDGE, floor - 1.6)}L${pt(FAR_EDGE, floor - 1.6)}L${pt(FAR_EDGE, FAR_EDGE)}L${pt(EDGE, FAR_EDGE)}Z`, "wood-700", 1),
    shape(`M${pt(EDGE, floor + 6)}L${pt(FAR_EDGE, floor + 6)}L${pt(FAR_EDGE, floor + 6.5)}L${pt(EDGE, floor + 6.5)}Z`, "wood-900", 0.5),
  ];
}

/** A circle of radius `r` about (cx, cy) in the box's own frame. */
function ellipsePathAt(cx: number, cy: number, r: number): string {
  return `M${pt(cx - r, cy)}A${fmt(r)} ${fmt(r)} 0 1 0 ${pt(cx + r, cy)}A${fmt(r)} ${fmt(r)} 0 1 0 ${pt(cx - r, cy)}Z`;
}

// Objects

/** A parachute drifting down, its load's hook at the anchor: a canopy of gores about 18 wide whose crown stands 22 above the anchor, its shroud lines gathered to the hook. */
function parachute(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -1.6, 1.6);
  const crown = -22;
  const rim = -15;
  const canopy = `M${f.p(-9 + tilt, rim)}Q${f.p(-9 + tilt, crown - 1)} ${f.p(tilt, crown)}Q${f.p(9 + tilt, crown - 1)} ${f.p(9 + tilt, rim)}Q${f.p(4.5 + tilt, rim - 1.6)} ${f.p(tilt, rim)}Q${f.p(-4.5 + tilt, rim - 1.6)} ${f.p(-9 + tilt, rim)}Z`;
  let gores = "";
  for (const x of [-4.5, 4.5]) gores += `M${f.p(tilt + x * 0.2, crown + 0.4)}Q${f.p(tilt + x * 0.85, crown + 1.2)} ${f.p(tilt + x, rim - 1.1)}L${f.p(tilt + x * 1.25, rim - 1.2)}Q${f.p(tilt + x * 1.05, crown + 1.4)} ${f.p(tilt + x * 0.35, crown + 0.3)}Z`;
  let lines = "";
  for (const x of [-9, -4.5, 0, 4.5, 9]) lines += f.band([[tilt + x, rim - (x === 0 ? 1.6 : 0.6)], [0, 0]], 0.18);
  return [
    shape(lines, "slate-900", 0.65),
    shape(canopy, "green-700", 1, MAIN_INKED),
    shape(gores, "white", 0.35),
    shape(`M${f.p(-8.6 + tilt, rim)}Q${f.p(-4.5 + tilt, rim - 1.6)} ${f.p(tilt, rim)}Q${f.p(4.5 + tilt, rim - 1.6)} ${f.p(8.6 + tilt, rim)}Q${f.p(tilt, rim - 3.6)} ${f.p(-8.6 + tilt, rim)}Z`, "slate-900", 0.3),
    shape(f.circle(tilt, crown + 0.4, 0.7), "slate-100", 0.8),
  ];
}

/** A soft shield bubble resting on the anchor: a translucent sphere about 28 across at scale 1 (its body `main`), a brighter rim, a curved sheen at its upper left, a glint and its glow pooled on the ground. */
function shieldBubble(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 14 * between(rng, 0.94, 1.06);
  const sheen = between(rng, -0.6, 0.6);
  return [
    shape(f.ellipse(0, 0, r * 0.9, r * 0.16), "white", 0.25),
    shape(f.circle(0, -r, r), "sky-300", 0.16, MAIN),
    shape(f.circle(0, -r, r) + f.circle(0, -r, r * 0.92), "slate-100", 0.5),
    shape(`M${f.p(-r * 0.72, -r * 1.42)}Q${f.p(-r * 0.3 + sheen, -r * 1.86)} ${f.p(r * 0.2, -r * 1.84)}Q${f.p(-r * 0.3 + sheen, -r * 1.7)} ${f.p(-r * 0.72, -r * 1.42)}Z`, "white", 0.55),
    shape(sparkle(f, r * 0.62, -r * 1.5, 1.4) + sparkle(f, -r * 0.8, -r * 0.5, 0.9), "white", 0.8),
  ];
}

/** R3393000: the Meditative+ parts, by id, each with its kind. */
export const MOUNTAIN_KIT = {
  peaks: { category: "terrain", draw: peaks },
  seal: { category: "objects", draw: seal },
  "brush-stroke": { category: "objects", draw: brushStroke },
  "ink-whip": { category: "weather", draw: inkWhip },
  flag: { category: "objects", draw: flag },
  "camp-tent": { category: "architecture", draw: campTent },
  "vending-machine": { category: "objects", draw: vendingMachine },
  towels: { category: "objects", draw: towels },
  "star-map": { category: "objects", draw: starMap },
  stapler: { category: "objects", draw: stapler },
  "round-window": { category: "architecture", draw: roundWindow },
  "ink-pine": { category: "plants", draw: pine },
  bloom: { category: "plants", draw: bloom },
  "plum-branch": { category: "plants", draw: plumBranch },
  "lacquer-pavilion": { category: "architecture", draw: pavilion },
  stands: { category: "architecture", draw: stands },
  ripples: { category: "weather", draw: ripples },
  sacks: { category: "objects", draw: sacks },
  "steam-curl": { category: "weather", draw: steam },
  pouch: { category: "objects", draw: pouch },
  cascade: { category: "terrain", draw: cascade },
  glove: { category: "objects", draw: glove },
  chime: { category: "objects", draw: chime },
  "mossy-boulder": { category: "terrain", draw: boulder },
  terrace: { category: "terrain", draw: terrace },
  pumpkin: { category: "plants", draw: pumpkin },
  beanstalk: { category: "plants", draw: beanstalk },
  "wood-ladder": { category: "objects", draw: ladder },
  berry: { category: "plants", draw: berry },
  snowplow: { category: "objects", draw: snowplow },
  "code-block": { category: "objects", draw: codeBlock },
  cursor: { category: "objects", draw: cursor },
  "card-frame": { category: "objects", draw: cardFrame },
  "paper-sheet": { category: "objects", draw: paperSheet },
  summit: { category: "terrain", draw: summit },
  "moon-wall": { category: "architecture", draw: moonWall },
  parachute: { category: "objects", draw: parachute },
  "shield-bubble": { category: "weather", draw: shieldBubble },
} as const satisfies Readonly<Record<string, KitPart>>;
