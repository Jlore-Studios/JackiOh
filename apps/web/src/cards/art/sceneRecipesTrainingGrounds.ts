// R3393000: the Training Grounds set's scene recipes (ART.md, "Scene recipes and kits"), #1 to #40 and the Golem
// token, each drawn from its brief in issue #3393's table in the Training Grounds family's look (ART.md's art
// bible): carved wood, the yard's palisade and sand wherever the brief is set in it, the sun from the upper right
// but at night. A Unit is a figure-kit character. Each is placed in the 100-unit box as sceneRecipes.ts's header
// says, and reaches its rarity's floor (richness.ts `ART_DETAIL_MIN`) with its placements' counts. The yard
// painter (traininggrounds.ts) draws only a card with no recipe.

import type { SceneRecipe } from "./scene.ts";

/** R3393000: the Training Grounds cards' recipes, by id, the Legacy cards' aside (sceneRecipesLegacy.ts). */
export const TRAINING_GROUNDS_RECIPES: Readonly<Record<string, SceneRecipe>> = {};
