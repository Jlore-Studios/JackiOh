// R3403000: the scene kit's weather of space (ART.md, "Scene recipes and kits"), which Stinkyscape's later cards
// (#3403) added: a comet, frost, an ice shard and an ice block, confetti, a planet's ring, an alarm light, orbit
// traces, speed streaks, a crashing wave, a lens flare, a wormhole, a rift in space's fabric, a galaxy and a stink
// cloud. sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css name, none in Radiant's gold
// window; a glow is a stack of translucent shapes, never a gradient; shapes are filled even-odd, so a hole is a
// second sub-path and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, blob, DEGREES, INKED, local, MAIN, MAIN_INKED, shape, sparkle, spiralArm, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";

// Helpers

/** Points along an ellipse about (cx, cy), `rx` by `ry` with its axis turned `tilt` degrees, from `from` to `to` degrees in `steps` steps. */
function ellipseArc(cx: number, cy: number, rx: number, ry: number, tilt: number, from: number, to: number, steps: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const angle = (from + ((to - from) * i) / steps) * DEGREES;
    const [x, y] = turned([rx * Math.cos(angle), ry * Math.sin(angle)], tilt);
    out.push([cx + x, cy + y]);
  }
  return out;
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

/** A closed outline smoothed through the midpoints of its rim, as `blob` smooths its own. */
function smooth(f: Local, rim: readonly Point[]): string {
  const at = (i: number): Point => rim[i % rim.length] ?? [0, 0];
  const mid = (a: Point, b: Point): string => f.p((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
  let d = `M${mid(at(0), at(1))}`;
  for (let i = 1; i <= rim.length; i += 1) d += `Q${f.p(...at(i))} ${mid(at(i), at(i + 1))}`;
  return `${d}Z`;
}

// Ice and comets

/** A comet flying toward its facing, centred on its head (about 6 across): a tail about 30 long behind it in three layered translucent bands, a glowing coma, a bright white core and bits of debris along the tail. */
function comet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -3, 3);
  const length = between(rng, 27, 32);
  const tail = (reach: number, half: number): string =>
    `M${f.p(0, -half)}Q${f.p(-reach * 0.45, -half * 0.8 + bend * 0.25)} ${f.p(-reach, (bend * reach) / length)}` +
    `Q${f.p(-reach * 0.45, half * 0.8 + bend * 0.25)} ${f.p(0, half)}A${fmt(f.r(half))} ${fmt(f.r(half))} 0 0 ${f.sweep(0)} ${f.p(0, -half)}Z`;
  let debris = "";
  for (let k = 0; k < 3; k += 1) {
    const x = -(9 + k * 6 + between(rng, 0, 3));
    const t = -x / length;
    debris += f.circle(x, bend * t * t + between(rng, -1.2, 1.2), between(rng, 0.3, 0.5));
  }
  return [
    shape(tail(length, 3.6), "cyan-400", 0.13, MAIN),
    shape(tail(length * 0.78, 2.6), "cyan-400", 0.2, MAIN),
    shape(tail(length * 0.52, 1.6), "cyan-400", 0.3, MAIN),
    shape(tail(length * 0.3, 0.8), "white", 0.35),
    shape(f.circle(0, 0, 5), "sky-300", 0.16),
    shape(f.circle(0, 0, 3), "white", 0.5),
    shape(f.circle(0.4, 0, 1.6), "white", 1),
    shape(debris, "white", 0.55),
  ];
}

