// R3331000: the scene recipes, data per card (ART.md, "Scene recipes and kits"): the proof cards that set
// the bar each set's picture pass meets (#3331), R3390000's Classic+ scenes (classicPlusRecipes1.ts to
// classicPlusRecipes3.ts), R3392000's Core scenes, every Core entry but Ceaseless Void in three files of
// its own (sceneRecipesCore1.ts to sceneRecipesCore3.ts; Speciesist's and Heroic Power's proof scenes are
// redrawn there from their briefs), R3401000's Meditative scenes, every Meditative entry
// (meditativeRecipes1.ts to meditativeRecipes5.ts), and R3391000's Stinkyscape scenes, cards 1 to 90 in a file of
// their own (stinkyscapeRecipes.ts; Arg Arg Defender's proof scene is redrawn there from its brief). A card listed
// here draws its picture from its recipe, on every face, in every painter that hands it one (scene.ts
// `sceneRecipeOf`); a card not listed keeps its picture byte for byte until its set's pass lists it. Each recipe is
// placed in the 100-unit box: a face's window shows about y 18 to 82 of it and a Unit's portrait 12 to 88, so each
// keeps its subject within x 30 to 70 and y 25 to 85 and reaches its rarity's floor (richness.ts `ART_DETAIL_MIN`)
// with its placements' counts. R3393000: the Meditative+ and Training Grounds passes keep their recipes in files of
// their own (sceneRecipesMeditativePlus.ts, sceneRecipesTrainingGrounds.ts and, for the Legacy cards,
// sceneRecipesLegacy.ts, R3393001), spread in after the Core scenes. R3396000: a Classic+ card offered Alt
// art draws an alt recipe of its own (`ALT_SCENE_RECIPES`) under its Alt art seed (scene.ts `sceneRecipeOf`);
// R3397000: so does a Meditative+ or Training Grounds card offered it (meditativePlusAltRecipes.ts,
// trainingGroundsAltRecipes.ts), and R3404000: so does a Meditative card offered Alt art
// (meditativeAltRecipes1.ts to meditativeAltRecipes3.ts), and R3398000 a Core card (coreAltRecipes1.ts to
// coreAltRecipes3.ts), and R3406000 every card of #3406's table, Stinkyscape's later cards
// (stinkyscape91AltRecipes1.ts to stinkyscape91AltRecipes3.ts).

import { CLASSIC_PLUS_ALT_RECIPES_1 } from "./classicPlusAltRecipes1.ts";
import { CLASSIC_PLUS_ALT_RECIPES_2 } from "./classicPlusAltRecipes2.ts";
import { CLASSIC_PLUS_RECIPES_1 } from "./classicPlusRecipes1.ts";
import { CLASSIC_PLUS_RECIPES_2 } from "./classicPlusRecipes2.ts";
import { CLASSIC_PLUS_RECIPES_3 } from "./classicPlusRecipes3.ts";
import { CORE_ALT_RECIPES_1 } from "./coreAltRecipes1.ts";
import { CORE_ALT_RECIPES_2 } from "./coreAltRecipes2.ts";
import { CORE_ALT_RECIPES_3 } from "./coreAltRecipes3.ts";
import { MEDITATIVE_ALT_RECIPES_1 } from "./meditativeAltRecipes1.ts";
import { MEDITATIVE_ALT_RECIPES_2 } from "./meditativeAltRecipes2.ts";
import { MEDITATIVE_ALT_RECIPES_3 } from "./meditativeAltRecipes3.ts";
import { MEDITATIVE_PLUS_ALT_RECIPES } from "./meditativePlusAltRecipes.ts";
import { MEDITATIVE_RECIPES_1 } from "./meditativeRecipes1.ts";
import { MEDITATIVE_RECIPES_2 } from "./meditativeRecipes2.ts";
import { MEDITATIVE_RECIPES_3 } from "./meditativeRecipes3.ts";
import { MEDITATIVE_RECIPES_4 } from "./meditativeRecipes4.ts";
import { MEDITATIVE_RECIPES_5 } from "./meditativeRecipes5.ts";
import type { SceneRecipe } from "./scene.ts";
import { STINKYSCAPE_91_ALT_RECIPES_1 } from "./stinkyscape91AltRecipes1.ts";
import { STINKYSCAPE_91_ALT_RECIPES_2 } from "./stinkyscape91AltRecipes2.ts";
import { STINKYSCAPE_91_ALT_RECIPES_3 } from "./stinkyscape91AltRecipes3.ts";
import { STINKYSCAPE_91_RECIPES_1 } from "./stinkyscape91Recipes1.ts";
import { STINKYSCAPE_91_RECIPES_2 } from "./stinkyscape91Recipes2.ts";
import { STINKYSCAPE_91_RECIPES_3 } from "./stinkyscape91Recipes3.ts";
import { CORE_SCENES_1 } from "./sceneRecipesCore1.ts";
import { CORE_SCENES_2 } from "./sceneRecipesCore2.ts";
import { CORE_SCENES_3 } from "./sceneRecipesCore3.ts";
import { LEGACY_RECIPES } from "./sceneRecipesLegacy.ts";
import { MEDITATIVE_PLUS_RECIPES } from "./sceneRecipesMeditativePlus.ts";
import { TRAINING_GROUNDS_RECIPES } from "./sceneRecipesTrainingGrounds.ts";
import { STINKYSCAPE_RECIPES } from "./stinkyscapeRecipes.ts";
import { TRAINING_GROUNDS_ALT_RECIPES } from "./trainingGroundsAltRecipes.ts";

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

