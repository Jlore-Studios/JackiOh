// R3331000: the figure kit (ART.md, "Scene recipes and kits"), which replaces the procedural painter's
// one silhouette in a scene: a pose (legs, arms and hands), an outfit, a head, hair or headgear and a
// held item, each a part of its own. A figure stands 56 units tall at scale 1, its feet at its anchor;
// its pose places its neck, its hip and its hand (`POSE_ANCHORS`), where the head and hair, the
// outfit and the item are drawn. A look (`FigureLook`, `FIGURE_LOOKS`) picks the parts and paints a
// tag's people: a Terminus, a CN or KY character, a Felinor, a Pancake, a Fruit, Jlockeed's people, a
// Human. Its skin repaints each part's `skin` shapes and its cloth each `main` one. Every colour is a
// palette.css name, none in Radiant's gold window. R3391000: Stinkyscape's pass adds the parts its briefs
// name: a wide, a seated and a reaching pose; a horn-mawed and a one-eyed lantern head; a helmet, goggles,
// a headset, shades and silver hair; a lab coat, academic robes, an apron, a hydraulic arm and a body of
// wreckage; and a hammer, a scanner, a stylus, a star chart, a fan of drone cores, a drill and a venom blade.
// A recipe puts them on through its look (`{ ...figureLookFor(tags), outfit, item }`); no tag's look changes.

import type { Tag } from "@jackioh/shared";

import type { FigureRecipe, KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";
import { local, type Local } from "./sceneKit.ts";

type Point = readonly [number, number];

export type PoseId = "pose-stand" | "pose-guard" | "pose-raise" | "pose-wide" | "pose-sit" | "pose-reach";
export type HeadId = "head-human" | "head-felinor" | "head-pancake" | "head-fruit" | "head-void" | "head-horn" | "head-lantern";
export type HairId =
  | "hair-short"
  | "hair-long"
  | "hood"
  | "hat-pointed"
  | "antennae"
  | "visor"
  | "toque"
  | "leaf-crown"
  | "helmet"
  | "goggles"
  | "headset"
  | "shades"
  | "hair-silver";
export type OutfitId =
  | "outfit-tunic"
  | "outfit-armor"
  | "outfit-coat"
  | "outfit-jacket"
  | "outfit-suit"
  | "outfit-fur"
  | "outfit-stack"
  | "outfit-peel"
  | "outfit-void"
  | "outfit-labcoat"
  | "outfit-robes"
  | "outfit-apron"
  | "outfit-hydraulic"
  | "outfit-wreck";
export type ItemId =
  | "item-sword"
  | "item-shield"
  | "item-staff"
  | "item-book"
  | "item-blaster"
  | "item-hammer"
  | "item-scanner"
  | "item-stylus"
  | "item-chart"
  | "item-core"
  | "item-drill"
  | "item-venom-blade";
export type FigurePartId = PoseId | HeadId | HairId | OutfitId | ItemId;

/** The parts a figure wears, and the two paints its look lays on them. */
export type FigureLook = {
  readonly head: HeadId;
  readonly hair: HairId | null;
  readonly outfit: OutfitId;
  readonly item: ItemId | null;
  readonly skin: PaletteName;
  readonly cloth: PaletteName;
};

/** A figure's height at scale 1, feet to crown. */
export const FIGURE_HEIGHT = 56;
/** Where a head's centre stands above its neck. */
const HEAD_RISE = 5.4;
const HEAD_R = 4.8;
/** An outfit is drawn about the hip, its shoulders this far above it (the neck is 20 above in every pose). */
const SHOULDER = -17.5;
/** The ground below the hip, where a long hem ends. */
const HEM = 22.5;
const DEGREES = Math.PI / 180;

/** Each pose's neck, hand and hip, as offsets from the feet at scale 1. */
export const POSE_ANCHORS: Readonly<Record<PoseId, { readonly neck: Point; readonly hand: Point; readonly hip: Point }>> = {
  "pose-stand": { neck: [0, -44], hand: [9, -23.4], hip: [0, -24] },
  "pose-guard": { neck: [0, -43], hand: [8.5, -31], hip: [0, -23] },
  "pose-raise": { neck: [0, -44], hand: [11, -56.2], hip: [0, -24] },
  "pose-wide": { neck: [0, -44], hand: [14, -48], hip: [0, -24] },
  "pose-sit": { neck: [0, -34], hand: [9, -20], hip: [0, -14] },
  "pose-reach": { neck: [0, -44], hand: [15, -36], hip: [0, -24] },
};

type Flags = { readonly inked?: boolean; readonly main?: boolean; readonly skin?: boolean };
const MAIN: Flags = { main: true };
const INKED: Flags = { inked: true };
const MAIN_INKED: Flags = { main: true, inked: true };
const SKIN: Flags = { skin: true };
const SKIN_INKED: Flags = { skin: true, inked: true };

function shape(d: string, paint: PaletteName, opacity: number, flags?: Flags): KitShape {
  return flags === undefined ? { d, paint, opacity } : { d, paint, opacity, ...flags };
}

function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

/** A pose's limbs: both legs and boots, the far arm, the near arm and both hands. */
function limbs(f: Local, legs: readonly [readonly Point[], readonly Point[]], arms: readonly [readonly Point[], readonly Point[]]): KitShape[] {
  const [left, right] = legs;
  const [far, near] = arms;
  const foot = (leg: readonly Point[]): Point => leg[leg.length - 1] ?? [0, 0];
  const hand = (arm: readonly Point[]): Point => arm[arm.length - 1] ?? [0, 0];
  const [lx, ly] = foot(left);
  const [rx, ry] = foot(right);
  const [fx, fy] = hand(far);
  const [nx, ny] = hand(near);
  return [
    shape(f.band(left, 3.8) + f.band(right, 3.8), "slate-800", 1, INKED),
    shape(f.ellipse(lx - 0.6, ly + 0.4, 2.6, 1.2) + f.ellipse(rx + 0.6, ry + 0.4, 2.6, 1.2), "wood-800", 1, INKED),
    shape(f.band(far, 2.8), "slate-700", 1, MAIN_INKED),
    shape(f.band(near, 2.8), "slate-700", 1, MAIN_INKED),
    shape(f.circle(fx, fy, 1.5) + f.circle(nx, ny, 1.5), "red-300", 1, SKIN),
  ];
}

// Poses, drawn at the feet

function poseStand(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.4, -24], [-2.8, -12], [-3, -1.5]],
      [[2.4, -24], [2.8, -12], [3, -1.5]],
    ],
    [
      [[-6.6, -41], [-8.6 - swing, -32], [-9 - swing, -24.6]],
      [[6.6, -41], [8.6 + swing, -32], [9, -24.6]],
    ],
  );
}

function poseGuard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stance = between(rng, 0, 1);
  return limbs(
    f,
    [
      [[-2.6, -23], [-5 - stance, -12], [-6.5 - stance, -1.5]],
      [[2.6, -23], [4.2 + stance * 0.5, -12], [5.5 + stance, -1.5]],
    ],
    [
      [[-6.6, -40.5], [-9.6, -33], [-7.5, -28.5]],
      [[6.6, -40.5], [10, -35], [8.5, -31.6]],
    ],
  );
}

