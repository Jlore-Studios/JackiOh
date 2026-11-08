// R3391000: Stinkyscape's scene recipes (ART.md, "Stinkyscape's scenes"): each of the 64 cards of #3391's
// table, cards 1 to 90 but the Starship Pieces (#3333's) and Olivian Expanse (#3338's), drawn from its brief
// in the set's deep space, base and Radiant. A person is the figure kit's, starting from its tag's look; a
// ship Unit's character is its own craft part in the subject layer. Each keeps its subject within x 30 to 70
// and y 25 to 85 (a face's window shows about y 18 to 82), stands its emblem on its subject as an insignia,
// and clears its rarity's floor (richness.ts `ART_DETAIL_MIN`): an Epic in depth, a Legendary or Mythic in
// every layer, each Mythic with a part of its own. SCENE_RECIPES (sceneRecipes.ts) spreads them in.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A Human's look with `over` laid on it (a recipe's people start from their tag's look). */
function human(over: Partial<FigureLook>): FigureLook {
  return { ...figureLookFor(["Human"]), ...over };
}

/** Freeze Ray (Stinkyscape #1, Common): a ray gun's pale-blue beam freezes a fighter docked at an orbital marina. */
const FREEZE_RAY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 62, y: 52, r: 30, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 62, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 1.6 }],
    far: [{ part: "dock-gantry", x: 58, y: 70, scale: 1.3 }],
    mid: [
      { part: "fighter", x: 30, y: 60, scale: 0.9 },
      { part: "fighter", x: 88, y: 60, scale: 0.9 },
      { part: "frost-shard", x: 34, y: 60, scale: 0.9 },
      { part: "frost-shard", x: 84, y: 60, scale: 0.9 },
    ],
    subject: [
      { part: "fighter", x: 62, y: 56, scale: 1.5, paint: "slate-100" },
      { part: "frost-shard", x: 60, y: 58, scale: 1.3 },
    ],
    near: [{ part: "ray-gun", x: 22, y: 74, scale: 1.6 }],
    light: [{ part: "beam", x: 31, y: 72, scale: 0.85, rotate: -26, paint: "sky-300" }],
  },
};

/** Absolute Zero (Stinkyscape #2, Rare): a scout sealed in a block of ice drifts past a dim, dying star. */
const ABSOLUTE_ZERO: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 76, y: 28, r: 24, paint: "red-900", opacity: 0.35 },
  emblem: { x: 60, y: 46, size: 5, rotate: -12 },
  layers: {
    sky: [
      { part: "stars", x: 40, y: 22, scale: 1.8 },
      { part: "stars", x: 50, y: 74, scale: 1.6 },
    ],
    far: [
      { part: "star-glare", x: 78, y: 28, scale: 0.8, paint: "red-400" },
      { part: "nebula", x: 22, y: 30, scale: 0.9, paint: "blue-800" },
    ],
    mid: [
      { part: "asteroid", x: 14, y: 74, scale: 0.6 },
      { part: "asteroid", x: 88, y: 70, scale: 0.5 },
    ],
    subject: [
      { part: "scout", x: 50, y: 54, scale: 1.4 },
      { part: "ice-block", x: 50, y: 55, scale: 1.1 },
    ],
    near: [
      { part: "frost-shard", x: 36, y: 70 },
      { part: "frost-shard", x: 66, y: 40, scale: 0.8 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 54, scale: 1.1, paint: "blue-200" },
      { part: "halo", x: 50, y: 55, scale: 1.6, paint: "sky-300" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" }],
  },
};

/** Meteor (Stinkyscape #3, Epic): a burning meteor slams into a parked walker on a red desert's landing pad. */
const METEOR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "orange-400"],
  light: { x: 52, y: 58, r: 36, paint: "orange-400", opacity: 0.55 },
  emblem: { x: 52, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "smoke-trail", x: 6, y: 12, scale: 1.2, rotate: 18 },
      { part: "smoke-trail", x: 62, y: 6, scale: 1, rotate: 32 },
    ],
    far: [{ part: "regolith", x: 50, y: 60, paint: "red-900" }],
    mid: [
      { part: "walker", x: 52, y: 70, scale: 1.2 },
      { part: "wreck", x: 24, y: 68, scale: 0.8 },
      { part: "wreck", x: 80, y: 69, scale: 0.8, flip: -1 },
    ],
    subject: [{ part: "meteor", x: 32, y: 28, scale: 1.2, rotate: 48 }],
    near: [
      { part: "regolith", x: 50, y: 80, paint: "red-600" },
      { part: "debris", x: 28, y: 88, scale: 1.1 },
      { part: "debris", x: 74, y: 90, scale: 1.2 },
    ],
    light: [
      { part: "shockwave", x: 52, y: 66, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 52, y: 58, scale: 2, paint: "ember-500" },
    ],
    particles: [
      { part: "embers", x: 52, y: 60, scale: 1.5, count: 4, spread: [22, 10] },
      { part: "sparks", x: 52, y: 58, count: 2, spread: [10, 6] },
    ],
  },
};

/** Masochism Unmasked (Stinkyscape #5, Rare): a pilot's cracked chrome mask hurls every hit back along a red tether. */
const MASOCHISM_UNMASKED: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 46, y: 48, r: 30, paint: "red-600", opacity: 0.45 },
  emblem: { x: 44, y: 44, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 1.4 }],
    far: [{ part: "fighter", x: 82, y: 30, scale: 0.8, flip: -1, paint: "slate-700" }],
    mid: [{ part: "cockpit-frame", x: 50, y: 74 }],
    subject: [{ part: "cracked-mask", x: 44, y: 50, scale: 1.6 }],
    near: [{ part: "debris", x: 18, y: 82 }],
    light: [
      { part: "light-ribbon", x: 50, y: 46, scale: 0.9, rotate: -28, paint: "red-500" },
      { part: "halo", x: 44, y: 50, scale: 1.4, paint: "red-600" },
    ],
    particles: [
      { part: "sparks", x: 62, y: 40, count: 2, spread: [12, 8] },
      { part: "embers", x: 40, y: 70, count: 2, spread: [14, 6] },
    ],
  },
};

/** Tylannt (Stinkyscape #6, Mythic): a smug thief on his getaway skiff's ramp, a bank station blazing behind him. */
const TYLANNT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-900"],
  light: { x: 26, y: 34, r: 34, paint: "red-500", opacity: 0.4 },
  emblem: { x: 36, y: 52, size: 4, rotate: 0 },
  figure: { x: 36, y: 70, scale: 0.72, pose: "pose-reach", look: human({ outfit: "outfit-coat", item: "item-drill", cloth: "slate-800" }) },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2 },
      { part: "nebula", x: 78, y: 24, paint: "blue-800" },
      { part: "stars", x: 30, y: 62, scale: 1.5 },
    ],
    far: [
      { part: "station", x: 22, y: 34, scale: 1.3 },
      { part: "vault-door", x: 72, y: 26, scale: 0.7 },
      { part: "credit-chips", x: 60, y: 32, count: 3, spread: [10, 6] },
    ],
    mid: [
      { part: "smoke-trail", x: 26, y: 40, scale: 0.9, rotate: 28 },
      { part: "asteroid", x: 10, y: 70, count: 2, spread: [6, 8] },
      { part: "debris", x: 86, y: 58 },
    ],
    subject: [{ part: "skiff", x: 54, y: 62, scale: 1.6 }],
    near: [
      { part: "duffel", x: 31, y: 40, scale: 0.8 },
      { part: "credit-chips", x: 72, y: 84, count: 2, spread: [8, 3] },
      { part: "debris", x: 14, y: 88 },
    ],
    light: [
      { part: "spotlight", x: 22, y: 20, scale: 0.9, rotate: 62, paint: "red-500" },
      { part: "rim-light", x: 36, y: 52, paint: "sky-300" },
      { part: "halo", x: 22, y: 34, scale: 2, paint: "red-600" },
    ],
    particles: [
      { part: "sparks", x: 66, y: 30, count: 2, spread: [10, 6] },
      { part: "embers", x: 24, y: 36, count: 3, spread: [8, 6] },
      { part: "motes", x: 50, y: 60, paint: "sky-300" },
    ],
  },
};

/** The Greatest Heist (Stinkyscape #7, Legendary): the VC vault in a comet's frozen core, its laser grid flickering off. */
const THE_GREATEST_HEIST: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 50, r: 30, paint: "green-400", opacity: 0.35 },
  emblem: { x: 50, y: 66, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 44, y: 10, scale: 1.2 },
      { part: "comet", x: 8, y: 6, scale: 0.7, rotate: 12 },
    ],
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-800" },
      { part: "vault-door", x: 22, y: 44, scale: 1.2 },
      { part: "light-grid", x: 70, y: 40, scale: 0.6, paint: "green-400" },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 66, paint: "slate-800" },
      { part: "crate", x: 72, y: 74, count: 3, spread: [9, 1], paint: "slate-700" },
      { part: "crate", x: 76, y: 66, scale: 0.9, count: 2, spread: [5, 0], paint: "slate-700" },
      { part: "credit-chips", x: 66, y: 76, count: 2, spread: [6, 2] },
    ],
    subject: [
      { part: "glass-case", x: 50, y: 76, scale: 1.3 },
      { part: "crystal", x: 50, y: 70, scale: 1.4, paint: "cyan-400" },
    ],
    near: [
      { part: "rope", x: 38, y: 0, scale: 1.1 },
      { part: "credit-chips", x: 26, y: 86, count: 2, spread: [8, 3] },
    ],
    light: [
      { part: "light-grid", x: 50, y: 52, scale: 1.1, paint: "green-400" },
      { part: "beam", x: 6, y: 34, scale: 1.1, rotate: 14, paint: "green-400" },
      { part: "beam", x: 94, y: 30, scale: 1.1, rotate: 166, paint: "green-400" },
      { part: "halo", x: 50, y: 64, scale: 1.4, paint: "cyan-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, count: 2, spread: [10, 10], paint: "green-200" },
      { part: "sparks", x: 50, y: 62, count: 2, spread: [8, 6] },
    ],
  },
};

/** Conjure Rosemary Triscuit (Stinkyscape #8, Rare): a galley cook conjures a herb-flecked cracker from a rosemary sprig. */
const CONJURE_ROSEMARY_TRISCUIT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "wood-800"],
  light: { x: 58, y: 40, r: 28, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 46, y: 60, size: 4, rotate: 0 },
  figure: { x: 46, y: 86, scale: 0.95, pose: "pose-raise", look: human({ hair: "toque", outfit: "outfit-apron", item: null, cloth: "slate-100" }) },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-700" },
      { part: "porthole", x: 80, y: 30, scale: 0.8 },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 70, paint: "wood-700" }],
    subject: [{ part: "herb-sprig", x: 34, y: 34, scale: 1.2 }],
    near: [
      { part: "crate", x: 76, y: 92, scale: 1.3 },
      { part: "war-drum", x: 74, y: 78, scale: 1 },
      { part: "cracker", x: 60, y: 32, scale: 1.3 },
    ],
    light: [
      { part: "halo", x: 60, y: 32, scale: 1.2, paint: "orange-400" },
      { part: "sparks", x: 48, y: 30, count: 2, spread: [10, 4] },
    ],
  },
};

