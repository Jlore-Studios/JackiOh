// R2801000: a Dormant Starship's hull plates, one bolted on for each piece fused into it, up to
// `STARSHIP_HULL_PLATES_MAX`, on its flanks, port and starboard in turn as its art's modules are
// (cards/art/stinkyscape.ts), above the Dormant seal (dormant.css).
// The count is the view's (`UnitView.starship`, R1852005), public on both seats, and nothing is
// decided (CLAUDE.md rule 7). A launched Starship wears none: its name and its art say it has flown.

import type { ReactElement } from "react";

import { STARSHIP_HULL_PLATES_MAX } from "./constants.ts";

export { STARSHIP_HULL_PLATES_MAX } from "./constants.ts";

export function StarshipHull({ pieces }: { pieces: number }): ReactElement | null {
  const n = Math.max(0, Math.min(pieces, STARSHIP_HULL_PLATES_MAX));
  if (n === 0) return null;
  const plates = Array.from({ length: n }, (_, k) => <i key={k} className="cf-starship-plate" data-plate={k + 1} />);
  return (
    <span className="cf-starship-hull" data-plates={n} aria-hidden="true">
      <span className="cf-starship-flank" data-flank="port">
        {plates.filter((_, k) => k % 2 === 0)}
      </span>
      <span className="cf-starship-flank" data-flank="starboard">
        {plates.filter((_, k) => k % 2 === 1)}
      </span>
    </span>
  );
}
