// R3393000: the scene kit's parts that the Training Grounds scenes (#3393) added: the yard's ground (raked sand,
// three sand lanes), its buildings (a spiked palisade, a barracks hall, a moon gate, a well, a smithy forge, a
// carved central pole, a siege tower, a recycling mill on its conveyor, an obstacle course's cargo net, a covered
// walkway and a bench), a dinosaur and a tumbleweed, its props (a throwing knife, a straw dummy, an iron-banded oak
// block, a scorched rune, a booby-trapped welcome mat, a wooden runner token, a wordless poster, a training
// catapult, a jelly bean, a verdigris bell on its pole, a designer's napkin and a sandbag wall) and its weather
// (flying splinters and pixel sparks). sceneKit.ts spreads the table into `SCENE_KIT`. Each part keeps the kit's
// rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream alone, every
// colour a palette name out of Radiant's gold window, a glow a stack of translucent shapes, and no two sub-paths of
// one shape overlapping but for a hole.

import { between, blob, EDGE, ellipsePath, FAR_EDGE, groundBand, INKED, local, MAIN, MAIN_INKED, pt, shape, sparkle, turned, whole, type Local, type Point } from "./kitDraw.ts";
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

/** A ring about (`dx`, `dy`), from `inner` out to `outer`. */
function ring(f: Local, dx: number, dy: number, outer: number, inner: number): string {
  return f.circle(dx, dy, outer) + f.circle(dx, dy, inner);
}

/** `points` turned `degrees` about the frame's anchor. */
function spun(points: readonly Point[], degrees: number): Point[] {
  return points.map((point) => turned(point, degrees));
}

// Terrain

/** The yard's raked sand from the placement's y down across the box: rake streaks, scuffs and a few pebbles. */
function sand(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  let streaks = "";
  for (let k = 1; k <= 3; k += 1) {
    const y = top + depth * (k / 4) ** 1.3;
    const w = 0.4 + 0.3 * k * at.s;
    streaks += `M${pt(EDGE, y)}Q${pt(50, y + between(rng, -1.5, 1.5))} ${pt(FAR_EDGE, y + between(rng, -1, 1))}L${pt(FAR_EDGE, y + w)}Q${pt(50, y + w)} ${pt(EDGE, y + w)}Z`;
  }
  let scuffs = "";
  for (let k = 0; k < 4; k += 1) {
    const x = between(rng, 6, 94);
    const y = top + depth * between(rng, 0.15, 0.85);
    scuffs += `M${pt(x - 3 * at.s, y)}Q${pt(x, y - 0.8 * at.s)} ${pt(x + 3 * at.s, y)}Q${pt(x, y + 0.8 * at.s)} ${pt(x - 3 * at.s, y)}Z`;
  }
  let pebbles = "";
  for (let k = 0; k < 6; k += 1) {
    const x = between(rng, 4, 96);
    const y = top + depth * between(rng, 0.1, 0.9);
    const r = (0.3 + 0.4 * rng()) * at.s;
    pebbles += ellipsePath(x, y, r, r * 0.6);
  }
  return [
    shape(groundBand(rng, top, 0.8 * at.s, 4), "brass-500", 1, MAIN),
    shape(streaks, "brass-600", 0.35),
    shape(scuffs, "wood-700", 0.2),
    shape(pebbles, "stone-500", 0.7),
  ];
}

/** Three sand lanes running from the placement's y to the box's bottom, narrowing to its x on the horizon, chalk lines between them and boards across. */
function lanes(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const far = 6 * at.s;
  const near = between(rng, 54, 62) * at.s;
  const xAt = (edge: number, y: number): number => at.x + edge * (far + ((near - far) * (y - top)) / depth);
  const strip = (a: number, b: number): string =>
    `M${pt(xAt(a, top), top)}L${pt(xAt(b, top), top)}L${pt(xAt(b, FAR_EDGE), FAR_EDGE)}L${pt(xAt(a, FAR_EDGE), FAR_EDGE)}Z`;
  const edges = [-1, -1 / 3, 1 / 3, 1];
  let chalk = "";
  for (const edge of edges) {
    const w0 = 0.15 * at.s;
    const w1 = 0.9 * at.s;
    chalk += `M${pt(xAt(edge, top) - w0, top)}L${pt(xAt(edge, top) + w0, top)}L${pt(xAt(edge, FAR_EDGE) + w1, FAR_EDGE)}L${pt(xAt(edge, FAR_EDGE) - w1, FAR_EDGE)}Z`;
  }
  let boards = "";
  const rows = whole(rng, 4, 6);
  for (let k = 1; k <= rows; k += 1) {
    const y = top + depth * (k / (rows + 1)) ** 1.5;
    const w = (0.2 + 0.6 * ((y - top) / depth)) * at.s;
    boards += `M${pt(xAt(-1, y), y)}L${pt(xAt(1, y), y)}L${pt(xAt(1, y + w), y + w)}L${pt(xAt(-1, y + w), y + w)}Z`;
  }
  return [
    shape(strip(-1, -1 / 3) + strip(1 / 3, 1), "brass-500", 1, MAIN),
    shape(strip(-1 / 3, 1 / 3), "brass-600", 1),
    shape(boards, "wood-700", 0.3),
    shape(chalk, "white", 0.7),
  ];
}

// Architecture

/** A wall of sharpened logs standing on the anchor, two rails lashed across them; about 52 wide and 15 tall at scale 1. */
function palisade(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 26;
  const logs = 15;
  const w = (2 * half) / logs;
  let body = "";
  let shade = "";
  let tips = "";
  for (let i = 0; i < logs; i += 1) {
    const x0 = -half + i * w + 0.08;
    const x1 = x0 + w - 0.16;
    const h = between(rng, 13, 16);
    const point = h + between(rng, 1.8, 2.8);
    body += f.poly([[x0, 0], [x0, -h], [(x0 + x1) / 2, -point], [x1, -h], [x1, 0]]);
    shade += f.poly([[x1 - w * 0.32, 0], [x1 - w * 0.32, -h], [x1, -h], [x1, 0]]);
    tips += f.poly([[x0 + 0.3, -h + 0.4], [(x0 + x1) / 2, -point + 0.4], [(x0 + x1) / 2, -h + 0.4]]);
  }
  const rail = (y: number): string => rect(f, -half, y - 0.7, half, y + 0.7);
  return [
    shape(body, "wood-700", 1, MAIN_INKED),
    shape(shade, "wood-900", 0.35),
    shape(tips, "wood-600", 0.7),
    shape(rail(-4) + rail(-10.5), "wood-800", 1, INKED),
    shape(f.band([[-half + 3, -4], [-half + 3, -10.5]], 0.5) + f.band([[half - 3, -4], [half - 3, -10.5]], 0.5) + f.band([[0, -4], [0, -10.5]], 0.5), "stone-500", 0.8),
  ];
}