/** Ticketed (Stinkyscape #9, Common): a small ship clamped under a glowing boot-lock in a grey moon's impound lot. */
const TICKETED: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-800"],
  light: { x: 50, y: 50, r: 30, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 46, y: 56, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.4 }],
    mid: [
      { part: "regolith", x: 50, y: 64 },
      { part: "fence", x: 50, y: 66, scale: 1.2, paint: "slate-700" },
    ],
    subject: [
      { part: "scout", x: 48, y: 60, scale: 1.5 },
      { part: "boot-clamp", x: 52, y: 66, scale: 1 },
    ],
    light: [
      { part: "spotlight", x: 20, y: 12, scale: 0.9, rotate: 60, paint: "slate-100" },
      { part: "halo", x: 54, y: 66, scale: 0.8, paint: "orange-400" },
    ],
  },
};

/** Otherworldly Endurance (Stinkyscape #10, Epic): a gloved hand on a chapel deck receives three prayer-lights from a portal. */
const OTHERWORLDLY_ENDURANCE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 40, r: 32, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 40, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 30, y: 18, scale: 1.4 },
      { part: "stars", x: 72, y: 24, scale: 1.2 },
    ],
    far: [
      { part: "nebula", x: 72, y: 30, scale: 0.9, paint: "violet-500" },
      { part: "planet", x: 16, y: 26, scale: 0.6, paint: "violet-300" },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 70, paint: "slate-800" },
      { part: "rift", x: 50, y: 40, scale: 1.3, paint: "violet-300" },
    ],
    subject: [
      { part: "flame-column", x: 38, y: 52, scale: 0.5 },
      { part: "wisp", x: 50, y: 44, scale: 1.1 },
      { part: "hex-shield", x: 62, y: 46, scale: 0.4, paint: "violet-300" },
    ],
    near: [{ part: "gloved-hand", x: 50, y: 92, scale: 1.8 }],
    light: [
      { part: "halo", x: 50, y: 44, scale: 2, paint: "white" },
      { part: "light-shaft", x: 50, y: 30, count: 2, spread: [8, 0] },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, count: 2, spread: [8, 8], paint: "violet-300" }],
  },
};

/** Otherworldly Magic (Stinkyscape #11, Epic): a rift over an observatory lectern spills frost, violet missiles and flame. */
const OTHERWORLDLY_MAGIC: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "jade-400"],
  light: { x: 50, y: 40, r: 32, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 50, y: 38, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1.6 }],
    far: [
      { part: "nebula", x: 30, y: 30, scale: 1.1, paint: "cyan-400" },
      { part: "nebula", x: 76, y: 22, scale: 0.8, paint: "jade-400" },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 74, paint: "slate-700" },
      { part: "fence", x: 50, y: 74, scale: 1.4, paint: "slate-400" },
      { part: "rift", x: 50, y: 38, scale: 1.2, paint: "cyan-400" },
    ],
    subject: [
      { part: "frost-shard", x: 34, y: 50, scale: 1.1 },
      { part: "missiles", x: 52, y: 50, scale: 0.9 },
      { part: "flame-column", x: 66, y: 60, scale: 0.6 },
    ],
    near: [{ part: "lectern", x: 50, y: 92, scale: 1.4 }],
    light: [{ part: "halo", x: 50, y: 40, scale: 1.8, paint: "cyan-400" }],
    particles: [
      { part: "embers", x: 66, y: 46, count: 2, spread: [6, 6] },
      { part: "sparks", x: 40, y: 46, count: 2, spread: [8, 6] },
      { part: "motes", x: 56, y: 70, scale: 1.2, paint: "slate-400" },
    ],
  },
};

/** Otherworldly Nightmare (Stinkyscape #12, Epic): three bad dreams pour from a rift over a sleeper in her bunk pod. */
const OTHERWORLDLY_NIGHTMARE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 34, r: 30, paint: "purple-400", opacity: 0.4 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.2 }],
    far: [
      { part: "room-wall", x: 50, y: 78, paint: "slate-800" },
      { part: "porthole", x: 80, y: 28, scale: 1 },
    ],
    mid: [
      { part: "rift", x: 46, y: 28, scale: 1.1, paint: "purple-400" },
      { part: "mist", x: 50, y: 46, scale: 0.9, count: 2, spread: [10, 4], paint: "purple-400" },
    ],
    subject: [
      { part: "bunk-pod", x: 50, y: 66, scale: 1.3 },
      { part: "sleeper", x: 50, y: 66, scale: 1 },
      { part: "pale-hand", x: 36, y: 50, scale: 1.2, flip: -1, paint: "slate-900" },
      { part: "shockwave", x: 62, y: 46, scale: 0.6, paint: "violet-300" },
    ],
    near: [{ part: "deck-floor", x: 50, y: 86, paint: "slate-900" }],
    light: [{ part: "halo", x: 46, y: 28, scale: 1.6, paint: "purple-400" }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, count: 2, spread: [10, 6], paint: "violet-300" }],
  },
};

/** Asphyxiants Cloud (Stinkyscape #13, Rare): a sickly gas cloud rolls down a station corridor over choking crew. */
const ASPHYXIANTS_CLOUD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 42, r: 30, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 56, y: 58, size: 4, rotate: 0 },
  figure: { x: 56, y: 86, scale: 0.8, pose: "pose-raise", look: human({ hair: "helmet", outfit: "outfit-suit", item: null, cloth: "slate-400" }) },
  extras: [{ x: 32, y: 78, scale: 0.55, flip: -1, pose: "pose-guard", look: human({ hair: "helmet", outfit: "outfit-suit", item: null, cloth: "slate-400" }), layer: "subject" }],
  layers: {
    far: [{ part: "corridor", x: 50, y: 44, paint: "red-500" }],
    mid: [{ part: "mist", x: 50, y: 50, scale: 1.2, count: 3, spread: [16, 10], paint: "lime-400" }],
    near: [{ part: "mist", x: 50, y: 76, scale: 1.4, count: 2, spread: [14, 6], paint: "lime-200" }],
    light: [{ part: "halo", x: 50, y: 30, scale: 1.4, paint: "red-500" }],
  },
};

/** Arm of the Jempire (Stinkyscape #14, Common): a plum-and-steel enforcer braces a shield-bearer with his hydraulic arm. */
const ARM_OF_THE_JEMPIRE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 44, r: 30, paint: "slate-400", opacity: 0.3 },
  emblem: { x: 40, y: 54, size: 4, rotate: 0 },
  figure: { x: 40, y: 86, scale: 1, pose: "pose-stand", look: human({ outfit: "outfit-hydraulic", item: null, cloth: "violet-700" }) },
  extras: [{ x: 66, y: 82, scale: 0.8, flip: -1, pose: "pose-guard", look: human({ outfit: "outfit-armor", item: "item-shield", cloth: "slate-400" }), layer: "subject" }],
  layers: {
    far: [{ part: "banner", x: 14, y: 70, scale: 1.2, paint: "violet-700" }],
    mid: [{ part: "deck-floor", x: 50, y: 72, paint: "slate-700" }],
  },
};

/** Arg Arg Defender (Stinkyscape #15, Common): a stocky defender in dented orange plate flings arms and shield wide over two crewmates. */
const ARG_ARG_DEFENDER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 44, r: 30, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 50, y: 54, size: 4, rotate: 0 },
  figure: { x: 50, y: 86, scale: 1, pose: "pose-wide", look: human({ outfit: "outfit-armor", item: "item-shield", cloth: "orange-400" }) },
  extras: [
    { x: 30, y: 80, scale: 0.6, pose: "pose-stand", look: human({ outfit: "outfit-jacket", item: null, cloth: "slate-700" }), layer: "subject" },
    { x: 72, y: 80, scale: 0.6, flip: -1, pose: "pose-stand", look: human({ hair: "hair-long", outfit: "outfit-jacket", item: null, cloth: "blue-800" }), layer: "subject" },
  ],
  layers: {
    far: [{ part: "hangar-wall", x: 50, y: 68 }],
    mid: [{ part: "deck-floor", x: 50, y: 68 }],
  },
};

/** Rod of Ages (Stinkyscape #16, Mythic): an ancient rod adrift in a starless gulf, a lone warrior stepping onto its shrine. */
const ROD_OF_AGES: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 40, r: 40, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 50, y: 44, size: 5, rotate: 0 },
  figure: { x: 50, y: 82, scale: 0.32, pose: "pose-stand", look: human({ outfit: "outfit-armor", item: "item-sword", cloth: "slate-400" }) },
  layers: {
    sky: [
      { part: "mist", x: 30, y: 20, scale: 1.2, paint: "slate-400" },
      { part: "mist", x: 74, y: 30, scale: 1, paint: "slate-400" },
    ],
    far: [
      { part: "hover-platform", x: 18, y: 66, scale: 0.6, paint: "stone-500" },
      { part: "hover-platform", x: 84, y: 62, scale: 0.55, paint: "stone-500" },
      { part: "hover-platform", x: 30, y: 30, scale: 0.4, paint: "stone-500" },
      { part: "hover-platform", x: 72, y: 26, scale: 0.4, paint: "stone-500" },
      { part: "debris", x: 50, y: 30, scale: 0.8, count: 6, spread: [40, 20] },
    ],
    mid: [
      { part: "ruin-wall", x: 26, y: 82, scale: 0.8, paint: "stone-500" },
      { part: "ruin-wall", x: 76, y: 82, scale: 0.8, flip: -1, paint: "stone-500" },
    ],
    subject: [
      { part: "hover-platform", x: 50, y: 84, scale: 1.4, paint: "stone-500" },
      { part: "ancient-rod", x: 50, y: 44, scale: 1 },
    ],
    near: [{ part: "debris", x: 14, y: 88, scale: 1.2, count: 2, spread: [6, 4] }],
    light: [
      { part: "light-shaft", x: 50, y: -2, scale: 1.2 },
      { part: "light-ribbon", x: 50, y: 70, scale: 0.4, rotate: -80, paint: "slate-100" },
      { part: "light-ribbon", x: 48, y: 72, scale: 0.4, rotate: -110, paint: "slate-100" },
      { part: "halo", x: 50, y: 60, scale: 1.6, paint: "slate-100" },
      { part: "rim-light", x: 50, y: 44, scale: 1.6, paint: "slate-100" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.6, count: 4, spread: [14, 14], paint: "slate-100" },
      { part: "sparks", x: 50, y: 60, count: 2, spread: [10, 14] },
    ],
  },
};

/** Imprisoned Ship Smith (Stinkyscape #17, Common): a chained smith hammers a glowing axe head in a prison hulk's cell. */
const IMPRISONED_SHIP_SMITH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-800"],
  light: { x: 62, y: 70, r: 24, paint: "red-500", opacity: 0.4 },
  emblem: { x: 42, y: 58, size: 4, rotate: 0 },
  figure: { x: 42, y: 86, scale: 0.95, pose: "pose-raise", look: human({ outfit: "outfit-apron", item: "item-hammer", cloth: "wood-700" }) },
  layers: {
    mid: [{ part: "deck-floor", x: 50, y: 74, paint: "slate-800" }],
    subject: [{ part: "anvil", x: 62, y: 84, scale: 1.2 }],
    near: [{ part: "cell-bars", x: 50, y: 100, paint: "slate-400" }],
    light: [{ part: "halo", x: 62, y: 76, scale: 1.2, paint: "red-500" }],
  },
};