/** R3403000: every entry of #3403's table (Stinkyscape #91 to #176 and their tokens, and the early cards' tokens it names), by id. */
export const STINKYSCAPE_91_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  ...STINKYSCAPE_91_RECIPES_1,
  ...STINKYSCAPE_91_RECIPES_2,
  ...STINKYSCAPE_91_RECIPES_3,
};

/** R3406000: the alt recipes of #3406's table (Stinkyscape's later cards offered Alt art, the Dormant Starship aside), by id. */
export const STINKYSCAPE_91_ALT_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  ...STINKYSCAPE_91_ALT_RECIPES_1,
  ...STINKYSCAPE_91_ALT_RECIPES_2,
  ...STINKYSCAPE_91_ALT_RECIPES_3,
};

/** R3331000: the cards that draw from a scene, by id: the proof cards left here, R3390000 every Classic+ entry, R3391000 every Stinkyscape entry of its pass (cards 1 to 90), R3392000 every Core entry, R3393000 every Meditative+ and Training Grounds entry, R3401000 every Meditative entry and R3403000 #3403's Stinkyscape entries. */
export const SCENE_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "classic-074": CORPSE_PLANTATION,
  "core-100": CEASELESS_VOID,
  ...STINKYSCAPE_RECIPES,
  ...CORE_SCENES_1,
  ...CORE_SCENES_2,
  ...CORE_SCENES_3,
  ...MEDITATIVE_PLUS_RECIPES,
  ...TRAINING_GROUNDS_RECIPES,
  ...LEGACY_RECIPES,
  ...CLASSIC_PLUS_RECIPES_1,
  ...CLASSIC_PLUS_RECIPES_2,
  ...CLASSIC_PLUS_RECIPES_3,
  ...STINKYSCAPE_91_RECIPES,
  ...MEDITATIVE_RECIPES_1,
  ...MEDITATIVE_RECIPES_2,
  ...MEDITATIVE_RECIPES_3,
  ...MEDITATIVE_RECIPES_4,
  ...MEDITATIVE_RECIPES_5,
};

/**
 * R3396000: the alt recipes, by card id: every Classic+ card offered Alt art (`version-alt-art`) draws its subject
 * restaged in another place, time or moment from its brief (classicPlusAltRecipes1.ts and classicPlusAltRecipes2.ts),
 * read under its Alt art seed alone (scene.ts `sceneRecipeOf`), at every Alt art level (R3015002). R3397000: and so
 * does every Meditative+ and Training Grounds card offered it (meditativePlusAltRecipes.ts, trainingGroundsAltRecipes.ts).
 * R3404000: and so does every Meditative card offered Alt art (meditativeAltRecipes1.ts to meditativeAltRecipes3.ts).
 * R3398000: and every Core card offered it (coreAltRecipes1.ts to coreAltRecipes3.ts). R3406000: and every card of
 * #3406's table (`STINKYSCAPE_91_ALT_RECIPES`), a Starship Piece's, a Jlockeed weapon's and Dice of Doom's among them,
 * whose normal face their own painter still draws.
 */
export const ALT_SCENE_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  ...CLASSIC_PLUS_ALT_RECIPES_1,
  ...CLASSIC_PLUS_ALT_RECIPES_2,
  ...MEDITATIVE_ALT_RECIPES_1,
  ...MEDITATIVE_ALT_RECIPES_2,
  ...MEDITATIVE_ALT_RECIPES_3,
  ...MEDITATIVE_PLUS_ALT_RECIPES,
  ...TRAINING_GROUNDS_ALT_RECIPES,
  ...CORE_ALT_RECIPES_1,
  ...CORE_ALT_RECIPES_2,
  ...CORE_ALT_RECIPES_3,
  ...STINKYSCAPE_91_ALT_RECIPES,
};

/**
 * R3396000: the least share of an Alt art window's sampled points that differ from the same card's normal face, base
 * and Radiant: e2e/cypress/component/classicplus-alt-scenes.cy.tsx samples both windows on card-versions.cy.tsx's grid
 * and counts a point as differing as card-versions.cy.tsx does for `PICTURE_SHARE_MIN`; R3397000: and
 * e2e/cypress/component/set-alt-scenes.cy.tsx for Meditative+'s and Training Grounds'; R3404000: and
 * e2e/cypress/component/meditative-alt-scenes.cy.tsx for Meditative's; R3398000: and
 * e2e/cypress/component/core-alt-scenes.cy.tsx the Core cards' the same way; R3406000: and
 * e2e/cypress/component/stinkyscape91-alt-scenes.cy.tsx for Stinkyscape's later cards'.
 */
export const ALT_PICTURE_CHANGE_MIN = 0.6;
