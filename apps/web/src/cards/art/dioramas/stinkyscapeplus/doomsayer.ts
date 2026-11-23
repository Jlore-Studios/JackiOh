// R3350000: Stinkyscape+ #81 Doomsayer (`stinkyscapeplus-081`)'s diorama: the galaxy turning at the back, the warp gate
// pulsing, the satellite and the freighter bent across the middle, the starship, its thrusters flaring behind it, the ore
// and streaks and light, and the comets falling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscapeplus-081",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "galaxy", plane: "far", depth: 0.12, pick: ["scene:sky:galaxy:"], loop: { move: "spin", seconds: 58 } },
    { name: "warp gate", plane: "far", depth: 0.28, pick: ["scene:far:"], loop: { move: "pulse", seconds: 5.7 } },
    { name: "satellite and freighter", plane: "middle", depth: 0.46, pick: ["scene:mid:"], bend: { axis: "y", deg: 9 } },
    { name: "starship", plane: "near", depth: 0.64, pick: ["scene:subject:starship:", "emblem"] },
    { name: "thrusters", plane: "near", depth: 0.76, pick: ["scene:subject:thruster-flame:"], loop: { move: "flare", seconds: 1.9 } },
    { name: "ore and streaks", plane: "nearest", depth: 0.9, pick: ["scene:near:", "scene:light:"] },
    { name: "comets", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 4.9, stagger: true } },
  ],
};
