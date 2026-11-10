// One pool card drawn as the deck builder draws it, from the catalog; a card with none, by its
// name and type. Shared by the preset tiles and the builder's deck list.

import type { ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { RunPoolCard } from "@jackioh/run";

import { CardFace, faceModel } from "../../cards/index.ts";

export function PoolFace({ card }: { card: RunPoolCard }): ReactElement {
  const def = CATALOG[card.id];
  return (
    <CardFace
      face={faceModel({ defId: card.id, ...(def === undefined ? { name: card.name, type: card.cardType } : { def }), radiant: false })}
      layout="full"
    />
  );
}
