// R3393000, R3393001: the scene kit's yard props that the Training Grounds and Legacy scenes (#3393) added: an
// archery target, and each Legacy card's yard touch (traininggrounds.ts `LEGACY_TOUCHES`) that the kit lacked,
// drawn anew as a kit part: a rope cordon, a practice sword stuck in sand, a trail of paw prints, range flags, a
// straw heap, a hammock, a scratching post, a bucket of carrots, a lifting stone, a water barrel, a pell post and a
// straw scarecrow; and a fluffy white bunny with a hypnotic eye. The other touches are parts the kit held already
// (sceneRecipesLegacy.ts `LEGACY_TOUCH_PARTS`). sceneKit.ts spreads the table into `SCENE_KIT`. Each part keeps the
// kit's rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream alone,
// every colour a palette name out of Radiant's gold window, a glow a stack of translucent shapes, and no two
// sub-paths of one shape overlapping but for a hole.

import { between, DEGREES, INKED, leant, local, MAIN, MAIN_INKED, shape, whole, type Local, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

/** A flattened spiral about (cx, cy) as a band, `r` its outer reach, `squash` its height's share of its width. */
function swirl(f: Local, cx: number, cy: number, r: number, squash: number, start: number, width: number): string {
  const steps = 12;
  const points: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const angle = (start + 320 * t) * DEGREES;
    const reach = r * (0.2 + 0.8 * t);
    points.push([cx + reach * Math.cos(angle), cy + reach * Math.sin(angle) * squash]);
  }
  return f.band(points, width);
}

/** An archery target board on two splayed legs, its foot at the anchor: four rings round a red heart; about 10 wide and 14 tall at scale 1. */
function target(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.6, 0.6);
  const cx = tilt;
  const cy = -9;
  return [
    shape(f.band([[-3.6, 0], [cx - 0.8, cy]], 0.9) + f.band([[3.6, 0], [cx + 0.8, cy]], 0.9), "wood-700", 1, INKED),
    shape(f.band([[cx, cy + 2], [cx + 1.6, 0.2]], 0.8), "wood-800", 1),
    shape(f.circle(cx, cy, 5) + f.circle(cx, cy, 3.6), "slate-100", 1, MAIN_INKED),
    shape(f.circle(cx, cy, 3.6) + f.circle(cx, cy, 2.3), "blue-700", 1),
    shape(f.circle(cx, cy, 2.3) + f.circle(cx, cy, 1.1), "white", 1),
    shape(f.circle(cx, cy, 1.1), "red-600", 1, INKED),
  ];
}

/** Two brass stanchions on round feet, 20 apart about the anchor, a red rope sagging between them; about 24 wide and 13 tall at scale 1. */
function ropeCordon(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 2.2, 3.6);
  const lean = between(rng, -0.04, 0.04);
  const half = 10;
  const top = -11;
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const post = (x: number): string => f.poly([P(x - 0.6, -0.6), P(x - 0.6, top), P(x + 0.6, top), P(x + 0.6, -0.6)]);
  const rope: Point[] = [];
  for (let i = 0; i <= 8; i += 1) {
    const t = i / 8;
    rope.push(P(-half + 2 * half * t, top + 1.4 + sag * 4 * t * (1 - t)));
  }
  return [
    shape(f.ellipse(0, 0.3, 13, 0.9), "black", 0.22),
    shape(f.ellipse(-half, -0.3, 2.2, 0.8) + f.ellipse(half, -0.3, 2.2, 0.8), "slate-700", 1, INKED),
    shape(post(-half) + post(half), "brass-600", 1, INKED),
    shape(f.circle(...P(-half, top - 0.7), 1.1) + f.circle(...P(half, top - 0.7), 1.1), "brass-500", 1, INKED),
    shape(f.band(rope, 1), "red-600", 1, MAIN_INKED),
    shape(f.band(rope.map(([x, y]): Point => [x, y - 0.25]), 0.28), "red-300", 0.6),
    shape(f.band([P(-half - 0.2, -1.4), P(-half - 0.2, top + 0.6)], 0.25) + f.band([P(half - 0.2, -1.4), P(half - 0.2, top + 0.6)], 0.25), "white", 0.35),
  ];
}

