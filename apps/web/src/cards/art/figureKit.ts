// R3331000: the figure kit (ART.md, "Scene recipes and kits"), which replaces the procedural painter's
// one silhouette in a scene: a pose (legs, arms and hands), an outfit, a head, hair or headgear and a
// held item, each a part of its own. A figure stands 56 units tall at scale 1, its feet at its anchor;
// its pose places its neck, its hip and its hand (`POSE_ANCHORS`), where the head and hair, the
// outfit and the item are drawn. A look (`FigureLook`, `FIGURE_LOOKS`) picks the parts and paints a
// tag's people: a Terminus, a CN or KY character, a Felinor, a Pancake, a Fruit, Jlockeed's people, a
// Human. Its skin repaints each part's `skin` shapes and its cloth each `main` one. Every colour is a
// palette.css name, none in Radiant's gold window.
// R3390000: the Classic+ scenes (#3390) added six poses (spread, strike, run, sit, flex, kneel), eight
// heads, five hairs and headgears, twelve outfits and fourteen held items, each drawn as the first ones are.

import type { Tag } from "@jackioh/shared";

import { fmt } from "./hash.ts";
import { leant, sparkle, turned } from "./kitDraw.ts";
import type { FigureRecipe, KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";
import { local, type Local } from "./sceneKit.ts";

type Point = readonly [number, number];

export type PoseId = "pose-stand" | "pose-guard" | "pose-raise" | "pose-spread" | "pose-strike" | "pose-run" | "pose-sit" | "pose-flex" | "pose-kneel";
export type HeadId =
  | "head-human"
  | "head-felinor"
  | "head-pancake"
  | "head-fruit"
  | "head-void"
  | "head-bat"
  | "head-skull"
  | "head-mushroom"
  | "head-sun"
  | "head-golem"
  | "head-automaton"
  | "head-goblin"
  | "head-mummy";
export type HairId =
  | "hair-short"
  | "hair-long"
  | "hood"
  | "hat-pointed"
  | "antennae"
  | "visor"
  | "toque"
  | "leaf-crown"
  | "kerchief"
  | "hat-brim"
  | "goggles"
  | "helm"
  | "tonsure";
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
  | "outfit-apron"
  | "outfit-robe"
  | "outfit-plate"
  | "outfit-tabard"
  | "outfit-poncho"
  | "outfit-wraps"
  | "outfit-muscle"
  | "outfit-drum"
  | "outfit-cardigan"
  | "outfit-leaves"
  | "outfit-patched"
  | "outfit-stone";
export type ItemId =
  | "item-sword"
  | "item-shield"
  | "item-staff"
  | "item-book"
  | "item-blaster"
  | "item-mallet"
  | "item-dagger"
  | "item-spoon"
  | "item-banner"
  | "item-flask"
  | "item-bow"
  | "item-spear"
  | "item-coin"
  | "item-sling"
  | "item-tray"
  | "item-scroll"
  | "item-hourglass"
  | "item-sack"
  | "item-tower-shield";
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
  "pose-spread": { neck: [0, -44], hand: [17.9, -48.4], hip: [0, -24] },
  "pose-strike": { neck: [0, -43], hand: [-2.9, -58.3], hip: [0, -23] },
  "pose-run": { neck: [2, -43], hand: [14, -37.8], hip: [0, -23] },
  "pose-sit": { neck: [0, -34], hand: [10.8, -16.4], hip: [0, -14] },
  "pose-flex": { neck: [0, -44], hand: [11, -49.6], hip: [0, -24] },
  "pose-kneel": { neck: [0, -36], hand: [17.3, -42.3], hip: [0, -16] },
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

/** An ellipse about (cx, cy) in the part's frame, its `rx` axis turned `degrees` (clockwise as drawn facing right). */
function oval(f: Local, cx: number, cy: number, rx: number, ry: number, degrees: number): string {
  const [ax, ay] = turned([rx, 0], degrees);
  const arc = `A${fmt(f.r(rx))} ${fmt(f.r(ry))} ${fmt(f.sweep(1) === 1 ? degrees : -degrees)} 1 0 `;
  return `M${f.p(cx - ax, cy - ay)}${arc}${f.p(cx + ax, cy + ay)}${arc}${f.p(cx - ax, cy - ay)}Z`;
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

/** Legs apart and both arms flung wide and a little raised (shielding, welcoming); its hand at the near arm's end, 18 out and 48 up. */
function poseSpread(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stance = between(rng, 0, 1.2);
  const lift = between(rng, -1.2, 1.2);
  return limbs(
    f,
    [
      [[-2.6, -24], [-5.2 - stance * 0.5, -12], [-7.6 - stance, -1.5]],
      [[2.6, -24], [5.2 + stance * 0.5, -12], [7.6 + stance, -1.5]],
    ],
    [
      [[-6.6, -41], [-12, -43.4 - lift], [-17, -47.6 - lift * 2]],
      [[6.6, -41], [12, -43.4 + lift * 0.4], [17, -47.6]],
    ],
  );
}

/** Both hands raised together overhead, about to bring something down (a mallet, an axe, drumsticks), legs braced; its hand overhead, a little behind the crown. */
function poseStrike(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const brace = between(rng, 0, 1.2);
  const wind = between(rng, -0.8, 0.8);
  return limbs(
    f,
    [
      [[-2.6, -23], [-6 - brace, -12], [-7.6 - brace, -1.5]],
      [[2.6, -23], [5.4 + brace * 0.5, -12.4], [7 + brace, -1.5]],
    ],
    [
      [[-6.6, -40], [-9.2 - wind, -47.5], [-4.2, -55]],
      [[6.6, -40], [6, -49.5], [-2.2, -57.6]],
    ],
  );
}

/** A mid-stride sprint toward its facing, the front knee high, the back leg stretched behind and the arms pumping; its neck leant 2 forward, its hand at the front fist. */
function poseRun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const kick = between(rng, -1, 1);
  const swing = between(rng, -1, 1);
  return limbs(
    f,
    [
      [[-1.6, -23], [-5.2, -12.6], [-10.6 - kick, -2.2]],
      [[2, -23], [10.2, -18.2], [8.4 + kick * 0.5, -8.4]],
    ],
    [
      [[-6.6, -40], [-10.4, -33.2], [-14 - swing, -30 + swing]],
      [[6.6, -40], [10, -33], [13.4, -37]],
    ],
  );
}

/** Seated on a ledge or the ground, thighs forward and shins down, the hip 14 above the feet and the hands on the knees; its hand on the near knee. */
function poseSit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-1, -14], [8, -14.4], [9.4, -12.6], [9.6 + reach, -1.5]],
      [[1.6, -13.6], [10.6, -13.6], [12, -11.8], [12.2 + reach, -1.5]],
    ],
    [
      [[-6.6, -31], [-6, -22.6], [7.6, -16.6]],
      [[6.6, -31], [9.6, -24], [10.6, -17.4]],
    ],
  );
}

/** Both arms raised and bent at the elbows, fists by the head (a double-biceps flex), legs apart; its hand at the near fist. */
function poseFlex(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stance = between(rng, 0, 1.2);
  const pump = between(rng, -0.8, 0.8);
  return [
    ...limbs(
      f,
      [
        [[-2.6, -24], [-5, -12], [-6.8 - stance, -1.5]],
        [[2.6, -24], [5, -12], [6.8 + stance, -1.5]],
      ],
      [
        [[-6.6, -41], [-12.6, -41.6 - pump], [-11.2, -48.6]],
        [[6.6, -41], [12.6, -41.6], [11.2, -48.6]],
      ],
    ),
    shape(f.ellipse(-9.8, -42.8 - pump / 2, 2.4, 1.9) + f.ellipse(9.8, -42.8, 2.4, 1.9), "slate-700", 1, MAIN_INKED),
  ];
}

