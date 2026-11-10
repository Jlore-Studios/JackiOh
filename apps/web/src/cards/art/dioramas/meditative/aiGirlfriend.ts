// R3350000: Meditative #49.3 AI Girlfriend (`meditative-049-3`)'s diorama: the room and its paper screens at the back, the
// tatami bent toward the viewer, the far admirers, her halo flickering and her hologram floating in it, the near admirers,
// and her glitch twinkling nearest.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-049-3",
  slices: [
    { name: "dim room", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "screens", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "tatami", plane: "middle", depth: 0.3, pick: ["scene:mid:tatami:"], bend: { axis: "x", deg: 12 } },
    { name: "far admirers", plane: "middle", depth: 0.42, pick: ["scene:mid:ink-person:"] },
    { name: "halo", plane: "middle", depth: 0.52, pick: ["scene:subject:halo:"], loop: { move: "pulse", seconds: 3.8 } },
    { name: "hologram", plane: "near", depth: 0.66, pick: ["scene:subject:pose-point:", "scene:subject:outfit-silks:", "scene:subject:head-human:", "scene:subject:hair-bun:", "emblem"], loop: { move: "float", seconds: 5.9 } },
    { name: "near admirers", plane: "near", depth: 0.84, pick: ["scene:near:ink-person:", "scene:near:ink-seal:"] },
    { name: "glitch", plane: "nearest", depth: 1, pick: ["scene:near:glitch:"], loop: { move: "twinkle", seconds: 2.3, stagger: true } },
  ],
};
