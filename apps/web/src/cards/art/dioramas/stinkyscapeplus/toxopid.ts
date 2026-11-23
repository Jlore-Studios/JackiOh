// R3350000: Stinkyscape+ #121 Toxopid (`stinkyscapeplus-121`)'s diorama: the smoke-red nebulae drifting at the back, the
// gantry, the reactor breathing, the shipyard crane bent over it, the Toxopid and its drill swaying in the middle, the
// cables, ore and light before it, and the embers rising one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-121",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.1, pick: ["scene:sky:nebula:"], loop: { move: "drift", seconds: 13.4 } },
    { name: "gantry", plane: "far", depth: 0.22, pick: ["scene:far:"] },
    { name: "reactor", plane: "middle", depth: 0.4, pick: ["scene:mid:reactor:"], loop: { move: "breathe", seconds: 7.9 } },
    { name: "crane", plane: "middle", depth: 0.52, pick: ["scene:mid:shipyard-crane:"], bend: { axis: "x", deg: 8 } },
    { name: "Toxopid", plane: "near", depth: 0.7, pick: ["scene:subject:", "emblem"], loop: { move: "sway", seconds: 6.6 } },
    { name: "cables and light", plane: "near", depth: 0.86, pick: ["scene:near:", "scene:light:"] },
    { name: "embers", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 3.9, stagger: true } },
  ],
};