/** A long timber barracks hall standing on the anchor: plank walls, a shingled roof, a door and lit or dark windows; about 36 wide and 22 tall at scale 1. */
function barracks(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const eave = -13 + between(rng, -0.6, 0.6);
  const apex = -22 + between(rng, -1, 1);
  const lit = rng() < 0.6;
  let planks = "";
  for (let x = -16; x < 17; x += 3) planks += f.band([[x, -0.4], [x, eave + 0.6]], 0.25);
  let shingles = "";
  for (let k = 1; k <= 3; k += 1) {
    const y = eave + ((apex - eave) * k) / 4;
    const inset = (k / 4) * 4;
    shingles += f.band([[-19 + inset, y], [19 - inset, y]], 0.35);
  }
  const windows = [-11, -5, 8, 13].map((x) => rect(f, x - 1.6, -9, x + 1.6, -5.4)).join("");
  return [
    shape(rect(f, -17, eave, 17, 0), "wood-700", 1, MAIN_INKED),
    shape(planks, "wood-900", 0.45),
    shape(f.poly([[-19.5, eave + 0.5], [19.5, eave + 0.5], [15, apex], [-15, apex]]), "slate-700", 1, INKED),
    shape(shingles, "slate-800", 0.6),
    shape(f.circle(-8, -7, 4.5) + f.circle(10.5, -7, 4.5), "orange-400", lit ? 0.18 : 0.04),
    shape(windows, lit ? "orange-400" : "slate-900", 0.9, INKED),
    shape(rect(f, 0, -9.6, 4.4, 0), "wood-900", 1, INKED),
    shape(rect(f, -1.4, -0.2, 5.8, 1), "stone-500", 1),
  ];
}

/** A plastered wall with a round moon gate in a carved wooden ring, tiles along its top, its foot on the anchor; about 40 wide and 26 tall at scale 1. */
function moonGate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = between(rng, 9.4, 10.4);
  const cy = -r - 1;
  let notches = "";
  for (let k = 0; k < 8; k += 1) {
    const a = (k * 360) / 8 + 22.5;
    const [x0, y0] = turned([r + 0.5, 0], a);
    const [x1, y1] = turned([r + 1.8, 0], a);
    notches += f.band([[x0, cy + y0], [x1, cy + y1]], 0.45);
  }
  return [
    shape(rect(f, -20, -23, 20, 0) + f.circle(0, cy, r + 2.2), "slate-100", 1, MAIN_INKED),
    shape(ring(f, 0, cy, r + 2.2, r), "wood-700", 1, INKED),
    shape(notches, "wood-900", 0.7),
    shape(f.poly([[-22, -23], [22, -23], [20, -26.5], [-20, -26.5]]), "slate-800", 1, INKED),
    shape(rect(f, -20, -2, 20, 0), "stone-500", 0.6),
  ];
}

/** A round stone well standing on the anchor under a little wooden roof, a rope and its bucket hanging; about 14 wide and 20 tall at scale 1. */
function well(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drop = between(rng, -11, -7);
  let joints = "";
  for (const y of [-2, -4.4]) for (const x of [-4, 0, 4]) joints += f.band([[x - 1.2 + (y < -3 ? 2 : 0), y], [x + 0.6 + (y < -3 ? 2 : 0), y]], 0.3);
  return [
    shape(rect(f, -6.5, -6.4, 6.5, 0), "stone-500", 1, MAIN_INKED),
    shape(joints, "slate-700", 0.5),
    shape(f.ellipse(0, -6.4, 6.5, 1.4), "slate-900", 1, INKED),
    shape(rect(f, -6, -18, -5, -6.4) + rect(f, 5, -18, 6, -6.4), "wood-700", 1, INKED),
    shape(f.poly([[-8, -16.5], [8, -16.5], [0, -21]]), "wood-800", 1, INKED),
    shape(f.band([[0, -16], [0, drop]], 0.3), "stone-500", 0.9),
    shape(f.poly([[-1.4, drop], [1.4, drop], [1.1, drop + 2.2], [-1.1, drop + 2.2]]), "wood-600", 1, INKED),
  ];
}

/** A smithy forge standing on the anchor: a stone hearth and hood rising to its chimney, coals glowing in its mouth, an anvil before it; about 22 wide and 24 tall at scale 1. */
function forge(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const heat = between(rng, 0.7, 1);
  return [
    shape(f.poly([[-9, 0], [9, 0], [9, -9], [4, -15], [3, -24], [-3, -24], [-4, -15], [-9, -9]]), "stone-500", 1, MAIN_INKED),
    shape(f.band([[-7, -3], [-3, -3]], 0.3) + f.band([[2, -6], [6, -6]], 0.3) + f.band([[-2, -18], [1, -18]], 0.3), "slate-700", 0.5),
    shape(f.poly([[-5.5, -1], [5.5, -1], [5.5, -6], [0, -8.4], [-5.5, -6]]), "slate-900", 1),
    shape(blob(f, rng, 0, -2.6, 3.6, 6), "ember-500", heat),
    shape(f.circle(0, -3, 8), "orange-400", 0.2 * heat),
    shape(f.poly([[6, 0], [8, 0], [8.4, -2.6], [10, -3.4], [15.6, -3.4], [14, -4.6], [7, -4.6], [6.4, -3.4]]), "slate-800", 1, INKED),
    shape(f.band([[8, -4.3], [14, -4.3]], 0.3), "slate-400", 0.7),
  ];
}