/** Imitated Instrumentality (Stinkyscape #18, Legendary): three self-made computer cores boot up and turn to a small commander. */
const IMITATED_INSTRUMENTALITY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 40, r: 36, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 50, y: 58, size: 4, rotate: 0 },
  figure: { x: 50, y: 86, scale: 0.34, pose: "pose-stand", look: human({ hair: "hair-long", outfit: "outfit-jacket", item: null, cloth: "slate-700" }) },
  layers: {
    sky: [{ part: "stars", x: 50, y: 12, scale: 1.2 }],
    far: [
      { part: "column", x: 10, y: 74, scale: 1.4, paint: "slate-800" },
      { part: "column", x: 90, y: 74, scale: 1.4, paint: "slate-800" },
      { part: "arch", x: 50, y: 70, scale: 2.2, paint: "slate-800" },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 72, paint: "slate-900" }],
    subject: [
      { part: "server-core", x: 26, y: 74, scale: 1.3, paint: "cyan-400" },
      { part: "server-core", x: 74, y: 74, scale: 1.3, paint: "red-400" },
      { part: "server-core", x: 50, y: 66, scale: 1.4, paint: "violet-300" },
    ],
    near: [{ part: "console", x: 18, y: 96, scale: 1.2 }],
    light: [
      { part: "beam", x: 26, y: 44, scale: 0.6, rotate: 0, paint: "cyan-400" },
      { part: "beam", x: 26, y: 44, scale: 0.7, rotate: -36, paint: "cyan-400" },
      { part: "beam", x: 74, y: 44, scale: 0.7, rotate: 216, paint: "red-400" },
      { part: "halo", x: 50, y: 34, scale: 1.6, paint: "blue-200" },
      { part: "light-shaft", x: 50, y: 0, count: 2, spread: [16, 0] },
    ],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1.6, count: 2, spread: [12, 8], paint: "blue-200" },
      { part: "sparks", x: 50, y: 30, count: 2, spread: [20, 6] },
    ],
  },
};

/** Imprisoned Cosmality (Stinkyscape #19, Common): a starstuff creature glares out of a cracking containment sphere by a black moon. */
const IMPRISONED_COSMALITY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 50, r: 28, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 66, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 1.4 },
      { part: "moon", x: 78, y: 26, scale: 0.9, paint: "slate-900" },
    ],
    subject: [
      { part: "cosmality", x: 50, y: 50, scale: 1.4 },
      { part: "containment-sphere", x: 50, y: 52, scale: 1.3 },
    ],
    light: [{ part: "halo", x: 50, y: 50, scale: 1.6, paint: "violet-300" }],
  },
};

/** Tnack Trav (Stinkyscape #20, Epic): a grinning copycat's scanner copies a towering hero's outline onto his own body. */
const TNACK_TRAV: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-900"],
  light: { x: 54, y: 46, r: 32, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 40, y: 58, size: 4, rotate: 0 },
  figure: { x: 40, y: 88, scale: 0.9, pose: "pose-reach", look: human({ outfit: "outfit-coat", item: "item-scanner", cloth: "jade-400" }) },
  extras: [{ x: 70, y: 84, scale: 1.15, flip: -1, pose: "pose-guard", look: human({ outfit: "outfit-armor", item: "item-sword", cloth: "cyan-400", skin: "sky-300" }), layer: "mid" }],
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.2 }],
    far: [
      { part: "station", x: 18, y: 26, scale: 0.9 },
      { part: "banner", x: 88, y: 60, scale: 1.1, paint: "cyan-400" },
      { part: "banner", x: 8, y: 62, scale: 1, paint: "red-600" },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 72, paint: "slate-700" }],
    near: [{ part: "crate", x: 12, y: 96, scale: 1.4 }],
    light: [
      { part: "beam", x: 54, y: 52, scale: 0.4, rotate: -20, paint: "cyan-400" },
      { part: "rim-light", x: 70, y: 56, scale: 1.4, paint: "cyan-400" },
      { part: "halo", x: 70, y: 54, scale: 2, paint: "cyan-400" },
    ],
    particles: [{ part: "sparks", x: 60, y: 50, count: 2, spread: [10, 10] }],
  },
};

/** Moonberry (Stinkyscape #21, Common): a single glowing blue berry on a silver vine on a lunar orchard terrace. */
const MOONBERRY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 50, r: 26, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 56, y: 70, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 30, y: 20, scale: 1.2 },
      { part: "gas-giant", x: 70, y: 32, scale: 1.3, paint: "violet-300" },
    ],
    mid: [{ part: "regolith", x: 50, y: 72, paint: "slate-400" }],
    subject: [{ part: "berry-vine", x: 50, y: 78, scale: 2.4 }],
    light: [{ part: "halo", x: 50, y: 50, scale: 1.2, paint: "sky-300" }],
  },
};

/** Gatinsir (Stinkyscape #22, Epic): space folds open behind an attacking fighter and a tortoise-plated gunship emerges. */
const GATINSIR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 60, y: 40, r: 32, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 60, y: 38, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 1.8 },
      { part: "nebula", x: 24, y: 70, scale: 0.9, paint: "violet-700" },
    ],
    far: [
      { part: "rift", x: 62, y: 38, scale: 1.6, paint: "violet-500" },
      { part: "gunship", x: 62, y: 38, scale: 0.9, paint: "slate-700" },
    ],
    mid: [{ part: "disruptor-beacon", x: 82, y: 66, scale: 1 }],
    subject: [{ part: "fighter", x: 40, y: 60, scale: 1.6, paint: "red-600" }],
    near: [{ part: "asteroid", x: 12, y: 88, scale: 1.3 }],
    light: [
      { part: "beam", x: 48, y: 59, scale: 0.6, rotate: -8, paint: "red-400" },
      { part: "beam", x: 48, y: 62, scale: 0.6, rotate: 6, paint: "red-400" },
      { part: "halo", x: 62, y: 38, scale: 2.2, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 62, y: 40, count: 3, spread: [16, 12] },
      { part: "motes", x: 50, y: 50, paint: "violet-300" },
    ],
  },
};

/** Warp Disruptor (Stinkyscape #23, Rare): a disruptor beacon at a blue nebula's edge bends starlight into rings. */
const WARP_DISRUPTOR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 50, r: 30, paint: "purple-400", opacity: 0.45 },
  emblem: { x: 50, y: 52, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 1.8 },
      { part: "nebula", x: 24, y: 34, scale: 1.2, paint: "blue-600" },
    ],
    far: [
      { part: "fighter", x: 18, y: 72, scale: 0.6, paint: "slate-700" },
      { part: "fighter", x: 84, y: 30, scale: 0.6, flip: -1, paint: "slate-700" },
    ],
    subject: [{ part: "disruptor-beacon", x: 50, y: 50, scale: 1.6 }],
    light: [
      { part: "shockwave", x: 50, y: 52, scale: 1.6, paint: "purple-400" },
      { part: "halo", x: 50, y: 50, scale: 1.6, paint: "violet-300" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "violet-300" }],
  },
};

/** Warp Disruptor II (Stinkyscape #24, Rare): a cracked disruptor array shatters space above a storm-banded gas giant. */
const WARP_DISRUPTOR_II: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 44, r: 30, paint: "purple-400", opacity: 0.5 },
  emblem: { x: 50, y: 44, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1.6 }],
    far: [{ part: "gas-giant", x: 50, y: 92, scale: 2.4, paint: "red-600" }],
    mid: [{ part: "space-shards", x: 50, y: 40, scale: 1.3, paint: "purple-400" }],
    subject: [
      { part: "disruptor-beacon", x: 42, y: 46, scale: 1.4, paint: "purple-400" },
      { part: "disruptor-beacon", x: 62, y: 40, scale: 1.1, paint: "red-400" },
    ],
    near: [{ part: "debris", x: 20, y: 30, scale: 0.9 }],
    light: [{ part: "halo", x: 52, y: 44, scale: 2, paint: "purple-400" }],
    particles: [{ part: "sparks", x: 50, y: 44, count: 2, spread: [14, 10] }],
  },
};

/** KY's Tricknology (Stinkyscape #25, Epic): KY sketches a light grid that skews over the lanes, ship parts dropping in. */
const KYS_TRICKNOLOGY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-800"],
  light: { x: 58, y: 44, r: 30, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 36, y: 58, size: 4, rotate: 0 },
  figure: { x: 36, y: 88, scale: 0.9, pose: "pose-reach", look: human({ outfit: "outfit-labcoat", item: "item-stylus", cloth: "slate-100" }) },
  layers: {
    sky: [{ part: "stars", x: 50, y: 16, scale: 1.4 }],
    far: [
      { part: "debris", x: 62, y: 20, scale: 0.8 },
      { part: "debris", x: 84, y: 26, scale: 0.7 },
      { part: "drone-core", x: 74, y: 16, scale: 0.6 },
    ],
    mid: [
      { part: "light-grid", x: 66, y: 52, scale: 1, paint: "slate-100" },
      { part: "light-grid", x: 70, y: 70, scale: 0.8, paint: "slate-100" },
    ],
    near: [{ part: "deck-floor", x: 50, y: 88, paint: "slate-700" }],
    light: [{ part: "halo", x: 52, y: 52, scale: 1, paint: "slate-100" }],
    particles: [
      { part: "motes", x: 60, y: 50, scale: 1.2, paint: "slate-100" },
      { part: "sparks", x: 52, y: 52, count: 2, spread: [6, 4] },
    ],
  },
};

/** ZolarKeth (Stinkyscape #26, Legendary): a medalled professor on a moon university's balcony, a rocket still not his. */
const ZOLARKETH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "sky-400"],
  light: { x: 70, y: 70, r: 34, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 44, y: 56, size: 4, rotate: 0 },
  figure: { x: 44, y: 88, scale: 0.92, pose: "pose-reach", look: human({ hair: "hair-silver", outfit: "outfit-robes", item: "item-chart", cloth: "blue-800" }) },
  layers: {
    sky: [{ part: "stars", x: 40, y: 14, scale: 1.4 }],
    far: [
      { part: "gas-giant", x: 74, y: 84, scale: 1.4, paint: "blue-600" },
      { part: "station", x: 26, y: 30, scale: 1.1 },
      { part: "tower", x: 14, y: 62, scale: 0.8 },
      { part: "arch", x: 30, y: 62, scale: 0.8 },
    ],
    mid: [
      { part: "rocket", x: 80, y: 66, scale: 1.3 },
      { part: "antenna-mast", x: 62, y: 66, scale: 0.6 },
    ],
    subject: [{ part: "deck-floor", x: 50, y: 74, paint: "slate-700" }],
    near: [{ part: "fence", x: 50, y: 90, scale: 2, paint: "slate-400" }],
    light: [
      { part: "rim-light", x: 44, y: 60, scale: 1.4, paint: "blue-200" },
      { part: "light-shaft", x: 74, y: 40, scale: 0.6, count: 2, spread: [6, 0] },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4, count: 2, spread: [16, 10], paint: "blue-200" }],
  },
};

