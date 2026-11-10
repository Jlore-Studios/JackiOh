// R3350000: Stinkyscape #16 Rod of Ages (`stinkyscape-016`)'s diorama: the far platforms and debris drifting at the back, the
// ruin walls bent before them, then the hover platform, the ancient rod floating over it and its keeper, the near debris, and
// the light streams and motes pulsing in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-016",
  slices: [
    { name: "mist", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "floating ruins", plane: "far", depth: 0.15, pick: ["scene:far:"], loop: { move: "drift", seconds: 12.4 } },
    { name: "ruin walls", plane: "middle", depth: 0.32, pick: ["scene:mid:"], bend: { axis: "y", deg: -10 } },
    { name: "platform", plane: "middle", depth: 0.45, pick: ["scene:subject:hover-platform:"] },
    { name: "the rod", plane: "middle", depth: 0.58, pick: ["scene:subject:ancient-rod:", "emblem"], loop: { move: "float", seconds: 6.8 } },
    { name: "keeper", plane: "near", depth: 0.7, pick: ["scene:subject:pose-stand:", "scene:subject:outfit-armor:", "scene:subject:head-human:", "scene:subject:hair-short:", "scene:subject:item-sword:"] },
    { name: "near debris", plane: "near", depth: 0.84, pick: ["scene:near:"] },
    { name: "light and motes", plane: "nearest", depth: 1, pick: ["scene:light:", "scene:particles:"], loop: { move: "pulse", seconds: 5.4 } },
  ],
};
