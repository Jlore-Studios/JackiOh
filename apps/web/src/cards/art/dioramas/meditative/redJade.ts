// R3350000: Meditative #39.4 Red Jade (`meditative-039-4`)'s diorama: the cave at the back, its floor bent toward the viewer,
// the red jade breathing in its halo, its crystals twinkling one after another, the lantern swaying from its hook, the
// sparks rising and the seal nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-039-4",
  slices: [
    { name: "cave dark", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "cave", plane: "far", depth: 0.15, pick: ["scene:far:cave:"] },
    { name: "floor", plane: "middle", depth: 0.32, pick: ["scene:mid:"], bend: { axis: "x", deg: 12 } },
    { name: "red jade", plane: "middle", depth: 0.5, pick: ["scene:subject:halo:", "scene:subject:jade-boulder:", "emblem"], loop: { move: "breathe", seconds: 7.1, origin: [50, 74] } },
    { name: "crystals", plane: "near", depth: 0.64, pick: ["scene:subject:crystal:"], loop: { move: "twinkle", seconds: 3.9, stagger: true } },
    { name: "lantern", plane: "near", depth: 0.78, pick: ["scene:near:lantern:"], loop: { move: "sway", seconds: 4.7, origin: [72, 37] } },
    { name: "sparks", plane: "nearest", depth: 0.9, pick: ["scene:near:sparks:"], loop: { move: "rise", seconds: 3.3, stagger: true } },
    { name: "seal", plane: "nearest", depth: 1, pick: ["scene:near:ink-seal:"] },
  ],
};
