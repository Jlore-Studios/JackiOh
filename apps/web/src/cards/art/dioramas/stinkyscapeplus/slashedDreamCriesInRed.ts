// R3350000: Stinkyscape+ #78 Slashed Dream Cries in Red (`stinkyscapeplus-078`)'s diorama: the red nebulae drifting at the
// back, the slash through space bent and pulsing, the wreckage it left bent across the middle, the dream torn open, the ice
// shards falling one after another, and the sparks and light glinting one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-078",
  slices: [
    { name: "red space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.14, pick: ["scene:sky:nebula:"], loop: { move: "drift", seconds: 11.2 } },
    { name: "the slash", plane: "far", depth: 0.3, pick: ["scene:far:"], bend: { axis: "y", deg: 15 }, loop: { move: "pulse", seconds: 4.8 } },
    { name: "wreckage", plane: "middle", depth: 0.48, pick: ["scene:mid:"], bend: { axis: "x", deg: -9 } },
    { name: "the torn dream", plane: "near", depth: 0.66, pick: ["scene:subject:", "emblem"] },
    { name: "ice shards", plane: "near", depth: 0.82, pick: ["scene:near:"], loop: { move: "fall", seconds: 7.1, stagger: true } },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:", "scene:light:"], loop: { move: "glint", seconds: 3.4, stagger: true } },
  ],
};
