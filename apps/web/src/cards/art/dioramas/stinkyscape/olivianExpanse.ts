// R3350000: Stinkyscape #4 Olivian Expanse (`stinkyscape-004`)'s diorama: the violet sky and its drifting nebulae at the back,
// the six Terminus cards' trails pulsing and the cards twinkling one after another as distant stars, and the endless plain
// before them bent away toward its horizon.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-004",
  slices: [
    { name: "violet sky", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-2"], loop: { move: "drift", seconds: 16.2 } },
    { name: "card trails", plane: "far", depth: 0.26, pick: ["subject:22-24", "subject:26-28", "subject:30-32", "subject:34-36", "subject:38-40", "subject:42-44"], loop: { move: "pulse", seconds: 5.8, stagger: true } },
    { name: "falling cards", plane: "middle", depth: 0.34, pick: ["subject:25", "subject:29", "subject:33", "subject:37", "subject:41", "subject:45"], loop: { move: "twinkle", seconds: 4.4, stagger: true } },
    { name: "plain", plane: "near", depth: 0.8, pick: ["subject:0-21"], bend: { axis: "x", deg: 14 } },
  ],
};
