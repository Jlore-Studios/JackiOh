// R3350000: Stinkyscape #86 Prototype-X2 (`stinkyscape-086`)'s diorama: the collapsing forge sphere and its debris breathing at
// the back, the smoke and the flame column, the fused hull bent and floating free in the middle, the near debris, its welds'
// glow pulsing, and the embers rising and the sparks glinting one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-086",
  slices: [
    { name: "violet space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "forge sphere", plane: "far", depth: 0.12, pick: ["scene:far:"], loop: { move: "breathe", seconds: 5.6, origin: [40, 50] } },
    { name: "smoke and flame", plane: "middle", depth: 0.3, pick: ["scene:mid:"] },
    { name: "fused hull", plane: "middle", depth: 0.5, pick: ["scene:subject:", "emblem"], bend: { axis: "y", deg: -8 }, loop: { move: "float", seconds: 4.8 } },
    { name: "near debris", plane: "near", depth: 0.68, pick: ["scene:near:"] },
    { name: "weld glow", plane: "near", depth: 0.8, pick: ["scene:light:"], loop: { move: "pulse", seconds: 2.9 } },
    { name: "embers", plane: "nearest", depth: 0.9, pick: ["scene:particles:embers:"], loop: { move: "rise", seconds: 3.4, stagger: true } },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:sparks:"], loop: { move: "glint", seconds: 2.6, stagger: true } },
  ],
};
