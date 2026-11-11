// R3396000: Classic+'s alt scene recipes, part 1 of two (#2 to #36), one per card offered Alt art, each drawn from its
// brief (ART.md, "Classic+'s Alt art restaged"): the card's own subject in another place, time or moment, under a sky
// sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one from
// above), its figure the normal picture's person (`normalLookOf`), in Classic's temple-stone look and at or above its
// rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed alone, at every Alt art level.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { CLASSIC_PLUS_RECIPES_1 } from "./classicPlusRecipes1.ts";
import { CLASSIC_PLUS_RECIPES_2 } from "./classicPlusRecipes2.ts";
import { CLASSIC_PLUS_RECIPES_3 } from "./classicPlusRecipes3.ts";
import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The card's normal recipe, from the three Classic+ tables. */
function normalOf(id: string): SceneRecipe | undefined {
  return CLASSIC_PLUS_RECIPES_1[id] ?? CLASSIC_PLUS_RECIPES_2[id] ?? CLASSIC_PLUS_RECIPES_3[id];
}

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
export function normalLookOf(id: string): FigureLook {
  return normalOf(id)?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a crowd its brief keeps is the same people; with none, the default person. */
export function normalExtraLookOf(id: string, k: number): FigureLook {
  return normalOf(id)?.extras?.[k]?.look ?? figureLookFor([]);
}

/** A bundled villager in the winter fair's queue. */
const BUNDLED: FigureLook = { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-coat", item: null, cloth: "blue-800" };

/** Groom Shroom (Classic+ #2, Epic), Alt art: before the wedding, the pink-gilled mushroom close up under fallen maple leaves in a rainy forest shrine, cat ears peeking from its cap. */
const GROOM_SHROOM_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "wood-700"],
  light: { x: 28, y: 24, r: 30, paint: "slate-400", opacity: 0.3 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, count: 3, spread: [30, 4], paint: "slate-800" }],
    far: [
      { part: "tree", x: 12, y: 62, scale: 2, paint: "wood-600" },
      { part: "tree", x: 88, y: 60, scale: 1.8, flip: -1, paint: "red-600" },
      { part: "gate", x: 50, y: 62, scale: 1.2, paint: "red-900" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "wood-800" },
      { part: "standing-stone", x: 20, y: 74, scale: 0.6, paint: "slate-400" },
      { part: "standing-stone", x: 80, y: 73, scale: 0.55, flip: -1, paint: "slate-400" },
      { part: "mist", x: 50, y: 72, scale: 0.9, paint: "slate-400" },
    ],
    subject: [
      { part: "cat", x: 58, y: 39, scale: 1.6, paint: "wood-600" },
      { part: "giant-mushroom", x: 50, y: 94, scale: 2.2, paint: "red-300" },
    ],
    near: [
      { part: "petals", x: 50, y: 94, scale: 1.4, paint: "red-600" },
      { part: "grass-tuft", x: 12, y: 99, count: 2, spread: [5, 1], paint: "wood-600" },
      { part: "pebbles", x: 86, y: 98 },
    ],
    light: [{ part: "halo", x: 50, y: 56, scale: 1.8, paint: "slate-100" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.3, paint: "slate-100" },
      { part: "petals", x: 50, y: 30, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Guy Att (Classic+ #5, Common), Alt art: a wide view after the job, Guy Att resting on a heap of smashed drawers on a rainy harbour quay, his mallet over his shoulder. */
const GUY_ATT_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 70, y: 22, r: 30, paint: "white", opacity: 0.2 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 52, y: 74, scale: 0.5, pose: "pose-sit", look: normalLookOf("classicplus-005") },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 24, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-700" }],
    far: [
      { part: "mountains", x: 50, y: 58, paint: "slate-700" },
      { part: "water", x: 50, y: 58, paint: "blue-800" },
      { part: "tower", x: 88, y: 60, scale: 0.6 },
    ],
    mid: [
      { part: "boat", x: 20, y: 66, scale: 0.55 },
      { part: "boat", x: 78, y: 64, scale: 0.45, flip: -1 },
      { part: "tile-floor", x: 50, y: 76, paint: "stone-500" },
    ],
    subject: [
      { part: "crate", x: 44, y: 84, scale: 1, count: 3, spread: [6, 2] },
      { part: "cabinet", x: 58, y: 84, scale: 0.7 },
    ],
    near: [
      { part: "standing-stone", x: 88, y: 99, scale: 0.6, paint: "slate-700" },
      { part: "pebbles", x: 30, y: 96 },
    ],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.3, paint: "slate-100" },
      { part: "rain", x: 50, y: 72, scale: 1, paint: "slate-100" },
    ],
  },
};

