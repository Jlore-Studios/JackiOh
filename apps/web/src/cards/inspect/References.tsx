// The cards a face's text names, beside it in the hover preview (SPEC §10.10, R279).
//
// The hover preview takes no pointer events and is hidden from assistive tech (B22), so a reference
// inside its face cannot be hovered or focused: the names are marked there, and this panel shows the
// cards they name, as a small printed face, so resting on any card on the board or in the collection
// shows what it refers to. A name the text calls Radiant shows the Radiant face.
//
// One named card at a time (issue #37, "don't show them all side by side"): a face that names
// several pages through them while the pointer rests, one every REF_CYCLE_MS, its place ("2 of 3")
// under it, since nothing inside a preview can be pressed. The deck builder's detail view pages
// through the same cards by hand (CardDetail.tsx). Presentation only: printed catalog faces (§5.1).

import { useEffect, useState, type ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { CardFace } from "../CardFace.tsx";
import { FACE_ASPECT, REF_PANEL_FACE_HEIGHT_PX } from "../constants.ts";
import { faceModel, type FaceModel } from "../model.ts";
import { RefsInteractive, useDefResolver } from "../refContext.tsx";
import { findRefs } from "../refs.ts";
import { REF_CYCLE_MS } from "./constants.ts";
import { INSPECT_REFS, INSPECT_REFS_POSITION } from "./testids.ts";

/**
 * The named cards of a face, in the order its text first names them, each once per face — the card
 * itself left out, since the preview already shows it (#3's "base Right-house defender", #95's
 * "Call to Chaos").
 */
export function namedCards(face: FaceModel, resolve: (id: string) => CardDef | undefined): { def: CardDef; radiant: boolean }[] {
  const defs = face.refs.filter((id) => id !== face.defId).flatMap((id) => resolve(id) ?? []);
  const seen = new Set<string>();
  const out: { def: CardDef; radiant: boolean }[] = [];
  for (const match of findRefs(face.text.full, defs)) {
    const key = `${match.id}:${String(match.radiant)}`;
    const def = defs.find((candidate) => candidate.id === match.id);
    if (def === undefined || seen.has(key)) continue;
    seen.add(key);
    out.push({ def, radiant: match.radiant });
  }
  return out;
}

export function References({ face }: { face: FaceModel }): ReactElement | null {
  const resolve = useDefResolver();
  const named = resolve === null ? [] : namedCards(face, resolve);
  const count = named.length;
  const [at, setAt] = useState(0);
  useEffect(() => {
    setAt(0);
    if (count < 2) return undefined;
    const timer = window.setInterval(() => setAt((current) => (current + 1) % count), REF_CYCLE_MS);
    return () => window.clearInterval(timer);
  }, [count, face.defId, face.radiant]);
  const shown = named[at % Math.max(count, 1)];
  if (shown === undefined) return null;
  const width = REF_PANEL_FACE_HEIGHT_PX * FACE_ASPECT;
  return (
    <div className="inspect-refs" data-testid={INSPECT_REFS} data-count={count}>
      <p className="inspect-refs-label">Mentions</p>
      <div className="inspect-refs-faces">
        <RefsInteractive enabled={false}>
          <div
            key={`${shown.def.id}:${String(shown.radiant)}`}
            className="inspect-refs-face"
            data-ref={shown.def.id}
            data-ref-face={shown.radiant ? "radiant" : "base"}
            style={{ width, height: REF_PANEL_FACE_HEIGHT_PX }}
          >
            <CardFace face={faceModel({ defId: shown.def.id, def: shown.def, radiant: shown.radiant })} layout="full" />
          </div>
        </RefsInteractive>
      </div>
      {count > 1 && (
        <p className="inspect-refs-position" data-testid={INSPECT_REFS_POSITION}>
          {at % count + 1} of {count}
        </p>
      )}
    </div>
  );
}
