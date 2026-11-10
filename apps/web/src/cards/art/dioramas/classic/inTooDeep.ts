// R3350000: Classic #90 In Too Deep (`classic-090`)'s diorama: the flooded stairway at the back of the cavern, the ledge, the
// spiral shaft breathing like a dizzy pull downward, the adventurer with her lantern, the flood bent like a floor, the violet
// glow pulsing from the depths, and the motes and crystals falling into the shaft at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "classic-090",
  slices: [
    { name: "cavern", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "stairway", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "ledge", plane: "middle", depth: 0.28, pick: ["scene:mid:"] },
    { name: "shaft", plane: "middle", depth: 0.42, pick: ["scene:subject:spiral-shaft:"], loop: { move: "breathe", seconds: 9.4, origin: [58, 66] } },
    { name: "adventurer", plane: "near", depth: 0.56, pick: ["scene:subject:pose-hold:", "scene:subject:outfit-travel-cloak:", "scene:subject:head-human:", "scene:subject:hair-long:", "scene:subject:item-lantern:", "emblem"] },
    { name: "flood", plane: "near", depth: 0.7, pick: ["scene:near:"], bend: { axis: "x", deg: 9 } },
    { name: "glow", plane: "nearest", depth: 0.84, pick: ["scene:light:"], loop: { move: "pulse", seconds: 5.8 } },
    { name: "falling motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 6.2, stagger: true } },
  ],
};
