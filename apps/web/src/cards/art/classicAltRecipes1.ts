// R3388000: Classic's alt scene recipes, part 1 of three (#1 to #26), one per card offered Alt art, each drawn from its
// brief (#3388's table; ART.md, "Classic's Alt art restaged"): the card's own subject in another place, time or moment,
// under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one
// from above), its figure the normal picture's person (`normalClassicLookOf`), in Classic's temple-stone look and at or
// above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed alone, at every Alt art
// level. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { CLASSIC_RECIPES_1 } from "./sceneRecipesClassic1.ts";
import { CLASSIC_RECIPES_2 } from "./sceneRecipesClassic2.ts";
import { CLASSIC_RECIPES_3 } from "./sceneRecipesClassic3.ts";

/** The card's normal recipe, from the three Classic tables. */
function normalOf(id: string): SceneRecipe | undefined {
  return CLASSIC_RECIPES_1[id] ?? CLASSIC_RECIPES_2[id] ?? CLASSIC_RECIPES_3[id];
}

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
export function normalClassicLookOf(id: string): FigureLook {
  return normalOf(id)?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a crowd its brief keeps is the same people; with none, the default person. */
export function normalClassicExtraLookOf(id: string, k: number): FigureLook {
  return normalOf(id)?.extras?.[k]?.look ?? figureLookFor([]);
}

/** Curse of the Forgotten Classic (Classic #1, Rare), Alt art: Close-up of the cursed hero's face lit from below by pale green exile-ghosts circling him in a snowbound mountain pass at dawn; white and pale green. */
const CURSE_OF_THE_FORGOTTEN_CLASSIC_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-100", "green-200"],
  light: { x: 50, y: 96, r: 44, paint: "green-200", opacity: 0.5 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 118, scale: 1.7, pose: "pose-stand", look: { ...normalClassicLookOf("classic-001") } },
  layers: {
    sky: [
      { part: "sun-disc", x: 22, y: 34, scale: 0.8 },
      { part: "cloud", x: 70, y: 24, scale: 1.2, count: 2, spread: [16, 3], paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 46, paint: "slate-400" },
      { part: "snowfield", x: 50, y: 60 },
    ],
    mid: [
      { part: "pine", x: 10, y: 72, scale: 1.6, count: 2, spread: [5, 2], paint: "green-900" },
      { part: "pine", x: 90, y: 70, scale: 1.4, count: 2, spread: [5, 2], paint: "green-900" },
      { part: "ghost", x: 50, y: 34, scale: 1.2, paint: "green-200", fade: 0.7 },
    ],
    near: [
      { part: "ghost", x: 18, y: 58, scale: 1.6, paint: "green-200", fade: 0.8 },
      { part: "ghost", x: 82, y: 62, scale: 1.5, flip: -1, paint: "green-200", fade: 0.8 },
      { part: "snow", x: 50, y: 96, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 88, scale: 1.8, paint: "green-200" },
      { part: "rim-light", x: 50, y: 70, scale: 1.4, paint: "green-200" },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4 }],
  },
};

