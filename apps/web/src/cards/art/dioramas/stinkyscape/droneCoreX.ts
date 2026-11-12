// R3350000: Stinkyscape #36 Drone Core X (`stinkyscape-036`)'s diorama: the planet-sized core breathing at the back, the dock
// gantry bent before it, the interceptor swarm floating, the battleship sliding out in the middle with its escorts, the near
// drone and the light, and the sparks and embers rising one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-036",
  slices: [
    { name: "red space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "the core", plane: "far", depth: 0.12, pick: ["scene:far:drone-core:"], loop: { move: "breathe", seconds: 8.6, origin: [22, 50] } },
    { name: "gantry", plane: "far", depth: 0.24, pick: ["scene:far:dock-gantry:"], bend: { axis: "y", deg: -8 } },
    { name: "interceptors", plane: "middle", depth: 0.4, pick: ["scene:mid:"], loop: { move: "float", seconds: 5.2 } },
    { name: "battleship", plane: "middle", depth: 0.58, pick: ["scene:subject:drone-battleship:", "emblem"], loop: { move: "drift", seconds: 10.4 } },
    { name: "escorts", plane: "near", depth: 0.72, pick: ["scene:subject:eye-drone:", "scene:subject:drone-core:"] },
    { name: "near drone", plane: "near", depth: 0.86, pick: ["scene:near:", "scene:light:"] },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 3.6, stagger: true } },
  ],
};
