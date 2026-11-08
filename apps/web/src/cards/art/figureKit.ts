// R3331000: the figure kit (ART.md, "Scene recipes and kits"), which replaces the procedural painter's
// one silhouette in a scene: a pose (legs, arms and hands), an outfit, a head, hair or headgear and a
// held item, each a part of its own. A figure stands 56 units tall at scale 1, its feet at its anchor;
// its pose places its neck, its hip and its hand (`POSE_ANCHORS`), where the head and hair, the
// outfit and the item are drawn. A look (`FigureLook`, `FIGURE_LOOKS`) picks the parts and paints a
// tag's people: a Terminus, a CN or KY character, a Felinor, a Pancake, a Fruit, Jlockeed's people, a
// Human. Its skin repaints each part's `skin` shapes and its cloth each `main` one. Every colour is a
// palette.css name, none in Radiant's gold window. R3392000 added the poses, heads, headgear, outfits and
// items Core's scenes draw, each after the old ones of its kind.

import type { Tag } from "@jackioh/shared";

import type { FigureRecipe, KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";
import { local, type Local } from "./sceneKit.ts";

type Point = readonly [number, number];

export type PoseId = "pose-stand" | "pose-guard" | "pose-raise" | "pose-lunge" | "pose-run" | "pose-sit" | "pose-kneel" | "pose-cast" | "pose-point" | "pose-hold" | "pose-spread";
export type HeadId = "head-human" | "head-felinor" | "head-pancake" | "head-fruit" | "head-void" | "head-lizard" | "head-robot" | "head-stone" | "head-ghoul" | "head-lantern" | "head-cloud";
export type HairId = "hair-short" | "hair-long" | "hood" | "hat-pointed" | "antennae" | "visor" | "toque" | "leaf-crown" | "steel-helmet" | "horns" | "cap-feather" | "bucket-hat" | "spectacles" | "headband" | "face-mask" | "goggles" | "veil" | "hat-brim";
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
  | "outfit-plate"
  | "outfit-waistcoat"
  | "outfit-sweater"
  | "outfit-labcoat"
  | "outfit-cardigan"
  | "outfit-bomber"
  | "outfit-apron"
  | "outfit-hoodie"
  | "outfit-rags"
  | "outfit-rock"
  | "outfit-robe"
  | "outfit-blazer"
  | "outfit-stripes"
  | "outfit-lantern";
export type ItemId =
  | "item-sword"
  | "item-shield"
  | "item-staff"
  | "item-book"
  | "item-blaster"
  | "item-rapier"
  | "item-dagger"
  | "item-tower-shield"
  | "item-coin"
  | "item-card"
  | "item-stopwatch"
  | "item-chalk"
  | "item-cone"
  | "item-map"
  | "item-clipboard"
  | "item-trumpet"
  | "item-binoculars"
  | "item-spade"
  | "item-light-stick"
  | "item-club"
  | "item-spear"
  | "item-broom";
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
  "pose-lunge": { neck: [0, -42], hand: [17, -31], hip: [0, -22] },
  "pose-run": { neck: [0, -44], hand: [9, -30], hip: [0, -24] },
  "pose-sit": { neck: [0, -33], hand: [8, -15], hip: [0, -13] },
  "pose-kneel": { neck: [0, -35], hand: [9, -27], hip: [0, -15] },
  "pose-cast": { neck: [0, -44], hand: [14, -34], hip: [0, -24] },
  "pose-point": { neck: [0, -44], hand: [13, -41], hip: [0, -24] },
  "pose-hold": { neck: [0, -44], hand: [6, -30], hip: [0, -24] },
  "pose-spread": { neck: [0, -44], hand: [17, -37], hip: [0, -24] },
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

/** A fencer's lunge toward the facing side: front knee bent, back leg stretched, near arm thrust level. */
function poseLunge(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.4, -22], [-9, -11.5], [-16, -1.5]],
      [[2.4, -22], [10, -16], [10, -1.5]],
    ],
    [
      [[-6.6, -39.5], [-11, -36], [-13.5, -31]],
      [[6.6, -39.5], [12, -35.5 + reach * 0.4], [17, -31]],
    ],
  );
}

/** Mid-stride: front knee up, back foot pushing off, arms pumping. */
function poseRun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pump = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.4, -24], [-8, -13], [-14, -2.5]],
      [[2.4, -24], [9, -17], [7, -6.5]],
    ],
    [
      [[-6.6, -41], [-11, -37 + pump], [-12, -31]],
      [[6.6, -41], [10.5, -36 + pump], [9, -30]],
    ],
  );
}

/** Seated at hip height: thighs forward, shins down, hands forward. */
function poseSit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lap = between(rng, -0.5, 0.5);
  return limbs(
    f,
    [
      [[-2.4, -13], [4.4, -13.4], [6.6, -1.5]],
      [[2.4, -13], [6.4, -13.4], [8, -1.5]],
    ],
    [
      [[-6.6, -30], [-4.6, -22], [2 + lap, -16]],
      [[6.6, -30], [9.6, -22], [8, -15]],
    ],
  );
}

/** One knee down: the front foot flat, the back knee on the ground, the near arm raised forward. */
function poseKneel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.5, 0.5);
  return limbs(
    f,
    [
      [[-2.4, -15], [-3, -2], [-11, -2]],
      [[2.4, -15], [9, -14], [9, -1.5]],
    ],
    [
      [[-6.6, -32], [-4, -25], [3, -19]],
      [[6.6, -32], [11, -30 + reach], [9, -27]],
    ],
  );
}

/** Both arms thrown forward at shoulder height, feet planted wide. */
function poseCast(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spread = between(rng, 0, 1);
  return limbs(
    f,
    [
      [[-2.6, -24], [-5.4 - spread, -12], [-7 - spread, -1.5]],
      [[2.6, -24], [4.6 + spread, -12], [6 + spread, -1.5]],
    ],
    [
      [[-6.6, -41], [3, -39.5], [12, -36.5]],
      [[6.6, -41], [10.4, -37.4], [14, -34]],
    ],
  );
}

/** The near arm up and forward, pointing; the far hand on the hip. */
function posePoint(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.4, -24], [-2.8, -12], [-3.2, -1.5]],
      [[2.4, -24], [3.2, -12], [4.4, -1.5]],
    ],
    [
      [[-6.6, -41], [-10, -32], [-4.4, -25]],
      [[6.6, -41], [10, -42.4 + lift], [13, -41]],
    ],
  );
}

/** Both arms cradling at the chest. */
function poseHold(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hug = between(rng, -0.4, 0.4);
  return limbs(
    f,
    [
      [[-2.4, -24], [-2.6, -12], [-3.2, -1.5]],
      [[2.4, -24], [2.9, -12], [3.6, -1.5]],
    ],
    [
      [[-6.6, -41], [-8.2, -34.6], [-1.6 + hug, -29.4]],
      [[6.6, -41], [9, -35], [6, -30]],
    ],
  );
}

