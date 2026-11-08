// R3335000: a Starship look as the painter draws it on a launched Starship (cards/art/starshipHulls.ts
// `starshipArtOf`): six real pieces under The Ninth Life, its base face, wearing `look`. The Collection's tiles and a
// Fleet reward's preview draw it. Presentation only (CLAUDE.md rule 7): it names no card and decides nothing.

import type { ReactElement } from "react";

import type { StarshipPieceRef } from "../wire/generated/StarshipPieceRef.ts";
import { starshipArtOf } from "../cards/art/starshipHulls.ts";
import type { StarshipLook } from "../cards/art/starshipLooks.ts";
import { starshipArtUri } from "../cards/art/svg.ts";

/** The launched Starship a preview draws: one module of each of six kinds (a beam, a drill, a shield, a capsule, rounds, a repeater). */
export const STARSHIP_PREVIEW_PIECES: readonly StarshipPieceRef[] = [
  "stinkyscape-058",
  "stinkyscape-066",
  "stinkyscape-038",
  "stinkyscape-043",
  "stinkyscape-100",
  "stinkyscape-051",
].map((defId) => ({ defId, radiant: false }));

/** The name the preview's Starship launched under, whose design a standard hull keeps. */
export const STARSHIP_PREVIEW_NAME = "The Ninth Life";

/** The picture of `look` on the preview's Starship. */
export function starshipPreviewUri(look: StarshipLook): string {
  return starshipArtUri(starshipArtOf(STARSHIP_PREVIEW_PIECES, true, STARSHIP_PREVIEW_NAME, false, look));
}

export default function StarshipPreview({ look }: { look: StarshipLook }): ReactElement {
  return (
    <img
      className="collection-starship"
      src={starshipPreviewUri(look)}
      alt=""
      aria-hidden="true"
      draggable={false}
      data-hull={look.starshipHull}
      data-livery={look.starshipLivery}
      data-trail={look.starshipTrail}
      data-piece-skin={look.starshipPieceSkin}
    />
  );
}
