// R3350000: Felinors vs Humans #82 The Last Meow (`felinorsvshumans-082`)'s diorama: the battlefield's stars wheeling, the hills and towers behind, the two camps' banners mid-frame, the last black cat over its rune circle, its light pulsing, the cat and crates up close, and the sparks and motes storming in front.

import type { Diorama } from "../types.ts";

export const DIORAMA_THE_LAST_MEOW: Diorama = {
  id: "felinorsvshumans-082",
  slices: [
    { name: "back", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "sky", plane: "far", depth: 0.12, pick: ["scene:sky:"], loop: { move: "spin", seconds: 76 } },
    { name: "backdrop", plane: "far", depth: 0.28, pick: ["scene:far:"], bend: { axis: "y", deg: -10 } },
    { name: "midground", plane: "middle", depth: 0.46, pick: ["scene:mid:"], loop: { move: "float", seconds: 6.9 } },
    { name: "the last cat", plane: "near", depth: 0.62, pick: ["scene:subject:", "emblem"], loop: { move: "breathe", seconds: 8.8 } },
    { name: "light", plane: "near", depth: 0.76, pick: ["scene:light:"], loop: { move: "pulse", seconds: 3.6 } },
    { name: "foreground", plane: "nearest", depth: 0.9, pick: ["scene:near:"] },
    { name: "motes", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "twinkle", seconds: 4.7, stagger: true } },
  ],
};
