// R3352000: the scene kit's Classic parts (ART.md, "Scene recipes and kits"), which Classic's scenes (#3352) added
// for the things their briefs name and the kit lacked. sceneKit.ts spreads them into `SCENE_KIT`. Each part keeps
// the kit's rules: one to twelve filled shapes drawn in its own frame (`local`), varied by the scene's stream alone,
// every colour a palette.css name none in Radiant's gold window, a glow or a wash a stack of translucent shapes,
// never a gradient, and every shape filled even-odd, so a hole is a second sub-path and no two sub-paths of one
// shape overlap otherwise.

import type { KitPart } from "./scene.ts";

export const CLASSIC_KIT = {} as const satisfies Readonly<Record<string, KitPart>>;
