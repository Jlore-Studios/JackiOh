// R3350000: Stinkyscape+ token The Enigmata (`stinkyscapeplus-t-the-enigmata`)'s diorama: the nebulae drifting at the back, the
// left and the right girders bent toward each other, the enigma, its hole and its disk turning slowly in the middle, the
// gears turning one after another, and the conveyor in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-t-the-enigmata",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.16, pick: ["sky:1-3"], loop: { move: "drift", seconds: 15.3 } },
    { name: "left girder", plane: "middle", depth: 0.32, pick: ["props:0-15"], bend: { axis: "y", deg: 10 } },
    { name: "right girder", plane: "middle", depth: 0.4, pick: ["props:16-31"], bend: { axis: "y", deg: -10 } },
    { name: "the enigma", plane: "near", depth: 0.6, pick: ["subject:"], loop: { move: "spin", seconds: 34 } },
    { name: "gears", plane: "near", depth: 0.8, pick: ["props:32-40"], loop: { move: "spin", seconds: 7.6, stagger: true } },
    { name: "conveyor", plane: "nearest", depth: 1, pick: ["props:41-54"] },
  ],
};
