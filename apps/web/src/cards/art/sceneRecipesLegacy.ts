// R3393001: the Legacy cards' scene recipes (Training Grounds #41 to #60), each a scene of its own from its brief
// in issue #3393's table, in the Training Grounds family's look, placing its own yard touch (traininggrounds.ts
// `LEGACY_TOUCHES`) once as the part `LEGACY_TOUCH_PARTS` names. Its face wears its Core card's frozen emblem
// (svg.ts `legacySceneSvg`) and never draws its Core card's scene. A part id is written out in each recipe, as
// importing a value from traininggrounds.ts here would close a module cycle; `setScenes.test.ts` checks it.

import type { SceneRecipe } from "./scene.ts";

/** R3393001: the Legacy cards' recipes, by id. */
export const LEGACY_RECIPES: Readonly<Record<string, SceneRecipe>> = {};
