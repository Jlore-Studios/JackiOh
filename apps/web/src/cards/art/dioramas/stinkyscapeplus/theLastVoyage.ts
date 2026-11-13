// R3350000: Stinkyscape+ #80 The Last Voyage (`stinkyscapeplus-080`)'s diorama: the nebulae and the galaxy at the back, the rift
// breathing, the probe floating on its way, the wreckage of those before it bent aside, the gate it sails for, the comets
// and the light, and the motes falling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-080",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae and galaxy", plane: "far", depth: 0.12, pick: ["scene:sky:nebula:", "scene:sky:galaxy:"] },
    { name: "rift", plane: "far", depth: 0.26, pick: ["scene:far:"], loop: { move: "breathe", seconds: 10.2 } },
    { name: "probe", plane: "middle", depth: 0.44, pick: ["scene:mid:probe:"], loop: { move: "float", seconds: 6.4 } },
    { name: "wreckage", plane: "middle", depth: 0.56, pick: ["scene:mid:wreckage:"], bend: { axis: "y", deg: -7 } },
    { name: "the gate", plane: "near", depth: 0.72, pick: ["scene:subject:", "emblem"] },
    { name: "comets and light", plane: "nearest", depth: 0.88, pick: ["scene:near:", "scene:light:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 8.2, stagger: true } },
  ],
};
