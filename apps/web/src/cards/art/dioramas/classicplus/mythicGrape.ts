// R3350000: Classic+ #65.5 Mythic Grape (`classicplus-065-5`)'s diorama: the far vines behind the fence, the fence bent like a
// wall, the near vines swaying from their roots, the altar, the void spinning behind the grape, the grape floating in its halo,
// and its sparks twinkling at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "classicplus-065-5",
  slices: [
    { name: "night", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "back vines", plane: "far", depth: 0.14, pick: ["scene:far:vine:0-29"] },
    { name: "fence", plane: "middle", depth: 0.28, pick: ["scene:far:fence:"], bend: { axis: "y", deg: 10 } },
    { name: "front vines", plane: "middle", depth: 0.42, pick: ["scene:far:vine:30-54"], loop: { move: "sway", seconds: 7.6, origin: [50, 70] } },
    { name: "altar", plane: "near", depth: 0.56, pick: ["scene:mid:altar:"] },
    { name: "void", plane: "near", depth: 0.7, pick: ["scene:subject:void-swirl:"], loop: { move: "spin", seconds: 12.5 } },
    { name: "grape", plane: "nearest", depth: 0.85, pick: ["scene:subject:light-ribbon:", "scene:subject:orb:", "scene:light:", "emblem"], loop: { move: "float", seconds: 4.9 } },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 3.4, stagger: true } },
  ],
};
