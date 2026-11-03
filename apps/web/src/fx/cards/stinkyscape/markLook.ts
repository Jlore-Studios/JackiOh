// #2798: the look Stinkyscape's two mark flourishes (flashFreeze.ts, duoBond.ts) draw in: the colors
// and particle of the mark the engine sends (cards/marks.ts), so the flourish matches the brand the
// mark's own row draws on the card. The colors are the event's, the same on both seats (R202).

import { paletteFor } from "../../../cards/marks.ts";
import type { FlavorLook } from "../../flavors.ts";

/** A mark's palette as a flavor look: its core hot first, its rim the body, its glow the edge. */
export function markLook(color: string, base: FlavorLook): FlavorLook {
  const mark = paletteFor(color);
  return { colors: [mark.core, mark.rim, mark.glow], particle: mark.preset, projectile: base.projectile };
}
