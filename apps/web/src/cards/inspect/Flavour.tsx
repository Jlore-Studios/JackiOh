// A card's flavour line and artist credit (R660) where a card is read at leisure: the hover
// preview's column, the touch sheet and the detail view (the deck builder's and the Card Almanac's).
// Never on a face itself, where the rules have the room. A card with neither draws nothing.
//
// R711000: a Chinese reader who is shown a card the view flagged Chinese (R1301, `face.flagged`) also
// gets one small line where the flavour sits, that it would have been translated, and already is.
// Presentation only: no sound, no state.

import type { ReactElement } from "react";
import type { CardFlavour } from "@jackioh/cards";
import { isZhCn, t, useLocale } from "../../i18n/i18n.ts";
import { CARD_FLAVOUR, flavourFor } from "../flavour.ts";
import { INSPECT_ARTIST, INSPECT_FLAVOUR } from "./testids.ts";

/** R711000: the easter egg line inside the flavour block. */
export const INSPECT_EASTER = "inspect-easter-egg";

type FlavourProps = {
  defId: string;
  /** R711000: the card is one the view flagged Chinese (`FaceModel.flagged`); a Chinese reader gets the egg line. */
  easter?: boolean;
  /** For tests. Defaults to the shipped sidecar. */
  sidecar?: Readonly<Record<string, CardFlavour>>;
};

/** R711000: whether the card's reader gets the easter egg line: Chinese flagged, read in Chinese. */
export function showsEasterEgg(face: { flagged?: boolean }): boolean {
  return face.flagged === true && isZhCn();
}

export function Flavour({ defId, sidecar = CARD_FLAVOUR, easter = false }: FlavourProps): ReactElement | null {
  useLocale();
  const entry = flavourFor(defId, sidecar);
  const egg = easter && isZhCn();
  if (entry === null && !egg) return null;
  return (
    <div className="inspect-flavour" data-testid={INSPECT_FLAVOUR}>
      {egg ? (
        <p className="inspect-flavour-text" data-testid={INSPECT_EASTER}>
          {t("This would be translated to Chinese, but it's already in Chinese!")}
        </p>
      ) : null}
      {entry?.flavour === undefined ? null : <p className="inspect-flavour-text">{entry.flavour}</p>}
      {entry?.artist === undefined ? null : (
        <p className="inspect-artist" data-testid={INSPECT_ARTIST}>
          {t("Art by {artist}", { artist: entry.artist })}
        </p>
      )}
    </div>
  );
}
