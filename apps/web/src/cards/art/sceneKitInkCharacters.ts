// R3401000: the scene kit's ink characters (ART.md, "Scene recipes and kits"), which the Meditative scenes
// (#3401) added: the Units that are no person, each its portrait's whole subject and drawn to read at once as
// who it is, with a face or an eye where it has one. Creatures that stand on their anchor (a scarred,
// plate-armoured rhino at the charge, the shade-iris sprite, the fizzing jar sprite, a crouching glitch critter,
// a giant of rock and pine rising out of its mountainside, a pair of living cat ears), the bronze taotie mask
// rising from its anchor as from a cauldron's rim, and the totem pole of five peoples' faces standing on it.
// sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css name, none in Radiant's gold window;
// a glow is a stack of translucent shapes, never a gradient; shapes are filled even-odd, so a hole is a second
// sub-path and no two sub-paths of one shape overlap otherwise.

import { between, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, sparkle, spiralArm, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

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

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** A rectangle `w` by `h` about (cx, cy), turned `degrees` (a card in the air). */
function tilted(f: Local, cx: number, cy: number, w: number, h: number, degrees: number): string {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  const corner = (dx: number, dy: number): Point => [cx + dx * c - dy * s, cy + dx * s + dy * c];
  return f.poly([corner(-w / 2, -h / 2), corner(w / 2, -h / 2), corner(w / 2, h / 2), corner(-w / 2, h / 2)]);
}

/** A frame of the part's own scale and facing anchored at (dx, dy) of `f`'s (a spiral or a curl drawn about a point). */
function at(place: PartPlace, f: Local, dx: number, dy: number, s = 1): Local {
  return local({ x: f.x(dx), y: f.y(dy), s: place.s * s, flip: place.flip });
}

/** A path drawn on both sides of the part's centre line: `side` is 1 for its right half and -1 for its left. */
function both(draw: (side: 1 | -1) => string): string {
  return draw(1) + draw(-1);
}

/** A craggy boulder about (cx, cy), `r` its reach, its rim jittered by the stream. */
function blobAt(f: Local, rng: Rng, cx: number, cy: number, r: number): string {
  const rim: Point[] = [];
  for (let k = 0; k < 7; k += 1) {
    const angle = (k * 360) / 7 + 20;
    const reach = r * between(rng, 0.82, 1.08);
    rim.push([cx + reach * Math.cos(angle * DEGREES), cy + reach * 0.85 * Math.sin(angle * DEGREES)]);
  }
  return f.poly(rim);
}

// Characters

/** Rampaging Rhino: a scarred, plate-armoured rhino at the charge on the anchor, facing the way it faces, its head lowered, two horns, legs mid-gallop, saddlebags spilling cards behind it; about 36 long and 21 tall at scale 1. MAIN is its armoured hide (body, plates, head and near legs). */
function rhino(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const stride = between(rng, -1, 1);
  const dip = between(rng, 0, 0.9);
  const h = (dx: number, dy: number): string => f.p(dx, dy + dip);
  const H = (points: readonly Point[]): Point[] => points.map(([dx, dy]) => [dx, dy + dip]);
  const body =
    `M${f.p(-12, -6)}Q${f.p(-16.6, -6.8)} ${f.p(-16.2, -11)}Q${f.p(-15.8, -15.6)} ${f.p(-10.5, -16.2)}Q${f.p(-5.5, -16.4)} ${f.p(-2, -17)}` +
    `Q${f.p(2, -19.6)} ${f.p(5.4, -17.8)}L${f.p(8.4, -12.6)}L${f.p(8, -8)}Q${f.p(5, -5.2)} ${f.p(0, -5.6)}Q${f.p(-6, -5)} ${f.p(-12, -6)}Z`;
  const plates =
    `M${f.p(-1.2, -17.2)}Q${f.p(2.4, -19.9)} ${f.p(5.6, -17.6)}Q${f.p(7.6, -14)} ${f.p(6.8, -10.4)}Q${f.p(3, -8.4)} ${f.p(-0.6, -9.6)}Q${f.p(-1.9, -13.4)} ${f.p(-1.2, -17.2)}Z` +
    `M${f.p(-15.8, -12)}Q${f.p(-15.6, -15.8)} ${f.p(-10.6, -16.4)}Q${f.p(-8.2, -16.6)} ${f.p(-7, -16.2)}Q${f.p(-6.2, -12)} ${f.p(-7.6, -7.4)}Q${f.p(-11.6, -6.2)} ${f.p(-15, -8)}Q${f.p(-16.4, -9.6)} ${f.p(-15.8, -12)}Z`;
  const head =
    `M${h(3.6, -16.4)}L${h(4.2, -19.8)}L${h(6.6, -16.4)}Q${h(9.6, -15.2)} ${h(12, -11.6)}Q${h(14.6, -8.6)} ${h(16.8, -6.8)}` +
    `Q${h(17.8, -5.2)} ${h(16.4, -4.3)}Q${h(13.4, -3.4)} ${h(10.8, -4.8)}Q${h(6.8, -6)} ${h(4.2, -9)}Q${h(2.6, -12.6)} ${h(3.6, -16.4)}Z`;
  const horns =
    `M${h(13.6, -8.6)}Q${h(16.4, -10.4)} ${h(19.6, -14)}Q${h(18.4, -9.2)} ${h(16.6, -6.4)}Z` + `M${h(10.6, -11.8)}Q${h(12, -12.6)} ${h(12.9, -14.6)}Q${h(13.1, -12.2)} ${h(12.7, -10.2)}Z`;
  const far =
    taper(f, [[1.4, -7.4], [-0.4 - stride * 0.6, -4.2], [1 - stride, -1.8]], [3.4, 3, 2.8]) +
    taper(f, [[-7.6, -8], [-6.8 + stride * 0.4, -4], [-5.6 + stride, -0.3]], [3.6, 3, 2.8]) +
    f.band([[-16, -12.4], [-17.6, -11], [-17.4, -9.2]], 0.6);
  const near =
    taper(f, [[4.6, -8.6], [6.2 + stride * 0.4, -4.4], [7.6 + stride * 1.2, -0.3]], [4, 3.4, 3.2]) +
    taper(f, [[-11, -9], [-12.6 - stride * 0.4, -4.4], [-13.8 - stride * 1.2, -0.3]], [4.2, 3.4, 3.2]);
  const cards = whole(rng, 2, 3);
  const spill: [number, number, number][] = [
    [-8.6, -17.4, -28],
    [-12, -20.4, -62],
    [-15.8, -18.8, 14],
  ];
  return [
    shape(far, "slate-800", 1, INKED),
    shape(body, "slate-700", 1, MAIN_INKED),
    shape(plates, "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-6.6, -15], [-1.6, -15], [-1.2, -11], [-2, -8.8], [-4.2, -8], [-6.2, -8.8], [-7, -11.4]]) + f.band([[-4.3, -15.05], [-4, -17.4]], 1.1), "wood-700", 1, INKED),
    shape(head, "slate-700", 1, MAIN_INKED),
    shape(near, "slate-700", 1, MAIN_INKED),
    shape(
      `M${f.p(-12, -6.3)}Q${f.p(-6, -5.3)} ${f.p(0, -5.9)}Q${f.p(5, -5.6)} ${f.p(7.6, -8.2)}L${f.p(7, -9.4)}Q${f.p(0, -7.6)} ${f.p(-11.6, -8.2)}Z` +
        `M${h(11, -4.9)}Q${h(13.6, -3.7)} ${h(16.4, -4.4)}Q${h(13.8, -5.4)} ${h(11, -4.9)}Z` +
        f.band([[-6.4, -12.6], [-1.6, -12.6]], 0.7),
      "black",
      0.28,
    ),
    shape(
      `M${f.p(-15, -13)}Q${f.p(-14, -15.6)} ${f.p(-10.6, -15.8)}Q${f.p(-13.6, -14.8)} ${f.p(-15, -13)}Z` +
        `M${f.p(-0.2, -17.1)}Q${f.p(2.4, -19.1)} ${f.p(5, -17.4)}Q${f.p(2.4, -18.3)} ${f.p(-0.2, -17.1)}Z` +
        `M${h(7, -15.8)}Q${h(9.8, -14.6)} ${h(11.4, -12.2)}Q${h(9, -14)} ${h(7, -15.8)}Z`,
      "white",
      0.3,
    ),
    shape(f.band(H([[8.2, -14.4], [9.4, -12.6], [9, -11.8], [10.2, -10]]), 0.5) + f.band([[2.2, -16.4], [3.6, -14.2], [3.2, -13.4], [4.6, -11.4]], 0.5), "red-300", 0.85),
    shape(horns, "slate-100", 1, INKED),
    shape(f.circle(10.6, -11 + dip, 0.6) + f.ellipse(16.1, -5.6 + dip, 0.45, 0.3), "black", 0.95),
    shape(spill.slice(0, cards).map(([x, y, turn]) => tilted(f, x + between(rng, -0.3, 0.3), y + between(rng, -0.3, 0.3), 2.4, 3.3, turn + between(rng, -8, 8))).join(""), "white", 1, INKED),
  ];
}

/** The taotie of the ritual bronzes: a symmetric beast mask rising from the anchor (its lower edge, a cauldron's rim): curled horns, bulging eyes, a fanged maw with no lower jaw and carved spirals on its brow; about 31 wide and 25 tall at scale 1. MAIN is its bronze (horns, ears, face and ridge). */
function taotie(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const j = (n: number): number => between(rng, -n, n);
  const curl = j(0.5);
  const gaze = j(0.35);
  const s = (side: number, dx: number, dy: number): string => f.p(side * dx, dy);
  const horn = (side: 1 | -1): string =>
    taper(
      f,
      ([[5.2, -18.2], [8.2, -21.8], [12, -23.4], [14.8, -21.8 + curl], [15.2, -18.6 + curl], [13.4, -17 + curl], [11.6, -17.8 + curl], [11.7, -19.6 + curl]] as const).map(
        ([dx, dy]): Point => [side * dx, dy],
      ),
      [4, 3.6, 3, 2.4, 1.9, 1.4, 1, 0.6],
    );
  const ear = (side: 1 | -1): string => `M${s(side, 11, -14.2)}Q${s(side, 13.8, -14.4)} ${s(side, 15.2, -10.8)}Q${s(side, 13, -9.4)} ${s(side, 10.8, -9.8)}Z`;
  const face =
    `M${f.p(0, -19.6)}Q${f.p(3.5, -18.4)} ${f.p(7, -18.6)}Q${f.p(10.6, -17.4)} ${f.p(11.4, -13)}Q${f.p(12, -9.2)} ${f.p(10.4, -5.4)}L${f.p(2.6, -5.4)}` +
    `Q${f.p(0, -7)} ${f.p(-2.6, -5.4)}L${f.p(-10.4, -5.4)}Q${f.p(-12, -9.2)} ${f.p(-11.4, -13)}Q${f.p(-10.6, -17.4)} ${f.p(-7, -18.6)}Q${f.p(-3.5, -18.4)} ${f.p(0, -19.6)}Z`;
  const fangs = both(
    (side) =>
      `M${s(side, 6.2, -5.6)}L${s(side, 9.2, -5.6)}Q${s(side, 9, -2.2)} ${s(side, 7.4, 0)}Q${s(side, 7, -3)} ${s(side, 6.2, -5.6)}Z` +
      `M${s(side, 3.2, -5.6)}L${s(side, 5.4, -5.6)}L${s(side, 4.4, -3)}Z`,
  );
  const spirals = both((side) => spiralArm(at(place, f, side * 4.4, -16.4), side === 1 ? 180 : 0, side * 420, 0.3, 1.5, 0.5) + spiralArm(at(place, f, side * 8.8, -8.4), side === 1 ? 0 : 180, -side * 400, 0.3, 1.3, 0.5));
  return [
    shape(f.ellipse(0, -12, 16, 11), "jade-400", 0.12),
    shape(both(horn) + both(ear), "green-700", 1, MAIN_INKED),
    shape(`M${f.p(-9.8, -5.8)}L${f.p(9.8, -5.8)}Q${f.p(9.6, -1.6)} ${f.p(7, -0.3)}L${f.p(-7, -0.3)}Q${f.p(-9.6, -1.6)} ${f.p(-9.8, -5.8)}Z`, "slate-900", 0.9),
    shape(face, "green-700", 1, MAIN_INKED),
    shape(
      `M${f.p(-10.4, -5.4)}L${f.p(10.4, -5.4)}Q${f.p(11.2, -7.6)} ${f.p(10.8, -9)}Q${f.p(0, -6.6)} ${f.p(-10.8, -9)}Q${f.p(-11.2, -7.6)} ${f.p(-10.4, -5.4)}Z` +
        both((side) => `M${s(side, 2.2, -15.2)}Q${s(side, 5.4, -16.6)} ${s(side, 8.6, -14.6)}Q${s(side, 5.4, -15.6)} ${s(side, 2.2, -15.2)}Z`),
      "black",
      0.28,
    ),
    shape(f.poly([[-0.9, -21.2], [0.9, -21.2], [1.3, -10], [2.8, -8], [2, -6.8], [0, -7.6], [-2, -6.8], [-2.8, -8], [-1.3, -10]]), "green-700", 1, MAIN_INKED),
    shape(fangs, "green-200", 1, INKED),
    shape(f.circle(5.4, -12.2, 2.3) + f.circle(-5.4, -12.2, 2.3), "jade-400", 1, INKED),
    shape(f.circle(5.4 + gaze, -12.2, 1.1) + f.circle(-5.4 + gaze, -12.2, 1.1), "slate-900", 1),
    shape(spirals, "jade-400", 0.8),
    shape(
      `M${f.p(-6.6, -18.3)}Q${f.p(0, -20.4)} ${f.p(6.6, -18.3)}Q${f.p(0, -19.2)} ${f.p(-6.6, -18.3)}Z` + both((side) => f.band([[side * 8.6, -22.4], [side * 12, -23.7], [side * 14.4, -22.4]], 0.7)),
      "jade-400",
      0.4,
    ),
    shape(f.circle(5 + gaze, -12.7, 0.38) + f.circle(-5.8 + gaze, -12.7, 0.38), "white", 0.85),
  ];
}

/** Shade-iris: a shadowy violet iris sprite standing on the anchor, facing the way it faces: a hood of three petals over a dark face with two pale eyes, falls for shoulders, a slim stem, root legs and a root hand holding a seed; about 10 wide and 18.5 tall at scale 1. MAIN is its hood (the three petals). */
function irisSprite(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const lean = between(rng, -0.6, 0.6);
  const reach = between(rng, -0.5, 0.5);
  const L = (points: readonly Point[]): Point[] => points.map(([dx, dy]) => [dx - (lean * dy) / 18, dy]);
  const q = (dx: number, dy: number): string => f.p(dx - (lean * dy) / 18, dy);
  const j = (n: number): number => between(rng, -n, n);
  const side = (k: 1 | -1, open: number): string =>
    `M${q(k * 0.5, -10.4)}Q${q(k * 4.6, -11.4)} ${q(k * (4.2 + open), -17 + j(0.3))}Q${q(k * 2.4, -16.2)} ${q(k * 1.6, -14.8)}Q${q(k * 2.6, -12.4)} ${q(k * 0.5, -10.4)}Z`;
  const fall = (k: 1 | -1): string => `M${q(k * 0.4, -10.6)}Q${q(k * 3.6, -11)} ${q(k * 4.4, -7.6)}Q${q(k * 2.4, -8.2)} ${q(k * 0.4, -9.6)}Z`;
  const roots = f.poly([
    [-0.75, -3.6], [-1.5, -1.6], [-4.2, -0.2], [-3.4, 0], [-1.6, -0.8], [-1.6, 0], [-0.9, 0], [-0.5, -0.9],
    [0.5, -0.9], [0.9, 0], [1.6, 0], [1.6, -0.8], [3.4, 0], [4.2, -0.2], [1.5, -1.6], [0.75, -3.6],
  ]);
  const hand: Point[] = L([[0.7, -6.6], [2.2, -7.8 + reach], [3.4, -7.6 + reach]]);
  const seed: Point = L([[4, -8.2 + reach]])[0] ?? [4, -8.2];
  return [
    shape(f.ellipse(0, -9.4, 6, 9.6), "slate-900", 0.12),
    shape(
      `M${f.p(-0.4, -0.6)}Q${f.p(-3.6, -3)} ${f.p(-3.8, -8.6)}Q${f.p(-2.4, -4)} ${f.p(0, -1.4)}Z` + `M${f.p(0.6, -0.8)}Q${f.p(2.6, -2.6)} ${f.p(2.4, -6.2)}Q${f.p(1.6, -3)} ${f.p(0.9, -1.6)}Z`,
      "green-900",
      1,
    ),
    shape(roots + f.band(hand, 0.7), "slate-800", 1),
    shape(f.band(L([[0, -3.2], [0.3, -7], [0, -10.8]]), 1.3), "green-900", 1),
    shape(fall(1) + fall(-1), "violet-500", 1, INKED),
    shape(f.ellipse(seed[0], seed[1], 0.85, 1.05), "brass-500", 1),
    shape(f.band([[seed[0] - 0.7, seed[1] + 0.5], [seed[0] - 0.9, seed[1] - 0.4], [seed[0] - 0.4, seed[1] - 1]], 0.35) + f.band([[seed[0] + 0.2, seed[1] + 1], [seed[0] + 0.9, seed[1] + 0.6]], 0.35), "slate-800", 1),
    shape(`M${q(-1.6, -13.8)}Q${q(-2.4, -17.2)} ${q(0, -18.6)}Q${q(2.4, -17.2)} ${q(1.6, -13.8)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.ellipse(-lean * 0.7, -12.8, 2.2, 2.5), "slate-900", 1),
    shape(side(1, j(0.4)) + side(-1, j(0.4)), "violet-700", 1, MAIN_INKED),
    shape(f.circle(0.9 - lean * 0.7, -13, 0.42) + f.circle(-0.7 - lean * 0.7, -13, 0.42), "slate-100", 1),
    shape(f.band(L([[0, -15.2], [0, -17.6]]), 0.3) + f.band(L([[2.2, -12], [3.4, -15.6]]), 0.28) + f.band(L([[-2.2, -12], [-3.4, -15.6]]), 0.28), "violet-300", 0.55),
  ];
}

/** The jar's outline, its belly `w` wide each side, inset by `i` (0 its glass's outside). */
function jarPath(f: Local, w: number, i: number): string {
  const b = -1.4 - i;
  return (
    `M${f.p(-2.7 + i, -10.8 + i)}Q${f.p(-2.6 + i, -9.8 + i)} ${f.p(-3.6 + i, -9.4 + i)}Q${f.p(-w - 0.2 + i, -8)} ${f.p(-w + i, -5.4)}Q${f.p(-w + i, b)} ${f.p(-2.8, b)}` +
    `L${f.p(2.8, b)}Q${f.p(w - i, b)} ${f.p(w - i, -5.4)}Q${f.p(w + 0.2 - i, -8)} ${f.p(3.6 - i, -9.4 + i)}Q${f.p(2.6 - i, -9.8 + i)} ${f.p(2.7 - i, -10.8 + i)}Z`
  );
}

/** Volatility: a jittery sprite of fizzing liquid in a glass jar standing on the anchor on two tiny legs: a lidded jar, bright liquid whose level and bulge vary, bubbles, two eyes; about 10 wide and 14 tall at scale 1. MAIN is the liquid. */
function jarSprite(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const w = between(rng, 4.3, 4.8);
  const level = between(rng, -8.6, -6.6);
  const bulge = between(rng, 0.4, 1.4);
  const inset = 0.55;
  // The glass's inner half-width at the liquid's level, along its side's curve from the belly (-5.4) to the shoulder (-9.4).
  const t = (-5.4 - level) / 4;
  const side = (1 - t) ** 2 * (w - inset) + 2 * t * (1 - t) * (w + 0.2 - inset) + t ** 2 * (3.6 - inset);
  const bottom = -1.4 - inset;
  const liquid =
    `M${f.p(-side, level)}L${f.p(-w + inset, -5.4)}Q${f.p(-w + inset, bottom)} ${f.p(-2.8, bottom)}L${f.p(2.8, bottom)}Q${f.p(w - inset, bottom)} ${f.p(w - inset, -5.4)}` +
    `L${f.p(side, level)}Q${f.p(side * 0.55, level - bulge)} ${f.p(0, level - bulge)}Q${f.p(-side * 0.55, level - bulge)} ${f.p(-side, level)}Z`;
  const tip = between(rng, -10, 10);
  const lid = (dx: number, dy: number): string => {
    const [x, y] = turned([dx, dy + 11.5], tip);
    return f.p(x, y - 11.5);
  };
  const kick = between(rng, -0.5, 0.5);
  const spots: [number, number, number][] = [
    [-1.8, level + 1.6, 0.45],
    [1.6, level + 1.2, 0.35],
    [0.2, level + 2.4, 0.3],
    [-0.6, level - bulge + 0.7, 0.25],
    [2.6, -3.2, 0.3],
  ];
  const fizz = spots.slice(0, whole(rng, 3, 5)).map(([x, y, r]) => f.circle(x + between(rng, -0.2, 0.2), y, r));
  return [
    shape(f.ellipse(0, -7, 6.6, 7.4), "lime-400", 0.14),
    shape(f.band([[-6, -8.4], [-6.4, -6.8]], 0.35) + f.band([[-6.6, -5.6], [-6.7, -4.4]], 0.35) + f.band([[6, -8.6], [6.4, -7]], 0.35) + f.band([[6.6, -5.8], [6.7, -4.6]], 0.35), "slate-900", 0.45),
    shape(f.band([[-1.5, -1.8], [-1.9 + kick, -0.5], [-2.8 + kick, -0.2]], 0.55) + f.band([[1.5, -1.8], [1.9 - kick, -0.5], [2.8 - kick, -0.2]], 0.55), "slate-900", 1),
    shape(jarPath(f, w, 0), "slate-100", 0.45),
    shape(liquid, "lime-400", 0.95, MAIN),
    shape(`M${f.p(-w + inset + 0.3, -4.4)}Q${f.p(-w + inset + 0.3, -2.2)} ${f.p(-2.2, -2.2)}L${f.p(2.2, -2.2)}Q${f.p(w - inset - 0.3, -2.2)} ${f.p(w - inset - 0.3, -4.4)}Q${f.p(0, -2.8)} ${f.p(-w + inset + 0.3, -4.4)}Z`, "green-500", 0.55),
    shape(f.ellipse(-1.2, -4.9, 0.95, 1.05) + f.ellipse(1.4, -4.9, 0.95, 1.05), "white", 1),
    shape(f.circle(-0.95, -4.8, 0.5) + f.circle(1.65, -4.8, 0.5), "black", 0.95),
    shape(fizz.join("") + `M${f.p(-3.3, -8.2)}Q${f.p(-3.8, -5.4)} ${f.p(-3.2, -3)}Q${f.p(-3.3, -5.4)} ${f.p(-2.8, -8)}Z`, "white", 0.7),
    shape(jarPath(f, w, 0) + jarPath(f, w, inset), "slate-900", 0.9),
    shape(rect(f, -3.1, -11.6, 3.1, -10.5), "slate-400", 1, INKED),
    shape(`M${lid(-2.3, -11.6)}L${lid(2.3, -11.6)}L${lid(2, -13.8)}L${lid(-2, -13.8)}Z`, "wood-600", 1, INKED),
  ];
}

/** Yile's Virus: a small boxy glitch critter crouched on guard on the anchor, facing the way it faces: a blocky body flickering with pixels, a slice of it slipped sideways, two square eyes, stubby legs and a guarding fist, sparks; about 11 wide and 9 tall at scale 1. MAIN is its body (and its slipped slice). */
function glitchCritter(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const slip = between(rng, 0.8, 1.6) * (rng() < 0.5 ? 1 : -1);
  const band = between(rng, -5.4, -3.4);
  const cells: Point[] = [];
  for (let y = -7; y <= -2.6; y += 1.1) for (let x = -3.6; x <= 3.2; x += 1.1) if (!(y < -4.6 && x > -0.4)) cells.push([x, y]);
  const pick = (): Point => {
    const k = Math.floor(rng() * cells.length);
    return cells.splice(k, 1)[0] ?? [0, 0];
  };
  const pixels = (count: number, size: number): string => {
    let d = "";
    for (let k = 0; k < count; k += 1) {
      const [x, y] = pick();
      d += rect(f, x, y, x + size, y + size);
    }
    return d;
  };
  const cyan = pixels(whole(rng, 2, 3), 0.8);
  const violet = pixels(whole(rng, 2, 3), 0.8);
  const red = pixels(whole(rng, 1, 2), 0.8);
  return [
    shape(f.ellipse(0, -4.6, 7, 5.6), "cyan-400", 0.12),
    shape(rect(f, -3.8, -2, -2.4, 0) + rect(f, -0.7, -2, 0.7, 0) + rect(f, 2.4, -2, 3.8, 0) + rect(f, 4.6, -6, 6, -3.8), "slate-900", 1),
    shape(f.poly([[-4.4, -7.6], [-3.8, -8.2], [3.6, -7.4], [4.4, -6.6], [4.4, -2.2], [3.8, -1.6], [-3.8, -1.6], [-4.4, -2.2]]), "slate-800", 1, MAIN_INKED),
    shape(rect(f, -4.4 + slip, band, 4.4 + slip, band + 0.7) + rect(f, -5.6 - slip * 0.4, -7.2, -4.8 - slip * 0.4, -6.4), "slate-800", 1, MAIN),
    shape(f.poly([[-3.8, -7.7], [3.5, -7], [3.2, -6.5], [-3.6, -7.1]]), "slate-400", 0.45),
    shape(cyan + rect(f, 5.2, -8.6, 5.8, -8), "cyan-400", 1),
    shape(violet + rect(f, -6, -4.6, -5.4, -4), "violet-500", 1),
    shape(red, "red-500", 1),
    shape(rect(f, 0, -6.4, 1.4, -5) + rect(f, 2.2, -6.2, 3.6, -4.8), "white", 1),
    shape(rect(f, 0.7, -6, 1.3, -5.4) + rect(f, 2.9, -5.8, 3.5, -5.2), "black", 1),
    shape(sparkle(f, 6.4, -9.6, 1.3) + sparkle(f, -5.8, -8.8, 0.9) + sparkle(f, 1.2, -10, 0.8), "cyan-400", 0.9),
  ];
}

/** Montana Giant: a giant of rock and pine rising out of the mountainside on the anchor: a craggy boulder torso and head, stone arms with fists on the slope, small pines on its shoulders, glowing jade eyes; about 37 wide and 46 tall at scale 1. MAIN is its rock (torso, head, arms and fists). */
function rockGiant(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const j = (n: number): number => between(rng, -n, n);
  const J = (points: readonly Point[]): Point[] => points.map(([dx, dy]) => [dx + j(0.35), dy + j(0.35)]);
  const torso = f.poly(
    J([[-12.5, -4], [-13.6, -14], [-12.4, -22], [-13.2, -28], [-10.6, -32.6], [-5.8, -34.2], [-3.4, -33], [3.4, -33], [6.2, -34.4], [11, -32.8], [13.4, -28.4], [12.6, -21.6], [13.8, -13.6], [12.6, -4]]),
  );
  const head = f.poly(J([[-5.4, -32.6], [-6.2, -37.6], [-4.8, -42.8], [-1.2, -45.8], [2.8, -45.2], [5.6, -42], [6.4, -37], [5, -32.6]]));
  const arm = (k: 1 | -1): string =>
    f.poly(J([[k * 10.4, -32.4], [k * 14.4, -31.6], [k * 17.8, -25], [k * 18.4, -15], [k * 17.6, -9], [k * 13.8, -9.4], [k * 14.2, -19], [k * 12.2, -26.6]]));
  const fists = blobAt(f, rng, 15.4, -6.6, 3.6) + blobAt(f, rng, -15.4, -6.6, 3.6);
  const pine = (x: number, y: number, size: number): string =>
    f.poly(
      ([[0, -5.4], [1.1, -3.6], [0.6, -3.6], [1.6, -1.8], [0.9, -1.8], [1.9, 0.4], [-1.9, 0.4], [-0.9, -1.8], [-1.6, -1.8], [-0.6, -3.6], [-1.1, -3.6]] as const).map(
        ([dx, dy]): Point => [x + dx * size, y + dy * size],
      ),
    );
  const pines = [pine(-10.4, -33, 1), pine(9.6, -33.6, 1.1)];
  if (rng() < 0.6) pines.push(pine(-6.4, -34.2, 0.75));
  if (rng() < 0.5) pines.push(pine(13.2, -32, 0.7));
  const gaze = j(0.4);
  return [
    shape(torso, "slate-700", 1, MAIN_INKED),
    shape(f.poly([[4, -31], [10.6, -32], [12.8, -28], [12, -21.4], [13, -13.6], [12, -5], [6, -8.4], [8, -18], [5.4, -26]]) + f.poly([[-11, -12], [-6, -10], [-9, -18]]), "black", 0.28),
    shape(head, "slate-700", 1, MAIN_INKED),
    shape(arm(1) + arm(-1), "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(-19, 0)}L${f.p(-19, -4.6)}Q${f.p(-14, -8.4)} ${f.p(-8, -9.8)}Q${f.p(0, -11.6)} ${f.p(8, -10)}Q${f.p(14, -8.8)} ${f.p(19, -4.4)}L${f.p(19, 0)}Z`, "slate-400", 1, INKED),
    shape(fists, "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-12.6, -21], [-11.4, -30.4], [-7.4, -32.6], [-8.6, -26]]) + f.poly([[-4.6, -41.8], [-1.4, -45], [-1.2, -42]]) + f.poly([[-17.4, -24], [-16.6, -14], [-15.6, -24.4]]), "white", 0.18),
    shape(
      f.band(J([[-4, -30], [-2.4, -26], [-3.4, -22], [-1.6, -18]]), 0.45) +
        f.band(J([[7, -24], [9, -21], [8.2, -17]]), 0.45) +
        f.band([[-5.6, -36.6], [-0.4, -37.6], [5.8, -36.4]], 0.9) +
        f.ellipse(-2.1, -39.1, 1.7, 1.1) +
        f.ellipse(2.7, -39.1, 1.7, 1.1),
      "slate-900",
      0.75,
    ),
    shape(pines.join(""), "green-900", 1, INKED),
    shape(`M${f.p(-17, -9.4)}Q${f.p(-10, -12.8)} ${f.p(-2, -11)}Q${f.p(6, -13.2)} ${f.p(16, -9.8)}Q${f.p(8, -8.4)} ${f.p(0, -9.2)}Q${f.p(-9, -8.2)} ${f.p(-17, -9.4)}Z`, "white", 0.45),
    shape(f.ellipse(0.3, -39, 5.6, 3), "jade-400", 0.28),
    shape(f.ellipse(-2.1 + gaze, -39.1, 1.1, 0.55) + f.ellipse(2.7 + gaze, -39.1, 1.1, 0.55), "jade-400", 1),
  ];
}

/** Sentient Cat Ears: a pair of fluffy living cat ears standing on the anchor (a warrior's helmet, or mid-leap), pink inside, joined by a furry band with two tiny bright eyes, a motion arc behind; about 10 wide and 7 tall at scale 1. MAIN is the fur (ears and band). */
function catEars(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const twitch = [between(rng, -0.7, 0.7), between(rng, -0.7, 0.7)] as const;
  const ear = (k: 1 | -1, lean: number): string =>
    f.poly([
      [k * 4.8, -1.2], [k * 5.1, -3.2], [k * 4.6, -3.3], [k * (4.6 + lean * 0.3), -5], [k * (4.1 + lean * 0.3), -5], [k * (3.4 + lean), -7.1],
      [k * (2.6 + lean * 0.4), -5.6], [k * (2.2 + lean * 0.4), -5.7], [k * 1.4, -3.8], [k * 1, -3.7], [k * 0.7, -2],
    ]);
  const inner = (k: 1 | -1, lean: number): string => f.poly([[k * 4, -2.2], [k * (3.4 + lean * 0.9), -5.9], [k * 1.6, -2.6]]);
  const tufts = (k: 1 | -1, lean: number): string =>
    f.band([[k * 3.3, -2.4], [k * (3.1 + lean * 0.4), -3.9]], 0.32) + f.band([[k * 2.5, -2.5], [k * (2.6 + lean * 0.5), -3.6]], 0.3);
  const blink = between(rng, 0.25, 0.45);
  return [
    shape(f.band([[-7.8, -0.4], [-7.8, -3.4], [-6.6, -6]], 0.4) + f.band([[-6.6, -0.6], [-6.7, -2.6]], 0.3), "slate-900", 0.4),
    shape(ear(1, twitch[0]) + ear(-1, -twitch[1]), "orange-400", 1, MAIN_INKED),
    shape(inner(1, twitch[0]) + inner(-1, -twitch[1]), "red-300", 1),
    shape(f.band([[1.6, -4.8 - twitch[0] * 0.1], [2.4, -4.6]], 0.3) + f.band([[-1.6, -4.8], [-2.4, -4.6]], 0.3) + f.band([[4.9, -2.2], [4.2, -2.4]], 0.3) + f.band([[-4.9, -2.2], [-4.2, -2.4]], 0.3), "wood-700", 0.5),
    shape(tufts(1, twitch[0]) + tufts(-1, -twitch[1]), "white", 0.85),
    shape(f.band([[-5, -0.5], [-2.6, -1.5], [0, -1.8], [2.6, -1.5], [5, -0.5]], 1.5), "orange-400", 1, MAIN_INKED),
    shape(f.ellipse(-1.1, -1.4, 0.6, blink) + f.ellipse(1.1, -1.4, 0.6, blink), "lime-400", 1),
    shape(f.band([[-1, -1.4 - blink * 0.8], [-1, -1.4 + blink * 0.8]], 0.16) + f.band([[1.2, -1.4 - blink * 0.8], [1.2, -1.4 + blink * 0.8]], 0.16), "black", 0.9),
  ];
}

/** Tatches the Totem: a carved totem pole standing on the anchor, five tiers of five peoples' faces from the foot up (a human, a cat-folk mask with ears, a scholar in spectacles under a winged cap, a maned lion-dog, a soldier's crested helmet), in carved wood with red and jade paint; about 13 wide and 46 tall at scale 1. MAIN is its wood (the tiers and what juts from them). */
function totem(place: PartPlace, rng: Rng): KitShape[] {
  const f = local(place);
  const s = (side: number, dx: number, dy: number): string => f.p(side * dx, dy);
  const gape = between(rng, 0, 0.6);
  const prick = between(rng, -0.5, 0.5);
  const sweep = between(rng, -0.6, 0.6);
  // Behind the tiers: the human's ears, the cat's ears, the scholar's cap wings, the lion's mane.
  const maneAt = (k: number, rx: number, ry: number): Point => [rx * Math.cos((k * 30) * DEGREES), -29 + ry * Math.sin((k * 30) * DEGREES)];
  let mane = `M${f.p(...maneAt(0, 6.4, 5.4))}`;
  for (let k = 1; k <= 12; k += 1) mane += `Q${f.p(...maneAt(k - 0.5, 7.6, 6.4))} ${f.p(...maneAt(k, 6.4, 5.4))}`;
  const behind =
    both((side) => f.ellipse(side * 4.8, -4.4, 1.1, 1.6)) +
    both((side) => `M${s(side, 2.6, -15.6)}L${s(side, 6.6 + prick, -19.2)}L${s(side, 5.6, -13.6)}Z`) +
    both((side) => f.ellipse(side * 6.8, -23.1, 2.6, 0.9)) +
    `${mane}Z`;
  const tiers =
    `M${f.p(-4.6, 0)}L${f.p(4.6, 0)}L${f.p(4.6, -6.6)}Q${f.p(4.6, -8.2)} ${f.p(3.2, -8.2)}L${f.p(-3.2, -8.2)}Q${f.p(-4.6, -8.2)} ${f.p(-4.6, -6.6)}Z` +
    `M${f.p(-4, -8.2)}L${f.p(4, -8.2)}Q${f.p(5.2, -9.6)} ${f.p(5.2, -12)}L${f.p(5, -16.4)}L${f.p(-5, -16.4)}L${f.p(-5.2, -12)}Q${f.p(-5.2, -9.6)} ${f.p(-4, -8.2)}Z` +
    f.poly([[-4.4, -16.4], [4.4, -16.4], [4.4, -21.4], [5.2, -21.6], [4.8, -24.6], [-4.8, -24.6], [-5.2, -21.6], [-4.4, -21.4]]) +
    `M${f.p(-4.2, -24.6)}L${f.p(4.2, -24.6)}Q${f.p(4.8, -28.6)} ${f.p(4.2, -32.8)}L${f.p(-4.2, -32.8)}Q${f.p(-4.8, -28.6)} ${f.p(-4.2, -24.6)}Z` +
    `M${f.p(-4.2, -32.8)}L${f.p(4.2, -32.8)}L${f.p(4.6, -37)}L${f.p(5.6, -37.4)}Q${f.p(5.2, -42.6)} ${f.p(0, -42.8)}Q${f.p(-5.2, -42.6)} ${f.p(-5.6, -37.4)}L${f.p(-4.6, -37)}Z`;
  const recess =
    both((side) => f.ellipse(side * 1.8, -5.1, 1.25, 0.8)) +
    `M${f.p(-3, -27.6 + gape * 0.3)}Q${f.p(0, -25 + gape)} ${f.p(3, -27.6 + gape * 0.3)}L${f.p(3.2, -28.2)}L${f.p(-3.2, -28.2)}Z` +
    rect(f, -3.6, -36.4, 3.6, -35.2);
  const raised =
    f.poly([[-0.5, -5], [0.5, -5], [0.8, -3.2], [-0.8, -3.2]]) +
    both((side) => f.band([[side * 0.8, -6.3], [side * 2.9, -6.6]], 0.55)) +
    both((side) => f.ellipse(side * 0.8, -10.2, 0.75, 0.6)) +
    `M${f.p(-3.2, -17.6)}Q${f.p(-1.6, -18.8)} ${f.p(0, -18.2)}Q${f.p(1.6, -18.8)} ${f.p(3.2, -17.6)}Q${f.p(1.6, -18.1)} ${f.p(0, -17.6)}Q${f.p(-1.6, -18.1)} ${f.p(-3.2, -17.6)}Z` +
    f.band([[-3.6, -31.8], [0, -32.4], [3.6, -31.8]], 0.7) +
    f.band([[0, -37.2], [0, -34.2]], 0.8) +
    f.band([[-5, -37.5], [0, -38], [5, -37.5]], 0.5);
  const jade =
    both((side) => `M${s(side, 0.9, -13.2)}Q${s(side, 2.2, -14.4)} ${s(side, 3.4, -13.6)}Q${s(side, 2.2, -12.4)} ${s(side, 0.9, -13.2)}Z`) +
    rect(f, -4.4, -22.2, 4.4, -21.6) +
    f.circle(1.8, -30, 1.05) +
    f.circle(-1.8, -30, 1.05) +
    both((side) => f.circle(side * 1.6, -35.8, 0.42));
  const red =
    `M${f.p(-1.4, -2.2)}Q${f.p(0, -1.4)} ${f.p(1.4, -2.2)}Q${f.p(0, -1.9)} ${f.p(-1.4, -2.2)}Z` +
    f.poly([[-0.6, -11.2], [0.6, -11.2], [0, -10.4]]) +
    f.ellipse(0, -26.4 + gape * 0.5, 1, 0.5) +
    `M${f.p(-0.8, -42.6)}Q${f.p(-1, -46)} ${f.p(1.8, -46.2 + sweep)}Q${f.p(4.8, -46 + sweep)} ${f.p(5.8, -43.4 + sweep)}Q${f.p(3, -44.6)} ${f.p(1, -42.7)}Z` +
    both((side) => f.poly([[side * 5.2, -16.2], [side * (6.2 + prick * 0.8), -18.4], [side * 5.6, -15.4]]));
  const lines =
    both((side) => f.circle(side * 1.8, -5.1, 0.42)) +
    both((side) => f.band([[side * 1.6, -12.6], [side * 1.7, -13.8]], 0.25)) +
    both((side) => f.circle(side * 1.7, -19.8, 1.3) + f.circle(side * 1.7, -19.8, 0.85)) +
    f.band([[-0.35, -19.9], [0.35, -19.9]], 0.3) +
    both((side) => f.circle(side * 1.8, -30, 0.48)) +
    both((side) => f.band([[side * 3.2, -10.4], [side * 4.6, -10.8]], 0.2) + f.band([[side * 3.2, -9.8], [side * 4.6, -9.6]], 0.2));
  return [
    shape(behind, "wood-700", 1, MAIN_INKED),
    shape(tiers, "wood-700", 1, MAIN_INKED),
    shape(recess, "black", 0.45),
    shape(raised, "wood-600", 1),
    shape(jade, "jade-400", 1),
    shape(red, "red-600", 1),
    shape(f.poly([[-2.8, -28.1], [2.8, -28.1], [2.2, -27.3], [1.6, -27.9], [1, -27.2], [0.3, -27.9], [-0.3, -27.9], [-1, -27.2], [-1.6, -27.9], [-2.2, -27.3]]), "white", 0.95),
    shape(lines, "slate-900", 1),
  ];
}

/** R3401000: the ink characters the Meditative scenes added, by id, each with its kind. */
export const INK_CHARACTERS_KIT = {
  rhino: { category: "creatures", draw: rhino },
  taotie: { category: "creatures", draw: taotie },
  "iris-sprite": { category: "creatures", draw: irisSprite },
  "jar-sprite": { category: "creatures", draw: jarSprite },
  "glitch-critter": { category: "creatures", draw: glitchCritter },
  "rock-giant": { category: "creatures", draw: rockGiant },
  "cat-ears": { category: "creatures", draw: catEars },
  totem: { category: "architecture", draw: totem },
} as const satisfies Readonly<Record<string, KitPart>>;