/** One knee on the ground and the other foot forward, the hip 16 above the feet, the near arm reaching forward and up; its hand at that arm's end. */
function poseKneel(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rest = between(rng, -0.8, 0.8);
  return limbs(
    f,
    [
      [[-1.6, -16], [-2.8, -3.2], [-11.4, -1.5]],
      [[1.6, -16], [8.6, -15.4], [10.2, -13.4], [10.4 + rest * 0.4, -1.5]],
    ],
    [
      [[-6.6, -33], [-9.4 - rest, -25.6], [-8.4 - rest, -19]],
      [[6.6, -33], [12, -36.6], [16.6, -41.6]],
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

/** A dress to the shins (main) with a white apron over its front and a pale tie at the waist bowed behind; about 20 wide at its hem, 18 below the hip. */
function apron(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const flare = between(rng, 9, 10);
  const hem = HEM - 4.5;
  return [
    shape(f.poly([[-7, S], [7, S], [6, -1], [flare, hem], [-flare, hem], [-6, -1]]), "blue-800", 1, MAIN_INKED),
    shape(f.poly([[-2.6, S], [2.6, S], [1.4, S + 2.4], [-1.4, S + 2.4]]), "slate-100", 0.9),
    shape(f.poly([[-3.2, S + 3.4], [3.2, S + 3.4], [3.6, -1.4], [5.6, 0], [6.8, hem - 2.2], [-6.8, hem - 2.2], [-5.6, 0], [-3.6, -1.4]]), "white", 1, INKED),
    shape(f.band([[-2.2, 2.4], [-3, hem - 3]], 0.4) + f.band([[2.6, 7.4], [3.4, hem - 3]], 0.4) + f.poly([[1, 1.8], [4, 1.8], [4, 4.8], [1, 4.8]]), "slate-400", 0.45),
    shape(f.band([[-6.4, -0.8], [6.4, -0.8]], 1.1), "slate-100", 1),
    shape(f.poly([[-6.4, -0.8], [-9.4, -2.8], [-9, 0]]) + f.poly([[-6.6, -0.4], [-8.4, 0.8], [-7.4, 3.2]]), "slate-100", 1, INKED),
  ];
}

/** A plain long robe (main) from the shoulders to the feet, wide bell sleeves hanging from its shoulders and a rope belt knotted with its ends hanging; about 19 wide at the hem and 23 across the sleeves. */
function plainRobe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const sway = between(rng, -0.6, 0.6);
  const sleeve = (side: number): string => f.poly([[side * 6.6, S + 0.2], [side * 9.4, S + 3], [side * (11.6 + sway), S + 10.4], [side * 7.6, S + 11.6], [side * 6.4, S + 5]]);
  return [
    shape(f.poly([[-7, S], [7, S], [7.8, 2], [9.4 + sway, HEM], [-9.4 + sway, HEM], [-7.8, 2]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-2.4, 3], [-3.4 + sway, HEM - 0.6]], 0.5) + f.band([[3, 4], [4.4 + sway, HEM - 0.6]], 0.5) + f.poly([[-2.2, S], [2.2, S], [0, S + 3.4]]), "slate-900", 0.3),
    shape(sleeve(-1) + sleeve(1), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-7.4, -1.2], [7.4, -1.4]], 0.9) + f.band([[1.6, -0.4], [1.2, 6 + sway]], 0.55) + f.band([[2.6, -0.4], [3.2, 5]], 0.55), "brass-500", 1),
    shape(f.circle(2, -1.3, 0.9), "brass-600", 1, INKED),
    shape(f.poly([[-6.8, S + 0.6], [-5.6, S + 0.6], [-6.6, HEM - 1], [-8.6, HEM - 1]]), "white", 0.12),
  ];
}

/** Plate armour (main): a ridged breastplate, a fauld of lames to 7 below the hip and huge round pauldrons, each about 12 wide, so it is about 27 across the shoulders. */
function plate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const lift = between(rng, -0.4, 0.4);
  const y = S + 0.8 + lift;
  const pauldron = (cx: number): string =>
    `M${f.p(cx - 6, y + 2)}Q${f.p(cx - 6.2, y - 4.6)} ${f.p(cx, y - 4.4)}Q${f.p(cx + 6.2, y - 4.6)} ${f.p(cx + 6, y + 2)}Q${f.p(cx, y + 3.6)} ${f.p(cx - 6, y + 2)}Z`;
  const lame = (cx: number, dy: number): string => f.band([[cx - 5.6, y + dy], [cx, y + dy + 1.4], [cx + 5.6, y + dy]], 0.4);
  return [
    shape(f.poly([[-6.2, -0.6], [6.2, -0.6], [7.8, 7], [-7.8, 7]]), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-6.8, 2.4], [6.8, 2.4]], 0.4) + f.band([[-7.3, 4.8], [7.3, 4.8]], 0.4), "slate-900", 0.35),
    shape(`M${f.p(-6.6, S + 0.6)}L${f.p(6.6, S + 0.6)}Q${f.p(8, -8)} ${f.p(6, 0)}L${f.p(-6, 0)}Q${f.p(-8, -8)} ${f.p(-6.6, S + 0.6)}Z`, "slate-400", 1, MAIN_INKED),
    shape(f.poly([[0.4, S + 0.8], [6.6, S + 0.8], [7.2, -8], [5.8, -0.4], [0.4, -0.4]]), "slate-900", 0.2),
    shape(f.band([[0, S + 1.4], [0, -1]], 0.7), "slate-100", 0.6),
    shape(f.band([[-6.4, -0.4], [6.4, -0.4]], 1.4), "wood-700", 1, INKED),
    shape(pauldron(-7.6) + pauldron(7.6), "slate-400", 1, MAIN_INKED),
    shape(lame(-7.6, 0.2) + lame(7.6, 0.2), "slate-900", 0.35),
    shape(f.ellipse(-9.6, y - 2.4, 2.2, 1) + f.ellipse(5.6, y - 2.4, 1.6, 0.8), "white", 0.35),
  ];
}

/** A tabard (main) over mail: a long front panel to 17 below the hip with a pale stripe down it, the grey mail showing at the sides and below, belted; about 16 wide. */
function tabard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const swing = between(rng, -0.3, 0.3);
  const hem = HEM - 5;
  const rows = [S + 4, S + 7, S + 10, S + 13, S + 16, 2, 4.4, 6.8].map((y) => f.band([[-7.6, y], [7.6, y]], 0.35)).join("");
  return [
    shape(f.poly([[-7, S], [7, S], [6.6, 0], [8, 8], [-8, 8], [-6.6, 0]]), "slate-400", 1, INKED),
    shape(rows, "slate-700", 0.5),
    shape(f.poly([[-6.4, S], [6.4, S], [4.6, S + 4.6], [4.6 + swing, hem], [-4.6 + swing, hem], [-4.6, S + 4.6]]), "red-600", 1, MAIN_INKED),
    shape(f.band([[0, S + 0.6], [swing, hem - 0.6]], 1.8), "slate-100", 0.9),
    shape(f.poly([[1.2, S + 4.6], [4.6, S + 4.6], [4.6 + swing, hem], [1.2 + swing, hem]]), "slate-900", 0.2),
    shape(f.band([[-7.2, -1], [7.2, -1]], 1.4), "wood-700", 1, INKED),
    shape(f.poly([[-1, -1.9], [1, -1.9], [1, -0.1], [-1, -0.1]]), "brass-600", 1),
  ];
}

/** A striped poncho (main) from the neck to the thighs, over the arms' tops, a pale and a brass stripe across it and a fringed hem; about 22 wide, 6 below the hip. */
function poncho(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const hem = 6 + between(rng, -0.6, 0.6);
  const halfAt = (y: number): number => (y < S + 4.4 ? 4.4 + ((y - S + 0.4) / 4.8) * 5.4 : 9.8 + (y - S - 4.4) / (hem - S - 4.4)) - 0.2;
  const stripe = (y: number, w: number): string => f.poly([[-halfAt(y - w), y - w], [halfAt(y - w), y - w], [halfAt(y + w), y + w], [-halfAt(y + w), y + w]]);
  const zig: Point[] = [];
  for (let k = 0; k <= 10; k += 1) zig.push([-halfAt(-4) + (k * 2 * halfAt(-4)) / 10, k % 2 === 0 ? -4.8 : -3.2]);
  const fringe: Point[] = [[10.6, hem]];
  for (let x = 10.2; x > -10.6; x -= 1.2) fringe.push([x, hem + 2], [x - 0.6, hem]);
  return [
    shape(f.poly([[-4.4, S - 0.4], [4.4, S - 0.4], [9.8, S + 4.4], [10.8, hem], [-10.8, hem], [-9.8, S + 4.4]]), "red-600", 1, MAIN_INKED),
    shape(f.poly(fringe), "red-600", 1, MAIN),
    shape(stripe(-9, 0.9) + stripe(2.4, 0.9), "brass-500", 1),
    shape(f.band(zig, 0.7), "slate-100", 0.9),
    shape(f.poly([[1, S + 1], [4.4, S - 0.2], [10.4, hem - 0.2], [2, hem - 0.2]]), "slate-900", 0.18),
  ];
}

/** A body wound in bandages (main, slate-100): diagonal wraps from the shoulders to 6 below the hip and two loose ends fluttering at its sides; about 14 wide, 22 with its ends. */
function wraps(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const flutter = between(rng, -1, 1);
  const rise = between(rng, 1.4, 2);
  const lines = (from: number, step: number, count: number, slope: number): string => {
    let d = "";
    for (let k = 0; k < count; k += 1) d += f.band([[-6, from + k * step + slope], [6, from + k * step - slope]], 0.45);
    return d;
  };
  return [
    shape(f.band([[5.8, -5], [8.6, -3.4 + flutter], [11.2, -4.6 + flutter]], 1.2) + f.band([[-6, 2.4], [-8.4, 5.6 - flutter], [-8.8, 9]], 1.1), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-6.6, S], [6.6, S], [6, 0], [6.4, 6], [-6.4, 6], [-6, 0]]), "slate-100", 1, MAIN_INKED),
    shape(lines(S + 2.4, 2.6, 8, rise), "stone-500", 0.6),
    shape(lines(S + 4, 7, 3, -rise * 0.8), "slate-400", 0.55),
    shape(f.poly([[3.4, S + 0.4], [6.6, S], [6, 0], [6.4, 6], [3.8, 6]]), "slate-900", 0.15),
  ];
}

