// R3350000: Stinkyscape #4.3 Event Horizon (`stinkyscape-004-3`)'s diorama: the lensed starlight circling slowly about the
// hole at the back, the stopped hours pulsing round it one after another, the hole and its photon ring, the bent ray before
// it, and the caught light flaring at its edge nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004-3",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-2"] },
    { name: "lensed light", plane: "far", depth: 0.26, pick: ["subject:0-3"], loop: { move: "spin", seconds: 48 } },
    { name: "stopped hours", plane: "middle", depth: 0.44, pick: ["subject:4-15"], loop: { move: "pulse", seconds: 6, stagger: true } },
    { name: "black hole", plane: "middle", depth: 0.6, pick: ["subject:16-18"] },
    { name: "bent ray", plane: "near", depth: 0.8, pick: ["subject:19-20"] },
    { name: "caught light", plane: "nearest", depth: 1, pick: ["subject:21-22"], loop: { move: "flare", seconds: 3.6 } },
  ],
};