function poseRaise(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.8, 0.8);
  return limbs(
    f,
    [
      [[-2.4, -24], [-2.6, -12], [-3.2, -1.5]],
      [[2.4, -24], [2.9, -12], [3.6, -1.5]],
    ],
    [
      [[-6.6, -41], [-10, -48], [-11 - reach, -55]],
      [[6.6, -41], [10, -48], [11, -55]],
    ],
  );
}

/** R3391000: arms flung wide and up, legs braced apart (Arg Arg Defender, A'Zoth). */
function poseWide(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const brace = between(rng, 0, 1);
  const lift = between(rng, -1, 1);
  return limbs(
    f,
    [
      [[-2.6, -24], [-5.6 - brace, -12], [-7.6 - brace, -1.5]],
      [[2.6, -24], [5.6 + brace, -12], [7.6 + brace, -1.5]],
    ],
    [
      [[-6.6, -41], [-10.6, -43.6 + lift], [-14, -48 + lift]],
      [[6.6, -41], [10.6, -43.6], [14, -48]],
    ],
  );
}

/** R3391000: seated, facing the way the figure faces, thighs forward and shins down (a pilot in her seat). */
function poseSit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const knee = between(rng, -0.4, 0.4);
  return limbs(
    f,
    [
      [[-1.6, -15], [5.4, -14.6 + knee], [10.4, -13.6 + knee], [10.8, -1.5]],
      [[0.6, -14], [7, -13.6], [12.2, -12.8], [12.8, -1.5]],
    ],
    [
      [[-6.6, -31], [-4.4, -24], [5.6, -21.6 + knee]],
      [[6.6, -31], [8.4, -25.4], [9, -20.4]],
    ],
  );
}

/** R3391000: the near arm reaching forward, the far one down, in a stride. */
function poseReach(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stride = between(rng, 0, 1);
  const swing = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.4, -24], [-4.6 - stride * 0.5, -12], [-6.6 - stride, -1.5]],
      [[2.4, -24], [4.4 + stride * 0.5, -12], [6.2 + stride, -1.5]],
    ],
    [
      [[-6.6, -41], [-8.4 - swing, -32], [-8.6 - swing, -24.6]],
      [[6.6, -41], [11, -38.6], [15, -36.4]],
    ],
  );
}

// Outfits, drawn at the hip

function tunic(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, 7.8, 9);
  return [
    shape(f.poly([[-7.2, SHOULDER], [7.2, SHOULDER], [6.2, 1], [flare, 8], [-flare, 8], [-6.2, 1]]), "blue-700", 1, MAIN_INKED),
    shape(f.poly([[-2.4, SHOULDER], [2.4, SHOULDER], [0, SHOULDER + 3.4]]), "slate-900", 0.35),
    shape(f.poly([[-6.5, -1.2], [6.5, -1.2], [6.4, 0.8], [-6.4, 0.8]]), "wood-700", 1, INKED),
    shape(f.poly([[-1, -1.3], [1, -1.3], [1, 0.9], [-1, 0.9]]), "brass-600", 1),
  ];
}

function armor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-6.2, 0], [6.2, 0], [7.2, 6], [-7.2, 6]]), "slate-700", 1, INKED),
    shape(f.poly([[-7, SHOULDER + 0.5], [7, SHOULDER + 0.5], [6, 0], [-6, 0]]), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(-7.2, SHOULDER + 0.6 + lift, 3.2, 2.2), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(7.2, SHOULDER + 0.6 - lift, 3.2, 2.2), "slate-400", 1, MAIN_INKED),
    shape(f.band([[0, SHOULDER + 1.5], [0, -1.5]], 0.6), "slate-100", 0.6),
    shape(f.poly([[-6.3, -1.2], [6.3, -1.2], [6.2, 0.6], [-6.2, 0.6]]), "wood-700", 1, INKED),
  ];
}

function coat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, 8.2, 9.4);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, 0], [flare, HEM - 6], [-flare, HEM - 6], [-6.4, 0]]), "blue-800", 1, MAIN_INKED),
    shape(f.poly([[-2.2, SHOULDER], [2.2, SHOULDER], [0.6, -4], [-0.6, -4]]), "slate-100", 0.9),
    shape(f.poly([[-2.2, SHOULDER], [-4.6, SHOULDER + 1], [-1.2, -5]]) + f.poly([[2.2, SHOULDER], [4.6, SHOULDER + 1], [1.2, -5]]), "slate-900", 0.3),
    shape(f.circle(1.6, -3, 0.45) + f.circle(1.7, 0, 0.45) + f.circle(1.8, 3, 0.45), "brass-600", 1),
  ];
}

function jacket(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hem = between(rng, 1.6, 2.6);
  return [
    shape(f.poly([[-7.2, SHOULDER], [7.2, SHOULDER], [6.8, hem], [-6.8, hem]]), "green-700", 1, MAIN_INKED),
    shape(f.poly([[-2.6, SHOULDER], [-4.4, SHOULDER - 1.6], [-0.4, SHOULDER + 2.8]]) + f.poly([[2.6, SHOULDER], [4.4, SHOULDER - 1.6], [0.4, SHOULDER + 2.8]]), "slate-900", 0.45),
    shape(f.band([[0, SHOULDER + 3], [0, hem - 0.4]], 0.4), "slate-100", 0.7),
    shape(f.circle(-3.6, -11.5, 1.1), "lime-400", 1, INKED),
  ];
}

function suit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lights = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, 1.2], [-6.4, 1.2]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-3.4, SHOULDER + 3], [3.4, SHOULDER + 3], [3, -8], [-3, -8]]), "slate-900", 0.6),
    shape([-1.8, 0, 1.8].map((x) => f.poly([[x - 0.5, -13 + lights], [x + 0.5, -13 + lights], [x + 0.5, -12 + lights], [x - 0.5, -12 + lights]])).join(""), "cyan-400", 0.9),
    shape(f.poly([[-6.5, -1.4], [6.5, -1.4], [6.4, 0.8], [-6.4, 0.8]]) + f.poly([[-4.6, 0.8], [-2.4, 0.8], [-2.4, 2.6], [-4.6, 2.6]]), "slate-900", 1, INKED),
    shape(f.circle(-6.2, SHOULDER + 2.4, 1.2), "red-600", 1),
  ];
}

function fur(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -1, 1);
  const hem: Point[] = [[6.6, 2], [5, 3.6], [3.4, 2.2], [1.7, 3.8], [0, 2.2], [-1.7, 3.8], [-3.4, 2.2], [-5, 3.6], [-6.6, 2]];
  return [
    shape(f.band([[4, -1], [9, -2], [12, -6 + curl], [11, -10 + curl]], 1.6), "orange-400", 1, SKIN_INKED),
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], ...hem]), "orange-400", 1, SKIN_INKED),
    shape(f.poly([[-2.8, SHOULDER + 1], [2.8, SHOULDER + 1], [1.8, -9], [0, -7.4], [-1.8, -9]]), "white", 0.85),
    shape(f.band([[-6.4, SHOULDER + 1], [6, -1.6]], 1.6), "violet-700", 1, MAIN),
  ];
}