/** Withering Storm (Classic+ #8, Rare), Alt art: the storm rolling across a vast lavender heath at noon, a lone shepherd's cards scattering and ageing in the gale. */
const WITHERING_STORM_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "violet-300"],
  light: { x: 76, y: 16, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 80, y: 32, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 76, y: 76, scale: 0.3, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-poncho", item: "item-staff", cloth: "wood-700" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 78, y: 16, scale: 0.7 },
      { part: "cloud", x: 22, y: 22, scale: 1.4, count: 2, spread: [14, 3], paint: "slate-400" },
    ],
    far: [{ part: "hills", x: 50, y: 58, paint: "violet-500" }],
    mid: [
      { part: "field", x: 50, y: 64, paint: "violet-500" },
      { part: "flowers", x: 18, y: 76, count: 3, spread: [12, 3] },
    ],
    subject: [
      { part: "tornado", x: 30, y: 72, scale: 1.4, paint: "slate-400" },
      { part: "card", x: 58, y: 58, scale: 0.8, count: 4, spread: [16, 10], paint: "stone-500" },
      { part: "card", x: 66, y: 46, scale: 0.7, paint: "wood-600" },
    ],
    near: [
      { part: "grass-tuft", x: 20, y: 97, count: 3, spread: [12, 2], paint: "violet-500" },
      { part: "flowers", x: 82, y: 96, count: 2, spread: [6, 2] },
    ],
    particles: [
      { part: "dust", x: 44, y: 50, scale: 1.2, paint: "wood-600" },
      { part: "petals", x: 54, y: 40, scale: 1.3, paint: "violet-300" },
    ],
  },
};

/** Silence (Classic+ #9, Common), Alt art: a close-up of the stone warrior's carved face, a finger at its lips and its runes going dark, in a sunlit autumn orchard. */
const SILENCE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "orange-400"],
  light: { x: 24, y: 26, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.8, pose: "pose-raise", look: { ...normalLookOf("classicplus-009"), item: null } },
  layers: {
    sky: [{ part: "sun", x: 20, y: 20, scale: 0.8 }],
    far: [
      { part: "fruit-tree", x: 14, y: 64, scale: 1.2, paint: "orange-400" },
      { part: "fruit-tree", x: 86, y: 62, scale: 1.1, flip: -1, paint: "red-600" },
    ],
    mid: [{ part: "field", x: 50, y: 70, paint: "brass-600" }],
    near: [{ part: "grass-tuft", x: 50, y: 99, count: 3, spread: [40, 1], paint: "wood-600" }],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.4, paint: "orange-400" },
      { part: "motes", x: 40, y: 30, scale: 1, paint: "white" },
    ],
  },
};

