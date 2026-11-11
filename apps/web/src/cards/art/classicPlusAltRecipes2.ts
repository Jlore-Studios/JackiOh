// R3396000: Classic+'s alt scene recipes, part 2 of two (#37 to #78), one per card offered Alt art, each drawn from
// its brief (ART.md, "Classic+'s Alt art restaged"): the card's own subject in another place, time or moment, under a
// sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one from
// above), its figure the normal picture's person (classicPlusAltRecipes1.ts `normalLookOf`), in Classic's
// temple-stone look and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art
// seed alone, at every Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { normalExtraLookOf, normalLookOf } from "./classicPlusAltRecipes1.ts";
import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A villager of the Alt art scenes' markets, fairs and harbours: bareheaded, in a plain tunic. */
const VILLAGER: FigureLook = { ...figureLookFor(["Human"]), outfit: "outfit-tunic", item: null, cloth: "wood-700" };

/** Wardrum (Classic+ #37, Legendary), Alt art: marching alone across a misty highland at dawn, mallets raised, each beat rippling the heather in rings; a distant figure. */
const WARDRUM_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "green-200"],
  light: { x: 18, y: 52, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 60, y: 72, scale: 0.55, pose: "pose-strike", look: { ...normalLookOf("classicplus-037") } },
  layers: {
    sky: [
      { part: "cloud", x: 70, y: 22, scale: 0.9, count: 3, spread: [18, 4], paint: "slate-100" },
      { part: "mist", x: 30, y: 40, scale: 1.2, count: 2, spread: [14, 3] },
    ],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "hills", x: 50, y: 62, paint: "green-900" },
      { part: "standing-stone", x: 16, y: 64, scale: 0.6 },
      { part: "standing-stone", x: 24, y: 63, scale: 0.45, flip: -1 },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "green-700" },
      { part: "shockwave", x: 60, y: 72, scale: 1.3, paint: "green-200" },
      { part: "shockwave", x: 60, y: 72, scale: 0.7, paint: "slate-100" },
      { part: "mist", x: 40, y: 68, scale: 1, count: 2, spread: [22, 3] },
      { part: "flowers", x: 30, y: 78, scale: 1, count: 3, spread: [16, 3], paint: "violet-300" },
    ],
    subject: [{ part: "shockwave", x: 60, y: 72, scale: 0.4, paint: "green-200" }],
    near: [
      { part: "grass-tuft", x: 50, y: 97, count: 4, spread: [40, 2], paint: "green-900" },
      { part: "fern", x: 10, y: 98, scale: 1.6 },
      { part: "flowers", x: 84, y: 96, scale: 1.4, count: 2, spread: [6, 2], paint: "violet-300" },
    ],
    light: [
      { part: "halo", x: 18, y: 52, scale: 2.4, paint: "white" },
      { part: "light-shaft", x: 22, y: 0, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 60, y: 58, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "white" },
      { part: "petals", x: 60, y: 60, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** KY's Constant (Classic+ #41, Rare), Alt art: KY sketching triangles in wet sand on a windy beach at sunset while the tide smooths every other shape away; wide. */
const KYS_CONSTANT_ALT: SceneRecipe = {
  family: "classic",
  sky: ["violet-500", "red-300"],
  light: { x: 74, y: 56, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  figure: { x: 36, y: 78, scale: 0.55, pose: "pose-kneel", look: { ...normalLookOf("classicplus-041"), cloth: "blue-800" } },
  layers: {
    sky: [
      { part: "sun", x: 74, y: 56, scale: 0.9 },
      { part: "cloud", x: 30, y: 26, scale: 0.8, count: 2, spread: [16, 4], paint: "red-300" },
    ],
    far: [{ part: "water", x: 50, y: 58, paint: "blue-600" }],
    mid: [
      { part: "rock-floor", x: 50, y: 70, paint: "brass-500" },
      { part: "mist", x: 70, y: 72, scale: 1, paint: "white" },
      { part: "rune-circle", x: 54, y: 82, scale: 0.6, paint: "slate-100" },
    ],
    near: [
      { part: "mist", x: 50, y: 92, scale: 1.4, count: 2, spread: [20, 2], paint: "white" },
      { part: "pebbles", x: 18, y: 96, count: 2, spread: [8, 2] },
      { part: "grass-tuft", x: 88, y: 97, count: 2, spread: [5, 1], paint: "wood-600" },
    ],
    light: [{ part: "halo", x: 74, y: 56, scale: 2, paint: "orange-400" }],
    particles: [{ part: "dust", x: 56, y: 70, scale: 1.2, paint: "brass-500" }],
  },
};

/** AI Slop (Classic+ #43, Legendary), Alt art: the uncanny melting creature waddling through a sleepy village market at noon, villagers squinting at its extra fingers; street level. */
const AI_SLOP_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "slate-100"],
  light: { x: 80, y: 18, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 24, y: 76, scale: 0.42, pose: "pose-stand", look: { ...VILLAGER, hair: "kerchief", cloth: "green-700" } },
    { layer: "mid", x: 76, y: 74, scale: 0.4, flip: -1, pose: "pose-raise", look: { ...VILLAGER, hair: "hat-brim", cloth: "brass-600" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.6 },
      { part: "cloud", x: 26, y: 22, scale: 0.8, count: 2, spread: [14, 3], paint: "white" },
    ],
    far: [
      { part: "house", x: 12, y: 64, scale: 1 },
      { part: "house", x: 88, y: 62, scale: 0.9, flip: -1 },
      { part: "tower", x: 62, y: 58, scale: 0.5 },
      { part: "tree", x: 36, y: 62, scale: 0.8, paint: "green-500" },
    ],
    mid: [
      { part: "street", x: 50, y: 64, paint: "stone-500" },
      { part: "awning", x: 16, y: 78, scale: 0.9 },
      { part: "awning", x: 84, y: 76, scale: 0.85, flip: -1 },
    ],
    subject: [
      { part: "boar", x: 48, y: 84, scale: 1.6, paint: "violet-500" },
      { part: "pale-hand", x: 32, y: 74, scale: 0.9, paint: "purple-400" },
      { part: "pale-hand", x: 62, y: 72, scale: 0.8, flip: -1 },
      { part: "glitch", x: 48, y: 68, scale: 0.9 },
    ],
    near: [
      { part: "crate", x: 10, y: 97, scale: 1.3 },
      { part: "crate", x: 90, y: 98, scale: 1.1 },
      { part: "grass-tuft", x: 70, y: 98, count: 2, spread: [6, 1], paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 80, y: 18, scale: 2, paint: "white" },
      { part: "rim-light", x: 48, y: 72, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "glitch", x: 50, y: 60, scale: 0.8, count: 2, spread: [16, 6] },
      { part: "sparks", x: 50, y: 70, scale: 1, paint: "purple-400" },
    ],
  },
};

