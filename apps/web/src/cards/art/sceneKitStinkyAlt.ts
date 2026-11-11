// R3406000: the scene kit's parts that Stinkyscape's later Alt art pass (#3406) added (ART.md, "Stinkyscape's later
// Alt art restaged"), so a Starship Piece's, a Jlockeed weapon's and Dice of Doom's Alt art draws its own piece in a
// scene of the kit and a brief's subject is no stand-in: a swarm of nanobots, a hull plate, a cartridge, an overdrive
// engine, a warp ring, a rail gun, a great cannon, a missile pod, a capacitor cell, a flank pod, a launcher tube, a
// hundred-sided die, a pineapple, an escalator and a seashell. sceneKit.ts spreads them into `SCENE_KIT` after every
// other module. Every colour is a palette.css name, none in Radiant's gold window; a glow is a stack of translucent
// shapes, never a gradient; shapes are filled even-odd, so a hole is a second sub-path and no two sub-paths of one
// shape overlap otherwise.

import { between, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, sparkle, spiralArm, turned, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** What a part draws with: its own frame, or that frame turned about a pivot (`tilted`). */
export type Pen = Pick<Local, "p" | "poly" | "circle" | "band">;

/** A part's frame turned `degrees` about (`cx`, `cy`), clockwise on screen as drawn facing right. */
export function tilted(f: Local, degrees: number, cx = 0, cy = 0): Pen {
  const T = (dx: number, dy: number): Point => {
    const [x, y] = turned([dx, dy], degrees);
    return [cx + x, cy + y];
  };
  return {
    p: (dx, dy) => f.p(...T(dx, dy)),
    poly: (points) => f.poly(points.map(([dx, dy]) => T(dx, dy))),
    circle: (dx, dy, r) => f.circle(...T(dx, dy), r),
    band: (points, width) => f.band(points.map(([dx, dy]) => T(dx, dy)), width),
  };
}

/** An upright rectangle from (x0, y0) to (x1, y1). */
export function box(f: Pen, x0: number, y0: number, x1: number, y1: number): string {
  return f.poly([
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ]);
}

/** A rectangle from (x0, y0) to (x1, y1), its corners cut `c` deep: a plate. */
export function cut(f: Pen, x0: number, y0: number, x1: number, y1: number, c: number): string {
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
export function rounded(f: Pen, x0: number, y0: number, x1: number, y1: number, r: number): string {
  return (
    `M${f.p(x0 + r, y0)}L${f.p(x1 - r, y0)}Q${f.p(x1, y0)} ${f.p(x1, y0 + r)}L${f.p(x1, y1 - r)}Q${f.p(x1, y1)} ${f.p(x1 - r, y1)}` +
    `L${f.p(x0 + r, y1)}Q${f.p(x0, y1)} ${f.p(x0, y1 - r)}L${f.p(x0, y0 + r)}Q${f.p(x0, y0)} ${f.p(x0 + r, y0)}Z`
  );
}

/** Points round an ellipse about (cx, cy), `rx` by `ry`, its axis turned `tilt` degrees, in `steps` steps. */
function ovalRim(cx: number, cy: number, rx: number, ry: number, tilt: number, steps: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i < steps; i += 1) {
    const [x, y] = turned([rx * Math.cos((i * 360 * DEGREES) / steps), ry * Math.sin((i * 360 * DEGREES) / steps)], tilt);
    out.push([cx + x, cy + y]);
  }
  return out;
}

/** A tongue of flame from `base`, `half` to either side of it, tapering `length` back along -x. */
function flameBack(f: Pen, base: Point, length: number, half: number): string {
  const [x, y] = base;
  return `M${f.p(x, y - half)}Q${f.p(x - length * 0.45, y - half)} ${f.p(x - length, y)}Q${f.p(x - length * 0.45, y + half)} ${f.p(x, y + half)}Z`;
}

/** A four-pointed sparkle about (cx, cy) in a turned frame, `size` its reach (kitDraw.ts `sparkle` draws in the part's own). */
function sparkleAt(f: Pen, cx: number, cy: number, size: number): string {
  const points: Point[] = [];
  for (let i = 0; i < 8; i += 1) {
    const angle = (i * 360) / 8 - 90;
    const reach = i % 2 === 0 ? size : size * 0.28;
    points.push([cx + reach * Math.cos(angle * DEGREES), cy + reach * Math.sin(angle * DEGREES)]);
  }
  return f.poly(points);
}

// Starship pieces

/** A swarm of nanobots, centred on the anchor: a cloud of tiny hexagonal bots each with a cyan eye, in a glow, two weld beams reaching down from it to a spray of sparks; about 24 wide and 12 tall (its sparks 9 below) at scale 1. */
function nanobotSwarm(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bots: Point[] = [];
  for (let row = -2; row <= 2; row += 1) {
    for (let col = -4; col <= 4; col += 1) {
      const x = col * 2.8 + (row % 2 === 0 ? 0 : 1.4);
      const y = row * 2.4;
      if ((x / 12.5) ** 2 + (y / 6) ** 2 > 1) continue;
      if ((row !== 0 || col !== 0) && rng() < 0.25) continue;
      bots.push([x + between(rng, -0.35, 0.35), y + between(rng, -0.35, 0.35)]);
    }
  }
  const hex = ([x, y]: Point): string =>
    f.poly(
      [0, 60, 120, 180, 240, 300].map((angle): Point => [x + 0.8 * Math.cos((angle + 30) * DEGREES), y + 0.8 * Math.sin((angle + 30) * DEGREES)]),
    );
  const reach = between(rng, 7.5, 9.5);
  return [
    shape(f.ellipse(0, 0, 14, 8), "cyan-400", 0.1),
    shape(f.ellipse(0, 0, 9, 5), "sky-300", 0.16),
    shape(bots.map(hex).join(""), "slate-400", 1, MAIN_INKED),
    shape(bots.map(([x, y]) => f.circle(x, y, 0.3)).join(""), "cyan-400", 1),
    shape(f.band([[-3, 4.4], [-3.6, reach]], 0.5) + f.band([[3, 4.4], [3.8, reach - 0.4]], 0.5), "cyan-400", 0.55),
    shape(sparkle(f, -3.6, reach + 0.4, 1.8) + sparkle(f, 3.8, reach, 1.4), "white", 0.95),
    shape(f.circle(-5.2, reach - 0.6, 0.35) + f.circle(-2.2, reach + 1.4, 0.3) + f.circle(5.4, reach + 0.8, 0.35), "orange-400", 0.9),
  ];
}

/** A bolted hull plate, centred on the anchor and turned a little: a plate with cut corners, two panel seams, a lit top edge, a scorch streak and a ring of rivets; about 16 wide and 10 tall at scale 1. */
function hullPlate(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -5, 5));
  const rivets: Point[] = [];
  for (const x of [-5.6, -2, 2, 5.6]) rivets.push([x, -3.8], [x, 3.8]);
  rivets.push([-6.8, 0], [6.8, 0]);
  const scorch = between(rng, -1, 1);
  return [
    shape(cut(f, -8, -5, 8, 5, 1.6), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-2.6 + scorch * 0.2, -5], [-2.6, 5]], 0.35) + f.band([[3.4, -5], [3.4 - scorch * 0.2, 5]], 0.35), "slate-700", 0.6),
    shape(f.band([[-6.4, -4.4], [6.4, -4.4]], 0.5), "white", 0.3),
    shape(f.band([[-5, 2.2 + scorch * 0.3], [-1, 1.4], [3.4, 2.6 - scorch * 0.3]], 1.2), "slate-800", 0.35),
    shape(rivets.map(([x, y]) => f.circle(x, y, 0.38)).join(""), "slate-100", 1),
  ];
}

