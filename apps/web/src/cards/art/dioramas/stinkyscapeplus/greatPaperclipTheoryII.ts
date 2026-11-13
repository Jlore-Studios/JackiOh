// R3350000: Stinkyscape+ #132 Great Paperclip Theory II (`stinkyscapeplus-132`)'s diorama: the stars twinkling one after
// another at the back, the gantry bent round them, the reactor and the crane, the drill bobbing at its work, the mech that
// runs it, the cables, ore and light, and the sparks glinting one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-132",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "stars", plane: "far", depth: 0.14, pick: ["scene:sky:stars:"], loop: { move: "twinkle", seconds: 5.3, stagger: true } },
    { name: "gantry", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: 11 } },
    { name: "reactor and crane", plane: "middle", depth: 0.46, pick: ["scene:mid:"] },
    { name: "drill", plane: "near", depth: 0.6, pick: ["scene:subject:mining-drill:"], loop: { move: "bob", seconds: 3.4 } },
    { name: "mech", plane: "near", depth: 0.74, pick: ["scene:subject:mech:", "emblem"] },
    { name: "cables and light", plane: "near", depth: 0.88, pick: ["scene:near:", "scene:light:"] },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "glint", seconds: 2.9, stagger: true } },
  ],
};