/** Felinor Flagbearer (Classic+ #46, Legendary), Alt art: at rest on a sunny lakeshore, banner planted, a crowd of kittens napping in its shade; a close-up portrait in soft lilac and meadow green. */
const FELINOR_FLAGBEARER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["violet-300", "blue-200"],
  light: { x: 80, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 80, y: 74, size: 5, rotate: 0 },
  figure: { x: 42, y: 116, scale: 1.7, pose: "pose-stand", look: { ...normalLookOf("classicplus-046") } },
  layers: {
    sky: [
      { part: "sun", x: 82, y: 16, scale: 0.5 },
      { part: "cloud", x: 66, y: 24, scale: 0.8, count: 2, spread: [12, 3], paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 52, paint: "violet-500" },
      { part: "water", x: 50, y: 56, paint: "sky-400" },
      { part: "banner", x: 86, y: 58, scale: 0.6, props: true },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "green-500" },
      { part: "flowers", x: 78, y: 76, scale: 1.2, count: 3, spread: [10, 3], paint: "violet-300" },
      { part: "tree", x: 90, y: 70, scale: 1.1, paint: "green-700" },
    ],
    near: [
      { part: "banner", x: 76, y: 100, scale: 1.6, flip: -1, props: true },
      { part: "cat", x: 16, y: 97, scale: 1.4 },
      { part: "cat", x: 64, y: 98, scale: 1.2, flip: -1, paint: "slate-400" },
      { part: "cat", x: 88, y: 96, scale: 1.1, paint: "wood-600" },
      { part: "grass-tuft", x: 40, y: 99, count: 3, spread: [20, 1], paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 80, y: 18, scale: 2, paint: "white" },
      { part: "rim-light", x: 42, y: 64, scale: 1.6, paint: "white" },
    ],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.2, paint: "violet-300" },
      { part: "sparks", x: 64, y: 34, scale: 1, count: 2, spread: [12, 6], paint: "white" },
    ],
  },
};

/** Adaptive Growth (Classic+ #50, Epic), Alt art: a red-rock canyon floor where one army's cacti swell into towering spires while the other's wither and shrink; a wide panorama. */
const ADAPTIVE_GROWTH_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-600", "orange-400"],
  light: { x: 22, y: 26, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 78, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 22, y: 26, scale: 0.7 }],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "red-900" },
      { part: "cliff", x: 16, y: 58, scale: 1, paint: "red-600" },
      { part: "cliff", x: 86, y: 60, scale: 0.9, flip: -1, paint: "red-900" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "brass-600" },
      { part: "dead-tree", x: 90, y: 74, scale: 0.8 },
    ],
    subject: [
      { part: "cactus", x: 32, y: 86, scale: 2.2 },
      { part: "cactus", x: 46, y: 84, scale: 1.7 },
      { part: "vine", x: 32, y: 86, scale: 1.6 },
      { part: "cactus", x: 64, y: 80, scale: 0.65, paint: "wood-700" },
      { part: "cactus", x: 74, y: 82, scale: 0.55, flip: -1, paint: "wood-700" },
      { part: "cactus", x: 84, y: 80, scale: 0.5, paint: "wood-700" },
    ],
    near: [
      { part: "pebbles", x: 60, y: 96, count: 2, spread: [16, 2] },
      { part: "grass-tuft", x: 86, y: 98, count: 3, spread: [8, 1], paint: "wood-600" },
      { part: "standing-stone", x: 8, y: 100, scale: 0.6, paint: "red-900" },
    ],
    light: [
      { part: "halo", x: 22, y: 26, scale: 2, paint: "orange-400" },
      { part: "light-shaft", x: 28, y: 0, scale: 1.2, paint: "orange-400" },
    ],
    particles: [
      { part: "dust", x: 60, y: 64, scale: 1.4, paint: "orange-400" },
      { part: "petals", x: 70, y: 66, scale: 1, paint: "wood-600" },
    ],
  },
};

