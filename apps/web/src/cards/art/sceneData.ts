// #4222: the scene data as one lazily-imported module. `sceneLoader.ts` is the only module that
// imports this one, dynamically, so the kits and both recipe tables leave the first load and arrive
// in a chunk beside the engine module. Nothing else names these files: `first-load.test.ts` fails a
// static import of them anywhere else.

export { SCENE_KIT } from "./sceneKit.ts";
export { drawFigure } from "./figureKit.ts";
export { ALT_SCENE_RECIPES, SCENE_RECIPES } from "./sceneRecipes.ts";
