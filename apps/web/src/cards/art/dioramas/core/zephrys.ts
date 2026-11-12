// R3350000: Core #97 Zephrys (`core-097`)'s diorama: the mesas and the rainbow at the back, the sand bent like a floor, the gusts
// circling each about its own curl in a slice of their own, the cyclone swaying from its foot, Zephrys breathing in his card
// rings, the hand and its card before him, and the sparks twinkling and the loose cards fluttering at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "core-097",
  slices: [
    { name: "desert sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sand", plane: "middle", depth: 0.2, pick: ["scene:mid:sand:", "scene:mid:streaks:"], bend: { axis: "x", deg: 10 } },
    { name: "gusts", plane: "middle", depth: 0.32, pick: ["scene:mid:gust:"], loop: { move: "spin", seconds: 6.2, stagger: true } },
    { name: "cyclone", plane: "middle", depth: 0.44, pick: ["scene:subject:cyclone:"], loop: { move: "sway", seconds: 4.8, origin: [50, 82] } },
    { name: "zephrys", plane: "near", depth: 0.58, pick: ["scene:subject:card-ring:", "scene:subject:pose-hold:", "scene:subject:outfit-gown:", "scene:subject:head-cloud:", "emblem"], loop: { move: "breathe", seconds: 7.4, origin: [50, 64] } },
    { name: "hand and card", plane: "near", depth: 0.74, pick: ["scene:near:", "scene:light:"] },
    { name: "sparks", plane: "nearest", depth: 0.88, pick: ["scene:particles:sparks:", "scene:particles:motes:"], loop: { move: "twinkle", seconds: 3.6, stagger: true } },
    { name: "whirling cards", plane: "nearest", depth: 1, pick: ["scene:particles:playing-card:"], loop: { move: "flutter", seconds: 4.4, stagger: true } },
  ],
};