/** Palantir (Classic #4, Legendary), Alt art: Wide vista from outside: the Jlockeed watchtower on a rust-red desert ridge at noon, the orb's beam sweeping a rival army's camp below; red sand, pale sky. Stand-ins: cyan `god-rays` fanning down from the orb for its sweeping beam. */
const PALANTIR_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "blue-200"],
  light: { x: 30, y: 30, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun-disc", x: 70, y: 22, scale: 1 },
      { part: "cloud", x: 52, y: 26, scale: 1.1, count: 2, spread: [20, 3], paint: "white" },
    ],
    far: [
      { part: "mesa", x: 84, y: 56, scale: 0.8, paint: "red-600" },
      { part: "hills", x: 50, y: 58, paint: "red-900" },
    ],
    mid: [
      { part: "sand", x: 50, y: 66, paint: "red-600" },
      { part: "tower", x: 30, y: 72, scale: 1.4 },
      { part: "tent", x: 62, y: 74, scale: 0.7 },
      { part: "tent", x: 76, y: 76, scale: 0.75, flip: -1 },
      { part: "tent", x: 88, y: 73, scale: 0.6 },
      { part: "flag", x: 70, y: 72, scale: 0.6, paint: "blue-700" },
    ],
    subject: [
      { part: "orb", x: 30, y: 38, scale: 0.9, paint: "slate-900" },
      { part: "eye", x: 30, y: 38, scale: 0.3 },
    ],
    near: [
      { part: "cactus", x: 10, y: 96, scale: 1.2 },
      { part: "pebbles", x: 60, y: 98, count: 2, spread: [16, 1] },
      { part: "tall-grass", x: 92, y: 98, scale: 1, paint: "wood-600" },
    ],
    light: [
      { part: "god-rays", x: 30, y: 38, scale: 0.7, paint: "cyan-400" },
      { part: "halo", x: 30, y: 38, scale: 0.9, paint: "cyan-400" },
      { part: "halo", x: 74, y: 76, scale: 1, paint: "cyan-400" },
    ],
    particles: [{ part: "dust", x: 60, y: 80, scale: 1, paint: "red-300" }],
  },
};

/** Cloaked Toe Cracker (Classic #6, Common), Alt art: Overhead view of the barefoot rogue tiptoeing through a sunlit autumn orchard strewn with disarmed snares, cloak full of leaves; russet and orange. */
const CLOAKED_TOE_CRACKER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "orange-400"],
  light: { x: 70, y: 20, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  figure: { x: 50, y: 74, scale: 0.6, pose: "pose-crouch", look: { ...normalClassicLookOf("classic-006"), cloth: "wood-600" } },
  layers: {
    far: [
      { part: "field", x: 50, y: 30, paint: "wood-600" },
      { part: "fruit-tree", x: 14, y: 44, scale: 0.9 },
      { part: "fruit-tree", x: 84, y: 42, scale: 0.85, flip: -1 },
    ],
    mid: [
      { part: "fruit-tree", x: 22, y: 70, scale: 1.1 },
      { part: "fruit-tree", x: 82, y: 72, scale: 1.2, flip: -1 },
      { part: "leaves", x: 50, y: 50, scale: 1.4, count: 2, spread: [20, 6] },
    ],
    subject: [
      { part: "snare", x: 36, y: 78, scale: 1.2 },
      { part: "snare", x: 64, y: 80, scale: 1.1 },
      { part: "snare", x: 52, y: 62, scale: 0.9 },
    ],
    near: [
      { part: "leaves", x: 50, y: 70, scale: 1 },
      { part: "grass-tuft", x: 12, y: 96, count: 3, spread: [8, 2], paint: "orange-400" },
    ],
    particles: [{ part: "leaves", x: 30, y: 34, scale: 1.2 }],
  },
};

/** InfiniScepter (Classic #7, Legendary), Alt art: Close-up of the crystal sceptre held aloft on a stormy sea-cliff, its trapped spell crackling white as waves crash below; slate grey and sea green. Stand-ins: a gloved `reaching-hand` for the hand holding it. */
const INFINISCEPTER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "slate-400"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.6, count: 3, spread: [24, 4], paint: "slate-700" },
      { part: "cloud", x: 76, y: 20, scale: 1.4, count: 2, spread: [14, 3], paint: "slate-800" },
      { part: "lightning", x: 82, y: 10, scale: 1 },
      { part: "lightning", x: 18, y: 12, scale: 0.7 },
    ],
    far: [
      { part: "sea", x: 50, y: 60, paint: "jade-400" },
      { part: "wave", x: 84, y: 66, scale: 0.7, flip: -1 },
      { part: "wave", x: 14, y: 64, scale: 0.5 },
    ],
    mid: [
      { part: "cliff", x: 40, y: 70, paint: "slate-700" },
      { part: "spray", x: 80, y: 62, scale: 1.4, paint: "white" },
    ],
    subject: [
      { part: "sceptre", x: 52, y: 48, scale: 2.2 },
      { part: "reaching-hand", x: 40, y: 80, scale: 1.3 },
      { part: "zap", x: 56, y: 30, scale: 0.8, paint: "white" },
    ],
    near: [
      { part: "spray", x: 16, y: 88, scale: 1.6, paint: "white" },
      { part: "spray", x: 86, y: 76, scale: 1.2, paint: "white" },
      { part: "rain", x: 50, y: 70, scale: 1.2, paint: "slate-100" },
      { part: "gull", x: 18, y: 40, scale: 0.9, count: 3, spread: [10, 6] },
    ],
    light: [
      { part: "halo", x: 54, y: 30, scale: 1.4, paint: "white" },
      { part: "rim-light", x: 52, y: 50, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 54, y: 30, scale: 1.4, count: 2, spread: [10, 6], paint: "white" },
      { part: "drops", x: 70, y: 56, scale: 0.8, paint: "jade-400" },
      { part: "rain", x: 50, y: 34, scale: 1.4, paint: "slate-100" },
    ],
  },
};

