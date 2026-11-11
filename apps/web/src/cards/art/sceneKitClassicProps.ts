// R3352000: the scene kit's Classic props (ART.md, "Scene recipes and kits"), which Classic's scenes (#3352) added
// for the things their briefs name and the kit lacked. sceneKitClassic.ts spreads them into `CLASSIC_KIT`, after
// its creatures. Each part keeps the kit's rules: one to twelve filled shapes drawn in its own frame (`local`),
// varied by the scene's stream alone, every colour a palette.css name none in Radiant's gold window, a glow or a
// wash a stack of translucent shapes, never a gradient, and every shape filled even-odd, so a hole is a second
// sub-path and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import { between, crescent, DEGREES, INKED, leant, local, MAIN, MAIN_INKED, shape, sparkle, spiralArm, turned, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Helpers

/** A closed curve smoothed through the midpoints of `rim`, each point its control. */
function smooth(f: Local, rim: readonly Point[]): string {
  const at = (i: number): Point => rim[i % rim.length] ?? [0, 0];
  const mid = (a: Point, b: Point): string => f.p((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
  let d = `M${mid(at(0), at(1))}`;
  for (let i = 1; i <= rim.length; i += 1) d += `Q${f.p(...at(i))} ${mid(at(i), at(i + 1))}`;
  return `${d}Z`;
}

/** A falling drop of `r`, its point up, its round foot at (x, y). */
function drop(f: Local, x: number, y: number, r: number): string {
  return `M${f.p(x, y - r * 2.6)}C${f.p(x + r * 0.5, y - r * 1.2)} ${f.p(x + r * 1.1, y + r)} ${f.p(x, y + r)}C${f.p(x - r * 1.1, y + r)} ${f.p(x - r * 0.5, y - r * 1.2)} ${f.p(x, y - r * 2.6)}Z`;
}

/** The frame of a part's own piece about (dx, dy) of `at`'s frame: its circles, crescents and spirals centre there. */
function about(at: PartPlace, f: Local, dx: number, dy: number): Local {
  return local({ x: f.x(dx), y: f.y(dy), s: at.s, flip: at.flip });
}

// Objects

/** R3352000: a huge warty pickle standing upright on the anchor, leaning a little the way it faces, brine dripping off it into a puddle at its foot, about 10 wide and 24 tall at scale 1. */
function pickle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0.06, 0.13);
  const bow = between(rng, 0.6, 1.4);
  // A point `t` up the pickle (0 its foot, 1 its tip) and `side` across it (-1 its back edge, 1 its front).
  const P = (t: number, side: number): Point => {
    const half = 4.9 * Math.sin(Math.PI * (0.03 + 0.94 * t)) ** 0.32;
    return leant([side * half + bow * Math.sin(Math.PI * t), -0.4 - 23.4 * t], lean);
  };
  const steps = [0.06, 0.2, 0.34, 0.48, 0.62, 0.76, 0.9];
  const knob = steps.map((_, i) => (i % 2 === 0 ? 1.04 : 0.93) + between(rng, -0.03, 0.03));
  const rim: Point[] = [P(0, 0), ...steps.map((t, i) => P(t, knob[i] ?? 1)), P(1, 0), ...steps.map((t, i) => P(t, -(knob[6 - i] ?? 1))).reverse()];
  const warts: string[] = [];
  for (let i = 0; i < 6; i += 1) {
    const [wx, wy] = P(0.12 + i * 0.15, (i % 2 === 0 ? -0.5 : 0.35) + between(rng, -0.12, 0.12));
    warts.push(f.circle(wx, wy, 0.62));
  }
  const [tx, ty] = P(1, 0);
  const [bx, by] = P(0.3, -1);
  const [fx, fy] = P(0.42, 1);
  return [
    shape(f.ellipse(0.8, 0, 7.6, 1.5), "green-400", 0.45),
    shape(smooth(f, rim), "green-700", 1, MAIN_INKED),
    shape(smooth(f, [...steps.map((t) => P(t, 0.9)), ...steps.map((t) => P(t, 0.38)).reverse()]), "green-900", 0.5),
    shape(f.band([P(0.22, -0.55), P(0.42, -0.6), P(0.62, -0.58), P(0.8, -0.45)], 0.7), "lime-400", 0.6),
    shape(warts.join(""), "green-500", 1),
    shape(f.band([[tx, ty + 0.6], [tx + 0.6, ty - 1]], 1.2), "wood-700", 1, INKED),
    shape(drop(f, bx - 0.5, by + 2.6, 0.62) + drop(f, bx - 1.1, by + 6.4, 0.5) + drop(f, fx + 0.9, fy + 4, 0.55), "lime-200", 0.9),
  ];
}