/** The Mother Pancake (Classic+ #12, Legendary), Alt art: picnicking in a breezy spring meadow below a hill temple, her pancake children tumbling round her basket; wide and low. */
const MOTHER_PANCAKE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 78, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 32, size: 5, rotate: 0 },
  figure: { x: 46, y: 82, scale: 0.62, pose: "pose-sit", look: normalLookOf("classicplus-012") },
  extras: [
    { layer: "subject", x: 26, y: 78, scale: 0.26, pose: "pose-spread", look: normalExtraLookOf("classicplus-012", 0) },
    { layer: "subject", x: 72, y: 76, scale: 0.24, flip: -1, pose: "pose-run", look: normalExtraLookOf("classicplus-012", 1) },
    { layer: "near", x: 62, y: 90, scale: 0.28, pose: "pose-raise", look: { ...normalExtraLookOf("classicplus-012", 0), cloth: "red-400" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.8 },
      { part: "cloud", x: 26, y: 22, scale: 1, count: 2, spread: [16, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "temple", x: 64, y: 50, scale: 0.45, paint: "stone-500" },
      { part: "tree", x: 14, y: 58, scale: 1, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-500" },
      { part: "flowers", x: 18, y: 72, count: 2, spread: [10, 3] },
    ],
    subject: [
      { part: "basket", x: 62, y: 82, scale: 0.9 },
      { part: "pancakes", x: 32, y: 84, scale: 0.9 },
    ],
    near: [
      { part: "grass-tuft", x: 50, y: 98, count: 3, spread: [40, 2], paint: "green-500" },
      { part: "flowers", x: 12, y: 97, scale: 1.2 },
      { part: "bee", x: 82, y: 86 },
    ],
    light: [
      { part: "light-shaft", x: 76, y: 0, paint: "white" },
      { part: "halo", x: 46, y: 64, scale: 1.6, paint: "white" },
    ],
    particles: [
      { part: "petals", x: 50, y: 36, scale: 1.3, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" },
    ],
  },
};

/** Mommy Barker (Classic+ #13, Legendary), Alt art: at a snowy winter fair stall, barking through her griddle's steam at a queue of bundled villagers; seen from the side. */
const MOMMY_BARKER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "slate-100"],
  light: { x: 40, y: 64, r: 30, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 26, y: 88, scale: 0.9, pose: "pose-raise", look: { ...normalLookOf("classicplus-013"), cloth: "red-600" } },
  extras: [
    { layer: "mid", x: 66, y: 80, scale: 0.5, flip: -1, pose: "pose-stand", look: BUNDLED },
    { layer: "mid", x: 76, y: 78, scale: 0.46, flip: -1, pose: "pose-stand", look: { ...BUNDLED, cloth: "slate-700" } },
    { layer: "mid", x: 86, y: 77, scale: 0.42, flip: -1, pose: "pose-raise", look: { ...BUNDLED, hair: "hat-brim", cloth: "red-900" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 70, y: 20, scale: 0.9, paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-400" },
      { part: "house", x: 88, y: 60, scale: 0.7, flip: -1 },
      { part: "tree", x: 54, y: 60, scale: 0.8, paint: "green-900" },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 68 },
      { part: "awning", x: 30, y: 84, scale: 1.3, paint: "red-600" },
    ],
    subject: [
      { part: "table", x: 46, y: 90, scale: 0.8 },
      { part: "pancakes", x: 46, y: 83, scale: 0.9 },
      { part: "smoke", x: 46, y: 66, scale: 1.2, paint: "white" },
    ],
    near: [
      { part: "crate", x: 90, y: 99, scale: 1.2 },
      { part: "bread", x: 90, y: 92, scale: 1 },
      { part: "lantern", x: 8, y: 66, scale: 1.1 },
    ],
    light: [
      { part: "halo", x: 46, y: 80, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 8, y: 70, scale: 1, paint: "orange-400" },
      { part: "rim-light", x: 26, y: 60, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "snow", x: 50, y: 60, scale: 1.4 },
      { part: "smoke", x: 50, y: 40, scale: 1, paint: "white" },
    ],
  },
};