/** Frost spreading over the ground about the anchor: a pale sheet, angular crystal patches on it, a crown of spiky rime, fern-like frost lines and glints; about 30 wide and 6 tall (from 2 below the anchor to 4 above it) at scale 1. */
function frost(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rim: Point[] = [];
  for (let i = 0; i < 12; i += 1) {
    const angle = i * 30 * DEGREES;
    const reach = between(rng, 0.85, 1.1);
    rim.push([15 * reach * Math.cos(angle), 0.4 + 1.6 * reach * Math.sin(angle)]);
  }
  const count = 16;
  const spikes: Point[] = [[-14, 0.6]];
  for (let k = 0; k <= count; k += 1) {
    const x = -13 + (26 * k) / count;
    spikes.push([x, 0]);
    if (k === count) continue;
    const mid = x + 13 / count;
    spikes.push([mid + between(rng, -0.3, 0.3), -between(rng, 1.2, 3.8) * (1 - 0.5 * (mid / 14) ** 2)]);
  }
  spikes.push([14, 0.6]);
  const patch = (cx: number): string => {
    const points: Point[] = [];
    for (let i = 0; i < 6; i += 1) {
      const angle = (i * 60 + 30) * DEGREES;
      const reach = between(rng, 0.7, 1.1);
      points.push([cx + 2.4 * reach * Math.cos(angle), 0.9 + 0.9 * reach * Math.sin(angle)]);
    }
    return f.poly(points);
  };
  const patches = [-8, 0, 8].map((x) => patch(x + between(rng, -1, 1))).join("");
  const fern = (x0: number, dir: number): string => {
    const length = between(rng, 4, 6);
    const twig = (t: number, side: number): string => {
      const [bx, by] = [x0 + dir * length * t, 1.4 - 1.2 * t + side * 0.5];
      return f.band([[bx, by], [bx + dir * 1.3, by + side * 0.9]], 0.3);
    };
    return f.band([[x0, 1.4], [x0 + dir * length, 0.2]], 0.35) + twig(0.35, -1) + twig(0.65, 1);
  };
  const glints = [-8, 0, 8].map((x) => sparkle(f, x + between(rng, -2, 2), between(rng, -3.4, -1.6), between(rng, 1, 1.6))).join("");
  return [
    shape(f.ellipse(0, 0.2, 16.5, 3.2), "sky-300", 0.1),
    shape(smooth(f, rim), "sky-300", 0.32, MAIN),
    shape(patches, "blue-200", 0.55),
    shape(f.poly(spikes), "sky-300", 0.8, MAIN),
    shape(fern(-11 + between(rng, 0, 2), 1) + fern(11 - between(rng, 0, 2), -1), "white", 0.45),
    shape(glints, "white", 0.9),
  ];
}

/** A jagged ice shard flying toward its facing, centred on the anchor: a faceted spike about 12 long and 4 thick, its lit upper facet and shaded lower one, a glint at its point, chips and a trail of mist behind it. */
function iceShard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.35, 0.35);
  const tip: Point = [6.2, 0];
  const top: Point[] = [[1.6, -1.8 + j()], [-2, -1.4 + j()], [-4.2, -2.2 + j()], [-6, -1.1 + j()]];
  const notch: Point = [-4.6 + j(), 0];
  const bottom: Point[] = [[-6.2, 1.2 + j()], [-3.2, 1.9 + j()], [0.8, 1.6 + j()]];
  const chip = (x: number, y: number, r: number): string => f.poly([[x + r, y], [x, y - r * 0.6], [x - r, y], [x, y + r * 0.6]]);
  return [
    shape(f.ellipse(-10, 0, 7.6, 1.8), "white", 0.1),
    shape(f.ellipse(-8, 0, 4.6, 1.1), "sky-300", 0.18),
    shape(chip(-11.5, between(rng, -1.2, -0.4), 0.7) + chip(-14.5, between(rng, 0.4, 1.2), 0.5), "sky-300", 0.7, MAIN),
    shape(f.poly([tip, ...top, notch, ...bottom]), "sky-300", 1, MAIN_INKED),
    shape(f.poly([tip, [-0.8, 0.5], notch, ...bottom]), "blue-800", 0.3),
    shape(f.poly([tip, ...top, notch, [-0.8, -0.2]]), "white", 0.4),
    shape(f.band([tip, top[0] ?? tip, top[1] ?? tip], 0.35), "white", 0.75),
    shape(sparkle(f, 5.2, -0.4, 1.8), "white", 0.95),
  ];
}