/** R3352000: a crystal sceptre hovering upright and centred on the anchor, tipped a little, a captured spell coiled like a comet in the open loop of its head, about 9 wide and 30 tall at scale 1. */
function sceptre(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -7, 7);
  const turn = between(rng, 0, 360);
  const T = (x: number, y: number): Point => turned([x, y], tilt);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const head = about(at, f, ...T(0, -9.2));
  const ring = (a: number, r: number): Point => turned([0, r], a + tilt);
  const comet: Point = [2.1 * Math.cos(turn * DEGREES), 2.1 * Math.sin(turn * DEGREES)];
  return [
    shape(head.circle(0, 0, 8.6), "violet-300", 0.14),
    shape(head.circle(0, 0, 5.8), "sky-300", 0.22),
    shape(poly([[-0.7, -4.4], [0.7, -4.4], [1, 3], [0.6, 11], [-0.6, 11], [-1, 3]]), "violet-300", 1, MAIN_INKED),
    shape(poly([[-0.7, -4.4], [0, -4.4], [0, 11], [-0.6, 11], [-1, 3]]), "white", 0.45),
    shape(
      head.band([ring(-62, 4.6), ring(-30, 5.1), ring(0, 5.4), ring(30, 5.1), ring(62, 4.6)], 1.1) +
        poly([[-1.4, 2.2], [0, 1], [1.4, 2.2], [0, 3.4]]) +
        poly([[-1.1, 10.4], [1.1, 10.4], [0.8, 12.8], [0, 14.2], [-0.8, 12.8]]),
      "brass-500",
      1,
      INKED,
    ),
    shape(head.circle(0, 0, 4.3) + head.circle(0, 0, 3.3), "violet-300", 1, MAIN_INKED),
    shape(head.poly([ring(180, 3.6), ring(166, 5.4), ring(180, 6.8), ring(194, 5.4)]), "violet-300", 1, MAIN_INKED),
    shape(spiralArm(head, turn, 330, 2.1, 0.3, 1.5), "sky-300", 0.85),
    shape(spiralArm(head, turn, 330, 2.1, 0.3, 0.55), "white", 0.95),
    shape(head.circle(...comet, 0.85), "white", 1),
    shape(head.poly([ring(180, 4.1), ring(173, 5.4), ring(180, 6.3)]) + head.band([ring(120, 3.8), ring(135, 3.8), ring(150, 3.8)], 0.4), "white", 0.6),
    shape(sparkle(head, 4.8, -4.4, 1.9), "white", 0.9),
  ];
}

