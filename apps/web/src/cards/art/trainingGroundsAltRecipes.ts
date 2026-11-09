// R3397000: Training Grounds' alt scene recipes, one per card offered Alt art, each drawn from its brief (ART.md,
// "Meditative+ and Training Grounds Alt art restaged"): the card's own subject in another place, time or moment,
// under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or
// one from above), its figure the normal picture's person (`normalLookOf`) and a crowd its brief keeps the normal
// picture's people (`normalExtraLookOf`), in the Training Grounds family's carved-wood line, out of the yard
// wherever its brief leaves it, and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the
// card's Alt art seed alone, at every Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { TRAINING_GROUNDS_RECIPES } from "./sceneRecipesTrainingGrounds.ts";

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
function normalLookOf(id: string): FigureLook {
  return TRAINING_GROUNDS_RECIPES[id]?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a person its brief keeps is the same one; with none, the default person. */
function normalExtraLookOf(id: string, k: number): FigureLook {
  return TRAINING_GROUNDS_RECIPES[id]?.extras?.[k]?.look ?? figureLookFor([]);
}

/** A recruit of the yard out of it: short hair, a tunic, no weapon. */
const RECRUIT: FigureLook = { ...figureLookFor(["Human"]), item: null };

/** Spikey Steven (Training Grounds #1, Common), Alt art: close-up: Steven carefully hangs his spiked vest on a peg in a rainy barracks, his hair still spiked; muted slate blue. He is caught reaching for the peg with the vest still on: the kit dresses a figure in one outfit. */
const SPIKEY_STEVEN_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-700", "slate-400"],
  light: { x: 76, y: 30, r: 30, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 72, y: 66, size: 5, rotate: 0 },
  figure: { x: 40, y: 102, scale: 1.7, pose: "pose-raise", look: normalLookOf("traininggrounds-001") },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 84, paint: "slate-700" },
      { part: "window", x: 78, y: 46, scale: 1.4 },
    ],
    mid: [{ part: "fence-post", x: 64, y: 84, scale: 2.4, flip: -1 }],
    near: [{ part: "plank-bench", x: 80, y: 98, scale: 1.6 }],
    particles: [
      { part: "rain", x: 78, y: 40, scale: 0.6, paint: "blue-200" },
      { part: "rain", x: 50, y: 40, scale: 1.3, paint: "slate-100" },
    ],
  },
};

/** Personal Trainer (Training Grounds #2, Common), Alt art: the trainer jogs alone along a misty river path at sunrise, whistle bouncing, a log on one shoulder; soft pink-grey, wide shot. The whistle is too small to read at this distance and is left out. */
const PERSONAL_TRAINER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-300", "slate-400"],
  light: { x: 20, y: 56, r: 34, paint: "red-300", opacity: 0.4 },
  emblem: { x: 72, y: 36, size: 5, rotate: 0 },
  figure: { x: 42, y: 76, scale: 0.42, pose: "pose-run", look: normalLookOf("traininggrounds-002") },
  layers: {
    sky: [{ part: "sun", x: 20, y: 56, scale: 0.8 }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "slate-400" },
      { part: "tree", x: 80, y: 62, scale: 2.4, count: 3, spread: [10, 1], paint: "slate-700" },
    ],
    mid: [
      { part: "water", x: 50, y: 64, paint: "slate-400" },
      { part: "rock-floor", x: 50, y: 76, paint: "stone-500" },
      { part: "mist", x: 50, y: 66, scale: 1.6, count: 2, spread: [24, 2], paint: "white" },
    ],
    subject: [{ part: "log", x: 46, y: 59, scale: 0.5 }],
    near: [
      { part: "reed", x: 10, y: 98, scale: 1.8, paint: "slate-700" },
      { part: "reed", x: 88, y: 99, scale: 1.6, flip: -1, paint: "slate-700" },
    ],
    particles: [{ part: "motes", x: 40, y: 50, scale: 1, paint: "white" }],
  },
};

