// R3350000: Stinkyscape #4.1 Farewell (`stinkyscape-004-1`)'s diorama: the night and its stars twinkling at the back, the empty
// table under its pulsing pool of lamplight, the marks where the cards lay bent flat on its top, and the lamp hanging
// nearest, bobbing on its cord.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004-1",
  slices: [
    { name: "night", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.14, pick: ["sky:1-3"] },
    { name: "twinkling stars", plane: "far", depth: 0.27, pick: ["sky:22-39"], loop: { move: "twinkle", seconds: 6.2, stagger: true } },
    { name: "table", plane: "middle", depth: 0.5, pick: ["subject:0-6"] },
    { name: "lamplight", plane: "middle", depth: 0.56, pick: ["subject:7-9"], loop: { move: "pulse", seconds: 5.4 } },
    { name: "card marks", plane: "middle", depth: 0.62, pick: ["subject:10-13"], bend: { axis: "x", deg: 10 } },
    { name: "lamp", plane: "nearest", depth: 1, pick: ["subject:14-18"], loop: { move: "float", seconds: 7.4 } },
  ],
};