function stack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cake = (y: number, w: number): string => `M${f.p(-w, y)}Q${f.p(-w - 1.2, y + 1.6)} ${f.p(-w, y + 3.2)}L${f.p(w, y + 3.2)}Q${f.p(w + 1.2, y + 1.6)} ${f.p(w, y)}Z`;
  const drip = between(rng, -2, 2);
  return [
    shape(cake(1.2, 7.8), "brass-600", 1, SKIN_INKED),
    shape(cake(-3.2, 8.2), "brass-600", 1, SKIN_INKED),
    shape(cake(-7.6, 7.9), "brass-600", 1, SKIN_INKED),
    shape(cake(-12, 8.3), "brass-600", 1, SKIN_INKED),
    shape(cake(-16.4, 7.6), "brass-600", 1, SKIN_INKED),
    shape(`M${f.p(-6.6, SHOULDER + 1)}L${f.p(6.6, SHOULDER + 1)}L${f.p(6.2, SHOULDER + 2.4)}L${f.p(drip + 1, SHOULDER + 2.4)}L${f.p(drip + 0.6, SHOULDER + 6)}L${f.p(drip - 0.6, SHOULDER + 6)}L${f.p(drip - 1, SHOULDER + 2.4)}L${f.p(-6.2, SHOULDER + 2.4)}Z`, "wood-700", 0.9),
  ];
}

function peel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const twist = between(rng, -1, 1);
  return [
    shape(f.circle(0, -8, 9.2), "red-600", 1, SKIN_INKED),
    shape(f.band([[-8.6, -3], [-6, 2 + twist], [-2, 3.4], [2.6, 2.2 + twist]], 1.8), "orange-400", 1, MAIN_INKED),
    shape(f.ellipse(-3.6, -12.4, 2.4, 1.4), "white", 0.4),
    shape(f.circle(3.4, -6, 1.4) + f.circle(-3.2, -5.6, 1.2), "red-400", 0.45),
  ];
}

function robe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.8, 0.8);
  const hem: Point[] = [[9.6, HEM], [7.4, HEM - 2.6 + j()], [5.2, HEM], [2.6, HEM - 2.4 + j()], [0, HEM], [-2.6, HEM - 2.2 + j()], [-5.2, HEM], [-7.4, HEM - 2.8 + j()], [-9.6, HEM]];
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [8.4, 4], ...hem, [-8.4, 4]]), "slate-900", 1, MAIN_INKED),
    shape(f.poly([[-2.6, SHOULDER + 1], [2.6, SHOULDER + 1], [3.6, HEM - 3], [-3.6, HEM - 3]]), "black", 0.9),
    shape(f.circle(-1.2, -8, 0.35) + f.circle(1.4, -3.4, 0.3) + f.circle(-0.6, 2.6, 0.4) + f.circle(1, 9, 0.3) + f.circle(-1.6, 14, 0.35), "violet-300", 0.85),
    shape(f.band([[-7, SHOULDER + 0.4], [-8.2, 4], [-9.4, HEM - 0.6]], 0.6) + f.band([[7, SHOULDER + 0.4], [8.2, 4], [9.4, HEM - 0.6]], 0.6), "purple-400", 0.6),
  ];
}

/** R3391000: a long lab coat open to the knee over a dark shirt, its pockets, chalk dust on it. */
function labcoat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, 8, 9.2);
  const dust = between(rng, -1.2, 1.2);
  return [
    shape(f.poly([[-7.2, SHOULDER], [7.2, SHOULDER], [6.6, 0], [flare, 12], [-flare, 12], [-6.6, 0]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-2.2, SHOULDER], [2.2, SHOULDER], [2.8, 12], [-2.8, 12]]), "slate-800", 1),
    shape(f.poly([[-2.2, SHOULDER], [-4.6, SHOULDER + 1], [-1.4, -6]]) + f.poly([[2.2, SHOULDER], [4.6, SHOULDER + 1], [1.4, -6]]), "slate-400", 0.7),
    shape(f.poly([[-6.4, 2], [-3.6, 2], [-3.6, 2.6], [-6.4, 2.6]]) + f.poly([[3.6, 2], [6.4, 2], [6.4, 2.6], [3.6, 2.6]]) + f.poly([[3.4, -11], [5.4, -11], [5.4, -10.5], [3.4, -10.5]]), "slate-400", 0.9),
    shape(f.circle(-4.8 + dust, -8, 0.35) + f.circle(5, -4 - dust, 0.3) + f.circle(-5.6, 6 + dust, 0.35) + f.circle(4.6, 8, 0.3) + f.circle(-3.6, -2, 0.25), "white", 0.9),
  ];
}

/** R3391000: deep academic robes to the ground, a stole and a row of medals across the chest. */
function robes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  const medal = (x: number): string => f.circle(x, -8.4, 0.85);
  return [
    shape(f.poly([[-7.4, SHOULDER], [7.4, SHOULDER], [8.6, 4], [10 + sway, HEM], [-10 + sway, HEM], [-8.6, 4]]), "blue-900", 1, MAIN_INKED),
    shape(f.poly([[-2.4, SHOULDER + 0.4], [2.4, SHOULDER + 0.4], [0, SHOULDER + 4.4]]), "slate-100", 0.85),
    shape(f.band([[-6.8, SHOULDER + 0.6], [-1.4, SHOULDER + 5.2]], 1.2) + f.band([[6.8, SHOULDER + 0.6], [1.4, SHOULDER + 5.2]], 1.2), "red-900", 0.9),
    shape(medal(-4.4) + medal(0), "red-600", 1, INKED),
    shape(medal(-2.2) + medal(4.4), "slate-100", 1, INKED),
    shape(medal(2.2), "cyan-400", 1, INKED),
  ];
}

/** R3391000: a smith's or cook's apron over a shirt, its neck strap and ties and its pocket; the apron is the cloth. */
function apron(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, 6, 6.8);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, 1.2], [-6.4, 1.2]]), "slate-700", 1, INKED),
    shape(f.poly([[-4.6, -11], [4.6, -11], [5.6, 0], [flare, 14], [-flare, 14], [-5.6, 0]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-3.8, -11], [-2.2, SHOULDER], [2.2, SHOULDER], [3.8, -11]], 0.6) + f.band([[-5.6, -0.6], [-7.6, 1.2]], 0.5) + f.band([[5.6, -0.6], [7.6, 1.2]], 0.5), "wood-800", 1),
    shape(f.poly([[-2.6, 1.6], [2.6, 1.6], [2.6, 5], [-2.6, 5]]), "wood-800", 0.45),
  ];
}

