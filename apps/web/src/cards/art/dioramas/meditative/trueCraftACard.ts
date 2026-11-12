// R3350000: Meditative #17 True Craft a Card (`meditative-017`)'s diorama: the round window at the back, the workshop before
// it with its gear and cog each turning, the card being crafted floating before the crafter, then the anvil, the taper and
// the light, and the forge's sparks and motes rising nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-017",
  slices: [
    { name: "night window", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "round window", plane: "far", depth: 0.12, pick: ["scene:far:"] },
    { name: "workshop", plane: "middle", depth: 0.28, pick: ["scene:mid:boards:", "scene:mid:pipes:", "scene:mid:table:", "scene:mid:astrolabe:"] },
    { name: "gears", plane: "middle", depth: 0.36, pick: ["scene:mid:gear:", "scene:mid:cog:"], loop: { move: "spin", seconds: 14.5, stagger: true } },
    { name: "the card", plane: "middle", depth: 0.48, pick: ["scene:subject:card:", "scene:subject:card-imp:", "scene:subject:squares:", "scene:subject:glitch:", "emblem"], loop: { move: "float", seconds: 4.9 } },
    { name: "crafter", plane: "near", depth: 0.62, pick: ["scene:subject:pose-cast:", "scene:subject:outfit-work-apron:", "scene:subject:head-human:", "scene:subject:goggles:", "scene:subject:item-ink-brush:"] },
    { name: "forge", plane: "near", depth: 0.78, pick: ["scene:near:", "scene:light:"] },
    { name: "forge sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 4.3, stagger: true } },
  ],
};
