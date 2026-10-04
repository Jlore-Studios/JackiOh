// A card's flavour line and artist credit (R658) where a card is read at leisure: the hover
// preview's column, the touch sheet and the detail view (the deck builder's and the Card Almanac's).
// Never on a face itself, where the rules have the room. A card with neither draws nothing.

import type { ReactElement } from "react";
import type { CardFlavour } from "@jackioh/cards";
import { CARD_FLAVOUR, flavourFor } from "../flavour.ts";
import { INSPECT_ARTIST, INSPECT_FLAVOUR } from "./testids.ts";

type FlavourProps = {
  defId: string;
  /** For tests. Defaults to the shipped sidecar. */
  sidecar?: Readonly<Record<string, CardFlavour>>;
};

export function Flavour({ defId, sidecar = CARD_FLAVOUR }: FlavourProps): ReactElement | null {
  const entry = flavourFor(defId, sidecar);
  if (entry === null) return null;
  return (
    <div className="inspect-flavour" data-testid={INSPECT_FLAVOUR}>
      {entry.flavour === undefined ? null : <p className="inspect-flavour-text">{entry.flavour}</p>}
      {entry.artist === undefined ? null : (
        <p className="inspect-artist" data-testid={INSPECT_ARTIST}>
          Art by {entry.artist}
        </p>
      )}
    </div>
  );
}
