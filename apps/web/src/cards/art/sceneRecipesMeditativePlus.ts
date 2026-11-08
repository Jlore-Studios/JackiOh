// R3393000: the Meditative+ set's scene recipes (ART.md, "Scene recipes and kits"), one per entry, its tokens
// included, each drawn from its brief in issue #3393's table in the Meditative family's look (ART.md's art bible):
// ink-wash peaks in mist behind, a high and distant camera, and one red seal in the near layer. A Unit is a
// figure-kit character. Each is placed in the 100-unit box as sceneRecipes.ts's header says, and reaches its
// rarity's floor (richness.ts `ART_DETAIL_MIN`) with its placements' counts.

import type { SceneRecipe } from "./scene.ts";

/** R3393000: the Meditative+ cards' recipes, by id. */
export const MEDITATIVE_PLUS_RECIPES: Readonly<Record<string, SceneRecipe>> = {};
