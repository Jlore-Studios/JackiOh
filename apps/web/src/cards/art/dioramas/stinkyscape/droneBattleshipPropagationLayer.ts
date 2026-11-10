// R3350000: Stinkyscape token Drone Battleship (Propagation Layer) (`stinkyscape-t-propagation-layer`)'s diorama: swelling
// nebulae, twinkling stars and the planet at the back, the faint keel and bridge decks behind, and the hangar deck nearest,
// banking and floating.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-t-propagation-layer",
  slices: [
    { name: "deep space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-2"], loop: { move: "swell", seconds: 14.8 } },
    { name: "twinkling stars", plane: "far", depth: 0.24, pick: ["sky:19-36"], loop: { move: "twinkle", seconds: 5.4, stagger: true } },
    { name: "planet", plane: "far", depth: 0.34, pick: ["sky:37"] },
    { name: "keel and bridge", plane: "middle", depth: 0.52, pick: ["subject:0-1"] },
    { name: "hangar deck", plane: "nearest", depth: 1, pick: ["subject:2"], bend: { axis: "y", deg: -8 }, loop: { move: "float", seconds: 7.2 } },
  ],
};
