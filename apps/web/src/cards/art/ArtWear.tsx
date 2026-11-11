// R3340000: what a unit's art wears over its picture: cracks for the health it is missing (wear.ts), a glow in its card's
// colours when it is buffed and frost when it is frozen (artLive.css). They are still drawings that sit over the window
// inside CardArt's span, aria-hidden like the art, so a face under reduced motion shows them all the same.
//
// Presentation only (CLAUDE.md rule 7): the props are numbers and flags a unit's view gives its viewer.

import type { ReactElement } from "react";

import { crackPaths } from "./wear.ts";

export type ArtWearProps = {
  /** wear.ts `damageLevel`: how many cracks, 0 for none. */
  damage: number;
  buffed: boolean;
  frozen: boolean;
  /** The card's id, which its cracks are grown from. */
  seedId: string;
};

export function ArtWear({ damage, buffed, frozen, seedId }: ArtWearProps): ReactElement {
  return (
    <span className="cf-art-wear" aria-hidden="true">
      {damage > 0 && (
        <svg className="cf-art-cracks" viewBox="0 0 100 100" preserveAspectRatio="none">
          {crackPaths(seedId, damage).map((d) => (
            <path key={d} d={d} fill="none" />
          ))}
        </svg>
      )}
      {buffed && <span className="cf-art-glow" />}
      {frozen && <span className="cf-art-frost" />}
    </span>
  );
}