/** Arms spread wide to block, feet apart. */
function poseSpread(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stance = between(rng, 0, 1);
  return limbs(
    f,
    [
      [[-2.6, -24], [-5.4 - stance, -12], [-7 - stance, -1.5]],
      [[2.6, -24], [5 + stance, -12], [6.6 + stance, -1.5]],
    ],
    [
      [[-6.6, -41], [-12, -39], [-17, -37]],
      [[6.6, -41], [12, -39], [17, -37]],
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

/** Layered plate on the torso, big pauldrons and a tasset skirt, in the look's cloth. */
function outfitPlate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.2, 0], [-6.2, 0]]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-6.6, -12], [6.6, -12]], 0.6) + f.band([[-6.4, -7.2], [6.4, -7.2]], 0.6) + f.band([[-6.2, -2.6], [6.2, -2.6]], 0.6), "slate-900", 0.4),
    shape(f.ellipse(-7.4, SHOULDER + 0.8 + lift, 3.4, 2.5) + f.ellipse(7.4, SHOULDER + 0.8 - lift, 3.4, 2.5), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6.4, 0.6], [-2.2, 0.6], [-2.8, 7.8], [-7, 7.2]]) + f.poly([[-1.6, 0.6], [1.6, 0.6], [1.4, 8.4], [-1.4, 8.4]]) + f.poly([[2.2, 0.6], [6.4, 0.6], [7, 7.2], [2.8, 7.8]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6.5, -1.4], [6.5, -1.4], [6.4, 0.8], [-6.4, 0.8]]), "wood-700", 1, INKED),
    shape(f.circle(-7.4, SHOULDER + 0.6, 0.45) + f.circle(7.4, SHOULDER + 0.6, 0.45) + f.circle(-4, -0.3, 0.4) + f.circle(0, -0.3, 0.4) + f.circle(4, -0.3, 0.4), "slate-100", 1),
  ];
}

/** A loud checked waistcoat over a white shirt, with a watch chain. */
function outfitWaistcoat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 0.4, 1.2);
  const checks: string[] = [];
  for (let row = 0; row < 4; row++) for (let col = 0; col < 2; col++) {
    const y = -14.4 + row * 3.2;
    const left = -5.6 + col * 1.6 + (row % 2) * 1.6;
    if (left < -2.6) checks.push(f.poly([[left, y], [left + 1.6, y], [left + 1.6, y + 1.6], [left, y + 1.6]]));
    const right = 2.4 + col * 1.6 + (row % 2) * 1.6;
    if (right + 1.6 < 6) checks.push(f.poly([[right, y], [right + 1.6, y], [right + 1.6, y + 1.6], [right, y + 1.6]]));
  }
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, 1.2], [-6.4, 1.2]]), "white", 1, INKED),
    shape(f.poly([[-6.6, SHOULDER], [-2.4, SHOULDER], [0, -8], [2.4, SHOULDER], [6.6, SHOULDER], [6.2, 1.2], [-6.2, 1.2]]), "red-600", 1, MAIN_INKED),
    shape(checks.join(""), "slate-900", 0.5),
    shape(f.band([[-4.6, -4], [-1, -2.2 + sag], [2.4, -4.4]], 0.35), "slate-100", 1),
    shape(f.circle(-4.8, -4, 0.7), "slate-100", 1, INKED),
  ];
}

/** A plain sweater: ribbed collar and hem, cable lines down the front. */
function outfitSweater(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const twist = between(rng, 0.5, 0.9);
  const cable = (x: number): string => f.band([[x, -14], [x + twist, -11.5], [x, -9], [x + twist, -6.5], [x, -4]], 0.5);
  return [
    shape(f.poly([[-7.2, SHOULDER], [7.2, SHOULDER], [7, 0.4], [-7, 0.4]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-3, SHOULDER], [3, SHOULDER], [2.4, SHOULDER + 1.6], [-2.4, SHOULDER + 1.6]]), "slate-900", 0.4),
    shape(f.poly([[-7.1, 0.4], [7.1, 0.4], [7, 2.8], [-7, 2.8]]), "stone-500", 1, MAIN_INKED),
    shape([-5.6, -3.6, -1.6, 0.4, 2.4, 4.4, 6.2].map((x) => f.band([[x, 0.6], [x, 2.6]], 0.3)).join(""), "slate-900", 0.4),
    shape(cable(-3.6) + cable(2.6), "slate-900", 0.3),
  ];
}

/** A long open white coat to the hem, the shirt beneath in the look's cloth, two pockets. */
function outfitLabcoat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-2.8, SHOULDER], [2.8, SHOULDER], [2.6, 2], [-2.6, 2]]), "blue-700", 1, MAIN_INKED),
    shape(f.poly([[-7, SHOULDER], [-2.4, SHOULDER], [-3 + swing, HEM], [-9.4, HEM], [-7, 2]]) + f.poly([[2.4, SHOULDER], [7, SHOULDER], [7, 2], [9.4, HEM], [3 - swing, HEM]]), "white", 1, INKED),
    shape(f.poly([[-2.4, SHOULDER], [-4.2, SHOULDER - 0.8], [-3, SHOULDER + 5]]) + f.poly([[2.4, SHOULDER], [4.2, SHOULDER - 0.8], [3, SHOULDER + 5]]), "slate-400", 0.6),
    shape(f.poly([[-6.4, -2], [-3.6, -2], [-3.6, 2.6], [-6.4, 2.6]]) + f.poly([[3.6, -2], [6.4, -2], [6.4, 2.6], [3.6, 2.6]]), "slate-400", 0.55, INKED),
    shape(f.band([[-6, 5], [-6.4, HEM - 1]], 0.4) + f.band([[6, 5], [6.4, HEM - 1]], 0.4), "slate-400", 0.35),
  ];
}

/** A cardigan over a white shirt and tie, buttoned, with faint chalk dust. */
function outfitCardigan(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dust = (): number => between(rng, -0.5, 0.5);
  return [
    shape(f.poly([[-3, SHOULDER], [3, SHOULDER], [2.6, 1.4], [-2.6, 1.4]]), "white", 1, INKED),
    shape(f.poly([[-0.8, SHOULDER + 0.6], [0.8, SHOULDER + 0.6], [1.1, -8], [0, -4.4], [-1.1, -8]]), "blue-900", 1),
    shape(f.poly([[-7, SHOULDER], [-2.2, SHOULDER], [-1.8, 1.8], [-7, 1.8]]) + f.poly([[2.2, SHOULDER], [7, SHOULDER], [7, 1.8], [1.8, 1.8]]), "wood-700", 1, MAIN_INKED),
    shape(f.circle(-2.8, -9, 0.5) + f.circle(-2.8, -4.6, 0.5) + f.circle(-2.8, -0.2, 0.5), "slate-100", 1),
    shape(f.circle(-5.2 + dust(), -10.6, 0.5) + f.circle(5 + dust(), -8 + dust(), 0.45) + f.circle(-4 + dust(), -2.6, 0.4) + f.circle(3.8, -12.4 + dust(), 0.4) + f.circle(5.6, -2.2 + dust(), 0.35), "white", 0.3),
  ];
}

/** A bomber jacket: ribbed hem, collar and zip, white and sky-blue sequin dots. */
function outfitBomber(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dot = (x: number, y: number): [number, number] => [x + between(rng, -0.4, 0.4), y + between(rng, -0.4, 0.4)];
  const a = [dot(-4.8, -13), dot(-1.4, -10.4), dot(3.4, -12.2), dot(-4, -5.4), dot(5, -6.4)];
  const b = [dot(-2.2, -14.4), dot(4.6, -9.4), dot(-5.6, -9.6), dot(0.8, -6), dot(-1.6, -2.4)];
  return [
    shape(f.poly([[-7.4, SHOULDER], [7.4, SHOULDER], [7.2, 0.2], [-7.2, 0.2]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-7.2, 0.2], [7.2, 0.2], [6.8, 2.8], [-6.8, 2.8]]) + f.poly([[-3.4, SHOULDER], [3.4, SHOULDER], [2.6, SHOULDER + 1.8], [-2.6, SHOULDER + 1.8]]), "slate-900", 0.55, INKED),
    shape(f.band([[0, SHOULDER + 1.8], [0, 0.2]], 0.35), "slate-100", 0.7),
    shape(a.map(([x, y]) => f.circle(x, y, 0.45)).join(""), "white", 0.9),
    shape(b.map(([x, y]) => f.circle(x, y, 0.45)).join(""), "sky-300", 0.9),
  ];
}

