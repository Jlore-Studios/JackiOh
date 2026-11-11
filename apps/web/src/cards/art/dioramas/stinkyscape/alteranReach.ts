// R3350000: Stinkyscape #148 Alteran Reach (`stinkyscape-148`)'s diorama: the cliff and the rift bent at the back, the far
// islands bobbing, the nearer islands, the stations, the lone ship floating mid-jump, the foreground rocks and the ribbon of
// light, and the star dust along the path glinting one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-148",
  slices: [
    { name: "deep space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "cliff and rift", plane: "far", depth: 0.12, pick: ["scene:far:cliff:", "scene:far:space-rift:"], bend: { axis: "y", deg: 10 } },
    { name: "far islands", plane: "far", depth: 0.26, pick: ["scene:far:asteroid:"], loop: { move: "bob", seconds: 9.4 } },
    { name: "near islands", plane: "middle", depth: 0.42, pick: ["scene:mid:"] },
    { name: "stations", plane: "middle", depth: 0.56, pick: ["scene:subject:asteroid:", "scene:subject:tower:", "emblem"] },
    { name: "ship mid-jump", plane: "near", depth: 0.68, pick: ["scene:subject:shuttle:", "scene:subject:speed-streaks:"], loop: { move: "float", seconds: 4.2 } },
    { name: "rocks and ribbon", plane: "near", depth: 0.82, pick: ["scene:near:", "scene:light:"] },
    { name: "path lights", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "glint", seconds: 6.6, stagger: true } },
  ],
};
