// R3350000: Stinkyscape+ #133 Great Paperclip Theory III (`stinkyscapeplus-133`)'s diorama: the alarm lights pulsing one after
// another at the back, the nebulae, the gantry, the reactor and crane bent as a floor, the mech and its drill floating in
// the middle, the light and the cables before it, and the embers falling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-133",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "alarm lights", plane: "far", depth: 0.1, pick: ["scene:sky:alarm-light:"], loop: { move: "pulse", seconds: 3.7, stagger: true } },
    { name: "nebulae", plane: "far", depth: 0.2, pick: ["scene:sky:nebula:"] },
    { name: "gantry", plane: "middle", depth: 0.36, pick: ["scene:far:"] },
    { name: "reactor and crane", plane: "middle", depth: 0.5, pick: ["scene:mid:"], bend: { axis: "x", deg: -10 } },
    { name: "mech and drill", plane: "near", depth: 0.68, pick: ["scene:subject:", "emblem"], loop: { move: "float", seconds: 5.9 } },
    { name: "light and cables", plane: "near", depth: 0.84, pick: ["scene:light:", "scene:near:"] },
    { name: "embers", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 6.3, stagger: true } },
  ],
};