/** A stone canvas apron with a pocket over a shirt in the look's cloth. */
function outfitApron(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, 5.4, 6.2);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, 1.6], [-6.4, 1.6]]), "blue-700", 1, MAIN_INKED),
    shape(f.band([[-3.6, SHOULDER + 1.4], [-2.6, SHOULDER]], 0.8) + f.band([[3.6, SHOULDER + 1.4], [2.6, SHOULDER]], 0.8), "stone-500", 1),
    shape(f.poly([[-3.8, SHOULDER + 1.4], [3.8, SHOULDER + 1.4], [5, -4], [flare, 7.4], [-flare, 7.4], [-5, -4]]), "stone-500", 1, INKED),
    shape(f.poly([[-3, -2.2], [3, -2.2], [3, 3], [-3, 3]]), "slate-900", 0.25, INKED),
    shape(f.band([[-3, -2.2], [3, -2.2]], 0.4), "slate-100", 0.5),
  ];
}

/** An oversized hoodie: hood behind the neck, pouch pocket, drawstrings. */
function outfitHoodie(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const string = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-6.2, SHOULDER + 0.6], [-4.8, SHOULDER - 3.2], [0, SHOULDER - 3.8], [4.8, SHOULDER - 3.2], [6.2, SHOULDER + 0.6]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-8, SHOULDER], [8, SHOULDER], [8.6, 3], [-8.6, 3]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-3.4, SHOULDER], [3.4, SHOULDER], [2.2, SHOULDER + 2.4], [-2.2, SHOULDER + 2.4]]), "slate-900", 0.4),
    shape(f.poly([[-4.8, -6.4], [4.8, -6.4], [5.4, -0.6], [-5.4, -0.6]]) + f.poly([[-8.6, 1.4], [8.6, 1.4], [8.6, 3], [-8.6, 3]]), "slate-900", 0.25),
    shape(f.band([[-1.5, SHOULDER + 1.4], [-1.6 + string, -10]], 0.4) + f.band([[1.5, SHOULDER + 1.4], [1.6 - string, -10]], 0.4), "white", 0.9),
  ];
}

/** Torn rags with a jagged hem, bandage wraps across the chest. */
function outfitRags(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -1.2, 1.2);
  const hem: Point[] = [[6.8, 3], [5.4, 7 + j()], [4, 3.6], [2.4, 8 + j()], [0.8, 3.4], [-0.8, 7 + j()], [-2.4, 3.2], [-4, 8.4 + j()], [-5.4, 3.6], [-6.8, 6.4 + j()]];
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], ...hem, [-6.8, 3]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-4, -6.4], [-2.6, -4.6], [-4.6, -2.6]]) + f.poly([[3.6, -12.4], [5.4, -9.6], [2.6, -9.2]]), "slate-900", 0.35),
    shape(f.band([[-7, -13], [7, -5.4]], 1.5), "slate-100", 0.95, INKED),
    shape(f.band([[7, -14.4], [-6.6, -6.2]], 1.5), "slate-100", 0.95, INKED),
    shape(f.band([[-5.8, -9.4], [5.6, -13.2]], 0.3), "slate-400", 0.5),
  ];
}

/** A torso of stacked rock in the look's skin, with glowing seams. */
function outfitRock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.5, 0.5);
  return [
    shape(f.poly([[-7.4, SHOULDER], [7.4, SHOULDER], [7.8 + j(), -11.6], [-7.6 + j(), -11.2]]) + f.poly([[-7.8, -10.4], [7.6, -10.6], [7.2 + j(), -5.4], [-7.4 + j(), -5]]), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-7, -3.8], [7.2, -4], [6.6 + j(), 1.6], [-6.8, 1.8]]) + f.poly([[-5.6, 2.6], [5.8, 2.4], [5 + j(), 6.4], [-5.2, 6.6]]), "stone-500", 1, SKIN_INKED),
    shape(f.band([[-7.6, -11], [-3, -10.6], [1, -11.2], [7.6, -10.6]], 1) + f.band([[-7.2, -4.4], [-1, -4.8], [3.4, -4], [7, -4.2]], 1) + f.band([[-5.6, 2], [0, 2.8], [5.6, 2.2]], 1), "ember-500", 0.45),
    shape(f.band([[-7.6, -11], [-3, -10.6], [1, -11.2], [7.6, -10.6]], 0.45) + f.band([[-7.2, -4.4], [-1, -4.8], [3.4, -4], [7, -4.2]], 0.45) + f.band([[-5.6, 2], [0, 2.8], [5.6, 2.2]], 0.45), "orange-400", 1),
    shape(f.poly([[-6, SHOULDER + 1.2], [-2.6, SHOULDER + 1.2], [-3.2, -12.4], [-6.4, -12.4]]), "white", 0.15),
  ];
}

/** A long white robe to the hem with soft folds and a sky-blue sash. */
function gown(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.6, 0.6);
  const hem: Point[] = [[9.2, HEM], [6.6, HEM - 1.8 + j()], [3.8, HEM], [1, HEM - 1.6 + j()], [-1.8, HEM], [-4.6, HEM - 1.8 + j()], [-7, HEM], [-9.2, HEM - 0.4]];
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [8.2, 4], ...hem, [-8.2, 4]]), "white", 1, MAIN_INKED),
    shape(f.band([[-3.4, -6], [-4.2, 6], [-5, HEM - 2]], 0.7) + f.band([[1.4, -6], [1.8, 8], [2.6, HEM - 2]], 0.7) + f.band([[4.8, -8], [6, 6], [7, HEM - 2]], 0.6), "slate-900", 0.12),
    shape(f.poly([[-2.6, SHOULDER + 1], [2.6, SHOULDER + 1], [1.6, -11], [0, -9.4], [-1.6, -11]]), "slate-900", 0.14),
    shape(f.poly([[-7.2, -5], [7.2, -5], [7.4, -2.2], [-7.4, -2.2]]), "sky-300", 1, INKED),
    shape(f.poly([[2.2, -2.2], [4, -2.2], [4.6, 5.4], [3, 5.6]]), "sky-300", 1, INKED),
  ];
}

/** A plain suit jacket: lapels, a white shirt and a dark tie. */
function outfitBlazer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hem = between(rng, 1.8, 2.8);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.6, hem], [-6.6, hem]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-2.8, SHOULDER], [2.8, SHOULDER], [0, -7]]), "white", 1),
    shape(f.poly([[-0.7, SHOULDER + 0.8], [0.7, SHOULDER + 0.8], [1, -9], [0, -6.4], [-1, -9]]), "slate-900", 1),
    shape(f.poly([[-2.8, SHOULDER], [-4.8, SHOULDER - 1.2], [-0.2, -6.4]]) + f.poly([[2.8, SHOULDER], [4.8, SHOULDER - 1.2], [0.2, -6.4]]), "slate-900", 0.45),
    shape(f.circle(1.6, -3.4, 0.5), "slate-900", 1),
  ];
}

/** A striped suit with a bow tie. */
function outfitStripes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.2, 0.2);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, 1.4], [-6.4, 1.4]]), "slate-700", 1, MAIN_INKED),
    shape([-6, -3.8, -1.4, 1.4, 3.8, 6].map((x) => f.band([[x, SHOULDER + 1], [x * 0.95, 1]], 0.5)).join(""), "white", 0.55),
    shape(f.poly([[-2.4, SHOULDER], [2.4, SHOULDER], [0, -8]]), "white", 1),
    shape(f.poly([[0, -16.4 + tilt], [-2.8, -17.8], [-2.8, -15]]) + f.poly([[0, -16.4 + tilt], [2.8, -17.8], [2.8, -15]]), "red-600", 1, INKED),
    shape(f.circle(0, -16.4 + tilt, 0.65), "red-900", 1),
  ];
}

