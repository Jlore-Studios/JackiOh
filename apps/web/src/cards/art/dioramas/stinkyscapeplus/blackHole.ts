// R3350000: Stinkyscape+ #77 Black Hole (`stinkyscapeplus-077`)'s diorama: the galaxy turning slowly at the back, the rift bent
// beside it, the probe and the wreckage floating toward it, the hole breathing, its warp flash and lens light flaring, the
// ice shards and comets, and the motes twinkling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-077",
  slices: [
    { name: "deep space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "galaxy", plane: "far", depth: 0.12, pick: ["scene:sky:galaxy:"], loop: { move: "spin", seconds: 52 } },
    { name: "rift", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -12 } },
    { name: "probe and wreckage", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 8.3 } },
    { name: "the hole", plane: "near", depth: 0.62, pick: ["scene:subject:wormhole:", "emblem"], loop: { move: "breathe", seconds: 9.1 } },
    { name: "warp flash", plane: "near", depth: 0.76, pick: ["scene:subject:warp-flash:", "scene:light:"], loop: { move: "flare", seconds: 3.2 } },
    { name: "shards and comets", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.4, stagger: true } },
  ],
};