/** Pickle (Classic #8, Rare), Alt art: Wide view of the giant pickle jar sunk in a summer picnic meadow, the duelist dithering between three paths through red poppies; red and sky blue. Stand-ins: three `lanes` for the three paths, red `flowers` for the poppies. */
const PICKLE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "sky-300"],
  light: { x: 76, y: 18, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 30, y: 80, scale: 0.5, pose: "pose-block", look: { ...normalClassicLookOf("classic-008") } },
  layers: {
    sky: [
      { part: "sun-disc", x: 80, y: 18, scale: 0.9 },
      { part: "cloud", x: 40, y: 24, scale: 1.2, count: 2, spread: [20, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "tree", x: 10, y: 60, scale: 0.9 },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "green-500" },
      { part: "lanes", x: 52, y: 62, paint: "wood-600" },
    ],
    subject: [
      { part: "pickle", x: 74, y: 70, scale: 0.75 },
      { part: "jar", x: 74, y: 72, scale: 3.6, paint: "green-200" },
    ],
    near: [
      { part: "flowers", x: 12, y: 96, scale: 1.4, count: 3, spread: [8, 2], paint: "red-500" },
      { part: "flowers", x: 88, y: 96, scale: 1.3, count: 3, spread: [8, 2], paint: "red-500" },
      { part: "flowers", x: 34, y: 76, scale: 0.8, count: 2, spread: [6, 1], paint: "red-500" },
    ],
    particles: [{ part: "petals", x: 50, y: 50, scale: 1.2, paint: "red-500" }],
  },
};

/** Income Tax (Classic #9, Legendary), Alt art: After the raid: the tax collector's cart rolls down a snowy mountain road heaped with card backs, one lonely card left in the snow; white and slate blue. */
const INCOME_TAX_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 78, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 70, scale: 0.55, pose: "pose-hold", look: { ...normalClassicLookOf("classic-009") } },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.3, count: 3, spread: [26, 4], paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 52, paint: "blue-800" },
      { part: "pine", x: 86, y: 58, scale: 0.7, count: 3, spread: [8, 1], paint: "green-900" },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 60 },
      { part: "road", x: 40, y: 60, paint: "slate-400" },
      { part: "pine", x: 12, y: 70, scale: 1.2, count: 2, spread: [5, 2], paint: "green-900" },
    ],
    subject: [
      { part: "cart", x: 52, y: 72, scale: 1 },
      { part: "playing-card", x: 50, y: 64, scale: 0.7, count: 6, spread: [5, 2], paint: "blue-800" },
    ],
    near: [
      { part: "snowfield", x: 50, y: 86 },
      { part: "playing-card", x: 74, y: 82, scale: 1.4, paint: "blue-800" },
    ],
    light: [
      { part: "halo", x: 74, y: 76, scale: 0.8, paint: "white" },
      { part: "rim-light", x: 50, y: 64, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "snow", x: 50, y: 78, scale: 1.2 },
    ],
  },
};