/** Jlockeed's J15 Fighter (Classic+ #51, Epic), Alt art: skimming low over a sunset ocean, its shadow racing across the waves, the archers' arrows far behind; a low chase angle. */
const J15_FIGHTER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "orange-400"],
  light: { x: 78, y: 52, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 22, y: 30, size: 4, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 78, y: 54, scale: 1.1 },
      { part: "cloud", x: 30, y: 26, scale: 0.8, count: 2, spread: [16, 4], paint: "violet-500" },
    ],
    far: [
      { part: "water", x: 50, y: 60, paint: "blue-900" },
      { part: "arrow", x: 14, y: 52, scale: 0.5, count: 3, spread: [6, 3] },
    ],
    mid: [
      { part: "jet", x: 46, y: 78, scale: 1.4, paint: "slate-900" },
      { part: "mist", x: 60, y: 72, scale: 1, paint: "orange-400" },
    ],
    subject: [{ part: "jet", x: 48, y: 56, scale: 1.9 }],
    near: [
      { part: "mist", x: 50, y: 94, scale: 1.4, count: 2, spread: [20, 2], paint: "white" },
      { part: "bubbles", x: 24, y: 90, scale: 1 },
    ],
    light: [
      { part: "halo", x: 78, y: 52, scale: 2.4, paint: "orange-400" },
      { part: "rim-light", x: 48, y: 56, scale: 1.4, paint: "orange-400" },
    ],
    particles: [{ part: "sparks", x: 30, y: 60, scale: 1, count: 2, spread: [10, 4], paint: "white" }],
  },
};

/** Jlockeed's Permanent Defense Contract (Classic+ #52, Epic), Alt art: the contract scroll unrolling down the steps of a cliffside monastery under a bright noon sky, mule carts of crates climbing up; wide. */
const DEFENSE_CONTRACT_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 80, y: 16, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 16, scale: 0.6 },
      { part: "cloud", x: 30, y: 22, scale: 0.9, count: 3, spread: [18, 4], paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 50, paint: "slate-400" },
      { part: "cliff", x: 22, y: 50, scale: 1.2, paint: "stone-500" },
      { part: "temple", x: 26, y: 42, scale: 0.45, paint: "slate-100" },
      { part: "tower", x: 10, y: 40, scale: 0.5 },
    ],
    mid: [
      { part: "stairs", x: 50, y: 54, paint: "slate-100" },
      { part: "cliff", x: 20, y: 60, scale: 1.2, paint: "stone-500" },
      { part: "cliff", x: 80, y: 62, scale: 1.1, flip: -1, paint: "slate-400" },
      { part: "cart", x: 74, y: 70, scale: 0.6 },
      { part: "cart", x: 82, y: 80, scale: 0.75 },
      { part: "crate", x: 78, y: 64, scale: 0.5 },
    ],
    subject: [
      { part: "scroll", x: 48, y: 64, scale: 1.4 },
      { part: "scroll", x: 52, y: 78, scale: 1.9 },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 97, count: 3, spread: [10, 1], paint: "green-700" },
      { part: "crate", x: 90, y: 98, scale: 1.2 },
      { part: "pebbles", x: 60, y: 98 },
    ],
    light: [
      { part: "halo", x: 80, y: 16, scale: 2, paint: "white" },
      { part: "god-rays", x: 80, y: 10, scale: 0.6, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Book of Tokens (Classic+ #53, Epic), Alt art: the tiny ink sprinters racing out of the open book across a picnic blanket in a sunny meadow, scattering crumbs; ground level. */
const BOOK_OF_TOKENS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "green-200"],
  light: { x: 78, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 62, y: 72, scale: 0.26, pose: "pose-run", look: normalExtraLookOf("classicplus-053", 0) },
    { layer: "subject", x: 72, y: 78, scale: 0.28, pose: "pose-run", look: normalExtraLookOf("classicplus-053", 1) },
    { layer: "subject", x: 30, y: 76, scale: 0.26, flip: -1, pose: "pose-run", look: normalExtraLookOf("classicplus-053", 1) },
    { layer: "subject", x: 82, y: 86, scale: 0.32, pose: "pose-run", look: normalExtraLookOf("classicplus-053", 0) },
  ],
  layers: {
    sky: [
      { part: "sun", x: 78, y: 18, scale: 0.6 },
      { part: "cloud", x: 30, y: 24, scale: 0.8, count: 2, spread: [14, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "tree", x: 14, y: 58, scale: 1.2, paint: "green-700" },
      { part: "tree", x: 88, y: 56, scale: 1, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 60, paint: "green-500" },
      { part: "tile-floor", x: 50, y: 70, paint: "red-400" },
    ],
    subject: [{ part: "tome", x: 50, y: 72, scale: 1.8, paint: "blue-900" }],
    near: [
      { part: "bread", x: 14, y: 95, scale: 1.2 },
      { part: "basket", x: 88, y: 97, scale: 1.1 },
      { part: "grass-tuft", x: 50, y: 99, count: 3, spread: [24, 1], paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 78, y: 18, scale: 2, paint: "white" },
      { part: "rim-light", x: 50, y: 64, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "dust", x: 56, y: 80, scale: 1.2, paint: "brass-500" },
      { part: "sparks", x: 52, y: 60, scale: 1, count: 2, spread: [10, 4], paint: "violet-300" },
    ],
  },
};

/** Book of Books (Classic+ #54, Epic), Alt art: soaring over a rolling green moorland with a flock of smaller books like starlings, pages flapping; a wide sky-blue vista. */
const BOOK_OF_BOOKS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-600", "sky-300"],
  light: { x: 24, y: 24, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 80, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1, count: 3, spread: [20, 4], paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 64, paint: "green-700" },
      { part: "standing-stone", x: 82, y: 66, scale: 0.5 },
    ],
    mid: [
      { part: "field", x: 50, y: 72, paint: "green-500" },
      { part: "tome", x: 70, y: 28, scale: 0.4, count: 6, spread: [14, 8], paint: "blue-800" },
      { part: "tome", x: 24, y: 46, scale: 0.35, count: 4, spread: [10, 6], paint: "red-900" },
    ],
    subject: [{ part: "tome", x: 46, y: 42, scale: 1.8, paint: "red-900" }],
    near: [
      { part: "grass-tuft", x: 50, y: 97, count: 4, spread: [40, 2], paint: "green-900" },
      { part: "fern", x: 88, y: 98, scale: 1.4 },
      { part: "flowers", x: 14, y: 96, scale: 1.2, count: 2, spread: [6, 2], paint: "violet-300" },
    ],
    light: [
      { part: "halo", x: 24, y: 24, scale: 2, paint: "white" },
      { part: "rim-light", x: 46, y: 36, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "petals", x: 56, y: 40, scale: 1.3, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
    ],
  },
};

