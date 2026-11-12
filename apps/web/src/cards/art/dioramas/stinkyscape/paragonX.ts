// R3350000: Stinkyscape #74 Paragon-X (`stinkyscape-074`)'s diorama: the moon at the back, the wrecks and their smoke, the
// blade fighter bent and banking as it rises, its hex shield pulsing about it, the beams and near debris, and the embers
// rising and the sparks twinkling one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-074",
  slices: [
    { name: "stars", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "moon", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "wrecks", plane: "middle", depth: 0.34, pick: ["scene:mid:"] },
    { name: "blade fighter", plane: "near", depth: 0.56, pick: ["scene:subject:blade-fighter:", "emblem"], bend: { axis: "y", deg: 14 }, loop: { move: "sway", seconds: 6.4, origin: [50, 46] } },
    { name: "hex shield", plane: "near", depth: 0.68, pick: ["scene:subject:hex-shield:"], loop: { move: "pulse", seconds: 3.8 } },
    { name: "beams", plane: "near", depth: 0.8, pick: ["scene:near:", "scene:light:"] },
    { name: "embers", plane: "nearest", depth: 0.9, pick: ["scene:particles:embers:"], loop: { move: "rise", seconds: 5.2, stagger: true } },
    { name: "sparks", plane: "nearest", depth: 1, pick: ["scene:particles:sparks:", "scene:particles:motes:"], loop: { move: "twinkle", seconds: 4.6, stagger: true } },
  ],
};
