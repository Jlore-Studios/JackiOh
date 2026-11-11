// R3350000: Stinkyscape #4.5 1v1 Final Destination (`stinkyscape-004-5`)'s diorama: the abyss at the back, the broken lanes over
// it bent like a floor, their shards falling into it one by one, the one open lane pulsing with light, and its two heroes
// nearest, bouncing in turn.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004-5",
  slices: [
    { name: "void", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-2"] },
    { name: "abyss", plane: "far", depth: 0.26, pick: ["subject:0-4"] },
    { name: "broken lanes", plane: "middle", depth: 0.44, pick: ["subject:5-29"], bend: { axis: "x", deg: 12 } },
    { name: "falling shards", plane: "middle", depth: 0.58, pick: ["subject:30-32"], loop: { move: "fall", seconds: 4.6, stagger: true } },
    { name: "open lane", plane: "near", depth: 0.76, pick: ["subject:33-35"], loop: { move: "pulse", seconds: 3.8 } },
    { name: "heroes", plane: "nearest", depth: 1, pick: ["subject:36-37"], loop: { move: "float", seconds: 3.4, stagger: true } },
  ],
};
