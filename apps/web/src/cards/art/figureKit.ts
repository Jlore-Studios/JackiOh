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
// R3392000: Core's scenes (#3392) added the poses, heads, headgear, outfits and items their briefs name, each
// after the old ones of its kind.
// R3393000: the Meditative+ and Training Grounds scenes (#3393) added four poses (post, treads, lift, hips), three
// heads, six hairs, headgears and glasses, eight outfits (the living potions' flask, the dummy's straw, the totem,
// the sluice gate, the Golem's posts, the Shredder's machine among them) and ten held items; its lab coat
// and club are #3392's (`outfit-labcoat`, `item-club`), its tree-trunk club `item-trunk-club`.
// R3401000: the Meditative scenes (#3401) added six hairs and headgears, four outfits and ten held items.
// R3403000: Stinkyscape's later scenes (#3403) added four headgears and hairs (a neural crown, a bubble helmet, silver
// hair, a top hat), seven outfits and six held items.

import type { Tag } from "@jackioh/shared";

import { fmt } from "./hash.ts";
import { between, DEGREES, blob, leant, sparkle, turned } from "./kitDraw.ts";
import type { FigureRecipe, KitPart, KitShape, PaletteName, PartPlace, Rng } from "./scene.ts";
import { local, type Local } from "./sceneKit.ts";

type Point = readonly [number, number];

export type PoseId =
  | "pose-stand"
  | "pose-guard"
  | "pose-raise"
  | "pose-spread"
  | "pose-strike"
  | "pose-run"
  | "pose-sit"
  | "pose-flex"
  | "pose-kneel"
  | "pose-lunge"
  | "pose-stride"
  | "pose-perch"
  | "pose-genuflect"
  | "pose-cast"
  | "pose-point"
  | "pose-hold"
  | "pose-block"
  | "pose-post"
  | "pose-treads"
  | "pose-lift"
  | "pose-hips";
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
  | "head-mummy"
  | "head-lizard"
  | "head-robot"
  | "head-stone"
  | "head-ghoul"
  | "head-lantern"
  | "head-cloud"
  | "head-eyes"
  | "head-sack"
  | "head-ogre";
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
  | "tonsure"
  | "steel-helmet"
  | "horns"
  | "cap-feather"
  | "bucket-hat"
  | "spectacles"
  | "headband"
  | "face-mask"
  | "wild-goggles"
  | "veil"
  | "fedora"
  | "hair-spiky"
  | "beard"
  | "flap-cap"
  | "scholar-cap"
  | "sweatband"
  | "glasses"
  | "headphones"
  | "beanie"
  | "pompadour"
  | "conductor-cap"
  | "tall-hat"
  | "hair-bun"
  | "neural-crown"
  | "space-helmet"
  | "hair-silver"
  | "top-hat";
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
  | "outfit-stone"
  | "outfit-layered-plate"
  | "outfit-waistcoat"
  | "outfit-sweater"
  | "outfit-labcoat"
  | "outfit-tutor-cardigan"
  | "outfit-bomber"
  | "outfit-work-apron"
  | "outfit-hoodie"
  | "outfit-rags"
  | "outfit-rock"
  | "outfit-gown"
  | "outfit-blazer"
  | "outfit-stripes"
  | "outfit-lantern"
  | "outfit-flask"
  | "outfit-straw"
  | "outfit-totem"
  | "outfit-sluice"
  | "outfit-posts"
  | "outfit-machine"
  | "outfit-parka"
  | "outfit-vest"
  | "outfit-overalls"
  | "outfit-kaftan"
  | "outfit-tailcoat"
  | "outfit-silks"
  | "outfit-flightsuit"
  | "outfit-greatcoat"
  | "outfit-sleeveless"
  | "outfit-plated-parka"
  | "outfit-frock"
  | "outfit-cloak"
  | "outfit-harness";
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
  | "item-tower-shield"
  | "item-rapier"
  | "item-knife"
  | "item-pavise"
  | "item-silver-coin"
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
  | "item-twig-spear"
  | "item-broom"
  | "item-axe"
  | "item-boulder"
  | "item-wand"
  | "item-ladle"
  | "item-purse"
  | "item-brush"
  | "item-oar"
  | "item-pointer"
  | "item-lance"
  | "item-trunk-club"
  | "item-lantern"
  | "item-phone"
  | "item-key"
  | "item-baton"
  | "item-ink-brush"
  | "item-loupe"
  | "item-fan"
  | "item-twin-blades"
  | "item-rod"
  | "item-comb"
  | "item-shock-axe"
  | "item-dart"
  | "item-orb"
  | "item-slate"
  | "item-torch"
  | "item-vial";
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

/** Each pose's neck, hand and hip, as offsets from the feet at scale 1. */
export const POSE_ANCHORS: Readonly<Record<PoseId, { readonly neck: Point; readonly hand: Point; readonly hip: Point }>> = {
  "pose-stand": { neck: [0, -44], hand: [9, -23.4], hip: [0, -24] },
  "pose-guard": { neck: [0, -43], hand: [8.5, -31], hip: [0, -23] },
  "pose-raise": { neck: [0, -44], hand: [11, -56.2], hip: [0, -24] },
  "pose-lunge": { neck: [0, -42], hand: [17, -31], hip: [0, -22] },
  "pose-stride": { neck: [0, -44], hand: [9, -30], hip: [0, -24] },
  "pose-perch": { neck: [0, -33], hand: [8, -15], hip: [0, -13] },
  "pose-genuflect": { neck: [0, -35], hand: [9, -27], hip: [0, -15] },
  "pose-cast": { neck: [0, -44], hand: [14, -34], hip: [0, -24] },
  "pose-point": { neck: [0, -44], hand: [13, -41], hip: [0, -24] },
  "pose-hold": { neck: [0, -44], hand: [6, -30], hip: [0, -24] },
  "pose-block": { neck: [0, -44], hand: [17, -37], hip: [0, -24] },
  "pose-spread": { neck: [0, -44], hand: [17.9, -48.4], hip: [0, -24] },
  "pose-strike": { neck: [0, -43], hand: [-2.9, -58.3], hip: [0, -23] },
  "pose-run": { neck: [2, -43], hand: [14, -37.8], hip: [0, -23] },
  "pose-sit": { neck: [0, -34], hand: [10.8, -16.4], hip: [0, -14] },
  "pose-flex": { neck: [0, -44], hand: [11, -49.6], hip: [0, -24] },
  "pose-kneel": { neck: [0, -36], hand: [17.3, -42.3], hip: [0, -16] },
  "pose-post": { neck: [0, -44], hand: [9, -34], hip: [0, -24] },
  "pose-treads": { neck: [0, -34], hand: [12, -24], hip: [0, -14] },
  "pose-lift": { neck: [0, -44], hand: [0, -62.4], hip: [0, -24] },
  "pose-hips": { neck: [0, -44], hand: [7.8, -25.6], hip: [0, -24] },
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
function poseStride(at: PartPlace, rng: Rng): KitShape[] {
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
function posePerch(at: PartPlace, rng: Rng): KitShape[] {
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
function poseGenuflect(at: PartPlace, rng: Rng): KitShape[] {
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
function poseBlock(at: PartPlace, rng: Rng): KitShape[] {
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

/** Layered plate on the torso, big pauldrons and a tasset skirt, in the look's cloth. */
function outfitLayeredPlate(at: PartPlace, rng: Rng): KitShape[] {
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
function outfitTutorCardigan(at: PartPlace, rng: Rng): KitShape[] {
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
function outfitWorkApron(at: PartPlace, rng: Rng): KitShape[] {
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

/** R3401000: denim overalls (blue-800), bib and shorts in one, on straps with brass buttons over a shirt in the look's cloth. */
function outfitOveralls(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const roll = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-7.2, SHOULDER], [7.2, SHOULDER], [6.8, 1.6], [-6.8, 1.6]]), "red-400", 1, MAIN_INKED),
    shape(
      f.poly([[-4.2, -11.6], [4.2, -11.6], [4.6, -0.4], [6.8, 0.4], [7, 8.6 + roll], [1, 8.6 - roll], [0, 4.6], [-1, 8.6 - roll], [-7, 8.6 + roll], [-6.8, 0.4], [-4.6, -0.4]]),
      "blue-800",
      1,
      INKED,
    ),
    shape(f.band([[-3.6, -11.4], [-4.8, SHOULDER + 0.2]], 1.1) + f.band([[3.6, -11.4], [4.8, SHOULDER + 0.2]], 1.1), "blue-800", 1, INKED),
    shape(f.poly([[-2.4, -9], [2.4, -9], [2.2, -5.4], [-2.2, -5.4]]) + f.band([[-6.4, 7.4], [-1.4, 7.4]], 0.5) + f.band([[1.4, 7.4], [6.4, 7.4]], 0.5), "slate-900", 0.3),
    shape(f.circle(-3.4, -10.8, 0.6) + f.circle(3.4, -10.8, 0.6), "brass-600", 1),
    shape(f.band([[-0.2, -11], [0, 3.6]], 0.3), "slate-100", 0.4),
  ];
}

/** R3401000: a long kaftan to the ankle in the look's cloth, a brass-embroidered placket down its front and a wide red-900 sash at the waist. */
function outfitKaftan(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.5, 0.5);
  return [
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [7.8, 4], [8.8, HEM - 1 + j()], [-8.8, HEM - 1 + j()], [-7.8, 4]]), "slate-800", 1, MAIN_INKED),
    shape(f.poly([[-2.6, SHOULDER], [2.6, SHOULDER], [0, SHOULDER + 4.6]]), "slate-900", 0.5),
    shape(f.band([[0, SHOULDER + 4.4], [0.2, -1.6]], 1) + f.band([[0.2, 1.4], [0.6, HEM - 1.4]], 1), "brass-600", 0.85),
    shape(f.poly([[-7.4, -2], [7.4, -2], [7.6, 1.6], [-7.6, 1.6]]) + f.poly([[-3.2, 1.6], [-1.6, 1.6], [-2.2, 7.4 + j()], [-3.8, 7]]), "red-900", 1, INKED),
    shape(f.band([[-4.4, 3], [-5.4, HEM - 2]], 0.6) + f.band([[4, 3], [5.2, HEM - 2]], 0.6), "slate-900", 0.2),
  ];
}

/** R3401000: an evening tailcoat in the look's cloth, cut away in front, its two tails hanging behind to the knee, over a white shirt front and a black bow tie. */
function outfitTailcoat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swing = between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-6.4, -1.2], [-2, 0], [-3 + swing, 13], [-5.2 + swing, 14.6], [-7.8 + swing, 12.4]]), "slate-900", 1, MAIN_INKED),
    shape(f.poly([[-7, SHOULDER], [7, SHOULDER], [6.4, -1.4], [3.8, 1.2], [0.6, -3], [-6.4, 0.4]]), "slate-900", 1, MAIN_INKED),
    shape(f.poly([[-2.4, SHOULDER], [2.4, SHOULDER], [1.4, -4.4], [-1.2, -4.4]]), "white", 1, INKED),
    shape(f.poly([[-2.4, SHOULDER], [-4.4, SHOULDER + 1.2], [-1.2, -6]]) + f.poly([[2.4, SHOULDER], [4.4, SHOULDER + 1.2], [1.2, -6]]), "black", 0.45),
    shape(f.poly([[0, SHOULDER + 1.4], [-2, SHOULDER + 0.4], [-2, SHOULDER + 2.4]]) + f.poly([[0, SHOULDER + 1.4], [2, SHOULDER + 0.4], [2, SHOULDER + 2.4]]), "black", 1),
    shape(f.circle(0.2, -11, 0.35) + f.circle(0.3, -8, 0.35), "slate-900", 0.9),
  ];
}

/** R3401000: loose silks in the look's cloth, a cross collar and a sash, their hem and the sash's ends blown back hard the way the figure does not face. */
function outfitSilks(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gust = between(rng, -1, 1);
  return [
    shape(f.poly([[-6.4, -1.4], [-12.6, -4.2 + gust], [-14.6, -2.6 + gust], [-12, -0.8], [-6.4, 0.8]]), "slate-100", 0.85, INKED),
    shape(
      f.poly([[-7.4, SHOULDER], [7.4, SHOULDER], [8.4, 2], [8, 9], [4, 12.6], [-1.6, 12.4 + gust * 0.5], [-7.4, 14.2 + gust], [-12.8 + gust, 12], [-9.4, 7.6], [-8.6, 2]]),
      "slate-400",
      1,
      MAIN_INKED,
    ),
    shape(f.band([[-3, SHOULDER], [2.6, -7]], 0.8) + f.band([[3, SHOULDER], [0.6, -10.6]], 0.6), "slate-900", 0.4),
    shape(f.poly([[-7.6, -1.8], [7.8, -1.8], [7.9, 0.8], [-7.7, 0.8]]), "slate-100", 0.9, INKED),
    shape(f.band([[3.6, 2.4], [-1.4, 10.4]], 0.5) + f.band([[-2, 2.4], [-8.6, 10.6 + gust * 0.5]], 0.5) + f.band([[6.4, 3], [5, 9]], 0.4), "slate-900", 0.18),
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
function wildGoggles(at: PartPlace, rng: Rng): KitShape[] {
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
function fedora(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const dent = between(rng, 0.4, 1);
  return [
    shape(f.ellipse(0, c - 3, 7.4, 1.5), "slate-700", 1, INKED),
    shape(`M${f.p(-4.4, c - 3.2)}C${f.p(-4.8, c - 8)} ${f.p(-1.2, c - 8.6 + dent)} ${f.p(0, c - 7.6)}C${f.p(1.2, c - 8.6 + dent)} ${f.p(4.8, c - 8)} ${f.p(4.4, c - 3.2)}Z`, "slate-700", 1, INKED),
    shape(f.poly([[-4.5, c - 3.8], [4.5, c - 3.8], [4.6, c - 5.2], [-4.6, c - 5.2]]), "slate-900", 1),
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

/** R3401000: an over-ear headset: a thin band over the crown and slate cups low at the sides (so a cat-folk's ears still show), their pads lit cyan, a mic boom forward. */
function headphones(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const boom = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-5.4, c + 0.4], [-5.4, c - 3.4], [-2.6, c - 5.8], [0, c - 6.2], [2.6, c - 5.8], [5.4, c - 3.4], [5.4, c + 0.4]], 0.8), "slate-800", 1, INKED),
    shape(f.band([[5.6, c + 1.8], [4.6, c + 3.6 + boom], [2.2, c + 3.8 + boom]], 0.4) + f.circle(1.8, c + 3.8 + boom, 0.6), "slate-800", 1),
    shape(f.ellipse(-5.4, c + 0.8, 1.7, 2.4) + f.ellipse(5.4, c + 0.8, 1.7, 2.4), "slate-700", 1, INKED),
    shape(f.ellipse(-5.4, c + 0.8, 0.8, 1.3) + f.ellipse(5.4, c + 0.8, 0.8, 1.3), "cyan-400", 0.85),
    shape(f.circle(5.4, c + 0.8, 3), "cyan-400", 0.15),
  ];
}

