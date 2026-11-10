// R3350000: Stinkyscape #148.1 Edge of the Universe (`stinkyscape-148-1`)'s diorama: the galaxy turning at the back, the frayed
// edge of white light bent beside it, the rock, the traveller stepping off it, the light pulsing, and the motes and shards
// falling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-148-1",
  slices: [
    { name: "starfield", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "galaxy", plane: "far", depth: 0.14, pick: ["scene:sky:galaxy:"], loop: { move: "spin", seconds: 42 } },
    { name: "the edge", plane: "far", depth: 0.3, pick: ["scene:far:"], bend: { axis: "y", deg: 14 } },
    { name: "rock", plane: "middle", depth: 0.5, pick: ["scene:subject:asteroid:"] },
    { name: "traveller", plane: "near", depth: 0.68, pick: ["scene:subject:pose-stride:", "scene:subject:outfit-cloak:", "scene:subject:head-human:", "scene:subject:hair-long:", "emblem"] },
    { name: "white light", plane: "near", depth: 0.84, pick: ["scene:light:"], loop: { move: "pulse", seconds: 6.8 } },
    { name: "motes and shards", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 5.6, stagger: true } },
  ],
};
