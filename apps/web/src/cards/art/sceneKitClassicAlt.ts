// R3388000: the scene kit's parts that Classic's Alt art pass (#3388) added (ART.md, "Classic's Alt art restaged"),
// so a brief's key creature is no stand-in: the frogs swelling and shrinking on Transmutable Toxins' pink river and
// the shark Turtinator swivels its cannon toward. sceneKit.ts spreads them into `SCENE_KIT` after Classic's own parts
// and before the Stinkyscape Alt art pass's, which stay last. Each keeps the kit's rules: one to twelve filled shapes
// drawn in its own frame (`local`), varied by the scene's stream alone, every colour a palette.css name, none in
// Radiant's gold window; shapes are filled even-odd, so no two sub-paths of one shape overlap.

import { between, INKED, local, MAIN_INKED, shape } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

// Creatures

/** R3388000: a frog squatting on the anchor, facing the way it faces: a plump body and folded legs (main), a pale throat the stream puffs, two bulging eyes on top of its head and a wide mouth; about 10 wide and 8 tall at scale 1. */
function frog(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const puff = between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-5.4, 0], [-1, 0], [-1.6, -0.8], [-4.6, -0.9]]) + f.poly([[2.2, 0], [4.8, 0], [4.2, -0.6], [2.6, -0.7]]), "green-700", 1, INKED),
    shape(
      `M${f.p(-4.8, -2)}C${f.p(-4.8, -5.4)} ${f.p(-1.4, -6.6)} ${f.p(1.6, -6.8)}C${f.p(3.8, -7)} ${f.p(5.2, -6)} ${f.p(5.2, -4.8)}C${f.p(5.2, -3.6)} ${f.p(3.6, -2.6)} ${f.p(1.6, -2)}C${f.p(-0.6, -1.2)} ${f.p(-4.8, -0.6)} ${f.p(-4.8, -2)}Z`,
      "green-500",
      1,
      MAIN_INKED,
    ),
    shape(f.ellipse(-2.6, -2.4, 2.6, 1.7) + f.band([[2.6, -2.8], [3, -1.2], [3.2, -0.3]], 1.1), "green-500", 1, MAIN_INKED),
    shape(f.ellipse(3.3, -3.3, 1.5 + puff * 0.5, 1.1 + puff), "green-200", 1, INKED),
    shape(f.circle(-1.6, -4.6, 0.5) + f.circle(0.2, -5.4, 0.4) + f.circle(-3.4, -3.4, 0.4), "green-700", 0.6),
    shape(f.circle(2.6, -7, 1.05) + f.circle(0.4, -6.8, 0.85), "white", 1, INKED),
    shape(f.circle(2.9, -7, 0.5) + f.circle(0.6, -6.8, 0.4), "black", 1),
    shape(f.band([[5, -4.9], [3.8, -4.5], [2.2, -4.7]], 0.25), "slate-900", 1),
  ];
}

/** R3388000: a shark swimming level about the anchor, facing the way it faces: a long tapering body and a forked tail the stream bends (main), a pale belly, dark fins, gill slits, a wide mouth and a black eye; about 32 long and 12 tall at scale 1. */
function shark(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -1, 1);
  return [
    shape(f.poly([[-4, -3.4], [1.4, -3.6], [-2.6, -7.2]]) + f.poly([[-10, -1.6], [-8, -2.1], [-9.6, -3.6]]), "slate-700", 1, INKED),
    shape(f.poly([[-13, -0.8], [-17.4, -5.4 + bend], [-15.8, -0.2 + bend * 0.3], [-17.2, 4.2 + bend], [-13, 0.8]]), "slate-400", 1, MAIN_INKED),
    shape(
      `M${f.p(15.6, 0.4)}C${f.p(14.6, -2.6)} ${f.p(8, -3.8)} ${f.p(2, -3.8)}C${f.p(-5, -3.8)} ${f.p(-10, -2)} ${f.p(-13.6, -0.6)}L${f.p(-13.6, 0.6)}C${f.p(-10, 1.6)} ${f.p(-5, 3.2)} ${f.p(2, 3.2)}C${f.p(8, 3.2)} ${f.p(13.6, 2.6)} ${f.p(15.6, 0.4)}Z`,
      "slate-400",
      1,
      MAIN_INKED,
    ),
    shape(`M${f.p(14.2, 1.2)}C${f.p(10, 2.8)} ${f.p(4, 3)} ${f.p(0, 2.9)}C${f.p(-5, 2.6)} ${f.p(-9, 1.6)} ${f.p(-12.6, 0.6)}C${f.p(-8, 1)} ${f.p(-3, 1.4)} ${f.p(2, 1.4)}C${f.p(7, 1.4)} ${f.p(11, 1.4)} ${f.p(14.2, 1.2)}Z`, "slate-100", 1),
    shape(f.band([[10, -2.8], [2, -3.2], [-6, -2.4]], 0.6), "white", 0.2),
    shape(
      f.band([[7, -1.8], [6.6, 0.6]], 0.3) + f.band([[8.2, -1.9], [7.8, 0.7]], 0.3) + f.band([[9.4, -1.9], [9, 0.7]], 0.3) + f.band([[15, 0.9], [12.6, 1.5], [11.2, 1.3]], 0.3),
      "slate-800",
      1,
    ),
    shape(f.circle(11.6, -1, 0.55), "black", 1),
    shape(f.poly([[3, 1.6], [6.4, 2], [1.4, 5.8]]), "slate-700", 1, INKED),
  ];
}

/** R3388000: the parts Classic's Alt art pass added, by id; sceneKit.ts spreads them into `SCENE_KIT` before the Stinkyscape Alt art pass's. */
export const CLASSIC_ALT_KIT = {
  frog: { category: "creatures", draw: frog },
  shark: { category: "creatures", draw: shark },
} as const satisfies Readonly<Record<string, KitPart>>;
