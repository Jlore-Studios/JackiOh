// R3350000: Felinors vs Humans #112.2 Durability Module (`felinorsvshumans-112-2`)'s diorama: the shop lights and stars wheeling, the shelf and robot arm behind, the workbench and cables mid-frame, the steel sentry module standing guard on the bench, its light pulsing, the crate and hammer up close, and the sparks and motes storming in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_DURABILITY_MODULE: Diorama = {
  id: "felinorsvshumans-112-2",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 81 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -10 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.9 } },
    { name: "the module", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.8 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
  ],
};
