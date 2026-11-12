// R3350000: Meditative #97.9 Jlockeed's Headquarters (`meditative-097-9`)'s diorama: the range at the back, the mist swelling
// below the fortress and the cloud drifting beside it, the fortress, its supply carts jolting along before it, the seal,
// and the red halo above it pulsing nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-097-9",
  slices: [
    { name: "dusk", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "range", plane: "far", depth: 0.15, pick: ["scene:far:"] },
    { name: "mist", plane: "middle", depth: 0.3, pick: ["scene:mid:mist:"], loop: { move: "swell", seconds: 9.7, origin: [50, 80] } },
    { name: "cloud", plane: "middle", depth: 0.4, pick: ["scene:mid:ink-cloud:"], loop: { move: "drift", seconds: 12.2, origin: [20, 40] } },
    { name: "fortress", plane: "near", depth: 0.6, pick: ["scene:subject:", "emblem"] },
    { name: "carts", plane: "near", depth: 0.8, pick: ["scene:near:cart:"], loop: { move: "bob", seconds: 2.6 } },
    { name: "seal", plane: "near", depth: 0.88, pick: ["scene:near:ink-seal:"] },
    { name: "red halo", plane: "nearest", depth: 1, pick: ["scene:light:"], loop: { move: "pulse", seconds: 4.1 } },
  ],
};