/** Book of Greed (Classic+ #55, Epic), Alt art: the jewelled tome lying open on a mossy forest altar under a waterfall, a fox nosing at its rare pages; emerald mist, side view. */
const BOOK_OF_GREED_ALT: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "jade-400"],
  light: { x: 50, y: 40, r: 34, paint: "green-200", opacity: 0.4 },
  emblem: { x: 78, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 1.4, count: 2, spread: [16, 4], paint: "green-200" }],
    far: [
      { part: "cliff", x: 18, y: 50, scale: 1.1, paint: "green-900" },
      { part: "tree", x: 86, y: 60, scale: 1.4, paint: "green-700" },
      { part: "tree", x: 12, y: 62, scale: 1.1, paint: "green-700" },
      { part: "waterfall", x: 52, y: 66, scale: 1.4 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "green-900" },
      { part: "altar", x: 40, y: 84, scale: 1.6, paint: "green-700" },
      { part: "mist", x: 60, y: 66, scale: 1, paint: "white" },
    ],
    subject: [
      { part: "tome", x: 38, y: 70, scale: 1.2, paint: "red-600" },
      { part: "gems", x: 38, y: 69, scale: 0.6 },
      { part: "fox", x: 66, y: 84, scale: 1.1, flip: -1 },
    ],
    near: [
      { part: "fern", x: 10, y: 98, scale: 1.6 },
      { part: "fern", x: 90, y: 99, scale: 1.4, flip: -1 },
      { part: "gems", x: 30, y: 95, scale: 1 },
    ],
    light: [
      { part: "halo", x: 40, y: 66, scale: 1.6, paint: "jade-400" },
      { part: "light-shaft", x: 40, y: 0, scale: 1.2, paint: "green-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "green-200" },
      { part: "sparks", x: 38, y: 64, scale: 0.9, count: 2, spread: [6, 3], paint: "white" },
    ],
  },
};

/** All Purpose Apple (Classic+ #59, Rare), Alt art: a still life of the apple on a frosty windowsill of a snowbound monastery at dawn, its healing glow seeping out; a close-up in cool blue. */
const ALL_PURPOSE_APPLE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "sky-300"],
  light: { x: 50, y: 64, r: 30, paint: "green-200", opacity: 0.4 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 30, y: 20, scale: 1 }],
    far: [
      { part: "mountains", x: 50, y: 54, paint: "slate-400" },
      { part: "temple", x: 72, y: 60, scale: 0.45, paint: "slate-100" },
      { part: "tower", x: 22, y: 60, scale: 0.5 },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 62 },
      { part: "table", x: 50, y: 100, scale: 3, paint: "slate-400" },
    ],
    subject: [{ part: "apple", x: 50, y: 78, scale: 5.2 }],
    near: [
      { part: "column", x: 6, y: 104, scale: 1.6 },
      { part: "column", x: 94, y: 104, scale: 1.6 },
    ],
    light: [
      { part: "halo", x: 50, y: 66, scale: 2, paint: "green-400" },
      { part: "light-shaft", x: 30, y: 0, paint: "blue-200" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.2 },
      { part: "sparks", x: 50, y: 62, scale: 1, count: 3, spread: [12, 6], paint: "green-400" },
    ],
  },
};