/** R3391000: plate armour with one massive hydraulic arm on the near side, its pistons and its fist reaching out. */
function hydraulic(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.8, 0.8);
  return [
    shape(f.poly([[-7, SHOULDER + 0.5], [7, SHOULDER + 0.5], [6, 0], [-6, 0]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6.2, 0], [6.2, 0], [7.2, 6], [-7.2, 6]]), "slate-700", 1, INKED),
    shape(f.ellipse(-7.2, SHOULDER + 0.6, 3.2, 2.2), "slate-400", 1, MAIN_INKED),
    shape(f.band([[8, -16], [12.2, -9.6]], 4.4), "slate-700", 1, INKED),
    shape(f.poly([[10, -7.4], [10.6, -12.4], [18.4, -15.8 + reach], [19, -10.4 + reach]]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[9.8, -17.6], [14.6, -11.4]], 0.7) + f.band([[11.4, -7.6], [17.6, -10.4 + reach]], 0.6), "slate-100", 0.9),
    shape(f.circle(20.6, -13 + reach, 3.2), "slate-700", 1, INKED),
    shape(f.circle(8, -16.6, 3.8), "slate-400", 1, MAIN_INKED),
  ];
}

/** R3391000: a body of drifting wreckage: a ribcage of broken hull beams on a spine over a dark core, plates hanging; the beams are the cloth. */
function wreckBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -1, 1);
  const hem: Point[] = [[8.6, HEM - 3], [6, HEM - 6 + j()], [3.6, HEM - 2.4], [1, HEM - 6.4 + j()], [-1.6, HEM - 2.6], [-4.2, HEM - 6.6 + j()], [-6.6, HEM - 2.8], [-8.6, HEM - 5 + j()]];
  const ribs: string[] = [];
  for (let k = 0; k < 4; k += 1) {
    const y = SHOULDER + 2.4 + k * 3;
    const broken = k === 1 || k === 3 ? 0.55 : 1;
    ribs.push(f.band([[-0.9, y], [-4.6, y + 0.6], [-6 + k * 0.3, y + 2.4]], 0.85));
    ribs.push(f.band([[0.9, y], [0.9 + 3.7 * broken, y + 0.6 * broken], [0.9 + (5.1 - k * 0.3) * broken, y + 2.4 * broken]], 0.85));
  }
  return [
    shape(f.poly([[-6.6, SHOULDER], [6.6, SHOULDER], [7.8, 3], ...hem, [-7.8, 3]]), "slate-900", 0.95, INKED),
    shape(f.ellipse(0, -9, 3.6, 5.4), "blue-200", 0.18),
    shape(f.band([[0, SHOULDER + 0.6], [0, 4]], 1.2), "slate-400", 1, MAIN),
    shape(ribs.join(""), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6, 1], [-2.6, 1.4], [-3.2, 7 + j()], [-6.4, 6]]) + f.poly([[2.8, 1.2], [6.4, 0.6], [6.8, 6.6], [3.4, 8 + j()]]), "slate-700", 1, INKED),
  ];
}

// Heads, drawn at the neck

function headHuman(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glance = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "red-300", 1, SKIN),
    shape(f.circle(0, -HEAD_RISE, HEAD_R), "red-300", 1, SKIN_INKED),
    shape(f.circle(-1.6 + glance, -HEAD_RISE + 0.2, 0.45) + f.circle(1.6 + glance, -HEAD_RISE + 0.2, 0.45), "slate-900", 1),
    shape(f.band([[-1, -HEAD_RISE + 2.4], [1, -HEAD_RISE + 2.4]], 0.35), "slate-900", 0.5),
  ];
}

function headFelinor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.6, 0.6);
  const c = -HEAD_RISE;
  return [
    shape(f.poly([[-4.6, c - 1.6], [-4.2 + tilt, c - 7.4], [-1.4, c - 4.2]]) + f.poly([[1.4, c - 4.2], [4.2 + tilt, c - 7.4], [4.6, c - 1.6]]), "orange-400", 1, SKIN_INKED),
    shape(f.circle(0, c, HEAD_R), "orange-400", 1, SKIN_INKED),
    shape(f.poly([[-3.7, c - 2.4], [-3.6 + tilt, c - 5.6], [-2.2, c - 3.6]]) + f.poly([[2.2, c - 3.6], [3.6 + tilt, c - 5.6], [3.7, c - 2.4]]), "red-300", 0.8),
    shape(f.ellipse(-1.8, c - 0.2, 0.9, 0.6) + f.ellipse(1.8, c - 0.2, 0.9, 0.6), "lime-400", 1),
    shape(f.ellipse(-1.8, c - 0.2, 0.22, 0.55) + f.ellipse(1.8, c - 0.2, 0.22, 0.55), "black", 1),
    shape(f.ellipse(0, c + 2.2, 2, 1.3), "white", 0.8),
    shape(f.poly([[-0.6, c + 1.2], [0.6, c + 1.2], [0, c + 1.9]]), "red-300", 1),
  ];
}

function headPancake(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const smile = between(rng, 0.3, 0.7);
  const disc = (y: number): string => `M${f.p(-5.6, y)}Q${f.p(-6.6, y + 1.7)} ${f.p(-5.6, y + 3.4)}L${f.p(5.6, y + 3.4)}Q${f.p(6.6, y + 1.7)} ${f.p(5.6, y)}Z`;
  return [
    shape(disc(c - 0.6), "brass-600", 1, SKIN_INKED),
    shape(disc(c - 4), "brass-600", 1, SKIN_INKED),
    shape(f.circle(-1.8, c + 1, 0.45) + f.circle(1.8, c + 1, 0.45), "wood-950", 1),
    shape(`M${f.p(-1.2, c + 1.9)}Q${f.p(0, c + 1.9 + smile * 2)} ${f.p(1.2, c + 1.9)}Q${f.p(0, c + 1.9 + smile)} ${f.p(-1.2, c + 1.9)}Z`, "wood-950", 1),
  ];
}

function headFruit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const lean = between(rng, -0.6, 0.6);
  return [
    shape(f.circle(0, c, HEAD_R + 0.4), "red-600", 1, SKIN_INKED),
    shape(f.band([[0, c - HEAD_R], [lean, c - HEAD_R - 2]], 0.5), "wood-700", 1),
    shape(f.circle(-1.7, c + 0.2, 0.5) + f.circle(1.7, c + 0.2, 0.5), "wood-950", 1),
    shape(f.ellipse(-1.8, c - 2.4, 1.4, 0.8), "white", 0.45),
  ];
}

function headVoid(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const gaze = between(rng, -0.3, 0.3);
  return [
    shape(f.ellipse(0, c + 0.4, HEAD_R * 0.86, HEAD_R), "black", 0.95, SKIN),
    shape(f.circle(-1.5 + gaze, c, 1.4) + f.circle(1.5 + gaze, c, 1.4), "purple-400", 0.35),
    shape(f.ellipse(-1.5 + gaze, c, 0.7, 0.45) + f.ellipse(1.5 + gaze, c, 0.7, 0.45), "violet-300", 1),
  ];
}