/** Group leader (Training Grounds #3, Common), Alt art: the leader wades first across a chest-deep river, the column holding hands behind him; murky green-brown water, low angle at the surface. */
const GROUP_LEADER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "green-200"],
  light: { x: 76, y: 24, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 22, y: 36, size: 5, rotate: 0 },
  figure: { x: 34, y: 100, scale: 1.25, pose: "pose-stride", look: normalLookOf("traininggrounds-003") },
  extras: [
    { layer: "mid", x: 58, y: 82, scale: 0.55, pose: "pose-spread", look: normalExtraLookOf("traininggrounds-003", 0) },
    { layer: "mid", x: 72, y: 79, scale: 0.47, pose: "pose-spread", look: normalExtraLookOf("traininggrounds-003", 1) },
    { layer: "mid", x: 84, y: 77, scale: 0.4, pose: "pose-spread", look: normalExtraLookOf("traininggrounds-003", 2) },
  ],
  layers: {
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-700" },
      { part: "tree", x: 16, y: 60, scale: 2.2, count: 3, spread: [10, 1], paint: "green-900" },
    ],
    mid: [{ part: "reed", x: 92, y: 66, scale: 1.4, paint: "green-700" }],
    near: [
      { part: "water", x: 50, y: 70, paint: "green-900" },
      { part: "reed", x: 8, y: 98, scale: 1.6, paint: "wood-700" },
    ],
    particles: [{ part: "drops", x: 40, y: 62, scale: 1, paint: "green-200" }],
  },
};

/** Boulder Fist Ogre (Training Grounds #4, Common), Alt art: close-up at rest in a quiet quarry: the ogre cracks walnuts between his boulder fists on a ledge; pale sandstone pink, soft afternoon. */
const BOULDER_FIST_OGRE_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-300", "slate-100"],
  light: { x: 70, y: 30, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 76, y: 34, size: 5, rotate: 0 },
  figure: { x: 40, y: 102, scale: 1.5, pose: "pose-sit", look: normalLookOf("traininggrounds-004") },
  layers: {
    far: [
      { part: "mesa", x: 50, y: 60, paint: "red-300" },
      { part: "cliff", x: 92, y: 70, flip: -1, paint: "stone-500" },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 80, paint: "stone-500" }],
    subject: [{ part: "walnuts", x: 68, y: 86, scale: 0.9 }],
    near: [
      { part: "pebbles", x: 80, y: 98, scale: 1.2 },
      { part: "mossy-boulder", x: 90, y: 100, scale: 0.9 },
    ],
    particles: [{ part: "dust", x: 60, y: 76, scale: 1, paint: "red-300" }],
  },
};

/** Midlane Menace (Training Grounds #5, Common), Alt art: winter, from above: the menace stands on the middle of three frozen lanes, a snowman built on each side lane; icy white and blue. */
const MIDLANE_MENACE_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-200", "white"],
  light: { x: 50, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 78, y: 34, size: 5, rotate: 0 },
  figure: { x: 50, y: 80, scale: 0.55, pose: "pose-hips", look: normalLookOf("traininggrounds-005") },
  layers: {
    far: [{ part: "snowfield", x: 50, y: 40 }],
    mid: [
      { part: "ice-floor", x: 50, y: 46 },
      { part: "lanes", x: 50, y: 46, paint: "blue-200" },
    ],
    subject: [
      { part: "snowman", x: 24, y: 80, scale: 1.4 },
      { part: "snowman", x: 76, y: 80, scale: 1.4, flip: -1 },
    ],
    near: [{ part: "fence-post", x: 8, y: 98, scale: 1.2, paint: "slate-400" }],
    particles: [{ part: "snow", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Clobberer (Training Grounds #8, Common), Alt art: before the hitting: the Clobberer carves his next dummy from a log in a quiet woodshop, club leaning in the corner; soft teal window light, close-up. He holds a carving knife; the kit draws his club only in his hand, so the leaning club is left out. */
const CLOBBERER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-800", "cyan-400"],
  light: { x: 76, y: 34, r: 34, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 22, y: 34, size: 5, rotate: 0 },
  figure: { x: 34, y: 104, scale: 1.6, pose: "pose-hold", look: { ...normalLookOf("traininggrounds-008"), item: "item-knife" } },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 78, paint: "wood-800" },
      { part: "window", x: 76, y: 50, scale: 1.3 },
    ],
    mid: [
      { part: "boards", x: 50, y: 78 },
      { part: "plank-bench", x: 68, y: 86, scale: 2.4 },
    ],
    subject: [{ part: "log", x: 68, y: 74, scale: 1.1 }],
    near: [{ part: "splinters", x: 60, y: 92, scale: 1.2 }],
    light: [{ part: "light-shaft", x: 76, y: 40, scale: 1, paint: "cyan-400" }],
    particles: [{ part: "dust", x: 70, y: 60, scale: 1, paint: "cyan-400" }],
  },
};