/** A wooden practice sword stuck point down in a mound of sand on the anchor, leant either way; about 6 wide and 18 tall at scale 1. */
function practiceSword(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0.08, 0.2) * (rng() < 0.5 ? -1 : 1);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const p = (dx: number, dy: number): string => f.p(...P(dx, dy));
  const guard = -11.6 - between(rng, 0, 0.8);
  return [
    shape(f.ellipse(0.6, 0.2, 6, 1), "black", 0.2),
    shape(`M${p(-0.95, 0)}L${p(-0.95, guard)}L${p(0.95, guard)}L${p(0.95, 0)}Z`, "wood-600", 1, MAIN_INKED),
    shape(f.band([P(0, -0.6), P(0, guard + 0.8)], 0.32), "wood-800", 0.5),
    shape(`M${p(-3.2, guard - 0.1)}L${p(3.2, guard - 0.1)}L${p(2.8, guard - 1.2)}L${p(-2.8, guard - 1.2)}Z`, "wood-800", 1, INKED),
    shape(f.band([P(0, guard - 1.2), P(0, guard - 4.6)], 1.2), "slate-700", 1, INKED),
    shape([0.6, 1.5, 2.4].map((k) => f.band([P(-0.6, guard - 1.2 - k), P(0.6, guard - 1.6 - k)], 0.22)).join(""), "wood-800", 0.7),
    shape(f.circle(...P(0, guard - 5.4), 0.9), "wood-700", 1, INKED),
    shape(`M${f.p(-5, 0.4)}Q${f.p(-2.4, -1.6)} ${f.p(0, -1.9)}Q${f.p(2.6, -1.6)} ${f.p(5, 0.4)}Z`, "brass-500", 1, INKED),
    shape(f.circle(-3, -0.1, 0.25) + f.circle(3.3, 0, 0.22) + f.circle(1.6, -1, 0.2), "wood-600", 0.6),
  ];
}

/** A trail of four to six paw prints across the ground about the anchor toward its facing, a fainter trail doubling it behind; about 26 wide at scale 1. */
function pawPrints(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const prints = whole(rng, 4, 6);
  const toes: Point[] = [
    [1.5, -1.1],
    [2.15, -0.4],
    [2.15, 0.45],
    [1.5, 1.2],
  ];
  const pads: string[] = [];
  const digits: string[] = [];
  const ghost: string[] = [];
  for (let i = 0; i < prints; i += 1) {
    const x = -12 + (22 * i) / (prints - 1) + between(rng, -0.5, 0.5);
    const y = (i % 2 === 0 ? 0 : -2.2) + between(rng, -0.3, 0.3);
    pads.push(f.ellipse(x, y, 1.2, 0.85));
    for (const [tx, ty] of toes) digits.push(f.ellipse(x + tx, y + ty * 0.8, 0.38, 0.3));
    ghost.push(f.ellipse(x - 2.6, y - 1.1, 1, 0.7));
  }
  return [shape(ghost.join(""), "wood-800", 0.3), shape(pads.join(""), "wood-800", 0.75, MAIN), shape(digits.join(""), "wood-800", 0.75, MAIN)];
}

/** Three marker poles in a row about the anchor, tallest first, each flying a pennant toward its facing; about 22 wide and 14 tall at scale 1. */
function rangeFlags(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const poles = [-9, 0, 9].map((x, k) => ({ x, h: 13.5 - k * 2 + between(rng, -0.6, 0.6), wave: between(rng, -0.8, 0.8) }));
  return [
    shape(poles.map(({ x }) => f.ellipse(x, 0.1, 1.6, 0.5)).join(""), "black", 0.22),
    shape(poles.map(({ x }) => f.ellipse(x, -0.2, 1.1, 0.5)).join(""), "stone-500", 1, INKED),
    shape(poles.map(({ x, h }) => f.band([[x, 0], [x, -h]], 0.5)).join(""), "wood-800", 1, INKED),
    shape(poles.map(({ x, h, wave }) => f.poly([[x + 0.3, -h], [x + 5.4, -h + 1.5 + wave * 0.4], [x + 0.3, -h + 3]])).join(""), "red-600", 1, MAIN_INKED),
    shape(poles.map(({ x, h, wave }) => f.band([[x + 0.5, -h + 1.5], [x + 3.6, -h + 1.5 + wave * 0.3]], 0.45)).join(""), "white", 0.8),
    shape(poles.map(({ x, h }) => f.circle(x, -h - 0.4, 0.45)).join(""), "slate-400", 1),
  ];
}

