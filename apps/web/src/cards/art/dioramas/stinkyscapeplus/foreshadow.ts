// R3350000: Stinkyscape+ #72 Foreshadow (`stinkyscapeplus-072`)'s diorama: the alarm lights flashing one after another at the
// back, the gantry bent round it, the reactor and the crane, the great gun-mech and its drill, its charging beam flaring
// before it, the cables and ore, and the sparks and embers rising one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-072",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "alarm lights", plane: "far", depth: 0.12, pick: ["scene:sky:alarm-light:"], loop: { move: "pulse", seconds: 2.6, stagger: true } },
    { name: "gantry", plane: "far", depth: 0.26, pick: ["scene:far:"], bend: { axis: "y", deg: -9 } },
    { name: "reactor and crane", plane: "middle", depth: 0.44, pick: ["scene:mid:"] },
    { name: "gun-mech", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"] },
    { name: "charging beam", plane: "near", depth: 0.78, pick: ["scene:light:"], loop: { move: "flare", seconds: 3.3 } },
    { name: "cables and ore", plane: "near", depth: 0.9, pick: ["scene:near:"] },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 4.1, stagger: true } },
  ],
};