/** A lantern-bodied chest: a cloth-coloured metal frame round glass panes, a blazing flame inside. */
function outfitLantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flick = between(rng, -0.5, 0.5);
  const flame = `M${f.p(0, -3)}Q${f.p(-3.2, -6.4)} ${f.p(-0.6 + flick, -12.6)}Q${f.p(0.4, -9.6)} ${f.p(1.2, -10.4)}Q${f.p(3.4, -6.6)} ${f.p(0, -3)}Z`;
  return [
    shape(f.poly([[-6.6, SHOULDER], [6.6, SHOULDER], [6.2, 1.4], [-6.2, 1.4]]) + f.poly([[-5, SHOULDER + 1.8], [-5, -0.4], [5, -0.4], [5, SHOULDER + 1.8]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-5, SHOULDER + 1.8], [5, SHOULDER + 1.8], [5, -0.4], [-5, -0.4]]), "sky-300", 0.5),
    shape(f.circle(0, -7.4, 5.6), "orange-400", 0.3),
    shape(flame, "orange-400", 1),
    shape(`M${f.p(0, -3.6)}Q${f.p(-1.4, -6)} ${f.p(0, -9)}Q${f.p(1.6, -6)} ${f.p(0, -3.6)}Z`, "white", 0.9),
    shape(f.band([[-1.8, SHOULDER + 1.8], [-1.8, -0.4]], 0.5) + f.band([[1.8, SHOULDER + 1.8], [1.8, -0.4]], 0.5), "slate-700", 1, MAIN),
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

/** A lizard's head in skin: a long snout toward the facing side, a slit eye, a crest of spines. */
function headLizard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const spine = between(rng, 0, 0.8);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "green-500", 1, SKIN),
    shape(f.poly([[-3.4, c - 2.4], [-5.2, c - 5.4 - spine], [-2.2, c - 3.8], [-3.6, c - 7 - spine], [-0.6, c - 4.6], [-1.2, c - 7.6 - spine], [1.2, c - 4.4]]), "green-500", 1, SKIN_INKED),
    shape(f.ellipse(-0.4, c, 4.4, 4), "green-500", 1, SKIN_INKED),
    shape(f.poly([[1.4, c - 2.2], [8.8, c - 0.8], [8.8, c + 1.8], [1.4, c + 3]]), "green-500", 1, SKIN_INKED),
    shape(f.band([[3, c + 2.2], [8.4, c + 1.2]], 0.3) + f.circle(8, c - 0.2, 0.3), "slate-900", 0.7),
    shape(f.ellipse(1.6, c - 1.4, 0.95, 0.8), "lime-400", 1),
    shape(f.ellipse(1.6, c - 1.4, 0.2, 0.75), "black", 1),
  ];
}

/** A boxy chrome head: two glowing eyes and a toothy grille mouth. */
function headRobot(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flick = between(rng, 0.8, 1);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "slate-400", 1, SKIN),
    shape(f.poly([[-4.6, c - 4.4], [4.6, c - 4.4], [4.6, c + 4.2], [-4.6, c + 4.2]]), "slate-400", 1, SKIN_INKED),
    shape(f.band([[0, c - 4.4], [0, c - 6.4]], 0.4) + f.circle(0, c - 6.8, 0.7), "slate-700", 1),
    shape(f.poly([[-4, c - 2.6], [4, c - 2.6], [4, c + 0.2], [-4, c + 0.2]]), "slate-900", 0.55),
    shape(f.circle(-1.9, c - 1.2, 1.6) + f.circle(1.9, c - 1.2, 1.6), "cyan-400", 0.3),
    shape(f.circle(-1.9, c - 1.2, 0.85) + f.circle(1.9, c - 1.2, 0.85), "cyan-400", flick),
    shape(f.poly([[-3, c + 1.6], [3, c + 1.6], [3, c + 3.4], [-3, c + 3.4]]), "slate-900", 1),
    shape([-2, -0.7, 0.7, 2].map((x) => f.band([[x, c + 1.7], [x, c + 3.3]], 0.35)).join(""), "white", 0.8),
  ];
}

/** A rough boulder head: glowing orange eyes and cracks. */
function headStone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const j = (): number => between(rng, -0.4, 0.4);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.8), "stone-500", 1, SKIN),
    shape(f.poly([[-4.4, c - 2.4 + j()], [-2.4, c - 4.8], [1.4 + j(), c - 5.2], [4.2, c - 3.2], [5, c + 0.6 + j()], [3.6, c + 4], [-0.6, c + 4.8], [-3.8, c + 3.6], [-5.2, c + 0.6]]), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-3.4, c - 3.6], [0.4, c - 4.4], [-1, c - 2.8]]), "white", 0.25),
    shape(f.circle(-1.8, c - 0.2, 1.5) + f.circle(1.8, c - 0.2, 1.5), "orange-400", 0.3),
    shape(f.ellipse(-1.8, c - 0.2, 0.9, 0.6) + f.ellipse(1.8, c - 0.2, 0.9, 0.6), "orange-400", 1),
    shape(f.band([[1.4, c - 4.8], [0.6, c - 3], [1.6, c - 1.8]], 0.3) + f.band([[-3.8, c + 1.8], [-2.4, c + 2.8], [-2.8, c + 4.2]], 0.3) + f.band([[2.4, c + 2], [3.6, c + 3.2]], 0.3), "slate-900", 0.7),
  ];
}

/** A gaunt head: sunken dark eyes with pale pupils, a wide toothy grin. */
function headGhoul(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const gaze = between(rng, -0.3, 0.3);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.2), "stone-500", 1, SKIN),
    shape(f.ellipse(0, c + 0.2, 4, 5), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-3.8, c + 1], [-2.4, c + 2.6], [-3.6, c + 3.4]]) + f.poly([[3.8, c + 1], [2.4, c + 2.6], [3.6, c + 3.4]]), "slate-900", 0.3),
    shape(f.circle(-1.8, c - 0.8, 1.3) + f.circle(1.8, c - 0.8, 1.3), "slate-900", 1),
    shape(f.circle(-1.8 + gaze, c - 0.8, 0.45) + f.circle(1.8 + gaze, c - 0.8, 0.45), "slate-100", 1),
    shape(f.poly([[-3, c + 2.2], [3, c + 2.2], [2.6, c + 3.8], [-2.6, c + 3.8]]), "slate-900", 1),
    shape([-2, -0.7, 0.7, 2].map((x) => f.band([[x, c + 2.3], [x, c + 3.5]], 0.4)).join(""), "slate-100", 1),
  ];
}

/** A glass lantern head: metal cap and base, translucent panes, a blazing flame. */
function headLantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flick = between(rng, -0.4, 0.4);
  const flame = `M${f.p(0, c + 2.6)}Q${f.p(-2.6, c)} ${f.p(-0.4 + flick, c - 3.6)}Q${f.p(0.4, c - 1.8)} ${f.p(1, c - 2.2)}Q${f.p(2.8, c + 0.2)} ${f.p(0, c + 2.6)}Z`;
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "slate-700", 1, SKIN),
    shape(f.poly([[-4.4, c - 3.6], [4.4, c - 3.6], [4.4, c + 3.6], [-4.4, c + 3.6]]), "sky-300", 0.45, INKED),
    shape(f.circle(0, c, 3.8), "orange-400", 0.3),
    shape(flame, "orange-400", 1),
    shape(`M${f.p(0, c + 1.8)}Q${f.p(-1.1, c + 0.2)} ${f.p(0, c - 1.6)}Q${f.p(1.2, c + 0.2)} ${f.p(0, c + 1.8)}Z`, "white", 0.95),
    shape(f.poly([[-3.6, c - 4.8], [3.6, c - 4.8], [4.6, c - 3.4], [-4.6, c - 3.4]]) + f.poly([[-4.6, c + 3.4], [4.6, c + 3.4], [3.8, c + 5], [-3.8, c + 5]]), "slate-700", 1, SKIN_INKED),
    shape(f.band([[0, c - 4.8], [0, c - 6.4]], 0.5) + f.band([[-4, c - 3.6], [-4, c + 3.6]], 0.45) + f.band([[4, c - 3.6], [4, c + 3.6]], 0.45), "slate-700", 1, SKIN),
  ];
}

