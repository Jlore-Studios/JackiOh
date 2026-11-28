// R3350000: Felinors vs Humans #194 Plagiarized: Model Collapse (`felinorsvshumans-194`)'s diorama: Model Collapse's drone bay photocopied in greys, its ceiling lights and alarm wheeling, the racks behind, the arms and drones mid-frame, the copied machine stepping forward, the grey light pulsing, the cables, wreck and red stamp up close, and the toner grain and motes drifting in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_PLAGIARIZED_MODEL_COLLAPSE: Diorama = {
  id: "felinorsvshumans-194",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 95 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -10 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.9 } },
    { name: "the copied machine", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.8 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
  ],
};