/** Exile (Classic #10, Common), Alt art: From the far side: the tiny imp tumbling onto a distant icy moon, the portal a small dot in a pale sky; wide vista in icy white and cyan. Stand-ins: a small `void-swirl` for the portal. */
const EXILE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "slate-100"],
  light: { x: 70, y: 26, r: 34, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "void-swirl", x: 72, y: 26, scale: 0.3, paint: "blue-700" },
      { part: "planet", x: 20, y: 28, scale: 0.6 },
      { part: "stars", x: 50, y: 24, scale: 1, paint: "white" },
    ],
    far: [{ part: "moon-surface", x: 50, y: 66 }],
    mid: [
      { part: "ice-block", x: 18, y: 76, scale: 1.2 },
      { part: "ice-shard", x: 84, y: 74, scale: 1, count: 2, spread: [6, 2] },
    ],
    subject: [
      { part: "card-imp", x: 46, y: 62, scale: 0.6 },
      { part: "streaks", x: 52, y: 52, scale: 0.4, paint: "cyan-400" },
    ],
    near: [{ part: "frost", x: 50, y: 92, scale: 1.2 }],
    particles: [{ part: "sparks", x: 60, y: 40, scale: 1, count: 2, spread: [12, 8], paint: "cyan-400" }],
  },
};

/** Book of Blood (Classic #12, Epic), Alt art: Close-up of a scholar's pale hands on the crimson tome in a frosty birch forest, the red stream winding away across snow; white with red. Stand-ins: a `road` painted red for the stream, white `tree`s for the birches. */
const BOOK_OF_BLOOD_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-100", "blue-200"],
  light: { x: 50, y: 70, r: 40, paint: "red-400", opacity: 0.3 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 34, scale: 1.4, paint: "white" }],
    far: [
      { part: "tree", x: 14, y: 56, scale: 1.2, paint: "white" },
      { part: "tree", x: 40, y: 54, scale: 0.9, paint: "white" },
      { part: "tree", x: 86, y: 55, scale: 1.1, flip: -1, paint: "white" },
      { part: "snowfield", x: 50, y: 56 },
    ],
    mid: [
      { part: "road", x: 62, y: 56, paint: "red-600" },
      { part: "tree", x: 92, y: 72, scale: 1.6, flip: -1, paint: "white" },
    ],
    subject: [
      { part: "tome", x: 50, y: 84, scale: 3, paint: "red-600" },
      { part: "open-hand", x: 22, y: 82, scale: 2 },
      { part: "open-hand", x: 78, y: 82, scale: 2, flip: -1 },
    ],
    near: [
      { part: "snow", x: 50, y: 60, scale: 1.4 },
      { part: "drops", x: 54, y: 66, scale: 0.8, paint: "red-600" },
    ],
    light: [{ part: "halo", x: 50, y: 74, scale: 1.4, paint: "red-400" }],
    particles: [{ part: "snow", x: 50, y: 30, scale: 1.2 }],
  },
};

/** Boots on the Ground (Classic #13, Common), Alt art: Ground-level close-up of the soldier's boots splashing through a bright spring meadow of wildflowers at midday, card tucked in his belt; vivid green. */
const BOOTS_ON_THE_GROUND_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "sky-300"],
  light: { x: 70, y: 14, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 84, scale: 2.6, pose: "pose-run", look: { ...normalClassicLookOf("classic-013"), item: "item-card" } },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 24, scale: 1.2, count: 2, spread: [14, 3], paint: "white" }],
    far: [{ part: "hills", x: 50, y: 60, paint: "green-500" }],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-500" },
      { part: "flowers", x: 14, y: 78, scale: 1.4, count: 3, spread: [8, 2] },
      { part: "flowers", x: 86, y: 80, scale: 1.4, count: 3, spread: [8, 2], paint: "violet-300" },
    ],
    near: [
      { part: "spray", x: 40, y: 82, scale: 1.4, paint: "white" },
      { part: "drops", x: 60, y: 78, scale: 1.2, paint: "sky-300" },
      { part: "grass-tuft", x: 50, y: 92, count: 4, spread: [30, 2], paint: "green-500" },
    ],
    particles: [{ part: "petals", x: 50, y: 50, scale: 1.2 }],
  },
};

