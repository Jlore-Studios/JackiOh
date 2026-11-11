// R3350000: Stinkyscape #111 Shockwave447 (`stinkyscape-111`)'s diorama: the viewport and the tiers at the back, the crowd, his
// light diagram turning, Shockwave447 lecturing before it, the storm of cards and gears bent and swaying, the front rows, and
// his arcs, shockwave and sparks crackling in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-111",
  slices: [
    { name: "navy hall", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "viewport and tiers", plane: "far", depth: 0.12, pick: ["scene:far:"] },
    { name: "crowd", plane: "middle", depth: 0.26, pick: ["scene:mid:"] },
    { name: "light diagram", plane: "middle", depth: 0.4, pick: ["scene:subject:light-diagram:"], loop: { move: "spin", seconds: 36 } },
    { name: "Shockwave447", plane: "middle", depth: 0.54, pick: ["scene:subject:pose-raise:", "scene:subject:outfit-harness:", "scene:subject:head-human:", "scene:subject:goggles:", "emblem"] },
    { name: "storm of cards", plane: "near", depth: 0.68, pick: ["scene:near:card:", "scene:near:cog:", "scene:near:gear:"], bend: { axis: "y", deg: 12 }, loop: { move: "sway", seconds: 3.2, origin: [50, 80] } },
    { name: "front rows", plane: "near", depth: 0.8, pick: ["scene:near:pose-spread:", "scene:near:outfit-jacket:", "scene:near:head-human:", "scene:near:hair-short:", "scene:near:pose-raise:", "scene:near:hair-long:"] },
    { name: "arcs and sparks", plane: "nearest", depth: 1, pick: ["scene:light:", "scene:particles:"], loop: { move: "pulse", seconds: 2.3 } },
  ],
};