/** Drone Core I (Stinkyscape #27, Common): a cheap dented core in a moon scrapyard gulps a card and spits out one drone. */
const DRONE_CORE_I: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-800"],
  light: { x: 50, y: 56, r: 26, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 46, y: 62, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.4 }],
    mid: [
      { part: "regolith", x: 50, y: 66, paint: "wood-700" },
      { part: "debris", x: 20, y: 66, scale: 1.2, paint: "wood-600" },
      { part: "debris", x: 84, y: 68, scale: 1.1, paint: "wood-600" },
    ],
    subject: [
      { part: "drone-core", x: 46, y: 62, scale: 1.8, paint: "orange-400" },
      { part: "drone", x: 68, y: 42, scale: 1.3 },
    ],
    near: [{ part: "blank-card", x: 34, y: 54, scale: 1.2 }],
    light: [{ part: "halo", x: 46, y: 62, scale: 1.2, paint: "orange-400" }],
  },
};

/** Drone Core II (Stinkyscape #28, Common): a gloved hand tightens the last bolt on a flat-pack drone in a garage pod. */
const DRONE_CORE_II: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "wood-800"],
  light: { x: 44, y: 44, r: 26, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 56, size: 4, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-700" },
      { part: "lantern", x: 44, y: 8, scale: 1.4 },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 66, paint: "wood-700" }],
    subject: [
      { part: "drone", x: 50, y: 56, scale: 2.2 },
      { part: "drone-core", x: 50, y: 57, scale: 0.6 },
    ],
    near: [
      { part: "gloved-hand", x: 70, y: 80, scale: 1.4, flip: -1 },
      { part: "crate", x: 18, y: 92, scale: 1.2 },
    ],
  },
};

/** Drone Core III (Stinkyscape #29, Common): two drones fly wingtip to wingtip out of a cargo hatch over a salt flat. */
const DRONE_CORE_III: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "slate-100"],
  light: { x: 50, y: 20, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 26, y: 56, size: 4, rotate: 0 },
  layers: {
    far: [{ part: "cloud", x: 70, y: 24, scale: 0.8, count: 2, spread: [16, 4] }],
    mid: [
      { part: "regolith", x: 50, y: 70, paint: "slate-100" },
      { part: "porthole", x: 24, y: 58, scale: 1.3, paint: "slate-700" },
    ],
    subject: [
      { part: "drone-core", x: 24, y: 58, scale: 1.4 },
      { part: "drone", x: 50, y: 44, scale: 1.6 },
      { part: "drone", x: 66, y: 44, scale: 1.6 },
    ],
    near: [{ part: "crate", x: 20, y: 92, scale: 1.6, paint: "slate-700" }],
  },
};

/** Drone Core IV (Stinkyscape #30, Rare): three interceptor drones launch in a V from a core on an asteroid outpost's mast. */
const DRONE_CORE_IV: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 76, y: 22, r: 28, paint: "white", opacity: 0.35 },
  emblem: { x: 34, y: 38, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.6 }],
    mid: [
      { part: "regolith", x: 50, y: 74, paint: "slate-400" },
      { part: "antenna-mast", x: 34, y: 76, scale: 1.2 },
    ],
    subject: [
      { part: "drone-core", x: 34, y: 40, scale: 1.2 },
      { part: "drone", x: 60, y: 40, scale: 1.4 },
      { part: "drone", x: 52, y: 30, scale: 1.2 },
      { part: "drone", x: 52, y: 50, scale: 1.2 },
    ],
    near: [{ part: "mist", x: 50, y: 82, scale: 1.2, paint: "slate-400" }],
    light: [{ part: "rim-light", x: 34, y: 56, paint: "white" }],
  },
};

/** Drone Core V (Stinkyscape #31, Rare): a bulky drone corvette shoulders out of a cracked-open core in a dry dock. */
const DRONE_CORE_V: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 50, r: 30, paint: "sky-300", opacity: 0.35 },
  emblem: { x: 56, y: 54, size: 4, rotate: 0 },
  layers: {
    far: [
      { part: "dock-gantry", x: 50, y: 78, scale: 1.8 },
      { part: "deck-floor", x: 50, y: 78, paint: "slate-800" },
    ],
    subject: [
      { part: "drone-core", x: 36, y: 56, scale: 2.2 },
      { part: "drone-corvette", x: 58, y: 54, scale: 1.4 },
    ],
    near: [{ part: "debris", x: 30, y: 74, scale: 1.2 }],
    light: [
      { part: "spotlight", x: 14, y: 10, scale: 1, rotate: 50, paint: "sky-300" },
      { part: "spotlight", x: 86, y: 10, scale: 1, rotate: 130, paint: "sky-300" },
      { part: "halo", x: 36, y: 56, scale: 1.4, paint: "sky-300" },
    ],
    particles: [{ part: "sparks", x: 40, y: 56, count: 2, spread: [10, 8] }],
  },
};

/** Drone Core VI (Stinkyscape #32, Epic): five drones strung on one glowing thread from a core on an icy ring fragment. */
const DRONE_CORE_VI: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 44, r: 30, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 30, y: 72, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1.6 }],
    far: [
      { part: "planet", x: 76, y: 28, scale: 1.6, paint: "slate-100" },
      { part: "asteroid", x: 60, y: 76, scale: 0.8, count: 3, spread: [24, 8], paint: "blue-200" },
    ],
    mid: [{ part: "asteroid", x: 30, y: 80, scale: 3.2, paint: "blue-200" }],
    subject: [
      { part: "drone-core", x: 30, y: 72, scale: 1.2 },
      { part: "drone", x: 40, y: 58, scale: 1 },
      { part: "drone", x: 50, y: 48, scale: 1 },
      { part: "drone", x: 60, y: 40, scale: 1 },
      { part: "drone", x: 70, y: 34, scale: 1 },
      { part: "drone", x: 80, y: 30, scale: 1 },
    ],
    near: [{ part: "frost-shard", x: 16, y: 90, scale: 1.4 }],
    light: [{ part: "light-ribbon", x: 30, y: 72, scale: 1.3, rotate: -40, paint: "cyan-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "cyan-400" }],
  },
};

/** Drone Core VII (Stinkyscape #33, Epic): a drone corvette cruises out of a core's hatch, two interceptors flanking it. */
const DRONE_CORE_VII: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 70, y: 40, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 56, y: 50, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 1.6 },
      { part: "comet", x: 52, y: 18, scale: 1.1, rotate: -12 },
    ],
    far: [{ part: "nebula", x: 20, y: 30, scale: 0.9, paint: "blue-600" }],
    mid: [{ part: "drone-core", x: 24, y: 64, scale: 2.4 }],
    subject: [
      { part: "drone-corvette", x: 56, y: 52, scale: 1.3 },
      { part: "drone", x: 70, y: 36, scale: 1 },
      { part: "drone", x: 72, y: 66, scale: 1 },
    ],
    near: [
      { part: "deck-floor", x: 30, y: 86, paint: "slate-700" },
      { part: "debris", x: 84, y: 86, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 56, y: 52, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 60, paint: "orange-400" }],
  },
};

/** Drone Core VIII (Stinkyscape #34, Legendary): an orbital factory's line floods interceptors from a cracked reactor's leftover energy. */
const DRONE_CORE_VIII: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 30, y: 60, r: 32, paint: "white", opacity: 0.4 },
  emblem: { x: 22, y: 56, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 60, y: 16, scale: 1.6 },
      { part: "drone", x: 70, y: 22, scale: 0.5, count: 6, spread: [26, 8] },
    ],
    far: [
      { part: "dock-gantry", x: 74, y: 56, scale: 1.2 },
      { part: "drone", x: 76, y: 38, scale: 0.7, count: 4, spread: [18, 6] },
    ],
    mid: [
      { part: "reactor", x: 20, y: 76, scale: 1.4 },
      { part: "conveyor", x: 56, y: 72, scale: 1.1 },
    ],
    subject: [
      { part: "drone-core", x: 22, y: 56, scale: 0.9 },
      { part: "drone", x: 48, y: 66, scale: 1.1 },
      { part: "drone", x: 62, y: 66, scale: 1.1 },
      { part: "drone", x: 74, y: 58, scale: 1.2 },
      { part: "drone", x: 84, y: 48, scale: 1.2 },
    ],
    near: [
      { part: "robot-arm", x: 38, y: 92, scale: 1.2 },
      { part: "robot-arm", x: 74, y: 92, scale: 1.1, flip: -1 },
    ],
    light: [
      { part: "beam", x: 24, y: 64, scale: 0.6, rotate: 0, paint: "blue-600" },
      { part: "halo", x: 20, y: 62, scale: 1.8, paint: "cyan-400" },
    ],
    particles: [
      { part: "sparks", x: 46, y: 70, count: 3, spread: [14, 6] },
      { part: "embers", x: 60, y: 76, count: 2, spread: [12, 4], paint: "white" },
    ],
  },
};

/** Drone Core IX (Stinkyscape #35, Legendary): three drone corvettes burst in a wedge from a giant core split like a seed pod over a volcanic world. */
const DRONE_CORE_IX: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 40, y: 70, r: 36, paint: "ember-500", opacity: 0.45 },
  emblem: { x: 58, y: 44, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.4 },
      { part: "smoke-trail", x: 8, y: 30, scale: 1.2, rotate: -12, paint: "slate-900" },
    ],
    far: [{ part: "lava-flats", x: 50, y: 74 }],
    mid: [
      { part: "drone-core", x: 30, y: 52, scale: 3.4, paint: "ember-500" },
      { part: "debris", x: 20, y: 36, scale: 1.2 },
      { part: "debris", x: 38, y: 68, scale: 1.1 },
      { part: "smoke-trail", x: 24, y: 50, scale: 0.9, rotate: -70, paint: "slate-400" },
    ],
    subject: [
      { part: "drone-corvette", x: 58, y: 44, scale: 1.1 },
      { part: "drone-corvette", x: 74, y: 30, scale: 0.9 },
      { part: "drone-corvette", x: 76, y: 58, scale: 0.9 },
    ],
    near: [{ part: "debris", x: 88, y: 86, scale: 1.3 }],
    light: [
      { part: "halo", x: 30, y: 52, scale: 2.2, paint: "ember-500" },
      { part: "rim-light", x: 60, y: 46, scale: 1.4, paint: "red-400" },
    ],
    particles: [
      { part: "embers", x: 40, y: 64, scale: 1.5, count: 4, spread: [24, 12] },
      { part: "sparks", x: 30, y: 52, count: 2, spread: [10, 10] },
    ],
  },
};

/** Drone Core X (Stinkyscape #36, Mythic): a planet-sized core opens and the three-layered Drone Battleship slides out, spitting interceptors. */
const DRONE_CORE_X: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 30, y: 50, r: 40, paint: "red-600", opacity: 0.5 },
  emblem: { x: 56, y: 52, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 2 },
      { part: "nebula", x: 70, y: 26, scale: 1.4, paint: "red-600" },
      { part: "nebula", x: 20, y: 76, scale: 1.1, paint: "red-900" },
    ],
    far: [
      { part: "drone-core", x: 22, y: 50, scale: 5, paint: "red-500" },
      { part: "dock-gantry", x: 84, y: 86, scale: 1.4, paint: "slate-800" },
    ],
    mid: [
      { part: "debris", x: 40, y: 26, count: 2, spread: [10, 6] },
      { part: "drone", x: 80, y: 30, scale: 0.6, count: 7, spread: [16, 12] },
    ],
    subject: [
      { part: "drone-battleship", x: 56, y: 52, scale: 0.85 },
      { part: "drone", x: 84, y: 62, scale: 0.9 },
      { part: "drone", x: 88, y: 46, scale: 0.8 },
      { part: "drone-core", x: 84, y: 62, scale: 0.25 },
      { part: "drone-core", x: 88, y: 46, scale: 0.22 },
    ],
    near: [
      { part: "drone", x: 10, y: 88, scale: 1.6 },
      { part: "drone-core", x: 10, y: 88, scale: 0.45 },
      { part: "debris", x: 72, y: 90, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 22, y: 50, scale: 2.6, paint: "red-500" },
      { part: "rim-light", x: 56, y: 52, scale: 2, paint: "white" },
      { part: "light-shaft", x: 22, y: 30, scale: 0.8, count: 2, spread: [10, 0] },
    ],
    particles: [
      { part: "sparks", x: 70, y: 52, scale: 1.2, count: 3, spread: [16, 12] },
      { part: "embers", x: 40, y: 50, scale: 1.4, count: 3, spread: [18, 18], paint: "red-400" },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "red-300" },
    ],
  },
};