/** Shadowstep (Classic #14, Common), Alt art: A sunbaked desert market at midday: the swordsman's shadow peels off the sand and darts into a merchant's hand under turquoise awnings; sand and turquoise. Stand-ins: a black `ghost-figure` for the swordsman's shadow. */
const SHADOWSTEP_ALT: SceneRecipe = {
  family: "classic",
  sky: ["cyan-400", "orange-400"],
  light: { x: 50, y: 14, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 26, size: 5, rotate: 0 },
  figure: { x: 26, y: 78, scale: 0.5, pose: "pose-spread", look: { ...normalClassicLookOf("classic-014") } },
  extras: [
    {
      layer: "subject",
      x: 74,
      y: 78,
      scale: 0.5,
      flip: -1,
      pose: "pose-reach",
      look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-kaftan", item: null, cloth: "cyan-400" },
    },
  ],
  layers: {
    sky: [{ part: "sun-disc", x: 50, y: 12, scale: 1 }],
    far: [
      { part: "whitewashed-house", x: 14, y: 62, scale: 0.9 },
      { part: "whitewashed-house", x: 86, y: 62, scale: 0.9, flip: -1 },
    ],
    mid: [
      { part: "sand", x: 50, y: 62, paint: "brass-500" },
      { part: "market-stall", x: 20, y: 70, scale: 0.8 },
      { part: "market-stall", x: 80, y: 70, scale: 0.8, flip: -1 },
      { part: "awning", x: 50, y: 62, scale: 0.9, paint: "cyan-400" },
    ],
    subject: [
      { part: "ghost-figure", x: 52, y: 78, scale: 0.9, paint: "black" },
      { part: "streaks", x: 40, y: 70, scale: 0.6, paint: "slate-900" },
    ],
    near: [{ part: "basket", x: 90, y: 96, scale: 1.2 }],
    particles: [{ part: "dust", x: 40, y: 82, scale: 1, paint: "brass-600" }],
  },
};

/** Nose Hunter (Classic #15, Common), Alt art: Close-up portrait of the tracker's huge nose sniffing a card held up in a lavender field at midday; purple and green. Stand-ins: violet `flowers` for the lavender. */
const NOSE_HUNTER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "violet-300"],
  light: { x: 70, y: 20, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 48, y: 118, scale: 1.7, pose: "pose-hold", look: { ...normalClassicLookOf("classic-015") } },
  layers: {
    sky: [{ part: "sun-disc", x: 78, y: 20, scale: 0.8 }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "field", x: 50, y: 64, paint: "violet-500" },
    ],
    mid: [
      { part: "flowers", x: 14, y: 76, scale: 1.4, count: 4, spread: [10, 3], paint: "violet-500" },
      { part: "flowers", x: 86, y: 76, scale: 1.4, count: 4, spread: [10, 3], paint: "violet-500" },
    ],
    near: [{ part: "flowers", x: 50, y: 98, scale: 2, count: 3, spread: [40, 2], paint: "violet-300" }],
    particles: [{ part: "honeybee", x: 72, y: 46, scale: 0.8 }],
  },
};

/** Counterspell (Classic #17, Common), Alt art: Wide vista: a warden on a sea stack at sunset swats a meteor-like spell into the ocean, spray flaring; coral pink and deep teal. Stand-ins: a `mesa` for the sea stack. */
const COUNTERSPELL_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-300", "orange-400"],
  light: { x: 70, y: 54, r: 38, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 36, y: 59, scale: 0.45, pose: "pose-block", look: { ...normalClassicLookOf("classic-017") } },
  layers: {
    sky: [
      { part: "sun-disc", x: 72, y: 54, scale: 1.4 },
      { part: "cloud", x: 30, y: 24, scale: 1.2, count: 2, spread: [16, 3], paint: "red-300" },
    ],
    far: [{ part: "sea", x: 50, y: 58, paint: "green-900" }],
    mid: [{ part: "mesa", x: 36, y: 75, scale: 0.8, paint: "wood-700" }],
    subject: [
      { part: "meteor", x: 42, y: 54, scale: 0.7 },
      { part: "spray", x: 70, y: 66, scale: 1.4, paint: "white" },
    ],
    near: [{ part: "spray", x: 50, y: 92, scale: 1, paint: "sky-300" }],
    particles: [{ part: "sparks", x: 42, y: 54, count: 2, spread: [8, 4], paint: "orange-400" }],
  },
};

