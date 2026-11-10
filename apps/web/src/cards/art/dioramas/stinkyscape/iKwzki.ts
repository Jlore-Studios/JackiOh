// R3350000: Stinkyscape #110 iKwzki (`stinkyscape-110`)'s diorama: the planet's rings and the sun at the back, the sister spires,
// the dim starship floating beside her, iKwzki breathing on her spire, the glass parapet bent before her, the ribbon of light
// pulsing, and the motes twinkling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-110",
  slices: [
    { name: "dawn sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "rings and sun", plane: "far", depth: 0.12, pick: ["scene:far:"] },
    { name: "sister spires", plane: "middle", depth: 0.3, pick: ["scene:mid:glass-spire:"] },
    { name: "starship", plane: "middle", depth: 0.44, pick: ["scene:mid:starship:"], loop: { move: "float", seconds: 6.2 } },
    { name: "iKwzki", plane: "near", depth: 0.6, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 7.6, origin: [40, 84] } },
    { name: "glass parapet", plane: "near", depth: 0.74, pick: ["scene:near:"], bend: { axis: "y", deg: -10 } },
    { name: "ribbon of light", plane: "near", depth: 0.86, pick: ["scene:light:"], loop: { move: "pulse", seconds: 4.4 } },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 5.4, stagger: true } },
  ],
};
