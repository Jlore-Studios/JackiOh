// R3350000: Infinite Assault #216 The Singularity (`infiniteassault-216`)'s diorama: the summoned countdown ticking, the gate and the city behind, the interceptors mid-frame, the waking brood turning over, its cold light flaring, the missiles and streaks rushing past, and the sparks scattering in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_THE_SINGULARITY: Diorama = {
  id: "infiniteassault-216",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 52 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -12 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 8.3 } },
    { name: "the brood", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 9.1 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "flare", seconds: 3.2 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.4, stagger: true } },
  ],
};