/** Kwzka (Stinkyscape #37, Legendary): an idol on a floating concert platform sends ribbons of pink light to the smitten ships and soldiers below. */
const KWZKA: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "violet-700"],
  light: { x: 50, y: 40, r: 34, paint: "red-300", opacity: 0.45 },
  emblem: { x: 50, y: 50, size: 4, rotate: 0 },
  figure: { x: 50, y: 70, scale: 0.72, pose: "pose-raise", look: human({ hair: "headset", outfit: "outfit-suit", item: null, cloth: "red-300" }) },
  extras: [
    { x: 18, y: 92, scale: 0.34, pose: "pose-raise", look: human({ hair: "visor", outfit: "outfit-armor", item: null, cloth: "slate-400" }), layer: "near" },
    { x: 82, y: 94, scale: 0.36, flip: -1, pose: "pose-raise", look: human({ hair: "visor", outfit: "outfit-armor", item: null, cloth: "slate-400" }), layer: "near" },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.6 }],
    far: [
      { part: "station", x: 16, y: 30, scale: 1 },
      { part: "fighter", x: 82, y: 30, scale: 0.6, flip: -1 },
      { part: "fighter", x: 88, y: 44, scale: 0.5, flip: -1 },
    ],
    mid: [{ part: "fighter", x: 14, y: 62, scale: 0.8 }],
    subject: [{ part: "hover-platform", x: 50, y: 72, scale: 1.1, paint: "red-300" }],
    near: [{ part: "mist", x: 50, y: 92, scale: 1.4, paint: "purple-400" }],
    light: [
      { part: "spotlight", x: 10, y: 0, scale: 1.2, rotate: 56, paint: "red-300" },
      { part: "spotlight", x: 90, y: 0, scale: 1.2, rotate: 124, paint: "purple-400" },
      { part: "light-ribbon", x: 56, y: 44, scale: 0.9, rotate: 30, paint: "red-300" },
      { part: "light-ribbon", x: 46, y: 44, scale: 0.9, rotate: 150, paint: "purple-400" },
      { part: "halo", x: 50, y: 44, scale: 1.6, paint: "red-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 30, count: 2, spread: [24, 8] },
      { part: "motes", x: 50, y: 50, scale: 1.6, count: 2, spread: [10, 10], paint: "red-300" },
    ],
  },
};

/** The Narcor Project (Stinkyscape #39, Legendary): a cracking asteroid lab crammed with glowing Narcor crystals, one more crate wheeled in. */
const THE_NARCOR_PROJECT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 50, r: 34, paint: "green-400", opacity: 0.3 },
  emblem: { x: 50, y: 60, size: 4, rotate: 0 },
  figure: { x: 70, y: 88, scale: 0.6, flip: -1, pose: "pose-reach", look: human({ outfit: "outfit-labcoat", item: null, cloth: "slate-100" }) },
  layers: {
    sky: [{ part: "stars", x: 50, y: 8, scale: 1 }],
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-700" },
      { part: "ruin-wall", x: 14, y: 70, scale: 1.2, paint: "slate-800" },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 70, paint: "slate-800" },
      { part: "glass-case", x: 18, y: 72, scale: 1.1 },
      { part: "crystal", x: 18, y: 66, scale: 0.8, count: 2, spread: [2, 1], paint: "purple-400" },
      { part: "glass-case", x: 82, y: 72, scale: 1.1 },
      { part: "crystal", x: 82, y: 66, scale: 0.8, count: 2, spread: [2, 1], paint: "red-400" },
    ],
    subject: [
      { part: "glass-case", x: 46, y: 76, scale: 1.6 },
      { part: "crystal", x: 46, y: 68, scale: 1.2, count: 3, spread: [4, 2], paint: "purple-400" },
      { part: "crate", x: 80, y: 88, scale: 1 },
    ],
    near: [{ part: "crystal", x: 30, y: 92, scale: 1.1, count: 3, spread: [10, 2], paint: "red-400" }],
    light: [
      { part: "lantern", x: 30, y: 0, scale: 1.2 },
      { part: "lantern", x: 70, y: 0, scale: 1.2 },
      { part: "halo", x: 46, y: 64, scale: 1.8, paint: "purple-400" },
      { part: "lightning", x: 10, y: 20, scale: 0.6 },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, count: 2, spread: [10, 10], paint: "green-200" },
      { part: "sparks", x: 46, y: 62, count: 2, spread: [10, 8] },
    ],
  },
};

/** Drone Sympathizer (Stinkyscape #41, Common): an eager drone pushes a core into a startled pilot's hands on a station tram. */
const DRONE_SYMPATHIZER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "blue-800"],
  light: { x: 30, y: 30, r: 26, paint: "white", opacity: 0.3 },
  emblem: { x: 44, y: 58, size: 4, rotate: 0 },
  figure: { x: 40, y: 88, scale: 0.9, pose: "pose-guard", look: human({ outfit: "outfit-jacket", item: null, cloth: "blue-800" }) },
  layers: {
    far: [{ part: "tram", x: 50, y: 54, scale: 2.3 }],
    subject: [
      { part: "helper-drone", x: 62, y: 58, scale: 1.4, flip: -1 },
      { part: "drone-core", x: 52, y: 60, scale: 0.8 },
    ],
  },
};

/** Core Researcher (Stinkyscape #42, Rare): a researcher in singed goggles holds up a glowing drone core in her scorched lab. */
const CORE_RESEARCHER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-800", "slate-400"],
  light: { x: 56, y: 40, r: 28, paint: "white", opacity: 0.35 },
  emblem: { x: 44, y: 56, size: 4, rotate: 0 },
  figure: { x: 44, y: 88, scale: 0.92, pose: "pose-raise", look: human({ hair: "goggles", outfit: "outfit-labcoat", item: null, cloth: "slate-100" }) },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-700" },
      { part: "smoke-trail", x: 70, y: 40, scale: 0.6, rotate: -80, paint: "slate-900" },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 70, paint: "slate-700" },
      { part: "console", x: 80, y: 74, scale: 1.2 },
    ],
    near: [{ part: "drone-core", x: 54, y: 34, scale: 1 }],
    light: [{ part: "halo", x: 54, y: 34, scale: 1.2, paint: "cyan-400" }],
  },
};

/** Great Paperclip Theory (Stinkyscape #44, Mythic): a staircase of ever larger drone cores bent from one silver wire climbs out the window into space. */
const GREAT_PAPERCLIP_THEORY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-800"],
  light: { x: 70, y: 30, r: 36, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 40, y: 64, size: 4, rotate: 0 },
  figure: { x: 18, y: 88, scale: 0.42, pose: "pose-stand", look: human({ outfit: "outfit-labcoat", item: null, cloth: "slate-100" }) },
  layers: {
    sky: [
      { part: "stars", x: 60, y: 16, scale: 1.8 },
      { part: "stars", x: 30, y: 46, scale: 1.4 },
      { part: "nebula", x: 30, y: 24, scale: 1.1, paint: "blue-800" },
      { part: "moon", x: 92, y: 40, scale: 0.9 },
      { part: "planet", x: 52, y: 14, scale: 0.7, paint: "slate-400" },
    ],
    far: [
      { part: "porthole", x: 46, y: 56, scale: 2.6, paint: "slate-700" },
      { part: "station", x: 18, y: 30, scale: 0.6 },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 82, paint: "slate-400" },
      { part: "console", x: 30, y: 84, scale: 1.2, paint: "slate-100" },
    ],
    subject: [
      { part: "wire-coil", x: 30, y: 74, scale: 1.6 },
      { part: "drone-core", x: 32, y: 72, scale: 0.4 },
      { part: "drone-core", x: 40, y: 66, scale: 0.6 },
      { part: "drone-core", x: 50, y: 58, scale: 0.8 },
      { part: "drone-core", x: 62, y: 46, scale: 1.1 },
      { part: "drone-core", x: 74, y: 30, scale: 1.4 },
      { part: "drone-core", x: 86, y: 12, scale: 2 },
    ],
    near: [{ part: "debris", x: 84, y: 88, scale: 1 }],
    light: [
      { part: "rim-light", x: 62, y: 46, scale: 1.4, paint: "slate-100" },
      { part: "halo", x: 74, y: 30, scale: 1.6, paint: "blue-200" },
      { part: "light-shaft", x: 72, y: 20, scale: 0.8, count: 2, spread: [8, 0] },
    ],
    particles: [
      { part: "motes", x: 60, y: 40, scale: 1.6, count: 3, spread: [14, 14], paint: "slate-100" },
      { part: "sparks", x: 60, y: 44, count: 2, spread: [16, 16] },
    ],
  },
};

/** Drone Commander (Stinkyscape #45, Epic): a stern commander holds a fan of glowing cores, her nameless drones hovering behind her. */
const DRONE_COMMANDER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-900"],
  light: { x: 56, y: 40, r: 30, paint: "sky-300", opacity: 0.35 },
  emblem: { x: 44, y: 56, size: 4, rotate: 0 },
  figure: { x: 44, y: 88, scale: 0.95, pose: "pose-stand", look: human({ hair: "hair-long", outfit: "outfit-jacket", item: "item-core", cloth: "slate-400" }) },
  layers: {
    far: [{ part: "hangar-wall", x: 50, y: 66, paint: "slate-800" }],
    mid: [
      { part: "deck-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "drone", x: 70, y: 34, scale: 1.1, count: 3, spread: [14, 10] },
      { part: "drone", x: 22, y: 30, scale: 1, count: 2, spread: [8, 6] },
    ],
    near: [{ part: "crate", x: 84, y: 94, scale: 1.3, paint: "slate-700" }],
    light: [
      { part: "halo", x: 54, y: 60, scale: 1, paint: "cyan-400" },
      { part: "light-shaft", x: 30, y: 0, count: 2, spread: [20, 0] },
    ],
  },
};

/** Circadian Halfcycler (Stinkyscape #46, Epic): a pilot dozes in her cockpit seat as an autopilot arm works the controls. */
const CIRCADIAN_HALFCYCLER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "orange-400"],
  light: { x: 70, y: 40, r: 30, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 40, y: 56, size: 4, rotate: 0 },
  figure: { x: 38, y: 82, scale: 0.9, pose: "pose-sit", look: human({ hair: "hair-long", outfit: "outfit-suit", item: null, cloth: "slate-700" }) },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.2 },
      { part: "station", x: 70, y: 30, scale: 1.1 },
      { part: "drone", x: 84, y: 20, scale: 0.6 },
    ],
    far: [{ part: "cockpit-frame", x: 50, y: 66 }],
    mid: [
      { part: "console", x: 72, y: 78, scale: 1.3 },
      { part: "robot-arm", x: 86, y: 76, scale: 0.9, flip: -1 },
    ],
    subject: [{ part: "pilot-seat", x: 38, y: 84, scale: 1 }],
    near: [{ part: "drone-core", x: 54, y: 46, scale: 0.4, paint: "orange-400" }],
    light: [
      { part: "halo", x: 72, y: 66, scale: 1.2, paint: "cyan-400" },
      { part: "rim-light", x: 40, y: 60, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, paint: "orange-400" }],
  },
};