/** Forever& (Classic+ #14, Epic), Alt art: a close-up of the returning scroll landing back in an open palm in a bright bamboo grove at noon, its looping trail fading among the stalks. */
const FOREVER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["green-200", "white"],
  light: { x: 50, y: 16, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 26, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "light-shaft", x: 30, y: 0, paint: "white" }],
    far: [
      { part: "bamboo", x: 12, y: 72, scale: 1.4 },
      { part: "bamboo", x: 30, y: 66, scale: 1.1 },
      { part: "bamboo", x: 70, y: 66, scale: 1.1 },
      { part: "bamboo", x: 88, y: 72, scale: 1.4 },
      { part: "mist", x: 50, y: 66, scale: 1, paint: "white" },
    ],
    mid: [
      { part: "field", x: 50, y: 74, paint: "green-500" },
      { part: "bamboo", x: 40, y: 80, scale: 1.2 },
    ],
    subject: [
      { part: "light-ribbon", x: 50, y: 40, scale: 0.9, paint: "green-400" },
      { part: "pale-hand", x: 50, y: 100, scale: 3.4 },
      { part: "scroll", x: 50, y: 64, scale: 1.8 },
    ],
    near: [
      { part: "bamboo", x: 4, y: 102, scale: 1.8 },
      { part: "bamboo", x: 96, y: 102, scale: 1.7 },
      { part: "grass-tuft", x: 50, y: 99, count: 3, spread: [40, 1], paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 1.6, paint: "white" },
      { part: "rim-light", x: 50, y: 62, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 52, scale: 1, count: 3, spread: [12, 8], paint: "white" },
      { part: "petals", x: 50, y: 30, scale: 1.2, paint: "green-400" },
    ],
  },
};

/** Conjure Rush Token (Classic+ #15, Common), Alt art: the sprinter mid-stride along an autumn forest road, red leaves whipped into a spiral behind him, from low under a crimson canopy. */
const CONJURE_RUSH_TOKEN_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 30, y: 40, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 82, y: 72, size: 5, rotate: 0 },
  figure: { x: 56, y: 98, scale: 1.3, pose: "pose-run", look: normalLookOf("classicplus-015") },
  layers: {
    sky: [
      { part: "tree", x: 20, y: 30, scale: 2.4, count: 2, spread: [30, 4], paint: "red-600" },
      { part: "tree", x: 82, y: 26, scale: 2.2, paint: "red-900" },
    ],
    far: [
      { part: "tree", x: 10, y: 62, scale: 1.4, paint: "red-600" },
      { part: "tree", x: 90, y: 60, scale: 1.3, flip: -1, paint: "wood-600" },
    ],
    mid: [{ part: "street", x: 50, y: 64, paint: "wood-700" }],
    subject: [
      { part: "tornado", x: 24, y: 84, scale: 0.9, paint: "red-600" },
      { part: "petals", x: 26, y: 60, scale: 1.4, count: 2, spread: [6, 8], paint: "red-600" },
    ],
    near: [{ part: "petals", x: 50, y: 92, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.2, paint: "red-400" }],
  },
};

