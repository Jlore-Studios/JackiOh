// R3350000: Meditative #71 Pareto Optimality (`meditative-071`)'s diorama: the paper screens and the bell curve at the back,
// the back rows of desks before them, the front row, the cane swaying with its card, the front desks, the light pulsing and
// the sparks glinting one after another nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-071",
  slices: [
    { name: "classroom air", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "screens", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "back rows", plane: "middle", depth: 0.32, pick: ["scene:mid:"] },
    { name: "front row", plane: "middle", depth: 0.46, pick: ["scene:subject:desk:", "scene:subject:ink-person:"] },
    { name: "the cane", plane: "near", depth: 0.6, pick: ["scene:subject:cane:", "scene:subject:streaks:", "scene:subject:playing-card:", "emblem"], loop: { move: "sway", seconds: 4.4, origin: [56, 56] } },
    { name: "front desks", plane: "near", depth: 0.74, pick: ["scene:near:"] },
    { name: "light", plane: "near", depth: 0.88, pick: ["scene:light:"], loop: { move: "pulse", seconds: 5.8 } },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "glint", seconds: 3.4, stagger: true } },
  ],
};