/** Fan of Knives (Training Grounds #10, Common), Alt art: overhead in a lavender field: a ring of knives flashes outward from a spinning figure, cutting the flowers flat in a perfect circle; purple and green. */
const FAN_OF_KNIVES_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-300", "purple-400"],
  light: { x: 50, y: 58, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 70, scale: 0.42, pose: "pose-spread", look: { ...RECRUIT, cloth: "green-700" } },
  layers: {
    far: [{ part: "field", x: 50, y: 34, paint: "violet-500" }],
    mid: [
      { part: "flowers", x: 22, y: 50, count: 4, spread: [14, 6] },
      { part: "flowers", x: 78, y: 52, count: 4, spread: [14, 6] },
      { part: "shockwave", x: 50, y: 72, scale: 1.4, paint: "green-400" },
    ],
    subject: [
      { part: "knife", x: 50, y: 36, scale: 0.9 },
      { part: "knife", x: 72, y: 46, scale: 0.9 },
      { part: "knife", x: 74, y: 66, scale: 0.9, flip: -1 },
      { part: "knife", x: 26, y: 66, scale: 0.9 },
      { part: "knife", x: 28, y: 46, scale: 0.9, flip: -1 },
    ],
    near: [{ part: "tall-grass", x: 50, y: 98, scale: 1.4, count: 2, spread: [30, 1], paint: "green-700" }],
    particles: [
      { part: "petals", x: 50, y: 60, scale: 1.2, paint: "violet-500" },
      { part: "sparks", x: 50, y: 56, scale: 1, paint: "white" },
    ],
  },
};

/** Thwomp (Training Grounds #11, Common), Alt art: the block waits in a mossy forest gorge, hung on a creaking rope between cliffs as a deer passes beneath; deep emerald, looking straight up. */
const THWOMP_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["green-900", "green-500"],
  light: { x: 50, y: 20, r: 30, paint: "lime-200", opacity: 0.3 },
  emblem: { x: 22, y: 40, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "cliff", x: 18, y: 44, paint: "green-900" },
      { part: "cliff", x: 82, y: 44, flip: -1, paint: "green-900" },
    ],
    mid: [
      { part: "vine", x: 14, y: 30, scale: 1.4 },
      { part: "fern", x: 84, y: 76, scale: 1.4, flip: -1 },
    ],
    subject: [
      { part: "rope", x: 54, y: 16, scale: 1.4 },
      { part: "oak-block", x: 54, y: 40, scale: 2 },
      { part: "deer", x: 40, y: 84, scale: 1.1 },
    ],
    near: [{ part: "mossy-boulder", x: 86, y: 100, scale: 1.2 }],
    particles: [{ part: "dust", x: 54, y: 46, scale: 1, paint: "lime-200" }],
  },
};

/** In a hurry (Training Grounds #15, Common), Alt art: distant wide shot: the runners race in single file across a snowy hillside at dusk, footprints behind, none coming back; cold violet light. The footprints are left out. */
const IN_A_HURRY_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "violet-300"],
  light: { x: 80, y: 30, r: 30, paint: "violet-300", opacity: 0.35 },
  emblem: { x: 24, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "moon", x: 80, y: 24, scale: 0.6 }],
    far: [{ part: "hills", x: 50, y: 50, paint: "slate-400" }],
    mid: [{ part: "snowfield", x: 50, y: 62 }],
    subject: [
      { part: "runner-token", x: 30, y: 76, scale: 0.8 },
      { part: "runner-token", x: 40, y: 72, scale: 0.75 },
      { part: "runner-token", x: 50, y: 68, scale: 0.7 },
      { part: "runner-token", x: 60, y: 64, scale: 0.65 },
      { part: "runner-token", x: 70, y: 60, scale: 0.6 },
    ],
    near: [{ part: "pine", x: 88, y: 98, scale: 1.6 }],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.3, paint: "white" }],
  },
};

