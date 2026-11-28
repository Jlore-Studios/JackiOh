// R3350000: Felinors vs Humans #107 KY, Himself (`felinorsvshumans-107`)'s diorama: the pocket galaxy wheeling, the blackboard and bookshelf behind, the floorboards and desk mid-frame, KY himself at the board, chalk in hand, the lamplight pulsing, the pages and candle up close, and the chalk dust and motes drifting in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_KY_HIMSELF: Diorama = {
  id: "felinorsvshumans-107",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 78 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -10 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.9 } },
    { name: "KY", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.8 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
  ],
};
