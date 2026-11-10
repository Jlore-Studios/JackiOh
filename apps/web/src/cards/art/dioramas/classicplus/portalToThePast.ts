// R3350000: Classic+ #29 Portal to the Past (`classicplus-029`)'s diorama: the hall's columns and the stairs at the back, the
// portal's vortex spinning behind the ruins and the two soldiers of the past, the portal ring before them, the flood and its
// pale hand bent like a floor, the light pulsing, and the hourglass, crystals and motes falling at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "classicplus-029",
  slices: [
    { name: "hall", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "columns and stairs", plane: "far", depth: 0.14, pick: ["scene:far:", "scene:mid:stairs:"] },
    { name: "vortex", plane: "middle", depth: 0.28, pick: ["scene:mid:void-swirl:"], loop: { move: "spin", seconds: 18 } },
    { name: "the past", plane: "middle", depth: 0.42, pick: ["scene:mid:ruin-wall:", "scene:mid:fallen-column:", "scene:mid:banner:", "scene:mid:pose-spread:", "scene:mid:outfit-armor:", "scene:mid:head-human:", "scene:mid:helm:", "scene:mid:item-spear:", "scene:mid:pose-strike:", "scene:mid:outfit-tabard:", "scene:mid:hair-short:", "scene:mid:item-sword:"] },
    { name: "portal", plane: "near", depth: 0.58, pick: ["scene:subject:portal-ring:", "emblem"] },
    { name: "flood", plane: "near", depth: 0.72, pick: ["scene:near:"], bend: { axis: "x", deg: 8 } },
    { name: "light", plane: "nearest", depth: 0.86, pick: ["scene:light:"], loop: { move: "pulse", seconds: 7.2 } },
    { name: "sands of time", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 6.6, stagger: true } },
  ],
};