/** R3391000: A'Zoth's head: a dome of polished wreckage, its seams and one socket, its face a horn's flared maw. */
function headHorn(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flare = between(rng, 3.2, 4);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "slate-400", 1, SKIN),
    shape(`M${f.p(-4.6, c + 2.6)}C${f.p(-6, c - 7.6)} ${f.p(4.6, c - 8)} ${f.p(4.4, c - 1.2)}L${f.p(2.4, c + 3.4)}Z`, "slate-400", 1, SKIN_INKED),
    shape(`M${f.p(1.6, c - 1.6)}Q${f.p(5, c - 2)} ${f.p(8.4, c - flare)}L${f.p(8.4, c + flare)}Q${f.p(5, c + 2)} ${f.p(1.6, c + 2.2)}Z`, "slate-400", 1, SKIN_INKED),
    shape(f.ellipse(8, c, 0.9, flare * 0.82) + f.ellipse(0.2, c - 2.8, 0.9, 0.7), "black", 0.9),
    shape(f.band([[-3.6, c - 3], [-1.4, c - 1.2], [-0.8, c - 5.4]], 0.35) + f.band([[-3.4, c + 0.8], [-0.4, c + 1.8]], 0.3), "slate-900", 0.55),
  ];
}

/** R3391000: Z'Zoth's head: a void for a face and one pale lantern eye glowing in it. */
function headLantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const gaze = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(0, c + 0.4, HEAD_R * 0.86, HEAD_R), "black", 0.95, SKIN),
    shape(f.circle(0.8 + gaze, c - 0.4, 2.8), "slate-100", 0.18),
    shape(f.circle(0.8 + gaze, c - 0.4, 1.6), "blue-200", 0.5),
    shape(f.circle(0.8 + gaze, c - 0.4, 0.8), "white", 0.95),
  ];
}

// Hair and headgear, drawn at the neck

/** R3391000: a round space helmet, its visor tinted and cracked across, its collar ring. */
function helmet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const crack = between(rng, -0.8, 0.8);
  return [
    shape(f.circle(0, c - 0.4, 6.4) + f.ellipse(0.6, c + 0.4, 4.4, 3.2), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(0.6, c + 0.4, 4.4, 3.2), "sky-300", 0.35, INKED),
    shape(f.poly([[-2.4, c - 1.8], [-1.2, c - 1.8], [-2.2, c + 0.6], [-3.4, c + 0.6]]), "white", 0.5),
    shape(f.band([[-3.2, c + 1.6], [-1, c + 0.6 + crack], [0.6, c + 1.8], [2.6, c - 0.6 + crack], [4.6, c + 0.2]], 0.4), "white", 0.9),
    shape(f.band([[-5.2, c + 4.6], [0, c + 6], [5.2, c + 4.6]], 1.4), "slate-700", 1, INKED),
  ];
}

/** R3391000: goggles strapped over short hair, their lenses' rims singed. */
function goggles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const soot = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.6)} ${f.p(5.4, c - 6.6)} ${f.p(5, c + 0.5)}Q${f.p(2, c - 2.6)} ${f.p(0, c - 2.4)}Q${f.p(-2.4, c - 2.2)} ${f.p(-5, c + 0.5)}Z`, "wood-800", 1, INKED),
    shape(f.band([[-5, c - 0.4], [5, c - 0.4]], 1.1), "slate-900", 1),
    shape(f.circle(-1.9, c, 1.7) + f.circle(1.9, c, 1.7), "slate-700", 1, INKED),
    shape(f.circle(-1.9, c, 1.05) + f.circle(1.9, c, 1.05), "sky-300", 0.8),
    shape(f.ellipse(-3.4, c + 0.8 + soot, 0.9, 0.6) + f.ellipse(3.2, c - 1.2, 0.8, 0.5) + f.ellipse(0, c - 1.6 - soot, 0.6, 0.35), "wood-950", 0.65),
  ];
}

/** R3391000: long hair under a headset, its mic arm curving to the mouth. */
function headset(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sway = between(rng, -0.6, 0.6);
  const lock = (side: number): string => f.poly([[side * 4.2, c - 1], [side * 5.6, c - 1.4], [side * 6.4 + sway, c + 8.4], [side * 4.4 + sway, c + 7.8]]);
  return [
    shape(`M${f.p(-5.2, c + 0.8)}C${f.p(-5.6, c - 7)} ${f.p(5.6, c - 7)} ${f.p(5.2, c + 0.8)}Q${f.p(2.6, c - 2.8)} ${f.p(0, c - 2.8)}Q${f.p(-2.6, c - 2.8)} ${f.p(-5.2, c + 0.8)}Z`, "wood-700", 1, INKED),
    shape(lock(-1) + lock(1), "wood-700", 1, INKED),
    shape(f.band([[-5.4, c], [-4.4, c - 5], [0, c - 6.6], [4.4, c - 5], [5.4, c]], 0.6) + f.band([[5.4, c + 1.2], [4.8, c + 3.4], [2, c + 3.2]], 0.35), "slate-900", 1),
    shape(f.circle(5.3, c + 0.2, 1.3) + f.circle(1.5, c + 3.2, 0.6), "slate-700", 1, INKED),
  ];
}

/** R3391000: dark glasses, within any head's width. */
function shades(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const glint = between(rng, -0.3, 0.3);
  return [
    shape(f.band([[-0.5, c - 0.5], [0.5, c - 0.5]], 0.35) + f.band([[-3.3, c - 0.7], [-4.6, c - 1.1]], 0.35) + f.band([[3.3, c - 0.7], [4.6, c - 1.1]], 0.35), "slate-900", 1),
    shape(f.ellipse(-1.9, c - 0.2, 1.5, 1) + f.ellipse(1.9, c - 0.2, 1.5, 1), "black", 0.95, INKED),
    shape(f.poly([[-2.6 + glint, c - 0.8], [-1.9 + glint, c - 0.8], [-2.4 + glint, c + 0.1], [-3.1 + glint, c + 0.1]]) + f.poly([[1.2 + glint, c - 0.8], [1.9 + glint, c - 0.8], [1.4 + glint, c + 0.1], [0.7 + glint, c + 0.1]]), "white", 0.5),
  ];
}

/** R3391000: silver hair swept back from the brow, a professor's. */
function hairSilver(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sweep = between(rng, -0.8, 0.8);
  return [
    shape(f.poly([[-4.6, c - 3], [-6.8 + sweep, c - 3.4], [-6, c - 1.4], [-5.2, c + 0.6]]), "slate-100", 1, INKED),
    shape(`M${f.p(-5.2, c + 1)}C${f.p(-6.4, c - 6.2)} ${f.p(3, c - 8)} ${f.p(5, c - 1.6)}Q${f.p(2.6, c - 3.6)} ${f.p(0, c - 3.2)}Q${f.p(-3, c - 2.8)} ${f.p(-5.2, c + 1)}Z`, "slate-100", 1, INKED),
    shape(f.band([[3.4, c - 4.4], [0, c - 5.6], [-4.4, c - 4.2]], 0.35) + f.band([[3.8, c - 2.8], [0, c - 4], [-4, c - 2.4]], 0.3), "slate-400", 0.8),
  ];
}

function hairShort(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const part = between(rng, -1, 1);
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.6)} ${f.p(5.4, c - 6.6)} ${f.p(5, c + 0.5)}Q${f.p(2, c - 2.6)} ${f.p(part, c - 2.4)}Q${f.p(-2.4, c - 2.2)} ${f.p(-5, c + 0.5)}Z`, "wood-800", 1, INKED),
    shape(f.poly([[part - 0.4, c - 2.6], [part + 2.4, c - 4.4], [part + 1.2, c - 1.6]]), "wood-800", 1),
  ];
}

