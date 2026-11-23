// R3350000: Stinkyscape+ #79 A Glimpse into the Endless Sea (`stinkyscapeplus-079`)'s diorama: the sea of stars twinkling one
// after another at the back, the galaxy wheeling very slowly, the rift bent as a far shore, the probe bobbing in the middle,
// the gate swelling open, the light and the shards before it, and the motes rising one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-079",
  slices: [
    { name: "the sea", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "stars", plane: "far", depth: 0.1, pick: ["scene:sky:stars:"], loop: { move: "twinkle", seconds: 6.7, stagger: true } },
    { name: "galaxy", plane: "far", depth: 0.22, pick: ["scene:sky:galaxy:"], loop: { move: "spin", seconds: 71 } },
    { name: "rift", plane: "middle", depth: 0.4, pick: ["scene:far:"], bend: { axis: "x", deg: 12 } },
    { name: "probe", plane: "middle", depth: 0.54, pick: ["scene:mid:"], loop: { move: "bob", seconds: 5.1 } },
    { name: "the gate", plane: "near", depth: 0.72, pick: ["scene:subject:", "emblem"], loop: { move: "swell", seconds: 9.7 } },
    { name: "light and shards", plane: "nearest", depth: 0.88, pick: ["scene:light:", "scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 5.3, stagger: true } },
  ],
};