/** A puffy cloud head: a furrowed brow over calm blue eyes. */
function headCloud(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const puff = between(rng, 0, 0.5);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.4), "slate-100", 1, SKIN),
    shape(f.circle(-3.6, c + 0.4, 2.8), "slate-100", 1, SKIN),
    shape(f.circle(3.6, c + 0.4, 2.8), "slate-100", 1, SKIN),
    shape(f.circle(-1.8, c - 2.8, 3 + puff), "slate-100", 1, SKIN),
    shape(f.circle(2.2, c - 2.8, 2.8), "slate-100", 1, SKIN),
    shape(f.circle(0, c + 0.4, 3.9), "slate-100", 1, SKIN_INKED),
    shape(f.ellipse(0, c + 3.8, 3.6, 1), "slate-900", 0.12),
    shape(f.band([[-3, c - 2.3], [-1, c - 1.5]], 0.5) + f.band([[3, c - 2.3], [1, c - 1.5]], 0.5) + f.band([[-0.6, c - 3.4], [0.6, c - 3.4]], 0.3), "slate-700", 0.9),
    shape(f.circle(-1.6, c - 0.2, 0.55) + f.circle(1.6, c - 0.2, 0.55), "blue-800", 1),
  ];
}

// Hair and headgear, drawn at the neck

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

/** A steel helmet with cheek guards, over the head at the neck. */
function steelHelmet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const shine = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-5.4, c + 1.2)}C${f.p(-5.8, c - 7.8)} ${f.p(5.8, c - 7.8)} ${f.p(5.4, c + 1.2)}L${f.p(5.2, c - 1.4)}L${f.p(-5.2, c - 1.4)}Z`, "slate-400", 1, INKED),
    shape(f.poly([[-5.4, c - 1.6], [5.4, c - 1.6], [5.4, c - 0.6], [-5.4, c - 0.6]]), "slate-700", 1),
    shape(f.poly([[-5.2, c - 0.6], [-3.8, c - 0.6], [-3.4, c + 3.6], [-4.8, c + 3]]) + f.poly([[3.8, c - 0.6], [5.2, c - 0.6], [4.8, c + 3], [3.4, c + 3.6]]), "slate-400", 1, INKED),
    shape(f.poly([[-0.6, c - 1.6], [0.6, c - 1.6], [0.5, c + 1.6], [-0.5, c + 1.6]]), "slate-400", 1),
    shape(f.poly([[-3.4 + shine, c - 5.4], [-1.8 + shine, c - 5.6], [-3.2 + shine, c - 2.6], [-4.2 + shine, c - 2.6]]), "white", 0.45),
  ];
}

/** Two coiled horns over the temples, no hair. */
function horns(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const curl = between(rng, -0.6, 0.6);
  const side = (s: number): Point[] => [[s * 3.2, c - 3.4], [s * 6.2, c - 4.6], [s * 7.6, c - 8 + curl], [s * 5.6, c - 10.4]];
  const tip = (s: number): Point[] => [[s * 7.6, c - 8 + curl], [s * 5.6, c - 10.4]];
  return [
    shape(f.band(side(-1), 1.6) + f.band(side(1), 1.6), "stone-500", 1, INKED),
    shape(f.band(tip(-1), 0.9) + f.band(tip(1), 0.9), "slate-100", 1),
    shape(f.band([[-3.4, c - 3.6], [-5.6, c - 4.4]], 0.5) + f.band([[3.4, c - 3.6], [5.6, c - 4.4]], 0.5), "slate-900", 0.35),
  ];
}

/** A cap in the look's cloth with a long white feather. */
function capFeather(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sweep = between(rng, -1, 1);
  const feather = `M${f.p(-2.6, c - 4.6)}Q${f.p(-6 + sweep, c - 8)} ${f.p(-9.4 + sweep, c - 9)}Q${f.p(-6.4, c - 5.4)} ${f.p(-2.6, c - 4.6)}Z`;
  return [
    shape(feather, "white", 1, INKED),
    shape(f.band([[-2.8, c - 4.6], [-8.6 + sweep, c - 8.6]], 0.3), "slate-400", 0.9),
    shape(`M${f.p(-5, c - 0.8)}C${f.p(-5.4, c - 7.6)} ${f.p(5.4, c - 7.6)} ${f.p(5, c - 0.8)}Z`, "blue-800", 1, MAIN_INKED),
    shape(f.poly([[0, c - 2.6], [7, c - 2], [7, c - 1], [0, c - 1.4]]), "blue-800", 1, MAIN_INKED),
  ];
}

/** An oversized bucket hat in the look's cloth, wide brim all round. */
function bucketHat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tilt = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(0, c - 2.6 + tilt, 8.6, 1.9), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-4.8, c - 3], [-4.2, c - 8.4], [4.2, c - 8.4], [4.8, c - 3]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-4.7, c - 3.4], [4.7, c - 3.4], [4.6, c - 4.6], [-4.6, c - 4.6]]), "slate-900", 0.35),
    shape(f.ellipse(0, c - 8.4, 4.2, 0.7), "slate-900", 0.25),
  ];
}

/** Neat side-parted wood-brown hair and round glasses with white glints. */
function spectacles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const part = between(rng, -1.6, -0.8);
  const ring = (x: number): string => f.circle(x, c + 0.2, 1.6) + f.circle(x, c + 0.2, 1.15);
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.8)} ${f.p(5.4, c - 6.8)} ${f.p(5, c + 0.5)}Q${f.p(2.4, c - 2.6)} ${f.p(part, c - 2.6)}Q${f.p(-2.4, c - 2)} ${f.p(-5, c + 0.5)}Z`, "wood-800", 1, INKED),
    shape(f.poly([[part - 0.2, c - 2.8], [part + 3, c - 4.4], [part + 1.6, c - 2]]), "wood-800", 1),
    shape(ring(-1.8) + ring(1.8) + f.band([[-0.2, c + 0.1], [0.2, c + 0.1]], 0.35) + f.band([[-3.4, c + 0.1], [-4.8, c - 0.2]], 0.3) + f.band([[3.4, c + 0.1], [4.8, c - 0.2]], 0.3), "slate-900", 1),
    shape(f.circle(-2.4, c - 0.4, 0.35) + f.circle(1.2, c - 0.4, 0.35), "white", 0.9),
  ];
}