/** A tall carved pole standing on the anchor, rings and eyes carved down it, wing boards near its top and a platform crowning it; about 12 wide and 42 tall at scale 1. */
function carvedPole(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 38, 42);
  let rings = "";
  let eyes = "";
  for (let k = 1; k <= 4; k += 1) {
    const y = (-h * k) / 5;
    rings += rect(f, -2.6, y - 0.4, 2.6, y + 0.4);
    eyes += f.circle(-1, y - 3.4, 0.6) + f.circle(1, y - 3.4, 0.6);
  }
  return [
    shape(f.poly([[-2.2, 0], [2.2, 0], [1.8, -h], [-1.8, -h]]), "wood-700", 1, MAIN_INKED),
    shape(f.poly([[0.8, 0], [2.2, 0], [1.8, -h], [0.7, -h]]), "wood-900", 0.35),
    shape(rings, "wood-900", 0.75),
    shape(eyes, "red-600", 0.85),
    shape(f.poly([[-2, -h + 8], [-7.5, -h + 5], [-6.5, -h + 9], [-2, -h + 10.5]]) + f.poly([[2, -h + 8], [7.5, -h + 5], [6.5, -h + 9], [2, -h + 10.5]]), "wood-600", 1, INKED),
    shape(f.poly([[-5.5, -h], [5.5, -h], [4.6, -h + 1.3], [-4.6, -h + 1.3]]), "wood-800", 1, INKED),
  ];
}

/** A carved wooden siege tower standing on its wheels at the anchor, tapering up to a crenellated top with its drawbridge; about 18 wide and 36 tall at scale 1. */
function siegeTower(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = -34 + between(rng, -1, 1);
  const bridge = between(rng, 0, 1);
  let planks = "";
  for (let y = -6; y > top + 3; y -= 4) planks += f.band([[-7.6 - (y / top) * -2.4, y], [7.6 + (y / top) * -2.4, y]], 0.3);
  return [
    shape(f.poly([[-8, -2], [8, -2], [5.5, top], [-5.5, top]]), "wood-700", 1, MAIN_INKED),
    shape(planks, "wood-900", 0.5),
    shape(f.band([[-7.4, -4], [5, top + 2]], 0.6) + f.band([[7.4, -4], [-5, top + 2]], 0.6), "wood-800", 0.6),
    shape(f.poly([[-6.5, top], [6.5, top], [6.5, top - 3], [4.2, top - 3], [4.2, top - 1.5], [1.1, top - 1.5], [1.1, top - 3], [-1.1, top - 3], [-1.1, top - 1.5], [-4.2, top - 1.5], [-4.2, top - 3], [-6.5, top - 3]]), "wood-600", 1, INKED),
    shape(f.poly([[5.6, top + 4], [5.6 + 6 * bridge, top + 4 - 6 * (1 - bridge)], [6.4 + 6 * bridge, top + 5 - 6 * (1 - bridge)], [6.2, top + 5.5]]), "wood-600", 1, INKED),
    shape(ring(f, -5, -2, 2.6, 1.1) + ring(f, 5, -2, 2.6, 1.1), "wood-900", 1, INKED),
    shape(f.circle(-5, -2, 0.7) + f.circle(5, -2, 0.7), "stone-500", 1),
  ];
}

/** A ramshackle mill shed standing on the anchor, a creaking conveyor climbing into it from the left and a chimney; about 34 wide and 22 tall at scale 1. */
function mill(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.6, 0.6);
  let boards = "";
  for (let x = -5; x < 13; x += 3) boards += f.band([[x, -0.4], [x + lean * 0.3, -12.4]], 0.25);
  return [
    shape(f.poly([[-6, 0], [14, 0], [14 + lean, -13], [-6 + lean, -13]]), "wood-700", 1, MAIN_INKED),
    shape(boards, "wood-900", 0.45),
    shape(f.poly([[-8, -12.5], [16, -12.5], [12, -18], [-4, -18]]), "wood-800", 1, INKED),
    shape(rect(f, 8, -22, 11, -15), "stone-500", 1, INKED),
    shape(rect(f, -6, -9, -1, -3), "slate-900", 1),
    shape(rect(f, 6, -9, 10, -5.5), "green-400", 0.85, INKED),
    shape(f.poly([[-21, -1], [-6, -7.4], [-6, -5.6], [-21, 0.8]]), "slate-800", 1, INKED),
    shape(f.circle(-19, -0.8, 0.9) + f.circle(-13.5, -3.2, 0.9) + f.circle(-8, -5.5, 0.9), "stone-500", 1),
  ];
}

/** An obstacle course's cargo net hung between two posts under a beam, sagging, its foot on the anchor; about 22 wide and 24 tall at scale 1. */
function cargoNet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 1, 2.4);
  const cols = [-7.5, -4.5, -1.5, 1.5, 4.5, 7.5];
  const rows = [-20, -16, -12, -8, -4];
  const xAt = (x: number, y: number): number => x + Math.sin(((y + 22) / 22) * Math.PI) * sag * (x / 8);
  let down = "";
  for (const x of cols) down += f.band(rows.map((y): Point => [xAt(x, y), y]).concat([[xAt(x, -1), -1]]), 0.45);
  let across = "";
  for (const y of rows) across += f.band(cols.map((x): Point => [xAt(x, y), y + sag * 0.3 * (1 - (x / 8) ** 2)]), 0.4);
  return [
    shape(rect(f, -11, -24, -9.6, 0) + rect(f, 9.6, -24, 11, 0) + rect(f, -12, -24.8, 12, -22.6), "wood-700", 1, MAIN_INKED),
    shape(down, "stone-500", 0.95),
    shape(across, "stone-500", 0.8),
  ];
}

/** A covered walkway standing on the anchor: posts under a low beam and a shingled roof, a lever on its near post; about 30 wide and 16 tall at scale 1. */
function walkway(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const beam = -11 + between(rng, -0.6, 0.6);
  const pull = between(rng, -20, 20);
  const [lx, ly] = turned([0, -4], pull);
  return [
    shape(f.poly([[-16, beam - 1.2], [16, beam - 1.2], [13, beam - 5.6], [-13, beam - 5.6]]), "slate-700", 1, INKED),
    shape(rect(f, -15, beam - 1.4, 15, beam + 0.6), "wood-700", 1, MAIN_INKED),
    shape(rect(f, -14, beam, -12.6, 0) + rect(f, -1, beam, 0.4, 0) + rect(f, 12.6, beam, 14, 0), "wood-700", 1, INKED),
    shape(f.band([[13.3, -5], [13.3 + lx, -5 + ly]], 0.5), "slate-800", 1),
    shape(f.circle(13.3 + lx, -5 + ly, 0.8), "red-600", 1, INKED),
  ];
}