/** Gullible Treatler (Classic+ #18, Common), Alt art: waiting alone on a rain-soaked village green at grey noon, his candy sack soggy, still smiling hopefully; a wide shot. */
const GULLIBLE_TREATLER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 20, r: 36, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 52, y: 78, scale: 0.55, pose: "pose-stand", look: normalLookOf("classicplus-018") },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-700" }],
    far: [
      { part: "house", x: 14, y: 60, scale: 0.6 },
      { part: "house", x: 84, y: 58, scale: 0.55, flip: -1 },
      { part: "tree", x: 32, y: 58, scale: 0.8, paint: "green-900" },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "green-900" },
      { part: "fence", x: 50, y: 70, scale: 0.8, paint: "wood-700" },
    ],
    near: [{ part: "grass-tuft", x: 20, y: 97, count: 3, spread: [14, 2], paint: "green-700" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 70, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** League of Losers (Classic+ #19, Legendary), Alt art: the night before battle, the five Losers squabbling round a campfire in a pine forest, nobody facing anybody, seen from overhead. */
const LEAGUE_OF_LOSERS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "slate-900"],
  light: { x: 50, y: 62, r: 30, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 84, y: 28, size: 5, rotate: 0 },
  figure: { x: 50, y: 50, scale: 0.42, flip: -1, pose: "pose-raise", look: normalLookOf("classicplus-019") },
  extras: [
    { layer: "subject", x: 30, y: 62, scale: 0.4, pose: "pose-guard", look: normalExtraLookOf("classicplus-019", 0) },
    { layer: "subject", x: 72, y: 64, scale: 0.42, pose: "pose-flex", look: normalExtraLookOf("classicplus-019", 1) },
    { layer: "subject", x: 38, y: 80, scale: 0.38, flip: -1, pose: "pose-kneel", look: normalExtraLookOf("classicplus-019", 2) },
    { layer: "subject", x: 64, y: 82, scale: 0.36, pose: "pose-sit", look: normalExtraLookOf("classicplus-019", 3) },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 12, scale: 1.2 }],
    far: [
      { part: "field", x: 50, y: 26, paint: "green-900" },
      { part: "tree", x: 8, y: 36, scale: 1.3, paint: "green-900" },
      { part: "tree", x: 30, y: 30, scale: 0.9, paint: "green-700" },
      { part: "tree", x: 92, y: 34, scale: 1.2, paint: "green-900" },
    ],
    mid: [{ part: "fallen-column", x: 84, y: 76, scale: 0.5, paint: "wood-700" }],
    subject: [{ part: "flames", x: 50, y: 70, scale: 1.1 }],
    near: [
      { part: "grass-tuft", x: 50, y: 98, paint: "green-900" },
      { part: "banner", x: 10, y: 96, scale: 1, props: true },
      { part: "banner", x: 90, y: 96, scale: 0.9, flip: -1, props: true },
    ],
    light: [
      { part: "halo", x: 50, y: 66, scale: 1.8, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 50, y: 56, scale: 1.2, count: 2, spread: [8, 6], paint: "orange-400" },
      { part: "embers", x: 50, y: 40, scale: 0.8, paint: "orange-400" },
    ],
  },
};

/** Mushroom Power (Classic+ #20, Common), Alt art: a close-up of Mushroom Power sheltering two sprouts under its cap in a summer thunderstorm on a riverbank, its spores glowing. */
const MUSHROOM_POWER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 72, y: 20, r: 30, paint: "violet-300", opacity: 0.35 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 114, scale: 1.6, pose: "pose-spread", look: normalLookOf("classicplus-020") },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 20, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-800" },
      { part: "lightning", x: 78, y: 14, scale: 0.6 },
    ],
    far: [
      { part: "water", x: 50, y: 62, paint: "blue-900" },
      { part: "field", x: 50, y: 74, paint: "green-900" },
    ],
    mid: [
      { part: "grass-tuft", x: 18, y: 78, count: 3, spread: [8, 2], paint: "green-700" },
      { part: "fern", x: 86, y: 80, scale: 1.2, paint: "green-900" },
    ],
    near: [
      { part: "sprout", x: 30, y: 92, scale: 2.2 },
      { part: "sprout", x: 70, y: 92, scale: 2 },
    ],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "blue-200" },
      { part: "spores", x: 36, y: 60, scale: 1.1, paint: "green-400" },
      { part: "spores", x: 64, y: 56, scale: 1, paint: "green-400" },
    ],
  },
};

/** Whirlwind (Classic+ #21, Common), Alt art: seen from above, the whirlwind carving a spiral through a snowy battlefield, fighters flung outward like spokes. */
const WHIRLWIND_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "blue-800"],
  light: { x: 50, y: 58, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 84, y: 28, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 24, y: 52, scale: 0.26, flip: -1, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: null, cloth: "red-900" } },
    { layer: "mid", x: 76, y: 50, scale: 0.26, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-tunic", item: null, cloth: "slate-700" } },
    { layer: "mid", x: 20, y: 80, scale: 0.3, flip: -1, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-tunic", item: null, cloth: "stone-500" } },
    { layer: "mid", x: 80, y: 82, scale: 0.3, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: null, cloth: "blue-800" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 50, y: 12, scale: 1, count: 2, spread: [30, 2], paint: "slate-400" }],
    far: [
      { part: "field", x: 50, y: 22, paint: "slate-100" },
      { part: "banner", x: 16, y: 40, scale: 0.6 },
      { part: "pebbles", x: 80, y: 36, paint: "slate-700" },
    ],
    mid: [
      { part: "shockwave", x: 50, y: 66, scale: 1.1, paint: "white" },
      { part: "dust", x: 50, y: 62, scale: 1.4, paint: "slate-100" },
    ],
    subject: [{ part: "tornado", x: 50, y: 74, scale: 1.1, paint: "slate-400" }],
    particles: [{ part: "snow", x: 50, y: 50, scale: 1.4 }],
  },
};

