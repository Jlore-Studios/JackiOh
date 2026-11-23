// R3350000: Infinite Assault #120 Eternal Return (`infiniteassault-120`)'s diorama: the shuffled-back haze settling, the gate and the city behind, the interceptors mid-frame, the brood returning over, its cold light flaring, the missiles and streaks rushing past, and the sparks scattering in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_ETERNAL_RETURN: Diorama = {
  id: "infiniteassault-120",
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