function hairLong(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sway = between(rng, -0.6, 0.6);
  const lock = (side: number): string => f.poly([[side * 4.2, c - 1], [side * 5.6, c - 1.4], [side * 6.4 + sway, c + 7.6], [side * 4.4 + sway, c + 7.2]]);
  return [
    shape(`M${f.p(-5.2, c + 0.8)}C${f.p(-5.6, c - 7)} ${f.p(5.6, c - 7)} ${f.p(5.2, c + 0.8)}Q${f.p(2.6, c - 2.8)} ${f.p(0, c - 2.8)}Q${f.p(-2.6, c - 2.8)} ${f.p(-5.2, c + 0.8)}Z`, "brass-800", 1, INKED),
    shape(lock(-1) + lock(1), "brass-800", 1, INKED),
  ];
}

function hood(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const peak = between(rng, -0.8, 0.8);
  const outer = `M${f.p(-6.4, c + 5.6)}L${f.p(-6.4, c - 1)}C${f.p(-6.4, c - 7.6)} ${f.p(peak, c - 8.6)} ${f.p(peak, c - 8.6)}C${f.p(peak, c - 8.6)} ${f.p(6.4, c - 7.6)} ${f.p(6.4, c - 1)}L${f.p(6.4, c + 5.6)}Z`;
  return [
    shape(outer + f.ellipse(0, c + 0.6, 3.9, 4.6), "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(-3.8, c)}Q${f.p(0, c - 5.6)} ${f.p(3.8, c)}Q${f.p(0, c - 3.4)} ${f.p(-3.8, c)}Z`, "slate-900", 0.4),
  ];
}

function hatPointed(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tip = between(rng, -3, 3);
  return [
    shape(f.poly([[-4, c - 3.8], [tip, c - 15], [4, c - 3.8]]), "blue-900", 1, MAIN_INKED),
    shape(f.ellipse(0, c - 3.6, 7.6, 1.4), "blue-900", 1, MAIN_INKED),
    shape(f.poly([[-3.7, c - 4.6], [3.7, c - 4.6], [3.4, c - 5.8], [-3.4, c - 5.8]]), "jade-400", 1),
  ];
}

function antennae(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const spread = between(rng, 0, 1.2);
  return [
    shape(f.band([[-1.6, c - 4.4], [-2.8 - spread, c - 8], [-4.6 - spread, c - 10.6]], 0.5) + f.band([[1.6, c - 4.4], [2.8 + spread, c - 8], [4.6 + spread, c - 10.6]], 0.5), "slate-900", 1),
    shape(f.circle(-4.6 - spread, c - 11.4, 1) + f.circle(4.6 + spread, c - 11.4, 1), "lime-400", 1, INKED),
    shape(f.poly([[-4.6, c - 2.6], [4.6, c - 2.6], [4.4, c - 3.6], [-4.4, c - 3.6]]), "slate-900", 0.8),
  ];
}

function visor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const glint = between(rng, -1, 1);
  return [
    shape(`M${f.p(-5.4, c + 1)}C${f.p(-5.8, c - 7.4)} ${f.p(5.8, c - 7.4)} ${f.p(5.4, c + 1)}Z`, "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-4.6, c - 1.6], [4.6, c - 1.6], [4.2, c + 1], [-4.2, c + 1]]), "cyan-400", 0.9, INKED),
    shape(f.poly([[glint - 2, c - 1.2], [glint - 0.8, c - 1.2], [glint - 1.6, c + 0.6], [glint - 2.8, c + 0.6]]), "white", 0.6),
  ];
}

function toque(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const puff = between(rng, 2.4, 3);
  return [
    shape(f.circle(-2.6, c - 8.2, puff), "white", 1, INKED),
    shape(f.circle(2.6, c - 8.2, puff), "white", 1, INKED),
    shape(f.circle(0, c - 9.4, puff + 0.4), "white", 1, INKED),
    shape(f.poly([[-4.4, c - 3.4], [4.4, c - 3.4], [4.4, c - 6.6], [-4.4, c - 6.6]]), "slate-100", 1, INKED),
  ];
}

function leafCrown(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const leaf = (dir: number, reach: number): string =>
    `M${f.p(0, c - HEAD_R - 1.6)}Q${f.p(dir * reach * 0.5, c - HEAD_R - 3.8)} ${f.p(dir * reach, c - HEAD_R - 2.2)}Q${f.p(dir * reach * 0.5, c - HEAD_R - 1)} ${f.p(0, c - HEAD_R - 1.6)}Z`;
  return [
    shape(f.band([[0, c - HEAD_R + 0.2], [0.3, c - HEAD_R - 1.8]], 0.55), "wood-700", 1),
    shape(leaf(-1, between(rng, 3.6, 4.6)), "green-500", 1, INKED),
    shape(leaf(1, between(rng, 3.2, 4.2)), "green-400", 1, INKED),
    shape(`M${f.p(0, c - HEAD_R - 1.6)}Q${f.p(1, c - HEAD_R - 4.6)} ${f.p(0.4, c - HEAD_R - 5.6)}Q${f.p(-0.8, c - HEAD_R - 4)} ${f.p(0, c - HEAD_R - 1.6)}Z`, "green-700", 1),
  ];
}

// Items, drawn at the hand

function sword(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 8, 22) * DEGREES;
  const T = ([dx, dy]: Point): Point => [dx * Math.cos(tilt) - dy * Math.sin(tilt), dx * Math.sin(tilt) + dy * Math.cos(tilt)];
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  return [
    shape(poly([[-0.7, -1.6], [-0.7, -14], [0, -16], [0.7, -14], [0.7, -1.6]]), "slate-100", 1, INKED),
    shape(poly([[-2.6, -1.8], [2.6, -1.8], [2.6, -0.8], [-2.6, -0.8]]), "brass-600", 1, INKED),
    shape(f.band([T([0, -0.8]), T([0, 1.6])], 0.9), "wood-700", 1),
    shape(f.circle(...T([0, 2.2]), 0.7), "brass-600", 1),
  ];
}

function shield(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 6.5 * between(rng, 0.95, 1.05);
  return [
    shape(f.circle(1.2, 0, r), "blue-700", 1, INKED),
    shape(f.circle(1.2, 0, r) + f.circle(1.2, 0, r * 0.86), "slate-400", 1),
    shape(f.poly([[0.6, -r * 0.86], [1.8, -r * 0.86], [1.8, r * 0.86], [0.6, r * 0.86]]), "slate-100", 0.25),
    shape(f.circle(1.2, 0, r * 0.22), "slate-100", 1, INKED),
  ];
}

function staff(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.08, 0.08);
  const top: Point = [-lean * 22, -22];
  return [
    shape(f.band([[lean * 6, 6], top], 0.9), "wood-700", 1, INKED),
    shape(f.circle(top[0], top[1] - 1.6, 3), "cyan-400", 0.25),
    shape(f.poly([[top[0], top[1] - 4], [top[0] + 1.2, top[1] - 1.6], [top[0], top[1] + 0.4], [top[0] - 1.2, top[1] - 1.6]]), "cyan-400", 1, INKED),
  ];
}