/** R3352000: an iron spike trap lying on the floor at the anchor in low perspective, a row of spikes rising from slots behind its pressure plate, a hinge at its back, about 18 wide and 7 tall at scale 1. */
function spikeTrap(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 5, 7);
  const spikes: string[] = [];
  const shades: string[] = [];
  const slots: string[] = [];
  const glints: string[] = [];
  for (let i = 0; i < count; i += 1) {
    const x = -6.4 + (12.8 * i) / (count - 1);
    const h = between(rng, 3.6, 4.6);
    spikes.push(f.poly([[x - 0.8, -3.1], [x, -3.1 - h], [x + 0.8, -3.1]]));
    shades.push(f.poly([[x, -3.1 - h], [x + 0.8, -3.1], [x + 0.15, -3.1]]));
    slots.push(f.poly([[x - 1, -2.8], [x + 0.9, -2.8], [x + 1, -3.4], [x - 0.9, -3.4]]));
    glints.push(f.band([[x - 0.5, -3.5], [x - 0.1, -2.6 - h]], 0.22));
  }
  return [
    shape(f.ellipse(0, 0.2, 10.4, 1.4), "black", 0.3),
    shape(f.poly([[-9, -1], [9, -1], [9, 0], [-9, 0]]), "slate-900", 1, INKED),
    shape(f.poly([[-9, -1], [9, -1], [7.6, -4.4], [-7.6, -4.4]]), "slate-700", 1, MAIN_INKED),
    shape(f.circle(-8, -1.6, 0.36) + f.circle(8, -1.6, 0.36) + f.circle(-6.9, -3.8, 0.3) + f.circle(6.9, -3.8, 0.3), "slate-400", 1),
    shape(f.band([[-7.4, -4.5], [-4.4, -4.5]], 1.1) + f.band([[4.4, -4.5], [7.4, -4.5]], 1.1), "slate-800", 1, INKED),
    shape(f.poly([[-4, -1.3], [4, -1.3], [3.5, -2.5], [-3.5, -2.5]]), "stone-500", 1, INKED),
    shape(slots.join(""), "black", 0.9),
    shape(spikes.join(""), "slate-400", 1, INKED),
    shape(shades.join(""), "slate-700", 0.75),
    shape(glints.join(""), "white", 0.7),
  ];
}

/** R3352000: a copper coil-tower automaton bursting up through the flagstones on the anchor, lightning lashing from its riveted sphere, the longest arc the way it faces, about 14 wide and 34 tall at scale 1 (its arcs beside). */
function teslaCoil(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ball = about(at, f, 0, -29.6);
  const halfAt = (y: number): number => 4.6 - (2.6 * (-2 - y)) / 23.5;
  const rings: string[] = [];
  for (let k = 0; k < 5; k += 1) {
    const y = -5 - k * 4.2;
    const h = halfAt(y) + 0.35;
    rings.push(f.band([[-h, y], [0, y + 0.7], [h, y]], 1.1));
  }
  // A jagged arc from (x0, y0) reaching (dx, dy), zigzagging across its way.
  const arc = (x0: number, y0: number, dx: number, dy: number, n: number): Point[] => {
    const length = Math.hypot(dx, dy);
    const pts: Point[] = [[x0, y0]];
    for (let i = 1; i <= n; i += 1) {
      const zig = (i % 2 === 0 ? -1 : 1) * between(rng, 0.3, 1.8) * (i < n ? 1 : 0.4);
      const along = (i - (i < n ? between(rng, 0, 0.45) : 0)) / n;
      pts.push([x0 + dx * along - (dy / length) * zig, y0 + dy * along + (dx / length) * zig]);
    }
    return pts;
  };
  const arcs = [arc(8.1, -30.8, 16, between(rng, -4, 4), 4), arc(-8.1, -30.8, -9, between(rng, -3, 3), 3), arc(0.8, -34.4, 4.5, -7, 2)];
  const shard = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => [x, y + between(rng, -0.5, 0.5)] as const));
  return [
    shape(f.ellipse(0, 0, 8, 1.8), "slate-900", 0.95),
    shape(f.poly([[-5.6, 0.4], [5.6, 0.4], [4.6, -2], [2, -25.6], [-2, -25.6], [-4.6, -2]]), "brass-600", 1, MAIN_INKED),
    shape(rings.join(""), "orange-400", 1, INKED),
    shape(shard([[-10, 1], [-8, -3], [-5.4, -1.6], [-6, 1.2]]) + shard([[5.8, 1.2], [5.7, -1.8], [8, -3.6], [10.2, 0.8]]) + shard([[-2.6, 2.2], [-1.2, 0.6], [1.8, 1], [1.2, 2.4]]), "stone-500", 1, INKED),
    shape(f.band([[-3.6, -29.4], [-6.5, -30.5]], 0.8) + f.band([[3.6, -29.4], [6.5, -30.5]], 0.8) + f.circle(-7.5, -30.8, 0.75) + f.circle(7.5, -30.8, 0.75), "slate-400", 1, INKED),
    shape(ball.circle(0, 0, 4.4), "brass-600", 1, MAIN_INKED),
    shape(f.poly([[2.6, 0.4], [5.6, 0.4], [4.6, -2], [2, -25.4], [0.8, -25.4]]) + crescent(ball, 4.3, 0.4), "brass-800", 0.5),
    shape([-2.8, 0, 2.8].map((x) => ball.poly([[x, 2.2 - Math.abs(x) * 0.3], [x + 0.4, 2.6 - Math.abs(x) * 0.3], [x, 3 - Math.abs(x) * 0.3], [x - 0.4, 2.6 - Math.abs(x) * 0.3]])).join(""), "slate-900", 0.85),
    shape(ball.ellipse(-2, -2, 1.1, 0.7), "white", 0.6),
    shape(arcs.map((pts) => f.band(pts, 2.2)).join(""), "violet-300", 0.45),
    shape(arcs.map((pts) => f.band(pts, 0.6)).join(""), "white", 0.95),
  ];
}