/** R3401000: a knitted beanie (green-700) pulled down to the brow: a folded cuff, ribbing and a small bobble on top. */
function beanie(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const slouch = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-5.2, c - 1.2)}C${f.p(-5.6, c - 8.4)} ${f.p(5.6 + slouch, c - 8.8)} ${f.p(5.2, c - 1.2)}Z`, "green-700", 1, INKED),
    shape(f.circle(-0.6 + slouch, c - 7.6, 1.2), "slate-100", 1, INKED),
    shape(f.poly([[-5.5, c - 2], [5.5, c - 2], [5.6, c + 0.4], [-5.6, c + 0.4]]), "green-900", 1, INKED),
    shape(f.band([[-2.4, c - 2.2], [-2.6, c - 6.4]], 0.3) + f.band([[0, c - 2.2], [0, c - 7]], 0.3) + f.band([[2.4, c - 2.2], [2.6, c - 6.4]], 0.3), "slate-900", 0.3),
  ];
}

/** R3401000: slick black hair swept up into a tall forward quiff over the brow, with short sideburns and a white shine. */
function pompadour(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const rise = between(rng, -0.6, 0.6);
  return [
    shape(
      `M${f.p(-5, c + 1.2)}C${f.p(-5.8, c - 5.6)} ${f.p(-2, c - 9.4 - rise)} ${f.p(2.4, c - 9 - rise)}C${f.p(5.6, c - 8.8 - rise)} ${f.p(7.4, c - 6.4)} ${f.p(5.6, c - 4.4)}Q${f.p(4.6, c - 3.4)} ${f.p(3, c - 3.4)}Q${f.p(-1, c - 3)} ${f.p(-3.6, c - 1.4)}L${f.p(-3.8, c + 1.6)}Z`,
      "slate-900",
      1,
      INKED,
    ),
    shape(`M${f.p(-2.4, c - 6.6 - rise)}Q${f.p(1.6, c - 9.2 - rise)} ${f.p(5, c - 7)}Q${f.p(1.6, c - 8 - rise)} ${f.p(-2.4, c - 6.6 - rise)}Z`, "white", 0.4),
    shape(f.poly([[3.6, c - 0.6], [4.6, c - 0.8], [4.4, c + 2], [3.6, c + 2]]), "slate-900", 1),
  ];
}

/** R3401000: a railway conductor's navy peaked cap (blue-900): a flat crown, a dark band, a black peak forward and a brass badge. */
function conductorCap(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tilt = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-4.9, c - 2.6], [-5.8, c - 7 + tilt], [5.8, c - 7.4 - tilt], [4.9, c - 2.6]]), "blue-900", 1, INKED),
    shape(f.poly([[-5, c - 2.4], [5, c - 2.4], [5.1, c - 4], [-5.1, c - 4]]), "slate-900", 1),
    shape(f.poly([[1.4, c - 2.8], [7.6, c - 1.8], [7.2, c - 0.9], [1.2, c - 1.8]]), "black", 1, INKED),
    shape(f.circle(0.8, c - 5.2, 0.9), "brass-600", 1, INKED),
    shape(f.circle(0.6, c - 5.4, 0.3), "white", 0.8),
  ];
}

/** R3401000: a mayor's tall black top hat: a narrow curled brim, a tall crown widening a little upward, a red-900 band and a white sheen. */
function tallHat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const lean = between(rng, -0.5, 0.5);
  return [
    shape(f.poly([[-4.2, c - 3.4], [-4.6 + lean, c - 15], [4.6 + lean, c - 15], [4.2, c - 3.4]]), "slate-900", 1, INKED),
    shape(f.ellipse(0, c - 3.4, 7, 1.4), "slate-900", 1, INKED),
    shape(f.poly([[-4.2, c - 3.8], [4.2, c - 3.8], [4.3, c - 5.8], [-4.3, c - 5.8]]), "red-900", 1),
    shape(f.poly([[-2.6, c - 6.4], [-1.6, c - 6.4], [-1.6 + lean, c - 14.4], [-2.6 + lean, c - 14.4]]), "white", 0.22),
  ];
}

/** R3401000: smooth black hair drawn up into a topknot bun on the crown, a jade pin through it. */
function hairBun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tip = between(rng, -0.6, 0.6);
  return [
    shape(f.circle(-0.4, c - 7.4, 2.3), "slate-900", 1, INKED),
    shape(`M${f.p(-5, c + 0.6)}C${f.p(-5.4, c - 6.4)} ${f.p(5.4, c - 6.4)} ${f.p(5, c + 0.6)}Q${f.p(2.6, c - 2.6)} ${f.p(0, c - 2.8)}Q${f.p(-2.6, c - 2.6)} ${f.p(-5, c + 0.6)}Z`, "slate-900", 1, INKED),
    shape(f.band([[-3.6, c - 6.4 + tip], [3, c - 8.6 - tip]], 0.5), "jade-400", 1),
    shape(f.ellipse(-1.4, c - 8.2, 0.9, 0.5) + f.ellipse(-2, c - 4.4, 1.2, 0.5), "white", 0.3),
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
function knife(at: PartPlace, rng: Rng): KitShape[] {
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
function pavise(at: PartPlace, rng: Rng): KitShape[] {
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
function silverCoin(at: PartPlace, rng: Rng): KitShape[] {
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
function twigSpear(at: PartPlace, rng: Rng): KitShape[] {
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

// R3393000: the Meditative+ and Training Grounds scenes' parts: things that stand as Units (a potion, a dummy, a
// totem, a sluice gate, a golem of posts, a shredder) and the people the briefs name.

/** No legs or arms: a wooden stake planted in a little mound of earth up to the hip (a dummy's, a totem's, a gate's); its hand beside the shoulder. */
function posePost(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.6, 0.6);
  const heap = between(rng, 1.6, 2.6);
  return [
    shape(`M${f.p(-7.4, 0.4)}Q${f.p(-3, -heap)} ${f.p(0, -heap)}Q${f.p(3.4, -heap)} ${f.p(7.4, 0.4)}Z`, "wood-800", 1, INKED),
    shape(f.band([[lean, -0.6], [0, -25]], 3.2), "wood-700", 1, INKED),
    shape(f.band([[lean + 0.9, -1], [0.9, -24.6]], 0.8), "wood-900", 0.4),
    shape(f.band([[lean - 0.8, -4], [-0.6, -12]], 0.4) + f.band([[lean - 0.4, -15], [-0.3, -21]], 0.4), "wood-600", 0.6),
    shape(f.poly([[-6, 0.2], [-5.4, -2.6], [-4.6, -0.4], [-4, -3], [-3.2, 0]]) + f.poly([[3.6, 0], [4.4, -2.4], [5, -0.2], [5.8, -2.8], [6.4, 0.2]]), "green-700", 0.9),
  ];
}

/** Tank treads for legs: a track about 29 long and 9 tall round four wheels, its teeth below it, and a mount (main) up to the hip 14 above the ground; its hand out in front. */
function poseTreads(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, 0, 2.4);
  const r = fmt(f.r(4.5));
  const track = `M${f.p(-10, -9)}L${f.p(10, -9)}A${r} ${r} 0 0 ${f.sweep(1)} ${f.p(10, 0)}L${f.p(-10, 0)}A${r} ${r} 0 0 ${f.sweep(1)} ${f.p(-10, -9)}Z`;
  let teeth = "";
  for (let x = -10 + phase; x < 10; x += 2.4) teeth += f.poly([[x - 0.6, -0.2], [x + 0.6, -0.2], [x + 0.6, 0.9], [x - 0.6, 0.9]]);
  const axles = [-9, -3, 3, 9];
  return [
    shape(f.poly([[-5, -8.6], [5, -8.6], [4, -15], [-4, -15]]), "slate-700", 1, MAIN_INKED),
    shape(track, "slate-800", 1, INKED),
    shape(teeth, "slate-900", 0.9),
    shape(axles.map((x) => f.circle(x, -4.5, 2.6)).join(""), "slate-400", 1, INKED),
    shape(axles.map((x) => f.circle(x, -4.5, 0.9)).join(""), "slate-900", 1),
    shape(f.band([[-9.6, -8.2], [9.6, -8.2]], 0.5), "white", 0.25),
  ];
}

/** Legs braced wide and both arms straight up, the hands together over the crown, holding something up (a boulder, a plough, a stone); its hand 62 up. */
function poseLift(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const brace = between(rng, 0, 1.2);
  const strain = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.6, -24], [-6.4 - brace * 0.5, -12.4], [-8.4 - brace, -1.5]],
      [[2.6, -24], [6.4 + brace * 0.5, -12.4], [8.4 + brace, -1.5]],
    ],
    [
      [[-6.6, -41], [-9.4 - strain, -50.6], [-2.6, -61.4]],
      [[6.6, -41], [9.4 + strain, -50.6], [2.6, -61.4]],
    ],
  );
}

/** Feet planted apart and both fists on the hips, the elbows jutting out to each side (arms akimbo: a toll-keeper's, a bruiser's); its hand at the near hip. */
function poseHips(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const stance = between(rng, 0, 1.2);
  const jut = between(rng, -0.6, 0.6);
  return limbs(
    f,
    [
      [[-2.6, -24], [-4.2 - stance * 0.4, -12], [-5.4 - stance, -1.5]],
      [[2.6, -24], [4.2 + stance * 0.4, -12], [5.4 + stance, -1.5]],
    ],
    [
      [[-6.6, -41], [-12 - jut, -33], [-7.8, -25.6]],
      [[6.6, -41], [12 + jut, -33], [7.8, -25.6]],
    ],
  );
}

/** A living potion's body: a round glass bottle from 12 below the hip (its pose's legs showing under it, stubby) up a short neck to a cork just over the neck, about 21 wide, its liquid (main) filling it to a wavy level, bubbles and a glint. */
function flaskBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const [cy, rx, ry] = [-1, 10.5, 13];
  const level = between(rng, -6, -2);
  const w = rx * Math.sqrt(1 - ((level - cy) / ry) ** 2);
  const arc = `A${fmt(f.r(rx))} ${fmt(f.r(ry))} 0 1 ${f.sweep(0)} `;
  const bubbles = [0, 1, 2].map(() => f.circle(between(rng, -5, 5), between(rng, level + 2.4, 8), between(rng, 0.45, 0.9))).join("");
  return [
    shape(f.ellipse(0, cy, rx, ry), "slate-100", 0.55, INKED),
    shape(`M${f.p(-w, level)}${arc}${f.p(w, level)}Q${f.p(w / 2, level - 1)} ${f.p(0, level)}Q${f.p(-w / 2, level + 1)} ${f.p(-w, level)}Z`, "blue-600", 1, MAIN),
    shape(bubbles, "white", 0.55),
    shape(f.poly([[-2.6, -13.6], [2.6, -13.6], [2.6, -18.6], [-2.6, -18.6]]), "slate-100", 0.6, INKED),
    shape(f.band([[-3.2, -18.4], [3.2, -18.4]], 1), "slate-400", 1, INKED),
    shape(f.poly([[-2.2, -18.8], [2.2, -18.8], [2.6, -21.8], [-2.6, -21.8]]), "wood-600", 1, INKED),
    shape(f.ellipse(-6, -4, 1.2, 3.6), "white", 0.7),
    shape(f.ellipse(3.6, 8.4, 4.2, 1.4), "slate-900", 0.18),
  ];
}

/** A training dummy's body: a crossbar through its shoulders for arms (about 26 across, straw bursting from its ends) and a stuffed burlap torso (main) to 3 below the hip, roped at the chest and waist, a patch stitched on, straw poking from its hem. */
function strawBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const sag = between(rng, -0.6, 0.6);
  const tuft = (x: number, y: number, dir: number): string =>
    f.poly([[x, y - 1.4], [x + dir * 3.2, y - 2.6], [x + dir * 1.8, y - 0.6], [x + dir * 3.6, y + 0.4], [x + dir * 1.8, y + 0.8], [x + dir * 2.8, y + 2.4], [x, y + 1.4]]);
  const hem: Point[] = [[6.4, 3], [5, 5.6], [3.6, 3.6], [2, 6], [0.4, 3.8], [-1.4, 6.2], [-3, 3.8], [-4.6, 5.8], [-6.2, 3.4]];
  return [
    shape(f.band([[-13, S + 2 + sag], [13, S + 2 - sag]], 1.8), "wood-700", 1, INKED),
    shape(tuft(-13, S + 2 + sag, -1) + tuft(13, S + 2 - sag, 1), "brass-500", 1, INKED),
    shape(f.poly(hem), "brass-500", 1),
    shape(`M${f.p(-6.4, S)}Q${f.p(0, S - 1.6)} ${f.p(6.4, S)}Q${f.p(8, -6)} ${f.p(6.4, 3.4)}L${f.p(-6.2, 3.4)}Q${f.p(-8, -6)} ${f.p(-6.4, S)}Z`, "brass-600", 1, MAIN_INKED),
    shape(f.band([[-6.8, S + 5], [6.8, S + 5]], 0.8) + f.band([[-7.2, -1], [7.2, -1]], 0.8), "wood-800", 1),
    shape(f.poly([[1.4, -10], [4.6, -10.4], [4.8, -6.8], [1.6, -6.6]]), "stone-500", 0.9, INKED),
    shape(f.band([[1.2, -10.8], [2, -9.6]], 0.3) + f.band([[3.6, -11], [4.2, -9.8]], 0.3) + f.band([[4.4, -6.2], [5.2, -7.4]], 0.3), "wood-950", 0.7),
    shape(f.poly([[3, S + 0.4], [6.4, S], [7.8, -6], [6.2, 3.4], [3.6, 3.4]]), "slate-900", 0.18),
  ];
}

/** A squat carved-cedar totem standing on the ground: three stacked blocks (main) from 2 above the neck to the ground, about 18 wide, red wing boards jutting from the top one, a peaked cap, carved grooves at the seams and chevrons down the foot. */
function totemBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, -1.2, 1.2);
  const top = -21.6;
  const chevrons = [7.4, 11.4, 15.4].map((y) => f.band([[-6.6, y], [0, y + 2.6], [6.6, y]], 0.8)).join("");
  return [
    shape(f.poly([[-8.6, 4], [8.6, 4], [9.4, 23], [-9.4, 23]]), "wood-600", 1, MAIN_INKED),
    shape(f.poly([[-8, -14.2], [8, -14.2], [8.6, 4], [-8.6, 4]]), "wood-600", 1, MAIN_INKED),
    shape(f.poly([[-7.4, top], [7.4, top], [8, -14.2], [-8, -14.2]]), "wood-600", 1, MAIN_INKED),
    shape(f.poly([[-7.6, -19.6], [-15, -22.4 - flare], [-14, -16.4 - flare], [-7.8, -15.4]]) + f.poly([[7.6, -19.6], [15, -22.4 - flare], [14, -16.4 - flare], [7.8, -15.4]]), "red-600", 1, INKED),
    shape(f.band([[-13.6, -20 - flare], [-8.4, -18.2]], 0.4) + f.band([[13.6, -20 - flare], [8.4, -18.2]], 0.4), "slate-900", 0.45),
    shape(f.poly([[-8.4, top + 0.4], [0, top - 4], [8.4, top + 0.4]]), "red-600", 1, INKED),
    shape(f.ellipse(-3.6, -17.6, 2.2, 1.4) + f.ellipse(3.6, -17.6, 2.2, 1.4), "slate-900", 0.5),
    shape(f.band([[-8.1, -14.2], [8.1, -14.2]], 0.7) + f.band([[-8.7, 4], [8.7, 4]], 0.7), "wood-900", 0.6),
    shape(chevrons, "wood-900", 0.45),
    shape(f.poly([[4.4, top], [7.4, top], [9.4, 23], [5.4, 23]]), "slate-900", 0.16),
  ];
}

/** A living sluice gate standing in its channel: a wide timber panel (main) about 23 wide from 2 above the shoulders to the ground between two frame posts, three iron bands riveted across it and cracks spurting jets of water. */
function sluiceBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const j = (): number => between(rng, -0.6, 0.6);
  const crackA = between(rng, -7, -2);
  const crackB = between(rng, 2, 7);
  const bands = [S + 2.4, -4, 12];
  const rivets = bands.flatMap((y) => [-9, -3, 3, 9].map((x) => f.circle(x, y, 0.45))).join("");
  return [
    shape(f.band([[-12.6, S - 4.4], [-12.6, 23]], 2.2) + f.band([[12.6, S - 4.4], [12.6, 23]], 2.2), "wood-800", 1, INKED),
    shape(f.poly([[-11.4, S - 2], [11.4, S - 2], [11.4, 23], [-11.4, 23]]), "wood-700", 1, MAIN_INKED),
    shape([-5.7, 0, 5.7].map((x) => f.band([[x, S - 1.6], [x, 22.6]], 0.35)).join(""), "wood-950", 0.45),
    shape(f.poly([[4, S - 2], [11.4, S - 2], [11.4, 23], [4, 23]]), "slate-900", 0.15),
    shape(bands.map((y) => f.band([[-11.6, y], [11.6, y]], 1.6)).join(""), "slate-700", 1, INKED),
    shape(rivets, "slate-400", 1),
    shape(f.band([[crackA, 2], [crackA + 1.2 + j(), 5.6], [crackA - 0.4, 9]], 0.5) + f.band([[crackB, 14.4], [crackB - 1 + j(), 18], [crackB + 0.6, 21]], 0.5), "wood-950", 0.85),
    shape(f.band([[crackA + 0.6, 5.6], [crackA - 3, 7.6], [crackA - 6.4, 12.6 + j()]], 0.9) + f.band([[crackB - 0.6, 18], [crackB + 3.4, 19.4], [crackB + 6.6, 23]], 0.9), "sky-300", 0.85),
    shape(f.circle(crackA - 7, 14.4, 0.6) + f.circle(crackA - 5, 15.8, 0.45) + f.circle(crackB + 7.6, 21.4, 0.5), "blue-200", 0.8),
  ];
}

/** A golem's body of lashed training posts: three upright posts (main) from the shoulders to 6 below the hip, a crossbeam through the shoulders with its end grain showing (about 30 across), rope lashings at the joints and a round practice shield strapped to the chest. */
function postsBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const j = (): number => between(rng, -0.4, 0.4);
  const lash = (x: number, y: number): string => f.band([[x - 1.6, y - 1.4], [x + 1.6, y + 0.4]], 0.6) + f.band([[x - 1.6, y + 0.8], [x + 1.6, y + 2.6]], 0.6);
  const grain = (x: number): string => f.circle(x, S + 1.6, 0.9) + f.circle(x, S + 1.6, 0.45);
  return [
    shape([-4.6, 0, 4.6].map((x) => f.band([[x + j(), S - 1], [x, 6]], 3.6)).join(""), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-13.4, S + 1.6], [13.4, S + 1.6]], 3.4), "wood-600", 1, INKED),
    shape(f.circle(-13.4, S + 1.6, 1.7) + f.circle(13.4, S + 1.6, 1.7), "wood-800", 1, INKED),
    shape(grain(-13.4) + grain(13.4), "brass-500", 0.6),
    shape(f.band([[-7.6, 4.4], [7.6, 4.4]], 2.2), "wood-600", 1, INKED),
    shape(lash(-4.6, S - 0.6) + lash(4.6, S - 0.6) + lash(-4.6, 2.4) + lash(4.6, 2.4), "brass-500", 1),
    shape(f.circle(0.4, -7.4, 5.2), "blue-700", 1, INKED),
    shape(f.circle(0.4, -7.4, 5.2) + f.circle(0.4, -7.4, 4.2), "slate-400", 1),
    shape(f.circle(0.4, -7.4, 1.3), "slate-100", 1, INKED),
    shape(f.band([[-0.6, -11.8], [-3.4, -9]], 0.6), "white", 0.3),
  ];
}

/** A shredder machine's body: a riveted steel box (main) about 20 wide from the shoulders to 5 below the hip, a hopper on top, a hazard-striped plate and a red gauge, and two saw-toothed blade discs spinning at its front. */
function machineBody(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const spin = between(rng, 0, 18);
  const saw = (cx: number, cy: number, r: number): string => {
    const points: Point[] = [];
    for (let k = 0; k < 20; k += 1) {
      const angle = (spin + k * 18) * DEGREES;
      const reach = k % 2 === 0 ? r : r * 0.78;
      points.push([cx + reach * Math.cos(angle), cy + reach * Math.sin(angle)]);
    }
    return f.poly(points);
  };
  const blur = (cx: number, cy: number, r: number): string => {
    const points: Point[] = [];
    for (let k = 0; k <= 6; k += 1) {
      const angle = (spin - 70 + k * 15) * DEGREES;
      points.push([cx + r * Math.cos(angle), cy + r * Math.sin(angle)]);
    }
    return f.band(points, 0.45);
  };
  const rivets = ([[-8.6, S + 1.4], [8.6, S + 1.4], [-8.6, 3.6], [0, S + 1.4], [-3, 3.6]] as const).map(([x, y]) => f.circle(x, y, 0.5)).join("");
  const stripes = [-6.6, -3.8, -1, 1.8].map((x) => f.poly([[x, 4.2], [x + 1.4, 4.2], [x + 2.6, 1.6], [x + 1.2, 1.6]])).join("");
  return [
    shape(f.poly([[-5.6, S - 5.4], [5.6, S - 5.4], [3.6, S], [-3.6, S]]), "slate-800", 1, INKED),
    shape(f.poly([[-10, S], [10, S], [10, 5], [-10, 5]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[3.6, S], [10, S], [10, 5], [3.6, 5]]), "slate-900", 0.18),
    shape(f.poly([[-7.4, 1.2], [5.6, 1.2], [5.6, 4.6], [-7.4, 4.6]]), "slate-900", 1),
    shape(stripes, "orange-400", 1),
    shape(rivets, "slate-400", 1),
    shape(f.circle(-6, -2.4, 1.6), "red-400", 1, INKED),
    shape(saw(10.6, -9.4, 5) + saw(10.6, 0.6, 3.8), "slate-100", 1, INKED),
    shape(f.circle(10.6, -9.4, 1.4) + f.circle(10.6, 0.6, 1.1), "slate-900", 1),
    shape(blur(10.6, -9.4, 6.4) + blur(10.6, 0.6, 5), "white", 0.35),
  ];
}

/** A face on a thing (a bottle's, a totem's, a gate's, a machine's body): two round eyes about 9 below the neck, their pupils glancing and glinting, blushing cheeks (skin) and a small grin; no skull and no hair. */
function headEyes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glance = between(rng, -0.4, 0.5);
  const grin = between(rng, 0.6, 1.4);
  const y = 8.6;
  return [
    shape(f.ellipse(-2.4, y, 1.7, 2) + f.ellipse(2.4, y, 1.7, 2), "white", 1, INKED),
    shape(f.circle(-2.4 + glance, y + 0.3, 0.95) + f.circle(2.4 + glance, y + 0.3, 0.95), "slate-900", 1),
    shape(f.circle(-2.7 + glance, y - 0.2, 0.3) + f.circle(2.1 + glance, y - 0.2, 0.3), "white", 0.9),
    shape(f.ellipse(-4.6, y + 2.8, 1.1, 0.6) + f.ellipse(4.6, y + 2.8, 1.1, 0.6), "red-300", 0.6, SKIN),
    shape(`M${f.p(-1.6, y + 3)}Q${f.p(0, y + 3 + grin * 2)} ${f.p(1.6, y + 3)}Q${f.p(0, y + 3 + grin)} ${f.p(-1.6, y + 3)}Z`, "slate-900", 1),
  ];
}

/** A training dummy's head: a burlap sack (skin) stuffed round and tied off at the neck with a rope, a seam stitched down it, one button eye and one stitched cross, straw poking from its crown. */
function headSack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const lop = between(rng, -0.8, 0.8);
  return [
    shape(f.poly([[-2.4, 1.6], [-1.6, -1.2], [1.6, -1.2], [2.4, 1.6], [0.8, 0.8], [0, 1.8], [-0.8, 0.8]]), "brass-600", 1, SKIN_INKED),
    shape(f.poly([[-0.8, c - 4.6], [-1.6 + lop, c - 7.8], [-0.2, c - 5.6], [0.6 + lop, c - 8.4], [1, c - 5.4], [2.6 + lop, c - 7.2], [1.8, c - 4.4]]), "brass-500", 1, INKED),
    shape(`M${f.p(-4.8, c + 2.6)}C${f.p(-6.2, c - 4)} ${f.p(-2, c - 6.6 + lop * 0.4)} ${f.p(0.6, c - 5.6)}C${f.p(4.4, c - 6.4)} ${f.p(6, c - 2)} ${f.p(4.8, c + 2.6)}Q${f.p(0, c + 4.6)} ${f.p(-4.8, c + 2.6)}Z`, "brass-600", 1, SKIN_INKED),
    shape(f.poly([[1.6, c - 5.4], [4.6, c - 3], [4.8, c + 2.6], [2.4, c + 3.6]]), "slate-900", 0.18),
    shape(f.band([[0.6, c - 5.6], [0.1, c - 1], [0.4, c + 3.6]], 0.35), "wood-800", 0.7),
    shape(f.band([[-2.6, 0.2], [2.6, 0.2]], 1) + f.band([[1.6, 0.8], [2.4, 2.8]], 0.5), "wood-800", 1, INKED),
    shape(f.circle(-2.2, c - 0.6, 1.1) + f.circle(-2.5, c - 0.8, 0.22) + f.circle(-1.9, c - 0.4, 0.22), "slate-700", 1),
    shape(f.band([[1.8, c - 1.6], [3.6, c + 0.2]], 0.45), "slate-900", 1),
    shape(f.band([[3.6, c - 1.6], [1.8, c + 0.2]], 0.45), "slate-900", 1),
  ];
}

/** An ogre's big head (skin, grey hide), about 14 wide: small pointed ears, a heavy brow shading small dark eyes, a broad flat nose and an underbite with two tusks jutting up. */
function headOgre(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE - 0.6;
  const scowl = between(rng, 0, 0.8);
  const tusk = between(rng, 1.6, 2.4);
  return [
    shape(f.band([[0, 2], [0, -1.6]], 4.6), "grey-500", 1, SKIN),
    shape(f.poly([[-6.2, c - 0.6], [-9, c - 3.4], [-6.4, c + 1.6]]) + f.poly([[6.2, c - 0.6], [9, c - 3.4], [6.4, c + 1.6]]), "grey-500", 1, SKIN_INKED),
    shape(
      `M${f.p(-6.6, c + 1)}C${f.p(-7.4, c - 7.4)} ${f.p(7.4, c - 7.4)} ${f.p(6.6, c + 1)}C${f.p(7, c + 5.6)} ${f.p(3.6, c + 6.6)} ${f.p(0, c + 6.6)}C${f.p(-3.6, c + 6.6)} ${f.p(-7, c + 5.6)} ${f.p(-6.6, c + 1)}Z`,
      "grey-500",
      1,
      SKIN_INKED,
    ),
    shape(`M${f.p(-6, c - 1.4)}Q${f.p(0, c - 3.6 + scowl)} ${f.p(6, c - 1.4)}L${f.p(5.6, c - 0.2)}Q${f.p(0, c - 1.8 + scowl)} ${f.p(-5.6, c - 0.2)}Z`, "slate-900", 0.45),
    shape(f.circle(-2.4, c + 0.4, 0.65) + f.circle(2.4, c + 0.4, 0.65), "slate-900", 1),
    shape(f.circle(-2.6, c + 0.2, 0.22) + f.circle(2.2, c + 0.2, 0.22), "white", 0.8),
    shape(f.ellipse(0, c + 2.4, 1.8, 1.1), "slate-900", 0.3),
    shape(f.band([[-3.6, c + 4.4], [0, c + 4.9], [3.6, c + 4.4]], 0.5), "slate-900", 0.8),
    shape(f.poly([[-3.2, c + 4.6], [-2.2, c + 4.6 - tusk], [-1.8, c + 4.8]]) + f.poly([[1.8, c + 4.8], [2.2, c + 4.6 - tusk], [3.2, c + 4.6]]), "white", 1, INKED),
    shape(f.ellipse(-3, c - 4, 2.2, 0.9), "white", 0.18),
  ];
}

/** Spiky hair: seven stiff spikes of different lengths standing out round the crown over a jagged fringe. */
function hairSpiky(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const rim = HEAD_R + 0.2;
  const points: Point[] = [[-5, c + 0.5]];
  for (let k = 0; k < 7; k += 1) {
    const tip = (196 + k * 24.7) * DEGREES;
    const valley = (208 + k * 24.7) * DEGREES;
    const reach = rim + between(rng, 2.6, 5);
    points.push([reach * Math.cos(tip), c + reach * Math.sin(tip)]);
    if (k < 6) points.push([(rim - 0.4) * Math.cos(valley), c + (rim - 0.4) * Math.sin(valley)]);
  }
  points.push([5, c + 0.5], [3.6, c - 1.4], [2.2, c - 2.8], [0.8, c - 1.8], [-0.8, c - 3], [-2.4, c - 1.8], [-3.8, c - 2.6]);
  return [
    shape(f.poly(points), "wood-800", 1, INKED),
    shape(f.band([[-2.6, c - 4.4], [-1, c - 5.6], [1.4, c - 5.4]], 0.5), "brass-800", 0.8),
  ];
}

/** A long white beard and moustache (slate-100) from the cheeks to about 9 below the chin, wavy at its point, bushy white hair round the crown and two bushy brows: a hermit's or an old master's. */
function beard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sway = between(rng, -0.8, 0.8);
  const length = between(rng, 8, 10.5);
  return [
    shape(
      `M${f.p(-5.6, c + 1.4)}C${f.p(-7, c - 3)} ${f.p(-5, c - 7.4)} ${f.p(0, c - 6.6)}C${f.p(5, c - 7.4)} ${f.p(7, c - 3)} ${f.p(5.6, c + 1.4)}Q${f.p(4.6, c - 2.4)} ${f.p(2.4, c - 3.4)}Q${f.p(0, c - 2)} ${f.p(-2.4, c - 3.4)}Q${f.p(-4.6, c - 2.4)} ${f.p(-5.6, c + 1.4)}Z`,
      "slate-100",
      1,
      INKED,
    ),
    shape(
      `M${f.p(-4.6, c + 0.6)}Q${f.p(-5.6, c + 5.6)} ${f.p(-3.2 + sway, c + length)}L${f.p(-1.4 + sway, c + length - 1.6)}L${f.p(sway, c + length + 2)}L${f.p(1.4 + sway, c + length - 1.4)}L${f.p(3.4 + sway, c + length + 0.4)}Q${f.p(5.6, c + 5.6)} ${f.p(4.6, c + 0.6)}Q${f.p(0, c + 2.6)} ${f.p(-4.6, c + 0.6)}Z`,
      "slate-100",
      1,
      INKED,
    ),
    shape(f.band([[-2.6, c + 4], [-2.2 + sway, c + length - 1.4]], 0.3) + f.band([[0.4, c + 4.6], [0.6 + sway, c + length]], 0.3) + f.band([[2.8, c + 4], [2.8 + sway, c + length - 1]], 0.3), "slate-400", 0.7),
    shape(`M${f.p(0, c + 2.2)}Q${f.p(-2.4, c + 1.4)} ${f.p(-4, c + 3.4)}Q${f.p(-2, c + 3)} ${f.p(0, c + 3.2)}Q${f.p(2, c + 3)} ${f.p(4, c + 3.4)}Q${f.p(2.4, c + 1.4)} ${f.p(0, c + 2.2)}Z`, "slate-400", 1, INKED),
    shape(f.band([[-3, c - 1.2], [-1, c - 1.6]], 0.7) + f.band([[1, c - 1.6], [3, c - 1.2]], 0.7), "slate-100", 1, INKED),
  ];
}

/** A fur-lined winter cap with ear flaps (green-900): a padded crown, a turned-up fur brim (stone-500) across the brow, a flap over each ear furred along its edge and a button on top. */
function flapCap(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flap = between(rng, -0.6, 0.6);
  const ear = (side: number): string => f.poly([[side * 4.4, c - 1.6], [side * 6.2, c - 1.2], [side * (6.4 + flap * 0.3), c + 4.6 + flap], [side * 4.6, c + 5.2 + flap], [side * 3.8, c + 1]]);
  const fur = (side: number): string => f.band([[side * 6.3, c - 0.6], [side * (6.5 + flap * 0.3), c + 4.8 + flap]], 1.1);
  return [
    shape(ear(-1) + ear(1), "green-900", 1, INKED),
    shape(fur(-1) + fur(1), "stone-500", 1),
    shape(`M${f.p(-5.4, c - 1.6)}C${f.p(-5.8, c - 8.4)} ${f.p(5.8, c - 8.4)} ${f.p(5.4, c - 1.6)}Z`, "green-900", 1, INKED),
    shape(f.band([[-2.4, c - 6.4], [-0.6, c - 4.4]], 0.4) + f.band([[2.4, c - 6.4], [0.6, c - 4.4]], 0.4), "slate-900", 0.3),
    shape(f.band([[-5.8, c - 2.2], [0, c - 3], [5.8, c - 2.2]], 1.8), "stone-500", 1, INKED),
    shape(f.circle(-3.4, c - 2.4, 0.3) + f.circle(-1.2, c - 2.9, 0.3) + f.circle(1.4, c - 2.9, 0.3) + f.circle(3.8, c - 2.4, 0.3), "wood-700", 0.5),
    shape(f.circle(0, c - 7.3, 0.9), "green-900", 1, INKED),
  ];
}

/** An oversized scholar's cap: a flat black board about 20 across, seen a little from above, over a skullcap, a red tassel hanging from its button over one side, tufts of short hair at the temples. */
function scholarCap(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tilt = between(rng, -0.8, 0.8);
  const swing = between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-5.2, c + 0.6], [-4.4, c - 1.8], [-3.4, c + 0.2]]) + f.poly([[5.2, c + 0.6], [4.4, c - 1.8], [3.4, c + 0.2]]), "wood-800", 1, INKED),
    shape(`M${f.p(-5, c - 1.2)}C${f.p(-5.2, c - 5.6)} ${f.p(5.2, c - 5.6)} ${f.p(5, c - 1.2)}Q${f.p(0, c - 2.4)} ${f.p(-5, c - 1.2)}Z`, "slate-900", 1, INKED),
    shape(f.poly([[-10, c - 5.4 + tilt], [-0.2, c - 3], [10.4, c - 5.6 - tilt], [10.4, c - 4.8 - tilt], [-0.2, c - 2.2], [-10, c - 4.6 + tilt]]), "slate-700", 1, INKED),
    shape(f.poly([[-10, c - 5.4 + tilt], [0.6, c - 8.6], [10.4, c - 5.6 - tilt], [-0.2, c - 3]]), "slate-900", 1, INKED),
    shape(f.poly([[-6, c - 5.6], [0.4, c - 7.6], [1.4, c - 7.2], [-5, c - 5]]), "white", 0.15),
    shape(f.band([[0.2, c - 5.8], [6.4, c - 5], [7.6 + swing, c - 1.6]], 0.4), "red-600", 1),
    shape(f.poly([[7 + swing, c - 2], [8.2 + swing, c - 2], [8.6 + swing, c + 1.6], [6.6 + swing, c + 1.6]]), "red-600", 1, INKED),
    shape(f.circle(0.2, c - 5.8, 0.7), "red-600", 1, INKED),
  ];
}

/** Short dark hair under a red sweatband round the brow, its knot's two tails flying at the back. */
function sweatband(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const flutter = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-5, c - 0.6)}C${f.p(-5.4, c - 7)} ${f.p(5.4, c - 7)} ${f.p(5, c - 0.6)}Q${f.p(2, c - 3.4)} ${f.p(0, c - 3.2)}Q${f.p(-2.4, c - 3)} ${f.p(-5, c - 0.6)}Z`, "wood-800", 1, INKED),
    shape(f.poly([[-1.4, c - 5.4], [-0.4, c - 7.4], [0.4, c - 5.6], [1.6, c - 7], [1.8, c - 5]]), "wood-800", 1, INKED),
    shape(f.poly([[-5, c - 1.6], [-8, c - 0.2 + flutter], [-7.6, c + 1 + flutter], [-5, c - 0.6]]) + f.poly([[-5, c - 0.4], [-7.2, c + 2.4 - flutter], [-6.4, c + 3.2 - flutter], [-4.6, c + 0.4]]), "red-600", 1, INKED),
    shape(f.band([[-5.1, c - 1.2], [0, c - 2.4], [5.1, c - 1.2]], 1.5), "red-600", 1, INKED),
    shape(f.band([[-4, c - 1.6], [-1, c - 2.3]], 0.35), "white", 0.4),
  ];
}