/** Celestial Caller (Training Grounds #16, Rare), Alt art: the caller kneels on a still lake at noon, sun and moon reflected side by side in the water around her; tranquil blue-white, eye level. */
const CELESTIAL_CALLER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 40, size: 5, rotate: 0 },
  figure: { x: 50, y: 84, scale: 0.7, pose: "pose-kneel", look: normalLookOf("traininggrounds-016") },
  layers: {
    sky: [
      { part: "sun", x: 30, y: 22, scale: 0.8 },
      { part: "moon", x: 70, y: 22, scale: 0.8 },
    ],
    far: [{ part: "mountains", x: 50, y: 58, paint: "slate-400" }],
    mid: [
      { part: "water", x: 50, y: 62, paint: "blue-600" },
      { part: "sun", x: 30, y: 78, scale: 0.8, fade: 0.45 },
      { part: "moon", x: 70, y: 78, scale: 0.8, fade: 0.45 },
    ],
    near: [{ part: "reed", x: 10, y: 98, scale: 1.6, paint: "blue-800" }],
    light: [{ part: "halo", x: 50, y: 70, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Flaming Felinor (Training Grounds #17, Rare), Alt art: rain falls on the flaming Felinor sitting alone on a hilltop, steam pouring from her fur; cool grey-blue, wide and lonely. */
const FLAMING_FELINOR_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "blue-200"],
  light: { x: 56, y: 50, r: 26, paint: "orange-400", opacity: 0.25 },
  emblem: { x: 24, y: 34, size: 5, rotate: 0 },
  figure: { x: 56, y: 62, scale: 0.4, pose: "pose-sit", look: normalLookOf("traininggrounds-017") },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.6, count: 3, spread: [28, 3], paint: "slate-700" }],
    far: [{ part: "hills", x: 50, y: 70, paint: "slate-700" }],
    mid: [{ part: "hills", x: 56, y: 64, paint: "slate-800" }],
    subject: [
      { part: "flames", x: 56, y: 58, scale: 0.4, fade: 0.8 },
      { part: "steam", x: 56, y: 46, scale: 1.2, paint: "white" },
    ],
    near: [{ part: "grass-tuft", x: 20, y: 96, count: 3, spread: [12, 2], paint: "slate-700" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 70, scale: 1.1, paint: "blue-200" },
    ],
  },
};

/** Right-Hand Felinor (Training Grounds #22, Rare), Alt art: from above at a spring wedding feast in the yard, the Felinor sits exactly at the knight's right, pouring his drink; blossom pink and white. */
const RIGHT_HAND_FELINOR_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-300", "white"],
  light: { x: 50, y: 30, r: 38, paint: "white", opacity: 0.35 },
  emblem: { x: 78, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 80, scale: 0.5, pose: "pose-sit", look: { ...normalLookOf("traininggrounds-022"), item: "item-flask" } },
  extras: [
    { layer: "subject", x: 56, y: 80, scale: 0.56, flip: -1, pose: "pose-sit", look: { ...normalExtraLookOf("traininggrounds-022", 0), item: null } },
    { layer: "mid", x: 24, y: 70, scale: 0.36, pose: "pose-raise", look: { ...RECRUIT, cloth: "red-600" } },
    { layer: "mid", x: 76, y: 70, scale: 0.36, flip: -1, pose: "pose-stand", look: { ...RECRUIT, hair: "hair-long", cloth: "green-700" } },
  ],
  layers: {
    far: [
      { part: "fruit-tree", x: 14, y: 60, scale: 2.4, paint: "red-300" },
      { part: "fruit-tree", x: 86, y: 58, scale: 2.2, flip: -1, paint: "red-300" },
      { part: "bunting", x: 50, y: 36, scale: 1.4 },
    ],
    mid: [{ part: "field", x: 50, y: 64, paint: "green-400" }],
    subject: [
      { part: "table", x: 48, y: 90, scale: 2.4 },
      { part: "cup", x: 50, y: 74, scale: 1.4 },
    ],
    near: [{ part: "flowers", x: 14, y: 98, count: 3, spread: [8, 1] }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.4, paint: "red-300" }],
  },
};