/** R3352000: a spider web centred on the anchor, eight irregular threads and a spiral strung between them drooping lower the farther out it runs, dew on it, about 22 across at scale 1. */
function web(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rings = whole(rng, 3, 4);
  const sag = between(rng, 0.08, 0.14);
  const spin = between(rng, 0, 45);
  const angles: number[] = [];
  const reach: number[] = [];
  for (let k = 0; k < 8; k += 1) {
    angles.push((spin + k * 45 + between(rng, -7, 7)) * DEGREES);
    reach.push(between(rng, 10, 11.8));
  }
  // A point `r` out along the thread at angle `a`, `across` it to the side, sagged: the farther out, the lower.
  const S = (a: number, r: number, across = 0): Point => [r * Math.cos(a) - across * Math.sin(a), r * Math.sin(a) + across * Math.cos(a) + sag * r];
  const star: Point[] = [];
  angles.forEach((a, k) => star.push(S(a - 22.5 * DEGREES, 0.45), S(a, reach[k] ?? 10, -0.18), S(a, reach[k] ?? 10, 0.18)));
  const spiral: Point[] = [];
  const turns = rings * 8;
  for (let i = 0; i <= turns; i += 1) spiral.push(S(angles[i % 8] ?? 0, 1.6 + (7.8 * i) / turns));
  const dew = [whole(rng, 6, 9), whole(rng, 12, 15), whole(rng, 17, 22)].map((i) => {
    const [x, y] = spiral[Math.min(i, turns)] ?? [0, 0];
    return f.circle(x, y, 0.55);
  });
  return [shape(f.poly(star), "slate-100", 0.7, MAIN), shape(f.band(spiral, 0.36), "slate-100", 0.75, MAIN), shape(dew.join(""), "white", 0.9)];
}

