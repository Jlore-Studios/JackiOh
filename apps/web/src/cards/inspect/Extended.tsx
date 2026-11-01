// The extended description (issues #1017, #814): a face that prints less than its whole text — a
// short form or a clamp (CardFace.tsx `faceText`, shortText.ts) — has its whole text at reading size
// on its left in the hover preview, and under it in the touch sheet. Its numbers (R280), marks (R277) and references
// (R279) are the face's own, a Resource's Credits line follows it (R2521009), and in play the keywords
// it has gained and its granted abilities follow.

import type { ReactElement } from "react";
import { t } from "../../i18n/i18n.ts";
import { faceText, gainedLine } from "../CardFace.tsx";
import { useFullArt } from "../fullArt.ts";
import { creditsLine, type FaceModel } from "../model.ts";
import { RulesText } from "../RulesText.tsx";
import { INSPECT_EXTENDED } from "./testids.ts";

export function Extended({ face, ascentLevel }: { face: FaceModel; ascentLevel?: number | null }): ReactElement | null {
  const fullArtOn = useFullArt(face.defId);
  if (!faceText(face, ascentLevel, fullArtOn).less) return null;
  const gained = gainedLine(face);
  const credits = creditsLine(face);
  return (
    <div className="inspect-printed inspect-extended" data-testid={INSPECT_EXTENDED}>
      <p className="inspect-printed-label">{t("Full text")}</p>
      <p className="inspect-printed-text">
        <RulesText
          text={face.text.full}
          marks={face.text.marks}
          refs={face.refs}
          values={face.values}
          {...(face.text.tuned === undefined ? {} : { tuned: face.text.tuned })}
          chinese={face.chinese === true}
        />
      </p>
      {credits === "" ? null : <p className="inspect-printed-text inspect-extended-credits">{credits}</p>}
      {gained === "" ? null : (
        <p className="inspect-printed-text inspect-extended-gained">
          <RulesText text={gained} />
        </p>
      )}
      {face.grants.map((grant) => (
        <p key={grant} className="inspect-printed-text">
          <RulesText text={grant} />
        </p>
      ))}
    </div>
  );
}