/** A red headband with trailing ends over short hair. */
function headband(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flutter = between(rng, -0.8, 0.8);
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.6)} ${f.p(5.4, c - 6.6)} ${f.p(5, c + 0.5)}Q${f.p(2, c - 2.8)} ${f.p(0, c - 2.8)}Q${f.p(-2.4, c - 2.4)} ${f.p(-5, c + 0.5)}Z`, "wood-950", 1, INKED),
    shape(f.poly([[-4.9, c - 2.4], [4.9, c - 2.4], [5, c - 0.8], [-5, c - 0.8]]), "red-500", 1, INKED),
    shape(f.poly([[-4.8, c - 1.8], [-8.6, c - 3.4 + flutter], [-8.2, c - 1.6 + flutter], [-4.8, c - 1.2]]) + f.poly([[-4.8, c - 1.2], [-9, c + 0.6 + flutter], [-7.8, c + 1.6 + flutter], [-4.8, c - 0.6]]), "red-500", 1, INKED),
  ];
}

/** A blue surgical mask over the lower face with ear loops, short greying hair. */
function faceMask(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const part = between(rng, -1, 1);
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.6)} ${f.p(5.4, c - 6.6)} ${f.p(5, c + 0.5)}Q${f.p(2, c - 2.6)} ${f.p(part, c - 2.4)}Q${f.p(-2.4, c - 2.2)} ${f.p(-5, c + 0.5)}Z`, "slate-400", 1, INKED),
    shape(f.band([[-3.8, c + 1], [-5, c - 0.2]], 0.3) + f.band([[3.8, c + 1], [5, c - 0.2]], 0.3), "white", 0.8),
    shape(f.poly([[-3.9, c + 0.6], [3.9, c + 0.6], [3.5, c + 3.4], [0, c + 4.4], [-3.5, c + 3.4]]), "blue-200", 1, INKED),
    shape(f.band([[-3.4, c + 1.8], [3.4, c + 1.8]], 0.3) + f.band([[-3, c + 2.9], [3, c + 2.9]], 0.3), "slate-400", 0.6),
  ];
}

/** Goggles pushed up on the forehead over wild white hair. */
function goggles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const wild = between(rng, 0, 1.2);
  return [
    shape(f.poly([[-5.2, c + 0.8], [-6.6, c - 3], [-4.6 - wild, c - 4.4], [-5 - wild, c - 8.2], [-2, c - 6], [-0.6, c - 9 - wild], [1.2, c - 6.2], [3.6 + wild, c - 8.6], [3.6, c - 5], [6.4, c - 4.2], [5.2, c - 0.4], [5.2, c + 0.8], [4, c - 2.4], [-4, c - 2.4]]), "slate-100", 1, INKED),
    shape(f.poly([[-5, c - 3.6], [5, c - 3.6], [5, c - 2.6], [-5, c - 2.6]]), "slate-700", 1, INKED),
    shape(f.circle(-2, c - 3.4, 2) + f.circle(2.6, c - 3.4, 2), "slate-700", 1, INKED),
    shape(f.circle(-2, c - 3.4, 1.4) + f.circle(2.6, c - 3.4, 1.4), "cyan-400", 0.9),
    shape(f.circle(-2.5, c - 3.9, 0.4) + f.circle(2.1, c - 3.9, 0.4), "white", 0.8),
  ];
}

/** A long white veil from the crown to the shoulders, a pale circlet on the brow. */
function veil(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sway = between(rng, -0.6, 0.6);
  const outer = `M${f.p(0, c - 5.8)}C${f.p(-7.4, c - 5.6)} ${f.p(-6.8, c + 2)} ${f.p(-8.4 + sway, c + 10)}L${f.p(8.4 + sway, c + 10)}C${f.p(6.8, c + 2)} ${f.p(7.4, c - 5.6)} ${f.p(0, c - 5.8)}Z`;
  return [
    shape(outer + f.ellipse(0, c + 0.2, 3.9, 4.8), "white", 0.95, INKED),
    shape(f.band([[-6, c], [-6.8 + sway, c + 8]], 0.5) + f.band([[6, c], [6.8 + sway, c + 8]], 0.5), "slate-400", 0.25),
    shape(f.band([[-4.4, c - 3], [0, c - 3.8], [4.4, c - 3]], 0.9), "slate-100", 1, INKED),
    shape(f.circle(0, c - 3.9, 0.55), "sky-300", 1),
  ];
}

/** A fedora: slate crown, wide brim, a darker band. */
function hatBrim(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const dent = between(rng, 0.4, 1);
  return [
    shape(f.ellipse(0, c - 3, 7.4, 1.5), "slate-700", 1, INKED),
    shape(`M${f.p(-4.4, c - 3.2)}C${f.p(-4.8, c - 8)} ${f.p(-1.2, c - 8.6 + dent)} ${f.p(0, c - 7.6)}C${f.p(1.2, c - 8.6 + dent)} ${f.p(4.8, c - 8)} ${f.p(4.4, c - 3.2)}Z`, "slate-700", 1, INKED),
    shape(f.poly([[-4.5, c - 3.8], [4.5, c - 3.8], [4.6, c - 5.2], [-4.6, c - 5.2]]), "slate-900", 1),
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

/** A needle-thin rapier held level forward: cup hilt, grip, pommel, about 17 long. */
function rapier(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, -1.2, 0.4);
  return [
    shape(f.poly([[2.4, -0.3], [17, -0.15 + rise], [18.6, rise], [17, 0.15 + rise], [2.4, 0.3]]), "slate-100", 1, INKED),
    shape(f.ellipse(2.2, 0, 1.1, 2.8), "brass-600", 1, INKED),
    shape(f.band([[-2.2, 0], [1.4, 0]], 0.9), "wood-700", 1),
    shape(f.circle(-2.8, 0, 0.8), "brass-600", 1),
  ];
}

/** A short dagger held point up, about 10 long. */
function dagger(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 6, 18) * DEGREES;
  const T = ([dx, dy]: Point): Point => [dx * Math.cos(tilt) - dy * Math.sin(tilt), dx * Math.sin(tilt) + dy * Math.cos(tilt)];
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  return [
    shape(poly([[-1, -1.4], [-0.8, -8], [0, -10], [0.8, -8], [1, -1.4]]), "slate-100", 1, INKED),
    shape(poly([[-2, -1.6], [2, -1.6], [2, -0.8], [-2, -0.8]]), "brass-600", 1, INKED),
    shape(f.band([T([0, -0.8]), T([0, 1.4])], 0.9), "wood-700", 1),
    shape(f.circle(...T([0, 2]), 0.6), "brass-600", 1),
  ];
}

/** A tall rectangular shield held before the figure, 18 tall and 7 wide at scale 1, centred on the hand. */
function towerShield(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 3.5 * between(rng, 0.95, 1.05);
  const rim = (inset: number): string => f.poly([[2 - w + inset, -9 + inset], [2 + w - inset, -9 + inset], [2 + w - inset, 9 - inset], [2 - w + inset, 9 - inset]]);
  return [
    shape(rim(0), "blue-800", 1, INKED),
    shape(rim(0) + rim(0.8), "slate-400", 1),
    shape(f.band([[0.4, -7], [0.4, 7]], 0.7), "white", 0.2),
    shape(f.circle(2, 0, 1.5), "slate-400", 1, INKED),
    shape(f.circle(2, 0, 0.6), "slate-100", 1),
  ];
}

/** A silver coin held up, 5 across. */
function coin(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = between(rng, 2.4, 2.8);
  return [
    shape(f.circle(1, -1.4, r), "slate-100", 1, INKED),
    shape(f.circle(1, -1.4, r * 0.75) + f.circle(1, -1.4, r * 0.6), "slate-400", 0.7),
    shape(f.poly([[0.4, -2.4], [1.6, -2.4], [1.6, -0.4], [0.4, -0.4]]), "slate-400", 0.6),
    shape(f.ellipse(0, -2.4, 0.9, 0.5), "white", 0.7),
  ];
}

