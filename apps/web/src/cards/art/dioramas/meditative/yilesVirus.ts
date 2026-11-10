// R3350000: Meditative #49.2 Yile's Virus (`meditative-049-2`)'s diorama: the range at the back with its glitch twinkling,
// the cracked earth bent toward the viewer, the virus hopping on it, its zaps flaring one after another, and the ink
// onlookers nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-049-2",
  slices: [
    { name: "grey sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "range", plane: "far", depth: 0.15, pick: ["scene:far:ink-peaks:"] },
    { name: "glitch", plane: "far", depth: 0.25, pick: ["scene:far:glitch:"], loop: { move: "twinkle", seconds: 2.7, stagger: true } },
    { name: "cracked earth", plane: "middle", depth: 0.4, pick: ["scene:mid:"], bend: { axis: "x", deg: 12 } },
    { name: "virus", plane: "near", depth: 0.6, pick: ["scene:subject:glitch-critter:", "emblem"], loop: { move: "hop", seconds: 3.1 } },
    { name: "zaps", plane: "near", depth: 0.74, pick: ["scene:subject:zap:"], loop: { move: "flare", seconds: 1.9, stagger: true } },
    { name: "onlookers", plane: "nearest", depth: 1, pick: ["scene:near:"] },
  ],
};
