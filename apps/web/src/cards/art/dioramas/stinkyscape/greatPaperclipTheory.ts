// R3350000: Stinkyscape #44 Great Paperclip Theory (`stinkyscape-044`)'s diorama: the porthole and the station at the back, the
// ship's deck bent as a floor, the wire coil, then the staircase of drone cores lighting one after another out of the window,
// the scientist before it, the light, and the motes and sparks twinkling in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-044",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "porthole", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "deck", plane: "middle", depth: 0.32, pick: ["scene:mid:"], bend: { axis: "x", deg: 10 } },
    { name: "wire coil", plane: "middle", depth: 0.46, pick: ["scene:subject:wire-coil:"] },
    { name: "staircase", plane: "near", depth: 0.6, pick: ["scene:subject:drone-core:", "emblem"], loop: { move: "pulse", seconds: 7.2, stagger: true } },
    { name: "scientist", plane: "near", depth: 0.74, pick: ["scene:subject:pose-stand:", "scene:subject:outfit-labcoat:", "scene:subject:head-human:", "scene:subject:hair-short:"] },
    { name: "light", plane: "near", depth: 0.86, pick: ["scene:near:", "scene:light:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 5.8, stagger: true } },
  ],
};