/** Crushing Walls (Classic+ #24, Epic), Alt art: from above, two moss-covered canyon walls closing on a narrow river gorge at noon, rafts caught at both ends. */
const CRUSHING_WALLS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 14, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 26, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 80, y: 14, scale: 0.8, paint: "white" }],
    far: [{ part: "water", x: 50, y: 22, paint: "blue-600" }],
    mid: [
      { part: "cliff", x: 34, y: 22, scale: 1.4, paint: "green-700" },
      { part: "cliff", x: 66, y: 22, scale: 1.4, flip: -1, paint: "green-700" },
      { part: "fern", x: 12, y: 44, scale: 1.2, paint: "green-500" },
      { part: "bush", x: 88, y: 48, scale: 1.1, paint: "green-500" },
    ],
    subject: [
      { part: "dust", x: 50, y: 60, scale: 1.2, paint: "stone-500" },
      { part: "boat", x: 50, y: 40, scale: 0.35 },
      { part: "boat", x: 52, y: 84, scale: 0.4, flip: -1 },
    ],
    near: [
      { part: "tree", x: 6, y: 98, scale: 1.6, paint: "green-700" },
      { part: "tree", x: 94, y: 96, scale: 1.4, paint: "green-500" },
      { part: "pebbles", x: 50, y: 97 },
    ],
    light: [{ part: "halo", x: 50, y: 30, scale: 1.6, paint: "white" }],
    particles: [
      { part: "dust", x: 50, y: 40, scale: 1, paint: "stone-500" },
      { part: "mist", x: 50, y: 70, scale: 0.6, paint: "white" },
    ],
  },
};

/** Felinor Fuser (Classic+ #30, Epic), Alt art: after the fusion, a close-up of the cat-folk alchemist proudly cradling one fluffy two-tailed kitten in a sunny rooftop garden. */
const FELINOR_FUSER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "blue-200"],
  light: { x: 26, y: 22, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 46, y: 116, scale: 1.7, pose: "pose-stand", look: { ...normalLookOf("classicplus-030"), item: null } },
  layers: {
    sky: [
      { part: "sun", x: 22, y: 18, scale: 0.8 },
      { part: "cloud", x: 72, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "house", x: 12, y: 62, scale: 0.9 },
      { part: "house", x: 88, y: 60, scale: 0.8, flip: -1 },
      { part: "tower", x: 70, y: 54, scale: 0.5 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "red-600" },
      { part: "flowers", x: 16, y: 80, count: 3, spread: [8, 2] },
      { part: "fern", x: 86, y: 82, scale: 1.3, paint: "green-500" },
    ],
    subject: [{ part: "vine", x: 82, y: 92, scale: 1.6, paint: "green-500" }],
    near: [
      { part: "cat", x: 58, y: 78, scale: 1.8, paint: "white" },
      { part: "flowers", x: 10, y: 97, scale: 1.2 },
      { part: "fern", x: 92, y: 99, scale: 1.4, paint: "green-500" },
    ],
    light: [
      { part: "halo", x: 56, y: 72, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 46, y: 60, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 58, y: 66, scale: 0.8, count: 2, spread: [6, 4], paint: "white" },
      { part: "petals", x: 50, y: 36, scale: 1.2, paint: "green-400" },
    ],
  },
};

