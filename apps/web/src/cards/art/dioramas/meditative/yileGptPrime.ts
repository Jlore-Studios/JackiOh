// R3350000: Meditative #49.1 YileGPT Prime (`meditative-049-1`)'s diorama: the range at the back, the pavilion and its ruins
// before it with their dust rising, the two wings of brushes floating out of step on either side, Prime breathing with its
// arms raised between them, and the near mist.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "meditative-049-1",
  slices: [
    { name: "violet night", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "range", plane: "far", depth: 0.14, pick: ["scene:far:"] },
    { name: "ruins", plane: "middle", depth: 0.32, pick: ["scene:mid:ink-pavilion:", "scene:mid:ruin-wall:", "scene:mid:fallen-column:"] },
    { name: "dust", plane: "middle", depth: 0.42, pick: ["scene:mid:dust:"], loop: { move: "rise", seconds: 6.7, stagger: true } },
    { name: "left brushes", plane: "near", depth: 0.56, pick: ["scene:subject:brush:0-23"], loop: { move: "float", seconds: 4.3 } },
    { name: "right brushes", plane: "near", depth: 0.62, pick: ["scene:subject:brush:24-47"], loop: { move: "float", seconds: 5.1 } },
    { name: "prime", plane: "near", depth: 0.78, pick: ["scene:subject:pose-raise:", "scene:subject:outfit-gown:", "scene:subject:head-void:", "scene:subject:hood:", "scene:subject:item-ink-brush:", "emblem"], loop: { move: "breathe", seconds: 9.3, origin: [50, 82] } },
    { name: "near mist", plane: "nearest", depth: 1, pick: ["scene:near:"] },
  ],
};