/** Doctors Orders (Classic+ #60, Rare), Alt art: the physician's tent pitched in a bustling desert caravan camp, apple crates stacked beside the cot; dusty ochre and red cloth, wide. */
const DOCTORS_ORDERS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["orange-400", "brass-500"],
  light: { x: 80, y: 22, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 22, y: 30, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 80, y: 22, scale: 0.7 }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "brass-600" },
      { part: "tent", x: 14, y: 64, scale: 0.6, paint: "slate-100" },
      { part: "tent", x: 86, y: 62, scale: 0.5, flip: -1, paint: "red-900" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "brass-600" },
      { part: "cart", x: 22, y: 74, scale: 0.7 },
    ],
    subject: [
      { part: "tent", x: 40, y: 82, scale: 1.6, paint: "red-600" },
      { part: "cot", x: 64, y: 84, scale: 0.9 },
      { part: "flask", x: 56, y: 84, scale: 0.5 },
      { part: "crate", x: 80, y: 84, scale: 0.7 },
      { part: "crate", x: 80, y: 76, scale: 0.6 },
      { part: "apple", x: 80, y: 71, scale: 0.8, count: 3, spread: [2, 0] },
    ],
    near: [
      { part: "crate", x: 12, y: 97, scale: 1.2 },
      { part: "basket", x: 88, y: 98, scale: 1 },
      { part: "pebbles", x: 50, y: 98 },
    ],
    light: [{ part: "halo", x: 80, y: 22, scale: 2, paint: "white" }],
    particles: [{ part: "dust", x: 50, y: 66, scale: 1.4, paint: "brass-500" }],
  },
};

/** Bauble Bubble (Classic+ #61, Rare), Alt art: the bubble drifting over a misty autumn marsh, trinkets glinting inside, a heron watching; rust-orange reeds and grey fog, a wide low view. */
const BAUBLE_BUBBLE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 56, y: 52, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 34, scale: 1.4, count: 2, spread: [18, 4], paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-700" },
      { part: "dead-tree", x: 14, y: 64, scale: 0.9 },
      { part: "dead-tree", x: 86, y: 62, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "water", x: 50, y: 68, paint: "slate-700" },
      { part: "wheat", x: 18, y: 76, scale: 1, count: 2, spread: [8, 2], paint: "orange-400" },
      { part: "bird", x: 80, y: 42, scale: 0.8 },
      { part: "mist", x: 50, y: 70, scale: 1.2, paint: "white" },
    ],
    subject: [
      { part: "dome", x: 56, y: 64, scale: 1.1 },
      { part: "gems", x: 56, y: 61, scale: 0.6 },
      { part: "gift-box", x: 50, y: 62, scale: 0.5 },
      { part: "orb", x: 62, y: 56, scale: 0.35, paint: "red-600" },
    ],
    near: [
      { part: "wheat", x: 10, y: 98, scale: 1.4, paint: "orange-400" },
      { part: "wheat", x: 90, y: 99, scale: 1.3, paint: "orange-400" },
      { part: "grass-tuft", x: 50, y: 98, count: 3, spread: [24, 1], paint: "orange-400" },
    ],
    light: [{ part: "halo", x: 56, y: 56, scale: 1.6, paint: "white" }],
    particles: [{ part: "sparks", x: 56, y: 58, scale: 0.9, count: 3, spread: [6, 4], paint: "white" }],
  },
};

/** Two Grapes (Classic+ #65, Rare), Alt art: a close-up of two grapes dangling on a sunlit vine above a gambler's open palm in a terraced vineyard; bright leaf green and blue sky. */
const TWO_GRAPES_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 78, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 78, y: 18, scale: 0.7 },
      { part: "cloud", x: 30, y: 24, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 60, paint: "green-700" },
      { part: "fence", x: 50, y: 66, scale: 0.8, paint: "wood-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-500" },
      { part: "vine", x: 50, y: 68, scale: 0.8, count: 4, spread: [36, 2] },
    ],
    subject: [
      { part: "vine", x: 50, y: 64, scale: 1.8, paint: "wood-600" },
      { part: "orb", x: 44, y: 46, scale: 1.3, paint: "violet-500" },
      { part: "orb", x: 57, y: 50, scale: 1.4, paint: "violet-700" },
      { part: "pale-hand", x: 50, y: 96, scale: 2.2 },
    ],
    near: [{ part: "grass-tuft", x: 50, y: 99, count: 3, spread: [36, 1], paint: "green-500" }],
    light: [
      { part: "halo", x: 50, y: 48, scale: 1.6, paint: "white" },
      { part: "light-shaft", x: 70, y: 0, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, paint: "white" }],
  },
};

/** Vine of Grapes (Classic+ #66, Rare), Alt art: the vine hanging over a still mountain lake at dawn, every grape reflected a different colour in the water; a close-up in misty pink and teal. */
const VINE_OF_GRAPES_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-300", "slate-100"],
  light: { x: 80, y: 50, r: 34, paint: "red-300", opacity: 0.4 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 36, scale: 1.4, count: 2, spread: [18, 4], paint: "white" }],
    far: [{ part: "mountains", x: 50, y: 54, paint: "slate-400" }],
    mid: [
      { part: "water", x: 50, y: 60, paint: "jade-400" },
      { part: "grapes", x: 32, y: 78, scale: 0.9, paint: "red-600" },
      { part: "grapes", x: 46, y: 80, scale: 0.8, paint: "green-400" },
      { part: "grapes", x: 60, y: 78, scale: 0.9, paint: "orange-400" },
      { part: "grapes", x: 74, y: 80, scale: 0.8, paint: "violet-500" },
    ],
    subject: [
      { part: "vine", x: 18, y: 66, scale: 2.2 },
      { part: "grapes", x: 30, y: 34, scale: 1.4, paint: "violet-500" },
      { part: "grapes", x: 44, y: 40, scale: 1.2, paint: "green-400" },
      { part: "grapes", x: 36, y: 50, scale: 1.1, paint: "red-600" },
      { part: "grapes", x: 52, y: 48, scale: 1, paint: "orange-400" },
    ],
    near: [
      { part: "grass-tuft", x: 10, y: 98, count: 3, spread: [8, 1], paint: "jade-400" },
      { part: "mist", x: 60, y: 92, scale: 1.2, paint: "white" },
    ],
    light: [{ part: "halo", x: 80, y: 50, scale: 2, paint: "red-300" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.3, paint: "white" }],
  },
};