/** A plank bench standing on the anchor; about 14 wide and 5 tall at scale 1. */
function bench(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.3, 0.3);
  return [
    shape(rect(f, -6, -3.4, -4.8, 0) + rect(f, 4.8, -3.4, 6, 0), "wood-800", 1, INKED),
    shape(f.poly([[-7, -3.4 + tilt], [7, -3.4 - tilt], [7, -4.8 - tilt], [-7, -4.8 + tilt]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-6.6, -3.6 + tilt], [6.6, -3.6 - tilt]], 0.3), "wood-900", 0.5),
  ];
}

// Creatures

/** A towering dinosaur rearing on the anchor mid-roar, facing the way it faces, jaws wide, tiny arms, rags of a tunic burst on its back; about 36 wide and 34 tall at scale 1. */
function dino(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const roar = between(rng, 0, 1.6);
  const swing = between(rng, -1.5, 1.5);
  const leg = (dx: number): string => f.poly([[dx - 5, -15], [dx + 1, -14], [dx + 2, -8], [dx + 0.4, -1.4], [dx + 3.6, 0], [dx - 3, 0], [dx - 3.4, -7], [dx - 6, -11]]);
  let teeth = "";
  for (const x of [10.5, 12.5, 14.5]) teeth += f.poly([[x, -30 - (x - 9) * 0.2], [x + 0.6, -28.6 - (x - 9) * 0.2], [x + 1.2, -30.2 - (x - 9) * 0.2]]);
  for (const x of [10.5, 12.5]) teeth += f.poly([[x, -26.6 + roar * (x - 9) * 0.2], [x + 0.6, -28 + roar * (x - 9) * 0.2], [x + 1.2, -26.4 + roar * (x - 9) * 0.2]]);
  return [
    shape(leg(-8), "green-900", 1, INKED),
    shape(f.poly([[-20, -4 + swing], [-12, -11.5], [-7, -17], [-2, -23], [4, -25], [7.5, -27], [9, -22], [8, -17], [4, -12.5], [-2, -11], [-9, -9.5]]), "green-700", 1, MAIN_INKED),
    shape(leg(0), "green-700", 1, MAIN_INKED),
    shape(f.poly([[9, -28.5], [16, -31], [15.5, -25 + roar], [9, -27.4]]), "red-900", 1),
    shape(f.poly([[6, -27], [7.4, -33], [13.5, -33.6], [16.6, -31.4], [16, -30.4], [9.4, -28.6]]) + f.poly([[8.8, -27.4], [15.6, -24.6 + roar], [14.6, -23.4 + roar], [7.6, -25.4]]), "green-700", 1, MAIN_INKED),
    shape(teeth, "white", 1),
    shape(f.circle(10, -31.4, 0.9), "white", 1),
    shape(f.circle(10.2, -31.4, 0.4), "black", 1),
    shape(f.band([[6.5, -19], [9.6, -17], [10.4, -18.6]], 0.9), "green-900", 1, INKED),
    shape(f.poly([[-11, -11.6], [-9.6, -14.6], [-8.4, -13.8], [-6, -18], [-4.6, -17], [-2.4, -21.6], [-1, -20.4]]), "green-900", 0.9),
    shape(f.poly([[-3, -12], [6.4, -14], [5, -18], [3, -15.4]]), "lime-400", 0.35),
    shape(f.poly([[-1, -23.6], [3, -24.6], [2.4, -22], [0.6, -22.8], [-0.4, -21.4]]) + f.poly([[-7.6, -16.6], [-4.6, -19], [-4.8, -16.8]]), "blue-700", 1, INKED),
  ];
}

// Plants

/** A tumbleweed bowling along on the anchor: a ball of tangled twigs and its dust; about 8 across at scale 1. */
function tumbleweed(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let loops = "";
  for (let k = 0; k < 4; k += 1) {
    const a = between(rng, 0, 180) + k * 45;
    const pts: Point[] = [];
    for (let i = 0; i <= 6; i += 1) {
      const [x, y] = turned([3.6 * Math.cos((i * Math.PI) / 3), 1.6 * Math.sin((i * Math.PI) / 3)], a);
      pts.push([x, y - 4]);
    }
    loops += f.band(pts, 0.3);
  }
  return [
    shape(f.circle(0, -4, 3.4), "wood-700", 0.45, MAIN),
    shape(loops, "wood-600", 0.9),
    shape(f.ellipse(-4, -0.6, 3, 0.8), "stone-500", 0.35),
  ];
}

// Objects

/** A throwing knife in flight centred on the anchor, turned by its stream, a faint streak behind it; about 9 long at scale 1. */
function knife(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -30, 30);
  const P = (points: readonly Point[]): string => f.poly(spun(points, a));
  return [
    shape(P([[-12, -0.4], [-4.5, -0.2], [-4.5, 0.2], [-12, 0.4]]), "slate-100", 0.25),
    shape(P([[-0.6, -0.9], [4.6, -0.5], [6, 0], [4.6, 0.5], [-0.6, 0.9]]), "slate-100", 1, MAIN_INKED),
    shape(P([[0, -0.2], [5, -0.1], [5.6, 0], [0, 0.1]]), "white", 0.7),
    shape(P([[-1.2, -1.6], [-0.4, -1.6], [-0.4, 1.6], [-1.2, 1.6]]), "slate-700", 1),
    shape(P([[-4.6, -0.6], [-1.2, -0.6], [-1.2, 0.6], [-4.6, 0.6]]), "wood-800", 1, INKED),
  ];
}

/** A straw sparring dummy on its post, its foot at the anchor: a crossbar for arms, a stuffed sack body tied with rope, a sack head with stitched eyes and a red patch for a heart; about 12 wide and 24 tall at scale 1. */
function strawDummy(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.08, 0.08);
  const P = (points: readonly Point[]): string => f.poly(points.map(([x, y]): Point => [x - lean * y, y]));
  const L = (x: number, y: number): Point => [x - lean * y, y];
  return [
    shape(P([[-0.7, 0], [0.7, 0], [0.7, -12], [-0.7, -12]]), "wood-700", 1, INKED),
    shape(P([[-6, -17.6], [6, -17.6], [6, -16.2], [-6, -16.2]]), "wood-700", 1, INKED),
    shape(P([[-3.6, -18], [3.6, -18], [4.2, -12], [2.4, -10], [-2.4, -10], [-4.2, -12]]), "stone-500", 1, MAIN_INKED),
    shape(f.band([L(-6, -16.9), L(-7.6, -15.4)], 0.6) + f.band([L(6, -16.9), L(7.6, -15.6)], 0.6) + f.band([L(-1.4, -10), L(-2, -8.6)], 0.5) + f.band([L(1.4, -10), L(2, -8.8)], 0.5), "brass-500", 0.95),
    shape(f.band([L(-3.9, -12.6), L(3.9, -12.6)], 0.5) + f.band([L(-0.8, -18.2), L(0.8, -18.2)], 0.9), "wood-800", 1),
    shape(f.circle(...L(0, -20.6), 2.8), "stone-500", 1, MAIN_INKED),
    shape(f.band([L(-1.6, -21.6), L(-0.6, -20.6)], 0.3) + f.band([L(-0.6, -21.6), L(-1.6, -20.6)], 0.3) + f.band([L(0.6, -21.6), L(1.6, -20.6)], 0.3) + f.band([L(1.6, -21.6), L(0.6, -20.6)], 0.3), "wood-900", 0.9),
    shape(f.circle(...L(-1.2, -15), 1), "red-600", 0.9),
  ];
}

