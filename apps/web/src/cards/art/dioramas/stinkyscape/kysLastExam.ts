// R3350000: Stinkyscape #4.2 KY's Last Exam (`stinkyscape-004-2`)'s diorama: the impossible problem glowing across the sky at
// the back, its chalk dust falling mote by mote, the hall's floor bent away to its vanishing point, the stopped clock, and the
// one desk nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004-2",
  slices: [
    { name: "exam hall", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-2"] },
    { name: "the problem", plane: "far", depth: 0.26, pick: ["subject:0-2"], loop: { move: "pulse", seconds: 7.2 } },
    { name: "chalk dust", plane: "middle", depth: 0.38, pick: ["subject:3-12"], loop: { move: "fall", seconds: 6.8, stagger: true } },
    { name: "floor", plane: "middle", depth: 0.55, pick: ["subject:13-25"], bend: { axis: "x", deg: 14 } },
    { name: "clock", plane: "near", depth: 0.74, pick: ["subject:26-41"] },
    { name: "desk", plane: "nearest", depth: 1, pick: ["subject:42"] },
  ],
};
