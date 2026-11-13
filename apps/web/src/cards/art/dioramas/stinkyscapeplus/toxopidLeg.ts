// R3350000: Stinkyscape+ #121.1 Toxopid Leg (`stinkyscapeplus-121-1`)'s diorama: the nebulae drifting at the back, the bright
// stars twinkling one after another, the girders up both edges, the machine's plated body, its domed head and antenna
// floating above it, the gears turning one after another in the margins, and the conveyor along the foot in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-121-1",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-3"], loop: { move: "drift", seconds: 12.2 } },
    { name: "bright stars", plane: "far", depth: 0.24, pick: ["sky:39-41"], loop: { move: "twinkle", seconds: 5.1, stagger: true } },
    { name: "girders", plane: "middle", depth: 0.4, pick: ["props:0-23"] },
    { name: "body", plane: "near", depth: 0.6, pick: ["subject:0-4"] },
    { name: "head", plane: "near", depth: 0.72, pick: ["subject:5-13"], loop: { move: "float", seconds: 4.8 } },
    { name: "gears", plane: "nearest", depth: 0.88, pick: ["props:24-29"], loop: { move: "spin", seconds: 9.4, stagger: true } },
    { name: "conveyor", plane: "nearest", depth: 1, pick: ["props:30-45"] },
  ],
};