function book(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const open = between(rng, 0.6, 1.2);
  return [
    shape(f.poly([[-4.6, -1], [0, 0.4], [4.6, -1], [4.6, 2.2], [0, 3.4], [-4.6, 2.2]]), "red-900", 1, INKED),
    shape(f.poly([[-4.2, -1.4 - open], [-0.1, -0.2], [-0.1, 2.6], [-4.2, 1.6]]), "white", 1),
    shape(f.poly([[0.1, -0.2], [4.2, -1.4 - open], [4.2, 1.6], [0.1, 2.6]]), "white", 1),
    shape(f.band([[-3.4, -0.4 - open * 0.7], [-1, 0.4]], 0.22) + f.band([[-3.4, 0.6 - open * 0.5], [-1, 1.3]], 0.22) + f.band([[1, 0.4], [3.4, -0.4 - open * 0.7]], 0.22), "slate-400", 0.7),
  ];
}

function blaster(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const charge = between(rng, 0.5, 0.9);
  return [
    shape(f.poly([[-1.4, 1.6], [0.2, 1.6], [0.6, -0.6], [-0.8, -0.6]]), "slate-900", 1, INKED),
    shape(f.poly([[-2.2, -2.6], [4.2, -2.6], [4.2, -0.4], [-2.2, -0.4]]), "slate-700", 1, INKED),
    shape(f.poly([[4.2, -2], [7, -2], [7, -1], [4.2, -1]]), "slate-400", 1),
    shape(f.circle(7.6, -1.5, 1), "cyan-400", charge),
  ];
}

/** `points` turned `degrees` about the hand (clockwise on screen). */
function aboutHand(points: readonly Point[], degrees: number): Point[] {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  return points.map(([dx, dy]) => [dx * c - dy * s, dx * s + dy * c]);
}

/** R3391000: a smith's hammer raised in the hand. */
function hammer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -16, 4);
  return [
    shape(f.band(aboutHand([[0, 2.4], [0, -9]], tilt), 0.9), "wood-700", 1, INKED),
    shape(f.poly(aboutHand([[-3.4, -8.4], [3.2, -8.4], [3.2, -11.6], [-3.4, -11.6]], tilt)), "slate-700", 1, INKED),
    shape(f.poly(aboutHand([[2.3, -8.6], [3.2, -8.6], [3.2, -11.4], [2.3, -11.4]], tilt)), "slate-100", 0.6),
  ];
}

/** R3391000: a handheld scanner, its screen and its emitter glowing. */
function scanner(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glow = between(rng, 0.2, 0.32);
  return [
    shape(f.poly([[-1, 2], [0.6, 2], [1, -0.6], [-0.8, -0.6]]), "slate-900", 1, INKED),
    shape(f.poly([[-2.4, -0.6], [4, -1.4], [4.2, -5], [-2.2, -4.2]]), "slate-700", 1, INKED),
    shape(f.poly([[-1.4, -1.4], [2.8, -2], [3, -4.2], [-1.2, -3.6]]), "cyan-400", 0.9),
    shape(f.circle(5, -3.2, 2.2), "cyan-400", glow),
    shape(f.circle(5, -3.2, 0.8), "white", 0.95),
  ];
}

/** R3391000: a glowing stylus, the line it sketches trailing from its tip. */
function stylus(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const trail = between(rng, -1, 1);
  return [
    shape(f.band([[-1, 1.4], [3.4, -4.2]], 0.7), "slate-100", 1, INKED),
    shape(f.circle(3.9, -4.8, 2.2), "cyan-400", 0.22),
    shape(f.band([[3.9, -4.8], [6, -7 + trail], [8.4, -6.2]], 0.35), "cyan-400", 0.65),
    shape(f.circle(3.9, -4.8, 0.9), "white", 0.95),
  ];
}

/** R3391000: a rolled star chart, its map marked on the roll and stars on its end. */
function chart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const roll = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-5, -1.2 + roll], [5, -2], [5.2, 1], [-4.8, 1.8 + roll]]), "slate-100", 1, INKED),
    shape(f.band([[-3.2, 0.2], [-1.2, -0.6], [1.4, 0.2], [3.2, -0.8]], 0.18) + f.circle(-3.2, 0.2, 0.35) + f.circle(1.4, 0.2, 0.35), "blue-700", 0.85),
    shape(f.ellipse(5.1, -0.5, 0.9, 1.6), "blue-900", 1, INKED),
    shape(f.circle(5.1, -1.1, 0.25) + f.circle(5.3, 0.2, 0.2) + f.circle(4.8, -0.2, 0.18), "white", 0.9),
  ];
}

/** R3391000: a fan of three glowing drone cores in the hand, each in its armoured casing. */
function cores(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spread = between(rng, 0.95, 1.1);
  const centres: Point[] = [[-3.6 * spread, -3.8], [0, -5.6], [3.6 * spread, -3.8]];
  const hex = ([cx, cy]: Point): string => {
    const points: Point[] = [];
    for (let i = 0; i < 6; i += 1) points.push([cx + 1.7 * Math.cos((i * 60 - 90) * DEGREES), cy + 1.7 * Math.sin((i * 60 - 90) * DEGREES)]);
    return f.poly(points);
  };
  return [
    shape(f.ellipse(0, -4.6, 7.6, 4.4), "cyan-400", 0.2),
    shape(centres.map(hex).join(""), "slate-700", 1, INKED),
    shape(centres.map(([cx, cy]) => f.circle(cx, cy, 0.9)).join(""), "cyan-400", 1),
    shape(centres.map(([cx, cy]) => f.circle(cx - 0.35, cy - 0.35, 0.32)).join(""), "white", 0.85),
  ];
}

/** R3391000: a pistol-gripped lockpick drill, its thin bit out front. */
function drill(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bit = between(rng, 3.4, 4.4);
  return [
    shape(f.poly([[-1.2, 2], [0.4, 2], [0.8, -0.6], [-0.8, -0.6]]), "slate-900", 1, INKED),
    shape(f.poly([[-2.6, -0.4], [3.6, -0.8], [3.8, -3.6], [-2.4, -3.6]]), "slate-700", 1, INKED),
    shape(f.poly([[3.6, -1], [5, -1.6], [5, -2.8], [3.6, -3.4]]), "slate-400", 1, INKED),
    shape(f.band([[5, -2.2], [5 + bit, -2.2]], 0.35), "slate-100", 1),
    shape(f.circle(-0.8, -2.2, 0.5), "red-500", 0.9),
  ];
}