/** A playing card held up, 4.4 by 6.4: blue back, white border. */
function card(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 8, 20) * DEGREES;
  const T = ([dx, dy]: Point): Point => [dx * Math.cos(tilt) - dy * Math.sin(tilt), dx * Math.sin(tilt) + dy * Math.cos(tilt)];
  const rect = (w: number, h: number): string => f.poly(([[-w, -h - 2.4], [w, -h - 2.4], [w, h - 2.4], [-w, h - 2.4]] as Point[]).map(T));
  return [
    shape(rect(2.2, 3.2), "white", 1, INKED),
    shape(rect(1.6, 2.6), "blue-800", 1),
    shape(f.poly(([[0, -4.8], [1, -2.4], [0, 0], [-1, -2.4]] as Point[]).map(T)), "slate-100", 0.6),
  ];
}

/** A stopwatch held up, 6 across: slate case, pale face, crown on top. */
function stopwatch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hand = between(rng, -1, 1);
  return [
    shape(f.poly([[-0.6, -6.8], [0.6, -6.8], [0.6, -5.6], [-0.6, -5.6]]), "slate-400", 1, INKED),
    shape(f.circle(0, -3, 3.1), "slate-700", 1, INKED),
    shape(f.circle(0, -3, 2.4), "slate-100", 1),
    shape(f.band([[0, -3], [0, -5]], 0.3) + f.band([[0, -3], [1.4 + hand, -2.4]], 0.3), "slate-900", 1),
  ];
}

/** A stick of white chalk with a dust puff at its tip, 5 long. */
function chalk(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const puff = between(rng, 0.8, 1.2);
  return [
    shape(f.band([[-1.4, 0.8], [2.6, -1.2]], 1.4), "white", 1, INKED),
    shape(f.circle(3.8, -1.8, 1.4 * puff), "white", 0.4),
    shape(f.circle(5, -3, 0.6) + f.circle(3.2, -3.6, 0.45) + f.circle(5.4, -1.2, 0.4), "white", 0.5),
  ];
}

/** An ice cream cone held up, 12 tall: a brass wafer under a pale scoop. */
function cone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drip = between(rng, 0.8, 1.6);
  return [
    shape(f.poly([[-2.2, -2], [2.2, -2], [0, 5.4]]), "brass-500", 1, INKED),
    shape(f.band([[-1.4, -1], [1, 2.6]], 0.3) + f.band([[1.4, -1], [-1, 2.6]], 0.3), "brass-800", 0.6),
    shape(f.circle(0, -4.2, 2.8), "slate-100", 1, INKED),
    shape(f.circle(-0.2, -7, 1.2), "slate-100", 1, INKED),
    shape(f.poly([[-2.6, -2.6], [-1.8, -2.6], [-1.8, -2.6 + drip], [-2.6, -2.6 + drip]]), "slate-100", 1),
    shape(f.ellipse(-1.1, -5.2, 0.8, 0.5), "white", 0.7),
  ];
}

/** A folded map held up, 9 by 6: pale paper with blue sea and green land. */
function map(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-4.4, -2 + tilt], [-1.5, -3.4], [1.5, -2.2], [4.4, -3.6 + tilt], [4.4, 2], [1.5, 3.4], [-1.5, 2.2], [-4.4, 3.6 + tilt]]), "slate-100", 1, INKED),
    shape(f.poly([[-4, -1.4 + tilt], [-1.7, -2.6], [-1.7, 1.6], [-4, 2.8 + tilt]]), "blue-600", 0.85),
    shape(f.poly([[-1.3, -2.4], [1.3, -1.4], [1.3, 2.4], [-1.3, 1.4]]), "green-400", 0.85),
    shape(f.poly([[1.7, -1.6], [4, -2.8 + tilt], [4, 1.2], [1.7, 2.6]]), "blue-600", 0.6),
    shape(f.band([[-1.5, -3.4], [-1.5, 2.2]], 0.25) + f.band([[1.5, -2.2], [1.5, 3.4]], 0.25), "slate-400", 0.7),
  ];
}

/** A clipboard held up, 6 by 8: wood board, white sheet. */
function clipboard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 6, 14) * DEGREES;
  const T = ([dx, dy]: Point): Point => [dx * Math.cos(tilt) - dy * Math.sin(tilt), dx * Math.sin(tilt) + dy * Math.cos(tilt)];
  const rect = (x0: number, y0: number, x1: number, y1: number): string => f.poly(([[x0, y0], [x1, y0], [x1, y1], [x0, y1]] as Point[]).map(T));
  return [
    shape(rect(-3, -8, 3, 0), "wood-600", 1, INKED),
    shape(rect(-2.4, -7, 2.4, -0.6), "white", 1),
    shape(rect(-1.2, -8.6, 1.2, -7.2), "slate-400", 1, INKED),
    shape([-5.6, -4.2, -2.8].map((y) => f.band([T([-1.6, y]), T([1.6, y])], 0.25)).join(""), "slate-400", 0.8),
  ];
}

/** A trumpet held level, its bell pointing forward, about 11 long. */
function trumpet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, -0.8, 0.2);
  return [
    shape(f.band([[-2.6, 0], [4, rise]], 1.1), "brass-600", 1, INKED),
    shape(f.poly([[4, -0.8 + rise], [8.4, -3.4 + rise], [8.4, 3.4 + rise], [4, 0.8 + rise]]), "brass-600", 1, INKED),
    shape(f.band([[-0.6, 0], [-0.6, -2]], 0.5) + f.band([[0.8, 0], [0.8, -2]], 0.5) + f.band([[2.2, 0], [2.2, -2]], 0.5), "brass-500", 1),
    shape(f.band([[-3.2, 0], [-2.2, 0]], 0.9), "slate-400", 1),
    shape(f.ellipse(8.4, rise, 0.7, 3), "white", 0.25),
  ];
}

/** A pair of binoculars held forward, 7 long. */
function binoculars(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const aim = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-2.2, -1.5], [3.6, -1.5 + aim]], 2.2), "slate-900", 1, INKED),
    shape(f.band([[-2.2, 1.1], [3.6, 1.1 + aim]], 2.2), "slate-900", 1, INKED),
    shape(f.band([[0, -2.4], [0, 2.2]], 1), "slate-900", 1),
    shape(f.ellipse(3.8, -1.5 + aim, 0.6, 1.2) + f.ellipse(3.8, 1.1 + aim, 0.6, 1.2), "slate-400", 0.9),
    shape(f.band([[-1.6, -2.4], [2.4, -2.4]], 0.3), "white", 0.3),
  ];
}

/** A gravedigger's spade standing on the ground below the hand, about 31 tall at scale 1. */
function spade(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.5, 0.5);
  return [
    shape(f.band([[0, -8], [lean, 17]], 0.9), "wood-600", 1, INKED),
    shape(f.poly([[-1.8, -8.6], [1.8, -8.6], [1.8, -7.6], [-1.8, -7.6]]), "wood-600", 1, INKED),
    shape(f.poly([[lean - 2.6, 16.6], [lean + 2.6, 16.6], [lean + 2.4, 21.6], [lean, 23.4], [lean - 2.4, 21.6]]), "slate-400", 1, INKED),
    shape(f.band([[lean - 1.4, 17.6], [lean - 1.4, 21]], 0.5), "white", 0.35),
  ];
}

/** A heart-shaped light stick with a red glow, 11 tall. */
function lightStick(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glow = between(rng, 0.3, 0.45);
  const heart = `M${f.p(0, -3)}C${f.p(-4.4, -5.6)} ${f.p(-2.8, -9.4)} ${f.p(0, -7.2)}C${f.p(2.8, -9.4)} ${f.p(4.4, -5.6)} ${f.p(0, -3)}Z`;
  return [
    shape(f.circle(0, -6, 5.4), "red-300", glow),
    shape(f.band([[0, 2.4], [0, -3]], 1), "slate-700", 1, INKED),
    shape(heart, "red-400", 1, INKED),
    shape(f.ellipse(-1.6, -6.6, 0.8, 0.5), "white", 0.7),
  ];
}