/** Circadian Fullcycler (Stinkyscape #47, Epic): a pilot sleeps in her bridge chair as robot arms run the bridge and a fleet gathers outside. */
const CIRCADIAN_FULLCYCLER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "sky-300"],
  light: { x: 50, y: 30, r: 34, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 46, y: 62, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.2 }],
    far: [
      { part: "porthole", x: 30, y: 30, scale: 1.4 },
      { part: "porthole", x: 70, y: 30, scale: 1.4 },
      { part: "drone-corvette", x: 30, y: 30, scale: 0.4 },
      { part: "drone-corvette", x: 70, y: 28, scale: 0.36 },
      { part: "drone-corvette", x: 72, y: 34, scale: 0.3 },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "console", x: 18, y: 72, scale: 1.1 },
      { part: "console", x: 82, y: 72, scale: 1.1 },
      { part: "robot-arm", x: 10, y: 72, scale: 0.8 },
      { part: "robot-arm", x: 90, y: 72, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "pilot-seat", x: 52, y: 84, scale: 1.2 },
      { part: "sleeper", x: 52, y: 64, scale: 1.4 },
    ],
    near: [{ part: "deck-floor", x: 50, y: 92, paint: "slate-800" }],
    light: [
      { part: "halo", x: 18, y: 64, scale: 0.8, paint: "cyan-400" },
      { part: "halo", x: 82, y: 64, scale: 0.8, paint: "cyan-400" },
    ],
  },
};

/** Stratos (Stinkyscape #68, Common): a cheap boxy little fighter, one of a dozen in a trade station's showroom rack. */
const STRATOS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 40, r: 28, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 48, y: 56, size: 4, rotate: 0 },
  layers: {
    far: [
      { part: "dock-gantry", x: 50, y: 50, scale: 1.6, paint: "slate-700" },
      { part: "fighter", x: 34, y: 34, scale: 0.6, paint: "slate-400" },
      { part: "fighter", x: 64, y: 34, scale: 0.6, paint: "slate-400" },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 66, paint: "slate-400" }],
    subject: [{ part: "fighter", x: 50, y: 58, scale: 1.7, paint: "blue-600" }],
  },
};

/** Falcon (Stinkyscape #69, Rare): a slender needle-nosed fighter streaks low over a hangar briefing deck as officers duck. */
const FALCON: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-800"],
  light: { x: 50, y: 50, r: 30, paint: "sky-400", opacity: 0.35 },
  emblem: { x: 56, y: 50, size: 4, rotate: 0 },
  extras: [
    { x: 30, y: 84, scale: 0.42, pose: "pose-guard", look: human({ outfit: "outfit-suit", item: null, cloth: "blue-800" }), layer: "mid" },
    { x: 72, y: 86, scale: 0.44, flip: -1, pose: "pose-guard", look: human({ hair: "hair-long", outfit: "outfit-suit", item: null, cloth: "blue-800" }), layer: "mid" },
  ],
  layers: {
    far: [{ part: "hangar-wall", x: 50, y: 70 }],
    mid: [{ part: "deck-floor", x: 50, y: 70 }],
    subject: [{ part: "needle-fighter", x: 56, y: 54, scale: 1.6 }],
    light: [{ part: "light-shaft", x: 30, y: 0, count: 2, spread: [24, 0] }],
  },
};

/** Z'Zoth (Stinkyscape #70, Legendary): a hooded wraith of wreckage with one lantern eye lifts ghost ships out of a dead nebula's graveyard. */
const Z_ZOTH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "jade-400"],
  light: { x: 50, y: 36, r: 36, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 1.25, pose: "pose-raise", look: human({ head: "head-lantern", hair: "hood", outfit: "outfit-wreck", item: null, skin: "black", cloth: "slate-400" }) },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.4 },
      { part: "nebula", x: 30, y: 30, scale: 1.3, paint: "jade-400" },
    ],
    far: [
      { part: "wreck", x: 14, y: 70, scale: 0.9 },
      { part: "wreck", x: 86, y: 66, scale: 0.8, flip: -1 },
      { part: "debris", x: 50, y: 74, count: 4, spread: [40, 6] },
    ],
    mid: [
      { part: "ghost-ship", x: 20, y: 30, scale: 1 },
      { part: "ghost-ship", x: 80, y: 24, scale: 0.9, flip: -1 },
      { part: "ghost-ship", x: 76, y: 46, scale: 0.7, flip: -1 },
    ],
    subject: [{ part: "wreck", x: 50, y: 88, scale: 1.4 }],
    near: [{ part: "debris", x: 14, y: 90, scale: 1.3, count: 2, spread: [6, 3] }],
    light: [
      { part: "halo", x: 34, y: 20, scale: 1, paint: "slate-100" },
      { part: "halo", x: 66, y: 20, scale: 1, paint: "slate-100" },
      { part: "rim-light", x: 50, y: 56, scale: 2, paint: "jade-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.6, count: 3, spread: [16, 12], paint: "slate-100" },
      { part: "wisp", x: 30, y: 46, scale: 0.6, count: 2, spread: [30, 8] },
    ],
  },
};

/** A'Zoth (Stinkyscape #71, Legendary): a robed herald of polished wreckage roars shockwaves over a debris arena; the fallen ships rise shouting. */
const A_ZOTH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 50, y: 30, r: 36, paint: "red-400", opacity: 0.45 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 1.2, pose: "pose-wide", look: human({ head: "head-horn", hair: "hood", outfit: "outfit-wreck", item: null, skin: "slate-100", cloth: "slate-100" }) },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.4 },
      { part: "star-glare", x: 50, y: 16, scale: 1, paint: "red-400" },
    ],
    far: [
      { part: "wreck", x: 14, y: 52, scale: 0.8, flip: -1 },
      { part: "wreck", x: 86, y: 48, scale: 0.8 },
      { part: "debris", x: 50, y: 76, count: 3, spread: [40, 4] },
    ],
    mid: [
      { part: "ruin-wall", x: 16, y: 86, scale: 1, paint: "slate-700" },
      { part: "ruin-wall", x: 84, y: 86, scale: 1, flip: -1, paint: "slate-700" },
    ],
    subject: [{ part: "shockwave", x: 64, y: 24, scale: 0.9, paint: "red-300" }],
    near: [{ part: "debris", x: 84, y: 92, scale: 1.3, count: 2, spread: [6, 3] }],
    light: [
      { part: "shockwave", x: 62, y: 24, scale: 1.5, paint: "white" },
      { part: "halo", x: 50, y: 16, scale: 2, paint: "red-500" },
      { part: "rim-light", x: 50, y: 56, scale: 2, paint: "red-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 30, scale: 1.2, count: 3, spread: [30, 10] },
      { part: "embers", x: 50, y: 70, count: 2, spread: [30, 10], paint: "red-400" },
    ],
  },
};

/** Paragon Mk1 (Stinkyscape #72, Epic): a sleek white arrowhead fighter peels away after its far shot, high over a planet's curve. */
const PARAGON_MK1: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "sky-400"],
  light: { x: 30, y: 30, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 54, y: 50, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.4 }],
    far: [
      { part: "gas-giant", x: 50, y: 130, scale: 4.4, paint: "blue-600" },
      { part: "station", x: 84, y: 22, scale: 0.5 },
    ],
    mid: [
      { part: "cloud", x: 30, y: 80, scale: 0.9, count: 3, spread: [24, 4] },
      { part: "mist", x: 50, y: 82, scale: 1.4, paint: "slate-100" },
    ],
    subject: [{ part: "arrow-fighter", x: 54, y: 50, scale: 1.8, flip: -1 }],
    near: [{ part: "beam", x: 2, y: 86, scale: 2.2, rotate: -28, paint: "sky-300" }],
    light: [
      { part: "rim-light", x: 54, y: 50, scale: 1.2, paint: "white" },
      { part: "halo", x: 30, y: 30, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 80, y: 30, count: 2, spread: [8, 6] },
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" },
    ],
  },
};

/** Paragon Mk2 (Stinkyscape #73, Legendary): a sharper arrowhead fighter dives through an asteroid choke point, its hex shield shrugging off flak. */
const PARAGON_MK2: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-800"],
  light: { x: 54, y: 50, r: 32, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 52, y: 50, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.4 }],
    far: [
      { part: "asteroid", x: 50, y: 30, scale: 1.2, count: 5, spread: [44, 16], paint: "wood-700" },
      { part: "nebula", x: 70, y: 70, scale: 1, paint: "orange-400" },
    ],
    mid: [
      { part: "asteroid", x: 12, y: 50, scale: 3.4, paint: "wood-700" },
      { part: "asteroid", x: 90, y: 60, scale: 3, paint: "wood-700" },
    ],
    subject: [
      { part: "arrow-fighter", x: 52, y: 52, scale: 1.7, paint: "slate-400" },
      { part: "hex-shield", x: 52, y: 52, scale: 1.4 },
    ],
    near: [
      { part: "asteroid", x: 20, y: 90, scale: 2.6, paint: "wood-800" },
      { part: "asteroid", x: 88, y: 18, scale: 1.8, paint: "wood-800" },
      { part: "debris", x: 80, y: 88, scale: 1.2 },
    ],
    light: [
      { part: "beam", x: 70, y: 52, scale: 0.6, rotate: 0, paint: "cyan-400" },
      { part: "halo", x: 52, y: 52, scale: 2, paint: "cyan-400" },
    ],
    particles: [
      { part: "sparks", x: 52, y: 52, scale: 1.3, count: 4, spread: [18, 18] },
      { part: "embers", x: 40, y: 40, count: 3, spread: [16, 10], paint: "orange-400" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "orange-400" },
    ],
  },
};

/** Paragon-X (Stinkyscape #74, Mythic): the finest fighter ever flown rises from the wrecks it was built from, spell bolts splashing off its prism hull. */
const PARAGON_X: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-800"],
  light: { x: 76, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 46, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 40, y: 16, scale: 1.8 },
      { part: "stars", x: 60, y: 56, scale: 1.4 },
      { part: "star-glare", x: 78, y: 22, scale: 1.4 },
    ],
    far: [
      { part: "regolith", x: 50, y: 72, paint: "slate-700" },
      { part: "moon", x: 20, y: 28, scale: 0.8, paint: "slate-400" },
    ],
    mid: [
      { part: "wreck", x: 26, y: 76, scale: 1.2, paint: "slate-100" },
      { part: "wreck", x: 76, y: 78, scale: 1.1, flip: -1 },
      { part: "smoke-trail", x: 26, y: 72, scale: 1.1, rotate: -80, paint: "slate-700" },
      { part: "smoke-trail", x: 76, y: 74, scale: 1, rotate: -100, paint: "slate-700" },
    ],
    subject: [
      { part: "blade-fighter", x: 50, y: 46, scale: 1.7 },
      { part: "hex-shield", x: 50, y: 46, scale: 1.9, paint: "sky-300" },
    ],
    near: [
      { part: "debris", x: 14, y: 90, scale: 1.3, count: 2, spread: [6, 3] },
      { part: "debris", x: 86, y: 92, scale: 1.2 },
    ],
    light: [
      { part: "beam", x: 4, y: 30, scale: 0.9, rotate: 18, paint: "violet-500" },
      { part: "beam", x: 96, y: 64, scale: 0.9, rotate: 200, paint: "purple-400" },
      { part: "beam", x: 90, y: 8, scale: 0.8, rotate: 150, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 46, scale: 2, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 46, scale: 1.3, count: 4, spread: [24, 16] },
      { part: "embers", x: 50, y: 76, count: 3, spread: [30, 6] },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "sky-300" },
    ],
  },
};