/** A bare, hugely muscled torso (skin): broad shoulders about 22 across, pecs and abs, a tiny belted waist and shorts (main) to 6 below the hip. */
function muscle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const w = 9.4 + between(rng, 0, 0.8);
  let abs = f.band([[0, S + 7], [0, -2.6]], 0.45);
  for (const y of [-9, -6, -3.4]) abs += f.band([[-3, y + 0.3], [-0.5, y]], 0.4) + f.band([[0.5, y], [3, y + 0.3]], 0.4);
  return [
    shape(f.poly([[-4.8, -1.8], [4.8, -1.8], [7.2, 6], [1, 6], [0, 3.2], [-1, 6], [-7.2, 6]]), "red-600", 1, MAIN_INKED),
    shape(
      `M${f.p(-w, S + 1.2)}Q${f.p(-w, S - 1)} ${f.p(-7, S - 1)}L${f.p(7, S - 1)}Q${f.p(w, S - 1)} ${f.p(w, S + 1.2)}L${f.p(w - 1.2, S + 6)}Q${f.p(5.8, -6)} ${f.p(4.4, -1.8)}L${f.p(-4.4, -1.8)}Q${f.p(-5.8, -6)} ${f.p(1.2 - w, S + 6)}Z`,
      "red-300",
      1,
      SKIN_INKED,
    ),
    shape(f.ellipse(-8, S + 1.6, 2.6, 2.8) + f.ellipse(8, S + 1.6, 2.6, 2.8), "red-300", 1, SKIN_INKED),
    shape(f.band([[-7, S + 4], [-4.2, S + 6.6], [-0.4, S + 6]], 0.5) + f.band([[0.4, S + 6], [4.2, S + 6.6], [7, S + 4]], 0.5) + abs, "slate-900", 0.32),
    shape(f.ellipse(-3.8, S + 3.8, 2.4, 1.3) + f.ellipse(3.6, S + 3.8, 1.6, 0.9) + f.ellipse(-8.6, S + 0.6, 1.2, 0.9), "white", 0.25),
    shape(f.poly([[4.6, S + 2], [w, S + 1.2], [w - 1.2, S + 6], [5.8, -6], [4.4, -1.8], [2.6, -1.8]]), "slate-900", 0.16),
    shape(f.band([[-4.8, -2], [4.8, -2]], 1.4), "wood-800", 1, INKED),
    shape(f.poly([[-0.9, -2.8], [0.9, -2.8], [0.9, -1.2], [-0.9, -1.2]]), "brass-500", 1),
  ];
}