/** Lizard's Breath (Classic #19, Rare), Alt art: Before the breath: the lizard coiled asleep on a frozen glacier lake, three-coloured wisps fogging from its nostrils in the cold; ice blue and white. Stand-ins: three `steam-curl`s, one per colour, for the wisps. */
const LIZARDS_BREATH_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "slate-100"],
  light: { x: 30, y: 24, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 24, scale: 1.2, count: 2, spread: [20, 3], paint: "white" }],
    far: [{ part: "mountains", x: 50, y: 56, paint: "slate-400" }],
    mid: [
      { part: "ice-floor", x: 50, y: 62 },
      { part: "ice-block", x: 12, y: 74, scale: 1.4 },
      { part: "ice-shard", x: 88, y: 72, scale: 1.2, count: 2, spread: [5, 2] },
    ],
    subject: [
      { part: "lizard", x: 44, y: 76, scale: 1.6 },
      { part: "steam-curl", x: 64, y: 58, scale: 0.8, paint: "green-200" },
      { part: "steam-curl", x: 67, y: 59, scale: 0.7, paint: "violet-300" },
      { part: "steam-curl", x: 70, y: 60, scale: 0.6, paint: "red-300" },
    ],
    near: [{ part: "frost", x: 50, y: 94, scale: 1.2 }],
    light: [{ part: "rim-light", x: 44, y: 64, scale: 1.4, paint: "white" }],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.2 }],
  },
};

/** The Power to Punish (Classic #20, Rare), Alt art: Outside the tribunal on a misty marsh island at dawn, its great balance silhouetted on the rooftop against pale green fog; green and grey. Stand-ins: a `temple` for the tribunal. */
const THE_POWER_TO_PUNISH_ALT: SceneRecipe = {
  family: "classic",
  sky: ["green-200", "slate-400"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 1.4, count: 2, spread: [30, 4], paint: "green-200" }],
    far: [
      { part: "still-water", x: 50, y: 64, paint: "slate-400" },
      { part: "reed", x: 12, y: 70, scale: 1.4, count: 3, spread: [6, 2] },
      { part: "reed", x: 88, y: 70, scale: 1.4, count: 3, spread: [6, 2] },
    ],
    mid: [
      { part: "temple", x: 50, y: 74, scale: 1.1, paint: "slate-700" },
      { part: "mist", x: 50, y: 72, scale: 1.6, paint: "green-200" },
    ],
    subject: [{ part: "scales", x: 50, y: 42, scale: 1.1, paint: "slate-800" }],
    near: [
      { part: "reed", x: 6, y: 98, scale: 2, count: 2, spread: [4, 1] },
      { part: "mist", x: 50, y: 94, scale: 1.4, paint: "slate-100" },
    ],
    light: [{ part: "halo", x: 50, y: 34, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Turtinator (Classic #21, Common), Alt art: Underwater: the turret-tortoise swims through a sunlit coral lagoon, cannon swivelling toward a shark; turquoise and coral. Stand-ins: red and orange `bush`es and `fern`s for the coral. */
const TURTINATOR_ALT: SceneRecipe = {
  family: "classic",
  sky: ["cyan-400", "jade-400"],
  light: { x: 40, y: 8, r: 44, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "light-shaft", x: 36, y: 0, scale: 1.4, paint: "white" }],
    far: [{ part: "sand", x: 50, y: 70, paint: "orange-400" }],
    mid: [
      { part: "bush", x: 12, y: 78, scale: 1.2, paint: "red-500" },
      { part: "fern", x: 26, y: 80, scale: 1.4, paint: "red-300" },
      { part: "bush", x: 90, y: 78, scale: 1, paint: "red-300" },
      { part: "fern", x: 62, y: 76, scale: 1.2, paint: "red-500" },
    ],
    subject: [
      { part: "tortoise", x: 36, y: 62, scale: 1 },
      { part: "shark", x: 78, y: 36, scale: 0.8, flip: -1 },
    ],
    near: [
      { part: "fern", x: 70, y: 98, scale: 1.6, paint: "red-500" },
      { part: "bubbles", x: 14, y: 50, scale: 1.2 },
    ],
    particles: [{ part: "bubbles", x: 46, y: 40, scale: 1 }],
  },
};

/** Lag in the System (Classic #25, Epic), Alt art: Close-up of one cheap goblin frozen mid-leap over a lush summer river, its edges stuttering into afterimages; vivid green and orange. */
const LAG_IN_THE_SYSTEM_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "lime-200"],
  light: { x: 76, y: 20, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun-disc", x: 78, y: 20, scale: 1 }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-500" },
      { part: "tree", x: 12, y: 62, scale: 1.2 },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-500" },
      { part: "water", x: 50, y: 72, paint: "blue-600" },
    ],
    subject: [
      { part: "goblin", x: 52, y: 46, scale: 4 },
      { part: "goblin", x: 44, y: 48, scale: 4, fade: 0.35 },
      { part: "goblin", x: 36, y: 50, scale: 4, fade: 0.2 },
    ],
    near: [
      { part: "reed", x: 8, y: 98, scale: 2, count: 2, spread: [4, 1] },
      { part: "bush", x: 92, y: 98, scale: 1.4, paint: "green-500" },
    ],
    light: [{ part: "rim-light", x: 52, y: 46, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "pixels", x: 44, y: 46, scale: 1.2 }],
  },
};