/** A heap of loose straw on the anchor, strands sticking out of its top and a few lying loose before it; about 22 wide and 9 tall at scale 1. */
function strawHeap(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 10, 11.5);
  const h = between(rng, 7, 8.6);
  const bumps = 5;
  let top = `M${f.p(-w, 0)}`;
  const crest: Point[] = [];
  for (let i = 0; i <= bumps; i += 1) {
    const t = i / bumps;
    const x = -w + 2 * w * t;
    crest.push([x, -h * Math.sin(Math.PI * (0.06 + 0.88 * t)) - between(rng, -0.5, 0.5)]);
  }
  for (let i = 0; i < crest.length - 1; i += 1) {
    const [x0, y0] = crest[i] ?? [0, 0];
    const [x1, y1] = crest[i + 1] ?? [0, 0];
    top += `Q${f.p((x0 + x1) / 2, Math.min(y0, y1) - 1)} ${f.p(x1, y1)}`;
  }
  const strands: string[] = [];
  for (let i = 1; i < crest.length - 1; i += 1) {
    const [x, y] = crest[i] ?? [0, 0];
    const tip = between(rng, -1.2, 1.2);
    strands.push(f.band([[x, y + 0.6], [x + tip, y - 2.2]], 0.3));
  }
  const loose = [-6, 1, 7].map((x) => f.band([[x + between(rng, -1, 1), 1.2], [x + 2.6, 0.6 + between(rng, -0.4, 0.4)]], 0.28)).join("");
  return [
    shape(f.ellipse(0, 0.4, w + 2, 1.2), "black", 0.22),
    shape(`${top}L${f.p(w, 0)}Z`, "brass-500", 1, MAIN_INKED),
    shape(`M${f.p(-w + 1, 0)}Q${f.p(0, -h * 0.45)} ${f.p(w - 1, 0)}Z`, "brass-600", 0.55),
    shape([-0.5, 0.5].map((side) => f.band([[side * w * 0.8, -1], [side * w * 0.45, -h * 0.55]], 0.3)).join("") + f.band([[0, -1.4], [0.4, -h * 0.7]], 0.3), "wood-600", 0.5),
    shape(strands.join(""), "brass-500", 1, INKED),
    shape(loose, "brass-500", 0.9),
  ];
}

/** A striped canvas hammock slung between two posts 26 apart about the anchor, sagging between them; about 30 wide and 17 tall at scale 1. */
function hammock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 5, 6.4);
  const end = -11.5;
  const lean = between(rng, 0, 0.05);
  const post = (x: number, side: number): string => f.poly([leant([x - 0.7, 0], -side * lean), leant([x - 0.7, -16.5], -side * lean), leant([x + 0.7, -16.5], -side * lean), leant([x + 0.7, 0], -side * lean)]);
  const deep = (depth: number): number => end + 2 * depth;
  const stripe: Point[] = [];
  for (let i = 0; i <= 8; i += 1) {
    const t = i / 8;
    stripe.push([-10.4 + 20.8 * t, end + (sag - 1.2) * 4 * t * (1 - t)]);
  }
  return [
    shape(f.ellipse(0, 0.3, 15, 1), "black", 0.22),
    shape(post(-13, -1) + post(13, 1), "wood-700", 1, INKED),
    shape(f.band([leant([-13, -15.6], lean), [-10.6, end]], 0.3) + f.band([leant([13, -15.6], -lean), [10.6, end]], 0.3), "wood-800", 1),
    shape(`M${f.p(-10.6, end)}Q${f.p(0, deep(sag - 2.6))} ${f.p(10.6, end)}Q${f.p(0, deep(sag))} ${f.p(-10.6, end)}Z`, "slate-100", 1, MAIN_INKED),
    shape(f.band(stripe, 0.6), "red-600", 0.85),
    shape(f.band([[-3, end + sag - 0.6], [3, end + sag - 0.6]], 0.3), "slate-400", 0.5),
  ];
}