/** The chest a great war drum (main): a barrel shell from the chest to 5 below the hip, slung from the shoulders, zigzag lacing between its hoops and a pale drumhead on top; about 19 wide. */
function drumChest(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const top = S + 5.6;
  const bottom = 3.4;
  const w = 8.6 + between(rng, 0, 0.6);
  const sag = (x: number): number => 1.3 * (1 - (x / w) ** 2);
  const lace: Point[] = [];
  for (let k = 0; k <= 8; k += 1) {
    const x = -w + 0.6 + (k * (2 * w - 1.2)) / 8;
    lace.push([x, k % 2 === 0 ? top + 0.4 + 2.6 * Math.sqrt(Math.max(0, 1 - (x / w) ** 2)) + 0.6 : bottom + sag(x) - 0.9]);
  }
  return [
    shape(f.poly([[-7, S], [7, S], [7, top + 1], [-7, top + 1]]), "slate-800", 1, INKED),
    shape(f.band([[-5.6, S + 0.2], [-2.6, top]], 1) + f.band([[5.6, S + 0.2], [2.6, top]], 1), "wood-800", 1),
    shape(`M${f.p(-w, top)}L${f.p(w, top)}Q${f.p(w + 1.2, (top + bottom) / 2)} ${f.p(w - 0.4, bottom)}Q${f.p(0, bottom + 2.6)} ${f.p(0.4 - w, bottom)}Q${f.p(-w - 1.2, (top + bottom) / 2)} ${f.p(-w, top)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.poly([[w * 0.35, top + 2.4], [w, top], [w + 0.8, (top + bottom) / 2], [w - 0.4, bottom], [w * 0.4, bottom + 1.1]]), "slate-900", 0.25),
    shape(f.band([[0.4 - w, bottom], [0, bottom + 1.3], [w - 0.4, bottom]], 0.9), "wood-800", 1),
    shape(f.band(lace, 0.45), "slate-100", 0.9),
    shape(f.ellipse(0, top, w, 2.6) + f.ellipse(0, top, w - 1, 1.9), "wood-800", 1, INKED),
    shape(f.ellipse(0, top, w - 1, 1.9), "slate-100", 1),
    shape(f.ellipse(-2.6, top - 0.5, 3, 0.8), "white", 0.6),
  ];
}

/** A cardigan (main) open over a white shirt with collar points, three buttons down its edge, two pockets and a ribbed hem about 4 below the hip; about 15 wide. */
function cardigan(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const hem = between(rng, 3.4, 4.4);
  const open = (y: number): number => 2.8 - ((y - S) / (hem - S)) * 1;
  return [
    shape(f.poly([[-7.2, S], [7.2, S], [7.1, hem], [-7.1, hem]]), "green-700", 1, MAIN_INKED),
    shape(f.poly([[-2.8, S], [2.8, S], [1.8, hem], [-1.8, hem]]), "slate-100", 1),
    shape(f.poly([[-2.8, S], [-0.2, S + 0.4], [-1.9, S + 2.8]]) + f.poly([[0.2, S + 0.4], [2.8, S], [1.9, S + 2.8]]), "white", 1, INKED),
    shape([-12, -7, -2].map((y) => f.circle(-open(y) - 0.9, y, 0.5)).join(""), "brass-600", 1, INKED),
    shape(f.poly([[-6.2, -3.4], [-3.8, -3.4], [-3.8, -0.6], [-6.2, -0.6]]) + f.poly([[3.8, -3.4], [6.2, -3.4], [6.2, -0.6], [3.8, -0.6]]) + f.poly([[-7.1, hem - 1.4], [-open(hem), hem - 1.4], [-open(hem), hem], [-7.1, hem]]) + f.poly([[open(hem), hem - 1.4], [7.1, hem - 1.4], [7.1, hem], [open(hem), hem]]), "slate-900", 0.3),
    shape(f.poly([[4, S], [7.2, S], [7.1, hem], [4.6, hem]]), "slate-900", 0.15),
  ];
}

/** Leaf camouflage (main green): a body of overlapping leaves from the shoulders to a ragged leafy hem about 8 below the hip, leaves jutting at its shoulders; about 16 wide. */
function leafSuit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const leaf = (cx: number, cy: number, length: number, width: number, degrees: number): string => {
    const [tx, ty] = turned([0, length], degrees + between(rng, -12, 12));
    const [nx, ny] = turned([width, 0], degrees);
    const [mx, my] = [cx + tx / 2, cy + ty / 2];
    return `M${f.p(cx, cy)}Q${f.p(mx + nx, my + ny)} ${f.p(cx + tx, cy + ty)}Q${f.p(mx - nx, my - ny)} ${f.p(cx, cy)}Z`;
  };
  return [
    shape(f.poly([[-6.8, S], [6.8, S], [7.4, 3], [-7.4, 3]]), "green-700", 1, MAIN_INKED),
    shape([-6, -3, 0, 3, 6].map((x) => leaf(x, 2.4, 5.2, 1.6, 0)).join("") + leaf(-6.4, S + 1.2, 4.4, 1.5, 70) + leaf(6.4, S + 1.2, 4.4, 1.5, -70), "green-700", 1, MAIN_INKED),
    shape(leaf(-4, S + 2.6, 4.2, 2.2, 10) + leaf(0.6, S + 3.4, 4.2, 2.2, -8) + leaf(4.8, S + 2.4, 4.2, 2.2, -14) + leaf(-2.2, -7.6, 4.2, 2.2, 6) + leaf(2.6, -8, 4.2, 2.2, -10) + leaf(-4.6, -2.4, 4, 2.2, 14) + leaf(0.4, -1.8, 4, 2.2, 0) + leaf(4.8, -2.6, 4, 2.2, -16), "green-900", 1),
    shape(leaf(-1.8, S + 0.4, 3.8, 2, 4) + leaf(2.8, S + 0.2, 3.8, 2, -6) + leaf(-4.8, -11, 3.8, 2, 16) + leaf(0.2, -11.6, 3.8, 2, 0) + leaf(4.6, -11, 3.8, 2, -12) + leaf(-2.4, -4.8, 3.6, 2, 8) + leaf(2.8, -5, 3.6, 2, -8), "green-500", 1),
    shape(f.poly([[3.6, S], [6.8, S], [7.4, 3], [4.4, 3]]), "slate-900", 0.15),
  ];
}

/** A baggy patched costume: a loose tunic (main) to 7 below the hip with three square patches in red, blue and green, big stitches across their edges and a rope belt; about 20 wide. */
function patched(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const square = (cx: number, cy: number, side: number, degrees: number): Point[] =>
    [[-side / 2, -side / 2], [side / 2, -side / 2], [side / 2, side / 2], [-side / 2, side / 2]].map(([x, y]) => {
      const [dx, dy] = turned([x ?? 0, y ?? 0], degrees);
      return [cx + dx, cy + dy] as const;
    });
  const patches: readonly (readonly [number, number, number, number])[] = [
    [-3.4, -11.4, 3.8, between(rng, -12, 12)],
    [3.4, -5.4, 3.4, between(rng, -12, 12)],
    [-2.4, 3, 3.2, between(rng, -12, 12)],
  ];
  const stitches = patches
    .flatMap(([cx, cy, side, degrees]) => {
      const corners = square(cx, cy, side, degrees);
      return [0, 1, 2].map((k) => {
        const [ax, ay] = corners[k] ?? [0, 0];
        const [bx, by] = corners[k + 1] ?? [0, 0];
        const [mx, my] = [(ax + bx) / 2, (ay + by) / 2];
        const [ox, oy] = [(mx - cx) * 0.36, (my - cy) * 0.36];
        return f.band([[mx - ox, my - oy], [mx + ox, my + oy]], 0.45);
      });
    })
    .join("");
  const paints: readonly PaletteName[] = ["red-600", "blue-700", "green-700"];
  return [
    shape(`M${f.p(-7.2, S)}L${f.p(7.2, S)}Q${f.p(10.4, -6)} ${f.p(9.2, 7)}L${f.p(-9.2, 7)}Q${f.p(-10.4, -6)} ${f.p(-7.2, S)}Z`, "stone-500", 1, MAIN_INKED),
    ...patches.map(([cx, cy, side, degrees], k) => shape(f.poly(square(cx, cy, side, degrees)), paints[k] ?? "red-600", 1, INKED)),
    shape(stitches, "slate-900", 0.85),
    shape(f.band([[-8.6, -1.2], [8.6, -1.2]], 1.1), "wood-700", 1),
    shape(`M${f.p(4, S)}L${f.p(7.2, S)}Q${f.p(10.4, -6)} ${f.p(9.2, 7)}L${f.p(5.4, 7)}Z`, "slate-900", 0.16),
  ];
}

/** A blocky stone or crystal body (skin, so the look's skin is its stone): a broad cracked slab chest, a shoulder block each side (about 24 across them) and a hip block to 5 below the hip. */
function stoneBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const j = (): number => between(rng, -0.5, 0.5);
  const block = (side: number): string => f.poly([[side * 12, S - 1 + j()], [side * 6.2, S - 1.8], [side * 5.6, S + 4.6], [side * 11.4, S + 5 + j()]]);
  return [
    shape(f.poly([[-7.6, S + 0.8 + j()], [7.6, S + 0.4 + j()], [6.6, -1.4], [-6.8, -1]]), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-5.6, -1.6], [5.4, -1.8], [6, 5], [-6.2, 5.2]]), "stone-500", 1, SKIN_INKED),
    shape(block(-1) + block(1), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-7.4, S + 1], [-2.6, S + 1], [-3.4, S + 6], [-6.8, S + 5.6]]) + f.poly([[-11.6, S - 0.8], [-6.6, S - 1.4], [-6.8, S + 1.2], [-11.2, S + 1.4]]), "white", 0.22),
    shape(f.poly([[3, -5], [6.9, -5.4], [6.6, -1.4], [2.4, -1.2]]) + f.poly([[2.4, 1.4], [5.6, 1.2], [6, 5], [2.8, 5.1]]) + f.poly([[7.6, S + 2.6], [11.6, S + 2.8], [11.4, S + 5], [8, S + 4.8]]), "slate-900", 0.25),
    shape(
      f.band([[-1, S + 0.8], [0.6, S + 4], [-0.6, S + 7], [1.2, S + 9.4]], 0.45) + f.band([[4.6, S + 6], [2.6, S + 9], [3.4, S + 11]], 0.4) + f.band([[-9.4, S - 1.2], [-8.6, S + 1.6], [-9.6, S + 3.4]], 0.4) + f.band([[-3, 0], [-2, 2.6]], 0.4),
      "slate-900",
      0.6,
    ),
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

/** A bat's face (skin): two big pointed ears about 6 above the crown, a dark brow, beady red eyes, a pug nose and two small fangs. */
function headBat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tilt = between(rng, -0.8, 0.8);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "slate-700", 1, SKIN),
    shape(f.poly([[-4.4, c - 0.6], [-6.2 - tilt, c - 10.4], [-1.4, c - 3.8]]) + f.poly([[1.4, c - 3.8], [6.2 + tilt, c - 10.4], [4.4, c - 0.6]]), "slate-700", 1, SKIN_INKED),
    shape(f.poly([[-4, c - 2.2], [-5.4 - tilt, c - 8.6], [-2.4, c - 4]]) + f.poly([[2.4, c - 4], [5.4 + tilt, c - 8.6], [4, c - 2.2]]), "red-400", 0.45),
    shape(f.circle(0, c, HEAD_R), "slate-700", 1, SKIN_INKED),
    shape(f.poly([[-3, c - 4], [0, c - 1.8], [3, c - 4], [0, c - 4.8]]), "slate-900", 0.3),
    shape(f.circle(-1.8, c - 0.4, 0.75) + f.circle(1.8, c - 0.4, 0.75), "red-500", 1),
    shape(f.circle(-1.6, c - 0.6, 0.25) + f.circle(2, c - 0.6, 0.25), "white", 0.8),
    shape(f.ellipse(0, c + 1.2, 1.1, 0.7), "slate-900", 0.7),
    shape(f.band([[-1.8, c + 2.6], [0, c + 3], [1.8, c + 2.6]], 0.4), "slate-900", 0.8),
    shape(f.poly([[-1.4, c + 2.7], [-0.6, c + 2.9], [-1, c + 4]]) + f.poly([[0.6, c + 2.9], [1.4, c + 2.7], [1, c + 4]]), "white", 1),
  ];
}

/** A skull (skin, bone): a round cranium over a narrower jaw, dark eye sockets and nose, a row of teeth and a hairline crack. */
function headSkull(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const crack = between(rng, -0.6, 0.6);
  const teeth = [-1.6, -0.5, 0.6, 1.7].map((x) => f.band([[x, c + 2.6], [x, c + 4.2]], 0.28)).join("");
  return [
    shape(f.band([[0, 1.5], [0, -1]], 1.4), "slate-100", 1, SKIN_INKED),
    shape(f.poly([[-3, c + 2], [3, c + 2], [2.6, c + 5.2], [-2.6, c + 5.2]]), "slate-100", 1, SKIN_INKED),
    shape(`M${f.p(-3.2, c + 3)}Q${f.p(-5.2, c + 1.8)} ${f.p(-4.8, c - 1)}C${f.p(-4.8, c - 6.6)} ${f.p(4.8, c - 6.6)} ${f.p(4.8, c - 1)}Q${f.p(5.2, c + 1.8)} ${f.p(3.2, c + 3)}Z`, "slate-100", 1, SKIN_INKED),
    shape(f.ellipse(-1.8, c - 0.4, 1.3, 1.5) + f.ellipse(1.8, c - 0.4, 1.3, 1.5), "slate-900", 0.95),
    shape(f.poly([[0, c + 1.1], [0.7, c + 2.3], [-0.7, c + 2.3]]), "slate-900", 0.9),
    shape(f.band([[-2.6, c + 3.4], [2.6, c + 3.4]], 0.35), "slate-900", 0.7),
    shape(teeth, "slate-900", 0.5),
    shape(f.band([[1.2 + crack, c - 5], [2.2, c - 3.8], [1.6, c - 2.8]], 0.3), "slate-900", 0.5),
  ];
}

/** A round face (skin) under a broad mushroom cap about 17 wide, red-600 with white spots (the cap is never skin), its pale gills showing below. */
function headMushroom(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tip = between(rng, -0.8, 0.8);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "red-300", 1, SKIN),
    shape(f.circle(0, c + 0.8, HEAD_R * 0.86), "red-300", 1, SKIN_INKED),
    shape(f.ellipse(0, c - 1.4, 7.4, 1.4), "slate-100", 1, INKED),
    shape(`M${f.p(-8.4, c - 0.8)}C${f.p(-8.2 + tip, c - 10.4)} ${f.p(8.2 + tip, c - 10.4)} ${f.p(8.4, c - 0.8)}Q${f.p(0, c - 3)} ${f.p(-8.4, c - 0.8)}Z`, "red-600", 1, INKED),
    shape(f.circle(-4.6, c - 3.6, 1.2) + f.circle(-0.4 + tip / 2, c - 6.2, 1.4) + f.circle(3.8, c - 4, 1.1) + f.circle(6, c - 2.4, 0.6), "white", 0.95),
    shape(f.circle(-1.6, c + 1.2, 0.45) + f.circle(1.6, c + 1.2, 0.45), "slate-900", 1),
    shape(f.band([[-1, c + 2.8], [0, c + 3.3], [1, c + 2.8]], 0.35), "slate-900", 0.6),
    shape(f.circle(-2.8, c + 2.4, 0.8) + f.circle(2.8, c + 2.4, 0.8), "red-400", 0.35),
  ];
}

/** A round smiling face (skin) ringed by short triangular rays, orange-400 and ember-500 by turns, none below its chin. */
function headSun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const turn = between(rng, 0, 15);
  const smile = between(rng, 0.5, 0.9);
  const point = (degrees: number, r: number): Point => {
    const [x, y] = turned([0, -r], degrees);
    return [x, c + y];
  };
  let long = "";
  let short = "";
  for (let k = 0; k < 12; k += 1) {
    const a = turn + k * 30;
    if (a > 140 && a < 220) continue;
    const ray = f.poly([point(a - 13, HEAD_R - 0.4), point(a, HEAD_R + (k % 2 === 0 ? 2.6 : 1.7)), point(a + 13, HEAD_R - 0.4)]);
    if (k % 2 === 0) long += ray;
    else short += ray;
  }
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "brass-500", 1, SKIN),
    shape(long, "orange-400", 1, INKED),
    shape(short, "ember-500", 1, INKED),
    shape(f.circle(0, c, HEAD_R), "brass-500", 1, SKIN_INKED),
    shape(f.circle(-1.7, c - 0.6, 0.5) + f.circle(1.7, c - 0.6, 0.5), "wood-950", 1),
    shape(`M${f.p(-2, c + 1.4)}Q${f.p(0, c + 1.4 + smile * 2.6)} ${f.p(2, c + 1.4)}Q${f.p(0, c + 1.4 + smile)} ${f.p(-2, c + 1.4)}Z`, "wood-950", 1),
    shape(f.circle(-3, c + 1, 0.8) + f.circle(3, c + 1, 0.8), "red-400", 0.4),
  ];
}

/** A blocky carved stone head (skin) on a neck block: a heavy brow over glowing cyan slit eyes, a carved mouth, cracks, a lit and a shaded face. */
function headGolem(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const j = (): number => between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-2.6, 0.8], [2.6, 0.8], [2.2, -1.6], [-2.2, -1.6]]), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-4.8, c - 4.2 + j()], [4.6, c - 4.8 + j()], [5, c + 2.8], [3.4, c + 4.8], [-3.6, c + 4.8], [-5, c + 3]]), "stone-500", 1, SKIN_INKED),
    shape(f.poly([[-4.6, c - 4], [0, c - 4.4], [-0.4, c - 2.6], [-4.4, c - 2.4]]), "white", 0.22),
    shape(f.poly([[2.6, c - 4.6], [4.6, c - 4.8], [5, c + 2.8], [3.4, c + 4.8], [2.4, c + 4.8]]), "slate-900", 0.2),
    shape(f.poly([[-4.6, c - 1.4], [4.7, c - 1.6], [4.7, c + 0.6], [-4.6, c + 0.6]]), "slate-900", 0.4),
    shape(f.ellipse(-1.9, c - 0.4, 1.8, 0.9) + f.ellipse(1.9, c - 0.4, 1.8, 0.9), "cyan-400", 0.3),
    shape(f.poly([[-3, c - 0.7], [-0.8, c - 0.2], [-0.8, c + 0.3], [-3, c - 0.1]]) + f.poly([[0.8, c - 0.2], [3, c - 0.7], [3, c - 0.1], [0.8, c + 0.3]]), "cyan-400", 1),
    shape(f.poly([[-2.4, c + 2.6], [2.4, c + 2.6], [2.4, c + 3.2], [-2.4, c + 3.2]]), "slate-900", 0.5),
    shape(f.band([[-3.4, c - 4.2], [-2.6, c - 2.6], [-3, c - 1.6]], 0.3) + f.band([[3.6, c + 1.2], [2.6, c + 2.2]], 0.3), "slate-900", 0.55),
  ];
}

/** A smooth porcelain oval head (skin) on a slim metal neck: two round brass-rimmed lenses, a seam down the brow and round the jaw, a sheen. */
function headAutomaton(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const gaze = between(rng, -0.3, 0.3);
  return [
    shape(f.band([[0, 1.5], [0, -1.8]], 2), "slate-700", 1, INKED),
    shape(f.band([[-1.4, -0.3], [1.4, -0.3]], 0.6), "brass-500", 1),
    shape(f.ellipse(0, c - 0.4, 4.2, 5.1), "slate-100", 1, SKIN_INKED),
    shape(f.band([[0, c - 5.4], [0, c - 2.4]], 0.3) + f.band([[-4, c + 1.8], [-1.6, c + 3.2], [1.6, c + 3.2], [4, c + 1.8]], 0.3), "slate-900", 0.4),
    shape(f.circle(-1.7, c - 0.6, 1.3) + f.circle(1.7, c - 0.6, 1.3), "brass-500", 1, INKED),
    shape(f.circle(-1.7 + gaze, c - 0.6, 0.8) + f.circle(1.7 + gaze, c - 0.6, 0.8), "cyan-400", 1),
    shape(f.circle(-2 + gaze, c - 0.9, 0.25) + f.circle(1.4 + gaze, c - 0.9, 0.25), "white", 0.85),
    shape(f.ellipse(-1.8, c - 3.4, 1.4, 0.8), "white", 0.5),
  ];
}

/** A goblin's face (skin, green-700 by default): long pointed ears out to its sides, scowling brows, orange eyes, a big hooked nose and a crooked tooth. */
function headGoblin(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const droop = between(rng, -1, 1);
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "green-700", 1, SKIN),
    shape(f.poly([[-3.6, c - 1.8], [-10, c - 4.2 + droop], [-4.2, c + 1]]) + f.poly([[3.6, c - 1.8], [10, c - 4.2 + droop], [4.2, c + 1]]), "green-700", 1, SKIN_INKED),
    shape(f.poly([[-4.4, c - 1.2], [-8.6, c - 3.6 + droop], [-4.6, c + 0.2]]) + f.poly([[4.4, c - 1.2], [8.6, c - 3.6 + droop], [4.6, c + 0.2]]), "slate-900", 0.3),
    shape(f.circle(0, c, HEAD_R - 0.2), "green-700", 1, SKIN_INKED),
    shape(f.band([[-3.2, c - 2.6], [-0.9, c - 1.6]], 0.55) + f.band([[0.9, c - 1.6], [3.2, c - 2.6]], 0.55), "slate-900", 0.75),
    shape(f.ellipse(-1.9, c - 0.5, 0.9, 0.6) + f.ellipse(1.9, c - 0.5, 0.9, 0.6), "orange-400", 1),
    shape(f.circle(-1.7, c - 0.5, 0.3) + f.circle(2.1, c - 0.5, 0.3), "slate-900", 1),
    shape(f.band([[-2.8, c + 2.6], [-0.8, c + 3.4], [2.2, c + 3.4], [3.2, c + 2.4]], 0.45), "slate-900", 0.75),
    shape(f.poly([[1, c + 3.4], [1.9, c + 3.3], [1.5, c + 4.4]]), "white", 1),
    shape(`M${f.p(-0.4, c - 0.8)}Q${f.p(2.6, c + 1.6)} ${f.p(1.6, c + 2.6)}Q${f.p(0.4, c + 3)} ${f.p(-0.8, c + 2.2)}Q${f.p(-0.6, c + 0.6)} ${f.p(-0.4, c - 0.8)}Z`, "green-700", 1, SKIN_INKED),
    shape(`M${f.p(0.6, c + 0.4)}Q${f.p(2.6, c + 1.6)} ${f.p(1.6, c + 2.6)}Q${f.p(1.6, c + 1.4)} ${f.p(0.6, c + 0.4)}Z`, "slate-900", 0.22),
  ];
}

/** A head wound in bandage strips (skin, the wraps), crossing two ways, one eye glinting from a gap and a loose end hanging behind. */
function headMummy(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const slope = between(rng, 0.4, 0.9);
  const flutter = between(rng, -0.8, 0.8);
  const half = (y: number): number => Math.sqrt(Math.max(0, HEAD_R ** 2 - (y - c) ** 2)) - 0.4;
  const strips = (from: number, count: number, step: number, tilt: number): string => {
    let d = "";
    for (let k = 0; k < count; k += 1) {
      const y = from + k * step;
      d += f.band([[-half(y + tilt), y + tilt], [half(y - tilt), y - tilt]], 0.4);
    }
    return d;
  };
  return [
    shape(f.band([[0, 1.5], [0, -1.5]], 2.6), "slate-100", 1, SKIN),
    shape(f.band([[-4, c + 1.6], [-6.4, c + 3.4 + flutter], [-7, c + 5.6]], 1.1), "slate-100", 1, SKIN_INKED),
    shape(f.circle(0, c, HEAD_R), "slate-100", 1, SKIN_INKED),
    shape(strips(c - 3.6, 5, 1.8, slope), "stone-500", 0.6),
    shape(strips(c - 2.6, 3, 2.6, -slope * 0.8), "slate-400", 0.55),
    shape(f.ellipse(1.7, c - 0.3, 1.6, 0.8), "slate-900", 0.9),
    shape(f.circle(1.9, c - 0.3, 0.45), "red-400", 1),
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

/** A headscarf (red-600, never the look's cloth, which paints the outfit) over the crown to the brow, white-spotted, knotted at the back with two tails flying. */
function kerchief(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flutter = between(rng, -0.8, 0.8);
  return [
    shape(f.poly([[-5, c - 0.2], [-8.6, c + 2.6 + flutter], [-7.4, c + 3.6 + flutter], [-4.8, c + 0.8]]), "red-600", 1, INKED),
    shape(f.poly([[-5.4, c - 1], [-9, c + 0.2 - flutter], [-8.6, c + 1.6 - flutter], [-5.2, c - 0.2]]), "red-600", 1, INKED),
    shape(`M${f.p(-5.3, c + 1)}C${f.p(-5.8, c - 7)} ${f.p(5.8, c - 7)} ${f.p(5.3, c + 0.2)}Q${f.p(0, c - 4.2)} ${f.p(-5.3, c + 1)}Z`, "red-600", 1, INKED),
    shape(f.circle(-2.6, c - 3.4, 0.5) + f.circle(0.8, c - 4.3, 0.5) + f.circle(3.2, c - 2.8, 0.45), "white", 0.8),
    shape(f.band([[-4.6, c + 0.2], [0, c - 1.6], [4.6, c - 0.4]], 0.5), "slate-900", 0.25),
    shape(f.circle(-5.4, c - 0.4, 1.2), "red-600", 1, INKED),
  ];
}

/** A wide-brimmed explorer's hat (wood-600) about 18 across its drooping brim, a dented crown and a dark band. */
function hatBrim(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const dent = between(rng, 0, 0.8);
  const tilt = between(rng, -0.4, 0.4);
  return [
    shape(`M${f.p(-8.8, c - 2.4 + tilt)}Q${f.p(-8, c - 5)} ${f.p(0, c - 5)}Q${f.p(8, c - 5)} ${f.p(8.8, c - 2.4 - tilt)}Q${f.p(0, c - 2.2)} ${f.p(-8.8, c - 2.4 + tilt)}Z`, "wood-600", 1, INKED),
    shape(`M${f.p(-4, c - 3.6)}L${f.p(-3.6, c - 8)}Q${f.p(0, c - 9.6 + dent)} ${f.p(3.6, c - 8)}L${f.p(4, c - 3.6)}Q${f.p(0, c - 2.8)} ${f.p(-4, c - 3.6)}Z`, "wood-600", 1, INKED),
    shape(f.poly([[-3.9, c - 3.8], [3.9, c - 3.8], [3.75, c - 5.4], [-3.75, c - 5.4]]), "wood-900", 1),
    shape(f.band([[0, c - 8.8 + dent], [0.2, c - 6.6]], 0.4), "slate-900", 0.3),
    shape(f.ellipse(-5.6, c - 3.4, 2.4, 0.5), "white", 0.15),
  ];
}

/** Brass-rimmed goggles pushed up on the forehead on a leather strap, over short dark hair with a tuft sticking up between the lenses. */
function goggles(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const part = between(rng, -0.6, 0.6);
  const glint = between(rng, -0.3, 0.3);
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.6)} ${f.p(5.4, c - 6.6)} ${f.p(5, c + 0.5)}Q${f.p(2, c - 2.6)} ${f.p(0, c - 2.4)}Q${f.p(-2.4, c - 2.2)} ${f.p(-5, c + 0.5)}Z`, "wood-800", 1, INKED),
    shape(f.poly([[part - 1.2, c - 4.6], [part - 0.6, c - 7], [part + 0.2, c - 5], [part + 1, c - 6.8], [part + 1.2, c - 4.6]]), "wood-800", 1, INKED),
    shape(f.band([[-5.2, c - 1.4], [-3.6, c - 3], [3.6, c - 3], [5.2, c - 1.4]], 0.9), "wood-600", 1, INKED),
    shape(f.circle(-2, c - 3.4, 1.7) + f.circle(2, c - 3.4, 1.7), "brass-500", 1, INKED),
    shape(f.circle(-2, c - 3.4, 1.05) + f.circle(2, c - 3.4, 1.05), "sky-300", 1),
    shape(f.ellipse(-2.4 + glint, c - 3.8, 0.4, 0.3) + f.ellipse(1.6 + glint, c - 3.8, 0.4, 0.3), "white", 0.85),
  ];
}