/** Round wire spectacles over short hair: two faintly blue lenses in dark rims, a bridge and arms back to the ears, a glint on one lens. */
function glasses(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const part = between(rng, -1, 1);
  const y = c + 0.2;
  const rims = [-1.9, 1.9].map((x) => f.circle(x, y, 1.5) + f.circle(x, y, 1.1)).join("");
  return [
    shape(`M${f.p(-5, c + 0.5)}C${f.p(-5.4, c - 6.6)} ${f.p(5.4, c - 6.6)} ${f.p(5, c + 0.5)}Q${f.p(2, c - 2.6)} ${f.p(part, c - 2.4)}Q${f.p(-2.4, c - 2.2)} ${f.p(-5, c + 0.5)}Z`, "wood-800", 1, INKED),
    shape(f.circle(-1.9, y, 1.5) + f.circle(1.9, y, 1.5), "sky-300", 0.35),
    shape(rims, "slate-900", 1),
    shape(f.band([[-0.5, y - 0.3], [0.5, y - 0.3]], 0.35) + f.band([[-3.3, y - 0.2], [-4.9, y - 0.8]], 0.35) + f.band([[3.3, y - 0.2], [4.9, y - 0.8]], 0.35), "slate-900", 1),
    shape(f.ellipse(-2.3, y - 0.5, 0.45, 0.3), "white", 0.8),
  ];
}

