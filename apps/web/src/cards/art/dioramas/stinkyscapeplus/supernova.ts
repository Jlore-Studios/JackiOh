// R3350000: Stinkyscape+ token Supernova (`stinkyscapeplus-t-supernova`)'s diorama: the bright stars glinting one after another
// at the back, the nebulae drifting, the blast's glow and the far half of its disk flaring, the collapsed core and the near
// half of its disk, the girders bent up both edges, and the gears and the conveyor in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-t-supernova",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "bright stars", plane: "far", depth: 0.12, pick: ["sky:30-32"], loop: { move: "glint", seconds: 3.1, stagger: true } },
    { name: "nebulae", plane: "far", depth: 0.24, pick: ["sky:1-2"], loop: { move: "drift", seconds: 10.7 } },
    { name: "the blast", plane: "middle", depth: 0.44, pick: ["subject:0-2"], loop: { move: "flare", seconds: 2.7 } },
    { name: "collapsed core", plane: "near", depth: 0.62, pick: ["subject:3-6"] },
    { name: "girders", plane: "near", depth: 0.78, pick: ["props:0-23"], bend: { axis: "y", deg: -8 } },
    { name: "gears and conveyor", plane: "nearest", depth: 1, pick: ["props:24-43"] },
  ],
};
