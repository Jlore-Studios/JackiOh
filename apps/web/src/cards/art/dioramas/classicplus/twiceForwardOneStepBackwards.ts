// R3350000: Classic+ #74 Twice Forward One Step Backwards (`classicplus-074`)'s diorama: the clock in the void at the back, its
// far gears ticking each in place, the stairs bent like a floor, the brass gears spinning, the standing stone with its bolt and
// fireball, the four soldiers stepping up the stairs, the near gear and the light, and the sparks and embers rising.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "classicplus-074",
  slices: [
    { name: "void", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "far gears", plane: "far", depth: 0.12, pick: ["scene:far:gear:"], loop: { move: "tick", seconds: 9.6, stagger: true } },
    { name: "stairs", plane: "middle", depth: 0.26, pick: ["scene:mid:stairs:"], bend: { axis: "x", deg: 10 } },
    { name: "brass gears", plane: "middle", depth: 0.38, pick: ["scene:mid:gear:"], loop: { move: "spin", seconds: 7.2, stagger: true } },
    { name: "standing stone", plane: "near", depth: 0.52, pick: ["scene:subject:standing-stone:", "scene:subject:lightning:", "scene:subject:fireball:", "emblem"] },
    { name: "soldiers", plane: "near", depth: 0.66, pick: ["scene:subject:pose-run:", "scene:subject:outfit-armor:", "scene:subject:head-human:", "scene:subject:visor:", "scene:subject:item-spear:", "scene:subject:item-sword:", "scene:subject:pose-raise:", "scene:subject:hood:", "scene:subject:item-staff:"], loop: { move: "hop", seconds: 3.2 } },
    { name: "near gear", plane: "nearest", depth: 0.82, pick: ["scene:near:", "scene:light:"] },
    { name: "rising sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 4.4, stagger: true } },
  ],
};