/** Fusion Lab (Classic+ #31, Epic), Alt art: the lab moved outdoors onto a windswept desert mesa, a welding arc joining figurines beneath a wide noon sky; a distant vista. */
const FUSION_LAB_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 14, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 50, y: 12, scale: 0.8 },
      { part: "cloud", x: 22, y: 24, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 62, paint: "brass-600" },
      { part: "cliff", x: 18, y: 58, scale: 0.9, paint: "orange-400" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 68, paint: "orange-400" },
      { part: "pipes", x: 22, y: 76, scale: 0.6, paint: "brass-600" },
      { part: "cauldron", x: 80, y: 76, scale: 0.6 },
      { part: "gear", x: 86, y: 66, scale: 0.4 },
    ],
    subject: [
      { part: "table", x: 50, y: 76, scale: 0.9, paint: "wood-700" },
      { part: "statue", x: 46, y: 70, scale: 0.5, paint: "stone-500" },
      { part: "statue", x: 54, y: 70, scale: 0.5, flip: -1, paint: "stone-500" },
      { part: "lightning", x: 50, y: 58, scale: 0.3 },
    ],
    near: [
      { part: "dust", x: 50, y: 92, scale: 1.4, paint: "orange-400" },
      { part: "pebbles", x: 20, y: 96 },
      { part: "cactus", x: 88, y: 99, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 50, y: 60, scale: 1.2, paint: "sky-300" }],
    particles: [
      { part: "sparks", x: 50, y: 60, scale: 1, count: 4, spread: [6, 4], paint: "white" },
      { part: "dust", x: 50, y: 40, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Memory Leak (Classic+ #34, Epic), Alt art: an aerial view of a sun-baked hillside village whose cobbled streets vanish tile by tile into white haze, padlocks hanging on the doors. */
const MEMORY_LEAK_ALT: SceneRecipe = {
  family: "classic",
  sky: ["orange-400", "brass-500"],
  light: { x: 20, y: 14, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 84, y: 26, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 16, y: 12, scale: 0.7 }],
    far: [
      { part: "tile-floor", x: 50, y: 26, paint: "brass-600" },
      { part: "house", x: 18, y: 38, scale: 0.4 },
      { part: "house", x: 50, y: 34, scale: 0.35 },
      { part: "house", x: 82, y: 38, scale: 0.4, flip: -1 },
    ],
    mid: [
      { part: "house", x: 28, y: 58, scale: 0.5 },
      { part: "house", x: 72, y: 56, scale: 0.5, flip: -1 },
      { part: "house", x: 50, y: 72, scale: 0.55 },
    ],
    subject: [
      { part: "padlock", x: 28, y: 54, scale: 0.7 },
      { part: "padlock", x: 72, y: 52, scale: 0.7 },
      { part: "padlock", x: 50, y: 68, scale: 0.7 },
    ],
    near: [
      { part: "mist", x: 60, y: 84, scale: 1.2, count: 2, spread: [16, 3], paint: "white" },
      { part: "house", x: 10, y: 98, scale: 0.8 },
    ],
    light: [{ part: "halo", x: 70, y: 72, scale: 1.6, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.2, paint: "white" }],
  },
};

/** Rollback (Classic+ #35, Legendary), Alt art: the chronomancer on a seaside lighthouse at dawn, rewinding the tide so the waves pull back and a wrecked ship rises whole; a wide horizon. */
const ROLLBACK_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-300", "sky-300"],
  light: { x: 80, y: 54, r: 36, paint: "red-300", opacity: 0.4 },
  emblem: { x: 50, y: 26, size: 6, rotate: 0 },
  figure: { x: 24, y: 44, scale: 0.4, pose: "pose-raise", look: { ...normalLookOf("classicplus-035"), item: "item-hourglass" } },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 54, scale: 0.9 },
      { part: "cloud", x: 44, y: 22, scale: 1, count: 2, spread: [16, 3], paint: "red-300" },
      { part: "clock-face", x: 72, y: 26, scale: 0.8 },
    ],
    far: [
      { part: "water", x: 50, y: 56, paint: "blue-600" },
      { part: "cliff", x: 14, y: 70, scale: 1, paint: "slate-700" },
    ],
    mid: [
      { part: "boat", x: 66, y: 64, scale: 0.9 },
      { part: "mist", x: 66, y: 68, scale: 0.8, paint: "white" },
      { part: "shockwave", x: 66, y: 66, scale: 0.6, paint: "white" },
    ],
    subject: [
      { part: "tower", x: 22, y: 90, scale: 1.5, paint: "stone-500" },
      { part: "light-ribbon", x: 54, y: 46, scale: 0.8, paint: "violet-300" },
    ],
    near: [
      { part: "standing-stone", x: 88, y: 100, scale: 1, paint: "slate-700" },
      { part: "pebbles", x: 60, y: 97 },
      { part: "hourglass", x: 86, y: 94, scale: 1 },
    ],
    light: [
      { part: "halo", x: 80, y: 54, scale: 2, paint: "red-300" },
      { part: "halo", x: 22, y: 46, scale: 0.8, paint: "white" },
      { part: "rim-light", x: 24, y: 40, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" },
      { part: "sparks", x: 66, y: 58, scale: 1, count: 2, spread: [8, 4], paint: "sky-300" },
    ],
  },
};

