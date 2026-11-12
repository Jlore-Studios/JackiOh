// R3350000: Stinkyscape token Anomaly Found! (`stinkyscape-t-anomaly-found`)'s diorama: drifting nebulae and the planet at the
// back, the anomaly's glow pulsing, its spiral arms spinning about it, and its white core flaring nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "stinkyscape-t-anomaly-found",
  slices: [
    { name: "space", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "nebulae", plane: "far", depth: 0.12, pick: ["sky:1-3"], loop: { move: "drift", seconds: 12.6 } },
    { name: "planet", plane: "far", depth: 0.26, pick: ["sky:45"] },
    { name: "anomaly glow", plane: "middle", depth: 0.45, pick: ["subject:0"], loop: { move: "pulse", seconds: 5.2 } },
    { name: "vortex", plane: "middle", depth: 0.62, pick: ["subject:1-4"], loop: { move: "spin", seconds: 22 } },
    { name: "core", plane: "nearest", depth: 1, pick: ["subject:5-6"], loop: { move: "flare", seconds: 3.1 } },
  ],
};