/** Trigor (Training Grounds #25, Rare), Alt art: close-up: the newest, biggest Trigor flexes on a beach at noon while three smaller sandcastles crumble behind it; bright turquoise sea. */
const TRIGOR_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "cyan-400"],
  light: { x: 72, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 76, y: 36, size: 5, rotate: 0 },
  figure: { x: 38, y: 104, scale: 1.75, pose: "pose-flex", look: normalLookOf("traininggrounds-025") },
  layers: {
    sky: [{ part: "sun", x: 80, y: 18, scale: 0.6 }],
    far: [{ part: "sea", x: 50, y: 58, paint: "cyan-400" }],
    mid: [
      { part: "sand", x: 50, y: 72 },
      { part: "sandcastle", x: 66, y: 80, scale: 0.9 },
      { part: "sandcastle", x: 80, y: 78, scale: 0.8, flip: -1 },
      { part: "sandcastle", x: 92, y: 80, scale: 0.7 },
    ],
    near: [{ part: "pebbles", x: 80, y: 96, scale: 1.2 }],
    particles: [{ part: "dust", x: 80, y: 70, scale: 1, paint: "brass-500" }],
  },
};

/** Occidental (Training Grounds #28, Epic), Alt art: close-up portrait at a rainy dawn in a canyon town: the drifter, the right-hand lane rolled up under his arm like a carpet; cool blue-grey drizzle. The lane is the normal picture's plank runner, carried flat at his side. */
const OCCIDENTAL_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-800", "slate-400"],
  light: { x: 24, y: 40, r: 32, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 76, y: 34, size: 5, rotate: 0 },
  figure: { x: 36, y: 104, scale: 1.8, pose: "pose-hold", look: normalLookOf("traininggrounds-028") },
  layers: {
    far: [
      { part: "mesa", x: 50, y: 62, paint: "slate-700" },
      { part: "whitewashed-house", x: 80, y: 70, scale: 0.7 },
    ],
    mid: [
      { part: "street", x: 50, y: 76, paint: "slate-700" },
      { part: "streetlamp", x: 90, y: 80, scale: 0.8 },
    ],
    subject: [{ part: "plank-rug", x: 58, y: 56, scale: 0.8 }],
    near: [{ part: "dust", x: 70, y: 96, scale: 1.2, paint: "slate-400" }],
    light: [{ part: "rim-light", x: 36, y: 50, scale: 1.4, paint: "slate-100" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.3, paint: "blue-200" },
      { part: "mist", x: 50, y: 70, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Easterner (Training Grounds #29, Epic), Alt art: in autumn snowfall the Easterner meditates on a mountain ledge, the leftmost lane's posts stacked neatly behind him; crimson maple on white, side view. The posts stand together behind him, the kit's posts upright. */
const EASTERNER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["white", "slate-100"],
  light: { x: 70, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 76, y: 30, size: 5, rotate: 0 },
  figure: { x: 36, y: 80, scale: 0.7, flip: -1, pose: "pose-sit", look: normalLookOf("traininggrounds-029") },
  layers: {
    far: [{ part: "mountains", x: 50, y: 60, paint: "slate-400" }],
    mid: [
      { part: "tree", x: 78, y: 82, scale: 5, paint: "red-600" },
      { part: "fence-post", x: 16, y: 80, scale: 1.1 },
      { part: "fence-post", x: 20, y: 80, scale: 1 },
      { part: "fence-post", x: 24, y: 80, scale: 0.95 },
    ],
    subject: [{ part: "light-ribbon", x: 36, y: 66, scale: 0.3, paint: "white" }],
    near: [{ part: "cliff", x: 40, y: 82, paint: "slate-700" }],
    particles: [
      { part: "leaves", x: 64, y: 50, scale: 1.4 },
      { part: "snow", x: 50, y: 40, scale: 1.3, paint: "white" },
    ],
  },
};

/** Floodgates (Training Grounds #30, Epic), Alt art: the day after, aerial: the broken floodgate lies in a sunny flooded yard where ducks paddle among floating dummies; bright reed green. The floodgate sits slumped in the water. */
const FLOODGATES_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["lime-400", "green-200"],
  light: { x: 80, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 24, y: 30, size: 5, rotate: 0 },
  figure: { x: 42, y: 74, scale: 0.55, pose: "pose-sit", look: normalLookOf("traininggrounds-030") },
  layers: {
    far: [
      { part: "palisade", x: 24, y: 44, scale: 0.8, paint: "wood-600" },
      { part: "palisade", x: 76, y: 44, scale: 0.8, flip: -1, paint: "wood-600" },
    ],
    mid: [{ part: "water", x: 50, y: 48, paint: "sky-400" }],
    subject: [
      { part: "duck", x: 66, y: 70, scale: 1 },
      { part: "duck", x: 76, y: 62, scale: 0.9, flip: -1 },
      { part: "straw-dummy", x: 24, y: 72, scale: 0.4 },
      { part: "straw-dummy", x: 80, y: 80, scale: 0.4 },
    ],
    near: [
      { part: "reed", x: 8, y: 98, scale: 2, paint: "green-500" },
      { part: "reed", x: 92, y: 99, scale: 1.8, flip: -1, paint: "green-500" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Selfless Protector (Training Grounds #32, Epic), Alt art: before the fight, close-up: the young guard polishes every comrade's shield in a row by lamplight, his own left for last; deep brown-orange. */
const SELFLESS_PROTECTOR_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["wood-900", "wood-700"],
  light: { x: 70, y: 40, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 76, y: 30, size: 5, rotate: 0 },
  figure: { x: 30, y: 104, scale: 1.6, pose: "pose-kneel", look: normalLookOf("traininggrounds-032") },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 82, paint: "wood-800" }],
    mid: [
      { part: "lantern", x: 72, y: 26, scale: 1.4 },
      { part: "weapon-rack", x: 90, y: 82, scale: 0.9, flip: -1 },
    ],
    subject: [
      { part: "shield", x: 52, y: 82, scale: 1.3 },
      { part: "shield", x: 64, y: 82, scale: 1.3 },
      { part: "shield", x: 76, y: 82, scale: 1.3 },
    ],
    near: [{ part: "candle", x: 88, y: 98, scale: 1.4 }],
    light: [{ part: "halo", x: 72, y: 34, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "motes", x: 60, y: 60, scale: 1.2, paint: "orange-400" }],
  },
};

/** Corpse Recycler (Training Grounds #35, Epic), Alt art: winter, wide: the recycler's mill wheel half-frozen in an icy stream, new runner tokens skating out across the ice; cold periwinkle blue. */
const CORPSE_RECYCLER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-300", "blue-200"],
  light: { x: 30, y: 30, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 76, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "hills", x: 50, y: 56, paint: "violet-500" },
      { part: "pine", x: 84, y: 62, scale: 1.2, count: 3, spread: [10, 1] },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 66 },
      { part: "ice-floor", x: 50, y: 76 },
    ],
    subject: [
      { part: "mill", x: 30, y: 70, scale: 0.9 },
      { part: "water-wheel", x: 52, y: 74, scale: 0.8 },
      { part: "runner-token", x: 66, y: 82, scale: 1 },
      { part: "runner-token", x: 78, y: 86, scale: 1.1 },
    ],
    near: [{ part: "snowfield", x: 50, y: 96 }],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.3, paint: "white" }],
  },
};