/** Badger (Stinkyscape #75, Common): a stubby badger-striped mining fighter clutches a shiny rock beside a lumpy asteroid. */
const BADGER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-800"],
  light: { x: 60, y: 50, r: 26, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 46, y: 54, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.4 }],
    far: [{ part: "asteroid", x: 70, y: 30, scale: 0.7, count: 2, spread: [14, 8], paint: "wood-700" }],
    mid: [{ part: "asteroid", x: 20, y: 74, scale: 4, paint: "wood-700" }],
    subject: [
      { part: "miner-craft", x: 48, y: 54, scale: 2 },
      { part: "crystal", x: 68, y: 58, scale: 0.8, paint: "sky-300" },
    ],
    light: [{ part: "halo", x: 68, y: 54, scale: 0.8, paint: "sky-300" }],
    particles: [{ part: "sparks", x: 68, y: 56, count: 1 }],
  },
};

/** Honey Badger (Stinkyscape #76, Rare): a scarred badger-striped brawler rams through an ore barge and grabs the ore under turret fire. */
const HONEY_BADGER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 50, y: 54, r: 30, paint: "red-500", opacity: 0.4 },
  emblem: { x: 46, y: 54, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1.4 }],
    mid: [{ part: "barge", x: 62, y: 56, scale: 1.6 }],
    subject: [
      { part: "miner-craft", x: 44, y: 54, scale: 1.8 },
      { part: "crystal", x: 30, y: 56, scale: 0.7, paint: "orange-400" },
    ],
    near: [{ part: "debris", x: 50, y: 66, scale: 1.2 }],
    light: [
      { part: "beam", x: 82, y: 34, scale: 0.6, rotate: 150, paint: "red-500" },
      { part: "beam", x: 80, y: 74, scale: 0.6, rotate: 200, paint: "red-500" },
    ],
    particles: [{ part: "sparks", x: 52, y: 54, count: 3, spread: [10, 8] }],
  },
};

/** Yuma (Stinkyscape #77, Epic): a long-winged trader banks away from a free port with one crate, a swapped parcel left in the other. */
const YUMA: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "blue-800"],
  light: { x: 70, y: 24, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 56, y: 44, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1 }],
    far: [
      { part: "station", x: 22, y: 30, scale: 1.4 },
      { part: "fighter", x: 40, y: 20, scale: 0.4 },
      { part: "fighter", x: 8, y: 44, scale: 0.4, flip: -1 },
    ],
    mid: [{ part: "planet", x: 84, y: 70, scale: 1.4, paint: "orange-400" }],
    subject: [{ part: "trader", x: 56, y: 46, scale: 1.7 }],
    near: [
      { part: "crate", x: 26, y: 92, scale: 1.8, paint: "wood-600" },
      { part: "duffel", x: 26, y: 80, scale: 0.6, paint: "red-600" },
    ],
    light: [{ part: "rim-light", x: 56, y: 46, scale: 1.2, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, paint: "white" }],
  },
};

/** Ozark (Stinkyscape #78, Epic): a wide rusty mining barge full of working miner pods, a tiny pod paying its rent into its flank. */
const OZARK: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-800"],
  light: { x: 60, y: 50, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 54, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.4 }],
    far: [{ part: "asteroid", x: 50, y: 28, scale: 1.1, count: 6, spread: [44, 10], paint: "red-600" }],
    mid: [
      { part: "asteroid", x: 14, y: 80, scale: 2.4, paint: "wood-700" },
      { part: "miner-craft", x: 80, y: 30, scale: 0.7 },
    ],
    subject: [{ part: "barge", x: 50, y: 54, scale: 1.6 }],
    near: [
      { part: "miner-craft", x: 26, y: 76, scale: 1.2 },
      { part: "credit-chips", x: 36, y: 70, scale: 0.8 },
    ],
    light: [{ part: "rim-light", x: 50, y: 54, scale: 1.6, paint: "orange-400" }],
    particles: [{ part: "embers", x: 50, y: 60, count: 2, spread: [24, 8], paint: "orange-400" }],
  },
};

/** Galaxy (Stinkyscape #80, Legendary): a towering flagship with a mine shaft through its belly drills a ringed dwarf planet, escorts falling away spent. */
const GALAXY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 70, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 40, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.6 },
      { part: "nebula", x: 22, y: 24, scale: 1, paint: "violet-700" },
    ],
    far: [
      { part: "fighter", x: 14, y: 30, scale: 0.6, flip: -1, paint: "slate-700" },
      { part: "fighter", x: 86, y: 26, scale: 0.6, paint: "slate-700" },
      { part: "smoke-trail", x: 14, y: 32, scale: 0.5, rotate: -150, paint: "slate-700" },
      { part: "smoke-trail", x: 86, y: 28, scale: 0.5, rotate: -30, paint: "slate-700" },
    ],
    mid: [{ part: "planet", x: 50, y: 96, scale: 3.4, paint: "slate-700" }],
    subject: [{ part: "flagship", x: 50, y: 44, scale: 1.5 }],
    near: [{ part: "debris", x: 84, y: 86, scale: 1.3 }],
    light: [
      { part: "halo", x: 50, y: 70, scale: 1.4, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 44, scale: 2, paint: "blue-200" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 70, count: 3, spread: [10, 6] },
      { part: "embers", x: 50, y: 66, count: 4, spread: [8, 10] },
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** Radix (Stinkyscape #81, Rare): a bare skeletal ship frame with two empty hardpoints waits in a spare-parts hangar. */
const RADIX: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 50, r: 30, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 50, y: 54, size: 4, rotate: 0 },
  layers: {
    far: [{ part: "hangar-wall", x: 50, y: 70 }],
    mid: [{ part: "deck-floor", x: 50, y: 70 }],
    subject: [{ part: "frame-ship", x: 50, y: 54, scale: 1.9 }],
    near: [
      { part: "crate", x: 16, y: 94, scale: 1.4, paint: "slate-700" },
      { part: "crate", x: 84, y: 94, scale: 1.3, paint: "slate-700" },
      { part: "ray-gun", x: 84, y: 76, scale: 0.8 },
      { part: "ray-gun", x: 16, y: 78, scale: 0.8, flip: -1 },
    ],
    light: [{ part: "light-shaft", x: 50, y: 0, count: 2, spread: [16, 0] }],
  },
};

/** Phalanx (Stinkyscape #82, Epic): a squat tortoise gunship bristling with barrels advances on a frontier outpost, its plates dented. */
const PHALANX: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-400"],
  light: { x: 50, y: 50, r: 30, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 50, y: 50, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "smoke-trail", x: 10, y: 20, scale: 1.2, rotate: -10 }],
    far: [
      { part: "ruin-wall", x: 16, y: 66, scale: 1.4, paint: "slate-700" },
      { part: "ruin-wall", x: 84, y: 66, scale: 1.4, flip: -1, paint: "slate-700" },
      { part: "tower", x: 50, y: 64, scale: 0.8, paint: "slate-700" },
    ],
    mid: [{ part: "regolith", x: 50, y: 66, paint: "slate-700" }],
    subject: [{ part: "gunship", x: 50, y: 58, scale: 1.8 }],
    near: [
      { part: "debris", x: 20, y: 86, scale: 1.4 },
      { part: "debris", x: 80, y: 88, scale: 1.4 },
      { part: "smoke-trail", x: 70, y: 82, scale: 0.8, rotate: -60 },
    ],
    light: [{ part: "rim-light", x: 50, y: 52, scale: 1.4, paint: "slate-100" }],
    particles: [{ part: "embers", x: 50, y: 40, count: 2, spread: [20, 8] }],
  },
};

/** Halo (Stinkyscape #83, Epic): a sweating pilot grips the controls in a glass cockpit, a full ring of guns turning about it as an armada closes in. */
const HALO: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 50, y: 50, r: 30, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 46, y: 60, size: 4, rotate: 0 },
  figure: { x: 46, y: 70, scale: 0.5, pose: "pose-sit", look: human({ hair: "helmet", outfit: "outfit-suit", item: null, cloth: "slate-700" }) },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.6 }],
    far: [
      { part: "fighter", x: 16, y: 20, scale: 0.6, count: 3, spread: [10, 8], paint: "red-600" },
      { part: "fighter", x: 86, y: 76, scale: 0.6, flip: -1, count: 3, spread: [10, 8], paint: "red-600" },
    ],
    mid: [{ part: "gun-ring", x: 50, y: 56, scale: 1.7 }],
    subject: [
      { part: "porthole", x: 50, y: 56, scale: 1.6, paint: "slate-400" },
      { part: "pilot-seat", x: 46, y: 71, scale: 0.5 },
    ],
    near: [{ part: "cockpit-frame", x: 50, y: 96 }],
    light: [
      { part: "halo", x: 50, y: 56, scale: 2, paint: "cyan-400" },
      { part: "halo", x: 86, y: 20, scale: 1, paint: "red-500" },
    ],
  },
};

/** Power Cell (Stinkyscape #84, Common): a cracked, overcharged power cell sparks in an engine room, flinging scraps into a deck crate. */
const POWER_CELL: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "green-900"],
  light: { x: 46, y: 50, r: 28, paint: "green-200", opacity: 0.4 },
  emblem: { x: 46, y: 56, size: 4, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-700" },
      { part: "deck-floor", x: 50, y: 74, paint: "slate-800" },
    ],
    subject: [{ part: "reactor", x: 46, y: 78, scale: 1.6, paint: "green-400" }],
    near: [
      { part: "crate", x: 78, y: 94, scale: 1.4, paint: "slate-700" },
      { part: "debris", x: 70, y: 60, scale: 0.8 },
    ],
    light: [{ part: "lightning", x: 46, y: 40, scale: 0.6 }],
    particles: [{ part: "sparks", x: 46, y: 50, count: 2, spread: [14, 10] }],
  },
};