/** A block of translucent ice standing on the anchor, to be drawn over a figure or an object to freeze it: a faceted cube (a pale front, a side face and a lit top), streaks of light, cracks, bright edges, rime at its foot and a glint; about 16 wide and 20 tall at scale 1. */
function iceBlock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const depth = between(rng, 2.6, 3.4);
  const lift = between(rng, -0.5, 0.5);
  const right = 8 - depth;
  const top = depth - 20;
  const crack = between(rng, -0.5, 0.5);
  const rime: Point[] = [[-8, 0.4], [-8, 0]];
  for (let k = 0; k < 8; k += 1) {
    const x = -8 + ((k + 0.5) * (right + 8)) / 8;
    rime.push([x, -between(rng, 0.6, 1.4)], [-8 + ((k + 1) * (right + 8)) / 8, 0]);
  }
  rime.push([right, 0.4]);
  return [
    shape(f.poly([[-8, 0], [right, 0], [right, top], [-8, top + lift]]), "blue-200", 0.35, MAIN_INKED),
    shape(f.poly([[right, 0], [8, -depth], [8, -20], [right, top]]), "blue-200", 0.45, MAIN_INKED),
    shape(f.poly([[-8, top + lift], [right, top], [8, -20], [depth - 8, lift / 2 - 20]]), "white", 0.35, INKED),
    shape(f.band([[-6.8, -2.4], [-3.4, top + 3]], 0.9) + f.band([[-4.4, -1.8], [-3, -5.6]], 0.5), "white", 0.35),
    shape(f.band([[right - 1.2, -4 + crack], [right - 3, -6.6], [right - 2.2, -9], [right - 4.4, -11.6 - crack]], 0.32) + f.band([[-5, top + 2.4], [-3.6, top + 4.6], [-4.4, top + 6.4]], 0.3), "white", 0.65),
    shape(f.band([[-8, top + lift], [right, top], [right, 0]], 0.45), "white", 0.7),
    shape(f.poly(rime), "white", 0.5),
    shape(sparkle(f, right, top, 2.2), "white", 0.9),
  ];
}

// Festivities and the deep sky

/** Ticker tape and confetti over a field about 40 wide and 30 tall about the anchor: small turned squares and strips in five paints, one to each cell of a loose grid so no two overlap, and two or three curling tapes. */
function confetti(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const paints: readonly PaletteName[] = ["green-400", "red-400", "white", "sky-300", "violet-300"];
  const groups = paints.map(() => "");
  const offset = whole(rng, 0, paints.length - 1);
  const [cols, rows] = [6, 5];
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      const cx = -20 + ((col + 0.5) * 40) / cols + between(rng, -1.8, 1.8);
      const cy = -15 + ((row + 0.5) * 30) / rows + between(rng, -1.5, 1.5);
      const turn = between(rng, 0, 180);
      const [a, b] = rng() < 0.5 ? [1.1, 0.35] : [0.6, 0.6];
      const corners = ([[-a, -b], [a, -b], [a, b], [-a, b]] as const).map(([x, y]): Point => {
        const [tx, ty] = turned([x, y], turn);
        return [cx + tx, cy + ty];
      });
      const k = (row * cols + col + offset) % paints.length;
      groups[k] = (groups[k] ?? "") + f.poly(corners);
    }
  }
  const out = groups.map((d, k) => shape(d, paints[k] ?? "white", 0.95, k === 0 ? MAIN : undefined));
  const tapes = whole(rng, 2, 3);
  for (let t = 0; t < tapes; t += 1) {
    const x0 = -16 + t * 12 + between(rng, -2, 2);
    const y0 = between(rng, -14, -4);
    const phase = between(rng, 0, 6);
    const lean = between(rng, -0.6, 0.6);
    const points: Point[] = [];
    for (let i = 0; i <= 8; i += 1) points.push([x0 + i * (0.6 + lean) + 1.4 * Math.sin(phase + i * 1.1), y0 + i * 1.5]);
    const k = (offset + t + 1) % paints.length;
    out.push(shape(f.band(points, 0.55), paints[k] ?? "white", 0.9, k === 0 ? MAIN : undefined));
  }
  return out;
}