/** A fur-lined army parka (main), puffy and quilted from the shoulders to 7 below the hip: a fur ruff (stone-500) round the neck and along its hem, a zip and two flapped pockets; about 19 wide. */
function parka(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const puff = between(rng, 0, 0.8);
  const hem = 7;
  const ruff: Point[] = [];
  for (let k = 0; k <= 10; k += 1) {
    const x = -6.4 + k * 1.28;
    ruff.push([x, S - 0.8 + Math.abs(x) * 0.12 - (k % 2 === 0 ? 0.8 : 0)]);
  }
  for (let k = 10; k >= 0; k -= 1) {
    const x = -6.4 + k * 1.28;
    ruff.push([x * 0.8, S + 2.6 - Math.abs(x) * 0.18 + (k % 2 === 0 ? 0.6 : 0)]);
  }
  const trim: Point[] = [[-8.6, hem - 1.4], [8.6, hem - 1.4]];
  for (let x = 8.6; x >= -8.6; x -= 1.72) trim.push([x, hem + 0.4], [x - 0.86, hem + 1.4]);
  const flap = (x: number): string => f.poly([[x, -0.4], [x + 4, -0.4], [x + 4, 0.8], [x + 2, 1.6], [x, 0.8]]);
  return [
    shape(`M${f.p(-7.6, S + 0.4)}L${f.p(7.6, S + 0.4)}Q${f.p(9.4 + puff, -4)} ${f.p(8.4, hem)}L${f.p(-8.4, hem)}Q${f.p(-9.4 - puff, -4)} ${f.p(-7.6, S + 0.4)}Z`, "green-700", 1, MAIN_INKED),
    shape(f.band([[-8, S + 6.4], [8, S + 6.4]], 0.4) + f.band([[-8.6, -5.4], [8.6, -5.4]], 0.4) + f.band([[-8.8, -1.6], [8.8, -1.6]], 0.4), "slate-900", 0.22),
    shape(f.poly([[3, S + 0.6], [7.6, S + 0.4], [9.2 + puff, -4], [8.4, hem], [4, hem]]), "slate-900", 0.15),
    shape(f.poly([[-6.6, -0.4], [-2.6, -0.4], [-2.6, 4.4], [-6.6, 4.4]]) + f.poly([[2.6, -0.4], [6.6, -0.4], [6.6, 4.4], [2.6, 4.4]]), "slate-900", 0.22),
    shape(flap(-6.6) + flap(2.6), "green-700", 1, MAIN_INKED),
    shape(f.band([[0, S + 2.4], [0, hem - 0.6]], 0.5), "slate-400", 0.9),
    shape(f.poly(trim), "stone-500", 1, INKED),
    shape(f.poly(ruff), "stone-500", 1, INKED),
    shape(f.band([[-4.4, S + 1.4], [-3.6, S + 0.2]], 0.3) + f.band([[-1.2, S + 2], [-0.6, S + 0.6]], 0.3) + f.band([[2.4, S + 1.8], [3, S + 0.4]], 0.3), "wood-700", 0.45),
  ];
}

/** A spiked leather vest (main) open over a grey shirt from the shoulders to 2 below the hip: steel spikes bristling along its shoulders and sides, studs down its front and a belt with a buckle; about 16 wide, 22 with its spikes. */
function spikedVest(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const j = (): number => between(rng, -0.3, 0.3);
  const spike = (x: number, y: number, dx: number, dy: number): string => f.poly([[x - dy * 0.5, y + dx * 0.5], [x + dx, y + dy], [x + dy * 0.5, y - dx * 0.5]]);
  const spikes = [
    spike(-3.4, S, j(), -2.4),
    spike(3.4, S, j(), -2.4),
    spike(-6.2, S + 0.4, -0.9, -2.2),
    spike(6.2, S + 0.4, 0.9, -2.2),
    spike(-7.6, S + 3, -2.3, -0.6 + j()),
    spike(7.6, S + 3, 2.3, -0.6 + j()),
    spike(-7.6, -8, -2.3, 0),
    spike(7.6, -8, 2.3, 0),
    spike(-7.4, -3, -2.2, 0.3),
    spike(7.4, -3, 2.2, 0.3),
  ].join("");
  const panel = (side: number): string => f.poly([[side * 7.2, S], [side * 2.4, S], [side * 1.6, -6], [side * 2.6, 2], [side * 7, 2]]);
  return [
    shape(f.poly([[-7, S], [7, S], [6.6, 2], [-6.6, 2]]), "slate-400", 1, INKED),
    shape(spikes, "slate-100", 1, INKED),
    shape(panel(-1) + panel(1), "wood-700", 1, MAIN_INKED),
    shape([-4.8, -4.4, -4, 4.8, 4.4, 4].map((x, k) => f.circle(x, S + 3 + (k % 3) * 5, 0.45)).join(""), "slate-400", 1),
    shape(f.band([[-6.8, 0.8], [6.8, 0.8]], 1.4), "wood-900", 1, INKED),
    shape(f.poly([[-1.2, -0.2], [1.2, -0.2], [1.2, 1.8], [-1.2, 1.8]]), "slate-400", 1, INKED),
    shape(f.poly([[3.6, S + 0.4], [7.2, S], [7, 2], [3.4, 2]]), "slate-900", 0.18),
  ];
}

