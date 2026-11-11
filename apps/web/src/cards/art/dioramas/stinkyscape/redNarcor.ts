// R3350000: Stinkyscape token Red Narcor (`stinkyscape-t-red-narcor`)'s diorama: the dune bent as a floor at the back, the lamp
// tent, the narcor crystal humming in its own flare, the prospector holding it, its echo rings pulsing, and its mana sparks
// spilling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-t-red-narcor",
  slices: [
    { name: "red dusk", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "dune", plane: "far", depth: 0.15, pick: ["scene:far:"], bend: { axis: "x", deg: 10 } },
    { name: "lamp tent", plane: "middle", depth: 0.34, pick: ["scene:mid:"] },
    { name: "narcor crystal", plane: "middle", depth: 0.5, pick: ["scene:subject:crystal:"], loop: { move: "flare", seconds: 3.6 } },
    { name: "prospector", plane: "near", depth: 0.68, pick: ["scene:subject:pose-hold:", "scene:subject:outfit-overalls:", "scene:subject:head-human:", "scene:subject:hat-brim:", "emblem"] },
    { name: "echo rings", plane: "near", depth: 0.84, pick: ["scene:light:"], loop: { move: "pulse", seconds: 2.8 } },
    { name: "mana sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 4.2, stagger: true } },
  ],
};