/** A faceless iron-banded oak block, its foot at the anchor: grain, two iron bands and rivets; about 16 wide and 12 tall at scale 1. */
function oakBlock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 7.4, 8.4);
  let grain = "";
  for (const y of [-2.6, -5.4, -8.6]) grain += f.band([[-w + 1, y + between(rng, -0.5, 0.5)], [w - 1, y]], 0.25);
  let rivets = "";
  for (const x of [-w + 1.4, w - 1.4]) for (const y of [-3.6, -8.6]) rivets += f.circle(x, y, 0.45);
  return [
    shape(rect(f, -w, -12, w, 0), "wood-600", 1, MAIN_INKED),
    shape(f.poly([[-w, -12], [w, -12], [w - 1.4, -13.6], [-w + 1.4, -13.6]]), "wood-700", 1, INKED),
    shape(rect(f, w - 2.6, -12, w, 0), "wood-800", 0.4),
    shape(grain, "wood-800", 0.5),
    shape(rect(f, -w, -4.4, w, -2.8) + rect(f, -w, -9.4, w, -7.8), "slate-700", 1, INKED),
    shape(rivets, "slate-400", 1),
  ];
}

/** A scorched rune lying on the ground centred on the anchor, flaring red: a burnt patch, its wordless strokes and their glow; about 14 wide and 4 tall at scale 1. */
function glyphMark(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const kind = whole(rng, 0, 2);
  const flat = (points: readonly Point[]): Point[] => points.map(([x, y]): Point => [x, y * 0.36]);
  const strokes = [
    f.band(flat([[-4.4, 4], [0, -5], [4.4, 4], [-4.4, 4]]), 0.9),
    f.band(flat([[-5.4, 0], [0, -5], [5.4, 0], [0, 5], [-5.4, 0]]), 0.9),
    f.band(flat([[-5.4, -4], [5.4, 4]]), 0.9) + f.band(flat([[-5.4, 4], [-1.2, 1]]), 0.9) + f.band(flat([[1.2, -1], [5.4, -4]]), 0.9),
  ];
  return [
    shape(f.ellipse(0, 0, 7.6, 2.6), "black", 0.55),
    shape(f.ellipse(0, 0, 6.4, 2.2), "ember-500", 0.25),
    shape(strokes[kind] ?? "", "red-500", 1),
    shape(strokes[kind] ?? "", "orange-400", 0.5),
    shape(f.circle(0, 0, 0.8), "white", 0.8),
  ];
}

/** A welcome mat lying on the ground on the anchor, something lumped under it and a lit fuse curling out from beneath; about 14 wide and 4 tall at scale 1. */
function fuseMat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -1, 1);
  const end: Point = [12 + curl, -3 + curl * 0.5];
  return [
    shape(f.poly([[-7, 0.6], [6, 0.6], [8, -2.6], [-5, -2.6]]), "red-900", 1, MAIN_INKED),
    shape(f.poly([[-5.4, -0.1], [5.4, -0.1], [6.8, -2], [-4, -2]]) + f.poly([[-4.4, -0.6], [4.8, -0.6], [5.9, -1.5], [-3.2, -1.5]]), "wood-800", 0.6),
    shape(f.ellipse(0.4, -1.4, 3, 0.9), "wood-900", 0.35),
    shape(f.band([[6.6, -0.6], [9, 0.4], [11, -1], end], 0.4), "slate-800", 1),
    shape(f.circle(end[0], end[1], 2.2), "orange-400", 0.3),
    shape(sparkle(f, end[0], end[1], 1.6), "white", 1),
  ];
}

/** A wooden runner token mid-sprint on the anchor, facing the way it faces: a round wooden head on a peg body, legs striding, speed lines behind; about 6 wide and 10 tall at scale 1. */
function runnerToken(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stride = between(rng, 1.4, 2.4);
  return [
    shape(f.ellipse(-3, -0.4, 3, 0.8), "stone-500", 0.35),
    shape(f.band([[0, -4], [stride, 0]], 0.9) + f.band([[-0.4, -4], [-stride - 0.4, -0.8]], 0.9), "wood-800", 1),
    shape(f.poly([[-1.8, -3.6], [1.8, -3.6], [1.2, -7.4], [-1.2, -7.4]]), "wood-600", 1, MAIN_INKED),
    shape(f.circle(0.4, -8.8, 1.8), "wood-600", 1, MAIN_INKED),
    shape(f.circle(0.4, -8.8, 0.8), "wood-700", 0.6),
    shape(f.band([[-3, -6], [-6, -6]], 0.3) + f.band([[-3, -8.4], [-5.4, -8.4]], 0.3) + f.band([[-3.2, -4], [-6.4, -4]], 0.3), "white", 0.55),
  ];
}

