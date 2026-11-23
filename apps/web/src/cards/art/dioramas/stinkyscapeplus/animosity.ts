// R3350000: Stinkyscape+ #89 Animosity (`stinkyscapeplus-089`)'s diorama: the galaxy turning at the back, the rift bent hard
// beside it, the wreckage swaying, the hole and its flash flaring in the middle with its light, the ice shards, and the
// sparks glinting one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-089",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "galaxy", plane: "far", depth: 0.16, pick: ["scene:sky:galaxy:"], loop: { move: "spin", seconds: 39 } },
    { name: "rift", plane: "far", depth: 0.32, pick: ["scene:far:"], bend: { axis: "y", deg: 18 } },
    { name: "wreckage", plane: "middle", depth: 0.5, pick: ["scene:mid:"], loop: { move: "sway", seconds: 4.7 } },
    { name: "the hole", plane: "near", depth: 0.7, pick: ["scene:subject:", "emblem", "scene:light:"], loop: { move: "flare", seconds: 2.4 } },
    { name: "shards", plane: "nearest", depth: 0.86, pick: ["scene:near:"] },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "glint", seconds: 2.2, stagger: true } },
  ],
};
