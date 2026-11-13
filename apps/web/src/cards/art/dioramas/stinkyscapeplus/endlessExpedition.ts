// R3350000: Stinkyscape+ #82 Endless Expedition (`stinkyscapeplus-082`)'s diorama: the stars twinkling one after another at
// the back, the warp gate bent before them, the freighter drifting, the satellite, the starship floating on, the speed
// streaks and ore, and the comets and light rising one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-082",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "stars", plane: "far", depth: 0.12, pick: ["scene:sky:stars:"], loop: { move: "twinkle", seconds: 7.4, stagger: true } },
    { name: "warp gate", plane: "middle", depth: 0.3, pick: ["scene:far:"], bend: { axis: "x", deg: 11 } },
    { name: "freighter", plane: "middle", depth: 0.46, pick: ["scene:mid:freighter:"], loop: { move: "drift", seconds: 12.8 } },
    { name: "satellite", plane: "middle", depth: 0.56, pick: ["scene:mid:satellite:"] },
    { name: "starship", plane: "near", depth: 0.74, pick: ["scene:subject:", "emblem"], loop: { move: "float", seconds: 4.3 } },
    { name: "streaks and ore", plane: "nearest", depth: 0.88, pick: ["scene:near:"] },
    { name: "comets and light", plane: "nearest", depth: 1, pick: ["scene:particles:", "scene:light:"], loop: { move: "rise", seconds: 4.6, stagger: true } },
  ],
};
