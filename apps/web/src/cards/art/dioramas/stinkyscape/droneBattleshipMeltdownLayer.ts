// R3350000: Stinkyscape token Drone Battleship (Meltdown Layer) (`stinkyscape-t-meltdown-layer`)'s diorama: drifting nebulae,
// the stars and the planet at the back, the faint hangar and bridge decks above flickering, and the keel deck nearest,
// banking and floating.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-t-meltdown-layer",
  slices: [
    { name: "deep space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-3"], loop: { move: "drift", seconds: 11.8 } },
    { name: "stars", plane: "far", depth: 0.24, pick: ["sky:16-31"] },
    { name: "planet", plane: "far", depth: 0.34, pick: ["sky:32"] },
    { name: "upper decks", plane: "middle", depth: 0.5, pick: ["subject:0-1"], loop: { move: "pulse", seconds: 5.6 } },
    { name: "keel deck", plane: "nearest", depth: 1, pick: ["subject:2"], bend: { axis: "y", deg: -10 }, loop: { move: "float", seconds: 4.6 } },
  ],
};