/** A planet's ring across the box, centred on the anchor: only its front (lower) half, so a planet drawn behind shows above it, in three translucent concentric bands with a dark gap between the outer two, a faint dust band, a pale lane and a lit edge; its ellipse about 90 wide and 20 tall (the half drawn 10 below the anchor) at scale 1. */
function ringArc(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -5, 5);
  const flat = between(rng, 0.2, 0.24);
  const half = (rx: number): Point[] => ellipseArc(0, 0, rx, rx * flat, tilt, 0, 180, 28);
  const band = (outer: number, inner: number): string => f.poly([...half(outer), ...half(inner).reverse()]);
  return [
    shape(band(46.5, 45), "slate-400", 0.14, MAIN),
    shape(band(45, 39.6), "slate-400", 0.42, MAIN),
    shape(band(38.4, 31), "slate-400", 0.6, MAIN),
    shape(band(31, 26), "slate-400", 0.28, MAIN),
    shape(f.band(half(35), 0.5), "slate-100", 0.25),
    shape(f.band(half(38.2), 0.4), "white", 0.3),
  ];
}

/** A red alarm beacon standing on the anchor: a slate base, a caged glass dome with its lamp lit, a glow about it and a translucent beam sweeping out toward its facing, a fainter one behind; about 7 wide and 6 tall, its beam about 26 long at scale 1. */
function alarmLight(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const aim = between(rng, -28, 6);
  const spread = between(rng, 9, 13);
  const reach = between(rng, 22, 28);
  const cy = -3.4;
  const r = 2.6;
  const ray = (degrees: number, length: number): Point => [length * Math.cos(degrees * DEGREES), cy + length * Math.sin(degrees * DEGREES)];
  const cone = (degrees: number, half: number, length: number): string =>
    f.poly([[0, cy], ray(degrees - half, length), ray(degrees, length * 1.04), ray(degrees + half, length)]);
  const dome = `M${f.p(-r, -1.4)}L${f.p(-r, cy)}A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(r, cy)}L${f.p(r, -1.4)}Z`;
  return [
    shape(f.circle(0, cy, 6.4), "red-500", 0.14, MAIN),
    shape(cone(180 - aim, spread * 0.7, reach * 0.55), "red-500", 0.1, MAIN),
    shape(cone(aim, spread, reach), "red-500", 0.16, MAIN),
    shape(cone(aim, spread * 0.45, reach * 0.9), "red-400", 0.18),
    shape(f.poly([[-3.6, 0], [3.6, 0], [3.2, -1.6], [-3.2, -1.6]]), "slate-700", 1, INKED),
    shape(f.band([[-2.9, -1.3], [2.9, -1.3]], 0.4), "slate-400", 0.8),
    shape(dome, "red-500", 0.9, MAIN_INKED),
    shape(f.circle(0, cy + 0.2, 1.5), "red-300", 1),
    shape(f.circle(0, cy + 0.2, 0.7), "white", 1),
    shape(f.band([[-1.6, -1.6], [-1.7, cy], [-0.8, cy - 2]], 0.4) + f.band([[1.6, -1.6], [1.7, cy], [0.8, cy - 2]], 0.4), "slate-800", 0.9),
    shape(f.ellipse(-1.2, cy - 1.3, 0.8, 0.5), "white", 0.55),
  ];
}