/** Pear (Classic+ #67, Rare), Alt art: before it splits, the pear on a branch in a stormy autumn orchard, two tiny friends peering from a hole in its side; wide, wind-tossed orange and grey. */
const PEAR_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "orange-400"],
  light: { x: 30, y: 40, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 80, y: 74, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 55, y: 51, scale: 0.08, pose: "pose-raise", look: normalExtraLookOf("classicplus-067", 0) },
    { layer: "subject", x: 58, y: 51, scale: 0.08, flip: -1, pose: "pose-raise", look: normalExtraLookOf("classicplus-067", 1) },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.1, count: 3, spread: [20, 4], paint: "slate-800" },
      { part: "lightning", x: 84, y: 14, scale: 0.5 },
    ],
    far: [
      { part: "hills", x: 50, y: 62, paint: "wood-700" },
      { part: "fruit-tree", x: 12, y: 66, scale: 0.7 },
      { part: "fruit-tree", x: 88, y: 64, scale: 0.6 },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "wood-600" },
      { part: "fruit-tree", x: 46, y: 92, scale: 1.6 },
    ],
    subject: [{ part: "pear", x: 56, y: 54, scale: 1.6 }],
    near: [
      { part: "grass-tuft", x: 50, y: 98, count: 4, spread: [40, 1], paint: "orange-400" },
      { part: "basket", x: 86, y: 97, scale: 1 },
    ],
    light: [{ part: "halo", x: 56, y: 50, scale: 1.2, paint: "orange-400" }],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.4, count: 2, spread: [16, 8], paint: "orange-400" },
      { part: "rain", x: 50, y: 40, scale: 1.1, paint: "slate-400" },
    ],
  },
};

/** Book of Buff (Classic+ #71, Epic), Alt art: in a busy harbour, its beam swelling a fishing boat into a towering warship as sailors gape; sea teal and white sails, a low waterline angle. */
const BOOK_OF_BUFF_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "blue-200"],
  light: { x: 22, y: 40, r: 32, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "near", x: 12, y: 98, scale: 0.5, pose: "pose-raise", look: { ...VILLAGER, hair: "kerchief", cloth: "blue-800" } },
    { layer: "near", x: 90, y: 100, scale: 0.55, flip: -1, pose: "pose-spread", look: { ...VILLAGER, hair: "hat-brim", cloth: "red-600" } },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 70, y: 22, scale: 0.8, count: 2, spread: [12, 3], paint: "white" },
      { part: "bird", x: 50, y: 26, scale: 0.4, count: 2, spread: [10, 4] },
    ],
    far: [
      { part: "house", x: 10, y: 58, scale: 0.6 },
      { part: "house", x: 90, y: 56, scale: 0.55, flip: -1 },
      { part: "tower", x: 80, y: 56, scale: 0.5 },
    ],
    mid: [{ part: "water", x: 50, y: 60, paint: "cyan-400" }],
    subject: [
      { part: "tome", x: 22, y: 40, scale: 1.3, paint: "slate-700" },
      { part: "light-ribbon", x: 40, y: 52, scale: 1, paint: "sky-300" },
      { part: "boat", x: 30, y: 72, scale: 0.6 },
      { part: "boat", x: 62, y: 78, scale: 1.9 },
    ],
    near: [{ part: "mist", x: 50, y: 92, scale: 1.4, count: 2, spread: [20, 2], paint: "white" }],
    light: [
      { part: "halo", x: 22, y: 40, scale: 2, paint: "sky-300" },
      { part: "rim-light", x: 62, y: 60, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 46, y: 54, scale: 1.1, count: 3, spread: [10, 6], paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" },
    ],
  },
};

/** Book of Nerf (Classic+ #72, Epic), Alt art: open in a sunny lavender field, its beam shrinking a looming scarecrow giant to toy size; violet and leaf green, wide. */
const BOOK_OF_NERF_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "violet-300"],
  light: { x: 30, y: 60, r: 30, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 18, y: 20, scale: 0.6 },
      { part: "cloud", x: 64, y: 22, scale: 0.8, count: 2, spread: [12, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-500" },
      { part: "tree", x: 90, y: 60, scale: 1, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "violet-500" },
      { part: "bone-scarecrow", x: 66, y: 82, scale: 2 },
      { part: "flowers", x: 20, y: 76, scale: 1.1, count: 3, spread: [10, 3], paint: "violet-300" },
    ],
    subject: [
      { part: "tome", x: 30, y: 66, scale: 1.2, paint: "slate-400" },
      { part: "light-ribbon", x: 48, y: 64, scale: 0.9, paint: "violet-300" },
      { part: "bone-scarecrow", x: 64, y: 84, scale: 0.5 },
    ],
    near: [
      { part: "flowers", x: 12, y: 96, scale: 1.4, count: 2, spread: [6, 2], paint: "violet-300" },
      { part: "grass-tuft", x: 60, y: 98, count: 4, spread: [30, 1], paint: "green-500" },
    ],
    light: [
      { part: "halo", x: 30, y: 62, scale: 1.6, paint: "violet-300" },
      { part: "light-shaft", x: 20, y: 0, paint: "white" },
    ],
    particles: [
      { part: "petals", x: 50, y: 50, scale: 1.2, paint: "violet-300" },
      { part: "motes", x: 50, y: 60, scale: 1.2, paint: "white" },
    ],
  },
};