/** Conjure Bones (Classic+ #36, Rare), Alt art: a close-up of the necromancer's hands slipping tiny bones into a deck aboard a sunny river barge, ducks eyeing the bones. */
const CONJURE_BONES_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "green-200"],
  light: { x: 28, y: 18, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 20, y: 16, scale: 0.7 },
      { part: "cloud", x: 70, y: 20, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "water", x: 50, y: 52, paint: "jade-400" },
      { part: "grass-tuft", x: 20, y: 54, count: 3, spread: [12, 1], paint: "green-500" },
      { part: "wheat", x: 82, y: 54, count: 2, spread: [6, 1], paint: "green-700" },
    ],
    mid: [
      { part: "boat", x: 74, y: 60, scale: 0.7 },
      { part: "bird", x: 18, y: 64, scale: 0.7, count: 2, spread: [8, 3] },
      { part: "table", x: 50, y: 106, scale: 2.8, paint: "wood-700" },
    ],
    subject: [
      { part: "card", x: 50, y: 74, scale: 1.4, count: 3, spread: [6, 1], paint: "violet-700" },
      { part: "bones", x: 40, y: 80, scale: 0.8 },
      { part: "bones", x: 60, y: 72, scale: 0.5 },
      { part: "pale-hand", x: 36, y: 94, scale: 1.8 },
      { part: "pale-hand", x: 64, y: 94, scale: 1.8, flip: -1 },
    ],
    near: [{ part: "grass-tuft", x: 88, y: 99, count: 2, spread: [6, 1], paint: "green-500" }],
    particles: [{ part: "sparks", x: 50, y: 70, scale: 0.8, count: 2, spread: [10, 4], paint: "white" }],
  },
};

/** R3396000: Classic+'s alt recipes #2 to #36, by id. */
export const CLASSIC_PLUS_ALT_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "classicplus-002": GROOM_SHROOM_ALT,
  "classicplus-005": GUY_ATT_ALT,
  "classicplus-008": WITHERING_STORM_ALT,
  "classicplus-009": SILENCE_ALT,
  "classicplus-012": MOTHER_PANCAKE_ALT,
  "classicplus-013": MOMMY_BARKER_ALT,
  "classicplus-014": FOREVER_ALT,
  "classicplus-015": CONJURE_RUSH_TOKEN_ALT,
  "classicplus-018": GULLIBLE_TREATLER_ALT,
  "classicplus-019": LEAGUE_OF_LOSERS_ALT,
  "classicplus-020": MUSHROOM_POWER_ALT,
  "classicplus-021": WHIRLWIND_ALT,
  "classicplus-024": CRUSHING_WALLS_ALT,
  "classicplus-030": FELINOR_FUSER_ALT,
  "classicplus-031": FUSION_LAB_ALT,
  "classicplus-034": MEMORY_LEAK_ALT,
  "classicplus-035": ROLLBACK_ALT,
  "classicplus-036": CONJURE_BONES_ALT,
};
