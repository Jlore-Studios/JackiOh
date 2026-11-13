// R3350000: Stinkyscape+ token Alnilam (`stinkyscapeplus-t-alnilam`)'s diorama: the nebulae at the back, the bright stars
// twinkling one after another, the star's glow pulsing, its disk and core floating before it, the girders up both edges, the
// gears ticking round one after another, and the conveyor in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-t-alnilam",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.1, pick: ["sky:1-3"] },
    { name: "bright stars", plane: "far", depth: 0.22, pick: ["sky:43-46"], loop: { move: "twinkle", seconds: 6.1, stagger: true } },
    { name: "glow", plane: "middle", depth: 0.4, pick: ["subject:0-2"], loop: { move: "pulse", seconds: 7.7 } },
    { name: "disk and core", plane: "near", depth: 0.58, pick: ["subject:3-6"], loop: { move: "float", seconds: 5.6 } },
    { name: "girders", plane: "near", depth: 0.74, pick: ["props:0-31"] },
    { name: "gears", plane: "nearest", depth: 0.88, pick: ["props:32-40"], loop: { move: "tick", seconds: 12, stagger: true } },
    { name: "conveyor", plane: "nearest", depth: 1, pick: ["props:41-58"] },
  ],
};
