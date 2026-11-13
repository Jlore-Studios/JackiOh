// The extended description (issues #1017, #814): a face that prints less than its whole text — a
// short form or a clamp (CardFace.tsx `faceText`, the face model's `short`) — has its whole text at reading size
// on its left in the hover preview, and under it in the touch sheet. Its numbers (R280), marks (R277) and references
// (R279) are the face's own, a Resource's Credits line follows it (R2521009), and in play the keywords
// it has gained and its granted abilities follow. R2718000: a full-art card has no rules box, so the
// panel its words move to wears the set's watermark behind them, in the ink for a dark ground.
// R3838001: below an `AlwaysFullText` (the Almanac) every face with words has them there, whole or not.

import { createContext, useContext, type ReactElement, type ReactNode } from "react";
import { t } from "../../i18n/i18n.ts";
import { faceText, printsText } from "../CardFace.tsx";
import { isGlitch } from "../glitch.ts";
import { showsFullArt, useFullArt } from "../fullArt.ts";
import type { FaceModel } from "../model.ts";
import { RulesText } from "../RulesText.tsx";
import { watermarkOf } from "../setMark.ts";
import { useCardSettings } from "../settings.ts";
import { INSPECT_EXTENDED } from "./testids.ts";

const AlwaysFullTextContext = createContext(false);

/** R3838001: the hover previews, sheets and reference tooltips below it carry every card's whole text (the Almanac). */
export function AlwaysFullText({ children }: { children?: ReactNode }): ReactElement {
  return <AlwaysFullTextContext.Provider value={true}>{children}</AlwaysFullTextContext.Provider>;
}

/** R3838001: whether an `AlwaysFullText` is above. */
export function useAlwaysFullText(): boolean {
  return useContext(AlwaysFullTextContext);
}

/**
 * Whether `face` has the Full text panel: it prints less than its whole text (#1017), or (R3838001) `always`, below an
 * `AlwaysFullText`, and it has words. `fullArtOn` is the caller's `useFullArt(defId)`.
 */
export function showsFullText(face: FaceModel, ascentLevel: number | null | undefined, fullArtOn: boolean, always: boolean): boolean {
  return faceText(face, ascentLevel, fullArtOn).less || (always && printsText(face));
}

export function Extended({ face, ascentLevel }: { face: FaceModel; ascentLevel?: number | null }): ReactElement | null {
  const fullArtOn = useFullArt(face.defId);
  const always = useAlwaysFullText();
  const { setWatermarks } = useCardSettings();
  if (!showsFullText(face, ascentLevel, fullArtOn, always)) return null;
  // R2718000: only a full-art face lacks a rules box of its own to hold the watermark.
  const watermark = setWatermarks && showsFullArt(face, fullArtOn) && !isGlitch(face.defId) ? watermarkOf(face.set) : null;
  const gained = face.gainedLine;
  const credits = face.creditsLine;
  return (
    <div className="inspect-printed inspect-extended" data-testid={INSPECT_EXTENDED} data-watermark={watermark?.kind}>
      {watermark !== null && <img className="inspect-watermark" src={watermark.srcOnDark} alt="" aria-hidden="true" draggable={false} />}
      <p className="inspect-printed-label">{t("Full text")}</p>
      <p className="inspect-printed-text">
        <RulesText rich={face.text} />
      </p>
      {credits === "" ? null : <p className="inspect-printed-text inspect-extended-credits">{credits}</p>}
      {gained.full === "" ? null : (
        <p className="inspect-printed-text inspect-extended-gained">
          <RulesText rich={gained} />
        </p>
      )}
      {face.grants.map((grant) => (
        <p key={grant.full} className="inspect-printed-text">
          <RulesText rich={grant} />
        </p>
      ))}
    </div>
  );
}
