// R3338000, S #4.6 The Big Crunch (`bigCrunch`): over the hold of its first cast the board's backdrop
// collapses to a bright point, and each Call to Chaos it casts plays its own row out of that point; the
// point holds, entry by entry, until its play resolves (`planTerminusFollow`). It draws over its first
// cast and replaces none of its events.

import type { FxCue } from "../types.ts";
import { BOARD, momentPiece, type TerminusPlanner } from "./build.ts";

export const crunch: TerminusPlanner = (m) => {
  const cues: FxCue[] = [momentPiece(m, BOARD, [])];
  return { cues, replaces: new Set() };
};
