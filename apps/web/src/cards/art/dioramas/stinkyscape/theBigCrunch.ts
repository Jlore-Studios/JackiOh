// R3350000: Stinkyscape #4.6 The Big Crunch (`stinkyscape-004-6`)'s diorama: the shrinking rings and the galaxies drawn after
// them wheeling slowly about the point at the back, the light streaming in streak by streak, the point's glow, and the point
// itself flaring nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004-6",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-3"] },
    { name: "rings and galaxies", plane: "far", depth: 0.3, pick: ["subject:0-5"], loop: { move: "spin", seconds: 64 } },
    { name: "streaming light", plane: "middle", depth: 0.55, pick: ["subject:6-28"], loop: { move: "pulse", seconds: 4.6, stagger: true } },
    { name: "core glow", plane: "near", depth: 0.78, pick: ["subject:29-30"] },
    { name: "point", plane: "nearest", depth: 1, pick: ["subject:31-32"], loop: { move: "flare", seconds: 2.8 } },
  ],
};