/** Faint orbit traces centred on the anchor: two or three thin translucent ellipses one inside another, a little tilted, each dashed or broken where the stream says, and a small body riding the outermost; about 60 wide and 24 tall at scale 1. */
function orbitTrace(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -12, 12);
  const count = whole(rng, 2, 3);
  const out: KitShape[] = [];
  for (let k = 0; k < count; k += 1) {
    const rx = 30 - k * 8.5;
    const ry = rx * 0.4;
    const start = between(rng, 0, 360);
    let d = "";
    if (rng() < 0.45) {
      for (let a = 0; a < 360; a += 20) d += f.band(ellipseArc(0, 0, rx, ry, tilt, start + a, start + a + 11, 3), 0.4);
    } else {
      d = f.band(ellipseArc(0, 0, rx, ry, tilt, start, start + 360 - between(rng, 25, 70), 40), 0.45);
    }
    out.push(shape(d, "violet-300", 0.42 - k * 0.07, MAIN));
  }
  const [bx, by] = ellipseArc(0, 0, 30, 12, tilt, 0, between(rng, 0, 360), 1)[1] ?? [30, 0];
  out.push(shape(f.circle(bx, by, 2.2), "violet-300", 0.2));
  out.push(shape(f.circle(bx, by, 1), "white", 0.85));
  return out;
}

/** Speed streaks trailing behind the anchor, away from its facing: five to eight thin translucent lines in parallel lanes, each tapering from its head near the anchor to a point 12 to 30 behind it, a spark at the brightest heads; about 30 long and 14 tall at scale 1. */
function streaks(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 5, 8);
  const slope = between(rng, -0.12, 0.12);
  let bright = "";
  let faint = "";
  let heads = "";
  for (let k = 0; k < count; k += 1) {
    const y = (k - (count - 1) / 2) * 1.9 + between(rng, -0.35, 0.35);
    const x = between(rng, -3, 1);
    const length = between(rng, 12, 30);
    const w = between(rng, 0.4, 0.75);
    const line = f.poly([[x, y - w / 2], [x - length, y - slope * length], [x, y + w / 2]]);
    if (k % 3 === 0) {
      bright += line;
      heads += f.circle(x, y, w * 0.7);
    } else faint += line;
  }
  return [shape(faint, "white", 0.28, MAIN), shape(bright, "white", 0.55, MAIN), shape(heads, "white", 0.85, MAIN)];
}

/** A crashing wave curling toward its facing, standing on the anchor: a swelling back, a crest thrown forward into a lip over a hollow barrel (open, so what lies behind shows through), its falling curtain, a foam crest (white), foam at its toe and spray; about 30 wide and 20 tall at scale 1. */
function wave(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, -1.2, 1.2);
  const fling = between(rng, -0.8, 0.8);
  const lipTip: Point = [13.4 + fling, -10.6];
  const back = quad([-15, 0], [-9, -1.4], [-5.6, -6], 6);
  const swell = quad([-5.6, -6], [-2, -13 - rise], [3.6, -18.2 - rise], 6);
  const crest = quad([3.6, -18.2 - rise], [8.6, -21 - rise], [12.6 + fling, -16.8], 6);
  const lip = quad([12.6 + fling, -16.8], [15 + fling, -14], lipTip, 5);
  const under = quad(lipTip, [12.4 + fling, -12.8], [10, -13.2], 4);
  const roof = quad([10, -13.2], [6.4, -13.6], [5.2, -9.6], 5);
  const face = quad([5.2, -9.6], [4.4, -5], [7.4, -1.6], 5);
  const toe = quad([7.4, -1.6], [9.4, 0], [12, 0], 4);
  const body = [...back, ...swell.slice(1), ...crest.slice(1), ...lip.slice(1), ...under.slice(1), ...roof.slice(1), ...face.slice(1), ...toe.slice(1)];
  const spray = ([[15.6, -19.6], [17.4, -17], [16.4, -22.2], [18.8, -20.4], [14.2, -22.8]] as const)
    .map(([x, y]) => f.circle(x + fling + between(rng, -0.3, 0.3), y + between(rng, -0.3, 0.3), between(rng, 0.35, 0.7)))
    .join("");
  return [
    shape(f.poly(body), "jade-400", 1, MAIN_INKED),
    shape(f.poly([...roof, ...face.slice(1), [4, -1.4], [2.4, -6], [3.6, -11]]), "slate-900", 0.2),
    shape(f.band([[-9.6, -0.8], [-5.4, -4], [-2.2, -9.6]], 0.7) + f.band([[-6, -0.6], [-2.8, -3]], 0.5), "white", 0.22),
    shape(f.band([lipTip, [13.8 + fling, -6], [12.6, -1.2]], 1.4), "jade-400", 0.35, MAIN),
    shape(f.band([...swell.slice(3), ...crest.slice(1), ...lip.slice(1)], 1.5), "white", 1, INKED),
    shape(blob(f, rng, 11, -0.8, 1.8, 8), "white", 0.85, INKED),
    shape(spray, "white", 0.8),
  ];
}