/** A tree-trunk club about 22 long, swelling from the fist to a knotted head, a broken branch stub and bark lines. */
function trunkClub(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 4, 16);
  const poly = (points: readonly Point[]): string => f.poly(points.map((p) => turned(p, tilt)));
  return [
    shape(poly([[-1, 4], [1, 4], [2.6, -10], [3.4, -17], [0.4, -19.4], [-3.2, -17.6], [-2.8, -10]]), "wood-700", 1, INKED),
    shape(poly([[0.6, 2], [1.2, 2], [2.4, -10], [3, -16], [1.8, -16.4], [1.4, -10]]), "wood-900", 0.35),
    shape(poly([[2.4, -12], [5, -14.6], [5.4, -13.6], [2.8, -10.6]]), "wood-600", 1, INKED),
    shape(f.circle(...turned([-0.6, -14.8], tilt), 0.9) + f.circle(...turned([0.6, -8.4], tilt), 0.6), "wood-900", 0.7),
  ];
}

/** A broad war axe on a short haft, its bearded blade about 7 tall, a chain swinging from the pommel in three links. */
function axe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -12, 6);
  const swing = between(rng, -1.4, 1.4);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const link = (x: number, y: number): string => f.ellipse(...T([x, y]), 0.7, 1);
  return [
    shape(f.band([T([0, 4]), T([0, -15])], 0.8), "wood-800", 1, INKED),
    shape(poly([[0.6, -15.4], [3.4, -17.8], [6, -15.8], [6.4, -11.6], [5, -8.6], [3.2, -11.2], [0.6, -11.6]]), "slate-400", 1, INKED),
    shape(poly([[4.8, -16.8], [6, -15.8], [6.4, -11.6], [5, -8.6], [5.4, -11.8]]), "white", 0.55),
    shape(link(swing * 0.4, 5.6) + link(swing * 0.8, 7.8) + link(swing * 1.2, 10), "slate-700", 1, INKED),
  ];
}

/** A grey boulder for a fist, about 11 across, cracked and chipped, a lighter top. */
function boulderFist(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = between(rng, 4.8, 5.8);
  return [
    shape(f.poly([[-r, 0.4], [-r * 0.7, -r * 0.8], [r * 0.1, -r * 1.05], [r * 0.9, -r * 0.6], [r, r * 0.4], [r * 0.4, r], [-r * 0.6, r * 0.9]]), "stone-500", 1, INKED),
    shape(f.poly([[-r * 0.6, -r * 0.6], [r * 0.1, -r * 0.9], [r * 0.7, -r * 0.5], [r * 0.1, -r * 0.3]]), "slate-100", 0.3),
    shape(f.band([[-r * 0.3, -r * 0.2], [r * 0.1, r * 0.3], [r * 0.5, r * 0.2]], 0.3), "slate-900", 0.6),
  ];
}

/** A thin willow wand about 13 long, a fork at its tip and teal sparks spun off it. */
function wand(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 14, 30);
  const T = (p: Point): Point => turned(p, tilt);
  const [tx, ty] = T([0, -13]);
  return [
    shape(f.band([T([0, 2]), T([0.4, -7]), T([0, -13])], 0.45), "wood-600", 1, INKED),
    shape(f.band([T([0.2, -8]), T([1.8, -10])], 0.25), "green-500", 1),
    shape(f.circle(tx, ty, 2.2), "cyan-400", 0.3),
    shape(sparkle(f, tx, ty, 1.6) + sparkle(f, tx + 3, ty + 1.6, 0.9) + sparkle(f, tx - 2.4, ty - 2.6, 0.8), "cyan-400", 1),
  ];
}

/** A long ladle, its bowl about 4 across at the end of a handle 12 below the fist, held out to serve, soup steaming in it. */
function ladle(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 50, 70);
  const T = (p: Point): Point => turned(p, tilt);
  const [bx, by] = T([0, -12]);
  return [
    shape(f.band([T([0, 1.6]), T([0, -10.6])], 0.5), "slate-400", 1, INKED),
    shape(`M${f.p(bx - 2.4, by)}Q${f.p(bx, by + 4)} ${f.p(bx + 2.4, by)}Z`, "slate-400", 1, INKED),
    shape(`M${f.p(bx - 2, by + 0.2)}Q${f.p(bx, by + 1)} ${f.p(bx + 2, by + 0.2)}Z`, "orange-400", 1),
    shape(f.band([[bx - 0.6, by - 1], [bx + 0.4, by - 3], [bx - 0.4, by - 5]], 0.35), "white", 0.4),
  ];
}

/** A small leather coin purse dangling from the fist on its drawstring, a coin peeking from its mouth. */
function purse(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  const body = `M${f.p(sway - 1.2, 3)}Q${f.p(sway - 3.8, 4.4)} ${f.p(sway - 3, 7.4)}Q${f.p(sway, 9.2)} ${f.p(sway + 3, 7.4)}Q${f.p(sway + 3.8, 4.4)} ${f.p(sway + 1.2, 3)}Z`;
  return [
    shape(f.band([[0, 0], [sway, 3]], 0.25), "wood-900", 1),
    shape(body, "wood-600", 1, INKED),
    shape(f.band([[sway - 1.4, 3.2], [sway + 1.4, 3.2]], 0.5), "red-600", 1),
    shape(f.circle(sway + 0.6, 2.4, 0.9), "slate-400", 1, INKED),
  ];
}

/** A calligraphy brush about 11 long held up and forward, its tuft dipped black at the tip. */
function brush(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 20, 40);
  const poly = (points: readonly Point[]): string => f.poly(points.map((p) => turned(p, tilt)));
  return [
    shape(f.band([turned([0, 3], tilt), turned([0, -7], tilt)], 0.55), "wood-700", 1, INKED),
    shape(poly([[-0.8, -7], [0.8, -7], [0.5, -9.6], [0, -11.2], [-0.5, -9.6]]), "white", 1, INKED),
    shape(poly([[-0.5, -9.6], [0.5, -9.6], [0, -11.2]]), "black", 1),
  ];
}

/** A boat oar held two-handed for stirring, its shaft from 10 above the fist to its broad blade 20 below it. */
function oar(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 14, 26);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  return [
    shape(f.band([T([0, -10]), T([0, 13])], 0.8), "wood-600", 1, INKED),
    shape(poly([[-0.8, 13], [0.8, 13], [2.2, 16], [2, 21], [0, 22.4], [-2, 21], [-2.2, 16]]), "wood-700", 1, INKED),
    shape(poly([[0.2, 14], [0.8, 14], [1.8, 16.4], [1.6, 20.6], [0.2, 21.4]]), "wood-900", 0.3),
  ];
}

/** A long thin pointer stick about 18 long, raised up and forward to tap a board, its tip chalk-white. */
function pointer(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 40, 56);
  const T = (p: Point): Point => turned(p, tilt);
  return [
    shape(f.band([T([0, 2]), T([0, -17])], 0.4), "wood-700", 1, INKED),
    shape(f.band([T([0, -16]), T([0, -18.4])], 0.45), "white", 1),
    shape(f.circle(...T([0, 2.4]), 0.6), "wood-900", 1),
  ];
}

/** A jousting lance held upright, about 32 tall, a cone-shaped hand guard at the fist and a painted spiral up the shaft. */
function lance(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -4, 4);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  return [
    shape(poly([[-1.2, 6], [1.2, 6], [0.3, -26], [-0.3, -26]]), "white", 1, INKED),
    shape(poly([[-1, 0], [1, -3], [0.9, -5], [-0.9, -2]]) + poly([[-0.8, -10], [0.8, -13], [0.7, -15], [-0.7, -12]]) + poly([[-0.6, -20], [0.5, -22.6], [0.4, -24], [-0.5, -21.6]]), "red-600", 1),
    shape(poly([[-0.8, -2], [0.8, -2], [3, 2], [-3, 2]]), "slate-400", 1, INKED),
  ];
}

/** R3401000: an iron hand lantern hanging from the hand by its bail, a flame behind its bars and an orange glow about it; about 4 wide and 9 tall. */
function handLantern(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flicker = between(rng, -0.3, 0.3);
  return [
    shape(f.circle(0, 5.6, 6), "orange-400", 0.22),
    shape(`M${f.p(-1.6, 2.6)}Q${f.p(0, -1.4)} ${f.p(1.6, 2.6)}L${f.p(1.1, 2.6)}Q${f.p(0, -0.4)} ${f.p(-1.1, 2.6)}Z`, "slate-900", 1),
    shape(f.poly([[-2.2, 2.4], [2.2, 2.4], [2.6, 3.4], [-2.6, 3.4]]) + f.poly([[-2.4, 8.2], [2.4, 8.2], [1.8, 9.2], [-1.8, 9.2]]), "slate-700", 1, INKED),
    shape(f.poly([[-1.9, 3.4], [1.9, 3.4], [1.9, 8.2], [-1.9, 8.2]]), "orange-400", 0.95, INKED),
    shape(`M${f.p(flicker, 4.2)}Q${f.p(1, 5.8)} ${f.p(0, 7)}Q${f.p(-1, 5.8)} ${f.p(flicker, 4.2)}Z`, "white", 0.9),
    shape(f.poly([[-0.8, 3.4], [-0.45, 3.4], [-0.45, 8.2], [-0.8, 8.2]]) + f.poly([[0.45, 3.4], [0.8, 3.4], [0.8, 8.2], [0.45, 8.2]]), "slate-900", 0.85),
  ];
}

/** R3401000: a phone held up in the hand, its screen lit cyan and glowing, a red record light at its top; about 3.6 wide and 6.8 tall. */
function phone(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -14, -4);
  const poly = (points: readonly Point[]): string => f.poly(points.map((p) => turned(p, tilt)));
  const [gx, gy] = turned([0, -4], tilt);
  return [
    shape(f.circle(gx, gy, 4.6), "sky-300", 0.22),
    shape(poly([[-1.8, -7.6], [1.8, -7.6], [1.8, 0.4], [-1.8, 0.4]]), "slate-900", 1, INKED),
    shape(poly([[-1.3, -7], [1.3, -7], [1.3, -0.4], [-1.3, -0.4]]), "cyan-400", 0.9),
    shape(poly([[-0.9, -6.2], [0.4, -6.2], [0.4, -5.5], [-0.9, -5.5]]) + poly([[-0.9, -4.6], [0.9, -4.6], [0.9, -3.9], [-0.9, -3.9]]), "white", 0.6),
    shape(f.circle(...turned([1.1, -7.1], tilt), 0.3), "red-400", 1),
  ];
}

/** R3401000: a large iron key held up by its ring bow, its shaft about 12 long, its toothed bit at the top in a faint jade glow. */
function ironKey(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 10, 24);
  const T = (p: Point): Point => turned(p, tilt);
  const [bx, by] = T([0, -12]);
  return [
    shape(f.circle(bx, by, 3.6), "jade-400", 0.22),
    shape(f.circle(...T([0, 0.6]), 2) + f.circle(...T([0, 0.6]), 1.1), "slate-700", 1, INKED),
    shape(f.band([T([0, -1.4]), T([0, -13])], 1), "slate-700", 1, INKED),
    shape(f.poly([T([0.4, -13]), T([3, -13]), T([3, -11.8]), T([2, -11.8]), T([2, -10.8]), T([0.4, -10.8])]) + f.poly([T([0.4, -9.8]), T([2.4, -9.8]), T([2.4, -8.8]), T([0.4, -8.8])]), "slate-700", 1, INKED),
    shape(f.band([T([-0.2, -2]), T([-0.2, -12])], 0.25), "slate-100", 0.45),
  ];
}

/** R3401000: a conductor's thin white baton raised from the hand, its cork grip in the fist; about 10 long. */
function baton(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 20, 40);
  const T = (p: Point): Point => turned(p, tilt);
  return [
    shape(f.band([T([0, -0.4]), T([0, -10.4])], 0.45), "white", 1, INKED),
    shape(f.ellipse(...T([0, 0.4]), 0.9, 1.5), "wood-700", 1, INKED),
    shape(f.circle(...T([0, -10.8]), 1.1), "white", 0.3),
  ];
}

/** R3401000: a calligraphy brush held upright, a bamboo handle with nodes, a white ferrule and a black ink-loaded tip below the fist; about 10 long. */
function calligraphyBrush(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 6);
  const T = (p: Point): Point => turned(p, tilt);
  const [tx, ty] = T([0, 6.8]);
  return [
    shape(f.band([T([0, -6.4]), T([0, 2.4])], 0.9), "wood-600", 1, INKED),
    shape(f.band([T([-0.5, -3.6]), T([0.5, -3.6])], 0.35) + f.band([T([-0.5, -0.6]), T([0.5, -0.6])], 0.35), "wood-800", 0.9),
    shape(f.poly([T([-0.6, 2.4]), T([0.6, 2.4]), T([0.6, 3.2]), T([-0.6, 3.2])]), "slate-100", 1, INKED),
    shape(`M${f.p(...T([-0.8, 3.2]))}Q${f.p(...T([-1, 5.4]))} ${f.p(tx, ty)}Q${f.p(...T([1, 5.4]))} ${f.p(...T([0.8, 3.2]))}Z`, "slate-900", 1, INKED),
    shape(f.circle(tx + 0.4, ty + 1.6, 0.45), "slate-900", 0.8),
  ];
}

/** R3401000: a fishing rod held out and up from the fist, slender and bowed, a reel by the grip and its line hanging from the tip; the rod about 22 long, the line about 18. */
function fishingRod(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bow = between(rng, 0.6, 1.6);
  const drop = between(rng, 17, 19);
  const sway = between(rng, -0.6, 0.6);
  return [
    shape(f.band([[0, 0], [6, -5 - bow * 0.6], [12, -9 - bow * 0.4], [18, -12]], 0.5), "wood-700", 1, INKED),
    shape(f.band([[-2.6, 3.4], [0.4, -0.4]], 1.2), "wood-900", 1, INKED),
    shape(f.circle(1.6, 1.2, 0.9), "slate-700", 1, INKED),
    shape(f.band([[18, -12], [18.4 + sway, -12 + drop * 0.5], [18.8, -12 + drop]], 0.18), "slate-100", 0.85),
  ];
}

/** R3401000: a pocket comb raised in the fist to the hair, its spine and a row of teeth; about 6.4 long. */
function pocketComb(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -24, -10);
  const T = (p: Point): Point => turned(p, tilt);
  const spine: Point[] = [[-3.2, -0.6], [3.2, -0.6], [3.2, 0.8], [-3.2, 0.8]];
  const teeth = Array.from({ length: 7 }, (_, k) => f.band([T([-2.7 + k * 0.9, 0.6]), T([-2.7 + k * 0.9, 2.2])], 0.32)).join("");
  return [
    shape(f.poly(spine.map(T)), "slate-900", 1, INKED),
    shape(teeth, "slate-900", 1),
    shape(f.band([T([-2.8, -0.1]), T([2.8, -0.1])], 0.25), "slate-400", 0.7),
  ];
}

