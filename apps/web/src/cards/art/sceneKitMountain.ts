// R3393000: the scene kit's parts that the Meditative+ scenes (#3393) added: the ink-wash mountains' peaks, huts, pavilions and props. sceneKit.ts spreads the table into `SCENE_KIT`. Each part keeps
// the kit's rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream
// alone, every colour a palette name out of Radiant's gold window, a glow a stack of translucent shapes, and no
// two sub-paths of one shape overlapping but for a hole.

import type { KitPart } from "./scene.ts";

/** R3393000: the Meditative+ parts, by id, each with its kind. */
export const MOUNTAIN_KIT = {} as const satisfies Readonly<Record<string, KitPart>>;
