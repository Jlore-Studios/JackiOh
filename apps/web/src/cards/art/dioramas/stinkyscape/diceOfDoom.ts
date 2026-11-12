// R3350000: Stinkyscape #154 Dice of Doom (`stinkyscape-154`)'s diorama: drifting nebulae and twinkling stars at the back, the
// black velvet bent like a floor under the die, the die turning face by face (`tick`), and its glint nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-154",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-3"], loop: { move: "drift", seconds: 14.2 } },
    { name: "twinkling stars", plane: "far", depth: 0.26, pick: ["sky:20-35"], loop: { move: "twinkle", seconds: 5.2, stagger: true } },
    { name: "velvet", plane: "middle", depth: 0.5, pick: ["subject:0-6"], bend: { axis: "x", deg: 10 } },
    { name: "die", plane: "near", depth: 0.78, pick: ["subject:7-8"], loop: { move: "tick", seconds: 9.6 } },
    { name: "glint", plane: "nearest", depth: 1, pick: ["subject:9-10"], loop: { move: "glint", seconds: 3.4 } },
  ],
};
