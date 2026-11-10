// R3350000: Meditative #39.5 Jade Beauty (`meditative-039-5`)'s diorama: the ink peaks and the cliffs at the back, the rock
// floor bent toward the viewer with its jade mist drifting over it, the Beauty breathing on her boulder, and her two guards
// standing nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-039-5",
  slices: [
    { name: "pale sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "peaks", plane: "far", depth: 0.14, pick: ["scene:far:ink-peaks:"] },
    { name: "cliffs", plane: "far", depth: 0.26, pick: ["scene:far:cliff:"] },
    { name: "rock floor", plane: "middle", depth: 0.38, pick: ["scene:mid:rock-floor:"], bend: { axis: "x", deg: 10 } },
    { name: "jade mist", plane: "middle", depth: 0.48, pick: ["scene:mid:mist:"], loop: { move: "drift", seconds: 9.8, origin: [50, 72] } },
    { name: "jade beauty", plane: "near", depth: 0.64, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 7.4, origin: [50, 84] } },
    { name: "guards", plane: "near", depth: 0.82, pick: ["scene:near:soldier:"], loop: { move: "bob", seconds: 5.6 } },
    { name: "seal", plane: "nearest", depth: 1, pick: ["scene:near:ink-seal:"] },
  ],
};