/** A cat's scratching post on the anchor: a rope-wound column on a board base, a cushioned perch on top and a toy dangling; about 12 wide and 22 tall at scale 1. */
function scratchingPost(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -1.2, 1.2);
  const wraps: string[] = [];
  for (let y = -3.4; y > -19; y -= 2.2) wraps.push(f.band([[-2.1, y], [2.1, y - 0.9]], 0.55));
  const frays = [-15.6, -11.2, -6.8].map((y, k) => {
    const side = k % 2 === 0 ? -1 : 1;
    return f.band([[side * 2.1, y], [side * (3.4 + between(rng, 0, 0.8)), y + between(rng, -0.6, 0.6)]], 0.3);
  });
  return [
    shape(f.ellipse(0, 0.3, 8, 0.9), "black", 0.22),
    shape(f.poly([[-6.4, 0], [-6.4, -1.6], [6.4, -1.6], [6.4, 0]]), "wood-700", 1, INKED),
    shape(f.poly([[-2.2, -1.6], [-2.2, -20], [2.2, -20], [2.2, -1.6]]), "brass-600", 1, MAIN_INKED),
    shape(wraps.join(""), "wood-600", 0.7),
    shape(frays.join(""), "brass-500", 1),
    shape(f.poly([[-5.6, -20], [-5.6, -21.6], [5.6, -21.6], [5.6, -20]]), "wood-700", 1, INKED),
    shape(`M${f.p(-5, -21.6)}Q${f.p(0, -24)} ${f.p(5, -21.6)}Z`, "red-600", 1, INKED),
    shape(f.band([[4.6, -20], [4.8 + swing, -14.6]], 0.2), "slate-100", 0.9),
    shape(f.circle(4.8 + swing, -13.8, 0.9), "red-400", 1, INKED),
  ];
}

/** A wooden bucket of carrots on the anchor, iron-hooped, its handle up behind and three carrots poking out; about 9 wide and 11 tall at scale 1. */
function bucket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rim = -6.5;
  const carrots = [-2.2, 0.2, 2.4].map((x) => ({ x: x + between(rng, -0.3, 0.3), tilt: between(rng, -1.2, 1.2), tall: between(rng, 3.6, 5) }));
  return [
    shape(f.ellipse(0, 0.3, 5.6, 0.8), "black", 0.22),
    shape(f.band([[-4.2, rim], [-3, rim - 4.6], [0, rim - 5.8], [3, rim - 4.6], [4.2, rim]], 0.4), "slate-700", 1),
    shape(f.ellipse(0, rim, 4.2, 0.9), "wood-900", 1),
    shape(carrots.map(({ x, tilt, tall }) => f.poly([[x - 0.9, rim + 0.6], [x + tilt, rim - tall], [x + 0.9, rim + 0.6]])).join(""), "orange-400", 1, INKED),
    shape(carrots.map(({ x, tilt, tall }) => f.band([[x + tilt, rim - tall], [x + tilt - 0.8, rim - tall - 1.8]], 0.32) + f.band([[x + tilt + 0.1, rim - tall - 0.2], [x + tilt + 0.9, rim - tall - 1.7]], 0.32)).join(""), "green-500", 1),
    shape(f.poly([[-4.2, rim], [4.2, rim], [3.4, 0], [-3.4, 0]]), "wood-600", 1, MAIN_INKED),
    shape([-1.6, 0, 1.6].map((x) => f.band([[x * 1.15, rim + 0.5], [x, -0.4]], 0.25)).join(""), "wood-800", 0.45),
    shape(f.band([[-4.05, rim + 1.4], [4.05, rim + 1.4]], 0.6) + f.band([[-3.55, -1.3], [3.55, -1.3]], 0.6), "slate-700", 1, INKED),
  ];
}

/** A round atlas stone resting on the anchor, chalk-dusted and cracked; about 11 wide and 9 tall at scale 1. */
function liftingStone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = between(rng, 4.6, 5.2);
  const crack = between(rng, -1, 1);
  return [
    shape(f.ellipse(0, 0.2, r + 1.4, 0.9), "black", 0.25),
    shape(f.ellipse(0, -r * 0.88, r, r * 0.88), "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(-r * 0.92, -r * 0.6)}Q${f.p(0, r * 0.1)} ${f.p(r * 0.92, -r * 0.6)}Q${f.p(0, -r * 0.2)} ${f.p(-r * 0.92, -r * 0.6)}Z`, "slate-700", 0.4),
    shape(f.ellipse(-r * 0.35, -r * 1.25, r * 0.38, r * 0.22), "slate-100", 0.4),
    shape(f.band([[r * 0.2, -r * 1.7], [r * 0.05 + crack * 0.5, -r * 1.2], [r * 0.4, -r * 0.8]], 0.3), "slate-800", 0.7),
    shape(f.ellipse(r * 0.45, -r * 0.5, r * 0.3, r * 0.16), "white", 0.3),
  ];
}