/** Prototype-X1 (Stinkyscape #85, Legendary): a raw test ship trailing cables hovers over a barren moon's range, two test ships wrecked behind it. */
const PROTOTYPE_X1: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-800"],
  light: { x: 50, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 46, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.6 }],
    far: [
      { part: "regolith", x: 50, y: 66 },
      { part: "antenna-mast", x: 12, y: 68, scale: 1.1 },
      { part: "antenna-mast", x: 88, y: 66, scale: 0.9 },
    ],
    mid: [
      { part: "wreck", x: 26, y: 72, scale: 1 },
      { part: "wreck", x: 76, y: 74, scale: 0.9, flip: -1 },
      { part: "smoke-trail", x: 26, y: 70, scale: 1, rotate: -84, paint: "slate-700" },
      { part: "smoke-trail", x: 76, y: 72, scale: 0.9, rotate: -96, paint: "slate-700" },
    ],
    subject: [{ part: "frame-ship", x: 50, y: 46, scale: 1.9, paint: "stone-500" }],
    near: [{ part: "debris", x: 50, y: 90, scale: 1.2, count: 2, spread: [30, 2] }],
    light: [
      { part: "spotlight", x: 12, y: 30, scale: 0.9, rotate: 20, paint: "white" },
      { part: "spotlight", x: 88, y: 30, scale: 0.9, rotate: 160, paint: "white" },
      { part: "halo", x: 12, y: 30, scale: 0.6, paint: "red-500" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 52, count: 2, spread: [16, 6] },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "slate-400" },
    ],
  },
};

/** Prototype-X2 (Stinkyscape #86, Mythic): a hull fused from five ships erupts from a collapsing forge-sphere, its welds still glowing. */
const PROTOTYPE_X2: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 50, r: 40, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.8 },
      { part: "stars", x: 70, y: 70, scale: 1.4 },
      { part: "nebula", x: 74, y: 28, scale: 1.4, paint: "violet-500" },
      { part: "nebula", x: 22, y: 74, scale: 1.1, paint: "purple-400" },
    ],
    far: [
      { part: "containment-sphere", x: 40, y: 50, scale: 2.6, paint: "orange-400" },
      { part: "debris", x: 50, y: 50, scale: 1.2, count: 5, spread: [40, 30] },
    ],
    mid: [
      { part: "smoke-trail", x: 30, y: 60, scale: 1, rotate: 160, paint: "slate-900" },
      { part: "smoke-trail", x: 40, y: 40, scale: 1, rotate: -120, paint: "slate-900" },
      { part: "flame-column", x: 20, y: 80, scale: 0.8 },
    ],
    subject: [{ part: "fused-hull", x: 54, y: 50, scale: 1.4 }],
    near: [
      { part: "debris", x: 14, y: 88, scale: 1.4, count: 2, spread: [6, 3] },
      { part: "debris", x: 86, y: 90, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 40, y: 50, scale: 2.6, paint: "orange-400" },
      { part: "halo", x: 20, y: 30, scale: 0.6, paint: "orange-400" },
      { part: "halo", x: 54, y: 34, scale: 0.6, paint: "orange-400" },
      { part: "rim-light", x: 54, y: 50, scale: 2, paint: "violet-300" },
    ],
    particles: [
      { part: "embers", x: 50, y: 50, scale: 1.6, count: 5, spread: [30, 20] },
      { part: "sparks", x: 50, y: 50, scale: 1.2, count: 4, spread: [26, 16] },
    ],
  },
};

/** Siva 1 (Stinkyscape #87, Legendary): a cat-folk celebrity in lilac and dark glasses lounges on a red carpet, a rival on a leash of light. */
const SIVA_1: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 40, y: 40, r: 34, paint: "purple-400", opacity: 0.4 },
  emblem: { x: 40, y: 56, size: 4, rotate: 0 },
  figure: { x: 40, y: 78, scale: 0.78, pose: "pose-reach", look: { ...figureLookFor(["Felinor"]), hair: "shades", outfit: "outfit-coat", item: null, cloth: "violet-300" } },
  extras: [{ x: 74, y: 84, scale: 0.6, flip: -1, pose: "pose-stand", look: human({ outfit: "outfit-suit", item: null, cloth: "blue-800" }), layer: "subject" }],
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.4 }],
    far: [
      { part: "station", x: 72, y: 26, scale: 1.3 },
      { part: "banner", x: 14, y: 70, scale: 1.2, paint: "violet-500" },
      { part: "banner", x: 88, y: 70, scale: 1.2, flip: -1, paint: "violet-500" },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 74, paint: "slate-800" }],
    subject: [{ part: "hover-platform", x: 40, y: 80, scale: 1.1, paint: "red-600" }],
    near: [{ part: "mist", x: 50, y: 92, scale: 1.2, paint: "purple-400" }],
    light: [
      { part: "light-ribbon", x: 52, y: 52, scale: 0.55, rotate: 18, paint: "purple-400" },
      { part: "halo", x: 20, y: 30, scale: 0.6, paint: "white" },
      { part: "halo", x: 84, y: 40, scale: 0.6, paint: "white" },
      { part: "halo", x: 64, y: 20, scale: 0.5, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 34, scale: 1.2, count: 3, spread: [34, 10] },
      { part: "motes", x: 50, y: 50, paint: "violet-300" },
    ],
  },
};

/** Siva 2 (Stinkyscape #88, Legendary): the human sequel in black and toxic green leads a venom-armed crew down a back alley, the original's shadow behind. */
const SIVA_2: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 50, y: 40, r: 34, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 46, y: 58, size: 4, rotate: 0 },
  figure: { x: 46, y: 88, scale: 0.9, pose: "pose-stand", look: human({ outfit: "outfit-suit", item: "item-venom-blade", cloth: "black" }) },
  extras: [
    { x: 66, y: 70, scale: 1.05, flip: -1, pose: "pose-stand", look: { ...figureLookFor(["Felinor"]), item: null, skin: "black", cloth: "black" }, layer: "far" },
    { x: 26, y: 80, scale: 0.6, pose: "pose-guard", look: human({ outfit: "outfit-jacket", item: "item-venom-blade", cloth: "green-900" }), layer: "subject" },
    { x: 70, y: 80, scale: 0.6, flip: -1, pose: "pose-guard", look: human({ hair: "hood", outfit: "outfit-jacket", item: "item-venom-blade", cloth: "green-900" }), layer: "subject" },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 4, scale: 0.8 }],
    far: [{ part: "corridor", x: 50, y: 46, paint: "lime-400" }],
    mid: [{ part: "mist", x: 50, y: 70, scale: 1.2, paint: "green-400" }],
    near: [
      { part: "crate", x: 12, y: 96, scale: 1.4, paint: "slate-800" },
      { part: "mist", x: 50, y: 92, scale: 1.4, paint: "lime-400" },
    ],
    light: [
      { part: "lantern", x: 86, y: 6, scale: 1.2 },
      { part: "halo", x: 50, y: 40, scale: 1.4, paint: "lime-400" },
    ],
    particles: [
      { part: "wisp", x: 50, y: 60, scale: 0.5, count: 3, spread: [24, 12] },
      { part: "motes", x: 50, y: 70, paint: "lime-200" },
    ],
  },
};

/** Spice (Stinkyscape #89, Rare): a glowing orange spice crate changes hands at a floating bazaar, the merchant's scale tipping. */
const SPICE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "wood-800"],
  light: { x: 50, y: 50, r: 30, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 46, y: 64, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 16, scale: 1.2 }],
    far: [
      { part: "market-stall", x: 18, y: 66, scale: 1 },
      { part: "market-stall", x: 84, y: 64, scale: 0.9, flip: -1 },
    ],
    mid: [{ part: "hover-platform", x: 50, y: 82, scale: 1.8, paint: "wood-700" }],
    subject: [
      { part: "crate", x: 40, y: 78, scale: 1.6, paint: "orange-400" },
      { part: "scale", x: 64, y: 78, scale: 1.2 },
      { part: "credit-chips", x: 60, y: 54, count: 2, spread: [6, 4] },
    ],
    light: [
      { part: "halo", x: 40, y: 66, scale: 1.4, paint: "orange-400" },
      { part: "lantern", x: 30, y: 30, scale: 1 },
    ],
    particles: [{ part: "embers", x: 40, y: 60, count: 2, spread: [10, 8], paint: "orange-400" }],
  },
};

/** R3391000: Stinkyscape's cards that draw from a scene, by id. */
export const STINKYSCAPE_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-001": FREEZE_RAY,
  "stinkyscape-002": ABSOLUTE_ZERO,
  "stinkyscape-003": METEOR,
  "stinkyscape-005": MASOCHISM_UNMASKED,
  "stinkyscape-006": TYLANNT,
  "stinkyscape-007": THE_GREATEST_HEIST,
  "stinkyscape-008": CONJURE_ROSEMARY_TRISCUIT,
  "stinkyscape-009": TICKETED,
  "stinkyscape-010": OTHERWORLDLY_ENDURANCE,
  "stinkyscape-011": OTHERWORLDLY_MAGIC,
  "stinkyscape-012": OTHERWORLDLY_NIGHTMARE,
  "stinkyscape-013": ASPHYXIANTS_CLOUD,
  "stinkyscape-014": ARM_OF_THE_JEMPIRE,
  "stinkyscape-015": ARG_ARG_DEFENDER,
  "stinkyscape-016": ROD_OF_AGES,
  "stinkyscape-017": IMPRISONED_SHIP_SMITH,
  "stinkyscape-018": IMITATED_INSTRUMENTALITY,
  "stinkyscape-019": IMPRISONED_COSMALITY,
  "stinkyscape-020": TNACK_TRAV,
  "stinkyscape-021": MOONBERRY,
  "stinkyscape-022": GATINSIR,
  "stinkyscape-023": WARP_DISRUPTOR,
  "stinkyscape-024": WARP_DISRUPTOR_II,
  "stinkyscape-025": KYS_TRICKNOLOGY,
  "stinkyscape-026": ZOLARKETH,
  "stinkyscape-027": DRONE_CORE_I,
  "stinkyscape-028": DRONE_CORE_II,
  "stinkyscape-029": DRONE_CORE_III,
  "stinkyscape-030": DRONE_CORE_IV,
  "stinkyscape-031": DRONE_CORE_V,
  "stinkyscape-032": DRONE_CORE_VI,
  "stinkyscape-033": DRONE_CORE_VII,
  "stinkyscape-034": DRONE_CORE_VIII,
  "stinkyscape-035": DRONE_CORE_IX,
  "stinkyscape-036": DRONE_CORE_X,
  "stinkyscape-037": KWZKA,
  "stinkyscape-039": THE_NARCOR_PROJECT,
  "stinkyscape-041": DRONE_SYMPATHIZER,
  "stinkyscape-042": CORE_RESEARCHER,
  "stinkyscape-044": GREAT_PAPERCLIP_THEORY,
  "stinkyscape-045": DRONE_COMMANDER,
  "stinkyscape-046": CIRCADIAN_HALFCYCLER,
  "stinkyscape-047": CIRCADIAN_FULLCYCLER,
  "stinkyscape-068": STRATOS,
  "stinkyscape-069": FALCON,
  "stinkyscape-070": Z_ZOTH,
  "stinkyscape-071": A_ZOTH,
  "stinkyscape-072": PARAGON_MK1,
  "stinkyscape-073": PARAGON_MK2,
  "stinkyscape-074": PARAGON_X,
  "stinkyscape-075": BADGER,
  "stinkyscape-076": HONEY_BADGER,
  "stinkyscape-077": YUMA,
  "stinkyscape-078": OZARK,
  "stinkyscape-080": GALAXY,
  "stinkyscape-081": RADIX,
  "stinkyscape-082": PHALANX,
  "stinkyscape-083": HALO,
  "stinkyscape-084": POWER_CELL,
  "stinkyscape-085": PROTOTYPE_X1,
  "stinkyscape-086": PROTOTYPE_X2,
  "stinkyscape-087": SIVA_1,
  "stinkyscape-088": SIVA_2,
  "stinkyscape-089": SPICE,
};