/** Rapid Draw (Classic #26, Common), Alt art: A snowy winter rooftop at noon: the duelist's discarded card flutters down over a festival street below; white and red. Stand-ins: a `snowfield` across the foreground for the snowy roof. */
const RAPID_DRAW_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-100", "blue-200"],
  light: { x: 70, y: 20, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 26, y: 76, scale: 0.6, pose: "pose-cast", look: { ...normalClassicLookOf("classic-026"), cloth: "red-600" } },
  layers: {
    sky: [{ part: "cloud", x: 60, y: 24, scale: 1.2, count: 2, spread: [16, 3], paint: "white" }],
    far: [
      { part: "townhouse", x: 20, y: 70, scale: 0.6 },
      { part: "townhouse", x: 52, y: 72, scale: 0.65 },
      { part: "townhouse", x: 84, y: 70, scale: 0.6 },
    ],
    mid: [
      { part: "street", x: 60, y: 64, paint: "slate-700" },
      { part: "bunting", x: 50, y: 56, scale: 1 },
      { part: "lantern-string", x: 70, y: 62, scale: 0.8 },
    ],
    subject: [
      { part: "snowfield", x: 50, y: 76 },
      { part: "playing-card", x: 60, y: 44, scale: 1, paint: "red-600" },
    ],
    near: [{ part: "snow", x: 50, y: 90, scale: 1.2 }],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "confetti", x: 76, y: 60, scale: 0.8 },
    ],
  },
};

/** R3388000: Classic's alt recipes #1 to #26, by card id. */
export const CLASSIC_ALT_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "classic-001": CURSE_OF_THE_FORGOTTEN_CLASSIC_ALT,
  "classic-004": PALANTIR_ALT,
  "classic-006": CLOAKED_TOE_CRACKER_ALT,
  "classic-007": INFINISCEPTER_ALT,
  "classic-008": PICKLE_ALT,
  "classic-009": INCOME_TAX_ALT,
  "classic-010": EXILE_ALT,
  "classic-012": BOOK_OF_BLOOD_ALT,
  "classic-013": BOOTS_ON_THE_GROUND_ALT,
  "classic-014": SHADOWSTEP_ALT,
  "classic-015": NOSE_HUNTER_ALT,
  "classic-017": COUNTERSPELL_ALT,
  "classic-019": LIZARDS_BREATH_ALT,
  "classic-020": THE_POWER_TO_PUNISH_ALT,
  "classic-021": TURTINATOR_ALT,
  "classic-025": LAG_IN_THE_SYSTEM_ALT,
  "classic-026": RAPID_DRAW_ALT,
};