/** R3401000: a jeweller's loupe raised to the eye-side of the hand and a small torch in the fist casting a pale beam forward and down. */
function loupe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const aim = between(rng, -0.8, 0.8);
  return [
    shape(f.poly([[3, -1.8], [12, 0.6 + aim], [11, 5 + aim], [3, -0.2]]), "white", 0.3),
    shape(f.band([[-1.4, 0.6], [3, -1]], 1.4), "slate-700", 1, INKED),
    shape(f.circle(3.2, -1, 0.7), "white", 0.9),
    shape(f.circle(-0.6, -4.6, 1.7) + f.circle(-0.6, -4.6, 1), "slate-900", 1, INKED),
    shape(f.circle(-0.6, -4.6, 1), "sky-300", 0.55),
  ];
}

/** R3401000: a folding fan opened from its pivot in the hand: pale paper ribs over a jade wash along its edge, its guards dark; about 14 across. */
function foldingFan(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -12, 12);
  const R = 7.4;
  const arc = (r: number): Point[] => Array.from({ length: 9 }, (_, k) => turned([0, -r], tilt - 60 + k * 15));
  const outer = arc(R);
  const inner = arc(R * 0.62);
  const base = arc(1.2);
  const ribs = [1, 3, 5, 7].map((k) => f.band([base[k] ?? [0, 0], outer[k] ?? [0, 0]], 0.2)).join("");
  return [
    shape(f.poly([...base, ...[...outer].reverse()]), "slate-100", 1, INKED),
    shape(f.poly([...inner, ...[...outer].reverse()]), "jade-400", 0.45),
    shape(ribs, "slate-900", 0.35),
    shape(f.band([[0, 0.8], base[0] ?? [0, 0], outer[0] ?? [0, 0]], 0.5) + f.band([[0, 0.8], base[8] ?? [0, 0], outer[8] ?? [0, 0]], 0.5), "wood-800", 1),
    shape(f.circle(0, 0.6, 0.6), "brass-600", 1),
  ];
}

/** R3401000: twin curved blades from the fist, one sweeping up forward, the other back over the shoulder, steel with dark grips. */
function twinBlades(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flare = between(rng, -0.8, 0.8);
  const forward = `M${f.p(0.4, -1.4)}Q${f.p(6, -7.4)} ${f.p(3.4 + flare, -14.6)}Q${f.p(3, -7.8)} ${f.p(-0.6, -1.4)}Z`;
  const back = `M${f.p(-0.4, -1.6)}Q${f.p(-2.4, -11.4)} ${f.p(-10.2, -11.6 - flare)}Q${f.p(-5.2, -6)} ${f.p(-1.4, -0.6)}Z`;
  return [
    shape(forward + back, "slate-100", 1, INKED),
    shape(`M${f.p(1.6, -3)}Q${f.p(4.8, -7.6)} ${f.p(3.4 + flare, -14.6)}Q${f.p(3.8, -8)} ${f.p(1.6, -3)}Z`, "sky-300", 0.55),
    shape(f.band([[0, -1.6], [0.2, 1.8]], 1) + f.band([[-0.6, -1.4], [-1.8, 1.4]], 1), "wood-800", 1, INKED),
    shape(f.band([[-1.6, -1.6], [1.8, -1.4]], 0.6), "brass-600", 1),
  ];
}

// R3403000: the Stinkyscape scenes' parts (#3403): four headgears and hairs, seven outfits and six held items its
// briefs name. Its pointing pose and hoodie are #3392's and its headset #3401's (`headphones`); its sleeveless vest,
// plated parka and shock-axe stand beside #3393's spiked vest, fur parka and war axe under ids of their own.

/** A one-piece flight suit (main) from the shoulders to 3 below the hip: a high collar, a zip, harness straps down the chest and across it and the waist with buckles, a round shoulder patch and a shaded side. */
function flightsuit(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const hem = between(rng, 2.4, 3.4);
  const lift = between(rng, -0.3, 0.3);
  const square = (cx: number, cy: number): string => f.poly([[cx - 0.8, cy - 0.8], [cx + 0.8, cy - 0.8], [cx + 0.8, cy + 0.8], [cx - 0.8, cy + 0.8]]);
  return [
    shape(f.poly([[-7.2, S], [7.2, S], [6.6, -1], [7, hem], [-7, hem], [-6.6, -1]]), "orange-400", 1, MAIN_INKED),
    shape(f.poly([[4.2, S], [7.2, S], [6.6, -1], [7, hem], [4.6, hem]]), "slate-900", 0.16),
    shape(f.poly([[-2.8, S - 0.8], [2.8, S - 0.8], [2.2, S + 1.4], [-2.2, S + 1.4]]), "slate-900", 0.35),
    shape(f.band([[0.4, S + 1.4], [0.4, hem - 0.6]], 0.35), "slate-100", 0.8),
    shape(f.band([[-3.6, S + 0.2], [-3.2, hem - 0.4]], 1.1) + f.band([[3.6, S + 0.2], [3.2, hem - 0.4]], 1.1), "slate-800", 1, INKED),
    shape(f.band([[-6.7, S + 6.4], [6.7, S + 6.4]], 1) + f.band([[-6.7, -1], [6.7, -1]], 1.2), "slate-800", 1, INKED),
    shape(square(-3.4, S + 6.4) + square(3.4, S + 6.4) + square(-3.3, -1) + square(3.3, -1), "slate-400", 1, INKED),
    shape(f.circle(-5.4, S + 3 + lift, 1.3), "red-600", 1, INKED),
    shape(f.circle(-5.4, S + 3 + lift, 0.55), "white", 0.85),
  ];
}

/** A long greatcoat (main) to the shins, 18 below the hip, split up the front to 3 below it: a high standing collar, silver epaulettes (slate-100) with fringes, two rows of buttons down its double-breasted front, a belt and a shaded side. */
function greatcoat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const flare = between(rng, 9.4, 10.6);
  const split = between(rng, 0.8, 1.8);
  const hem = HEM - 4.5;
  let fringe = "";
  for (const side of [-1, 1]) for (let k = 0; k < 5; k += 1) fringe += f.band([[side * (4.4 + k * 0.95), S + 1.2], [side * (4.5 + k), S + 2.9]], 0.4);
  const buttons = [S + 4.2, S + 8, S + 11.8].map((y) => f.circle(0.8, y, 0.5) + f.circle(5.2, y, 0.5)).join("");
  const wing = (side: number): string => f.poly([[side * 1.8, S + 1], [side * 4.8, S - 0.2], [side * 4.4, S - 3.2], [side * 2, S - 2]]);
  return [
    shape(f.poly([[-7.4, S], [7.4, S], [6.8, -1], [flare, hem], [split, hem], [0, 3], [-split, hem], [-flare, hem], [-6.8, -1]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[4, S], [7.4, S], [6.8, -1], [flare, hem], [split + 2, hem], [3, 3]]), "slate-900", 0.16),
    shape(wing(-1) + wing(1), "slate-700", 1, MAIN_INKED),
    shape(f.poly([[-2, S - 1.6], [-4.2, S - 2.8], [-4.4, S - 0.6]]) + f.poly([[2, S - 1.6], [4.2, S - 2.8], [4.4, S - 0.6]]), "slate-900", 0.35),
    shape(f.band([[3, S + 2], [3, -1.6]], 0.35), "slate-900", 0.4),
    shape(f.band([[-6.9, -1], [6.9, -1]], 1.4), "slate-900", 1, INKED),
    shape(f.poly([[2.2, -2], [3.8, -2], [3.8, 0], [2.2, 0]]), "slate-400", 1),
    shape(fringe, "slate-100", 0.9),
    shape(f.ellipse(-6.4, S + 0.4, 2.6, 1.1) + f.ellipse(6.4, S + 0.4, 2.6, 1.1), "slate-100", 1, INKED),
    shape(buttons, "slate-100", 1),
  ];
}

/** A sleeveless vest (main) open over a bare muscled chest (skin): its two front panels to 3 below the hip leave the round shoulders bare and show the pecs and abs between them, over a belt with a brass buckle. */
function sleevelessVest(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const open = between(rng, 2.6, 3.4);
  let abs = f.band([[0, S + 6.6], [0, -2.6]], 0.45);
  for (const y of [-9, -6, -3.6]) abs += f.band([[-2.8, y + 0.3], [-0.5, y]], 0.4) + f.band([[0.5, y], [2.8, y + 0.3]], 0.4);
  const panel = (side: number): string =>
    f.poly([[side * 5.8, S - 0.6], [side * (open + 0.6), S - 0.4], [side * open, -6], [side * (open + 0.4), 3], [side * 7.2, 3], [side * 7, S + 6.6], [side * 5.2, S + 3.4]]);
  const trim = (side: number): string => f.band([[side * (open + 0.8), S - 0.2], [side * (open + 0.2), -6], [side * (open + 0.6), 2.6]], 0.45);
  return [
    shape(`M${f.p(-7.6, S + 1.4)}Q${f.p(-7.6, S - 0.8)} ${f.p(-5.4, S - 0.8)}L${f.p(5.4, S - 0.8)}Q${f.p(7.6, S - 0.8)} ${f.p(7.6, S + 1.4)}L${f.p(6.6, -1)}L${f.p(-6.6, -1)}Z`, "red-300", 1, SKIN_INKED),
    shape(f.ellipse(-7.6, S + 1.4, 2.2, 2.5) + f.ellipse(7.6, S + 1.4, 2.2, 2.5), "red-300", 1, SKIN_INKED),
    shape(f.band([[-5.6, S + 4], [-3.4, S + 5.8], [-0.5, S + 5.2]], 0.45) + f.band([[0.5, S + 5.2], [3.4, S + 5.8], [5.6, S + 4]], 0.45) + abs, "slate-900", 0.3),
    shape(f.ellipse(-3.4, S + 3.4, 2, 1.1) + f.ellipse(-7.9, S + 0.6, 1, 0.8), "white", 0.22),
    shape(panel(-1) + panel(1), "wood-700", 1, MAIN_INKED),
    shape(trim(-1) + trim(1), "slate-900", 0.35),
    shape(f.band([[-7, -1.6], [7, -1.6]], 1.4), "wood-800", 1, INKED),
    shape(f.poly([[-1, -2.5], [1, -2.5], [1, -0.7], [-1, -0.7]]), "brass-500", 1),
  ];
}

/** A puffy parka (main) to 5 below the hip, quilted in bulging rolls: its hood's white fur ruff about the neck, white fur at its hem, bolted armour plates (slate-700) on its chest and far shoulder and toggles down its front; about 20 wide. */
function platedParka(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const puff = between(rng, 0.6, 1.2);
  const hem = 4.4;
  const ruff: Point[] = [];
  for (let k = 0; k <= 12; k += 1) {
    const [x, y] = turned([-6.4 * (k % 2 === 0 ? 1 : 1.1), 0], k * 15);
    ruff.push([x, S + 1.4 + y * 0.8]);
  }
  for (let k = 12; k >= 0; k -= 1) {
    const [x, y] = turned([-3.8, 0], k * 15);
    ruff.push([x, S + 1.4 + y * 0.75]);
  }
  const fur: Point[] = [];
  for (let k = 0; k <= 12; k += 1) fur.push([-9 + k * 1.5, hem - (k % 2 === 0 ? 1.4 : 2)]);
  for (let k = 12; k >= 0; k -= 1) fur.push([-9 + k * 1.5, hem + (k % 2 === 0 ? 1.6 : 1)]);
  const body = `M${f.p(-7.6, S)}L${f.p(7.6, S)}Q${f.p(9 + puff, S + 4)} ${f.p(8.2, -8.4)}Q${f.p(9.4 + puff, -4)} ${f.p(8.6, hem)}L${f.p(-8.6, hem)}Q${f.p(-9.4 - puff, -4)} ${f.p(-8.2, -8.4)}Q${f.p(-9 - puff, S + 4)} ${f.p(-7.6, S)}Z`;
  const bolts: readonly Point[] = [[2, -13.8], [5.9, -14.3], [6.4, -9.4], [2.5, -8.7], [-8.6, S + 2.3], [-8.3, S + 4.2]];
  return [
    shape(body, "blue-800", 1, MAIN_INKED),
    shape([-12.6, -8.4, -4.2, 0].map((y) => f.band([[-8, y], [0, y + 0.6], [8, y]], 0.45)).join(""), "slate-900", 0.25),
    shape(f.poly([[3.6, S], [7.6, S], [8.6, hem], [5, hem]]), "slate-900", 0.15),
    shape(f.poly([[1.2, -14.6], [6.6, -15.2], [7.2, -8.6], [1.8, -7.8]]) + f.poly([[-9.4, S + 1.8], [-6.6, S + 0.6], [-5.6, S + 4], [-9, S + 5]]), "slate-700", 1, INKED),
    shape(f.band([[1.5, -14.3], [6.4, -14.8]], 0.45) + f.band([[-9, S + 1.9], [-6.7, S + 0.9]], 0.4), "white", 0.3),
    shape(bolts.map(([x, y]) => f.circle(x, y, 0.4)).join(""), "slate-400", 1),
    shape([-11, -6.4, -1.8].map((y) => f.poly([[-1.2, y - 0.4], [0.2, y - 0.4], [0.2, y + 0.4], [-1.2, y + 0.4]])).join(""), "wood-700", 1),
    shape(f.poly(fur), "white", 1, INKED),
    shape(f.poly(ruff), "white", 1, INKED),
  ];
}

/** A frock coat (main) fitted to the waist, its skirts cut away in front to tails 12 below the hip and one tail swinging out behind: a red-900 waistcoat with brass buttons between dark lapels, and a white cravat falling in tiers of lace frills. */
function frock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const swing = between(rng, 0, 1.6);
  const tail = 12.4;
  const frill = (y: number, w: number): string => {
    const points: Point[] = [[-w, y]];
    for (let k = 0; k <= 6; k += 1) points.push([-w + (k * 2 * w) / 6, y + (k % 2 === 0 ? 1.1 : 0.6)]);
    points.push([w, y]);
    return f.poly(points);
  };
  return [
    shape(f.poly([[-5.6, -1], [-9.4 - swing, tail + 1.8], [-6.6 - swing * 0.4, tail + 1.2]]), "blue-900", 1, MAIN_INKED),
    shape(f.poly([[-7, S], [7, S], [6.4, -1.4], [8.4, tail], [5.4, tail + 0.6], [2.2, 0.4], [-2.2, 0.4], [-5.4, tail + 0.6], [-8.4, tail], [-6.4, -1.4]]), "blue-900", 1, MAIN_INKED),
    shape(f.poly([[4.2, S], [7, S], [6.4, -1.4], [8.4, tail], [5.4, tail + 0.6], [3.4, 0]]), "slate-900", 0.16),
    shape(f.poly([[-2.6, S + 1.6], [2.6, S + 1.6], [2.4, -0.2], [0, 1], [-2.4, -0.2]]), "red-900", 1, INKED),
    shape([-8.6, -5.6, -2.6].map((y) => f.circle(0, y, 0.45)).join(""), "brass-500", 1),
    shape(f.poly([[-2.4, S], [-4.4, S + 1.2], [-2.5, -6]]) + f.poly([[2.4, S], [4.4, S + 1.2], [2.5, -6]]), "slate-900", 0.4),
    shape(f.poly([[-1.8, S - 0.6], [1.8, S - 0.6], [1.1, S + 1.2], [1.7, S + 5.6], [0, S + 6.8], [-1.7, S + 5.6], [-1.1, S + 1.2]]), "white", 1, INKED),
    shape(frill(S + 2.2, 1.3) + frill(S + 3.6, 1.6) + frill(S + 5, 1.9), "slate-100", 0.95),
  ];
}

