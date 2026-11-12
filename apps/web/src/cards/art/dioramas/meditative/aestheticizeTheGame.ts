// R3350000: Meditative #20 Aestheticize the Game (`meditative-020`)'s diorama: the ink range at the back, the battlefield
// before it bent like the painted sheet it becomes, its champion swaying with his sword up, the light ribbon fluttering off
// the painter's brush, the painter, the light pulsing and the petals falling nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-020",
  slices: [
    { name: "paper sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "ink range", plane: "far", depth: 0.16, pick: ["scene:far:"] },
    { name: "battlefield", plane: "middle", depth: 0.34, pick: ["scene:mid:mist:", "scene:mid:tombstone:", "scene:mid:fruit-tree:", "scene:mid:soldier:"], bend: { axis: "y", deg: 10 } },
    { name: "champion", plane: "middle", depth: 0.46, pick: ["scene:mid:pose-raise:", "scene:mid:outfit-armor:", "scene:mid:head-human:", "scene:mid:steel-helmet:", "scene:mid:item-sword:"], loop: { move: "sway", seconds: 6.9, origin: [56, 70] } },
    { name: "light ribbon", plane: "near", depth: 0.6, pick: ["scene:subject:light-ribbon:"], loop: { move: "flutter", seconds: 7.3 } },
    { name: "painter", plane: "near", depth: 0.72, pick: ["scene:subject:pose-cast:", "scene:subject:outfit-robe:", "scene:subject:head-human:", "scene:subject:hair-bun:", "scene:subject:item-ink-brush:", "emblem", "scene:near:"] },
    { name: "sunlight", plane: "near", depth: 0.86, pick: ["scene:light:"], loop: { move: "pulse", seconds: 5.4 } },
    { name: "falling petals", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 6.1, stagger: true } },
  ],
};