/** Call to Chaos (Classic+ Edition) (Classic+ #73, Legendary), Alt art: the gambler pulling the lever at a carnival on a windy coastal pier at noon, fruit and books raining onto the boardwalk; red and white awnings, wide. */
const CALL_TO_CHAOS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "slate-100"],
  light: { x: 80, y: 16, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 66, size: 5, rotate: 0 },
  figure: { x: 32, y: 84, scale: 0.6, pose: "pose-raise", look: { ...normalLookOf("classicplus-073") } },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 16, scale: 0.6 },
      { part: "cloud", x: 24, y: 22, scale: 0.8, count: 2, spread: [14, 3], paint: "white" },
    ],
    far: [
      { part: "water", x: 50, y: 56, paint: "blue-600" },
      { part: "boat", x: 84, y: 58, scale: 0.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "wood-700" },
      { part: "fence", x: 50, y: 64, scale: 0.6, paint: "slate-100" },
      { part: "awning", x: 14, y: 76, scale: 0.9 },
      { part: "awning", x: 86, y: 74, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "fortune-wheel", x: 56, y: 50, scale: 1.2 },
      { part: "apple", x: 36, y: 34, scale: 1.2 },
      { part: "pear", x: 72, y: 28, scale: 1.1 },
      { part: "tome", x: 46, y: 26, scale: 0.5, paint: "blue-800" },
      { part: "tome", x: 78, y: 44, scale: 0.5, paint: "red-900" },
    ],
    near: [
      { part: "apple", x: 60, y: 94, scale: 1.4 },
      { part: "tome", x: 76, y: 96, scale: 0.8, paint: "blue-800" },
      { part: "dice", x: 14, y: 95, scale: 1.2 },
      { part: "pear", x: 44, y: 96, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 56, y: 50, scale: 2, paint: "white" },
      { part: "rim-light", x: 32, y: 62, scale: 1.1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 56, y: 50, scale: 1.1, count: 3, spread: [10, 8], paint: "white" },
      { part: "petals", x: 50, y: 40, scale: 1.2, paint: "red-400" },
    ],
  },
};

/** Twice Forward One Step Backwards (Classic+ #74, Mythic), Alt art: the trap as a glacier staircase in a blizzard, climbers sliding back one step for every two, their frozen spells glued into the ice; white and steel grey, a vertical view. */
const TWICE_FORWARD_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 74, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 84, scale: 0.36, pose: "pose-run", look: normalExtraLookOf("classicplus-074", 0) },
    { layer: "subject", x: 44, y: 64, scale: 0.3, pose: "pose-run", look: normalExtraLookOf("classicplus-074", 1) },
    { layer: "subject", x: 58, y: 70, scale: 0.32, flip: -1, pose: "pose-spread", look: normalExtraLookOf("classicplus-074", 2) },
    { layer: "subject", x: 68, y: 86, scale: 0.38, flip: -1, pose: "pose-raise", look: normalExtraLookOf("classicplus-074", 3) },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 50, y: 18, scale: 1.2, count: 2, spread: [24, 4], paint: "slate-100" },
      { part: "mist", x: 30, y: 30, scale: 1.2, paint: "white" },
    ],
    far: [{ part: "mountains", x: 50, y: 46, paint: "slate-100" }],
    mid: [
      { part: "stairs", x: 50, y: 38, paint: "slate-100" },
      { part: "cliff", x: 14, y: 48, scale: 1.2, paint: "slate-100" },
      { part: "cliff", x: 86, y: 46, scale: 1.2, flip: -1, paint: "slate-400" },
      { part: "mist", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
    subject: [
      { part: "fireball", x: 36, y: 58, scale: 0.5 },
      { part: "crystal", x: 36, y: 62, scale: 1.3, paint: "blue-200" },
      { part: "lightning", x: 64, y: 48, scale: 0.45, paint: "cyan-400" },
      { part: "crystal", x: 64, y: 58, scale: 1.2, paint: "blue-200" },
    ],
    near: [
      { part: "ice-floor", x: 50, y: 94 },
      { part: "crystal", x: 10, y: 98, scale: 1.6, paint: "blue-200" },
      { part: "crystal", x: 90, y: 97, scale: 1.5, paint: "slate-100" },
    ],
    light: [
      { part: "halo", x: 50, y: 30, scale: 2.6, paint: "white" },
      { part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "blue-200" },
      { part: "light-shaft", x: 50, y: 0, paint: "white" },
    ],
    particles: [
      { part: "snow", x: 50, y: 56, scale: 1.6 },
      { part: "sparks", x: 36, y: 58, scale: 0.9, count: 2, spread: [4, 3], paint: "cyan-400" },
    ],
  },
};

