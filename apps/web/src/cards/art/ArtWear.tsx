// R3340000, R3776000: what a unit's art wears over its picture: a glow in its card's colours when it is buffed and frost when
// it is frozen (artLive.css); missing health draws no mark. They are still drawings that sit over the window inside CardArt's
// span, aria-hidden like the art, so a face under reduced motion shows them all the same.
//
// Presentation only (CLAUDE.md rule 7): the props are flags a unit's view gives its viewer.

import type { ReactElement } from "react";

export type ArtWearProps = {
  buffed: boolean;
  frozen: boolean;
};

export function ArtWear({ buffed, frozen }: ArtWearProps): ReactElement {
  return (
    <span className="cf-art-wear" aria-hidden="true">
      {buffed && <span className="cf-art-glow" />}
      {frozen && <span className="cf-art-frost" />}
    </span>
  );
}