// Light and the fabric of space

/** A lens flare centred on the anchor: a glow about a bright core and a four-pointed sparkle, a long horizontal streak, and three or four translucent rings and discs strung along a diagonal through it; about 44 wide and 30 tall at scale 1. */
function lensFlare(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const angle = between(rng, 25, 50) * DEGREES;
  const [ux, uy] = [-Math.cos(angle), Math.sin(angle)];
  const streak = between(rng, 20, 24);
  const star = between(rng, 4.5, 6);
  const count = whole(rng, 3, 4);
  const stops: readonly (readonly [number, number, PaletteName, boolean])[] = [
    [7, 1.4, "violet-300", false],
    [12.5, 3, "cyan-400", true],
    [-8, 2, "sky-300", true],
    [20, 4.2, "violet-300", false],
  ];
  const rings = stops.slice(0, count).map(([distance, r, paint, hollow]) => {
    const d = distance * between(rng, 0.9, 1.1);
    const [cx, cy] = [ux * d, uy * d];
    return shape(hollow ? f.circle(cx, cy, r) + f.circle(cx, cy, r * 0.78) : f.circle(cx, cy, r), paint, hollow ? 0.3 : 0.16, paint === "cyan-400" ? MAIN : undefined);
  });
  return [
    shape(f.circle(0, 0, 7), "cyan-400", 0.12, MAIN),
    shape(f.ellipse(0, 0, streak * 0.8, 1.4), "cyan-400", 0.18, MAIN),
    shape(f.ellipse(0, 0, streak, 0.4), "white", 0.7),
    shape(f.circle(0, 0, 3.4), "white", 0.3),
    shape(sparkle(f, 0, 0, star), "white", 0.85),
    shape(f.circle(0, 0, 1.3), "white", 1),
    ...rings,
  ];
}

/** A swirling wormhole centred on the anchor: a faint glow, five rings narrowing and leaning as they fall in toward a dark centre, spiral arms turning into it, a bright rim at its mouth and sparks; about 30 across at scale 1. */
function wormhole(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = between(rng, 0, 360);
  const lean = between(rng, 0, 360) * DEGREES;
  const [lx, ly] = [0.45 * Math.cos(lean), 0.35 * Math.sin(lean)];
  const radii = [15, 11.6, 8.8, 6.4, 4.6];
  const widths = [1.4, 1.2, 1, 0.8, 0.6];
  const ring = (k: number): string => {
    const r = radii[k] ?? 0;
    const w = widths[k] ?? 0;
    return f.ellipse(lx * k, ly * k, r, r * 0.86) + f.ellipse(lx * k, ly * k, r - w, (r - w) * 0.86);
  };
  let arms = "";
  for (let k = 0; k < 3; k += 1) arms += spiralArm(f, start + k * 120, 250, 4.2, 14.6, 2.4);
  const glint = between(rng, 0, 360) * DEGREES;
  const spark = (a: number, size: number): string => sparkle(f, 15.3 * Math.cos(a), 13.15 * Math.sin(a), size);
  return [
    shape(f.circle(0, 0, 18), "violet-500", 0.1),
    shape(ring(0) + ring(2) + ring(4), "violet-500", 0.45, MAIN),
    shape(ring(1) + ring(3), "violet-500", 0.65, MAIN),
    shape(arms, "violet-300", 0.3),
    shape(f.ellipse(0, 0, 15.6, 13.4) + f.ellipse(0, 0, 15, 12.9), "violet-300", 0.85),
    shape(f.ellipse(lx * 5, ly * 5, 3.8, 3.3), "black", 0.95),
    shape(spark(glint, 2.2) + spark(glint + Math.PI, 1.4), "white", 0.85),
  ];
}

