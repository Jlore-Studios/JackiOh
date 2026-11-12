// R3350000: Stinkyscape token Drone Battleship (Shield Layer) (`stinkyscape-t-shield-layer`)'s diorama: drifting nebulae, a
// twinkling star field and the ringed planet at the back, the faint keel and hangar decks in two ranks, and the bridge deck
// nearest, banking and floating.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-t-shield-layer",
  slices: [
    { name: "deep space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.1, pick: ["sky:1-3"], loop: { move: "drift", seconds: 18.4 } },
    { name: "star field", plane: "far", depth: 0.2, pick: ["sky:4-21"], loop: { move: "twinkle", seconds: 6.6, stagger: true } },
    { name: "ringed planet", plane: "far", depth: 0.32, pick: ["sky:42-44"] },
    { name: "keel deck", plane: "middle", depth: 0.48, pick: ["subject:0"] },
    { name: "hangar deck", plane: "middle", depth: 0.62, pick: ["subject:1"] },
    { name: "bridge deck", plane: "nearest", depth: 1, pick: ["subject:2"], bend: { axis: "y", deg: 8 }, loop: { move: "float", seconds: 6.2 } },
  ],
};