/** Seance (Training Grounds #37, Legendary), Alt art: a seance at noon in a bright summer meadow: recruits in a circle on a picnic blanket as a translucent copy of a catapult forms above them; sunny lime green. */
const SEANCE_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "lime-400"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 78, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 86, scale: 0.5, pose: "pose-sit", look: normalExtraLookOf("traininggrounds-037", 0) },
    { layer: "subject", x: 70, y: 86, scale: 0.5, flip: -1, pose: "pose-sit", look: normalExtraLookOf("traininggrounds-037", 1) },
    { layer: "subject", x: 50, y: 80, scale: 0.42, pose: "pose-kneel", look: normalExtraLookOf("traininggrounds-037", 2) },
  ],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.6 },
      { part: "cloud", x: 26, y: 24, scale: 1, count: 2, spread: [14, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-400" },
      { part: "tree", x: 14, y: 62, scale: 2.4, paint: "green-700" },
    ],
    mid: [{ part: "field", x: 50, y: 66, paint: "lime-400" }],
    subject: [
      { part: "picnic-blanket", x: 50, y: 92, scale: 1.8 },
      { part: "catapult", x: 50, y: 50, scale: 1.4, fade: 0.4 },
    ],
    near: [{ part: "flowers", x: 12, y: 98, count: 3, spread: [8, 1] }],
    light: [{ part: "halo", x: 50, y: 44, scale: 1.6, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 44, scale: 1.2, paint: "white" }],
  },
};