/** A knight's round steel helm (slate-400) over the whole head: a dark visor slit at the eyes split by a nose guard, breathing holes, a lit dome and a shaded side. */
function helm(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const glint = between(rng, -0.4, 0.4);
  return [
    shape(`M${f.p(-5.3, c + 3.4)}L${f.p(-5.6, c - 1)}C${f.p(-5.6, c - 7.8)} ${f.p(5.6, c - 7.8)} ${f.p(5.6, c - 1)}L${f.p(5.3, c + 3.4)}Q${f.p(0, c + 4.8)} ${f.p(-5.3, c + 3.4)}Z`, "slate-400", 1, INKED),
    shape(f.poly([[3, c - 5.4], [5.6, c - 1], [5.3, c + 3.4], [3.6, c + 3.9], [4.2, c - 1]]), "slate-900", 0.2),
    shape(f.poly([[-4.6, c - 1.4], [4.8, c - 1.4], [4.6, c - 0.2], [-4.4, c - 0.2]]), "slate-900", 1),
    shape(f.band([[0.4, c - 5.6], [0.4, c + 3.6]], 1.3), "slate-400", 1, INKED),
    shape(f.circle(2.4, c + 1.6, 0.3) + f.circle(3.4, c + 1.2, 0.3) + f.circle(2.6, c + 2.7, 0.3) + f.circle(3.6, c + 2.3, 0.3), "slate-900", 0.8),
    shape(f.band([[-4.2, c - 1.8], [-3.6 + glint, c - 4.4], [-1.6, c - 5.6]], 0.7), "white", 0.4),
  ];
}