/** R3352000: a big glass alchemist's vat standing on the anchor, iron-banded on riveted feet, its brew bubbling round a floating homunculus, a piped lid on it, about 14 wide and 25 tall at scale 1. */
function vat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const level = between(rng, -15.6, -13.6);
  const wave = between(rng, 0.4, 0.8);
  const bob = between(rng, -0.8, 0.8);
  const k = between(rng, 0.85, 1.15);
  const H = (x: number, y: number): string => f.p(0.3 + x * k, -9 + bob + y * k);
  const surface = (dy: number): string => `M${f.p(-5.8, level + dy)}Q${f.p(-2.9, level - wave + dy)} ${f.p(0, level + dy)}Q${f.p(2.9, level + wave + dy)} ${f.p(5.8, level + dy)}`;
  return [
    shape(f.poly([[-6.8, -2.2], [-4.6, -2.2], [-4.9, 0], [-7.1, 0]]) + f.poly([[4.6, -2.2], [6.8, -2.2], [7.1, 0], [4.9, 0]]), "slate-800", 1, INKED),
    shape(f.poly([[-6.1, -19.2], [6.1, -19.2], [6.1, -4], [-6.1, -4]]), "sky-300", 0.25, INKED),
    shape(`${surface(0)}L${f.p(5.8, -4.2)}L${f.p(-5.8, -4.2)}Z`, "green-400", 0.85, MAIN),
    shape(`${surface(0)}L${f.p(5.8, level + 0.7)}Q${f.p(2.9, level + wave + 0.5)} ${f.p(0, level + 0.5)}Q${f.p(-2.9, level - wave + 0.5)} ${f.p(-5.8, level + 0.7)}Z`, "green-200", 0.55),
    shape(`${f.circle(0.3 + 0.2 * k, -11.9 + bob - 0.3 * k, 1.4 * k)}M${H(-0.9, -0.8)}C${H(1.5, -1.4)} ${H(2.2, 1.8)} ${H(0.4, 2.6)}C${H(-1.2, 3.2)} ${H(-2.4, 1.4)} ${H(-0.9, -0.8)}Z`, "slate-900", 0.6),
    shape(f.circle(-3.6, -7, 0.8) + f.circle(-2.4, level + 2.6, 0.55) + f.circle(3.4, -9.6, 0.65) + f.circle(2.6, level + 2, 0.4), "green-200", 0.8),
    shape(f.band([[-4.8, -17.8], [-4.9, -6]], 0.8) + f.band([[4.6, -17.6], [4.7, -15.4]], 0.4), "white", 0.45),
    shape(f.poly([[-6.8, -21.2], [6.8, -21.2], [6.8, -19], [-6.8, -19]]) + f.poly([[-6.8, -4.4], [6.8, -4.4], [6.8, -2.2], [-6.8, -2.2]]), "slate-700", 1, INKED),
    shape(f.circle(-4.8, -20.1, 0.35) + f.circle(4.8, -20.1, 0.35) + f.circle(-4.8, -3.3, 0.35) + f.circle(4.8, -3.3, 0.35) + f.circle(-5.8, -1.1, 0.3) + f.circle(5.8, -1.1, 0.3), "slate-400", 1),
    shape(`M${f.p(-6.4, -21.2)}Q${f.p(0, -24.4)} ${f.p(6.4, -21.2)}Z`, "slate-800", 1, INKED),
    shape(f.band([[2.4, -22.4], [2.4, -24.4], [6.6, -24.4], [6.6, -23]], 1.1), "slate-700", 1, INKED),
  ];
}

