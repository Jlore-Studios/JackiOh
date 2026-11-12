// R3350000: Stinkyscape #6 Tylannt (`stinkyscape-006`)'s diorama: the ring station and the vault door bent at the back, the
// credit chips floating before them, the thief on his skiff bobbing in the middle, the loot nearer, the red light pulsing and
// the drill's sparks and embers rising one after another in front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-006",
  slices: [
    { name: "stars", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "vault", plane: "far", depth: 0.14, pick: ["scene:far:ring-station:", "scene:far:vault-door:"], bend: { axis: "y", deg: 9 } },
    { name: "chips afloat", plane: "far", depth: 0.28, pick: ["scene:far:credit-chips:"], loop: { move: "float", seconds: 5.2 } },
    { name: "asteroids", plane: "middle", depth: 0.44, pick: ["scene:mid:"] },
    { name: "skiff and thief", plane: "middle", depth: 0.6, pick: ["scene:subject:", "emblem"], loop: { move: "bob", seconds: 6.6 } },
    { name: "loot", plane: "near", depth: 0.76, pick: ["scene:near:"] },
    { name: "red light", plane: "near", depth: 0.88, pick: ["scene:light:"], loop: { move: "pulse", seconds: 4.6 } },
    { name: "drill sparks", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "rise", seconds: 3.8, stagger: true } },
  ],
};
