// R3350000: Core #98 Heroic Power (`core-098`)'s diorama: the rotunda at the back, its doors' powers in two ranks (the deep middle
// doors, then the outer ones nearer), each rank glowing on its own beat, the hero on the pedestal, and its power (the crystal and
// the orb in its halo) floating forward before him, then the cobbles and columns bent like a floor and the sparks rising.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "core-098",
  slices: [
    { name: "sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "rotunda", plane: "far", depth: 0.14, pick: ["scene:far:rotunda:"] },
    { name: "inner doors", plane: "middle", depth: 0.28, pick: ["scene:mid:gate:", "scene:mid:house-cat:", "scene:mid:cup:", "scene:mid:flying-arrow:", "scene:mid:doll:", "scene:mid:shield:", "scene:mid:honeybee:", "scene:mid:flame:", "scene:mid:eye:"], loop: { move: "pulse", seconds: 6.2 } },
    { name: "outer doors", plane: "middle", depth: 0.4, pick: ["scene:mid:map:", "scene:mid:open-hand:", "scene:mid:drops:", "scene:mid:bow:", "scene:mid:blackboard:", "scene:mid:tree:", "scene:mid:hatch:"], loop: { move: "pulse", seconds: 7.8 } },
    { name: "hero", plane: "near", depth: 0.55, pick: ["scene:subject:pedestal:", "scene:subject:pose-lunge:", "scene:subject:outfit-layered-plate:", "scene:subject:head-human:", "scene:subject:steel-helmet:", "scene:subject:item-sword:", "emblem"] },
    { name: "power", plane: "near", depth: 0.68, pick: ["scene:subject:crystal:", "scene:subject:orb:", "scene:light:halo:"], loop: { move: "float", seconds: 5.2 } },
    { name: "columns", plane: "nearest", depth: 0.84, pick: ["scene:near:", "scene:light:god-rays:", "scene:light:rim-light:"], bend: { axis: "x", deg: 8 } },
    { name: "rising sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 6.8, stagger: true } },
  ],
};
