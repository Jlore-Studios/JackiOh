// Stand-in for part F's map/run screen (#2706, `dungeon/map/RunScreen.tsx`): it keeps the frozen
// `RunScreen({ dungeon })` shape part D's page renders, until the part that owns this directory lands.
import type { Dungeon } from "../store.ts";

export function RunScreen(_props: { dungeon: Dungeon }): null {
  return null;
}