/** R3391000: a blade coated in green venom, dripping. */
function venomBlade(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 8, 22);
  const T = (points: readonly Point[]): Point[] => aboutHand(points, tilt);
  const drop = (x: number, y: number, length: number): string => {
    const [dx, dy] = T([[x, y]])[0] ?? [x, y];
    return `M${f.p(dx, dy)}Q${f.p(dx + 0.5, dy + length)} ${f.p(dx, dy + length + 0.5)}Q${f.p(dx - 0.5, dy + length)} ${f.p(dx, dy)}Z`;
  };
  return [
    shape(f.poly(T([[-0.7, -1.6], [-0.7, -12], [0, -14], [0.7, -12], [0.7, -1.6]])), "slate-400", 1, INKED),
    shape(f.band(T([[0.45, -3], [0.45, -11.6]]), 0.5), "green-400", 0.85),
    shape(f.poly(T([[-2.2, -1.8], [2.2, -1.8], [2.2, -0.9], [-2.2, -0.9]])) + f.band(T([[0, -0.6], [0, 1.6]]), 0.9), "slate-900", 1, INKED),
    shape(drop(0.6, -5, 1.6) + drop(0.6, -9, 2.2) + drop(0.4, -12, 1.2), "lime-400", 0.95),
  ];
}

/** R3331000: the figure parts, by id, each with its kind. */
export const FIGURE_KIT: Readonly<Record<FigurePartId, KitPart>> = {
  "pose-stand": { category: "pose", draw: poseStand },
  "pose-guard": { category: "pose", draw: poseGuard },
  "pose-raise": { category: "pose", draw: poseRaise },
  "head-human": { category: "head", draw: headHuman },
  "head-felinor": { category: "head", draw: headFelinor },
  "head-pancake": { category: "head", draw: headPancake },
  "head-fruit": { category: "head", draw: headFruit },
  "head-void": { category: "head", draw: headVoid },
  "hair-short": { category: "hair", draw: hairShort },
  "hair-long": { category: "hair", draw: hairLong },
  hood: { category: "hair", draw: hood },
  "hat-pointed": { category: "hair", draw: hatPointed },
  antennae: { category: "hair", draw: antennae },
  visor: { category: "hair", draw: visor },
  toque: { category: "hair", draw: toque },
  "leaf-crown": { category: "hair", draw: leafCrown },
  "outfit-tunic": { category: "outfit", draw: tunic },
  "outfit-armor": { category: "outfit", draw: armor },
  "outfit-coat": { category: "outfit", draw: coat },
  "outfit-jacket": { category: "outfit", draw: jacket },
  "outfit-suit": { category: "outfit", draw: suit },
  "outfit-fur": { category: "outfit", draw: fur },
  "outfit-stack": { category: "outfit", draw: stack },
  "outfit-peel": { category: "outfit", draw: peel },
  "outfit-void": { category: "outfit", draw: robe },
  "item-sword": { category: "item", draw: sword },
  "item-shield": { category: "item", draw: shield },
  "item-staff": { category: "item", draw: staff },
  "item-book": { category: "item", draw: book },
  "item-blaster": { category: "item", draw: blaster },
  "pose-wide": { category: "pose", draw: poseWide },
  "pose-sit": { category: "pose", draw: poseSit },
  "pose-reach": { category: "pose", draw: poseReach },
  "head-horn": { category: "head", draw: headHorn },
  "head-lantern": { category: "head", draw: headLantern },
  helmet: { category: "hair", draw: helmet },
  goggles: { category: "hair", draw: goggles },
  headset: { category: "hair", draw: headset },
  shades: { category: "hair", draw: shades },
  "hair-silver": { category: "hair", draw: hairSilver },
  "outfit-labcoat": { category: "outfit", draw: labcoat },
  "outfit-robes": { category: "outfit", draw: robes },
  "outfit-apron": { category: "outfit", draw: apron },
  "outfit-hydraulic": { category: "outfit", draw: hydraulic },
  "outfit-wreck": { category: "outfit", draw: wreckBody },
  "item-hammer": { category: "item", draw: hammer },
  "item-scanner": { category: "item", draw: scanner },
  "item-stylus": { category: "item", draw: stylus },
  "item-chart": { category: "item", draw: chart },
  "item-core": { category: "item", draw: cores },
  "item-drill": { category: "item", draw: drill },
  "item-venom-blade": { category: "item", draw: venomBlade },
};

/** The look of a card with none of `FIGURE_LOOKS`' tags: a hooded traveller in a plain tunic. */
export const FIGURE_DEFAULT_LOOK: FigureLook = { head: "head-human", hair: "hood", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "stone-500" };

/** Each tag's people, the first a card's tags hold winning (a Jlockeed Human wears Jlockeed's suit). */
export const FIGURE_LOOKS: readonly (readonly [Tag, FigureLook])[] = [
  ["Terminus", { head: "head-void", hair: "hood", outfit: "outfit-void", item: null, skin: "black", cloth: "slate-900" }],
  ["CN", { head: "head-human", hair: "antennae", outfit: "outfit-jacket", item: "item-book", skin: "red-300", cloth: "green-700" }],
  ["KY", { head: "head-human", hair: "hat-pointed", outfit: "outfit-coat", item: "item-staff", skin: "red-300", cloth: "blue-800" }],
  ["Felinor", { head: "head-felinor", hair: null, outfit: "outfit-fur", item: "item-staff", skin: "orange-400", cloth: "violet-700" }],
  ["Pancake", { head: "head-pancake", hair: "toque", outfit: "outfit-stack", item: null, skin: "brass-600", cloth: "white" }],
  ["Fruit", { head: "head-fruit", hair: "leaf-crown", outfit: "outfit-peel", item: null, skin: "red-600", cloth: "orange-400" }],
  ["Jlockeed", { head: "head-human", hair: "visor", outfit: "outfit-suit", item: "item-blaster", skin: "red-300", cloth: "slate-700" }],
  ["Human", { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: "item-sword", skin: "red-300", cloth: "blue-700" }],
];

/** The look of the first of `FIGURE_LOOKS`' tags that `tags` holds, else the default. */
export function figureLookFor(tags: readonly Tag[]): FigureLook {
  for (const [tag, look] of FIGURE_LOOKS) if (tags.includes(tag)) return look;
  return FIGURE_DEFAULT_LOOK;
}

/**
 * The figure's shapes, back to front: its pose at its feet, its outfit at its hip, its head and hair
 * at its neck and its item at its hand, each painted by its look and tagged with its part.
 */
export function drawFigure(figure: FigureRecipe, rng: Rng): (KitShape & { readonly part: FigurePartId })[] {
  const flip = figure.flip ?? 1;
  const anchors = POSE_ANCHORS[figure.pose];
  const at = ([dx, dy]: Point): PartPlace => ({ x: figure.x + flip * figure.scale * dx, y: figure.y + figure.scale * dy, s: figure.scale, flip });
  const { look } = figure;
  const out: (KitShape & { readonly part: FigurePartId })[] = [];
  const draw = (part: FigurePartId | null, point: Point): void => {
    if (part === null) return;
    for (const drawn of FIGURE_KIT[part].draw(at(point), rng)) {
      const paint = drawn.skin === true ? look.skin : drawn.main === true ? look.cloth : drawn.paint;
      out.push({ ...drawn, paint, part });
    }
  };
  draw(figure.pose, [0, 0]);
  draw(look.outfit, anchors.hip);
  draw(look.head, anchors.neck);
  draw(look.hair, anchors.neck);
  draw(look.item, anchors.hand);
  return out;
}