/** A wordless recruiting poster pinned up, its foot at the anchor: a flexing arm or a pointing hand drawn on it, a border and a curled corner; about 8 wide and 11 tall at scale 1. */
function poster(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -6, 6);
  const P = (points: readonly Point[]): string => f.poly(spun(points, tilt));
  const arm = rng() < 0.5;
  const glyph = arm
    ? f.band(spun([[-2, -2], [0.4, -2.4], [1, -5.6]], tilt), 1.1) + f.ellipse(...turned([-0.6, -3.3], tilt), 1.6, 1)
    : f.circle(...turned([0, -5.5], tilt), 2) + f.band(spun([[0, -7.4], [0, -9.4]], tilt), 0.9);
  return [
    shape(P([[-4, 0], [4, 0], [4, -11], [-4, -11]]), "slate-100", 1, MAIN_INKED),
    shape(P([[-3.3, -0.7], [3.3, -0.7], [3.3, -10.3], [-3.3, -10.3]]) + P([[-2.8, -1.2], [2.8, -1.2], [2.8, -9.8], [-2.8, -9.8]]), arm ? "red-600" : "blue-700", 0.9),
    shape(glyph, arm ? "red-600" : "blue-700", 1),
    shape(P([[4, 0], [2.4, 0], [4, -1.6]]), "stone-500", 1),
    shape(f.circle(...turned([-3.2, -10.2], tilt), 0.4) + f.circle(...turned([3.2, -10.2], tilt), 0.4), "slate-800", 1),
  ];
}

/** A training catapult on four wheels, its foot at the anchor: a timber frame, its throwing arm and bucket, a rope; about 22 wide and 16 tall at scale 1. */
function catapult(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cock = between(rng, -10, 10);
  const [ax, ay] = turned([-12, 0], 40 + cock);
  return [
    shape(f.poly([[-10, -2.4], [10, -2.4], [10, -4.4], [-10, -4.4]]) + f.poly([[2, -4.4], [4, -4.4], [3.6, -11], [2.4, -11]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[3, -9.6], [3 + ax, -9.6 + ay]], 1), "wood-600", 1, INKED),
    shape(f.circle(3 + ax, -9.6 + ay - 1.2, 2), "wood-800", 1, INKED),
    shape(f.band([[3, -9.6], [9, -4.4]], 0.3) + f.band([[-0.6 + ax * 0.4, -9.6 + ay * 0.4], [-8, -4.4]], 0.3), "stone-500", 0.85),
    shape(ring(f, -7, -2.4, 2.4, 1) + ring(f, 7, -2.4, 2.4, 1), "wood-900", 1, INKED),
    shape(f.circle(-7, -2.4, 0.6) + f.circle(7, -2.4, 0.6), "stone-500", 1),
  ];
}

/** A pale jelly bean lying tilted on the anchor, a glint on it and its glow; about 5 wide and 3 tall at scale 1. */
function jellyBean(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -25, 25);
  const bean: Point[] = [];
  for (let i = 0; i < 12; i += 1) {
    const a = (i * Math.PI) / 6;
    const pinch = Math.sin(a) < 0 ? 1 - 0.35 * Math.sin(a) ** 4 : 1;
    bean.push(turned([2.6 * Math.cos(a), -1.5 + 1.4 * Math.sin(a) * pinch], tilt));
  }
  return [
    shape(f.circle(0, -1.5, 4.4), "white", 0.22),
    shape(f.poly(bean), "lime-200", 1, MAIN_INKED),
    shape(f.ellipse(...turned([0.4, -0.7], tilt), 1.6, 0.5), "lime-400", 0.45),
    shape(f.ellipse(...turned([-0.9, -2.2], tilt), 0.9, 0.35), "white", 0.85),
  ];
}

/** A verdigris bell hung from a tall pole's arm, ringing, the pole's foot at the anchor; about 10 wide and 32 tall at scale 1. */
function bell(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -12, 12);
  const P = (points: readonly Point[]): string => f.poly(points.map((point) => turned(point, swing)).map(([x, y]): Point => [x + 4, y - 27]));
  let waves = "";
  for (const r of [5, 7]) waves += `M${f.p(4 + r * 0.7, -24 - r * 0.7)}Q${f.p(4 + r * 1.05, -24)} ${f.p(4 + r * 0.7, -24 + r * 0.7)}`.concat(`L${f.p(4 + r * 0.7 + 0.4, -24 + r * 0.7)}Q${f.p(4 + r * 1.05 + 0.4, -24)} ${f.p(4 + r * 0.7 + 0.4, -24 - r * 0.7)}Z`);
  return [
    shape(rect(f, -0.8, -31, 0.8, 0), "wood-700", 1, INKED),
    shape(rect(f, -0.8, -31, 6, -29.8), "wood-800", 1, INKED),
    shape(P([[-0.4, 0], [0.4, 0], [0.4, 1], [-0.4, 1]]), "slate-800", 1),
    shape(P([[-1.6, 1], [1.6, 1], [2.4, 4], [3.6, 6.4], [-3.6, 6.4], [-2.4, 4]]), "jade-400", 1, MAIN_INKED),
    shape(P([[-3.8, 6], [3.8, 6], [3.8, 6.9], [-3.8, 6.9]]), "green-700", 1),
    shape(P([[-0.6, 6.6], [0.6, 6.6], [0.6, 8], [-0.6, 8]]), "slate-800", 1),
    shape(waves, "white", 0.45),
  ];
}

/** A paper napkin lying on a counter on the anchor, a card's frame sketched on it, a coffee ring beside it and a pencil stub; about 14 wide and 5 tall at scale 1. */
function napkin(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const roll = between(rng, -1, 1);
  return [
    shape(f.poly([[-6.5, 1], [5, 1.4], [6.6, -3.4], [-5, -3.8]]), "white", 1, MAIN_INKED),
    shape(f.band([[-2.6, 0], [1.6, 0.2], [2.2, -2.6], [-2, -2.8], [-2.6, 0]], 0.25) + f.band([[-0.8, -0.6], [0.4, -2]], 0.2), "slate-700", 0.75),
    shape(f.ellipse(-9.6, -0.6, 2.4, 1) + f.ellipse(-9.6, -0.6, 1.8, 0.6), "wood-700", 0.5),
    shape(f.band([[3 + roll, 0.6], [8.4 + roll, -1.8]], 0.7), "orange-400", 1, INKED),
    shape(f.poly([[3 + roll, 0.95], [2 + roll, 1.1], [3 + roll, 0.25]]), "slate-800", 1),
    shape(f.band([[8.1 + roll, -1.66], [8.8 + roll, -1.98]], 0.7), "red-300", 1),
  ];
}