/** A raised wooden club, 15 long. */
function club(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.8, 0.8);
  return [
    shape(f.poly([[-0.8, 2.6], [0.8, 2.6], [2.4 + lean, -8], [2.2 + lean, -12.4], [-1.4 + lean, -12.6], [-2 + lean, -8]]), "wood-700", 1, INKED),
    shape(f.circle(0.4 + lean, -9, 0.45) + f.circle(-0.6 + lean, -11.6, 0.35), "slate-900", 0.4),
    shape(f.band([[1.6 + lean, -10], [1.2 + lean, -4]], 0.4), "white", 0.15),
  ];
}

/** A twig spear, 22 long, its tip whittled. */
function spear(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.08, 0.08);
  const top: Point = [-lean * 24, -20];
  return [
    shape(f.band([[lean * 6, 6], top], 0.8), "wood-600", 1, INKED),
    shape(f.poly([[top[0] - 0.8, top[1]], [top[0], top[1] - 4.2], [top[0] + 0.8, top[1]]]), "wood-600", 1, INKED),
    shape(f.band([[lean * 2, -4], [3 + lean * 2, -7]], 0.4), "wood-600", 1),
    shape(f.band([[top[0] - 1, top[1] + 1.6], [top[0] + 1, top[1] + 1.6]], 0.5), "wood-800", 1),
  ];
}

/** A broom standing on the ground below the hand: wood handle, brass bristles, about 33 tall. */
function broom(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.6, 0.6);
  return [
    shape(f.band([[0, -10], [lean, 13]], 0.9), "wood-600", 1, INKED),
    shape(f.poly([[lean - 0.7, 12], [lean + 0.7, 12], [lean + 3.6, 22.4], [lean - 3.6, 22.4]]), "brass-500", 1, INKED),
    shape(f.band([[lean - 1.2, 13.4], [lean + 1.2, 13.4]], 0.7), "wood-800", 1),
    shape(f.band([[lean - 0.8, 14.4], [lean - 1.8, 22]], 0.25) + f.band([[lean + 0.8, 14.4], [lean + 1.8, 22]], 0.25), "brass-800", 0.6),
  ];
}

/** R3331000: the figure parts, by id, each with its kind. */
export const FIGURE_KIT: Readonly<Record<FigurePartId, KitPart>> = {
  "pose-stand": { category: "pose", draw: poseStand },
  "pose-guard": { category: "pose", draw: poseGuard },
  "pose-raise": { category: "pose", draw: poseRaise },
  "pose-lunge": { category: "pose", draw: poseLunge },
  "pose-run": { category: "pose", draw: poseRun },
  "pose-sit": { category: "pose", draw: poseSit },
  "pose-kneel": { category: "pose", draw: poseKneel },
  "pose-cast": { category: "pose", draw: poseCast },
  "pose-point": { category: "pose", draw: posePoint },
  "pose-hold": { category: "pose", draw: poseHold },
  "pose-spread": { category: "pose", draw: poseSpread },
  "head-human": { category: "head", draw: headHuman },
  "head-felinor": { category: "head", draw: headFelinor },
  "head-pancake": { category: "head", draw: headPancake },
  "head-fruit": { category: "head", draw: headFruit },
  "head-void": { category: "head", draw: headVoid },
  "head-lizard": { category: "head", draw: headLizard },
  "head-robot": { category: "head", draw: headRobot },
  "head-stone": { category: "head", draw: headStone },
  "head-ghoul": { category: "head", draw: headGhoul },
  "head-lantern": { category: "head", draw: headLantern },
  "head-cloud": { category: "head", draw: headCloud },
  "hair-short": { category: "hair", draw: hairShort },
  "hair-long": { category: "hair", draw: hairLong },
  hood: { category: "hair", draw: hood },
  "hat-pointed": { category: "hair", draw: hatPointed },
  antennae: { category: "hair", draw: antennae },
  visor: { category: "hair", draw: visor },
  toque: { category: "hair", draw: toque },
  "leaf-crown": { category: "hair", draw: leafCrown },
  "steel-helmet": { category: "hair", draw: steelHelmet },
  horns: { category: "hair", draw: horns },
  "cap-feather": { category: "hair", draw: capFeather },
  "bucket-hat": { category: "hair", draw: bucketHat },
  spectacles: { category: "hair", draw: spectacles },
  headband: { category: "hair", draw: headband },
  "face-mask": { category: "hair", draw: faceMask },
  goggles: { category: "hair", draw: goggles },
  veil: { category: "hair", draw: veil },
  "hat-brim": { category: "hair", draw: hatBrim },
  "outfit-tunic": { category: "outfit", draw: tunic },
  "outfit-armor": { category: "outfit", draw: armor },
  "outfit-coat": { category: "outfit", draw: coat },
  "outfit-jacket": { category: "outfit", draw: jacket },
  "outfit-suit": { category: "outfit", draw: suit },
  "outfit-fur": { category: "outfit", draw: fur },
  "outfit-stack": { category: "outfit", draw: stack },
  "outfit-peel": { category: "outfit", draw: peel },
  "outfit-void": { category: "outfit", draw: robe },
  "outfit-plate": { category: "outfit", draw: outfitPlate },
  "outfit-waistcoat": { category: "outfit", draw: outfitWaistcoat },
  "outfit-sweater": { category: "outfit", draw: outfitSweater },
  "outfit-labcoat": { category: "outfit", draw: outfitLabcoat },
  "outfit-cardigan": { category: "outfit", draw: outfitCardigan },
  "outfit-bomber": { category: "outfit", draw: outfitBomber },
  "outfit-apron": { category: "outfit", draw: outfitApron },
  "outfit-hoodie": { category: "outfit", draw: outfitHoodie },
  "outfit-rags": { category: "outfit", draw: outfitRags },
  "outfit-rock": { category: "outfit", draw: outfitRock },
  "outfit-robe": { category: "outfit", draw: gown },
  "outfit-blazer": { category: "outfit", draw: outfitBlazer },
  "outfit-stripes": { category: "outfit", draw: outfitStripes },
  "outfit-lantern": { category: "outfit", draw: outfitLantern },
  "item-sword": { category: "item", draw: sword },
  "item-shield": { category: "item", draw: shield },
  "item-staff": { category: "item", draw: staff },
  "item-book": { category: "item", draw: book },
  "item-blaster": { category: "item", draw: blaster },
  "item-rapier": { category: "item", draw: rapier },
  "item-dagger": { category: "item", draw: dagger },
  "item-tower-shield": { category: "item", draw: towerShield },
  "item-coin": { category: "item", draw: coin },
  "item-card": { category: "item", draw: card },
  "item-stopwatch": { category: "item", draw: stopwatch },
  "item-chalk": { category: "item", draw: chalk },
  "item-cone": { category: "item", draw: cone },
  "item-map": { category: "item", draw: map },
  "item-clipboard": { category: "item", draw: clipboard },
  "item-trumpet": { category: "item", draw: trumpet },
  "item-binoculars": { category: "item", draw: binoculars },
  "item-spade": { category: "item", draw: spade },
  "item-light-stick": { category: "item", draw: lightStick },
  "item-club": { category: "item", draw: club },
  "item-spear": { category: "item", draw: spear },
  "item-broom": { category: "item", draw: broom },
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

/** R3392000: KY, the chalk-dusted tutor in a cardigan, as Core's KY cards draw him (#31, #51, #57, #82). */
export const KY_TUTOR_LOOK: FigureLook = { head: "head-human", hair: "spectacles", outfit: "outfit-cardigan", item: "item-chalk", skin: "red-300", cloth: "wood-700" };

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