/** J-lease J-Jungle EX-plorer (Classic+ #75, Legendary), Alt art: a close-up portrait of J-lease at a bustling river market at noon, handing a bulging pack to a nervous apprentice; terracotta and river blue. */
const J_LEASE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "blue-200"],
  light: { x: 80, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 116, scale: 1.7, pose: "pose-raise", look: { ...normalLookOf("classicplus-075") } },
  extras: [{ layer: "subject", x: 80, y: 104, scale: 0.9, flip: -1, pose: "pose-guard", look: normalExtraLookOf("classicplus-075", 0) }],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.6 },
      { part: "cloud", x: 26, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "house", x: 12, y: 54, scale: 0.7, paint: "red-600" },
      { part: "house", x: 88, y: 52, scale: 0.6, flip: -1, paint: "red-600" },
      { part: "tower", x: 70, y: 52, scale: 0.4 },
    ],
    mid: [
      { part: "water", x: 50, y: 58, paint: "blue-600" },
      { part: "boat", x: 64, y: 64, scale: 0.5 },
      { part: "awning", x: 14, y: 72, scale: 0.7, paint: "red-600" },
    ],
    near: [
      { part: "basket", x: 12, y: 98, scale: 1.3 },
      { part: "coins", x: 66, y: 97, scale: 1.1, props: true },
      { part: "gems", x: 90, y: 96, scale: 0.9, props: true },
    ],
    light: [
      { part: "halo", x: 80, y: 18, scale: 2, paint: "white" },
      { part: "rim-light", x: 40, y: 64, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "dust", x: 50, y: 70, scale: 1.2, paint: "red-300" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "white" },
    ],
  },
};

/** Claude's Datacenter (Classic+ #78, Legendary), Alt art: the datacenter from outside, a frozen hillside vault venting steam into a pink winter dawn while villagers warm bread on its walls; a wide exterior vista. */
const CLAUDES_DATACENTER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["violet-300", "red-300"],
  light: { x: 80, y: 50, r: 36, paint: "red-300", opacity: 0.45 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 28, y: 80, scale: 0.4, pose: "pose-raise", look: { ...VILLAGER, hair: "kerchief", cloth: "red-600" } },
    { layer: "subject", x: 74, y: 82, scale: 0.42, flip: -1, pose: "pose-stand", look: { ...VILLAGER, hair: "hat-brim", item: "item-tray" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 0.9, count: 2, spread: [16, 4], paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 52, paint: "slate-400" },
      { part: "hills", x: 50, y: 60, paint: "slate-400" },
      { part: "house", x: 10, y: 64, scale: 0.45 },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 66 },
      { part: "grave-mound", x: 50, y: 76, scale: 3, paint: "slate-100" },
      { part: "smoke", x: 42, y: 50, scale: 1.6, paint: "slate-100" },
      { part: "smoke", x: 58, y: 50, scale: 1.4, paint: "slate-100" },
      { part: "arch", x: 50, y: 74, scale: 1.4, paint: "stone-500" },
    ],
    subject: [
      { part: "server-rack", x: 44, y: 72, scale: 0.9 },
      { part: "server-rack", x: 56, y: 72, scale: 0.9 },
      { part: "bread", x: 32, y: 70, scale: 0.9 },
      { part: "bread", x: 70, y: 72, scale: 0.9 },
    ],
    near: [
      { part: "fence-post", x: 10, y: 96, scale: 1 },
      { part: "snow", x: 50, y: 88, scale: 1.2 },
      { part: "pebbles", x: 80, y: 98 },
    ],
    light: [
      { part: "halo", x: 80, y: 50, scale: 2.2, paint: "red-300" },
      { part: "rim-light", x: 50, y: 60, scale: 1.1, paint: "cyan-400" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.2 },
      { part: "motes", x: 50, y: 70, scale: 1.2, paint: "cyan-400" },
    ],
  },
};

/** R3396000: Classic+'s alt recipes #37 to #78, by id. */
export const CLASSIC_PLUS_ALT_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "classicplus-037": WARDRUM_ALT,
  "classicplus-041": KYS_CONSTANT_ALT,
  "classicplus-043": AI_SLOP_ALT,
  "classicplus-046": FELINOR_FLAGBEARER_ALT,
  "classicplus-050": ADAPTIVE_GROWTH_ALT,
  "classicplus-051": J15_FIGHTER_ALT,
  "classicplus-052": DEFENSE_CONTRACT_ALT,
  "classicplus-053": BOOK_OF_TOKENS_ALT,
  "classicplus-054": BOOK_OF_BOOKS_ALT,
  "classicplus-055": BOOK_OF_GREED_ALT,
  "classicplus-059": ALL_PURPOSE_APPLE_ALT,
  "classicplus-060": DOCTORS_ORDERS_ALT,
  "classicplus-061": BAUBLE_BUBBLE_ALT,
  "classicplus-065": TWO_GRAPES_ALT,
  "classicplus-066": VINE_OF_GRAPES_ALT,
  "classicplus-067": PEAR_ALT,
  "classicplus-071": BOOK_OF_BUFF_ALT,
  "classicplus-072": BOOK_OF_NERF_ALT,
  "classicplus-073": CALL_TO_CHAOS_ALT,
  "classicplus-074": TWICE_FORWARD_ALT,
  "classicplus-075": J_LEASE_ALT,
  "classicplus-078": CLAUDES_DATACENTER_ALT,
};
