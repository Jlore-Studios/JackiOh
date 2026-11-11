// R3350000: Meditative #97 Jlockeed's Evil Blueprints (`meditative-097`)'s diorama: the cave at the back, the four lairs'
// plans floating before it, the mastermind, his blueprint brought forward before him and bent like the sheet it is, the
// guards and their lanterns, the red light pulsing and the motes rising nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-097",
  slices: [
    { name: "black sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "cave", plane: "far", depth: 0.12, pick: ["scene:far:"] },
    { name: "lair plans", plane: "middle", depth: 0.3, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.3 } },
    { name: "mastermind", plane: "middle", depth: 0.46, pick: ["scene:subject:pose-hold:", "scene:subject:outfit-suit:", "scene:subject:head-void:", "scene:subject:visor:"] },
    { name: "blueprint", plane: "near", depth: 0.6, pick: ["scene:subject:blueprint:", "emblem"], bend: { axis: "x", deg: 14 } },
    { name: "guards", plane: "near", depth: 0.76, pick: ["scene:near:"] },
    { name: "red light", plane: "near", depth: 0.9, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 4.4, stagger: true } },
  ],
};