/** R3352000: a stone chalice standing on the anchor, brimming with green plague foam (green-400) that bulges over its rim and drips down its bowl, plague beads (lime-400) ringing it under the rim, its bowl (main, stone-500) on a knotted stem and a round foot; about 13 wide and 18 tall at scale 1. */
function chalice(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 0, 1.2);
  const drip = [between(rng, 2.2, 4.4), between(rng, 1.4, 3.2)] as const;
  return [
    shape(`M${f.p(-4.6, 0)}Q${f.p(-4.4, -1.6)} ${f.p(-0.9, -1.8)}L${f.p(-0.8, -7.1)}L${f.p(0.8, -7.1)}L${f.p(0.9, -1.8)}Q${f.p(4.4, -1.6)} ${f.p(4.6, 0)}Z`, "stone-500", 1, INKED),
    shape(f.ellipse(0, -4.4, 1.7, 0.8), "stone-500", 1, INKED),
    shape(`M${f.p(-6.2, -14)}C${f.p(-6.2, -10)} ${f.p(-3.2, -7.1)} ${f.p(0, -7)}C${f.p(3.2, -7.1)} ${f.p(6.2, -10)} ${f.p(6.2, -14)}Q${f.p(0, -15.2)} ${f.p(-6.2, -14)}Z`, "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(1.2, -14.4)}Q${f.p(6.2, -14.2)} ${f.p(6.2, -13)}C${f.p(6, -9.6)} ${f.p(3.4, -7.4)} ${f.p(0.6, -7.1)}Q${f.p(3, -10)} ${f.p(1.2, -14.4)}Z`, "slate-700", 0.35),
    shape(f.band([[-4.9, -12.8], [-4.2, -10.4], [-2.6, -8.6]], 0.6) + f.band([[-0.9, -6.4], [-0.9, -2.6]], 0.3), "white", 0.4),
    shape(`M${f.p(-6.7, -13.7)}Q${f.p(-7.3, -16.2)} ${f.p(-4.5, -16.3 - h * 0.5)}Q${f.p(-3.2, -18.6 - h)} ${f.p(-0.6, -17.5 - h)}Q${f.p(1.4, -19.6 - h)} ${f.p(3.4, -17.3 - h * 0.6)}Q${f.p(6.6, -17.6)} ${f.p(6.7, -14.4)}Q${f.p(0, -12.6)} ${f.p(-6.7, -13.7)}Z`, "green-400", 1, INKED),
    shape(f.circle(-3.6, -16.2 - h * 0.5, 1.1) + f.circle(-0.4, -17.4 - h, 0.9) + f.circle(2.8, -16.6 - h * 0.6, 0.8), "green-200", 0.6),
    shape(f.band([[-4.8, -13.4], [-5, -13.4 + drip[0]]], 0.9) + f.band([[4, -13.2], [4.2, -13.2 + drip[1]]], 0.8), "green-400", 0.9),
    shape(f.circle(-3.9, -11.6, 0.55) + f.circle(-1.9, -11.2, 0.55) + f.circle(0.2, -11.1, 0.55) + f.circle(2.3, -11.3, 0.55), "lime-400", 1),
    shape(f.circle(-1.8, -19.6 - h, 0.5) + f.circle(1.6, -20.6 - h, 0.4) + f.circle(4.6, -18.4, 0.35), "green-200", 0.7),
  ];
}

// Terrain