/** A long cloak (main) from the shoulders to the feet, clasped at the neck over a dark tunic: solid above a ragged line about 2 below the hip, a shimmering translucent stealth below it to the hem with cyan streaks and glints where it fades; about 23 wide at the hem. */
function cloak(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const fade = (side: number): Point[] => [0, 1, 2, 3, 4, 5].map((k): Point => [side * (3.2 + k * 1.3), 2 + between(rng, -1.4, 1.4)]);
  const left = fade(-1);
  const right = fade(1);
  const upper = (side: number, edge: readonly Point[]): string => f.poly([[side * 0.8, S - 0.6], [side * 7.6, S - 0.4], [side * 8.6, S + 6], ...[...edge].reverse(), [side * 1.6, S + 6]]);
  const lower = (side: number, edge: readonly Point[]): string => f.poly([...edge, [side * 11.4, HEM], [side * 8, HEM - 0.6], [side * 4.4, HEM]]);
  const glints = [...left, ...right]
    .filter((_, k) => k % 2 === 1)
    .map(([x, y]) => f.poly([[x, y + 0.8], [x + 0.6, y + 1.4], [x, y + 2], [x - 0.6, y + 1.4]]))
    .join("");
  return [
    shape(f.poly([[-4.4, S + 1], [4.4, S + 1], [3.8, 3], [-3.8, 3]]), "slate-800", 1, INKED),
    shape(lower(-1, left) + lower(1, right), "slate-700", 0.42, MAIN),
    shape(f.band([[-5.6, 5], [-6, HEM - 2]], 0.5) + f.band([[-8.6, 5.6], [-9.6, HEM - 2.4]], 0.4) + f.band([[5.8, 4.6], [6.4, HEM - 2]], 0.5) + f.band([[8.8, 5.2], [9.8, HEM - 2.6]], 0.4), "cyan-400", 0.25),
    shape(upper(-1, left) + upper(1, right), "slate-700", 1, MAIN_INKED),
    shape(f.band([[-5, S + 3], [-6.4, 0]], 0.4) + f.band([[5.2, S + 3], [6.6, 0]], 0.4), "slate-900", 0.3),
    shape(glints, "cyan-400", 0.6),
    shape(f.circle(0, S - 0.4, 1.1), "brass-500", 1, INKED),
  ];
}

/** A lab harness over a shirt (main) to 2 below the hip: slate-800 straps over the shoulders, across the chest and round the waist, two tool pouches on the belt with a screwdriver tucked by one, and a glowing power cell (cyan-400) on the chest. */
function labHarness(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const S = SHOULDER;
  const glow = between(rng, 0.16, 0.26);
  const lean = between(rng, -0.4, 0.4);
  const y = S + 6;
  return [
    shape(f.poly([[-7, S], [7, S], [6.4, 2.2], [-6.4, 2.2]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-2.4, S], [2.4, S], [0, S + 2.4]]), "slate-900", 0.35),
    shape(f.band([[5.4 + lean, -3.2], [5.2, 1.6]], 0.5), "slate-400", 1),
    shape(f.band([[-3.8, S + 0.2], [-3.4, 0]], 1.1) + f.band([[3.8, S + 0.2], [3.4, 0]], 1.1), "slate-800", 1, INKED),
    shape(f.band([[-6.7, y], [6.7, y]], 1) + f.band([[-6.5, 0.4], [6.5, 0.4]], 1.4), "slate-800", 1, INKED),
    shape(f.poly([[-6.2, 1.2], [-3.6, 1.2], [-3.8, 4.4], [-6, 4.4]]) + f.poly([[4, 1.2], [6.4, 1.2], [6.2, 3.8], [4.2, 3.8]]), "wood-700", 1, INKED),
    shape(f.poly([[-6.2, 1.2], [-3.6, 1.2], [-3.7, 2.2], [-6.1, 2.2]]) + f.poly([[4, 1.2], [6.4, 1.2], [6.3, 2], [4.1, 2]]), "slate-900", 0.3),
    shape(f.circle(0, y, 3.6), "cyan-400", glow),
    shape(f.circle(0, y, 2.4), "cyan-400", glow),
    shape(f.poly([[-1.6, y - 2], [1.6, y - 2], [1.6, y + 2], [-1.6, y + 2]]), "slate-700", 1, INKED),
    shape(f.poly([[-1, y - 1.4], [1, y - 1.4], [1, y + 1.4], [-1, y + 1.4]]), "cyan-400", 1),
    shape(f.band([[-0.4, y - 1], [-0.4, y + 1]], 0.4), "white", 0.8),
  ];
}


/** A glowing circlet of nodes (cyan-400) worn across the brow over swept-back grey hair: a dark band, five lit nodes with a glow about the middle three, and pale strands combed back. */
function neuralCrown(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const sweep = between(rng, 0, 1);
  const glow = between(rng, 0.25, 0.35);
  const band: Point[] = [[-4.9, c - 1.4], [-2.6, c - 2.5], [0, c - 2.8], [2.6, c - 2.5], [4.9, c - 1.4]];
  const nodes = (r: number, from: number, to: number): string =>
    band
      .slice(from, to)
      .map(([x, y]) => f.circle(x, y, r))
      .join("");
  return [
    shape(`M${f.p(-5.4 - sweep, c + 2.4)}C${f.p(-6.6 - sweep, c - 6.8)} ${f.p(4, c - 7.6)} ${f.p(5.1, c - 1.6)}Q${f.p(2.4, c - 3.4)} ${f.p(0, c - 3.4)}Q${f.p(-3.4, c - 2.8)} ${f.p(-4.4, c + 1.8)}Z`, "slate-400", 1, INKED),
    shape(f.band([[3.8, c - 2.9], [0.6, c - 3.9], [-3.4, c - 3.4], [-4.9 - sweep * 0.6, c - 0.6]], 0.4) + f.band([[2.4, c - 4.3], [-0.6, c - 4.7], [-2.8, c - 4]], 0.4), "slate-100", 0.55),
    shape(f.band(band, 2.4), "cyan-400", 0.14),
    shape(f.band(band, 0.8), "slate-700", 1, INKED),
    shape(nodes(1.2, 1, 4), "cyan-400", glow),
    shape(nodes(0.55, 0, 5), "cyan-400", 1),
    shape(nodes(0.25, 1, 4), "white", 0.9),
  ];
}

/** A round glass bubble helmet over the whole head: a translucent pale globe, a rim of light along its lit side and a glint, sat on a metal collar ring with bolts at the neck. */
function spaceHelmet(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const glint = between(rng, -0.4, 0.4);
  const [cy, R, y0] = [c + 0.2, 7.1, 0.3];
  const x0 = Math.sqrt(R ** 2 - (y0 - cy) ** 2);
  const rim: Point[] = [];
  for (let k = 0; k <= 6; k += 1) {
    const [x, y] = turned([0, 0.9 - R], 20 + k * 16);
    rim.push([x, cy + y]);
  }
  return [
    shape(`M${f.p(-x0, y0)}A${fmt(f.r(R))} ${fmt(f.r(R))} 0 1 ${f.sweep(1)} ${f.p(x0, y0)}Z`, "sky-300", 0.22, INKED),
    shape(f.band(rim, 0.7), "white", 0.3),
    shape(f.ellipse(0, 1.2, 5.6, 1.5), "slate-700", 1, INKED),
    shape(f.ellipse(0, 0.5, 5.6, 1.4), "slate-400", 1, INKED),
    shape(f.circle(-3.4, 0.8, 0.35) + f.circle(0, 1.2, 0.35) + f.circle(3.4, 0.8, 0.35), "slate-100", 0.9),
    shape(oval(f, -3.4 + glint, cy - 3.8, 1.7, 0.8, -40), "white", 0.7),
    shape(f.circle(-1.2 + glint, cy - 5.6, 0.45), "white", 0.85),
  ];
}

/** Short neat silver hair (slate-400) swept back from a side part over the near brow, a pale sheen (slate-100) combed through it and the parting line. */
function hairSilver(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const part = between(rng, 0.8, 1.8);
  const lift = between(rng, 0, 0.6);
  return [
    shape(`M${f.p(-5.1, c + 0.8)}C${f.p(-5.6, c - 6.8 - lift)} ${f.p(5.4, c - 7 - lift)} ${f.p(5.1, c + 0.2)}Q${f.p(4.4, c - 2.6)} ${f.p(part, c - 3)}Q${f.p(-2.6, c - 2.6)} ${f.p(-5.1, c + 0.8)}Z`, "slate-400", 1, INKED),
    shape(f.band([[part - 0.6, c - 3.5], [-1.6, c - 4.2], [-4.2, c - 2.2]], 0.55) + f.band([[part + 0.6, c - 3.7], [3.6, c - 3.4], [4.6, c - 1.6]], 0.5), "slate-100", 0.85),
    shape(f.band([[part, c - 3.1], [part - 0.5, c - 4.9]], 0.3), "slate-700", 0.5),
  ];
}

/** A tall black top hat over a curled brim about 15 across, a plum band (violet-700) and a sheen down its lit side, and a monocle: a brass ring over the near eye on a fine chain falling to the chest. */
function topHat(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = -HEAD_RISE;
  const tall = between(rng, 10, 11.4);
  const lean = between(rng, -0.05, 0.05);
  const L = (x: number, y: number): Point => [x - lean * (y - c + 4), y];
  return [
    shape(f.poly([L(-4.2, c - 4.2), L(4.2, c - 4.2), L(4.6, c - 4.2 - tall), L(-4.6, c - 4.2 - tall)]), "black", 1, INKED),
    shape(f.poly([L(-4.2, c - 4.4), L(4.2, c - 4.4), L(4.3, c - 6.2), L(-4.3, c - 6.2)]), "violet-700", 1),
    shape(f.band([L(2.9, c - 6.6), L(3.3, c - 3.8 - tall)], 0.9), "slate-700", 0.8),
    shape(`M${f.p(-7.4, c - 3.6)}Q${f.p(-7.2, c - 5.2)} ${f.p(-4.6, c - 4.6)}L${f.p(4.6, c - 4.6)}Q${f.p(7.2, c - 5.2)} ${f.p(7.4, c - 3.6)}Q${f.p(0, c - 2.6)} ${f.p(-7.4, c - 3.6)}Z`, "black", 1, INKED),
    shape(f.circle(1.6, c + 0.2, 1.05), "white", 0.2),
    shape(f.circle(1.6, c + 0.2, 1.45) + f.circle(1.6, c + 0.2, 1.05), "brass-500", 1),
    shape(f.band([[2.9, c + 0.9], [3.7, c + 3.4], [3.3, c + 6.4], [2.2, c + 8]], 0.3), "brass-500", 0.9),
  ];
}

/** A shock-axe held upright: a haft from 4 below the hand to 16 above it, a steel head jutting forward at its top and a spike behind, a cyan arc crackling along its edge in a glow, sparks at its ends. */
function shockAxe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, 4, 14);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const j = (): number => between(rng, -0.5, 0.5);
  const raw: Point[] = [[5.8, -16.6], [6.9 + j(), -15], [6.1, -13.4 + j() * 0.4], [7.2 + j(), -12], [6.2, -10.6], [7 + j(), -9.2]];
  const arc = raw.map(T);
  return [
    shape(f.band([T([0, 4]), T([0, -15.6])], 1), "wood-700", 1, INKED),
    shape(f.band([T([0, 2.6]), T([0, 4])], 1.3), "slate-900", 1),
    shape(poly([[-0.4, -14.6], [-2.8, -13.3], [-0.4, -12.2]]), "slate-700", 1, INKED),
    shape(poly([[0.4, -15], [3.4, -16.2], [5.6, -17.2], [6.2, -13.2], [5.6, -9.2], [3.4, -10.2], [0.4, -11.4]]), "slate-400", 1, INKED),
    shape(f.band([T([5.4, -16.6]), T([5.9, -13.2]), T([5.4, -9.8])], 0.6), "slate-100", 0.9),
    shape(f.band(arc, 1.8), "cyan-400", 0.22),
    shape(f.band(arc, 0.45), "cyan-400", 1),
    shape(sparkle(f, ...T([7.6, -17.4]), 1.1) + sparkle(f, ...T([7.6, -8.4]), 0.9), "white", 0.9),
  ];
}

/** A small venom dart pinched between two fingertips (skin) just above the hand, its needle pointing forward and a little up: white flights behind, a green-400 barrel, a needle beaded with venom and a drip falling from it. */
function dart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -24, -10);
  const drip = between(rng, 1.4, 2.4);
  const T = (p: Point): Point => {
    const [x, y] = turned(p, tilt);
    return [0.8 + x, -1.6 + y];
  };
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const [tx, ty] = T([4.4, 0]);
  return [
    shape(poly([[-2.2, -0.3], [-4.4, -1.5], [-3.8, -0.3]]) + poly([[-2.2, 0.3], [-4.4, 1.5], [-3.8, 0.3]]), "white", 0.9),
    shape(f.band([T([-4, 0]), T([-1.6, 0])], 0.35), "slate-400", 1),
    shape(poly([[-2, -0.6], [1.4, -0.5], [2, 0], [1.4, 0.5], [-2, 0.6]]), "green-400", 1, INKED),
    shape(f.band([T([2, 0]), T([4.6, 0])], 0.3), "slate-100", 1),
    shape(f.circle(tx, ty + 0.3, 0.45) + f.band([[tx, ty + 0.85], [tx, ty + drip - 0.15]], 0.35) + f.circle(tx, ty + drip + 0.4, 0.5), "green-400", 1),
    shape(f.band([[-0.8, -0.5], T([-0.6, -1])], 1.1) + f.ellipse(...T([-0.2, 1]), 0.75, 0.6), "red-300", 1, SKIN),
    shape(oval(f, ...T([-0.4, -0.3]), 1, 0.18, tilt), "white", 0.6),
  ];
}