/** A wall of sandbags stacked in rows on the anchor, seams and ties on each; about 16 wide and 7 tall at scale 1. */
function sandbags(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let bags = "";
  let seams = "";
  const rows = whole(rng, 2, 3);
  for (let r = 0; r < rows; r += 1) {
    const n = 4 - r;
    const w = 4;
    const y = -r * 2.6;
    for (let i = 0; i < n; i += 1) {
      const x = -((n - 1) * w) / 2 + i * w;
      bags += f.ellipse(x, y - 1.3, 1.9, 1.25);
      seams += f.band([[x - 1.2, y - 1.3], [x + 1.2, y - 1.3]], 0.2);
    }
  }
  return [
    shape(f.ellipse(0, 0, 9, 1.2), "slate-700", 0.3),
    shape(bags, "stone-500", 1, MAIN_INKED),
    shape(seams, "wood-800", 0.5),
  ];
}

// Weather

/** A burst of splinters flying out from the anchor, within about 12 at scale 1, and a puff of dust. */
function splinters(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let light = "";
  let dark = "";
  for (let k = 0; k < 10; k += 1) {
    const a = (k * 360) / 10 + between(rng, -14, 14);
    const r0 = between(rng, 3, 6);
    const r1 = r0 + between(rng, 2.5, 5);
    const [x0, y0] = turned([r0, 0], a);
    const [x1, y1] = turned([r1, 0], a);
    const [nx, ny] = turned([0, 0.35], a);
    const sliver = f.poly([[x0 - nx, y0 - ny], [x1, y1], [x0 + nx, y0 + ny]]);
    if (k % 2 === 0) light += sliver;
    else dark += sliver;
  }
  return [shape(blob(f, rng, 0, 0, 5, 7), "stone-500", 0.15), shape(light, "wood-600", 1, INKED), shape(dark, "wood-800", 1)];
}

/** Pixel sparks over a field about 30 wide and 20 tall centred on the anchor: cyan and magenta squares, white glints, a faint violet shimmer. */
function pixels(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const squares = (count: number, size: number): string => {
    let d = "";
    for (let k = 0; k < count; k += 1) {
      const x = between(rng, -15, 15);
      const y = between(rng, -10, 10);
      d += rect(f, x, y, x + size, y + size);
    }
    return d;
  };
  let shimmer = "";
  for (let k = 0; k < 3; k += 1) shimmer += sparkle(f, between(rng, -12, 12), between(rng, -8, 8), 2.4);
  return [
    shape(shimmer, "violet-300", 0.35),
    shape(squares(7, 1.1), "cyan-400", 0.9),
    shape(squares(6, 1), "purple-400", 0.9),
    shape(squares(4, 0.6), "white", 0.8),
  ];
}

/** A training log lying level, centred on the anchor: bark, a pale cut end with its rings, a knot and two rope grips; about 18 long and 4 thick at scale 1. */
function log(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 8.4, 9.6);
  return [
    shape(rect(f, -w, -2, w, 2), "wood-700", 1, MAIN_INKED),
    shape(rect(f, -w, 0.6, w, 2), "wood-900", 0.35),
    shape(f.band([[-w + 2, -0.8], [w - 3, -0.6]], 0.25) + f.circle(between(rng, -3, 3), 0.4, 0.6), "wood-900", 0.6),
    shape(f.ellipse(w, 0, 1.2, 2), "wood-600", 1, INKED),
    shape(ring(f, w, 0, 0.9, 0.5), "wood-800", 0.6),
    shape(rect(f, -w * 0.5 - 0.6, -2.2, -w * 0.5 + 0.6, 2.2) + rect(f, w * 0.5 - 0.6, -2.2, w * 0.5 + 0.6, 2.2), "brass-600", 1),
  ];
}

/** A wooden toll pail on the anchor heaped with silver coins, a rope handle and iron hoops, one coin dropped beside it; about 9 wide and 8 tall at scale 1. */
function pail(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rim = -6.4;
  const coins = (row: readonly (readonly [number, number])[]): string => row.map(([x, y]) => f.ellipse(x, rim + y + between(rng, -0.3, 0.3), 1.1, 0.6)).join("");
  return [
    shape(f.ellipse(0, 0.3, 5.4, 0.8), "black", 0.22),
    shape(f.band([[-4, rim], [-2.6, rim - 4.4], [2.6, rim - 4.4], [4, rim]], 0.4), "wood-800", 1),
    shape(coins([[-2.4, 0], [0, -0.4], [2.4, 0]]), "slate-400", 1, INKED),
    shape(coins([[-1.2, -1.5], [1.2, -1.6]]), "slate-400", 1, INKED),
    shape(f.poly([[-4.2, rim], [4.2, rim], [3.4, 0], [-3.4, 0]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-4, rim + 1.4], [4, rim + 1.4]], 0.6) + f.band([[-3.5, -1.3], [3.5, -1.3]], 0.6), "slate-700", 1, INKED),
    shape(f.ellipse(6, -0.2, 1.3, 0.6) + sparkle(f, 0.6, rim - 1.8, 1.1), "slate-100", 0.9),
  ];
}

/** The yard's main gate burst open, its foot at the anchor: two timber posts under a lintel beam, one door leaf hanging askew and one knocked flat, a cracked plank and iron straps; about 30 wide and 28 tall at scale 1. */
function yardGate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hang = between(rng, 14, 24);
  const leaf = (points: readonly Point[]): string => f.poly(spun(points, hang).map(([x, y]): Point => [x - 11, y - 24]));
  return [
    shape(rect(f, -15, -28, -11, 0) + rect(f, 11, -28, 15, 0), "wood-800", 1, MAIN_INKED),
    shape(rect(f, -16, -30, 16, -26), "wood-700", 1, MAIN_INKED),
    shape(leaf([[0, 0], [9, 0], [9, 22], [0, 22]]), "wood-600", 1, INKED),
    shape(leaf([[0, 4], [9, 4], [9, 5.4], [0, 5.4]]) + leaf([[0, 16], [9, 16], [9, 17.4], [0, 17.4]]), "slate-700", 1),
    shape(f.poly([[3, 0], [14, -1.4], [14.6, 0.4], [3.4, 1.6]]), "wood-600", 1, INKED),
    shape(leaf([[3, 7], [5, 10.4], [4, 10.4], [6, 14], [2.6, 9.4], [3.6, 9.4]]), "wood-900", 0.7),
  ];
}

