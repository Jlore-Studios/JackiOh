// R3350000: Meditative #71.1 The Cane (`meditative-071-1`)'s diorama: the blackboard wall and the floor each bent, the desk,
// the cane swinging over it, its zaps flickering one after another, its sparks glinting, and the seal nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-071-1",
  slices: [
    { name: "classroom", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "blackboard wall", plane: "far", depth: 0.15, pick: ["scene:far:"], bend: { axis: "y", deg: -9 } },
    { name: "floor", plane: "middle", depth: 0.32, pick: ["scene:mid:boards:"], bend: { axis: "x", deg: 12 } },
    { name: "desk", plane: "middle", depth: 0.48, pick: ["scene:subject:desk:", "emblem"] },
    { name: "cane", plane: "near", depth: 0.66, pick: ["scene:subject:cane:"], loop: { move: "flutter", seconds: 2.4 } },
    { name: "zaps", plane: "near", depth: 0.8, pick: ["scene:subject:zap:"], loop: { move: "pulse", seconds: 1.3, stagger: true } },
    { name: "sparks", plane: "nearest", depth: 0.9, pick: ["scene:near:sparks:"], loop: { move: "glint", seconds: 2.6, stagger: true } },
    { name: "seal", plane: "nearest", depth: 1, pick: ["scene:near:ink-seal:"] },
  ],
};
