// R3350000: Classic+ #27 Zephrys Zealotism (`classicplus-027`)'s diorama: the temple under the storm at the back, the tornado and
// its cards swaying from its foot, the kneeling zealot, the prophet's cards floating before the hooded prophet, the rune circle
// bent like a floor, the light pulsing, and the sparks and embers rising at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "classicplus-027",
  slices: [
    { name: "storm", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "tornado", plane: "middle", depth: 0.24, pick: ["scene:mid:tornado:", "scene:mid:card:"], loop: { move: "sway", seconds: 5.6, origin: [70, 76] } },
    { name: "kneeling zealot", plane: "middle", depth: 0.38, pick: ["scene:mid:pose-kneel:", "scene:mid:outfit-tunic:", "scene:mid:head-human:", "scene:mid:hair-short:"] },
    { name: "cards", plane: "near", depth: 0.52, pick: ["scene:subject:card:"], loop: { move: "float", seconds: 4.4 } },
    { name: "prophet", plane: "near", depth: 0.66, pick: ["scene:subject:pose-raise:", "scene:subject:outfit-robe:", "scene:subject:head-human:", "scene:subject:hood:", "emblem"] },
    { name: "rune circle", plane: "nearest", depth: 0.8, pick: ["scene:near:"], bend: { axis: "x", deg: 10 } },
    { name: "light", plane: "nearest", depth: 0.9, pick: ["scene:light:"], loop: { move: "pulse", seconds: 6.9 } },
    { name: "embers", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 5.3, stagger: true } },
  ],
};
