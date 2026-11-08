// R3393000, R3393001: the scene kit's yard props that the Training Grounds and Legacy scenes (#3393) added: an
// archery target, and each Legacy card's yard touch (traininggrounds.ts `LEGACY_TOUCHES`) drawn anew as a kit part.
// sceneKit.ts spreads the table into `SCENE_KIT`. Each part keeps the kit's rules: one to twelve filled shapes drawn
// in its own frame (`local`), varied by the scene's stream alone, every colour a palette name out of Radiant's gold
// window, a glow a stack of translucent shapes, and no two sub-paths of one shape overlapping but for a hole.

import { between, INKED, local, MAIN_INKED, shape } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

/** An archery target board on two splayed legs, its foot at the anchor: four rings round a red heart; about 10 wide and 14 tall at scale 1. */
function target(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.6, 0.6);
  const cx = tilt;
  const cy = -9;
  return [
    shape(f.band([[-3.6, 0], [cx - 0.8, cy]], 0.9) + f.band([[3.6, 0], [cx + 0.8, cy]], 0.9), "wood-700", 1, INKED),
    shape(f.band([[cx, cy + 2], [cx + 1.6, 0.2]], 0.8), "wood-800", 1),
    shape(f.circle(cx, cy, 5) + f.circle(cx, cy, 3.6), "slate-100", 1, MAIN_INKED),
    shape(f.circle(cx, cy, 3.6) + f.circle(cx, cy, 2.3), "blue-700", 1),
    shape(f.circle(cx, cy, 2.3) + f.circle(cx, cy, 1.1), "white", 1),
    shape(f.circle(cx, cy, 1.1), "red-600", 1, INKED),
  ];
}

/** R3393000: the yard's props and the Legacy cards' touches, by id, each with its kind. */
export const TOUCHES_KIT = {
  target: { category: "objects", draw: target },
} as const satisfies Readonly<Record<string, KitPart>>;
