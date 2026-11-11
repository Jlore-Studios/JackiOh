// R2801000: a Dormant Starship's hull plates, one bolted on for each piece fused into it, up to
// `STARSHIP_HULL_PLATES_MAX`, on its flanks, port and starboard in turn as its art's modules are
// (cards/art/stinkyscape.ts), above the Dormant seal (dormant.css). R3333002: each plate shows its
// piece's module icon (`starshipModuleIconUri`), oldest first, and names its kind (`data-module`).
// The list is the view's (`UnitView.starship`, R1852005), public on both seats, and nothing is
// decided (CLAUDE.md rule 7). A launched Starship wears none: its name and its art say it has flown.

import type { ReactElement } from "react";

import type { StarshipPieceRef } from "../wire/generated/StarshipPieceRef.ts";
import { starshipModuleOf } from "./art/starshipModules.ts";
import { starshipModuleIconUri } from "./art/svg.ts";
import { STARSHIP_HULL_PLATES_MAX } from "./constants.ts";

export { STARSHIP_HULL_PLATES_MAX } from "./constants.ts";

export function StarshipHull({ list }: { list: readonly StarshipPieceRef[] }): ReactElement | null {
  const shown = list.slice(0, STARSHIP_HULL_PLATES_MAX);
  if (shown.length === 0) return null;
  const plates = shown.map((piece, k) => (
    <i
      key={k}
      className="cf-starship-plate"
      data-plate={k + 1}
      data-module={starshipModuleOf(piece.defId).kind}
      style={{ backgroundImage: `url("${starshipModuleIconUri(piece.defId, piece.radiant)}")` }}
    />
  ));
  return (
    <span className="cf-starship-hull" data-plates={shown.length} aria-hidden="true">
      <span className="cf-starship-flank" data-flank="port">
        {plates.filter((_, k) => k % 2 === 0)}
      </span>
      <span className="cf-starship-flank" data-flank="starboard">
        {plates.filter((_, k) => k % 2 === 1)}
      </span>
    </span>
  );
}
