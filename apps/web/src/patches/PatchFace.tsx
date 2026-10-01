// A card face as a patch left it: the collection's printed face (CardFace with no `inPlay`, SPEC
// §10.10), drawn from that version's snapshot rather than the current catalog, so an old cost, old
// stats and old words print as they were (R388).
//
// cards/ is imported by path, not through its barrel: the barrel re-exports the detail view, which
// holds the History section, which holds this face, and a path import keeps that loop out.

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
