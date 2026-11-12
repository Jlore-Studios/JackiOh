// R3350000: Stinkyscape #4.4 Last Word (`stinkyscape-004-4`)'s diorama: the sky with its nearer stars twinkling at the back,
// the reaching hand, the streaks of the crown's fall streaming down, its light, the crown itself rocking as it turns, and its
// glint nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004-4",
  slices: [
    { name: "sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-2"] },
    { name: "near stars", plane: "far", depth: 0.26, pick: ["sky:25-48"], loop: { move: "twinkle", seconds: 7.4, stagger: true } },
    { name: "hand", plane: "middle", depth: 0.5, pick: ["subject:0"] },
    { name: "falling streaks", plane: "middle", depth: 0.64, pick: ["subject:1-3"], loop: { move: "fall", seconds: 2.6, stagger: true } },
    { name: "crown light", plane: "near", depth: 0.76, pick: ["subject:4-5"] },
    { name: "crown", plane: "near", depth: 0.9, pick: ["subject:6"], loop: { move: "flutter", seconds: 5.6 } },
    { name: "glint", plane: "nearest", depth: 1, pick: ["subject:7"], loop: { move: "glint", seconds: 3.2 } },
  ],
};