/** A hooped water barrel on the anchor, its water swirling at the top; about 12 wide and 14 tall at scale 1. */
function barrel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 5;
  const h = 13;
  const turn = between(rng, 0, 360);
  const hoop = (y: number): string => f.band([[-w - 0.55, y], [0, y + 0.4], [w + 0.55, y]], 0.9);
  return [
    shape(f.ellipse(0, 0.3, w + 2, 0.9), "black", 0.22),
    shape(`M${f.p(-w, 0)}Q${f.p(-w - 1.5, -h / 2)} ${f.p(-w, -h)}L${f.p(w, -h)}Q${f.p(w + 1.5, -h / 2)} ${f.p(w, 0)}Z`, "wood-600", 1, MAIN_INKED),
    shape([-2.6, 0, 2.6].map((x) => f.band([[x, -0.6], [x * 1.15, -h / 2], [x, -h + 0.6]], 0.25)).join(""), "wood-800", 0.45),
    shape(hoop(-2.6) + hoop(-h + 2.6), "slate-700", 1, INKED),
    shape(f.ellipse(0, -h, w, 1.5) + f.ellipse(0, -h, w - 0.6, 1.1), "wood-700", 1, INKED),
    shape(f.ellipse(0, -h, w - 0.6, 1.1), "blue-600", 1),
    shape(swirl(f, 0, -h, w - 1.2, 0.24, turn, 0.32), "sky-300", 0.85),
    shape(f.band([[-w + 0.8, -2.6], [-w - 0.4, -h / 2], [-w + 0.8, -h + 3]], 0.45), "white", 0.18),
  ];
}

/** A pell: a stout notched practice post planted in a mound on the anchor, a red cloth band round it and rope padding below; about 7 wide and 22 tall at scale 1. */
function pellPost(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.03, 0.03);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const cuts = [0, 1, 2].map(() => between(rng, -18, -5));
  return [
    shape(f.ellipse(0.6, 0.2, 6, 0.9), "black", 0.22),
    shape(f.poly([P(-2.6, 0), P(-2.6, -20), P(-1.8, -21.2), P(1.8, -21.2), P(2.6, -20), P(2.6, 0)]), "wood-700", 1, MAIN_INKED),
    shape(f.band([P(-1, -1), P(-1, -20)], 0.25) + f.band([P(1.1, -1), P(1.1, -19.6)], 0.25), "wood-800", 0.4),
    shape(cuts.map((y, k) => f.band([P(k % 2 === 0 ? -2.6 : 2.6, y), P(k % 2 === 0 ? -0.4 : 0.5, y + 1.2)], 0.4)).join(""), "wood-900", 0.7),
    shape(f.poly([P(-2.75, -15), P(2.75, -15), P(2.75, -13.2), P(-2.75, -13.2)]), "red-600", 1, INKED),
    shape([-7.6, -6.6, -5.6].map((y) => f.band([P(-2.7, y), P(2.7, y - 0.5)], 0.45)).join(""), "brass-600", 1),
    shape(`M${f.p(-4.6, 0.3)}Q${f.p(0, -2.2)} ${f.p(4.6, 0.3)}Z`, "stone-500", 1, INKED),
  ];
}

