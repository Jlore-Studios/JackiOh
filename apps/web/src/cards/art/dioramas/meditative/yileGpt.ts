// R3350000: Meditative #49 YileGPT (`meditative-049`)'s diorama: the glitching range at the back, the pavilion and its
// lanterns before it, the hooded scribe breathing over its brush, its desk and scroll bent toward the viewer, the ink imp
// hopping beside it, the light pulsing and the glitch motes twinkling one after another nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-049",
  slices: [
    { name: "night", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "range", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "pavilion", plane: "middle", depth: 0.32, pick: ["scene:mid:"] },
    { name: "scribe", plane: "near", depth: 0.5, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.2, origin: [46, 84] } },
    { name: "desk", plane: "near", depth: 0.64, pick: ["scene:near:desk:", "scene:near:scroll:", "scene:near:ink-seal:"], bend: { axis: "x", deg: 10 } },
    { name: "ink imp", plane: "near", depth: 0.76, pick: ["scene:near:ink-imp:", "scene:near:streaks:"], loop: { move: "hop", seconds: 2.9 } },
    { name: "light", plane: "near", depth: 0.88, pick: ["scene:light:"], loop: { move: "pulse", seconds: 5.1 } },
    { name: "glitch motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 3.3, stagger: true } },
  ],
};