/** An overcharged orb of raw power held up above the hand: a violet-300 and a cyan-400 glow, the orb with a white-hot core, two tilted rings of light about it and sparks; about 12 across its glow, 5 the orb. */
function orb(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const [cx, cy] = [0.8, -4.2];
  const tilt = between(rng, -30, -10);
  const pulse = between(rng, 0.9, 1.1);
  const spin = between(rng, 0, 120) * DEGREES;
  const ring = (rx: number, ry: number, degrees: number, w: number): string => oval(f, cx, cy, rx, ry, degrees) + oval(f, cx, cy, rx - w, ry - w * 0.6, degrees);
  let sparks = "";
  for (let k = 0; k < 3; k += 1) {
    const a = spin + (k * 2 * Math.PI) / 3;
    sparks += sparkle(f, cx + 5.2 * Math.cos(a), cy + 5.2 * Math.sin(a), between(rng, 0.9, 1.3));
  }
  return [
    shape(f.circle(cx, cy, 6.2 * pulse), "violet-300", 0.12),
    shape(f.circle(cx, cy, 4.4 * pulse), "cyan-400", 0.2),
    shape(f.circle(cx, cy, 2.6), "cyan-400", 0.85),
    shape(f.circle(cx - 0.3, cy - 0.3, 1.4), "white", 0.95),
    shape(ring(4.4, 1.5, tilt, 0.45), "cyan-400", 0.55),
    shape(ring(4, 1.3, tilt + 70, 0.4), "violet-300", 0.5),
    shape(sparks, "white", 0.9),
  ];
}

/** A datapad held up with its lower corner in the fist: a slate frame about 6 wide and 9 tall, a dark screen showing a lit header and a short list (two or three boxes with blank lines), a glint and the thumb over its edge. */
function slatePad(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -8, 4);
  const T = (p: Point): Point => turned(p, tilt);
  const rect = (x0: number, y0: number, x1: number, y1: number): string => f.poly([T([x0, y0]), T([x1, y0]), T([x1, y1]), T([x0, y1])]);
  const rows = rng() < 0.5 ? 2 : 3;
  let boxes = "";
  let lines = "";
  for (let k = 0; k < rows; k += 1) {
    const y = -4.8 + k * 1.6;
    boxes += rect(0.6, y - 0.35, 1.3, y + 0.35);
    lines += f.band([T([1.9, y]), T([1.9 + between(rng, 1.4, 2.6), y])], 0.4);
  }
  return [
    shape(rect(-0.6, -8, 5.6, 0.8), "slate-700", 1, INKED),
    shape(rect(0, -7.4, 5, 0.2), "slate-900", 1),
    shape(rect(0.5, -6.8, 3.4, -6), "cyan-400", 0.9),
    shape(boxes, "jade-400", 0.85),
    shape(lines, "slate-400", 0.8),
    shape(f.poly([T([3, -7.4]), T([5, -7.4]), T([5, -6.2]), T([1.4, 0.2]), T([0.2, 0.2])]), "white", 0.1),
    shape(f.ellipse(...T([-0.2, -0.4]), 0.9, 0.7), "red-300", 1, SKIN),
  ];
}

/** A welding torch held forward: a hose looping down from its grip, a slate body and a brass nozzle, a blue jet of flame (sky-300, a white core) about 7 long from its tip in a glow, and sparks spraying off it. */
function weldTorch(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -10, 12);
  const jet = between(rng, 5.6, 7.4);
  const T = (p: Point): Point => turned(p, tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const x0 = 6.2;
  const flame = (half: number, length: number): string =>
    poly([[x0, -0.4 - half], [x0 + length * 0.5, -0.4 - half], [x0 + length, -0.4], [x0 + length * 0.5, -0.4 + half], [x0, -0.4 + half]]);
  const [ox, oy] = [x0 + jet, -0.4];
  let sparks = "";
  for (let k = 0; k < 4; k += 1) {
    const a = (-50 + k * 33 + between(rng, -8, 8)) * DEGREES;
    const [d0, d1] = [1.2 + 0.4 * k, 1.2 + 0.4 * k + between(rng, 1.2, 2.2)];
    sparks += f.band([T([ox + d0 * Math.cos(a), oy + d0 * Math.sin(a)]), T([ox + d1 * Math.cos(a), oy + d1 * Math.sin(a)])], 0.3);
  }
  return [
    shape(f.band([T([-2, 0.4]), T([-3.4, 2.6]), T([-2.8, 5.6]), T([-1, 7.4])], 0.9), "slate-900", 1),
    shape(f.band([T([-2.4, 0]), T([3, -0.4])], 1.8), "slate-400", 1, INKED),
    shape(f.band([T([-1.2, -0.95]), T([-1.2, 0.85])], 0.3) + f.band([T([0, -1.05]), T([0, 0.75])], 0.3) + f.band([T([1.2, -1.15]), T([1.2, 0.65])], 0.3), "slate-700", 0.7),
    shape(poly([[3, -0.9], [x0, -0.7], [x0, -0.1], [3, 0.1]]), "brass-500", 1, INKED),
    shape(flame(1.2, jet + 1.4), "sky-300", 0.2),
    shape(flame(0.5, jet), "sky-300", 0.85),
    shape(flame(0.22, jet * 0.6), "white", 0.95),
    shape(sparks, "orange-400", 0.9),
    shape(f.circle(...T([ox + 0.4, oy]), 0.5), "white", 0.9),
  ];
}

/** A pocket vial snapped open, held upright in the fist: a glass tube about 7 tall with a broken lip, jade liquid (jade-400) in it foaming over the top, bubbles fizzing up to about 8 above it and the snapped-off tip tumbling away. */
function vial(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -8, 8);
  const T = (p: Point): Point => turned(p, lean);
  const poly = (points: readonly Point[]): string => f.poly(points.map(T));
  const fizz: readonly Point[] = [[0.4, -8.4], [-0.9, -10], [1.1, -11.2], [-0.2, -12.8], [1, -14.2]];
  const bubbles = fizz
    .map(([x, y], k) => {
      const [bx, by] = T([x + between(rng, -0.3, 0.3), y]);
      const r = k % 2 === 0 ? 0.65 : 0.45;
      return f.circle(bx, by, r) + f.circle(bx, by, r * 0.55);
    })
    .join("");
  const [fx, fy] = T([0, -6.2]);
  return [
    shape(poly([[-1.1, 1], [1.1, 1], [1.1, -5.2], [0.6, -5.8], [0.2, -5.2], [-0.3, -6], [-1.1, -5.4]]), "slate-100", 0.35, INKED),
    shape(poly([[-0.8, 0.7], [0.8, 0.7], [0.8, -4.6], [-0.8, -4.6]]), "jade-400", 1),
    shape(blob(f, rng, fx, fy, 1.5, 8), "jade-400", 0.85),
    shape(bubbles, "jade-400", 0.75),
    shape(f.band([T([-0.6, -0.2]), T([-0.6, -4.2])], 0.35), "white", 0.5),
    shape(poly([[2.2, -7.6], [3.2, -8.4], [3.6, -7.8], [2.6, -7]]), "slate-100", 0.6, INKED),
  ];
}

/** R3331000: the figure parts, by id, each with its kind. */
export const FIGURE_KIT: Readonly<Record<FigurePartId, KitPart>> = {
  "pose-stand": { category: "pose", draw: poseStand },
  "pose-guard": { category: "pose", draw: poseGuard },
  "pose-raise": { category: "pose", draw: poseRaise },
  "pose-lunge": { category: "pose", draw: poseLunge },
  "pose-stride": { category: "pose", draw: poseStride },
  "pose-perch": { category: "pose", draw: posePerch },
  "pose-genuflect": { category: "pose", draw: poseGenuflect },
  "pose-cast": { category: "pose", draw: poseCast },
  "pose-point": { category: "pose", draw: posePoint },
  "pose-hold": { category: "pose", draw: poseHold },
  "pose-block": { category: "pose", draw: poseBlock },
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
  "wild-goggles": { category: "hair", draw: wildGoggles },
  veil: { category: "hair", draw: veil },
  fedora: { category: "hair", draw: fedora },
  "outfit-tunic": { category: "outfit", draw: tunic },
  "outfit-armor": { category: "outfit", draw: armor },
  "outfit-coat": { category: "outfit", draw: coat },
  "outfit-jacket": { category: "outfit", draw: jacket },
  "outfit-suit": { category: "outfit", draw: suit },
  "outfit-fur": { category: "outfit", draw: fur },
  "outfit-stack": { category: "outfit", draw: stack },
  "outfit-peel": { category: "outfit", draw: peel },
  "outfit-void": { category: "outfit", draw: robe },
  "outfit-layered-plate": { category: "outfit", draw: outfitLayeredPlate },
  "outfit-waistcoat": { category: "outfit", draw: outfitWaistcoat },
  "outfit-sweater": { category: "outfit", draw: outfitSweater },
  "outfit-labcoat": { category: "outfit", draw: outfitLabcoat },
  "outfit-tutor-cardigan": { category: "outfit", draw: outfitTutorCardigan },
  "outfit-bomber": { category: "outfit", draw: outfitBomber },
  "outfit-work-apron": { category: "outfit", draw: outfitWorkApron },
  "outfit-hoodie": { category: "outfit", draw: outfitHoodie },
  "outfit-rags": { category: "outfit", draw: outfitRags },
  "outfit-rock": { category: "outfit", draw: outfitRock },
  "outfit-gown": { category: "outfit", draw: gown },
  "outfit-blazer": { category: "outfit", draw: outfitBlazer },
  "outfit-stripes": { category: "outfit", draw: outfitStripes },
  "outfit-lantern": { category: "outfit", draw: outfitLantern },
  "item-sword": { category: "item", draw: sword },
  "item-shield": { category: "item", draw: shield },
  "item-staff": { category: "item", draw: staff },
  "item-book": { category: "item", draw: book },
  "item-blaster": { category: "item", draw: blaster },
  "item-rapier": { category: "item", draw: rapier },
  "item-knife": { category: "item", draw: knife },
  "item-pavise": { category: "item", draw: pavise },
  "item-silver-coin": { category: "item", draw: silverCoin },
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
  "item-twig-spear": { category: "item", draw: twigSpear },
  "item-broom": { category: "item", draw: broom },
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
  // R3393000: the Meditative+ and Training Grounds scenes' parts.
  "pose-post": { category: "pose", draw: posePost },
  "pose-treads": { category: "pose", draw: poseTreads },
  "pose-lift": { category: "pose", draw: poseLift },
  "pose-hips": { category: "pose", draw: poseHips },
  "head-eyes": { category: "head", draw: headEyes },
  "head-sack": { category: "head", draw: headSack },
  "head-ogre": { category: "head", draw: headOgre },
  "outfit-flask": { category: "outfit", draw: flaskBody },
  "outfit-straw": { category: "outfit", draw: strawBody },
  "outfit-totem": { category: "outfit", draw: totemBody },
  "outfit-sluice": { category: "outfit", draw: sluiceBody },
  "outfit-posts": { category: "outfit", draw: postsBody },
  "outfit-machine": { category: "outfit", draw: machineBody },
  "hair-spiky": { category: "hair", draw: hairSpiky },
  beard: { category: "hair", draw: beard },
  "flap-cap": { category: "hair", draw: flapCap },
  "scholar-cap": { category: "hair", draw: scholarCap },
  sweatband: { category: "hair", draw: sweatband },
  glasses: { category: "hair", draw: glasses },
  "outfit-parka": { category: "outfit", draw: parka },
  "outfit-vest": { category: "outfit", draw: spikedVest },
  "item-trunk-club": { category: "item", draw: trunkClub },
  "item-axe": { category: "item", draw: axe },
  "item-boulder": { category: "item", draw: boulderFist },
  "item-wand": { category: "item", draw: wand },
  "item-ladle": { category: "item", draw: ladle },
  "item-purse": { category: "item", draw: purse },
  "item-brush": { category: "item", draw: brush },
  "item-oar": { category: "item", draw: oar },
  "item-pointer": { category: "item", draw: pointer },
  "item-lance": { category: "item", draw: lance },
  // R3401000: the Meditative scenes' parts, after the others so their order holds.
  headphones: { category: "hair", draw: headphones },
  beanie: { category: "hair", draw: beanie },
  pompadour: { category: "hair", draw: pompadour },
  "conductor-cap": { category: "hair", draw: conductorCap },
  "tall-hat": { category: "hair", draw: tallHat },
  "hair-bun": { category: "hair", draw: hairBun },
  "outfit-overalls": { category: "outfit", draw: outfitOveralls },
  "outfit-kaftan": { category: "outfit", draw: outfitKaftan },
  "outfit-tailcoat": { category: "outfit", draw: outfitTailcoat },
  "outfit-silks": { category: "outfit", draw: outfitSilks },
  "item-lantern": { category: "item", draw: handLantern },
  "item-phone": { category: "item", draw: phone },
  "item-key": { category: "item", draw: ironKey },
  "item-baton": { category: "item", draw: baton },
  "item-ink-brush": { category: "item", draw: calligraphyBrush },
  "item-loupe": { category: "item", draw: loupe },
  "item-fan": { category: "item", draw: foldingFan },
  "item-twin-blades": { category: "item", draw: twinBlades },
  "item-rod": { category: "item", draw: fishingRod },
  "item-comb": { category: "item", draw: pocketComb },
  // R3403000: the Stinkyscape scenes' parts, after the others so their order holds.
  "neural-crown": { category: "hair", draw: neuralCrown },
  "space-helmet": { category: "hair", draw: spaceHelmet },
  "hair-silver": { category: "hair", draw: hairSilver },
  "top-hat": { category: "hair", draw: topHat },
  "outfit-flightsuit": { category: "outfit", draw: flightsuit },
  "outfit-greatcoat": { category: "outfit", draw: greatcoat },
  "outfit-sleeveless": { category: "outfit", draw: sleevelessVest },
  "outfit-plated-parka": { category: "outfit", draw: platedParka },
  "outfit-frock": { category: "outfit", draw: frock },
  "outfit-cloak": { category: "outfit", draw: cloak },
  "outfit-harness": { category: "outfit", draw: labHarness },
  "item-shock-axe": { category: "item", draw: shockAxe },
  "item-dart": { category: "item", draw: dart },
  "item-orb": { category: "item", draw: orb },
  "item-slate": { category: "item", draw: slatePad },
  "item-torch": { category: "item", draw: weldTorch },
  "item-vial": { category: "item", draw: vial },
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
export const KY_TUTOR_LOOK: FigureLook = { head: "head-human", hair: "spectacles", outfit: "outfit-tutor-cardigan", item: "item-chalk", skin: "red-300", cloth: "wood-700" };

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
