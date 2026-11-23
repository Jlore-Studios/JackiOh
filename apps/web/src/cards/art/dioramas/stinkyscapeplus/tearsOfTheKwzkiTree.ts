// R3350000: Stinkyscape+ token Tears of the Kwzki Tree (`stinkyscapeplus-t-tears-of-the-kwzki-tree`)'s diorama: the nebulae at
// the back, the bright stars twinkling one after another, the girders bent up both edges, the dying star's glow and the far
// half of its disk pulsing, the hole and the near half of its disk, the gears turning one after another, and the conveyor
// in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-t-tears-of-the-kwzki-tree",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.1, pick: ["sky:1-3"] },
    { name: "bright stars", plane: "far", depth: 0.2, pick: ["sky:46-47"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
    { name: "girders", plane: "middle", depth: 0.34, pick: ["props:0-35"], bend: { axis: "y", deg: 6 } },
    { name: "glow and far disk", plane: "middle", depth: 0.5, pick: ["subject:0-2"], loop: { move: "pulse", seconds: 6.3 } },
    { name: "the hole", plane: "near", depth: 0.68, pick: ["subject:3-6"] },
    { name: "gears", plane: "near", depth: 0.84, pick: ["props:36-41"], loop: { move: "spin", seconds: 11.3, stagger: true } },
    { name: "conveyor", plane: "nearest", depth: 1, pick: ["props:42-59"] },
  ],
};
