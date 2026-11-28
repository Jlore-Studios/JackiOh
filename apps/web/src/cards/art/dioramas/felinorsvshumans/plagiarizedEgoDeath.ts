// R3350000: Felinors vs Humans #173 Plagiarized: Ego Death (`felinorsvshumans-173`)'s diorama: Ego Death's swarm photocopied in greys, its stars and nebula wheeling, the gate and the city behind, the interceptors mid-frame, the drained swarm and its eye, the cold light pulsing, the missiles and the red stamp up close, and the toner grain and motes drifting in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_PLAGIARIZED_EGO_DEATH: Diorama = {
  id: "felinorsvshumans-173",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 93 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -10 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.9 } },
    { name: "the copied swarm", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.8 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
  ],
};
