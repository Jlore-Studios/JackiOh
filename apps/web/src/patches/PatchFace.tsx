// A card face as a patch left it: the collection's printed face (CardFace with no `inPlay`, SPEC
// §10.10), drawn from that version's snapshot (R388). cards/ is imported by path, not through its
// barrel, whose detail view holds the History section that holds this face: an import loop.

import type { ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { CardFace } from "../cards/CardFace.tsx";
import { faceModel } from "../cards/model.ts";
import type { FaceKey } from "./diff.ts";

export function PatchFace({ def, face }: { def: CardDef; face: FaceKey }): ReactElement {
  return (
    <span className="patch-face" data-face={face}>
      <CardFace face={faceModel({ defId: def.id, def, radiant: face === "radiant" })} layout="full" />
    </span>
  );
}