/** A tear in space's fabric centred on the anchor: a jagged opening of white light, its torn edges peeling back in pale curled flaps with dark folds, a violet glow hugging its edge and about it, and motes drifting out; about 20 wide and 40 tall at scale 1. */
function rift(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -2, 2);
  const n = 10;
  const left: Point[] = [];
  const right: Point[] = [];
  for (let i = 1; i < n; i += 1) {
    const t = i / n;
    const y = -20 + 40 * t;
    const w = 5.2 * Math.sin(Math.PI * t) ** 0.7;
    const zig = i % 2 === 0 ? 1.15 : 0.75;
    const x = lean * Math.sin(Math.PI * t * 2);
    left.push([x - w * zig * between(rng, 0.85, 1.15), y]);
    right.push([x + w * (2 - zig) * between(rng, 0.85, 1.15), y]);
  }
  const outline = (grow: number): string => f.poly([[0, -20 - grow], ...right.map(([x, y]): Point => [x * (1 + grow / 4), y]), [0, 20 + grow], ...[...left].reverse().map(([x, y]): Point => [x * (1 + grow / 4), y])]);
  let flaps = "";
  let folds = "";
  const flap = (edge: readonly Point[], i: number, side: number): void => {
    const a = edge[i] ?? [0, 0];
    const b = edge[i + 1] ?? [0, 0];
    const reach = between(rng, 2.6, 4);
    const [mx, my] = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    const tip: Point = [mx + side * reach, my + between(rng, -1, 1)];
    flaps += `M${f.p(...a)}Q${f.p(a[0] + side * reach * 0.9, a[1] - 0.6)} ${f.p(...tip)}Q${f.p(mx + side * reach * 0.3, b[1])} ${f.p(...b)}Z`;
    folds += `M${f.p(...a)}Q${f.p(mx + side * 1.4, my)} ${f.p(...b)}Z`;
  };
  const shift = whole(rng, 0, 1);
  for (const i of [1, 4, 6]) flap(left, i + shift, -1);
  for (const i of [2, 5]) flap(right, i - shift, 1);
  const motes = [-1, 1, 1]
    .map((side, k) => f.circle(side * between(rng, 8, 11), -12 + k * 11 + between(rng, -2, 2), between(rng, 0.35, 0.6)))
    .join("");
  return [
    shape(f.ellipse(0, 0, 12, 24), "violet-300", 0.12),
    shape(f.ellipse(0, 0, 8, 21), "white", 0.12),
    shape(outline(1.6), "violet-300", 0.45),
    shape(outline(0), "white", 1, MAIN),
    shape(flaps, "slate-400", 1, INKED),
    shape(folds, "slate-900", 0.35),
    shape(motes, "white", 0.75),
  ];
}

/** A spiral galaxy centred on the anchor, seen tilted: a translucent halo, two or three curling arms with a brighter lane down each and star knots along them, and a bright core; about 30 across at scale 1. */
function galaxy(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -25, 25);
  const squash = between(rng, 0.55, 0.85);
  const view = ([x, y]: Point): Point => turned([x, y * squash], tilt);
  const g: Local = { ...f, poly: (points) => f.poly(points.map(view)) };
  const arms = whole(rng, 2, 3);
  const start = between(rng, 0, 360);
  const sweep = between(rng, 250, 300);
  let body = "";
  let lanes = "";
  let knots = "";
  for (let k = 0; k < arms; k += 1) {
    const from = start + (k * 360) / arms;
    body += spiralArm(g, from, sweep, 2.6, 15, 3.4);
    lanes += spiralArm(g, from, sweep * 0.8, 2.6, 2.6 + 12.4 * 0.8, 1.2);
    for (const t of [0.4, 0.7]) {
      const r = 2.6 + 12.4 * t + between(rng, -0.6, 0.6);
      const a = (from + sweep * t) * DEGREES;
      knots += f.circle(...view([r * Math.cos(a), r * Math.sin(a)]), between(rng, 0.35, 0.55));
    }
  }
  const disc = (r: number): string => f.poly(ellipseArc(0, 0, r, r * squash, tilt, 0, 360, 32));
  return [
    shape(disc(17), "violet-500", 0.1),
    shape(disc(10), "violet-300", 0.14),
    shape(body, "violet-300", 0.5, MAIN),
    shape(lanes, "white", 0.28),
    shape(knots, "white", 0.85),
    shape(f.circle(0, 0, 4.4), "white", 0.22),
    shape(f.circle(0, 0, 2), "white", 0.95),
  ];
}