/** A monk's tonsure: a ring of short brown hair round the head at the temples, the bald crown (the head's own skin) shining above it. */
function tonsure(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const shine = between(rng, -0.6, 0.6);
  const ring = `M${f.p(-5.2, c + 1.2)}Q${f.p(-5.8, c - 1.6)} ${f.p(-4.4, c - 2.4)}Q${f.p(0, c - 3.4)} ${f.p(4.4, c - 2.4)}Q${f.p(5.8, c - 1.6)} ${f.p(5.2, c + 1.2)}L${f.p(4.2, c + 1)}Q${f.p(4.4, c - 0.8)} ${f.p(3.4, c - 1.2)}Q${f.p(0, c - 2)} ${f.p(-3.4, c - 1.2)}Q${f.p(-4.4, c - 0.8)} ${f.p(-4.2, c + 1)}Z`;
  return [
    shape(ring, "wood-700", 1, INKED),
    shape(f.band([[-3, c - 2.4], [-2.7, c - 1.8]], 0.3) + f.band([[-0.4, c - 2.8], [-0.2, c - 2.1]], 0.3) + f.band([[2.4, c - 2.6], [2.6, c - 1.9]], 0.3), "slate-900", 0.3),
    shape(f.ellipse(-1.2 + shine, c - 3.8, 1.5, 0.7), "white", 0.45),
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

/** A big wooden mallet: a barrel head about 9 wide and 5 deep on a handle 4 below and 12 above the hand, leant a little forward. */
function mallet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 6, 16);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const head = `M${f.p(...T([-4, -17.4]))}L${f.p(...T([4, -17.4]))}Q${f.p(...T([5, -14.8]))} ${f.p(...T([4, -12.2]))}L${f.p(...T([-4, -12.2]))}Q${f.p(...T([-5, -14.8]))} ${f.p(...T([-4, -17.4]))}Z`;
  return [
    shape(f.band([T([0, 4]), T([0, -12.4])], 1.1), "wood-700", 1, INKED),
    shape(head, "wood-600", 1, INKED),
    shape(poly([[-3, -17.4], [-2.2, -17.4], [-2.2, -12.2], [-3, -12.2]]) + poly([[2.2, -17.4], [3, -17.4], [3, -12.2], [2.2, -12.2]]), "wood-800", 0.8),
    shape(poly([[-3.6, -16.8], [3.6, -16.8], [3.6, -16], [-3.6, -16]]), "white", 0.2),
    shape(f.circle(...T([0, 4.4]), 0.8), "wood-800", 1),
  ];
}

/** A short dagger held point-forward and a little up, its blade dripping green-400 poison. */
function dagger(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -34, -18);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const drop = (x: number, length: number): string => {
    const [dx, dy] = T([x, 0.5]);
    return f.band([[dx, dy], [dx, dy + length]], 0.45) + f.circle(dx, dy + length + 0.4, 0.55);
  };
  const [fx, fy] = T([4.6, 0.5]);
  return [
    shape(f.band([T([-1.6, 0]), T([1.2, 0])], 1.3), "wood-800", 1, INKED),
    shape(f.circle(...T([-2, 0]), 0.7), "brass-600", 1),
    shape(poly([[1.2, -1.8], [2, -1.8], [2, 1.8], [1.2, 1.8]]), "brass-600", 1, INKED),
    shape(poly([[2, -0.8], [6.6, -0.6], [9, 0], [6.6, 0.7], [2, 0.8]]), "slate-100", 1, INKED),
    shape(poly([[5, -0.55], [6.6, -0.5], [9, 0], [6.6, 0.6], [5, 0.65]]), "green-400", 0.85),
    shape(drop(5.8, between(rng, 1, 2.2)) + drop(7.6, between(rng, 0.4, 1.2)) + f.circle(fx, fy + 4.6, 0.4), "green-400", 1),
  ];
}

/** A long wooden spoon raised in the fist, its bowl about 6 deep at the top of a handle 13 above the hand. */
function spoon(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 6);
  const T = (p: Point): Point => turned(p, tilt);
  const [bx, by] = T([0, -15.4]);
  const [ix, iy] = T([0.3, -15.6]);
  return [
    shape(f.band([T([0, 3.4]), T([0, -12.8])], 0.9), "wood-600", 1, INKED),
    shape(oval(f, bx, by, 3, 2.2, tilt - 90), "wood-600", 1, INKED),
    shape(oval(f, ix, iy, 2.2, 1.4, tilt - 90), "wood-800", 0.45),
    shape(oval(f, ...T([-0.9, -16.4]), 0.9, 0.4, tilt - 90), "white", 0.3),
  ];
}

