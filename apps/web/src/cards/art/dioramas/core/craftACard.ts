// R3350000: Core #99 Craft a Card (`core-099`)'s diorama: the far cogs turning each in place against the stars, the floating island
// bent like a floor, the anvil with its creature-cards and the hybrid beast, the hammer lifting and striking, the craftsman with
// the fresh card in his palm, the forge's white light flaring, and the sparks and steam rising at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "core-099",
  slices: [
    { name: "night workshop", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "cogs", plane: "far", depth: 0.12, pick: ["scene:far:cog:"], loop: { move: "spin", seconds: 13, stagger: true } },
    { name: "island", plane: "middle", depth: 0.26, pick: ["scene:mid:"], bend: { axis: "x", deg: 9 } },
    { name: "forge", plane: "middle", depth: 0.42, pick: ["scene:subject:anvil:", "scene:subject:card-imp:", "scene:subject:beast:", "emblem"] },
    { name: "hammer", plane: "near", depth: 0.55, pick: ["scene:subject:hammer:"], loop: { move: "hop", seconds: 2.6 } },
    { name: "craftsman", plane: "near", depth: 0.7, pick: ["scene:subject:pose-block:", "scene:subject:outfit-work-apron:", "scene:subject:head-human:", "scene:subject:bucket-hat:", "scene:near:"] },
    { name: "white light", plane: "nearest", depth: 0.85, pick: ["scene:light:"], loop: { move: "flare", seconds: 4.2 } },
    { name: "sparks and steam", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 4.6, stagger: true } },
  ],
};
