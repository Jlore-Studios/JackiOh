// R3350000: Core #100 Ceaseless Void (`core-100`)'s diorama: the Void's giant outline cut into the stars at the back, breathing,
// its rocks and ruins before it, its crystals bent toward the viewer, and its sparks, motes and embers falling at every depth.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "core-100",
  slices: [
    { name: "stars", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "giant", plane: "far", depth: 0.12, pick: ["own:", "scene:subject:void-swirl:", "emblem"], loop: { move: "breathe", seconds: 9, origin: [50, 58] } },
    { name: "falling far", plane: "far", depth: 0.25, pick: ["scene:particles:sparks:0-3", "scene:particles:motes:0-4"], loop: { move: "fall", seconds: 7.5, stagger: true } },
    { name: "rocks", plane: "middle", depth: 0.45, pick: ["scene:far:planet:", "scene:far:tower:", "scene:far:asteroid:", "scene:mid:"] },
    { name: "falling middle", plane: "middle", depth: 0.6, pick: ["scene:particles:sparks:4-7", "scene:particles:motes:5-9"], loop: { move: "fall", seconds: 5.5, stagger: true } },
    { name: "crystals", plane: "near", depth: 0.78, pick: ["scene:near:crystal:", "scene:light:"], bend: { axis: "y", deg: 12 } },
    { name: "falling near", plane: "nearest", depth: 1, pick: ["scene:particles:sparks:8-11", "scene:particles:motes:10-14", "scene:particles:embers:"], loop: { move: "fall", seconds: 4, stagger: true } },
  ],
};