/** A tall pole rising 26 above the hand, a red-600 flag about 10 wide snapping out from its top, a white paw on it and a brass finial. */
function banner(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const snap = between(rng, -1, 1);
  const lean = between(rng, -0.04, 0.04);
  const L = (p: Point): Point => leant(p, lean);
  const top = -26;
  const flag = [
    `M${f.p(...L([0.4, top + 0.8]))}Q${f.p(...L([5.2, top - 0.6 + snap]))} ${f.p(...L([10.6, top + 1.2]))}L${f.p(...L([8.6, top + 4.6]))}`,
    `L${f.p(...L([10.4, top + 8]))}Q${f.p(...L([5.2, top + 6.8 - snap]))} ${f.p(...L([0.4, top + 8.2]))}Z`,
  ].join("");
  return [
    shape(f.band([L([0, 6]), L([0, top])], 0.9), "wood-700", 1, INKED),
    shape(flag, "red-600", 1, INKED),
    shape(f.poly([L([4.2, top + 0.4]), L([6.6, top + 0.2 + snap / 2]), L([6.4, top + 7.2 - snap / 2]), L([4.2, top + 7.4])]), "slate-900", 0.2),
    shape(f.ellipse(...L([5.2, top + 5.2]), 1.3, 1) + f.circle(...L([3.9, top + 3.4]), 0.5) + f.circle(...L([5.2, top + 2.8]), 0.55) + f.circle(...L([6.5, top + 3.4]), 0.5), "white", 0.9),
    shape(f.circle(...L([0, top - 0.8]), 1), "brass-500", 1, INKED),
  ];
}

/** A small round flask held up, glowing violet: a glass bulb half full of violet-500, a neck, a cork, a glint and a spark above it. */
function flask(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glow = between(rng, 0.2, 0.35);
  const [cx, cy] = [0.8, -3.6];
  const level = cy - 0.4;
  const w = Math.sqrt(2.2 ** 2 - 0.4 ** 2);
  const arc = `A${fmt(f.r(2.2))} ${fmt(f.r(2.2))} 0 1 ${f.sweep(0)} `;
  return [
    shape(f.circle(cx, cy - 0.6, 4.6), "violet-500", glow),
    shape(f.circle(cx, cy, 2.6), "slate-100", 0.4, INKED),
    shape(`M${f.p(cx - w, level)}${arc}${f.p(cx + w, level)}Z`, "violet-500", 1),
    shape(f.poly([[cx - 0.8, cy - 2.4], [cx + 0.8, cy - 2.4], [cx + 0.8, cy - 4.4], [cx - 0.8, cy - 4.4]]), "slate-100", 0.55, INKED),
    shape(f.poly([[cx - 1, cy - 4.2], [cx + 1, cy - 4.2], [cx + 0.9, cy - 5.6], [cx - 0.9, cy - 5.6]]), "wood-600", 1, INKED),
    shape(f.ellipse(cx - 1, cy - 1.2, 0.7, 0.45), "white", 0.8),
    shape(sparkle(f, cx + 3, cy - 5, 1.4), "violet-300", 0.9),
  ];
}

/** A drawn longbow about 24 tall, its grip in the hand, the string pulled back 5 to 6 behind it with an arrow nocked, its head forward. */
function bow(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const pull = between(rng, 5, 6.4);
  const tilt = between(rng, -6, 6);
  const T = (p: Point): Point => turned(p, tilt);
  const q = (t: number): Point => T([(1 - t) ** 2 * -2.6 + 2 * (1 - t) * t * 4.6 + t * t * -2.6, (1 - t) ** 2 * -11.8 + t * t * 11.8]);
  const back = -pull;
  return [
    shape(f.band([q(0), T([back, 0]), q(1)], 0.3), "slate-100", 0.9),
    shape(f.band([0, 0.15, 0.3, 0.42, 0.5, 0.58, 0.7, 0.85, 1].map(q), 1), "wood-700", 1, INKED),
    shape(f.band([q(0.44), q(0.56)], 1.6), "wood-800", 1),
    shape(f.band([T([back, 0]), T([4.4, 0])], 0.45), "wood-600", 1),
    shape(f.poly([T([4.2, -0.9]), T([6.4, 0]), T([4.2, 0.9])]), "slate-400", 1, INKED),
    shape(f.poly([T([back + 0.3, -0.2]), T([back + 2.2, -0.2]), T([back + 0.6, -1.3])]) + f.poly([T([back + 0.3, 0.2]), T([back + 2.2, 0.2]), T([back + 0.6, 1.3])]), "red-600", 1),
  ];
}

/** A long spear about 28 long, 6 below the hand to a leaf blade 22 above it, leant forward, a brass binding and a red tassel under its blade. */
function spear(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 2, 12);
  const T = (p: Point): Point => turned(p, tilt);
  return [
    shape(f.band([T([0, 6]), T([0, -17])], 0.9), "wood-700", 1, INKED),
    shape(f.poly([T([0, -16.6]), T([-1.3, -18.6]), T([0, -22.4]), T([1.3, -18.6])]), "slate-100", 1, INKED),
    shape(f.band([T([0, -15.4]), T([0, -16.8])], 1.3), "brass-600", 1),
    shape(f.band([T([0.4, -15.4]), T([2, -13.4]), T([1.6, -11.6])], 0.6), "red-600", 1),
    shape(f.band([T([0.2, -17.4]), T([0.2, -21.2])], 0.25), "white", 0.5),
  ];
}

/** A silver coin (slate-400) flipping about 7 above the hand, seen part edge-on, its rim, a pale face, a glint and two flick lines below it. */
function coin(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spin = between(rng, 0.35, 1);
  const tilt = between(rng, -20, 20);
  const [cx, cy] = [0.6, -7];
  return [
    shape(f.band([[-1, -2], [-1.6, -3.6], [-1.2, -5]], 0.35) + f.band([[2.2, -2.2], [2.8, -3.8], [2.4, -5]], 0.35), "slate-100", 0.45),
    shape(oval(f, cx + 0.5, cy + 0.2, 2.4 * spin, 2.4, tilt), "slate-700", 1, INKED),
    shape(oval(f, cx, cy, 2.4 * spin, 2.4, tilt), "slate-400", 1, INKED),
    shape(oval(f, cx, cy, 1.5 * spin, 1.5, tilt), "slate-100", 0.5),
    shape(sparkle(f, cx - 0.8 * spin, cy - 1.2, 1.3), "white", 0.9),
  ];
}

/** A sling swung from the fist: two cords to a leather pouch about 6 ahead, a pebble in it and the arc of its swing. */
function sling(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -1, 1);
  const [px, py] = [6.4, 3.4 + swing];
  return [
    shape(f.band([[2, -3.6], [5.6, -3.2], [8.4, -0.8], [9, 2.4]], 0.35), "slate-100", 0.35),
    shape(f.band([[0, 0], [3.2, 0.2 + swing * 0.4], [px - 0.8, py - 1]], 0.4) + f.band([[0.2, 0.6], [3.4, 2.4 + swing * 0.6], [px - 1, py + 0.6]], 0.4), "wood-600", 1),
    shape(f.circle(px + 0.2, py - 1.1, 0.9), "stone-500", 1, INKED),
    shape(oval(f, px, py, 1.8, 1.2, 20), "wood-700", 1, INKED),
    shape(f.circle(px - 0.1, py - 1.5, 0.3), "white", 0.4),
  ];
}

/** A round silver tray held flat on the hand, about 13 across, three small cards standing fanned on it, each with a red pip. */
function tray(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -6, 6);
  const [cx, cy] = [2.4, -1.6];
  const card = (x: number, degrees: number): string => f.poly(([[-1.2, 0], [1.2, 0], [1.2, -3.2], [-1.2, -3.2]] as const).map(([dx, dy]) => {
    const [tx, ty] = turned([dx, dy], degrees + lean);
    return [cx + x + tx, cy + ty] as const;
  }));
  const pip = (x: number, degrees: number): string => {
    const [tx, ty] = turned([0, -1.6], degrees + lean);
    return f.poly([[cx + x + tx, cy + ty - 0.6], [cx + x + tx + 0.5, cy + ty], [cx + x + tx, cy + ty + 0.6], [cx + x + tx - 0.5, cy + ty]]);
  };
  return [
    shape(f.ellipse(cx, cy + 0.6, 6.4, 1.7), "slate-700", 1, INKED),
    shape(f.ellipse(cx, cy, 6.4, 1.7), "slate-400", 1, INKED),
    shape(f.ellipse(cx, cy, 5, 1.1), "slate-100", 0.45),
    shape(card(-2.6, -12) + card(0, 0) + card(2.6, 12), "white", 1, INKED),
    shape(pip(-2.6, -12) + pip(0, 0) + pip(2.6, 12), "red-600", 1),
    shape(f.ellipse(cx - 3.4, cy - 0.5, 1.6, 0.35), "white", 0.6),
  ];
}

