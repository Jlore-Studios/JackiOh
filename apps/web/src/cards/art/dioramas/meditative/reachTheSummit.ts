// R3350000: Meditative #8 Reach the Summit (`meditative-008`)'s diorama: the ink range at the back, the great peak before it
// with its path of flames flickering one after another up to the climber breathing on the summit, the clouds and the
// summit's light pulsing in front, and its motes and sparks rising nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-008",
  slices: [
    { name: "night sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "ink range", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "great peak", plane: "middle", depth: 0.3, pick: ["scene:mid:needle-peak:"] },
    { name: "flame path", plane: "middle", depth: 0.44, pick: ["scene:mid:flame:"], loop: { move: "flare", seconds: 3.7, stagger: true } },
    { name: "climber", plane: "near", depth: 0.58, pick: ["scene:mid:heal-fountain:", "scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.6, origin: [50, 46] } },
    { name: "near clouds", plane: "near", depth: 0.74, pick: ["scene:near:"] },
    { name: "summit light", plane: "near", depth: 0.88, pick: ["scene:light:"], loop: { move: "pulse", seconds: 6.2 } },
    { name: "rising sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 5.3, stagger: true } },
  ],
};