/** A plain scarecrow on a crossed post, its foot at the anchor: a sack head in a floppy hat, a patched shirt and straw at its cuffs and hem; about 24 wide and 33 tall at scale 1. */
function scarecrow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.04, 0.04);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const p = (dx: number, dy: number): string => f.p(...P(dx, dy));
  const j = (): number => between(rng, -0.6, 0.6);
  const tuft = (x: number, y: number, side: number): string =>
    [-0.8, 0, 0.8].map((k) => f.band([P(x, y + k * 0.6), P(x + side * 1.8, y + k * 1.4 + j() * 0.5)], 0.3)).join("");
  return [
    shape(f.ellipse(0, 0.3, 4, 0.8), "black", 0.22),
    shape(f.band([P(0, 0), P(0, -30)], 1.4), "wood-700", 1, INKED),
    shape(f.band([P(-11, -21), P(11, -21.4)], 1.1), "wood-700", 1, INKED),
    shape(tuft(-10.4, -21, -1) + tuft(10.4, -21.4, 1), "brass-500", 1),
    shape(f.band([P(-10, -21.1), P(-3, -21.4)], 3) + f.band([P(3, -21.4), P(10, -21.3)], 3), "red-600", 1, MAIN_INKED),
    shape(`M${p(-4.4, -23)}L${p(4.4, -23)}L${p(4.8, -13.2 + j())}L${p(2, -14.4)}L${p(0, -12.8 + j())}L${p(-2.2, -14.4)}L${p(-4.8, -13.4 + j())}Z`, "red-600", 1, MAIN_INKED),
    shape(f.poly([P(1, -19.8), P(3.4, -19.8), P(3.4, -17.2), P(1, -17.2)]), "blue-800", 1, INKED),
    shape(tuft(-2.6, -13.6, -0.3) + tuft(2.6, -13.6, 0.3), "brass-500", 1),
    shape(f.circle(...P(0, -26.2), 3.2), "stone-500", 1, INKED),
    shape(f.band([P(-1.6, -27), P(-0.6, -26.2)], 0.3) + f.band([P(0.6, -27), P(1.6, -26.2)], 0.3) + f.band([P(-1.4, -24.6), P(1.4, -24.8)], 0.3), "wood-900", 0.85),
    shape(f.band([P(-4.6, -28.6), P(4.6, -28.9)], 0.8) + `M${p(-2.4, -29.1)}L${p(-1.4, -32.4)}L${p(1.8 + j(), -32.8)}L${p(2.6, -29.2)}Z`, "wood-800", 1, INKED),
  ];
}

/** A fluffy white bunny sitting on the anchor, facing its way: long ears up, a puff of a tail and a swirling hypnotic eye; about 8 wide and 11 tall at scale 1. */
function bunny(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flop = between(rng, -0.6, 0.6);
  const turn = between(rng, 0, 360);
  const ear = (x: number, tipX: number, tipY: number): string => `M${f.p(x - 0.55, -6.8)}Q${f.p(tipX - 1, (tipY - 6.8) / 2)} ${f.p(tipX, tipY)}Q${f.p(tipX + 1, (tipY - 6.8) / 2)} ${f.p(x + 0.55, -6.8)}Z`;
  return [
    shape(f.ellipse(0, 0.2, 4.4, 0.7), "black", 0.22),
    shape(f.circle(-3.4, -2.4, 1.2), "slate-100", 1, INKED),
    shape(f.ellipse(-0.4, -2.6, 3.4, 2.6), "white", 1, MAIN_INKED),
    shape(ear(1.2, 0.2 + flop, -11.2) + ear(2.8, 3.8, -11 - flop * 0.5), "white", 1, MAIN_INKED),
    shape(f.ellipse(0.4 + flop * 0.4, -8.8, 0.3, 1.4) + f.ellipse(3.3, -8.8, 0.3, 1.4), "red-300", 0.8),
    shape(f.ellipse(2.2, -5.6, 2.2, 1.9), "white", 1, MAIN_INKED),
    shape(f.circle(-1.6, -4, 0.5) + f.circle(0.2, -4.8, 0.4) + f.circle(-2.2, -1.6, 0.4), "slate-100", 0.7),
    shape(f.circle(2.9, -6, 0.75), "white", 1, INKED),
    shape(swirl(f, 2.9, -6, 0.7, 1, turn, 0.22), "violet-500", 1),
    shape(f.poly([[4.2, -5.4], [4.5, -5], [4, -4.9]]), "red-300", 1),
    shape(f.ellipse(2.2, -0.4, 1.3, 0.5), "slate-100", 1, INKED),
  ];
}

/** R3393000, R3393001: the yard's props and the Legacy cards' touches, by id, each with its kind. */
export const TOUCHES_KIT = {
  "target-board": { category: "objects", draw: target },
  "rope-cordon": { category: "objects", draw: ropeCordon },
  "practice-sword": { category: "objects", draw: practiceSword },
  "paw-prints": { category: "objects", draw: pawPrints },
  "range-flags": { category: "objects", draw: rangeFlags },
  "straw-heap": { category: "objects", draw: strawHeap },
  hammock: { category: "objects", draw: hammock },
  "scratching-post": { category: "objects", draw: scratchingPost },
  bucket: { category: "objects", draw: bucket },
  "lifting-stone": { category: "objects", draw: liftingStone },
  barrel: { category: "objects", draw: barrel },
  "pell-post": { category: "objects", draw: pellPost },
  scarecrow: { category: "objects", draw: scarecrow },
  "hypno-bunny": { category: "creatures", draw: bunny },
} as const satisfies Readonly<Record<string, KitPart>>;