/** A rolled scroll held half open: a sheet about 7 wide between a thin top roll and the thick roll of the rest, its lines of writing and a red seal. */
function scroll(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const open = between(rng, 4.4, 6);
  const [l, r, top] = [-0.6, 6, -4.6];
  const bottom = top + open;
  const lines = [1.4, 2.4, 3.4, 4.4].filter((dy) => top + dy < bottom - 0.6).map((dy, k) => f.band([[l + 0.8, top + dy], [r - 1 - (k % 2) * 1.4, top + dy]], 0.3)).join("");
  return [
    shape(f.poly([[l, top], [r, top], [r + 0.2, bottom], [l + 0.2, bottom]]), "slate-100", 1, INKED),
    shape(lines, "slate-400", 0.8),
    shape(f.band([[l - 0.8, top], [r + 0.8, top]], 1.5), "white", 1, INKED),
    shape(f.band([[l - 0.6, bottom + 0.6], [r + 1, bottom + 0.6]], 2.4), "white", 1, INKED),
    shape(f.band([[l - 0.4, bottom + 1.3], [r + 0.8, bottom + 1.3]], 0.9), "slate-900", 0.18),
    shape(f.circle(l - 1.2, top, 0.55) + f.circle(r + 1.2, top, 0.55) + f.circle(l - 1, bottom + 0.6, 0.6) + f.circle(r + 1.4, bottom + 0.6, 0.6), "wood-700", 1, INKED),
    shape(f.circle(r - 1.4, bottom - 0.8, 0.8), "red-600", 1, INKED),
  ];
}

/** A big hourglass about 12 tall held at its base: two glass bulbs between wooden plates and posts, the sand running from the top bulb to a pile in the bottom one. */
function hourglass(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sand = between(rng, 0.3, 0.7);
  const cx = 2;
  const [top, bottom] = [-11.4, 0.6];
  const mid = (top + bottom) / 2;
  const glass = [
    `M${f.p(cx - 2.6, top + 1.2)}L${f.p(cx + 2.6, top + 1.2)}Q${f.p(cx + 2.8, mid - 1.6)} ${f.p(cx + 0.5, mid)}Q${f.p(cx + 2.8, mid + 1.6)} ${f.p(cx + 2.6, bottom - 1.2)}`,
    `L${f.p(cx - 2.6, bottom - 1.2)}Q${f.p(cx - 2.8, mid + 1.6)} ${f.p(cx - 0.5, mid)}Q${f.p(cx - 2.8, mid - 1.6)} ${f.p(cx - 2.6, top + 1.2)}Z`,
  ].join("");
  const level = mid - 1 - sand * 3;
  const w = 1.4 + sand * 0.7;
  const pile = bottom - 1.6 - (1 - sand) * 2.4;
  return [
    shape(glass, "slate-100", 0.35, INKED),
    shape(f.poly([[cx - w, level], [cx + w, level], [cx + 0.4, mid - 0.4], [cx - 0.4, mid - 0.4]]), "brass-500", 1),
    shape(f.band([[cx, mid - 0.4], [cx, pile]], 0.3), "brass-500", 1),
    shape(f.poly([[cx - 2.3, bottom - 1.3], [cx + 2.3, bottom - 1.3], [cx + 0.6, pile], [cx - 0.6, pile]]), "brass-500", 1),
    shape(f.band([[cx - 3.1, top + 0.6], [cx - 3.1, bottom - 0.6]], 0.7) + f.band([[cx + 3.1, top + 0.6], [cx + 3.1, bottom - 0.6]], 0.7), "wood-700", 1, INKED),
    shape(f.poly([[cx - 3.8, top], [cx + 3.8, top], [cx + 3.8, top + 1.2], [cx - 3.8, top + 1.2]]) + f.poly([[cx - 3.8, bottom - 1.2], [cx + 3.8, bottom - 1.2], [cx + 3.8, bottom], [cx - 3.8, bottom]]), "wood-700", 1, INKED),
    shape(f.band([[cx - 1.8, top + 2], [cx - 1.4, mid - 1.8]], 0.4), "white", 0.6),
  ];
}

/** A tied loot sack (burlap brass-600) hung from the fist by its gathered neck, bulging about 9 wide and 9 deep below a red tie, a glint at its mouth. */
function sack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const b = between(rng, -0.6, 0.6);
  const body = `M${f.p(-0.4, 1.2)}Q${f.p(-4.6 - b, 2.2)} ${f.p(-3.8 - b, 6.8)}Q${f.p(-3, 9.8)} ${f.p(0.7, 9.8)}Q${f.p(4.6, 9.8)} ${f.p(5.2 + b, 6.6)}Q${f.p(5.6 + b, 2.2)} ${f.p(1.8, 1.2)}Z`;
  return [
    shape(body, "brass-600", 1, INKED),
    shape(`M${f.p(1.6, 2.4)}Q${f.p(5.6 + b, 2.2)} ${f.p(5.2 + b, 6.6)}Q${f.p(4.6, 9.8)} ${f.p(0.7, 9.8)}Q${f.p(4, 7)} ${f.p(1.6, 2.4)}Z`, "slate-900", 0.22),
    shape(f.band([[-2.2, 5.4], [-1, 7.6]], 0.4) + f.band([[1.8, 4.4], [2.6, 6.6]], 0.4), "wood-700", 0.5),
    shape(f.ellipse(-2, 4, 1, 1.6), "white", 0.2),
    shape(f.poly([[-0.6, 1], [-2, -1.8], [-0.6, -0.8], [0.4, -2.6], [1.2, -0.8], [2.8, -1.6], [2, 1]]), "brass-600", 1, INKED),
    shape(f.band([[-0.9, 1.2], [2.3, 1.2]], 1), "red-600", 1, INKED),
    shape(sparkle(f, 3.6, -2.4, 1.2), "white", 0.8),
  ];
}

/** A tall rectangular tower shield about 10 wide and 18 tall, its face bowed a little, a steel rim, a round boss, rivets and a pale stripe. */
function towerShield(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bow = between(rng, 0.4, 0.9);
  const face = (l: number, r: number, t: number, b: number): string =>
    `M${f.p(l, t)}Q${f.p((l + r) / 2, t - bow)} ${f.p(r, t)}L${f.p(r, b)}Q${f.p((l + r) / 2, b + bow)} ${f.p(l, b)}Z`;
  const rivets = [[-2.6, -8.8], [5.4, -8.8], [-2.6, 6.8], [5.4, 6.8]].map(([x, y]) => f.circle(x ?? 0, y ?? 0, 0.4)).join("");
  return [
    shape(face(-3.6, 6.4, -10, 8), "blue-800", 1, INKED),
    shape(face(-3.6, 6.4, -10, 8) + face(-2.7, 5.5, -9.1, 7.1), "slate-400", 1),
    shape(f.poly([[0.6, -9.1], [2.2, -9.1], [2.2, 7.1], [0.6, 7.1]]), "slate-100", 0.18),
    shape(f.circle(1.4, -1, 1.9), "slate-400", 1, INKED),
    shape(f.circle(1, -1.5, 0.6), "slate-100", 0.8),
    shape(rivets, "slate-100", 0.9),
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
  // R3390000: the Classic+ scenes' parts, after the first ones so their order holds.
  "pose-spread": { category: "pose", draw: poseSpread },
  "pose-strike": { category: "pose", draw: poseStrike },
  "pose-run": { category: "pose", draw: poseRun },
  "pose-sit": { category: "pose", draw: poseSit },
  "pose-flex": { category: "pose", draw: poseFlex },
  "pose-kneel": { category: "pose", draw: poseKneel },
  "head-bat": { category: "head", draw: headBat },
  "head-skull": { category: "head", draw: headSkull },
  "head-mushroom": { category: "head", draw: headMushroom },
  "head-sun": { category: "head", draw: headSun },
  "head-golem": { category: "head", draw: headGolem },
  "head-automaton": { category: "head", draw: headAutomaton },
  "head-goblin": { category: "head", draw: headGoblin },
  "head-mummy": { category: "head", draw: headMummy },
  kerchief: { category: "hair", draw: kerchief },
  "hat-brim": { category: "hair", draw: hatBrim },
  goggles: { category: "hair", draw: goggles },
  helm: { category: "hair", draw: helm },
  tonsure: { category: "hair", draw: tonsure },
  "outfit-apron": { category: "outfit", draw: apron },
  "outfit-robe": { category: "outfit", draw: plainRobe },
  "outfit-plate": { category: "outfit", draw: plate },
  "outfit-tabard": { category: "outfit", draw: tabard },
  "outfit-poncho": { category: "outfit", draw: poncho },
  "outfit-wraps": { category: "outfit", draw: wraps },
  "outfit-muscle": { category: "outfit", draw: muscle },
  "outfit-drum": { category: "outfit", draw: drumChest },
  "outfit-cardigan": { category: "outfit", draw: cardigan },
  "outfit-leaves": { category: "outfit", draw: leafSuit },
  "outfit-patched": { category: "outfit", draw: patched },
  "outfit-stone": { category: "outfit", draw: stoneBody },
  "item-mallet": { category: "item", draw: mallet },
  "item-dagger": { category: "item", draw: dagger },
  "item-spoon": { category: "item", draw: spoon },
  "item-banner": { category: "item", draw: banner },
  "item-flask": { category: "item", draw: flask },
  "item-bow": { category: "item", draw: bow },
  "item-spear": { category: "item", draw: spear },
  "item-coin": { category: "item", draw: coin },
  "item-sling": { category: "item", draw: sling },
  "item-tray": { category: "item", draw: tray },
  "item-scroll": { category: "item", draw: scroll },
  "item-hourglass": { category: "item", draw: hourglass },
  "item-sack": { category: "item", draw: sack },
  "item-tower-shield": { category: "item", draw: towerShield },
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