/** R3352000: a blast crater in the ground centred on the anchor, its shallow hollow shaded on the left and lit on the right about a dark smouldering pit half hidden by its near lip, a rubble rim, a scorch star and thrown rocks round it, about 44 wide and 13 tall at scale 1. */
function blastCrater(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  // A point at angle `a` round an ellipse of rx by ry, pushed out by a factor between `lo` and `hi`.
  const round = (a: number, rx: number, ry: number, lo: number, hi: number): Point => {
    const r = between(rng, lo, hi);
    return [rx * r * Math.cos(a * DEGREES), ry * r * Math.sin(a * DEGREES)];
  };
  const scorch = Array.from({ length: 14 }, (_, i) => round(i * (360 / 14), 27, 8.4, i % 2 === 0 ? 0.94 : 0.74, i % 2 === 0 ? 1.06 : 0.78));
  const hollow = Array.from({ length: 12 }, (_, i) => round(i * 30, 20.4, 6, 0.95, 1.05));
  // The near lip: the hollow's front edge, its crest broken over the pit.
  const lip: Point[] = [[19.2, 0.8]];
  for (let i = 1; i < 6; i += 1) lip.push([19.6 * Math.cos(i * 30 * DEGREES), 6.4 * Math.sin(i * 30 * DEGREES)]);
  lip.push([-19.2, 0.8]);
  for (let i = 1; i < 6; i += 1) {
    const x = -19.2 + (38.4 * i) / 6;
    lip.push([x, 2.8 * Math.sqrt(1 - (x / 20.4) ** 2) + between(rng, -0.6, 0.3)]);
  }
  // An inner slope: a crescent inside the hollow, `side` -1 its left and 1 its right, `inner` the hollow's width it leaves.
  const slope = (side: 1 | -1, inner: number): string => {
    const turn = side === 1 ? f.sweep(1) : f.sweep(0);
    const back = side === 1 ? f.sweep(0) : f.sweep(1);
    return `M${f.p(0, -5.2)}A${fmt(f.r(19))} ${fmt(f.r(5.2))} 0 0 ${turn} ${f.p(0, 5.2)}A${fmt(f.r(19 * inner))} ${fmt(f.r(5.2))} 0 0 ${back} ${f.p(0, -5.2)}Z`;
  };
  // A rock of half-width `w` sitting about (x, y), nudged along the ground by the stream; spaced so no two touch.
  const rock = (x0: number, y: number, w: number): Point[] => {
    const x = x0 + between(rng, -1.1, 1.1);
    const h = w * between(rng, 0.55, 0.95);
    return [[x - w, y], [x - w * 0.7, y - h * 0.7], [x - w * 0.1, y - h], [x + w * 0.8, y - h * 0.6], [x + w, y + 0.3]];
  };
  // A rock on the rim's back (`row` -1) or front (`row` 1) at `x`.
  const rimRock = (x: number, row: 1 | -1, w: number): Point[] => rock(x, row * 5.8 * Math.sqrt(1 - (x / 19.6) ** 2) + 0.8, w);
  const rocks = [rimRock(-15, -1, 1.8), rimRock(-6, -1, 1.3), rimRock(3, -1, 1.6), rimRock(12, -1, 1.9), rimRock(-18, 1, 1.8), rimRock(-11, 1, 2.4), rimRock(-3, 1, 2.8), rimRock(6, 1, 2), rimRock(14.5, 1, 2.6)];
  const thrown = [rock(-25, 1.6, 1), rock(23.6, 3, 1.1), rock(4.6, 9.4, 0.9)];
  return [
    shape(f.poly(scorch), "black", 0.35),
    shape(smooth(f, hollow), "stone-500", 1, MAIN),
    shape(slope(-1, 0.55), "black", 0.3),
    shape(slope(1, 0.75), "white", 0.14),
    shape(f.ellipse(1, 1, 9.6, 2.5), "black", 0.8),
    shape(f.ellipse(1, 1.8, 5, 0.9), "orange-400", 0.35),
    shape(f.poly(lip), "stone-500", 1, MAIN_INKED),
    shape([...rocks, ...thrown].map((r) => f.poly(r)).join(""), "stone-500", 1, MAIN_INKED),
    shape(rocks.slice(4).map(([a, b, c]) => f.poly([a ?? [0, 0], b ?? [0, 0], c ?? [0, 0]])).join(""), "white", 0.25),
  ];
}

