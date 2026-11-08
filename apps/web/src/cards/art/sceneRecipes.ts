// R3331000: the scene recipes, data per card (ART.md, "Scene recipes and kits"): the five proof cards
// that set the bar each set's picture pass meets (#3331). A card listed here draws its picture from its
// recipe, on every face, in every painter that hands it one (scene.ts `sceneRecipeOf`); a card not
// listed keeps its picture byte for byte until its set's pass lists it. Each recipe is placed in the
// 100-unit box: a face's window shows about y 18 to 82 of it and a Unit's portrait 12 to 88, so each
// keeps its subject within x 30 to 70 and y 25 to 85 and reaches its rarity's floor
// (richness.ts `ART_DETAIL_MIN`) with its placements' counts. R3393000: the Meditative+ and Training Grounds
// passes keep their recipes in files of their own (sceneRecipesMeditativePlus.ts, sceneRecipesTrainingGrounds.ts and,
// for the Legacy cards, sceneRecipesLegacy.ts, R3393001), spread in after the proofs.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { LEGACY_RECIPES } from "./sceneRecipesLegacy.ts";
import { MEDITATIVE_PLUS_RECIPES } from "./sceneRecipesMeditativePlus.ts";
import { TRAINING_GROUNDS_RECIPES } from "./sceneRecipesTrainingGrounds.ts";

/** Corpse Plantation (Classic #74, Epic): a moonlit graveyard farm under its bone scarecrow. */
const CORPSE_PLANTATION: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 74, y: 20, r: 34, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 50, y: 53, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "moon", x: 74, y: 20, scale: 0.9 },
      { part: "stars", x: 34, y: 18, scale: 1.3 },
    ],
    far: [
      { part: "hills", x: 50, y: 58, paint: "slate-800" },
      { part: "dead-tree", x: 12, y: 57, scale: 1.1 },
      { part: "dead-tree", x: 88, y: 56, scale: 0.8, flip: -1 },
      { part: "fence", x: 50, y: 60, scale: 0.7, paint: "wood-800" },
      { part: "crow", x: 26, y: 32, scale: 0.7, count: 2, spread: [14, 5] },
    ],
    mid: [
      { part: "furrows", x: 50, y: 60 },
      { part: "grave-mound", x: 24, y: 64, scale: 0.4 },
      { part: "grave-mound", x: 41, y: 63.6, scale: 0.36 },
      { part: "grave-mound", x: 62, y: 63.6, scale: 0.36 },
      { part: "grave-mound", x: 79, y: 64, scale: 0.42 },
      { part: "grave-mound", x: 13, y: 70, scale: 0.62 },
      { part: "grave-mound", x: 87, y: 70.5, scale: 0.62 },
      { part: "grave-mound", x: 22, y: 79, scale: 0.85 },
      { part: "grave-mound", x: 79, y: 79, scale: 0.85 },
      { part: "tombstone", x: 31, y: 68, scale: 0.7 },
      { part: "tombstone", x: 69, y: 67, scale: 0.6 },
      { part: "tombstone", x: 90, y: 77, scale: 0.9 },
      { part: "mist", x: 34, y: 76, scale: 0.7, count: 2, spread: [18, 4] },
    ],
    subject: [{ part: "bone-scarecrow", x: 50, y: 79, scale: 1.25 }],
    near: [
      { part: "pale-hand", x: 34, y: 88, scale: 1 },
      { part: "pale-hand", x: 82, y: 92, scale: 1.1, flip: -1 },
      { part: "bone-hand", x: 63, y: 90, scale: 0.95 },
      { part: "sprout", x: 46, y: 88, count: 4, spread: [26, 5] },
      { part: "fence-post", x: 12, y: 92 },
      { part: "lantern", x: 16.8, y: 80.2 },
    ],
    light: [
      { part: "halo", x: 16.8, y: 84.4, scale: 1.1 },
      { part: "halo", x: 50, y: 41.3, scale: 0.45, paint: "green-400" },
      { part: "rim-light", x: 50, y: 52, scale: 1.2, paint: "blue-200" },
    ],
    particles: [{ part: "motes", x: 50, y: 74, scale: 1.4, paint: "green-200" }],
  },
};

/** Speciesist (Core #2, Common): a Human on guard, shield raised, in a field at dusk. */
const SPECIESIST: SceneRecipe = {
  family: "core",
  sky: ["blue-800", "orange-400"],
  light: { x: 50, y: 50, r: 36, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 60.2, y: 52.8, size: 8, rotate: 0 },
  figure: { x: 50, y: 86, scale: 1.05, pose: "pose-guard", look: { ...figureLookFor(["Human"]), item: "item-shield" } },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 24, scale: 0.8, count: 2, spread: [30, 6] }],
    mid: [
      { part: "hills", x: 50, y: 70 },
      { part: "field", x: 50, y: 80 },
      { part: "grass-tuft", x: 30, y: 88, count: 6, spread: [24, 6] },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4 }],
  },
};