/** Tutorial Master (Training Grounds #38, Legendary), Alt art: side view at sunset: the Tutorial Master in a rowboat, patiently explaining the oar stroke again to one bored recruit; deep orange water. */
const TUTORIAL_MASTER_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-600", "orange-400"],
  light: { x: 50, y: 62, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  figure: { x: 60, y: 76, scale: 0.6, flip: -1, pose: "pose-sit", look: { ...normalLookOf("traininggrounds-038"), item: "item-oar" } },
  extras: [{ layer: "subject", x: 40, y: 77, scale: 0.5, pose: "pose-sit", look: normalExtraLookOf("traininggrounds-038", 0) }],
  layers: {
    sky: [
      { part: "sun", x: 50, y: 62, scale: 1.2 },
      { part: "cloud", x: 26, y: 26, scale: 1.3, count: 3, spread: [24, 4], paint: "red-300" },
      { part: "bird", x: 72, y: 30, scale: 0.4, count: 4, spread: [12, 5], paint: "red-900" },
    ],
    far: [
      { part: "hills", x: 50, y: 64, paint: "red-900" },
      { part: "tree", x: 84, y: 64, scale: 2, count: 3, spread: [8, 1], paint: "red-900" },
    ],
    mid: [{ part: "water", x: 50, y: 66, paint: "orange-400" }],
    subject: [{ part: "rowboat", x: 50, y: 80, scale: 1.6 }],
    near: [
      { part: "reed", x: 8, y: 98, scale: 1.8, paint: "red-900" },
      { part: "reed", x: 92, y: 99, scale: 1.5, flip: -1, paint: "red-900" },
    ],
    light: [{ part: "god-rays", x: 50, y: 62, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, count: 2, spread: [20, 10], paint: "red-300" }],
  },
};

/** Call to Chaos (Training-Grounds Edition) (Training Grounds #39, Legendary), Alt art: close-up, the moment before the roll: one carved die balanced on an instructor's fingertip in a silent snowy yard; pale sage and cream. The instructor is the normal picture's hooded caster. */
const CALL_TO_CHAOS_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["green-200", "slate-100"],
  light: { x: 66, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 24, y: 34, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 44, y: 104, scale: 1.7, pose: "pose-point", look: normalExtraLookOf("traininggrounds-039", 0) }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, count: 3, spread: [24, 3], paint: "white" }],
    far: [
      { part: "palisade", x: 24, y: 60, paint: "slate-400" },
      { part: "palisade", x: 76, y: 60, flip: -1, paint: "slate-400" },
      { part: "barracks", x: 82, y: 62, scale: 0.8, paint: "stone-500" },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 62 },
      { part: "straw-dummy", x: 14, y: 76, scale: 0.6 },
      { part: "target-board", x: 86, y: 76, scale: 0.8, flip: -1 },
    ],
    subject: [{ part: "die", x: 66, y: 32, scale: 3 }],
    near: [
      { part: "fence-post", x: 90, y: 98, scale: 1.3, flip: -1, paint: "slate-400" },
      { part: "snowfield", x: 50, y: 96 },
      { part: "grass-tuft", x: 10, y: 98, count: 2, spread: [6, 1], paint: "slate-400" },
    ],
    light: [{ part: "halo", x: 66, y: 32, scale: 0.8, paint: "white" }],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.3, paint: "white" },
      { part: "snow", x: 50, y: 70, scale: 1, paint: "green-200" },
    ],
  },
};

