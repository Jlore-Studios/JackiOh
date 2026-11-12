// R3350000: Meditative+ #26 Scratch Pad (`meditativeplus-026`)'s diorama: the faded peaks at the back, the scratch pad bent
// toward the viewer, its cursor hopping as it clicks, the sketcher kneeling before it, the plum branch swaying from its
// corner, the pine and the halo, and the near flock floating nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditativeplus-026",
  slices: [
    { name: "white sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "faded peaks", plane: "far", depth: 0.18, pick: ["scene:far:", "scene:mid:"] },
    { name: "scratch pad", plane: "middle", depth: 0.4, pick: ["scene:subject:paper-sheet:", "scene:subject:card-frame:", "emblem"], bend: { axis: "x", deg: 12 } },
    { name: "cursor", plane: "middle", depth: 0.5, pick: ["scene:subject:cursor:"], loop: { move: "hop", seconds: 2.2 } },
    { name: "sketcher", plane: "near", depth: 0.64, pick: ["scene:subject:pose-kneel:", "scene:subject:outfit-jacket:", "scene:subject:head-human:", "scene:subject:hair-short:", "scene:subject:item-brush:"] },
    { name: "plum branch", plane: "near", depth: 0.76, pick: ["scene:near:plum-branch:"], loop: { move: "sway", seconds: 6.6, origin: [100, 16] } },
    { name: "pine and halo", plane: "near", depth: 0.88, pick: ["scene:near:ink-pine:", "scene:near:seal:", "scene:light:"] },
    { name: "near flock", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "float", seconds: 3.9 } },
  ],
};
