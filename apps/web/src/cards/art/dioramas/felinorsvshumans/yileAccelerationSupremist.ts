// R3350000: Felinors vs Humans #74 Yile, Acceleration Supremist (`felinorsvshumans-074`)'s diorama: the town's stars wheeling, its houses and tower behind, the square's banner and stall mid-frame, Yile striding forward, the dusk glow pulsing, the barrels and crates up close, and the motes and sparks drifting in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_YILE_ACCELERATION_SUPREMIST: Diorama = {
  id: "felinorsvshumans-074",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 73 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -10 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.9 } },
    { name: "Yile", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.8 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
  ],
};