/** Goetzmann Game Design (Training Grounds #40, Mythic), Alt art: the morning after: the napkin pinned to a corkboard in a sunny instructors' office, new cards tacked around it in a spiral; peach and sky blue, flat front view. */
const GOETZMANN_GAME_DESIGN_ALT: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-300", "red-300"],
  light: { x: 20, y: 30, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 22, y: 22, scale: 0.7 },
      { part: "cloud", x: 64, y: 24, scale: 1.2, count: 3, spread: [24, 4], paint: "white" },
      { part: "bird", x: 80, y: 36, scale: 0.35, count: 3, spread: [8, 4], paint: "slate-700" },
    ],
    far: [
      { part: "hills", x: 50, y: 74, paint: "green-400" },
      { part: "glass-wall", x: 50, y: 90, scale: 3.4 },
    ],
    mid: [
      { part: "boards", x: 50, y: 90 },
      { part: "bookshelf", x: 90, y: 92, scale: 1.1, flip: -1 },
      { part: "bookshelf", x: 10, y: 92, scale: 1.1 },
    ],
    subject: [
      { part: "corkboard", x: 52, y: 50, scale: 2.4 },
      { part: "napkin", x: 52, y: 52, scale: 1.2 },
      { part: "card", x: 66, y: 50, scale: 0.6 },
      { part: "card", x: 62, y: 38, scale: 0.6 },
      { part: "card", x: 46, y: 36, scale: 0.6 },
      { part: "card", x: 36, y: 46, scale: 0.6 },
      { part: "card", x: 38, y: 62, scale: 0.6 },
      { part: "card", x: 56, y: 66, scale: 0.6 },
      { part: "card", x: 72, y: 64, scale: 0.6 },
      { part: "card", x: 74, y: 40, scale: 0.55 },
      { part: "card", x: 30, y: 34, scale: 0.55 },
    ],
    near: [
      { part: "table", x: 50, y: 104, scale: 3 },
      { part: "cup", x: 30, y: 82, scale: 1.4 },
      { part: "quill", x: 70, y: 82, scale: 1.2 },
      { part: "scroll", x: 52, y: 84, scale: 0.9 },
      { part: "candle", x: 86, y: 82, scale: 1.2 },
      { part: "card", x: 40, y: 84, scale: 0.5, count: 2, spread: [4, 1] },
    ],
    light: [
      { part: "light-shaft", x: 22, y: 20, scale: 1.2, paint: "white" },
      { part: "god-rays", x: 22, y: 22, scale: 1, paint: "white" },
    ],
    particles: [{ part: "motes", x: 40, y: 50, scale: 1.2, count: 2, spread: [20, 12], paint: "white" }],
  },
};

/** R3397000: Training Grounds' alt recipes, by card id. */
export const TRAINING_GROUNDS_ALT_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "traininggrounds-001": SPIKEY_STEVEN_ALT,
  "traininggrounds-002": PERSONAL_TRAINER_ALT,
  "traininggrounds-003": GROUP_LEADER_ALT,
  "traininggrounds-004": BOULDER_FIST_OGRE_ALT,
  "traininggrounds-005": MIDLANE_MENACE_ALT,
  "traininggrounds-008": CLOBBERER_ALT,
  "traininggrounds-010": FAN_OF_KNIVES_ALT,
  "traininggrounds-011": THWOMP_ALT,
  "traininggrounds-015": IN_A_HURRY_ALT,
  "traininggrounds-016": CELESTIAL_CALLER_ALT,
  "traininggrounds-017": FLAMING_FELINOR_ALT,
  "traininggrounds-022": RIGHT_HAND_FELINOR_ALT,
  "traininggrounds-025": TRIGOR_ALT,
  "traininggrounds-028": OCCIDENTAL_ALT,
  "traininggrounds-029": EASTERNER_ALT,
  "traininggrounds-030": FLOODGATES_ALT,
  "traininggrounds-032": SELFLESS_PROTECTOR_ALT,
  "traininggrounds-035": CORPSE_RECYCLER_ALT,
  "traininggrounds-037": SEANCE_ALT,
  "traininggrounds-038": TUTORIAL_MASTER_ALT,
  "traininggrounds-039": CALL_TO_CHAOS_ALT,
  "traininggrounds-040": GOETZMANN_GAME_DESIGN_ALT,
};