/** Heroic Power (Core #98, Mythic): a shrine on the heights, its orb ringed by crystals, under the rays. */
const HEROIC_POWER: SceneRecipe = {
  family: "core",
  sky: ["blue-900", "sky-400"],
  light: { x: 50, y: 46, r: 40, paint: "cyan-400", opacity: 0.55 },
  emblem: { x: 50, y: 46, size: 8, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 20, y: 24, scale: 0.7, count: 3, spread: [34, 8] }],
    far: [
      { part: "cliff", x: 24, y: 54, scale: 0.8 },
      { part: "cliff", x: 76, y: 56, scale: 0.8, flip: -1 },
      { part: "tower", x: 12, y: 52, scale: 0.7 },
      { part: "tower", x: 88, y: 54, scale: 0.6 },
      { part: "banner", x: 20, y: 54, scale: 0.6, count: 3, spread: [6, 1] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64 },
      { part: "column", x: 30, y: 72, scale: 0.9 },
      { part: "column", x: 70, y: 72, scale: 0.9 },
      { part: "column", x: 20, y: 76, scale: 1 },
      { part: "column", x: 80, y: 76, scale: 1 },
      { part: "arch", x: 50, y: 72, scale: 1.15 },
    ],
    subject: [
      { part: "crystal", x: 50, y: 31, scale: 0.9 },
      { part: "crystal", x: 37, y: 35, scale: 0.8, flip: -1 },
      { part: "crystal", x: 63, y: 35, scale: 0.8 },
      { part: "crystal", x: 32, y: 49, scale: 0.75, flip: -1 },
      { part: "crystal", x: 68, y: 49, scale: 0.75 },
      { part: "crystal", x: 38, y: 63, scale: 1, flip: -1 },
      { part: "crystal", x: 62, y: 63, scale: 1 },
      { part: "crystal", x: 50, y: 67, scale: 1.2 },
      { part: "orb", x: 50, y: 46, scale: 1.7 },
    ],
    near: [
      { part: "rock-floor", x: 50, y: 80 },
      { part: "banner", x: 88, y: 96, scale: 1.1, flip: -1 },
    ],
    light: [
      { part: "god-rays", x: 50, y: -4 },
      { part: "halo", x: 50, y: 46, scale: 2.2, paint: "sky-300" },
      { part: "light-shaft", x: 50, y: 0, count: 3, spread: [20, 0] },
    ],
    particles: [
      { part: "sparks", x: 50, y: 46, scale: 1.2, count: 4, spread: [18, 16] },
      { part: "embers", x: 50, y: 62, scale: 1.6, count: 5, spread: [22, 12] },
    ],
  },
};

/**
 * Ceaseless Void (Core #100, Mythic): the void given form among the ruins of a world. Its subject is its own
 * composition's, a hole in the shape of a giant with stars falling into it (R3338001, procedural.ts
 * `voidGiant`), its emblem on the giant's chest, in front of a swirl of the void.
 */
const CEASELESS_VOID: SceneRecipe = {
  family: "core",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 46, r: 42, paint: "violet-500", opacity: 0.6 },
  ownSubject: true,
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2.2, count: 2, spread: [10, 6] },
      { part: "nebula", x: 70, y: 28, scale: 1.2 },
    ],
    far: [
      { part: "planet", x: 18, y: 24, scale: 1.1 },
      { part: "tower", x: 84, y: 70, scale: 0.8 },
      { part: "asteroid", x: 50, y: 40, scale: 0.7, count: 6, spread: [40, 24] },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 82 },
      { part: "ruin-wall", x: 18, y: 84, scale: 1 },
      { part: "ruin-wall", x: 82, y: 86, scale: 0.9, flip: -1 },
    ],
    subject: [{ part: "void-swirl", x: 50, y: 44, scale: 1.9 }],
    near: [{ part: "crystal", x: 50, y: 94, count: 6, spread: [40, 4] }],
    light: [
      { part: "rim-light", x: 50, y: 52, scale: 1.8, paint: "violet-300" },
      { part: "halo", x: 50, y: 30, scale: 2, paint: "purple-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.6, count: 3, spread: [10, 10], paint: "violet-300" },
      { part: "sparks", x: 50, y: 28, scale: 1.2, count: 3, spread: [22, 8] },
      { part: "embers", x: 50, y: 70, scale: 1.4, count: 2, spread: [24, 8], paint: "purple-400" },
    ],
  },
};

/** Arg Arg Defender (Stinkyscape #15, Common): an armoured defender with shield raised among the stars. */
const ARG_ARG_DEFENDER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 68, y: 30, r: 30, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 60.2, y: 52.8, size: 8, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.05,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), outfit: "outfit-armor", item: "item-shield", cloth: "slate-400" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 30, scale: 2 },
      { part: "nebula", x: 30, y: 30 },
      { part: "planet", x: 78, y: 22, scale: 0.8 },
    ],
    mid: [
      { part: "asteroid", x: 18, y: 62, scale: 1.4 },
      { part: "crate", x: 76, y: 88, scale: 1.1 },
    ],
  },
};

/** R3331000: the cards that draw from a scene, by id: the five proof cards; R3393000: and every Meditative+ and Training Grounds entry. */
export const SCENE_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "classic-074": CORPSE_PLANTATION,
  "core-002": SPECIESIST,
  "core-098": HEROIC_POWER,
  "core-100": CEASELESS_VOID,
  "stinkyscape-015": ARG_ARG_DEFENDER,
  ...MEDITATIVE_PLUS_RECIPES,
  ...TRAINING_GROUNDS_RECIPES,
  ...LEGACY_RECIPES,
};