/** R3352000: In Too Deep's bottomless quest-cavern shaft seen steeply from above, the centre of its mouth on the anchor: five rings of rock ledge sinking smaller, lower and darker to a violet-lit void about 8 below the anchor, a spiral stair winding down them and shrine niches glowing on the far walls of its top three levels, about 66 wide and 46 tall at scale 1. */
function spiralShaft(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const q = 0.75;
  // How far down the shaft's centre has sunk at depth `k` (0 the mouth, 5 the void), and its scale there.
  const sink = (k: number): number => 10 * (1 - q ** k);
  // Level `k`'s ellipse, `inset` 1 its ledge's inner edge (wider on the far side, which faces the viewer), `grow` its scale.
  const E = (k: number, inset = 0, grow = 1): string =>
    f.ellipse(0, sink(k) + inset * 0.8 * q ** k, (30 - inset * 4.5) * q ** k * grow, (20 - inset * 2.8) * q ** k * grow);
  const rim: Point[] = [];
  for (let i = 0; i < 14; i += 1) {
    const a = ((i * 360) / 14) * DEGREES;
    const r = between(rng, 1.06, 1.14);
    rim.push([30 * r * Math.cos(a), 20 * r * Math.sin(a)]);
  }
  // The stair: quarter turns of a ribbon from the mouth down to the void, each drawn as an elliptical arc.
  const start = whole(rng, 0, 3) * 90 + 45;
  const turns = 7;
  const along = (i: number, rx: number, ry: number): Point => {
    const k = (4.4 * i) / turns;
    const a = (start + 90 * i) * DEGREES;
    return [rx * q ** k * Math.cos(a), sink(k) + ry * q ** k * Math.sin(a)];
  };
  const radii = (i: number, rx: number, ry: number): string => {
    const k = (4.4 * (i + 0.5)) / turns;
    return `${fmt(f.r(rx * q ** k))} ${fmt(f.r(ry * q ** k))}`;
  };
  let stair = `M${f.p(...along(0, 28.8, 19.2))}`;
  for (let i = 0; i < turns; i += 1) stair += `A${radii(i, 28.8, 19.2)} 0 0 ${f.sweep(1)} ${f.p(...along(i + 1, 28.8, 19.2))}`;
  stair += `L${f.p(...along(turns, 26, 17.2))}`;
  for (let i = turns - 1; i >= 0; i -= 1) stair += `A${radii(i, 26, 17.2)} 0 0 ${f.sweep(0)} ${f.p(...along(i, 26, 17.2))}`;
  const niches: string[] = [];
  const glows: string[] = [];
  for (let w = 0; w < 3; w += 1) {
    const k = w + 0.62;
    const a = (270 + between(rng, -40, 40)) * DEGREES;
    const x = 27.6 * q ** k * Math.cos(a);
    const y = sink(k) + 18.2 * q ** k * Math.sin(a);
    const r = 2 * q ** k;
    niches.push(f.poly([[x - r * 0.6, y + r * 0.7], [x - r * 0.6, y - r * 0.2], [x, y - r * 0.8], [x + r * 0.6, y - r * 0.2], [x + r * 0.6, y + r * 0.7]]));
    glows.push(f.circle(x, y, r * 1.6));
  }
  return [
    shape(f.poly(rim), "slate-700", 1, MAIN_INKED),
    shape(E(0), "black", 0.5),
    shape([E(1), E(1, 1), E(2), E(2, 1), E(3), E(3, 1), E(4), E(4, 1)].join(""), "slate-700", 1, MAIN_INKED),
    shape(`${stair}Z`, "stone-500", 1, INKED),
    shape(E(2), "black", 0.22),
    shape(E(3), "black", 0.25),
    shape(E(5, 0, 1.1), "black", 0.9),
    shape(E(5, 0, 0.85), "blue-900", 0.45),
    shape(E(4, 0, 0.95), "violet-500", 0.2),
    shape(E(5, 0, 0.6), "violet-300", 0.2),
    shape(glows.join(""), "cyan-400", 0.3),
    shape(niches.join(""), "violet-300", 1),
  ];
}

export const CLASSIC_PROPS_KIT = {
  pickle: { category: "objects", draw: pickle },
  sceptre: { category: "objects", draw: sceptre },
  "spike-trap": { category: "objects", draw: spikeTrap },
  "tesla-coil": { category: "objects", draw: teslaCoil },
  web: { category: "objects", draw: web },
  vat: { category: "objects", draw: vat },
  chalice: { category: "objects", draw: chalice },
  "blast-crater": { category: "terrain", draw: blastCrater },
  "spiral-shaft": { category: "terrain", draw: spiralShaft },
} as const satisfies Readonly<Record<string, KitPart>>;
