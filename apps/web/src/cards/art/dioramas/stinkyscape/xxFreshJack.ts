// R3350000: Stinkyscape #109 xxFreshJack (`stinkyscape-109`)'s diorama: the launch pad bent as a floor at the back, the crowd's
// ships tilting in the glare, the glitching warp gate pulsing, his comet floating over him, xxFreshJack with his orb, the crowd
// and the flare, and the pixels and sparks raining one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-109",
  slices: [
    { name: "violet sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "launch pad", plane: "far", depth: 0.12, pick: ["scene:far:launch-pad:"], bend: { axis: "x", deg: 12 } },
    { name: "ships", plane: "far", depth: 0.24, pick: ["scene:far:shuttle:", "scene:far:fighter:"], loop: { move: "sway", seconds: 8.8, origin: [50, 60] } },
    { name: "warp gate", plane: "middle", depth: 0.4, pick: ["scene:mid:"], loop: { move: "pulse", seconds: 3.1 } },
    { name: "comet", plane: "middle", depth: 0.56, pick: ["scene:subject:comet:"], loop: { move: "float", seconds: 3.4 } },
    { name: "xxFreshJack", plane: "near", depth: 0.7, pick: ["scene:subject:pose-raise:", "scene:subject:outfit-jacket:", "scene:subject:head-human:", "scene:subject:visor:", "scene:subject:item-orb:", "emblem"] },
    { name: "crowd and flare", plane: "near", depth: 0.84, pick: ["scene:near:", "scene:light:"] },
    { name: "raining sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 4.4, stagger: true } },
  ],
};