/** A cartridge flying toward its facing, centred on its case: a copper case with a rim and a groove behind a pointed tip, glints on both; about 10 long and 2.5 tall at scale 1. */
function cartridge(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -6, 6));
  return [
    shape(box(f, -4.6, -1.1, 1.6, 1.1), "brass-600", 1, INKED),
    shape(box(f, -5.2, -1.25, -4.6, 1.25), "brass-800", 1, INKED),
    shape(f.band([[-4.1, -1.1], [-4.1, 1.1]], 0.3), "brass-800", 0.8),
    shape(f.band([[-4.4, -0.55], [1.4, -0.55]], 0.35), "white", 0.4),
    shape(`M${f.p(1.6, -1.1)}Q${f.p(4, -0.9)} ${f.p(5, 0)}Q${f.p(4, 0.9)} ${f.p(1.6, 1.1)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.band([[2, -0.6], [3.8, -0.35]], 0.25), "white", 0.45),
  ];
}

/** An overdrive engine, centred on the anchor and facing +x: a housing with three red-hot vents in a glow, a turbine intake ahead (a ring, its blades and a hub), two nozzles behind burning in layered flames; about 26 long (its flames past that) and 9 tall at scale 1. */
function overdriveEngine(at: PartPlace, rng: Rng): KitShape[] {
  const L = local(at);
  const burn = between(rng, 5, 8);
  const spin = between(rng, 0, 60);
  const blades = [0, 60, 120, 180, 240, 300]
    .map((angle) => {
      const a = (angle + spin) * DEGREES;
      return L.band([[6 + 0.6 * Math.cos(a), 0.6 * Math.sin(a)], [6 + 2.1 * Math.cos(a + 0.4), 2.1 * Math.sin(a + 0.4)]], 0.45);
    })
    .join("");
  return [
    shape(L.ellipse(-2, 0, 7, 5), "red-500", 0.22),
    shape(rounded(L, -7, -4, 5, 4, 1.5), "slate-700", 1, MAIN_INKED),
    shape(L.band([[-5, -2], [1, -2]], 0.7) + L.band([[-5, 0], [1, 0]], 0.7) + L.band([[-5, 2], [1, 2]], 0.7), "ember-500", 1),
    shape(L.circle(6, 0, 3.2) + L.circle(6, 0, 2.2), "slate-400", 1, INKED),
    shape(blades, "slate-800", 0.9),
    shape(L.circle(6, 0, 0.7), "slate-100", 1),
    shape(box(L, -9, -3.4, -7, -0.8) + box(L, -9, 0.8, -7, 3.4), "slate-800", 1, INKED),
    shape(flameBack(L, [-9, -2.1], burn * 1.4, 2) + flameBack(L, [-9, 2.1], burn * 1.4, 2), "ember-500", 0.3),
    shape(flameBack(L, [-9, -2.1], burn, 1.2) + flameBack(L, [-9, 2.1], burn, 1.2), "orange-400", 0.9),
    shape(flameBack(L, [-9, -2.1], burn * 0.5, 0.5) + flameBack(L, [-9, 2.1], burn * 0.5, 0.5), "white", 0.85),
  ];
}

/** A warp-field ring standing on the anchor: a base and a pylon holding up a tilted violet ring coil, its windings lit, a field shimmering inside it in a glow, sparks at its rim; about 20 wide and 18 tall at scale 1. */
function warpRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -12, 12);
  const outer = ovalRim(0, -13, 10, 3.6, tilt, 24);
  const inner = ovalRim(0, -13, 8.4, 2.6, tilt, 24);
  const windings = [0, 45, 90, 135, 180, 225, 270, 315]
    .map((angle) => {
      const a = angle * DEGREES;
      const from = turned([8.6 * Math.cos(a), 2.7 * Math.sin(a)], tilt);
      const to = turned([9.8 * Math.cos(a), 3.5 * Math.sin(a)], tilt);
      return f.band([[from[0], from[1] - 13], [to[0], to[1] - 13]], 0.5);
    })
    .join("");
  const [rimX, rimY] = turned([10, 0], tilt);
  return [
    shape(f.poly(ovalRim(0, -13, 13, 6, tilt, 16)), "violet-300", 0.18),
    shape(f.poly([[-4, 0], [4, 0], [3, -1.4], [-3, -1.4]]), "slate-800", 1, INKED),
    shape(f.poly([[-1.6, -1.4], [1.6, -1.4], [0.9, -9.6], [-0.9, -9.6]]), "slate-700", 1, INKED),
    shape(f.poly(inner), "purple-400", 0.25),
    shape(f.poly(outer) + f.poly(inner), "violet-500", 1, MAIN_INKED),
    shape(windings, "violet-300", 0.9),
    shape(sparkle(f, rimX, rimY - 13, 1.6) + sparkle(f, -rimX, -rimY - 13, 1.2), "white", 0.9),
  ];
}

/** A rail gun standing on the anchor, facing +x and aimed up a little: a base and a swivel holding a breech and twin rails about 24 long, a charge crackling between them in a glow, a flare at the muzzle; about 30 wide and 12 tall at scale 1. */
function railGun(at: PartPlace, rng: Rng): KitShape[] {
  const L = local(at);
  const f = tilted(L, -between(rng, 10, 22), 0, -4);
  const zig: Point[] = [];
  for (let k = 0; k <= 8; k += 1) zig.push([1 + k * 2.3, k % 2 === 0 ? -0.6 : 0.6]);
  return [
    shape(L.poly([[-5, 0], [5, 0], [3.6, -2.6], [-3.6, -2.6]]), "slate-800", 1, INKED),
    shape(L.circle(0, -4, 2.6), "slate-700", 1, INKED),
    shape(f.band([[0, 0], [20, 0]], 3.6), "sky-300", 0.25),
    shape(f.band([[0, 0], [20, 0]], 1.4), "cyan-400", 0.75),
    shape(f.poly([[-4, -2.4], [20, -2.4], [20, -1.3], [-4, -1.3]]) + f.poly([[-4, 1.3], [20, 1.3], [20, 2.4], [-4, 2.4]]), "slate-400", 1, MAIN_INKED),
    shape(box(f, -6.4, -2.8, -2.4, 2.8), "slate-700", 1, INKED),
    shape(f.band(zig, 0.3), "white", 0.85),
    shape(sparkleAt(f, 21, 0, 2.4), "white", 0.9),
  ];
}

/** A great cannon standing on the anchor, facing +x: a turret block and its cap, a banded barrel about 40 long raised a little, its collar, a highlight along it and a glowing muzzle; about 50 long and 12 tall at scale 1. */
function greatCannon(at: PartPlace, rng: Rng): KitShape[] {
  const L = local(at);
  const f = tilted(L, -between(rng, 3, 8), 4, -6);
  return [
    shape(L.poly([[-9, 0], [9, 0], [7, -7], [-6, -8]]), "slate-700", 1, INKED),
    shape(L.poly([[-6, -8], [7, -7], [5, -10], [-4, -10.5]]), "slate-400", 1, INKED),
    shape(f.poly([[0, -2.4], [40, -1.8], [40, 1.8], [0, 2.4]]), "slate-400", 1, MAIN_INKED),
    shape([8, 16, 24, 32].map((x) => f.band([[x, -2.3], [x, 2.3]], 1.2)).join(""), "slate-800", 0.9),
    shape(f.band([[2, -1.4], [38, -1]], 0.5), "white", 0.25),
    shape(box(f, 38, -2.6, 42, 2.6), "slate-700", 1, INKED),
    shape(f.circle(43, 0, 5), "sky-300", 0.2),
    shape(f.circle(43, 0, 2.8), "cyan-400", 0.45),
    shape(f.circle(43, 0, 1.4), "white", 0.9),
  ];
}

/** A small missile pod, centred on the anchor: a plated box with two rows of three tube mouths, a red missile tip in each, a hazard stripe, a bracket below and a status light; about 10 wide and 9 tall at scale 1. */
function missilePod(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -6, 6));
  const tubes: Point[] = [];
  for (const y of [-1.6, 1.6]) for (const x of [-3.2, 0, 3.2]) tubes.push([x, y]);
  return [
    shape(cut(f, -5, -3.6, 5, 3.6, 1), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-5, 3], [5, 3]], 0.6), "red-500", 0.8),
    shape(tubes.map(([x, y]) => f.circle(x, y, 1.2)).join(""), "slate-800", 1, INKED),
    shape(tubes.map(([x, y]) => f.circle(x, y, 0.6)).join(""), "red-500", 1),
    shape(f.poly([[-2, 3.6], [2, 3.6], [1.4, 5.4], [-1.4, 5.4]]), "slate-700", 1, INKED),
    shape(f.circle(4, -4.3, 0.5), "lime-400", 1),
  ];
}

/** A capacitor cell standing on the anchor: a glass cylinder between two caps, a glowing violet core inside it wound by a copper coil, two terminals on top with an arc crackling between them, all in a glow; about 8 wide and 17 tall at scale 1. */
function capacitorCell(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const coil = [-4, -5.5, -7, -8.5, -10].map((y) => f.band([[-3.4, y + 0.6], [3.4, y - 0.6]], 0.6)).join("");
  const arc: Point[] = [[-2, -15.4]];
  for (let k = 1; k < 5; k += 1) arc.push([-2 + k, -15.8 - between(rng, 0.6, 1.8)]);
  arc.push([2, -15.4]);
  return [
    shape(f.ellipse(0, -7.5, 7, 9), "violet-300", 0.2),
    shape(box(f, -3.4, -11.6, 3.4, -2.4), "sky-300", 0.3),
    shape(box(f, -1.4, -11, 1.4, -3), "violet-500", 0.9, MAIN),
    shape(f.band([[0, -10.6], [0, -3.4]], 0.8), "white", 0.75),
    shape(coil, "brass-600", 1, INKED),
    shape(box(f, -4, -2.4, 4, 0) + box(f, -4, -14, 4, -11.6), "slate-700", 1, INKED),
    shape(box(f, -2.6, -15.4, -1.4, -14) + box(f, 1.4, -15.4, 2.6, -14), "slate-400", 1, INKED),
    shape(f.band(arc, 0.3), "white", 0.85),
  ];
}

/** A flank pod, centred on the anchor and facing +x: a rounded hull with a stripe and a panel seam, a porthole and its glass and glint, two docking clamps on top and a running light in its glow; about 14 long and 9 tall at scale 1. */
function podCapsule(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -6, 6));
  return [
    shape(rounded(f, -7, -3.5, 7, 3.5, 3.4), "slate-100", 1, MAIN_INKED),
    shape(f.band([[-5.6, 1.7], [5.6, 1.7]], 0.8), "blue-600", 0.8),
    shape(f.band([[-2, -3.4], [-2, 3.4]], 0.3), "slate-400", 0.8),
    shape(f.circle(2.6, -0.4, 2) + f.circle(2.6, -0.4, 1.4), "slate-700", 1, INKED),
    shape(f.circle(2.6, -0.4, 1.4), "cyan-400", 0.7),
    shape(f.band([[1.8, -1.1], [2.6, -1.5]], 0.3), "white", 0.7),
    shape(box(f, -4.6, -5, -3, -3.3) + box(f, -0.8, -5, 0.8, -3.3), "slate-700", 1, INKED),
    shape(f.circle(-6, -0.4, 1.3), "red-400", 0.25),
    shape(f.circle(-6, -0.4, 0.5), "red-400", 1),
  ];
}

/** A launcher tube standing on the anchor, aimed up toward its facing: a base, a yoke and its hub holding a banded tube about 18 long, a sight on its back and a muzzle collar; about 16 wide and 18 tall at scale 1. */
function launcherTube(at: PartPlace, rng: Rng): KitShape[] {
  const L = local(at);
  const f = tilted(L, -between(rng, 40, 60), 0, -5);
  return [
    shape(L.poly([[-5, 0], [5, 0], [3.6, -2.4], [-3.6, -2.4]]), "slate-800", 1, INKED),
    shape(L.poly([[-2.2, -2.4], [2.2, -2.4], [1.6, -5.4], [-1.6, -5.4]]), "slate-700", 1, INKED),
    shape(box(f, -5, -2, 13, 2), "slate-400", 1, MAIN_INKED),
    shape(f.band([[1, -2], [1, 2]], 0.8) + f.band([[8, -2], [8, 2]], 0.8), "slate-800", 0.9),
    shape(box(f, 3, -3.2, 6, -2), "slate-700", 0.9),
    shape(box(f, 12.4, -2.5, 14.4, 2.5), "slate-700", 1, INKED),
    shape(L.circle(0, -5, 1.2), "slate-100", 1),
  ];
}

/** A hundred-sided die, centred on the anchor and turned at random: a near-round ten-sided body, a lighter facet facing out, the facet edges running from it to the rim, a shade on its lower side, a highlight and a glint; no digits; about 12 across at scale 1. */
function d100(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spin = between(rng, 0, 36);
  const rim: Point[] = [];
  for (let i = 0; i < 10; i += 1) {
    const a = (spin + i * 36) * DEGREES;
    const r = 6 * between(rng, 0.94, 1.04);
    rim.push([r * Math.cos(a), r * Math.sin(a)]);
  }
  const face: Point[] = [];
  for (let i = 0; i < 5; i += 1) {
    const a = (spin + 18 + i * 72) * DEGREES;
    face.push([3 * Math.cos(a), 3 * Math.sin(a) - 0.4]);
  }
  const edges = face
    .map((corner, i) => {
      const left = rim[(2 * i) % 10] ?? corner;
      const right = rim[(2 * i + 1) % 10] ?? corner;
      return f.band([left, corner], 0.3) + f.band([corner, rim[(2 * i + 2) % 10] ?? right], 0.3);
    })
    .join("");
  const lower = rim.filter(([, y]) => y > 0.8);
  return [
    shape(f.poly(rim), "red-600", 1, MAIN_INKED),
    shape(f.poly(face), "red-400", 0.55),
    shape(edges, "red-900", 0.7),
    shape(lower.length > 1 ? f.poly([...lower, [0, 3.4]]) : f.circle(0, 3.4, 1), "red-900", 0.3),
    shape(f.ellipse(-2.2, -2.6, 1.6, 0.9), "white", 0.35),
    shape(sparkle(f, -3, -3.6, 1.2), "white", 0.9),
  ];
}

/** A pineapple standing on the anchor: an oval orange fruit dotted with its eyes, a highlight, and a spiky crown of leaves in two greens; about 7 wide and 16 tall at scale 1. */
function pineapple(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const eyes: string[] = [];
  for (let row = 0; row < 6; row += 1) {
    const y = -1.4 - row * 1.4;
    for (let col = -2; col <= 2; col += 1) {
      const x = col * 1.3 + (row % 2 === 0 ? 0 : 0.65);
      if ((x / 2.8) ** 2 + ((y + 4.8) / 4.3) ** 2 > 1) continue;
      eyes.push(f.circle(x, y, 0.32));
    }
  }
  const crown = (reach: number, spikes: number): Point[] => {
    const out: Point[] = [[-1.8, -9]];
    for (let k = 0; k < spikes; k += 1) {
      const a = (-150 + (120 * (k + 0.5)) / spikes) * DEGREES;
      const length = reach * between(rng, 0.75, 1.05);
      out.push([1.8 * Math.cos(a + 0.2) * 0.6, -9.6 + Math.sin(a + 0.2) * 0.6], [length * Math.cos(a) * 0.7, -9.4 + length * Math.sin(a)]);
    }
    out.push([1.8, -9]);
    return out;
  };
  return [
    shape(f.ellipse(0, -4.8, 3.2, 4.8), "orange-400", 1, MAIN_INKED),
    shape(eyes.join(""), "brass-800", 0.7),
    shape(f.ellipse(-1.2, -6, 0.8, 2.2), "white", 0.25),
    shape(f.poly(crown(6, 5)), "green-500", 1, INKED),
    shape(f.poly(crown(4, 3)), "green-700", 0.85),
  ];
}

/** An escalator rising from the anchor toward its facing: a truss from a landing on the ground up to a landing about 16 above, its steps along it, a glass balustrade and a handrail over them, lights along the truss's underside; about 31 long and 22 tall at scale 1. */
function escalator(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const run = between(rng, 26, 30);
  const rise = 16;
  const steps = 9;
  const tread: Point[] = [[2, -1.2]];
  for (let k = 0; k < steps; k += 1) {
    const x0 = 2 + ((run - 2) * k) / steps;
    const x1 = 2 + ((run - 2) * (k + 1)) / steps;
    const y1 = -1.2 - ((rise - 1.2) * (k + 1)) / steps;
    tread.push([x0, y1], [x1, y1]);
  }
  tread.push([run, -rise + 1.2], [3, 0]);
  const lights = [0.2, 0.4, 0.6, 0.8].map((t) => f.circle(3.4 + (run - 3.4) * t, -(rise - 2.6) * t + 0.4, 0.4)).join("");
  return [
    shape(f.poly([[0, 0], [0, -1.2], [2, -1.2], [run, -rise], [run + 3, -rise], [run + 3, -rise + 2.6], [run - 1, -rise + 2.6], [3.4, 0]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly(tread), "slate-700", 1),
    shape(f.poly([[1, -1.2], [run, -rise], [run, -rise - 5], [1, -6.2]]), "sky-300", 0.25),
    shape(f.band([[0.6, -6.2], [run, -rise - 5], [run + 3, -rise - 5]], 0.7), "slate-800", 1, INKED),
    shape(lights, "cyan-400", 0.9),
  ];
}

/** A glowing seashell, centred on the anchor: a spiral conch with its whorls, a violet lip at its mouth and a glint, all in a soft glow; about 9 wide and 7 tall (its glow 12 across) at scale 1. */
function seashell(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swell = between(rng, -0.4, 0.4);
  return [
    shape(f.circle(0, 0, 6), "cyan-400", 0.18),
    shape(f.circle(0, 0, 4.2), "sky-300", 0.2),
    shape(`M${f.p(-4.4, 1)}Q${f.p(-3, -4 - swell)} ${f.p(1, -3.6)}Q${f.p(4.6, -2.6)} ${f.p(4.2, 0.4)}Q${f.p(2.6, 3 + swell)} ${f.p(-1, 2.6)}Q${f.p(-3.6, 2.4)} ${f.p(-4.4, 1)}Z`, "red-300", 1, MAIN_INKED),
    shape(spiralArm(f, between(rng, 0, 90), 300, 0.4, 3, 0.8), "white", 0.55),
    shape(f.ellipse(1.8, 0.9, 1.6, 1), "violet-300", 0.85),
    shape(sparkle(f, -1.6, -2.4, 1.2), "white", 0.9),
  ];
}

/** R3406000: the parts Stinkyscape's later Alt art pass added, by id, each with its kind. */
export const STINKY_ALT_KIT = {
  "nanobot-swarm": { category: "creatures", draw: nanobotSwarm },
  "hull-plate": { category: "objects", draw: hullPlate },
  cartridge: { category: "objects", draw: cartridge },
  "overdrive-engine": { category: "objects", draw: overdriveEngine },
  "warp-ring": { category: "objects", draw: warpRing },
  "rail-gun": { category: "objects", draw: railGun },
  "great-cannon": { category: "objects", draw: greatCannon },
  "missile-pod": { category: "objects", draw: missilePod },
  "capacitor-cell": { category: "objects", draw: capacitorCell },
  "pod-capsule": { category: "objects", draw: podCapsule },
  "launcher-tube": { category: "objects", draw: launcherTube },
  d100: { category: "objects", draw: d100 },
  pineapple: { category: "plants", draw: pineapple },
  escalator: { category: "architecture", draw: escalator },
  seashell: { category: "objects", draw: seashell },
} as const satisfies Readonly<Record<string, KitPart>>;