/** A plank lane torn up and dragged like a rug, lying from the anchor back the way it faces away from, rucked up in the middle, a chalk line down it and a rope rising from its front end to a dragger's hand; about 40 long at scale 1. */
function plankRug(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = between(rng, -24, -18);
  const h = between(rng, 6, 9);
  const top = `M${f.p(-40, -1.8)}L${f.p(c - 7, -1.8)}Q${f.p(c, -h * 2)} ${f.p(c + 7, -2)}L${f.p(-1, -3.2)}L${f.p(-1, 0.8)}L${f.p(c + 7, 1.4)}Q${f.p(c, -h * 2 + 4)} ${f.p(c - 7, 1.6)}L${f.p(-40, 1.8)}Z`;
  const seams = [-36, -31, c - 10, c + 11, c + 16].map((x) => f.band([[x, -1.8], [x - 0.3, 1.6]], 0.25)).join("");
  return [
    shape(f.ellipse(-20, 2, 21, 1.4), "black", 0.2),
    shape(top, "wood-600", 1, MAIN_INKED),
    shape(`M${f.p(c, -h + 0.2)}Q${f.p(c + 3.5, -h * 0.5 - 1)} ${f.p(c + 7, -2)}L${f.p(c + 7, 1.4)}Q${f.p(c + 3.5, -h * 0.5 + 1.4)} ${f.p(c, -h + 2.2)}Z`, "wood-900", 0.4),
    shape(seams, "wood-900", 0.6),
    shape(f.band([[-39, 0], [c - 8, 0]], 0.3) + f.band([[c + 8, -0.3], [-2, -1.1]], 0.3), "white", 0.5),
    shape(f.band([[-0.6, -1.4], [3, -6.4], [6, -12]], 0.4), "wood-900", 1),
    shape(f.circle(-41.6, 0, 1.6) + f.circle(-38.6, -2.6, 1.1), "stone-500", 0.4),
  ];
}

/** A shimmering shield dome blooming over the anchor: a translucent half-bubble about 18 wide and 13 tall, a brighter rim, its ring on the ground, facet lines and a glint. */
function shieldDome(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 8.4, 9.6);
  const h = between(rng, 11.5, 13.5);
  const dome = (rx: number, ry: number): string => `M${f.p(-rx, 0)}A${f.r(rx)} ${f.r(ry)} 0 0 ${f.sweep(1)} ${f.p(rx, 0)}Z`;
  return [
    shape(dome(w, h), "cyan-400", 0.16),
    shape(dome(w, h) + dome(w - 0.8, h - 0.8), "cyan-400", 0.55),
    shape(f.ellipse(0, 0, w, 1.6), "cyan-400", 0.3),
    shape(f.band([[-w * 0.5, -h * 0.25], [0, -h * 0.5], [w * 0.5, -h * 0.25]], 0.2) + f.band([[0, -h * 0.5], [0, -h * 0.9]], 0.2), "white", 0.3),
    shape(f.band([[-w * 0.66, -h * 0.4], [-w * 0.48, -h * 0.66], [-w * 0.2, -h * 0.85]], 0.45), "white", 0.45),
    shape(sparkle(f, -w * 0.42, -h * 0.72, 1.4), "white", 0.85),
  ];
}

/** A spear snapped in two: its butt half planted aslant in the ground on the anchor, splintered at the top, and its head half lying beside it, a red binding at the steel head; about 20 wide and 12 tall at scale 1. */
function brokenSpear(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 12, 24);
  const jag = (x: number, y: number): Point[] => [[x, y - 0.5], [x + 1.2, y - 0.9], [x + 0.6, y - 0.2], [x + 1.6, y + 0.1], [x + 0.6, y + 0.4], [x, y + 0.4]];
  return [
    shape(f.ellipse(-5, 0.5, 11, 1.1), "black", 0.2),
    shape(f.band([turned([0, 0.6], tilt), turned([0, -11], tilt)], 0.9), "wood-700", 1, INKED),
    shape(f.poly(spun([[-0.5, -11], [-0.3, -13], [0, -11.6], [0.3, -13.6], [0.5, -11]], tilt)), "wood-600", 1),
    shape(f.band([[-14, -0.6], [-3.6, -0.7]], 0.9), "wood-700", 1, INKED),
    shape(f.poly(jag(-3.6, -0.7)), "wood-600", 1),
    shape(f.poly([[-14, -1.7], [-18.8, -0.6], [-14, 0.5]]), "slate-100", 1, INKED),
    shape(f.band([[-13.3, -1.5], [-13.3, 0.3]], 1.1), "red-600", 1),
    shape(f.band([[-14.4, -0.9], [-17.4, -0.6]], 0.25), "white", 0.6),
  ];
}

/** R3393000: the Training Grounds parts, by id, each with its kind. */
export const YARD_KIT = {
  "yard-sand": { category: "terrain", draw: sand },
  lanes: { category: "terrain", draw: lanes },
  palisade: { category: "architecture", draw: palisade },
  barracks: { category: "architecture", draw: barracks },
  "moon-gate": { category: "architecture", draw: moonGate },
  "yard-well": { category: "architecture", draw: well },
  "smithy": { category: "architecture", draw: forge },
  "carved-pole": { category: "architecture", draw: carvedPole },
  "siege-tower": { category: "architecture", draw: siegeTower },
  mill: { category: "architecture", draw: mill },
  "cargo-net": { category: "architecture", draw: cargoNet },
  walkway: { category: "architecture", draw: walkway },
  "plank-bench": { category: "architecture", draw: bench },
  dino: { category: "creatures", draw: dino },
  tumbleweed: { category: "plants", draw: tumbleweed },
  knife: { category: "objects", draw: knife },
  "straw-dummy": { category: "objects", draw: strawDummy },
  "oak-block": { category: "objects", draw: oakBlock },
  "glyph-mark": { category: "objects", draw: glyphMark },
  "fuse-mat": { category: "objects", draw: fuseMat },
  "runner-token": { category: "objects", draw: runnerToken },
  poster: { category: "objects", draw: poster },
  catapult: { category: "objects", draw: catapult },
  "pale-bean": { category: "objects", draw: jellyBean },
  "bell-pole": { category: "objects", draw: bell },
  napkin: { category: "objects", draw: napkin },
  sandbags: { category: "objects", draw: sandbags },
  splinters: { category: "weather", draw: splinters },
  pixels: { category: "weather", draw: pixels },
  log: { category: "objects", draw: log },
  "yard-gate": { category: "architecture", draw: yardGate },
  pail: { category: "objects", draw: pail },
  "plank-rug": { category: "objects", draw: plankRug },
  "shield-dome": { category: "weather", draw: shieldDome },
  "broken-spear": { category: "objects", draw: brokenSpear },
} as const satisfies Readonly<Record<string, KitPart>>;