/** A pungent stink cloud about the anchor: five lumpy translucent green puffs, a darker heart and a pale top, two or three wavy stink lines rising about 9 above it and two flies buzzing; about 24 across and 14 tall at scale 1. */
function stinkCloud(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const puffs: readonly (readonly [number, number, number])[] = [
    [-7, 1.4, 4.4],
    [7, 1, 4.6],
    [-2.6, 3.4, 4],
    [3.6, 3.4, 4],
    [0, -1.6, 5.6],
  ];
  const out = puffs.map(([x, y, r]) => shape(blob(f, rng, x + between(rng, -0.6, 0.6), y + between(rng, -0.5, 0.5), r, 9), "lime-400", 0.24, MAIN));
  out.push(shape(blob(f, rng, 0, 2.4, 5, 8), "green-700", 0.2));
  out.push(shape(blob(f, rng, -1.4, -3.6, 2.6, 7), "lime-200", 0.2));
  const lines = whole(rng, 2, 3);
  let wisps = "";
  for (let k = 0; k < lines; k += 1) {
    const x0 = (k - (lines - 1) / 2) * 5.6 + between(rng, -0.6, 0.6);
    const phase = between(rng, 0, Math.PI * 2);
    const points: Point[] = [];
    for (let i = 0; i <= 6; i += 1) points.push([x0 + 1.1 * Math.sin(phase + i * 1.3), -7 - i * 1.5]);
    wisps += f.band(points, 0.6);
  }
  out.push(shape(wisps, "green-400", 0.7));
  const flies: Point[] = [
    [-between(rng, 7, 11), -between(rng, 6, 10)],
    [between(rng, 7, 11), -between(rng, 7, 11)],
  ];
  out.push(shape(flies.map(([x, y]) => f.ellipse(x - 0.65, y - 0.6, 0.55, 0.35) + f.ellipse(x + 0.65, y - 0.6, 0.55, 0.35)).join(""), "white", 0.55));
  out.push(shape(flies.map(([x, y]) => f.circle(x, y, 0.5)).join(""), "slate-900", 0.9));
  return out;
}

/** R3403000: the weather of space Stinkyscape's later cards added, by id, each with its kind. */
export const COSMOS_KIT = {
  comet: { category: "weather", draw: comet },
  frost: { category: "weather", draw: frost },
  "ice-shard": { category: "weather", draw: iceShard },
  "ice-block": { category: "weather", draw: iceBlock },
  "ticker-tape": { category: "weather", draw: confetti },
  "ring-arc": { category: "weather", draw: ringArc },
  "alarm-light": { category: "weather", draw: alarmLight },
  "orbit-trace": { category: "weather", draw: orbitTrace },
  "speed-streaks": { category: "weather", draw: streaks },
  "surf-wave": { category: "weather", draw: wave },
  "lens-flare": { category: "weather", draw: lensFlare },
  wormhole: { category: "weather", draw: wormhole },
  "space-rift": { category: "weather", draw: rift },
  galaxy: { category: "weather", draw: galaxy },
  "stink-cloud": { category: "weather", draw: stinkCloud },
} as const satisfies Readonly<Record<string, KitPart>>;
