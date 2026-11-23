// R3350000: Stinkyscape+ #155 The Far Vagabond (`stinkyscapeplus-155`)'s diorama: the nebulae drifting at the back, the warp
// gate bent far off, the satellite and the freighter, the vagabond's starship bobbing in the middle, its lens light
// pulsing, and the speed streaks, ore and comets in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-155",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.14, pick: ["scene:sky:nebula:"], loop: { move: "drift", seconds: 14.1 } },
    { name: "warp gate", plane: "far", depth: 0.3, pick: ["scene:far:"], bend: { axis: "y", deg: -13 } },
    { name: "satellite and freighter", plane: "middle", depth: 0.48, pick: ["scene:mid:"] },
    { name: "the vagabond", plane: "near", depth: 0.66, pick: ["scene:subject:", "emblem"], loop: { move: "bob", seconds: 3.8 } },
    { name: "lens light", plane: "near", depth: 0.82, pick: ["scene:light:"], loop: { move: "pulse", seconds: 6.9 } },
    { name: "streaks and comets", plane: "nearest", depth: 1, pick: ["scene:near:", "scene:particles:"] },
  ],
};
