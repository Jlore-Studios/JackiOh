// R3350000: Meditative+ #1 Bob's Plan (`meditativeplus-001`)'s diorama: the visions of the plan fading in and out of the dawn
// sky, the summit and its pines, the flag swaying on its top, the plan's table bent toward the viewer, Bob over it, the cliff
// edge and the light, and the embers and motes rising nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditativeplus-001",
  slices: [
    { name: "dawn sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "visions", plane: "far", depth: 0.1, pick: ["scene:sky:camp-tent:", "scene:sky:star-map:", "scene:sky:nebula:", "scene:sky:towels:", "scene:sky:weights:"], loop: { move: "pulse", seconds: 7.2 } },
    { name: "summit", plane: "far", depth: 0.24, pick: ["scene:far:peaks:", "scene:far:summit:", "scene:mid:"] },
    { name: "flag", plane: "middle", depth: 0.32, pick: ["scene:far:flag:"], loop: { move: "sway", seconds: 3.4, origin: [50, 30] } },
    { name: "plan table", plane: "middle", depth: 0.48, pick: ["scene:subject:table:", "scene:subject:scroll:", "scene:subject:brush-stroke:", "emblem"], bend: { axis: "x", deg: 12 } },
    { name: "bob", plane: "near", depth: 0.62, pick: ["scene:subject:pose-strike:", "scene:subject:outfit-patched:", "scene:subject:head-human:", "scene:subject:hair-short:"] },
    { name: "cliff edge", plane: "near", depth: 0.78, pick: ["scene:near:", "scene:light:"] },
    { name: "embers", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 5.6, stagger: true } },
  ],
};
